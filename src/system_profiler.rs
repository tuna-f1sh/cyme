//! Parser for macOS `system_profiler` command -json output with SPUSBDataType.
//!
//! USBBus and USBDevice structs are used as deserializers for serde. The JSON output with the -json flag is not really JSON; all values are String regardless of contained data so it requires some extra work. Additionally, some values differ slightly from the non json output such as the speed - it is a description rather than numerical.
use std::fmt;
use std::io;
use std::process::Command;
use std::str::FromStr;
use colored::*;
use serde::de::{self, Visitor, SeqAccess, MapAccess};
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};

use crate::types::NumericalUnit;
use crate::usb::get_interface_path;
use crate::usb::get_parent_path;
use crate::usb::get_trunk_path;
use crate::usb::{get_port_path, get_dev_path, ClassCode, Speed, USBDeviceExtra};

/// Modified from https://github.com/vityafx/serde-aux/blob/master/src/field_attributes.rs with addition of base16 encoding
/// Deserializes an option number from string or a number.
/// TODO: - Only really used for vendor id and product id so make struct for these?
///       - handle DeviceNumericalUnit here or another deserializer?
fn deserialize_option_number_from_string<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: fmt::Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumericOrNull<'a, T> {
        Str(&'a str),
        FromStr(T),
        Null,
    }

    match NumericOrNull::<T>::deserialize(deserializer)? {
        NumericOrNull::Str(mut s) => match s {
            "" => Ok(None),
            _ => {
                // -json returns apple_vendor_id in vendor_id for some reason not base16 like normal
                if s.contains("apple_vendor_id") {
                    s = "0x05ac";
                }
                // the vendor_id can be appended with manufacturer name for some reason...split with space to get just base16 encoding
                let vendor_vec: Vec<&str> = s.split(" ").collect();

                if s.contains("0x") {
                    let removed_0x = vendor_vec[0].trim_start_matches("0x");
                    let base16_num = u64::from_str_radix(removed_0x.trim(), 16);
                    let result = match base16_num {
                        Ok(num) => T::from_str(num.to_string().as_str()),
                        Err(e) => return Err(serde::de::Error::custom(e)),
                    };
                    result.map(Some).map_err(serde::de::Error::custom)
                } else {
                    T::from_str(s.trim())
                        .map(Some)
                        .map_err(serde::de::Error::custom)
                }
            }
        },
        NumericOrNull::FromStr(i) => Ok(Some(i)),
        NumericOrNull::Null => Ok(None),
    }
}

/// Root JSON returned from system_profiler and used as holder for all static USB bus data
#[derive(Debug, Serialize, Deserialize)]
pub struct SPUSBDataType {
    /// system buses
    #[serde(rename(deserialize = "SPUSBDataType"), alias = "buses")]
    pub buses: Vec<USBBus>,
}

impl SPUSBDataType {
    /// Flattens entire data store by cloning the `buses`, flattening them and pushing into a new `Vec` and then assigning it to `buses`
    ///
    /// Requires clone of buses so not in place - maybe a more efficient method?
    pub fn flatten(&mut self) -> () {
        let mut new_buses: Vec<USBBus> = Vec::new();
        for mut bus in self.buses.clone() {
            bus.flatten();
            new_buses.push(bus);
        }

        self.buses = new_buses
    }

    /// Returns a flattened Vec of references to all `USBDevice`s in each of the `buses`
    pub fn flatten_devices<'a>(&'a self) -> Vec<&'a USBDevice> {
        let mut ret = Vec::new();
        for bus in &self.buses {
            ret.append(&mut bus.flattened_devices());
        }

        ret
    }
}

impl fmt::Display for SPUSBDataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for v in &self.buses {
            if f.alternate() {
                if f.sign_plus() {
                    writeln!(f, "{:+#}", v)?;
                } else {
                    writeln!(f, "{:#}", v)?;
                }
            } else if f.sign_plus() {
                write!(f, "{:+}", v)?;
            } else {
                write!(f, "{:}", v)?;
            }
        }
        Ok(())
    }
}

/// USB bus JSON returned from system_profiler but now used for other platforms
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct USBBus {
    /// Bus name or product name
    #[serde(rename(deserialize = "_name"), alias = "name")]
    pub name: String,
    /// Host Controller on macOS, vendor put here when using libusb
    pub host_controller: String,
    /// Understood to be product ID - it is when using libusb
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub pci_device: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// Revsision of hardware
    pub pci_revision: Option<u16>,
    /// Understood to be vendor ID - it is when using libusb
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub pci_vendor: Option<u16>,
    /// Number of bus on system
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub usb_bus_number: Option<u8>,
    /// `USBDevices` on the `USBBus`. Since a device can have devices too, need to walk all down all
    #[serde(rename(deserialize = "_items"), alias = "devices")]
    pub devices: Option<Vec<USBDevice>>,
}

/// Returns of Vec of devices in the USBBus as a reference
impl USBBus {
    /// Flattens the bus by copying each device into a new devices `Vec`
    ///
    /// Unlike the `flattened_devices` which returns references that may still contain a `Vec` of `USBDevice`, this function makes those `None` too since it is doing a hard copy.
    ///
    /// Not very pretty or efficient, probably a better way...
    pub fn flatten(&mut self) -> () {
        self.devices = Some(self.flattened_devices().iter().map(|d| {
            let mut new = d.clone().to_owned();
            new.devices = None;
            new
        }).collect());
    }

    /// Returns a flattened `Vec` of references to all `USBDevice`s on the bus
    ///
    /// Note that whilst `Vec` of references is flat, the `USBDevice`s still contain a `devices` `Vec` where the references point; recursive functions on the returned `Vec` will produce wierd results
    pub fn flattened_devices<'a>(&'a self) -> Vec<&'a USBDevice> {
        if let Some(devices) = &self.devices {
            get_all_devices(&devices)
        } else {
            Vec::new()
        }
    }

    /// Whether the bus has [`USBDevice`]s
    pub fn has_devices(&self) -> bool {
        match &self.devices {
            Some(d) => d.len() > 0,
            None => false,
        }
    }

    /// Whether the bus has any empty hubs
    pub fn has_empty_hubs(&self) -> bool {
        match &self.devices {
            Some(d) => d.iter().any(|dd| dd.is_hub() && !dd.has_devices()),
            None => false,
        }
    }

    /// usb_bus_number is not always present in system_profiler output so try to get from first device instead
    pub fn get_bus_number(&self) -> u8 {
        self.usb_bus_number.unwrap_or(
            self.devices
                .as_ref()
                .map_or(None, |d| d.first().map(|dd| dd.location_id.bus))
                .unwrap_or(0),
        )
    }

    /// syspath style path to bus
    pub fn path(&self) -> String {
        get_trunk_path(self.get_bus_number(), &vec![])
    }

    /// sysfs style path to bus interface
    pub fn interface(&self) -> String {
        get_interface_path(self.get_bus_number(), &Vec::new(), 1, 0)
    }

    /// Remove the root_hub if existing in bus
    pub fn remove_root_hub_device(&mut self) -> () {
        self.devices.as_mut().map_or((), |devs| devs.retain(|d| !d.is_root_hub()));
    }

    /// Gets the device that is the root_hub associated with this bus - Linux only
    pub fn get_root_hub_device(&self) -> Option<&USBDevice> {
        if cfg!(target_os = "linux") {
            self.get_node(&self.interface())
        } else {
            None
        }
    }

    /// Gets a mutable device that is the root_hub associated with this bus - Linux only
    pub fn get_root_hub_device_mut(&mut self) -> Option<&mut USBDevice> {
        if cfg!(target_os = "linux") {
            self.get_node_mut(&self.interface())
        } else {
            None
        }
    }

    /// Search for [`USBDevice`] in branches of bus and return reference
    pub fn get_node(&self, port_path: &str) -> Option<&USBDevice> {
        if let Some(devices) = self.devices.as_ref() {
            for dev in devices {
                if let Some(node) = dev.get_node(port_path) {
                    log::debug!("Found {}", node);
                    return Some(node);
                }
            }
        }

        None
    }

    /// Search for [`USBDevice`] in branches of bus and return mutable if found
    pub fn get_node_mut(&mut self, port_path: &str) -> Option<&mut USBDevice> {
        if let Some(devices) = self.devices.as_mut() {
            for dev in devices {
                if let Some(node) = dev.get_node_mut(port_path) {
                    log::debug!("Found {}", node);
                    return Some(node);
                }
            }
        }

        None
    }

    /// Generate a String from self like lsusb default list device
    pub fn to_lsusb_string(&self) -> String {
        format!(
            "Bus {:03} Device 000: ID {:04x}:{:04x} {:} {:}",
            self.get_bus_number(),
            self.pci_vendor.unwrap_or(0xffff),
            self.pci_device.unwrap_or(0xffff),
            self.name,
            self.host_controller,
        )
    }

    /// Generate a tuple (String, String, String) of the lsusb tree output at all three verbosity levels
    ///
    /// Only Linux systems with a root_hub will contain accurate data, others are mainly for styling
    pub fn to_lsusb_tree_string(&self) -> Vec<(String, String, String)> {
        if let Some(root_device) = self.get_root_hub_device() {
            let speed = match &root_device.device_speed {
                Some(v) => match v {
                    DeviceSpeed::SpeedValue(v) => {
                        v.to_lsusb_speed()
                    }
                    DeviceSpeed::Description(_) => String::new()
                }
                None => String::from(""),
            };

            // get these now to save unwrap and to_owned each interface
            let (driver, _, vendor, product) = match &root_device.extra {
                Some(v) => (v.driver.to_owned().unwrap_or(String::new()), v.syspath.to_owned().unwrap_or(String::new()), v.vendor.to_owned().unwrap_or(String::new()), v.product_name.to_owned().unwrap_or(String::new())),
                None => (String::new(), String::new(), String::new(), String::new())
            };

            Vec::from([(
                format!(
                    "Bus {:02}.Port 1: Dev 1, Class=root_hub, Driver={}, {}",
                    self.get_bus_number(),
                    driver,
                    speed
                ),
                format!(
                    "ID {:04x}:{:04x} {} {}",
                    self.pci_vendor.unwrap_or(0xFFFF),
                    self.pci_device.unwrap_or(0xFFFF),
                    vendor,
                    product,
                ),
                format!(
                    "/sys/bus/usb/devices/usb{} {}",
                    self.get_bus_number(),
                    get_dev_path(self.get_bus_number(), &Vec::new())
                )
            )])
        } else {
            log::warn!("Failed to get root_device in bus");
            Vec::from([(
                format!(
                    "Bus {:02}.Port 1: Dev 1, Class=root_hub, Driver=,",
                    self.get_bus_number(),
                ),
                format!(
                    "ID {:04x}:{:04x} {} {}",
                    self.pci_vendor.unwrap_or(0xFFFF),
                    self.pci_device.unwrap_or(0xFFFF),
                    self.host_controller,
                    self.name,
                ),
                format!(
                    "/sys/bus/usb/devices/usb{} {}",
                    self.get_bus_number(),
                    get_dev_path(self.get_bus_number(), &Vec::new())
                )
            )])
        }
    }
}

/// Recursively gets reference to all devices in a [`USBDevice`]
pub fn get_all_devices(devices: &Vec<USBDevice>) -> Vec<&USBDevice> {
    let mut ret: Vec<&USBDevice> = Vec::new();
    for device in devices {
        // push each device into pointer array
        ret.push(device);
        // and run recursively for the device if it has some
        if let Some(d) = &device.devices {
            ret.append(&mut get_all_devices(&d))
        }
    }

    return ret;
}

/// Recursively writeln! of all [`USBDevice`] references
pub fn write_devices_recursive(f: &mut fmt::Formatter, devices: &Vec<USBDevice>) -> fmt::Result {
    for device in devices {
        // don't print root hubs in tree
        if f.sign_plus() && device.is_root_hub() {
            continue
        }
        // print the device details
        if f.alternate() {
            if f.sign_plus() {
                writeln!(f, "{:+#}", device)?;
            } else {
                writeln!(f, "{:#}", device)?;
            }
        } else if f.sign_plus() {
            writeln!(f, "{:+}", device)?;
        } else {
            writeln!(f, "{}", device)?;
        }

        // print all devices with this device - if hub for example
        device
            .devices
            .as_ref()
            .map_or(Ok(()), |d| write_devices_recursive(f, d))?
    }
    Ok(())
}

impl fmt::Display for USBBus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // use plus formatter to add tree
        let tree: &str = if !f.sign_plus() {
            ""
        } else {
            if f.alternate() {
                "\u{25CF} "
            } else {
                "/: "
            }
        };

        // write the bus details - alternative for coloured and apple info style
        if f.alternate() {
            writeln!(
                f,
                "{:}{:} {:} {:}:{:} Revision: 0x{:04x}",
                tree.bright_black().bold(),
                self.name.blue(),
                self.host_controller.green(),
                format!("0x{:04x}", self.pci_vendor.unwrap_or(0xffff))
                    .yellow()
                    .bold(),
                format!("0x{:04x}", self.pci_device.unwrap_or(0xffff)).yellow(),
                self.pci_revision.unwrap_or(0xffff),
            )?;
        } else {
            if f.sign_plus() {
                let interface_strs: Vec<String> = self.to_lsusb_tree_string().iter().map(|s| format!("{}{}", tree, s.0)).collect();
                writeln!(f, "{}", interface_strs.join("\n\r"))?
            } else {
                writeln!(f, "{}", self.to_lsusb_string())?
            }
        }

        // followed by devices if there are some
        self.devices.as_ref().map_or(Ok(()), |d| write_devices_recursive(f, d))
    }
}

/// location_id `String` from system_profiler is "LocationReg / DeviceNo"
/// The LocationReg has the tree structure (0xbbdddddd):
///
///   0x  -- always
///   bb  -- bus number in hexadecimal
///   dddddd -- up to six levels for the tree, each digit represents its
///             position on that level
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct DeviceLocation {
    /// Number of bus attached too
    pub bus: u8,
    /// Will be len() depth in tree and position at each branch
    pub tree_positions: Vec<u8>,
    /// Device number on bus
    pub number: u8,
}

impl FromStr for DeviceLocation {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let location_split: Vec<&str> = s.split("/").collect();
        let reg = location_split
            .first()
            .unwrap()
            .trim()
            .trim_start_matches("0x");

        // get position in tree based on number of non-zero chars or just 0 if not using tree
        let tree_positions: Vec<u8> = reg
            .get(2..)
            .unwrap_or("0")
            .trim_end_matches("0")
            .chars()
            .map(|v| v.to_digit(10).unwrap_or(0) as u8)
            .collect();
        // bus no is msb
        let bus = (u32::from_str_radix(&reg, 16)
            .map_err(|v| io::Error::new(io::ErrorKind::Other, v))
            .unwrap()
            >> 24) as u8;
        // port is after / but not always present
        let number = match location_split.last().unwrap().trim().parse::<u8>() {
            Ok(v) => v,
            // port is not always present for some reason so sum tree positions will be unique
            Err(_) => tree_positions.iter().sum(),
        };

        Ok(DeviceLocation {
            bus,
            tree_positions,
            number,
            ..Default::default()
        })
    }
}

impl DeviceLocation {
    /// Linux style port path where it can be found on system device path - normaly /sys/bus/usb/devices
    pub fn port_path(&self) -> String {
        get_port_path(self.bus, &self.tree_positions)
    }

    /// Port path of parent
    pub fn parent_path(&self) -> Result<String, String> {
        get_parent_path(self.bus, &self.tree_positions)
    }

    /// Port path of trunk
    pub fn trunk_path(&self) -> String {
        get_trunk_path(self.bus, &self.tree_positions)
    }
}

impl<'de> Deserialize<'de> for DeviceLocation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field { Bus, Number, TreePositions }
        struct DeviceLocationVisitor;

        impl<'de> Visitor<'de> for DeviceLocationVisitor {
            type Value = DeviceLocation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with 0xLOCATION_REG/DEVICE_NO")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<DeviceLocation, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let bus = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let tree_positions = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let number = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                Ok(DeviceLocation{ bus, number, tree_positions })
            }

            fn visit_map<V>(self, mut map: V) -> Result<DeviceLocation, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut bus = None;
                let mut number = None;
                let mut tree_positions = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Bus => {
                            if bus.is_some() {
                                return Err(de::Error::duplicate_field("bus"));
                            }
                            bus = Some(map.next_value()?);
                        }
                        Field::Number => {
                            if number.is_some() {
                                return Err(de::Error::duplicate_field("number"));
                            }
                            number = Some(map.next_value()?);
                        }
                        Field::TreePositions => {
                            if tree_positions.is_some() {
                                return Err(de::Error::duplicate_field("tree_positions"));
                            }
                            tree_positions = Some(map.next_value()?);
                        }
                    }
                }
                let bus = bus.ok_or_else(|| de::Error::missing_field("bus"))?;
                let number = number.ok_or_else(|| de::Error::missing_field("number"))?;
                let tree_positions = tree_positions.ok_or_else(|| de::Error::missing_field("tree_positions"))?;
                Ok(DeviceLocation{ bus, number, tree_positions })
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                DeviceLocation::from_str(value.as_str()).map_err(serde::de::Error::custom)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                DeviceLocation::from_str(value).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_any(DeviceLocationVisitor)
    }
}

/// Used for macOS system_profiler dump. Speed is a snake_case string and in case we can't match to a [`Speed`], this allows the String to be stored and not panic
#[derive(Debug, Clone, PartialEq, DeserializeFromStr, SerializeDisplay)]
pub enum DeviceSpeed {
    /// Value as Deserialized into [`Speed`]
    SpeedValue(Speed),
    /// Failed to Deserialize so just the description provided by system_profiler
    Description(String),
}

impl fmt::Display for DeviceSpeed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceSpeed::SpeedValue(v) => {
                let dv = NumericalUnit::<f32>::from(v);
                if f.alternate() && dv.description.is_some() {
                    write!(f, "{}", dv.description.unwrap())
                } else {
                    write!(f, "{:.1}", dv)
                }
            }
            DeviceSpeed::Description(v) => {
                // don't print the description unless alt so it still fits in block
                if f.alternate() {
                    write!(f, "{}", v)
                } else {
                    write!(f, "{:3} {:3}", "-", "-")
                }
            }
        }
    }
}

impl FromStr for DeviceSpeed {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // try to match speed enum else provide string description provided in system_profiler dump
        match Speed::from_str(s) {
            Ok(v) => Ok(DeviceSpeed::SpeedValue(v)),
            Err(_) => Ok(DeviceSpeed::Description(s.to_owned())),
        }
    }
}

/// USB device data based on JSON object output from system_profiler but now used for other platforms
///
/// Desgined to hold static data for the device, obtained from system_profiler Deserializer or cyme::lsusb
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct USBDevice {
    /// The device product name as reported in descriptor or using usb_ids if None
    #[serde(rename(deserialize = "_name"), alias = "name")]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// Unique vendor identifier - purchased from USB IF
    pub vendor_id: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// Vendor unique product identifier
    pub product_id: Option<u16>,
    /// Device location information on bus
    pub location_id: DeviceLocation,
    /// Device serial number as reported by descriptor
    pub serial_num: Option<String>,
    /// The device manufacturer as provided in descriptor or using usb_ids if None
    pub manufacturer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// The device version
    pub bcd_device: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// The supported USB version
    pub bcd_usb: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// macOS system_profiler only - actually bus current in mA not power!
    pub bus_power: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// macOS system_profiler only - actually bus current used in mA not power!
    pub bus_power_used: Option<u16>,
    /// Advertised device capable speed
    pub device_speed: Option<DeviceSpeed>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// macOS system_profiler only - actually bus current used in mA not power!
    pub extra_current_used: Option<u8>,
    /// Devices can be hub and have devices attached so need to walk each device's devices...
    #[serde(rename(deserialize = "_items"), alias = "devices")]
    pub devices: Option<Vec<USBDevice>>,
    // below are not in macOS system profiler but useful enough to have outside of extra
    /// USB device class
    pub class: Option<ClassCode>,
    /// USB sub-class
    pub sub_class: Option<u8>,
    /// USB protocol
    pub protocol: Option<u8>,
    /// Extra data obtained by libusb/udev exploration
    #[serde(default)]
    pub extra: Option<USBDeviceExtra>,
}

impl USBDevice {
    /// Does the device have child devices
    pub fn has_devices(&self) -> bool {
        match &self.devices {
            Some(d) => d.len() > 0,
            None => false,
        }
    }

    /// Gets root_hub [`USBDevice`] if it is one
    pub fn get_root_hub(&self) -> Option<&USBDevice> {
        if self.is_root_hub() {
            return Some(self);
        } else {
            return None;
        }
    }

    /// Recursively walk all [`USBDevice`] from self, looking for the one with `port_path` and returning reference
    ///
    /// Will panic if `port_path` is not a child device or if it sits shallower than self
    pub fn get_node(&self, port_path: &str) -> Option<&USBDevice> {
        // special case for root_hub, it ends with :1.0
        if port_path.ends_with(":1.0") {
            return self.get_root_hub();
        }
        let node_depth = port_path.split('-').last().expect("Invalid port path").split('.').count();
        let current_depth = self.get_depth();
        log::debug!("Get node at {} with {} ({}); depth {}/{}", port_path, self.port_path(), self, current_depth, node_depth);

        // should not be looking for nodes below us unless root
        if current_depth > node_depth {
            panic!("Trying to find node at {}/{} shallower than current position {}!", &port_path, node_depth, current_depth);
        // if we are at depth, just check self or return none
        } else if node_depth == current_depth {
            if self.port_path() == port_path {
                return Some(self);
            } else {
                return None;
            }
        }

        // else walk through devices recursively running function and returning if found
        if let Some(devices) = self.devices.as_ref() {
            for dev in devices {
                if let Some(node) = dev.get_node(port_path) {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Recursively walk all [`USBDevice`] from self, looking for the one with `port_path` and returning mutable
    ///
    /// Will panic if `port_path` is not a child device or if it sits shallower than self
    pub fn get_node_mut(&mut self, port_path: &str) -> Option<&mut USBDevice> {
        if port_path.ends_with(":1.0") {
            if self.is_root_hub() {
                return Some(self);
            } else {
                return None;
            }
        }
        let node_depth = port_path.split('-').last().expect("Invalid port path").split('.').count();
        let current_depth = self.get_depth();
        log::debug!("Get node at {} with {} ({}); depth {}/{}", port_path, self.port_path(), self, current_depth, node_depth);

        // should not be looking for nodes below us
        if current_depth > node_depth {
            panic!("Trying to find node at {}/{} shallower than current position {}!", &port_path, node_depth, current_depth);
        // if we are at depth, just check self or return none
        } else if node_depth == current_depth {
            if self.port_path() == port_path {
                return Some(self);
            } else {
                return None;
            }
        }

        // else walk through devices recursively running function and returning if found
        if let Some(devices) = self.devices.as_mut() {
            for dev in devices {
                if let Some(node) = dev.get_node_mut(port_path) {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Returns position on branch (parent), which is the last number in `tree_positions` also sometimes refered to as port
    pub fn get_branch_position(&self) -> u8 {
        *self.location_id.tree_positions.last().unwrap_or(&0)
    }

    /// The number of [`USBDevice`] deep; branch depth
    pub fn get_depth(&self) -> usize {
        self.location_id.tree_positions.len()
    }

    /// Returns `true` if device is a hub based on device name - not perfect but most hubs advertise as a hub in name - or class code if it has one
    ///
    /// ```
    /// let d = cyme::system_profiler::USBDevice{ name: String::from("My special hub"), ..Default::default() };
    /// assert_eq!(d.is_hub(), true);
    /// ```
    ///
    /// ```
    /// let d = cyme::system_profiler::USBDevice{ name: String::from("My special device"), ..Default::default() };
    /// assert_eq!(d.is_hub(), false);
    /// ```
    pub fn is_hub(&self) -> bool {
        self.name.to_lowercase().contains("hub")
            || self.class.as_ref().map_or(false, |c| *c == ClassCode::Hub)
    }

    /// Linux style port path where it can be found on system device path - normaly /sys/bus/usb/devices
    pub fn port_path(&self) -> String {
        // special case for root_hub, it's the interface 0 on config 1
        if self.is_root_hub() {
            get_interface_path(self.location_id.bus, &self.location_id.tree_positions, 1, 0)
        } else {
            self.location_id.port_path()
        }
    }

    /// Path of parent [`USBDevice`]; one above in tree
    pub fn parent_path(&self) -> Result<String, String> {
        self.location_id.parent_path()
    }

    /// Path of trunk [`USBDevice`]; first in tree
    pub fn trunk_path(&self) -> String {
        self.location_id.trunk_path()
    }

    /// Linux devpath to [`USBDevice`]
    pub fn dev_path(&self) -> String {
        get_dev_path(self.location_id.bus, &self.location_id.tree_positions)
    }

    /// Trunk device is first in tree
    pub fn is_trunk_device(&self) -> bool {
        self.location_id.tree_positions.len() == 1
    }

    /// Root hub is a specific device on Linux, essentially the bus but sits in device tree because of system_profiler legacy
    pub fn is_root_hub(&self) -> bool {
        self.location_id.tree_positions.len() == 0
    }

    /// Generate a String from self like lsusb default list device
    pub fn to_lsusb_string(&self) -> String {
        format!("Bus {:03} Device {:03}: ID {:04x}:{:04x} {}",
            self.location_id.bus,
            self.location_id.number,
            self.vendor_id.unwrap_or(0xffff),
            self.product_id.unwrap_or(0xffff),
            self.name.trim(),
        )
    }

    /// Generate a tuple (String, String, String) of the lsusb tree output at all three verbosity levels
    pub fn to_lsusb_tree_string(&self) -> Vec<(String, String, String)> {
        let mut format_strs = Vec::new();

        let speed = match &self.device_speed {
            Some(v) => match v {
                DeviceSpeed::SpeedValue(v) => {
                    v.to_lsusb_speed()
                }
                DeviceSpeed::Description(_) => String::new()
            }
            None => String::from(""),
        };

        // get these now to save unwrap and to_owned each interface
        let (driver, _, vendor, product) = match &self.extra {
            Some(v) => (v.driver.to_owned().unwrap_or(String::new()), v.syspath.to_owned().unwrap_or(String::new()), v.vendor.to_owned().unwrap_or(String::new()), v.product_name.to_owned().unwrap_or(String::new())),
            None => (String::new(), String::new(), String::new(), String::new())
        };

        if let Some(extra) = self.extra.as_ref() {
            for config in &extra.configurations {
                for interface in &config.interfaces {
                    format_strs.push((
                        format!(
                            "Port {:}: Dev {:}, If {}, Class={:?}, Driver={}, {}",
                            self.get_branch_position(),
                            self.location_id.number,
                            interface.number,
                            interface.class.to_title_case(),
                            interface.driver.as_ref().unwrap_or(&String::new()),
                            speed
                        ),
                        format!(
                            "ID {:04x}:{:04x} {} {}",
                            self.vendor_id.unwrap_or(0xFFFF),
                            self.product_id.unwrap_or(0xFFFF),
                            vendor,
                            product,
                        ),
                        format!(
                            "{}/{} {}",
                            "/sys/bus/usb/devices",
                            interface.path,
                            get_dev_path(self.location_id.bus, &self.location_id.tree_positions)
                        ))
                    );
                }
            }
        } else {
            log::warn!("Rendering {} lsusb tree without extra data because it is missing. No configurations or interfaces will be shown", self);
            format_strs.push((
                format!(
                    "Port {:}: Dev {:}, If {}, Class={:?}, Driver={}, {}",
                    self.get_branch_position(),
                    self.location_id.number,
                    0,
                    self.class.as_ref().map_or(String::new(), |c| c.to_title_case()),
                    driver,
                    speed
                ),
                format!(
                    "ID {:04x}:{:04x} {} {}",
                    self.vendor_id.unwrap_or(0xFFFF),
                    self.product_id.unwrap_or(0xFFFF),
                    // these are actually usb_ids vendor/product but don't have those without extra
                    self.manufacturer.as_ref().unwrap_or(&String::new()),
                    self.name,
                ),
                format!(
                    "{}/{} {}",
                    "/sys/bus/usb/devices",
                    self.port_path(),
                    get_dev_path(self.location_id.bus, &self.location_id.tree_positions)
                ))
            );
        }

        format_strs
    }
}

impl fmt::Display for USBDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut spaces = if f.sign_plus() {
            self.location_id.tree_positions.len() * 4
        } else {
            0
        };

        // map speed from text back to data rate if tree
        let speed = match &self.device_speed {
            Some(v) => v.to_string(),
            None => String::from(""),
        };

        // tree chars to prepend if plus formatted
        let tree: &str = if !f.sign_plus() {
            ""
        } else {
            if f.alternate() {
                "\u{2514}\u{2500}\u{2500} "
            } else {
                "|__ "
            }
        };

        // alternate for coloured, slightly different format to lsusb
        if f.alternate() {
            write!(
                f,
                "{:>spaces$}{}/{} {}:{} {} {} {}",
                tree.bright_black(),
                format!("{:03}", self.location_id.bus).cyan(),
                format!("{:03}", self.location_id.number).magenta(),
                format!("0x{:04x}", self.vendor_id.unwrap_or(0)).yellow().bold(),
                format!("0x{:04x}", self.product_id.unwrap_or(0)).yellow(),
                self.name.trim().bold().blue(),
                self.serial_num
                    .as_ref()
                    .unwrap_or(&String::from("None"))
                    .trim()
                    .green(),
                speed.purple()
            )
        } else {
            // show what we can for lsusb style tree, driver and class can be just ,
            if f.sign_plus() {
                // add 3 because lsusb is aligned with parent
                if spaces > 0 {
                    spaces += 3;
                }
                let interface_strs: Vec<String> = self.to_lsusb_tree_string().iter().map(|s| format!("{:>spaces$}{}", tree, s.0)).collect();
                write!(f, "{}", interface_strs.join("\n\r"))
            } else {
                write!(f, "{}", self.to_lsusb_string())
            }
        }
    }
}

/// Used to filter devices within buses
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct USBFilter {
    /// Retain only devices with vendor id matching this
    pub vid: Option<u16>,
    /// Retain only devices with product id matching this
    pub pid: Option<u16>,
    /// Retain only devices on this bus
    pub bus: Option<u8>,
    /// Retain only devices with this device number
    pub number: Option<u8>,
    /// Retain only devices with name.contains(name)
    pub name: Option<String>,
    /// Retain only devices with serial.contains(serial)
    pub serial: Option<String>,
    /// Exlcude empty hubs in the tree
    pub exclude_empty_hub: bool,
    /// Don't exclude Linux root_hub devices - this is inverse because they are pseudo [`USBBus`]'s in the tree
    pub no_exclude_root_hub: bool,
}

impl USBFilter {
    /// Creates a new filter with defaults
    pub fn new() -> Self {
        Default::default()
    }

    /// Checks whether `device` passes through filter
    pub fn is_match(&self, device: &USBDevice) -> bool {
        (Some(device.location_id.bus) == self.bus || self.bus.is_none())
            && (Some(device.location_id.number) == self.number || self.number.is_none())
            && (device.vendor_id == self.vid || self.vid.is_none())
            && (device.product_id == self.pid || self.pid.is_none())
            && (self
                .name
                .as_ref()
                .map_or(true, |n| device.name.contains(n.as_str())))
            && (self.serial.as_ref().map_or(true, |n| {
                device
                    .serial_num
                    .as_ref()
                    .map_or(false, |s| s.contains(n.as_str()))
            }))
            && !(self.exclude_empty_hub && device.is_hub() && !device.has_devices())
            // && (!device.is_root_hub() || self.no_exclude_root_hub)
    }

    /// Recursively retain only `USBBus` in `buses` with `USBDevice` matching filter
    pub fn retain_buses(&self, buses: &mut Vec<USBBus>) -> () {
        buses.retain(|b| {
            b.usb_bus_number == self.bus || self.bus.is_none() || b.usb_bus_number.is_none()
        });

        for bus in buses {
            bus.devices.as_mut().map_or((), |d| self.retain_devices(d));
        }
    }

    /// Recursively retain only `USBDevice` in `devices` matching filter
    ///
    /// Note that non-matching parents will still be retained if they have a matching `USBDevice` within their branches
    pub fn retain_devices(&self, devices: &mut Vec<USBDevice>) -> () {
        devices.retain(|d| self.exists_in_tree(d));

        for d in devices {
            d.devices.as_mut().map_or((), |d| self.retain_devices(d));
        }
    }

    /// Recursively looks down tree for any `device` matching filter
    ///
    /// Important because a simple check of trunk device might remove a matching device further down the tree
    pub fn exists_in_tree(&self, device: &USBDevice) -> bool {
        // if device itself is a match, just return now and don't bother going keeper
        if self.is_match(device) {
            return true;
        }

        match &device.devices {
            Some(devs) => devs.iter().any(|d| self.exists_in_tree(d)),
            None => false,
        }
    }

    /// Retains only `&USBDevice` in `devices` which match filter
    ///
    /// Does not check down tree so should be used to flattened devices only (`get_all_devices`)
    pub fn retain_flattened_devices_ref(&self, devices: &mut Vec<&USBDevice>) -> () {
        devices.retain(|d| self.is_match(&d))
    }
}

/// Runs the system_profiler command for SPUSBDataType and parses the json stdout into a [`SPUSBDataType`]
pub fn get_spusb() -> Result<SPUSBDataType, io::Error> {
    let output = if cfg!(target_os = "macos") {
        Command::new("system_profiler")
            .args(["-json", "SPUSBDataType"])
            .output()?
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "system_profiler is only supported on macOS",
        ));
    };

    serde_json::from_str(String::from_utf8(output.stdout).unwrap().as_str())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_device() {
        let device_json = "{
              \"_name\" : \"Arduino Zero\",
              \"bcd_device\" : \"1.00\",
              \"bus_power\" : \"500\",
              \"bus_power_used\" : \"500\",
              \"device_speed\" : \"full_speed\",
              \"extra_current_used\" : \"0\",
              \"location_id\" : \"0x02110000 / 3\",
              \"manufacturer\" : \"Arduino LLC\",
              \"product_id\" : \"0x804d\",
              \"serial_num\" : \"6DC00ADC5053574C342E3120FF122422\",
              \"vendor_id\" : \"0x2341\"
            }";

        let device: USBDevice = serde_json::from_str(device_json).unwrap();

        assert_eq!(device.name, "Arduino Zero");
        assert_eq!(device.bcd_device, Some(1.00));
        assert_eq!(device.bus_power, Some(500));
        assert_eq!(device.bus_power_used, Some(500));
        assert_eq!(
            device.device_speed,
            Some(DeviceSpeed::SpeedValue(Speed::FullSpeed))
        );
        assert_eq!(device.extra_current_used, Some(0));
        assert_eq!(
            device.location_id,
            DeviceLocation {
                bus: 2,
                tree_positions: vec![1, 1],
                number: 3,
                ..Default::default()
            }
        );
        assert_eq!(device.manufacturer, Some("Arduino LLC".to_string()));
        assert_eq!(device.product_id, Some(0x804d));
        assert_eq!(device.vendor_id, Some(0x2341));
    }

    #[test]
    fn test_deserialize_bus() {
        let device_json = "{
            \"_name\" : \"USB31Bus\",
            \"host_controller\" : \"AppleUSBXHCITR\",
            \"pci_device\" : \"0x15f0 \",
            \"pci_revision\" : \"0x0006 \",
            \"pci_vendor\" : \"0x8086 \",
            \"usb_bus_number\" : \"0x00 \"
        }";

        let device: USBBus = serde_json::from_str(device_json).unwrap();

        assert_eq!(device.name, "USB31Bus");
        assert_eq!(device.host_controller, "AppleUSBXHCITR");
        assert_eq!(device.pci_device, Some(0x15f0));
        assert_eq!(device.pci_revision, Some(0x0006));
        assert_eq!(device.pci_vendor, Some(0x8086));
        assert_eq!(device.usb_bus_number, Some(0x00));
    }

    #[test]
    fn test_json_dump_read_not_panic() {
        use std::fs::File;
        use std::io::{BufReader, Read};

        let mut data = String::new();
        let f = File::open("./tests/data/system_profiler_dump.json").expect("Unable to open json dump file");
        let mut br = BufReader::new(f);
        br.read_to_string(&mut data).expect("Unable to read string");

        serde_json::from_str::<SPUSBDataType>(&data).unwrap();
    }
}
