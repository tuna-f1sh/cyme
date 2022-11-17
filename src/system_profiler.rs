///! Parser for macOS `system_profiler` command -json output with SPUSBDataType.
///!
///! USBBus and USBDevice structs are used as deserializers for serde. The JSON output with the -json flag is not really JSON; all values are String regardless of contained data so it requires some extra work. Additionally, some values differ slightly from the non json output such as the speed - it is a description rather than numerical.
use std::fmt;
use std::io;
use std::str::FromStr;

use colored::*;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use std::process::Command;

/// Modified from https://github.com/vityafx/serde-aux/blob/master/src/field_attributes.rs with addition of base16 encoding
/// Deserializes an option number from string or a number.
/// Only really used for vendor id and product id so TODO make struct for these
/// TODO handle DeviceNumericalUnit here or another deserializer?
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SPUSBDataType {
    #[serde(rename(deserialize = "SPUSBDataType"))]
    pub buses: Vec<USBBus>,
}

impl SPUSBDataType {
    /// Returns a flattened Vec of all the USBDevices returned from system_profiler as a reference
    pub fn flatten_devices<'a>(&'a self) -> Vec<&'a USBDevice> {
        let mut ret = Vec::new();
        for bus in &self.buses {
            ret.append(&mut bus.flatten_devices());
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

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBBus {
    #[serde(rename(deserialize = "_name"))]
    pub name: String,
    pub host_controller: String,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub pci_device: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub pci_revision: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub pci_vendor: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub usb_bus_number: Option<u8>,
    // devices are normally hubs
    #[serde(rename(deserialize = "_items"))]
    pub devices: Option<Vec<USBDevice>>,
}

/// Returns of Vec of devices in the USBBus as a reference
impl USBBus {
    pub fn flatten_devices<'a>(&'a self) -> Vec<&'a USBDevice> {
        if let Some(devices) = &self.devices {
            get_all_devices(&devices)
        } else {
            Vec::new()
        }
    }

    pub fn has_devices(&self) -> bool {
        match &self.devices {
            Some(d) => d.len() > 0,
            None => false,
        }
    }

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
}

/// Recursively gets reference to all devices in a `USBDevice`
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

pub fn write_devices_recursive(f: &mut fmt::Formatter, devices: &Vec<USBDevice>) -> fmt::Result {
    for device in devices {
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
            .map(|d| write_devices_recursive(f, d));
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
                if self.devices.is_some() {
                    "╓ "
                } else {
                    "- "
                }
            // lsusb tree
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
        // lsusb style but not really accurate...
        } else {
            writeln!(
                f,
                "{:}Bus {:03} Device 000: ID {:04x}:{:04x} {:} {:}",
                tree,
                self.get_bus_number(),
                self.pci_vendor.unwrap_or(0xffff),
                self.pci_device.unwrap_or(0xffff),
                self.name,
                self.host_controller,
            )?;
        }
        // followed by devices if there are some
        self.devices.as_ref().map(|d| write_devices_recursive(f, d));
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
/// location_id String from system_profiler is "LocationReg / Port"
/// The LocationReg has the tree structure (0xbbdddddd):
///   0x  -- always
///   bb  -- bus number in hexadecimal
///   dddddd -- up to six levels for the tree, each digit represents its
///             position on that level
pub struct DeviceLocation {
    pub bus: u8,
    pub tree_positions: Vec<u8>,
    pub port: Option<u8>,
    pub number: Option<u8>,
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
        let port = match location_split.last().unwrap().trim().parse::<u8>() {
            Ok(v) => Some(v),
            // port is not always present for some reason so sum tree positions will be unique
            Err(_) => Some(tree_positions.iter().sum()),
        };

        Ok(DeviceLocation {
            bus,
            tree_positions,
            port,
            ..Default::default()
        })
    }
}

impl<'de> Deserialize<'de> for DeviceLocation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DeviceLocationVisitor;

        impl<'de> Visitor<'de> for DeviceLocationVisitor {
            type Value = DeviceLocation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representation of speed")
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

/// A numerical `value` converted from a String, which includes a `unit` and `description`
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NumericalUnit<T> {
    value: T,
    unit: String,
    description: Option<String>,
}

impl fmt::Display for NumericalUnit<u32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:} {:}", self.value, self.unit)
    }
}

impl fmt::Display for NumericalUnit<f32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // If we received a precision, we use it.
        write!(
            f,
            "{1:.*} {2}",
            f.precision().unwrap_or(2),
            self.value,
            self.unit
        )
    }
}

impl FromStr for NumericalUnit<u32> {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value_split: Vec<&str> = s.trim().split(' ').collect();
        if value_split.len() >= 2 {
            Ok(NumericalUnit {
                value: value_split[0]
                    .trim()
                    .parse::<u32>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
                unit: value_split[1].trim().to_string(),
                description: None,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "string split does not contain [u32] [unit]",
            ))
        }
    }
}

impl FromStr for NumericalUnit<f32> {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value_split: Vec<&str> = s.trim().split(' ').collect();
        if value_split.len() >= 2 {
            Ok(NumericalUnit {
                value: value_split[0]
                    .trim()
                    .parse::<f32>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
                unit: value_split[1].trim().to_string(),
                description: None,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "string split does not contain [f32] [unit]",
            ))
        }
    }
}

impl<'de> Deserialize<'de> for NumericalUnit<u32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DeviceNumericalUnitU32Visitor;

        impl<'de> Visitor<'de> for DeviceNumericalUnitU32Visitor {
            type Value = NumericalUnit<u32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with format '[int] [unit]'")
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NumericalUnit::from_str(value.as_str()).map_err(E::custom)?)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NumericalUnit::from_str(value).map_err(E::custom)?)
            }
        }

        deserializer.deserialize_str(DeviceNumericalUnitU32Visitor)
    }
}

impl<'de> Deserialize<'de> for NumericalUnit<f32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DeviceNumericalUnitF32Visitor;

        impl<'de> Visitor<'de> for DeviceNumericalUnitF32Visitor {
            type Value = NumericalUnit<f32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with format '[float] [unit]'")
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NumericalUnit::from_str(value.as_str()).map_err(E::custom)?)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NumericalUnit::from_str(value).map_err(E::custom)?)
            }
        }

        deserializer.deserialize_str(DeviceNumericalUnitF32Visitor)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(untagged, rename_all = "snake_case")]
pub enum Speed {
    Unknown,
    LowSpeed,
    FullSpeed,
    HighSpeed,
    HighBandwidth,
    SuperSpeed,
    SuperSpeedPlus,
}

impl FromStr for Speed {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "super_speed_plus" => Speed::SuperSpeedPlus,
            "super_speed" => Speed::SuperSpeed,
            "high_speed" | "high_bandwidth" => Speed::HighSpeed,
            "full_speed" => Speed::FullSpeed,
            "low_speed" => Speed::LowSpeed,
            _ => Speed::Unknown,
        })
    }
}

/// Convert from byte returned from device
impl From<u8> for Speed {
    fn from(b: u8) -> Self {
        match b {
            5 => Speed::SuperSpeedPlus,
            4 => Speed::SuperSpeed,
            3 => Speed::HighSpeed,
            2 => Speed::FullSpeed,
            1 => Speed::LowSpeed,
            _ => Speed::Unknown,
        }
    }
}

impl fmt::Display for Speed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Speed::SuperSpeedPlus => "super_speed_plus",
                Speed::SuperSpeed => "super_speed",
                Speed::HighSpeed | Speed::HighBandwidth => "high_speed",
                Speed::FullSpeed => "full_speed",
                Speed::Unknown => "unknown",
                _ => todo!("Unsupported speed"),
            }
        )
    }
}

impl From<&Speed> for NumericalUnit<f32> {
    fn from(speed: &Speed) -> NumericalUnit<f32> {
        match speed {
            Speed::SuperSpeedPlus => NumericalUnit {
                value: 20.0,
                unit: String::from("Gb/s"),
                description: Some(speed.to_string()),
            },
            Speed::SuperSpeed => NumericalUnit {
                value: 5.0,
                unit: String::from("Gb/s"),
                description: Some(speed.to_string()),
            },
            Speed::HighSpeed | Speed::HighBandwidth => NumericalUnit {
                value: 480.0,
                unit: String::from("Mb/s"),
                description: Some(speed.to_string()),
            },
            Speed::FullSpeed => NumericalUnit {
                value: 12.0,
                unit: String::from("Mb/s"),
                description: Some(speed.to_string()),
            },
            Speed::LowSpeed => NumericalUnit {
                value: 1.5,
                unit: String::from("Mb/s"),
                description: Some(speed.to_string()),
            },
            Speed::Unknown => NumericalUnit {
                value: 0.0,
                unit: String::from("Mb/s"),
                description: Some(speed.to_string()),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, DeserializeFromStr, SerializeDisplay)]
pub enum DeviceSpeed {
    SpeedValue(Speed),
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

#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct USBDevice {
    #[serde(rename(deserialize = "_name"))]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub vendor_id: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub product_id: Option<u16>,
    pub location_id: DeviceLocation,
    pub serial_num: Option<String>,
    pub manufacturer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub bcd_device: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub bus_power: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub bus_power_used: Option<u16>,
    pub device_speed: Option<DeviceSpeed>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub extra_current_used: Option<u8>,
    // devices can be hub and have devices attached
    #[serde(rename(deserialize = "_items"))]
    pub devices: Option<Vec<USBDevice>>,
}

impl USBDevice {
    pub fn has_devices(&self) -> bool {
        match &self.devices {
            Some(d) => d.len() > 0,
            None => false,
        }
    }

    /// Returns position on branch (parent), which is the last number in `tree_positions`
    pub fn get_branch_position(&self) -> u8 {
        *self.location_id.tree_positions.last().unwrap_or(&0)
    }

    /// Returns `true` if device is a hub based on device name - not perfect but most hubs advertise as a hub in name
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
            // TODO use "╟─ " unless last
            if f.alternate() {
                "╙── "
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
                format!("{:03}", self.location_id.port.unwrap_or(0)).magenta(),
                format!("0x{:04x}", self.vendor_id.unwrap()).yellow().bold(),
                format!("0x{:04x}", self.product_id.unwrap()).yellow(),
                self.name.trim().bold().blue(),
                self.serial_num
                    .as_ref()
                    .unwrap_or(&String::from("None"))
                    .trim()
                    .green(),
                speed.purple()
            )
        // not same data as lsusb when tree (show port, class, driver etc.)
        } else {
            // add 3 because lsusb is like this
            if spaces > 0 {
                spaces += 3;
            }
            write!(
                f,
                "{:>spaces$}Bus {:03} Device {:03}: ID {:04x}:{:04x} {}",
                tree,
                self.location_id.bus,
                self.location_id.port.unwrap_or(0),
                self.vendor_id.unwrap_or(0xffff),
                self.product_id.unwrap_or(0xffff),
                self.name.trim(),
            )
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct USBFilter {
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub bus: Option<u8>,
    pub port: Option<u8>,
    pub name: Option<String>,
    pub serial: Option<String>,
    pub exclude_empty_hub: bool,
}

impl USBFilter {
    pub fn new() -> Self {
        Default::default()
    }

    /// Checks whether `device` passes through filter
    pub fn is_match(&self, device: &USBDevice) -> bool {
        (Some(device.location_id.bus) == self.bus || self.bus.is_none())
            && (device.location_id.port == self.port || self.port.is_none())
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
                port: Some(3),
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
}
