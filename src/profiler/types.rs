//! USB data structures for system profiling of USB devices and their descriptors.
//!
//! Originally based on serde deserialization of `system_profiler -json` output but now used as data structures for all platforms. Not all fields are used on all platforms or are completely logically in hindsight but it works. Naming is also based on `system_profiler` (SP..) and not very Rustian...
use colored::*;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use std::cmp::Ordering;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::*;
use crate::error::{Error, ErrorKind};
use crate::types::NumericalUnit;
use crate::usb::*;

/// Root JSON returned from system_profiler and used as holder for all static USB bus data
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemProfile {
    /// system buses
    #[serde(rename(deserialize = "SPUSBDataType"), alias = "buses")]
    pub buses: Vec<Bus>,
}

impl SystemProfile {
    /// Returns total number of devices across all buses
    pub fn len(&self) -> usize {
        self.buses.iter().map(|b| b.len()).sum()
    }

    /// Whether all buses are empty
    pub fn is_empty(&self) -> bool {
        self.buses.iter().all(|b| b.is_empty())
    }

    /// Flattens all [`Bus`]es by calling `into_flattened_devices` on each
    ///
    /// In place operation so it mutates the data and tree structure is lost. Location data is still present in each device.
    pub fn into_flattened(&mut self) {
        for bus in &mut self.buses {
            bus.into_flattened_devices();
        }
    }

    /// Returns a flattened Vec of references to all [`Device`]s in each of the `buses`
    pub fn flattened_devices(&self) -> Vec<&Device> {
        let mut ret = Vec::with_capacity(self.len());
        for bus in &self.buses {
            ret.extend(bus.flattened_devices());
        }

        ret
    }

    /// Returns reference to [`Bus`] `number` if it exists in data
    pub fn get_bus(&self, number: u8) -> Option<&Bus> {
        self.buses.iter().find(|b| b.usb_bus_number == Some(number))
    }

    /// Returns mutable reference to [`Bus`] `number` if it exists in data
    pub fn get_bus_mut(&mut self, number: u8) -> Option<&mut Bus> {
        self.buses
            .iter_mut()
            .find(|b| b.usb_bus_number == Some(number))
    }

    /// Search for reference to [`Device`] at [`PortPath`] on correct bus number if present else all buses
    pub fn get_node(&self, port_path: &PortPath) -> Option<&Device> {
        let bus_no = port_path.bus();

        // the logic of getting bus is required because bus_no is Optional; there may be valid port part on a bus with no number
        if let Some(bus) = self.get_bus(bus_no) {
            if let Some(node) = bus.get_node(port_path) {
                return Some(node);
            }
        } else {
            for bus in self.buses.iter() {
                if let Some(node) = bus.get_node(port_path) {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Search for mutable reference to [`Device`] at [`PortPath`] on correct bus number if present else all buses
    pub fn get_node_mut(&mut self, port_path: &PortPath) -> Option<&mut Device> {
        let bus_no = port_path.bus();

        if self.buses.iter().any(|b| b.usb_bus_number == Some(bus_no)) {
            if let Some(bus) = self.get_bus_mut(bus_no) {
                if let Some(node) = bus.get_node_mut(port_path) {
                    return Some(node);
                }
            }
        } else {
            for bus in self.buses.iter_mut() {
                if let Some(node) = bus.get_node_mut(port_path) {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Search for reference to [`Device`] at `port_path` as Path
    pub fn get_node_by_path<P: AsRef<Path>>(&self, path: P) -> Option<&Device> {
        UsbPath::from(path.as_ref())
            .port_path()
            .and_then(|pp| self.get_node(&pp))
    }

    /// Search for mutable reference to [`Device`] at `port_path` as Path
    pub fn get_node_path_by_path_mut<P: AsRef<Path>>(&mut self, path: P) -> Option<&mut Device> {
        UsbPath::from(path.as_ref())
            .port_path()
            .and_then(|pp| self.get_node_mut(&pp))
    }

    /// Search for reference to [`Device`] at port path as string
    pub fn get_node_by_str(&self, path: &str) -> Option<&Device> {
        PortPath::try_from(path)
            .ok()
            .and_then(|pp| self.get_node(&pp))
    }

    /// Search for mutable reference to [`Device`] at port path as string
    pub fn get_node_str_mut(&mut self, path: &str) -> Option<&mut Device> {
        PortPath::try_from(path)
            .ok()
            .and_then(|pp| self.get_node_mut(&pp))
    }

    /// Get reference to [`Configuration`] at `port_path` and `config` if present
    pub fn get_config(&self, port_path: &PortPath, config: u8) -> Option<&Configuration> {
        self.get_node(port_path).and_then(|d| d.get_config(config))
    }

    /// Get mutable reference to [`Configuration`] at [`PortPath`] and `config` if present
    pub fn get_config_mut(
        &mut self,
        port_path: &PortPath,
        config: u8,
    ) -> Option<&mut Configuration> {
        self.get_node_mut(port_path)
            .and_then(|d| d.get_config_mut(config))
    }

    /// Get reference to [`Interface`] at `port_path`, `config` and `interface` if present in [`DevicePath`]
    pub fn get_interface(&self, device_path: &DevicePath) -> Option<&Interface> {
        if let (Some(config), Some(interface)) =
            (device_path.configuration(), device_path.interface())
        {
            self.get_node(device_path.port_path())
                .and_then(|d| d.get_interface(config, interface, device_path.alt_setting()))
        } else {
            None
        }
    }

    /// Get mutable reference to [`Interface`] at `port_path`, `config` and `interface` if present in [`DevicePath`]
    pub fn get_interface_mut(&mut self, device_path: &DevicePath) -> Option<&mut Interface> {
        if let (Some(config), Some(interface)) =
            (device_path.configuration(), device_path.interface())
        {
            self.get_node_mut(device_path.port_path())
                .and_then(|d| d.get_interface_mut(config, interface, device_path.alt_setting()))
        } else {
            None
        }
    }

    /// Get reference to [`Endpoint`] at `port_path`, `config`, `interface` and `endpoint` if present
    pub fn get_endpoint(&self, endpoint_path: &EndpointPath) -> Option<&Endpoint> {
        if let (Some(config), Some(interface), endpoint) = (
            endpoint_path.device_path().configuration(),
            endpoint_path.device_path().interface(),
            endpoint_path.endpoint(),
        ) {
            self.get_node(endpoint_path.device_path().port_path())
                .and_then(|d| {
                    d.get_endpoint(
                        config,
                        interface,
                        endpoint_path.device_path().alt_setting(),
                        endpoint,
                    )
                })
        } else {
            None
        }
    }

    /// Get mutable reference to [`Endpoint`] at `port_path`, `config`, `interface` and `endpoint` if present
    pub fn get_endpoint_mut(&mut self, endpoint_path: &EndpointPath) -> Option<&mut Endpoint> {
        if let (Some(config), Some(interface), endpoint) = (
            endpoint_path.device_path().configuration(),
            endpoint_path.device_path().interface(),
            endpoint_path.endpoint(),
        ) {
            self.get_node_mut(endpoint_path.device_path().port_path())
                .and_then(|d| {
                    d.get_endpoint_mut(
                        config,
                        interface,
                        endpoint_path.device_path().alt_setting(),
                        endpoint,
                    )
                })
        } else {
            None
        }
    }

    #[cfg(feature = "nusb")]
    /// Search for [`::nusb::DeviceId`] in branches of bus and return reference
    pub fn get_id(&self, id: &::nusb::DeviceId) -> Option<&Device> {
        for bus in self.buses.iter() {
            if let Some(node) = bus.get_id(id) {
                return Some(node);
            }
        }
        None
    }

    #[cfg(feature = "nusb")]
    /// Search for [`::nusb::DeviceId`] in branches of bus and returns a mutable reference if found
    pub fn get_id_mut(&mut self, id: &::nusb::DeviceId) -> Option<&mut Device> {
        for bus in self.buses.iter_mut() {
            if let Some(node) = bus.get_id_mut(id) {
                return Some(node);
            }
        }
        None
    }

    /// Replace a [`Device`] in the correct [`Bus`] and parent device based on its location_id
    ///
    /// If the device was existing, it will be replaced with the old device and returned as `Ok`, else `Err` if the device was not found.
    pub fn replace(&mut self, mut new: Device) -> Result<Device> {
        if let Some(existing) = self.get_node_mut(&new.port_path()) {
            let devices = std::mem::take(&mut existing.devices);
            new.devices = devices;
            // config, interface and endpoint will loose internal but not worried about that
            new.internal = existing.internal.clone();
            // assume all was expanded if manually expanded
            // could drill down copying each internal above but not sure worth it
            if new.is_expanded() {
                new.set_all_expanded(true);
            }
            let ret = std::mem::replace(existing, new);
            Ok(ret)
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                "Device not found to replace",
            ))
        }
    }

    /// Insert a [`Device`] into the correct [`Bus`] and parent device based on its location_id
    ///
    /// If the device was existing, it will be replaced with the new device and returned as `Some` (without child devices), else `None`. `None` will also be returned if the device parent is not found.
    pub fn insert(&mut self, new: Device) -> Option<Device> {
        // check existing device and replace if found
        if self.get_node_mut(&new.port_path()).is_some() {
            return self.replace(new).ok();
        // else we have to stick into tree at correct place
        } else if new.is_trunk_device() {
            let bus = self.get_bus_mut(new.location_id.bus).unwrap();
            if let Some(bd) = bus.devices.as_mut() {
                bd.push(new);
            } else {
                bus.devices = Some(vec![new]);
            }
        } else if let Some(parent_path) = new.parent_port_path() {
            if let Some(parent) = self.get_node_mut(&parent_path) {
                if let Some(bd) = parent.devices.as_mut() {
                    bd.push(new);
                } else {
                    parent.devices = Some(vec![new]);
                }
            }
        }

        None
    }
}

impl<'a> IntoIterator for &'a SystemProfile {
    type Item = &'a Device;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> std::vec::IntoIter<Self::Item> {
        self.flattened_devices().into_iter()
    }
}

impl fmt::Display for SystemProfile {
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

/// Deprecated alias for [`SystemProfile`]
#[deprecated(since = "2.0.0", note = "Use SystemProfile instead")]
pub type SPUSBDataType = SystemProfile;

#[derive(Debug, Clone)]
pub(crate) struct PciInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub revision: u16,
}

/// USB bus returned from system_profiler but now used for other platforms.
///
/// It is a merging of the PCI Host Controller information and root hub device data (if present). Essentially a root hub but not as a pseudo device but an explicit type - since the root hub is a bit confusing in that sense.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bus {
    /// System internal bus name based on Root Hub device name
    ///
    /// Normally something generic like 'Root Hub', 'USB 3.0 Bus'
    #[serde(rename(deserialize = "_name"), alias = "name")]
    pub name: String,
    /// System internal bus provider name
    pub host_controller: String,
    /// Vendor name of PCI Host Controller from pci.ids
    pub host_controller_vendor: Option<String>,
    /// Device name of PCI Host Controller from pci.ids
    pub host_controller_device: Option<String>,
    /// PCI vendor ID (VID)
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub pci_vendor: Option<u16>,
    /// PCI device ID (PID)
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub pci_device: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// PCI Revsision ID
    pub pci_revision: Option<u16>,
    /// Number of bus on system
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub usb_bus_number: Option<u8>,
    /// [`Device`]s on the [`Bus`]. Since a device can have devices too, need to walk down all devices to get all devices on the bus
    ///
    /// On Linux, the root hub is also included in this list
    #[serde(rename(deserialize = "_items"), alias = "devices")]
    pub devices: Option<Vec<Device>>,
    /// Internal data for tracking events and other data
    #[serde(skip)]
    pub(crate) internal: InternalData,
}

/// Deprecated alias for [`Bus`]
#[deprecated(since = "2.0.0", note = "Use Bus instead")]
pub type USBBus = Bus;

impl TryFrom<Device> for Bus {
    type Error = Error;

    fn try_from(device: Device) -> Result<Self> {
        if !device.is_root_hub() {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Device is not a root hub",
            ));
        }

        // attempt to get PCI info from platform
        let (pci_vendor, pci_device, pci_revision) = match platform::pci_info_from_device(&device) {
            Some(v) => (Some(v.vendor_id), Some(v.product_id), Some(v.revision)),
            None => (None, None, None),
        };

        let (host_controller_vendor, host_controller_device) =
            if let (Some(v), Some(p)) = (pci_vendor, pci_device) {
                log::debug!("looking up PCI IDs: {:04x}:{:04x}", v, p);
                match pci_ids::Device::from_vid_pid(v, p) {
                    Some(d) => (
                        Some(d.vendor().name().to_string()),
                        Some(d.name().to_string()),
                    ),
                    None => (None, None),
                }
            } else {
                (None, None)
            };

        Ok(Bus {
            name: device.name,
            host_controller: device.manufacturer.unwrap_or_default(),
            host_controller_vendor,
            host_controller_device,
            pci_device: pci_device.filter(|v| *v != 0xffff && *v != 0),
            pci_vendor: pci_vendor.filter(|v| *v != 0xffff && *v != 0),
            pci_revision: pci_revision.filter(|v| *v != 0xffff && *v != 0),
            usb_bus_number: Some(device.location_id.bus),
            devices: device.devices,
            ..Default::default()
        })
    }
}

impl<'a> IntoIterator for &'a Bus {
    type Item = &'a Device;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> std::vec::IntoIter<Self::Item> {
        self.flattened_devices().into_iter()
    }
}

/// A generic Bus from a u8 bus number - used if Bus profiling is not available
impl From<u8> for Bus {
    fn from(bus: u8) -> Self {
        Bus {
            name: format!("USB Bus {:03}", bus),
            host_controller: String::from("USB Host Controller"),
            usb_bus_number: Some(bus),
            ..Default::default()
        }
    }
}

/// Returns of Vec of devices in the Bus as a reference
impl Bus {
    /// Returns total number of devices in the bus
    pub fn len(&self) -> usize {
        self.devices
            .as_ref()
            .map_or(0, |d| d.iter().map(|dd| dd.len()).sum())
    }

    /// Flattens the bus by copying each device into a new devices `Vec`
    ///
    /// Unlike the `flattened_devices` which returns references that may still contain a `Vec` of `Device`, this function makes those `None` too since it is doing a hard copy.
    ///
    /// Not very pretty or efficient, probably a better way...
    pub fn into_flattened_devices(&mut self) {
        if let Some(mut devices) = self.devices.take() {
            let mut new_devices: Vec<Device> = Vec::new();
            while let Some(device) = devices.pop() {
                new_devices.extend(device.into_flattened())
            }

            self.devices = Some(new_devices)
        }
    }

    /// Returns a flattened `Vec` of references to all `Device`s on the bus
    ///
    /// Note that whilst `Vec` of references is flat, the `Device`s still contain a `devices` `Vec` where the references point; recursive functions on the returned `Vec` will produce weird results
    pub fn flattened_devices(&self) -> Vec<&Device> {
        if let Some(devices) = &self.devices {
            devices.iter().flat_map(|d| d.flatten()).collect()
        } else {
            Vec::new()
        }
    }

    /// Whether the bus has no [`Device`]s
    pub fn is_empty(&self) -> bool {
        match &self.devices {
            Some(d) => d.is_empty() || d.iter().all(|dd| dd.internal.hidden),
            None => true,
        }
    }

    /// Whether the bus has just empty hubs
    pub fn has_empty_hubs(&self) -> bool {
        match &self.devices {
            Some(d) => d.iter().all(|dd| dd.is_hub() && !dd.has_devices()),
            None => false,
        }
    }

    /// usb_bus_number is not always present in system_profiler output so try to get from first device instead
    pub fn get_bus_number(&self) -> Option<u8> {
        self.usb_bus_number.or_else(|| {
            self.devices
                .as_ref()
                .and_then(|d| d.first().map(|dd| dd.location_id.bus))
        })
    }

    /// syspath style path to bus
    pub fn path(&self) -> Option<PathBuf> {
        self.get_bus_number().map(|n| get_trunk_path(n, &[]).into())
    }

    /// sysfs style path to bus interface
    pub fn interface(&self) -> Option<DevicePath> {
        self.get_bus_number()
            .map(|n| DevicePath::new(n, vec![0], Some(1), Some(0), None))
    }

    /// Remove the root_hub if existing in bus
    pub fn remove_root_hub_device(&mut self) {
        self.devices
            .iter_mut()
            .for_each(|devs| devs.retain(|d| !d.is_root_hub()));
    }

    /// Gets the device that is the root_hub associated with this bus - Linux only but exists in case of using --from-json
    pub fn get_root_hub_device(&self) -> Option<&Device> {
        self.interface().and_then(|i| self.get_node(i.port_path()))
    }

    /// Gets a mutable device that is the root_hub associated with this bus - Linux only but exists in case of using --from-json
    pub fn get_root_hub_device_mut(&mut self) -> Option<&mut Device> {
        self.interface()
            .and_then(|i| self.get_node_mut(i.port_path()))
    }

    /// Search for [`Device`] in branches of bus and return reference
    pub fn get_node(&self, port_path: &PortPath) -> Option<&Device> {
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

    /// Search for [`Device`] in branches of bus and return mutable if found
    pub fn get_node_mut(&mut self, port_path: &PortPath) -> Option<&mut Device> {
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

    /// Search for [`Device`] in branches of bus by Path and return reference
    pub fn get_node_by_path<P: AsRef<Path>>(&self, port_path: P) -> Option<&Device> {
        UsbPath::from(port_path.as_ref())
            .port_path()
            .and_then(|pp| self.get_node(&pp))
    }

    /// Search for [`Device`] in branches of bus by Path and return mutable reference
    pub fn get_node_path_by_path_mut<P: AsRef<Path>>(
        &mut self,
        port_path: P,
    ) -> Option<&mut Device> {
        UsbPath::from(port_path.as_ref())
            .port_path()
            .and_then(|pp| self.get_node_mut(&pp))
    }

    /// Search for reference to [`Device`] at port path as string
    pub fn get_node_by_str(&self, path: &str) -> Option<&Device> {
        PortPath::try_from(path)
            .ok()
            .and_then(|pp| self.get_node(&pp))
    }

    /// Search for mutable reference to [`Device`] at port path as string
    pub fn get_node_str_mut(&mut self, path: &str) -> Option<&mut Device> {
        PortPath::try_from(path)
            .ok()
            .and_then(|pp| self.get_node_mut(&pp))
    }

    /// Get reference to [`Configuration`] at [`PortPath`] and `config` if present
    pub fn get_config(&self, port_path: &PortPath, config: u8) -> Option<&Configuration> {
        self.get_node(port_path).and_then(|d| d.get_config(config))
    }

    /// Get mutable reference to [`Configuration`] at [`PortPath`] and `config` if present
    pub fn get_config_mut(
        &mut self,
        port_path: &PortPath,
        config: u8,
    ) -> Option<&mut Configuration> {
        self.get_node_mut(port_path)
            .and_then(|d| d.get_config_mut(config))
    }

    /// Get reference to [`Interface`] at [`DevicePath`] if config and interface are present
    pub fn get_interface(&self, device_path: &DevicePath) -> Option<&Interface> {
        if let (Some(config), Some(interface)) =
            (device_path.configuration(), device_path.interface())
        {
            self.get_node(device_path.port_path())
                .and_then(|d| d.get_interface(config, interface, device_path.alt_setting()))
        } else {
            None
        }
    }

    /// Get mutable reference to [`Interface`] at [`DevicePath`] if config and interface are present
    pub fn get_interface_mut(&mut self, device_path: &DevicePath) -> Option<&mut Interface> {
        if let (Some(config), Some(interface)) =
            (device_path.configuration(), device_path.interface())
        {
            self.get_node_mut(device_path.port_path())
                .and_then(|d| d.get_interface_mut(config, interface, device_path.alt_setting()))
        } else {
            None
        }
    }

    /// Get reference to [`Endpoint`] at [`EndpointPath`] if config and interface are present
    pub fn get_endpoint(&self, endpoint_path: &EndpointPath) -> Option<&Endpoint> {
        if let (Some(config), Some(interface), endpoint) = (
            endpoint_path.device_path().configuration(),
            endpoint_path.device_path().interface(),
            endpoint_path.endpoint(),
        ) {
            self.get_node(endpoint_path.device_path().port_path())
                .and_then(|d| {
                    d.get_endpoint(
                        config,
                        interface,
                        endpoint_path.device_path().alt_setting(),
                        endpoint,
                    )
                })
        } else {
            None
        }
    }

    /// Get mutable reference to [`Endpoint`] at [`EndpointPath`] if config and interface are present
    pub fn get_endpoint_mut(&mut self, endpoint_path: &EndpointPath) -> Option<&mut Endpoint> {
        if let (Some(config), Some(interface), endpoint) = (
            endpoint_path.device_path().configuration(),
            endpoint_path.device_path().interface(),
            endpoint_path.endpoint(),
        ) {
            self.get_node_mut(endpoint_path.device_path().port_path())
                .and_then(|d| {
                    d.get_endpoint_mut(
                        config,
                        interface,
                        endpoint_path.device_path().alt_setting(),
                        endpoint,
                    )
                })
        } else {
            None
        }
    }

    #[cfg(feature = "nusb")]
    /// Search for [`::nusb::DeviceId`] in branches of bus and return reference
    pub fn get_id(&self, id: &::nusb::DeviceId) -> Option<&Device> {
        if let Some(devices) = self.devices.as_ref() {
            for dev in devices {
                if let Some(node) = dev.get_id(id) {
                    return Some(node);
                }
            }
        }

        None
    }

    #[cfg(feature = "nusb")]
    /// Search for [`::nusb::DeviceId`] in branches of bus and returns a mutable reference if found
    pub fn get_id_mut(&mut self, id: &::nusb::DeviceId) -> Option<&mut Device> {
        if let Some(devices) = self.devices.as_mut() {
            for dev in devices {
                if let Some(node) = dev.get_id_mut(id) {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Generate a String from self like lsusb default list device
    pub fn to_lsusb_string(&self) -> String {
        format!(
            "Bus {:03} Device 000: ID {:04x}:{:04x} {} {}",
            self.get_bus_number().unwrap_or(0xff),
            self.pci_vendor.unwrap_or(0xffff),
            self.pci_device.unwrap_or(0xffff),
            self.name,
            self.host_controller,
        )
    }

    /// Generate a Vec of Strings from self in lsusb tree self in order of verbosity
    ///
    /// Only Linux systems with a root_hub will contain accurate data, others are mainly for styling
    pub fn to_lsusb_tree_string(&self, verbosity: u8) -> Vec<String> {
        let mut ret: Vec<String> = Vec::with_capacity(verbosity as usize);
        if let Some(root_device) = self.get_root_hub_device() {
            let speed = match &root_device.device_speed {
                Some(v) => match v {
                    DeviceSpeed::SpeedValue(v) => v.to_lsusb_speed(),
                    DeviceSpeed::Description(_) => String::new(),
                },
                None => String::from(""),
            };

            // no fallback for lsusb tree mode
            let (driver, vendor, product, ports) = match &root_device.extra {
                Some(v) => (
                    v.driver.to_owned().unwrap_or(String::from("[none]")),
                    v.vendor.to_owned().unwrap_or(String::from("[unknown]")),
                    v.product_name
                        .to_owned()
                        .unwrap_or(String::from("[unknown]")),
                    v.hub.to_owned().map(|h| h.num_ports),
                ),
                None => (
                    String::from("[none]"),
                    String::from("[unknown]"),
                    String::from("[unknown]"),
                    None,
                ),
            };

            let driver_string = if let Some(ports) = ports {
                format!("{}/{}p", driver, ports)
            } else {
                driver
            };

            ret.push(format!(
                "Bus {:03}.Port 001: Dev 001, Class=root_hub, Driver={}, {}",
                self.get_bus_number().unwrap_or(0xff),
                driver_string,
                speed
            ));
            if verbosity > 0 {
                ret.push(format!(
                    "ID {:04x}:{:04x} {} {}",
                    self.pci_vendor.unwrap_or(0xFFFF),
                    self.pci_device.unwrap_or(0xFFFF),
                    vendor,
                    product
                ));
            }
            if verbosity > 1 {
                ret.push(format!(
                    "/sys/bus/usb/devices/usb{}  {}",
                    self.get_bus_number().unwrap_or(0xff),
                    get_dev_path(self.get_bus_number().unwrap_or(0xff), None).display()
                ));
            }
        } else {
            log::warn!("Failed to get root_device in bus");
            ret.push(format!(
                "Bus {:03}.Port 001: Dev 001, Class=root_hub, Driver=[none],",
                self.get_bus_number().unwrap_or(0xff),
            ));
            if verbosity > 0 {
                ret.push(format!(
                    "ID {:04x}:{:04x} {} {}",
                    self.pci_vendor.unwrap_or(0xFFFF),
                    self.pci_device.unwrap_or(0xFFFF),
                    self.host_controller,
                    self.name,
                ));
            }
            if verbosity > 1 {
                ret.push(format!(
                    "/sys/bus/usb/devices/usb{}  {}",
                    self.get_bus_number().unwrap_or(0xff),
                    get_dev_path(self.get_bus_number().unwrap_or(0xff), None).display()
                ));
            }
        }

        ret
    }

    pub(crate) fn fill_host_controller_from_ids(&mut self) {
        if let (Some(v), Some(p)) = (self.pci_vendor, self.pci_device) {
            if let Some(d) = pci_ids::Device::from_vid_pid(v, p) {
                self.host_controller_vendor = Some(d.vendor().name().to_string());
                self.host_controller_device = Some(d.name().to_string());
            }
        }
    }

    /// Should the bus be hidden when printing
    pub fn is_hidden(&self) -> bool {
        self.internal.hidden
    }

    /// Should the bus be expanded when printing
    pub fn is_expanded(&self) -> bool {
        self.internal.expanded
    }

    /// Toggle the expanded state of the bus
    pub fn toggle_expanded(&mut self) {
        self.internal.expanded = !self.internal.expanded;
    }

    /// Set the expanded state of the bus and all devices
    pub fn set_all_expanded(&mut self, expanded: bool) {
        self.internal.expanded = expanded;
        if let Some(devices) = self.devices.as_mut() {
            for dev in devices {
                dev.set_all_expanded(expanded);
            }
        }
    }
}

/// Recursively writeln! of all [`Device`] references
pub fn write_devices_recursive(f: &mut fmt::Formatter, devices: &Vec<Device>) -> fmt::Result {
    for device in devices {
        // don't print root hubs in tree
        if f.sign_plus() && device.is_root_hub() {
            continue;
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

impl fmt::Display for Bus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // use plus formatter to add tree
        let tree: &str = if !f.sign_plus() {
            ""
        } else if f.alternate() {
            "\u{25CF} "
        } else {
            "/: "
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
        } else if f.sign_plus() {
            writeln!(
                f,
                "{}{}",
                tree,
                self.to_lsusb_tree_string(0).first().unwrap()
            )?;
        } else {
            writeln!(f, "{}", self.to_lsusb_string())?
        }

        // followed by devices if there are some
        self.devices
            .as_ref()
            .map_or(Ok(()), |d| write_devices_recursive(f, d))
    }
}

/// location_id `String` from system_profiler is "LocationReg / DeviceNo"
/// The LocationReg has the tree structure (0xbbdddddd):
///
///   0x  -- always
///   bb  -- bus number in hexadecimal
///   dddddd -- up to six levels for the tree, each digit represents its
///             position on that level
#[derive(Debug, Default, Clone, PartialEq, Serialize, Eq)]
pub struct DeviceLocation {
    /// Number of bus attached too
    pub bus: u8,
    /// Length is depth in tree and position at each branch, empty is bus controller
    pub tree_positions: Vec<u8>,
    /// Device number on bus, generally not related to tree
    pub number: u8,
}

/// Converts from macOS system_profiler string to `DeviceLocation`
impl FromStr for DeviceLocation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let location_split: Vec<&str> = s.split('/').collect();
        match (location_split.first(), location_split.get(1)) {
            (Some(reg), Some(dev)) => {
                let reg = reg.trim().trim_start_matches("0x");
                let tree_positions: Vec<u8> = reg
                    .get(2..)
                    .unwrap_or("0")
                    .trim_end_matches('0')
                    .chars()
                    .map(|v| v.to_digit(10).unwrap_or(0) as u8)
                    .collect();
                // bus no is msb
                let bus = (u32::from_str_radix(reg, 16)
                    .map_err(|v| Error::new(ErrorKind::Parsing, &v.to_string()))?
                    >> 24) as u8;
                let number = dev.trim().parse::<u8>().map_err(|v| {
                    Error::new(
                        ErrorKind::Parsing,
                        &format!("Invalid device number: {} {}", dev, v),
                    )
                })?;

                Ok(DeviceLocation {
                    bus,
                    tree_positions,
                    number,
                })
            }
            (Some(reg), None) => {
                let reg = reg.trim().trim_start_matches("0x");
                let tree_positions: Vec<u8> = reg
                    .get(2..)
                    .unwrap_or("0")
                    .trim_end_matches('0')
                    .chars()
                    .map(|v| v.to_digit(10).unwrap_or(0) as u8)
                    .collect();
                // bus no is msb
                let bus = (u32::from_str_radix(reg, 16)
                    .map_err(|v| Error::new(ErrorKind::Parsing, &v.to_string()))?
                    >> 24) as u8;
                // make it up but unique
                let number = tree_positions.iter().sum();
                Ok(DeviceLocation {
                    bus,
                    tree_positions,
                    number,
                })
            }
            _ => Err(Error::new(ErrorKind::Parsing, "Invalid location_id")),
        }
    }
}

impl From<DeviceLocation> for PortPath {
    fn from(location: DeviceLocation) -> Self {
        PortPath::new(location.bus, location.tree_positions)
    }
}

impl<'de> Deserialize<'de> for DeviceLocation {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Bus,
            Number,
            TreePositions,
        }
        struct DeviceLocationVisitor;

        impl<'de> Visitor<'de> for DeviceLocationVisitor {
            type Value = DeviceLocation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with 0xLOCATION_REG/DEVICE_NO")
            }

            fn visit_seq<V>(self, mut seq: V) -> core::result::Result<DeviceLocation, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let bus = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let tree_positions = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let number = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                Ok(DeviceLocation {
                    bus,
                    number,
                    tree_positions,
                })
            }

            fn visit_map<V>(self, mut map: V) -> core::result::Result<DeviceLocation, V::Error>
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
                let tree_positions =
                    tree_positions.ok_or_else(|| de::Error::missing_field("tree_positions"))?;
                Ok(DeviceLocation {
                    bus,
                    number,
                    tree_positions,
                })
            }

            fn visit_string<E>(self, value: String) -> core::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                DeviceLocation::from_str(value.as_str()).map_err(serde::de::Error::custom)
            }

            fn visit_str<E>(self, value: &str) -> core::result::Result<Self::Value, E>
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
                    write!(f, "{:5} {:4}", "-", "-")
                }
            }
        }
    }
}

impl FromStr for DeviceSpeed {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        // try to match speed enum else provide string description provided in system_profiler dump
        match Speed::from_str(s) {
            Ok(v) => Ok(DeviceSpeed::SpeedValue(v)),
            Err(_) => Ok(DeviceSpeed::Description(s.to_owned())),
        }
    }
}

/// Events used by the watch feature
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DeviceEvent {
    /// Device profiled at time
    Profiled(chrono::DateTime<chrono::Local>),
    /// Device connected at time
    Connected(chrono::DateTime<chrono::Local>),
    /// Device disconnected at time
    Disconnected(chrono::DateTime<chrono::Local>),
}

impl fmt::Display for DeviceEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceEvent::Profiled(t) => write!(f, "P: {}", t.format("%y-%m-%d %H:%M:%S")),
            DeviceEvent::Connected(t) => write!(f, "C: {}", t.format("%y-%m-%d %H:%M:%S")),
            DeviceEvent::Disconnected(t) => {
                write!(f, "D: {}", t.format("%y-%m-%d %H:%M:%S"))
            }
        }
    }
}

impl Default for DeviceEvent {
    fn default() -> Self {
        DeviceEvent::Profiled(chrono::Local::now())
    }
}

impl DeviceEvent {
    /// Get the time of the event
    pub fn time(&self) -> chrono::DateTime<chrono::Local> {
        match self {
            DeviceEvent::Profiled(t) => *t,
            DeviceEvent::Connected(t) => *t,
            DeviceEvent::Disconnected(t) => *t,
        }
    }

    /// Format the event time using the provided format string
    pub fn format(&self, fmt: &str) -> String {
        self.time().format(fmt).to_string()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
/// Internal data used by cyme for display
pub struct InternalData {
    pub(crate) expanded: bool,
    pub(crate) hidden: bool,
}

/// USB device data based on JSON object output from system_profiler but now used for other platforms
///
/// Designed to hold static data for the device, obtained from system_profiler Deserializer or cyme::lsusb. Fields should probably be non-pub with getters/setters but treat them as read-only.
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Device {
    /// The device product name as reported in descriptor or using usb_ids if None
    #[serde(rename(deserialize = "_name"), alias = "name")]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// Unique vendor identifier - purchased from USB IF
    pub vendor_id: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// Vendor unique product identifier
    pub product_id: Option<u16>,
    /// [`DeviceLocation`] information of position within bus
    pub location_id: DeviceLocation,
    /// Device serial number as reported by descriptor
    pub serial_num: Option<String>,
    /// The device manufacturer as provided in descriptor or using usb_ids if None
    pub manufacturer: Option<String>,
    #[serde(
        default,
        serialize_with = "version_serializer",
        deserialize_with = "deserialize_option_version_from_string"
    )]
    /// The device release number set by the developer as a [`Version`]
    pub bcd_device: Option<Version>,
    #[serde(
        default,
        serialize_with = "version_serializer",
        deserialize_with = "deserialize_option_version_from_string"
    )]
    /// The highest version of USB the device supports as a [`Version`]
    pub bcd_usb: Option<Version>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// macOS system_profiler only - actually bus current in mA not power!
    pub bus_power: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// macOS system_profiler only - actually bus current used in mA not power!
    pub bus_power_used: Option<u16>,
    /// Advertised device capable speed
    ///
    /// See [`DeviceExtra`] for negotiated speed
    pub device_speed: Option<DeviceSpeed>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    /// macOS system_profiler only - actually bus current used in mA not power!
    pub extra_current_used: Option<u16>,
    /// Devices can be hub and have devices attached so need to walk each device's devices...
    #[serde(rename(deserialize = "_items"), alias = "devices")]
    pub devices: Option<Vec<Device>>,
    // below are not in macOS system profiler but useful enough to have outside of extra
    /// USB device class
    pub class: Option<BaseClass>,
    /// USB sub-class
    pub sub_class: Option<u8>,
    /// USB protocol
    pub protocol: Option<u8>,
    /// Extra data obtained by libusb/udev exploration
    #[serde(default)]
    pub extra: Option<DeviceExtra>,
    /// Internal to store any non-critical errors captured whilst profiling, unable to open for example
    #[serde(skip)]
    pub profiler_error: Option<String>,
    /// Unique ID assigned by system
    #[serde(skip)]
    #[cfg(feature = "nusb")]
    pub id: Option<::nusb::DeviceId>,
    /// Last event that occurred on device
    pub last_event: Option<DeviceEvent>,
    /// Internal data for cyme
    #[serde(skip)]
    pub internal: InternalData,
}

/// Deprecated alias for [`Device`]
#[deprecated(since = "2.0.0", note = "Use Device instead")]
pub type USBDevice = Device;

#[cfg(feature = "nusb")]
impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        (self.id == other.id) && self.id.is_some()
    }
}

impl Device {
    /// Does the device have child devices; `devices` is Some and > 0
    pub fn has_devices(&self) -> bool {
        match &self.devices {
            Some(d) => !d.is_empty() && !d.iter().all(|dd| dd.internal.hidden),
            None => false,
        }
    }

    /// Returns total number of devices in the tree including self
    fn len(&self) -> usize {
        1 + self
            .devices
            .as_ref()
            .map_or(0, |d| d.iter().map(|dd| dd.len()).sum())
    }

    /// Does the device have an interface with `class`
    pub fn has_interface_class(&self, c: &BaseClass) -> bool {
        if let Some(extra) = self.extra.as_ref() {
            extra
                .configurations
                .iter()
                .any(|conf| conf.interfaces.iter().any(|i| i.class == *c))
        } else {
            false
        }
    }

    /// Gets root_hub [`Device`] if it is one
    pub fn get_root_hub(&self) -> Option<&Device> {
        if self.is_root_hub() {
            Some(self)
        } else {
            None
        }
    }

    /// Gets mutable root_hub [`Device`] if it is one
    pub fn get_root_hub_mut(&mut self) -> Option<&mut Device> {
        if self.is_root_hub() {
            Some(self)
        } else {
            None
        }
    }

    /// Recursively walk all [`Device`] from self, looking for the one with [`PortPath`] and returning reference
    pub fn get_node(&self, port_path: &PortPath) -> Option<&Device> {
        if port_path.is_root_hub() {
            return self.get_root_hub();
        }
        let search_depth = port_path.depth();
        let current_depth = self.get_depth();
        let self_port_path = self.port_path();
        log::debug!(
            "Get node @ {}: exploring {} ({}); depth {}/{}",
            port_path,
            self_port_path,
            self,
            current_depth,
            search_depth
        );

        // should not be looking for nodes below us unless root
        match current_depth.cmp(&search_depth) {
            Ordering::Greater => return None,
            Ordering::Equal => {
                if self_port_path == *port_path {
                    return Some(self);
                } else {
                    return None;
                }
            }
            Ordering::Less => {}
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

    /// Recursively walk all [`Device`] from self, looking for the one with [`PortPath`] and returning mutable
    pub fn get_node_mut(&mut self, port_path: &PortPath) -> Option<&mut Device> {
        if port_path.is_root_hub() {
            return self.get_root_hub_mut();
        }
        let search_depth = port_path.depth();
        let current_depth = self.get_depth();
        let self_port_path = self.port_path();
        log::debug!(
            "Get node @ {}: exploring {} ({}); depth {}/{}",
            port_path,
            self_port_path,
            self,
            current_depth,
            search_depth
        );

        // should not be looking for nodes below us
        match current_depth.cmp(&search_depth) {
            Ordering::Greater => return None,
            Ordering::Equal => {
                if self_port_path == *port_path {
                    return Some(self);
                } else {
                    return None;
                }
            }
            Ordering::Less => {}
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

    /// Find the [`Device`] by it's Path
    pub fn get_node_by_path<P: AsRef<Path>>(&self, path: P) -> Option<&Device> {
        UsbPath::from(path.as_ref())
            .port_path()
            .and_then(|pp| self.get_node(&pp))
    }

    /// Find the mutable [`Device`] by it's Path
    pub fn get_node_mut_by_path<P: AsRef<Path>>(&mut self, path: P) -> Option<&mut Device> {
        UsbPath::from(path.as_ref())
            .port_path()
            .and_then(|pp| self.get_node_mut(&pp))
    }

    /// Search for reference to [`Device`] at port path as string
    pub fn get_node_by_str(&self, path: &str) -> Option<&Device> {
        PortPath::try_from(path)
            .ok()
            .and_then(|pp| self.get_node(&pp))
    }

    /// Search for mutable reference to [`Device`] at port path as string
    pub fn get_node_str_mut(&mut self, path: &str) -> Option<&mut Device> {
        PortPath::try_from(path)
            .ok()
            .and_then(|pp| self.get_node_mut(&pp))
    }

    /// Get the [`Configuration`] with number `config` from the device's extra data
    pub fn get_config(&self, config: u8) -> Option<&Configuration> {
        self.extra
            .as_ref()
            .and_then(|e| e.configurations.iter().find(|c| c.number == config))
    }

    /// Get the mutable [`Configuration`] with number `config` from the device's extra data
    pub fn get_config_mut(&mut self, config: u8) -> Option<&mut Configuration> {
        self.extra
            .as_mut()
            .and_then(|e| e.configurations.iter_mut().find(|c| c.number == config))
    }

    /// Get the [`Interface`] with number `interface` from the device's extra data
    pub fn get_interface(&self, config: u8, interface: u8, alt_setting: u8) -> Option<&Interface> {
        self.get_config(config).and_then(|c| {
            c.interfaces
                .iter()
                .find(|i| i.number == interface && i.alt_setting == alt_setting)
        })
    }

    /// Get the mutable [`Interface`] with number `interface` from the device's extra data
    pub fn get_interface_mut(
        &mut self,
        config: u8,
        interface: u8,
        alt_setting: u8,
    ) -> Option<&mut Interface> {
        self.get_config_mut(config).and_then(|c| {
            c.interfaces
                .iter_mut()
                .find(|i| i.number == interface && i.alt_setting == alt_setting)
        })
    }

    /// Get the [`Endpoint`] with number `endpoint` from the device's extra data
    pub fn get_endpoint(
        &self,
        config: u8,
        interface: u8,
        alt_setting: u8,
        endpoint_address: u8,
    ) -> Option<&Endpoint> {
        self.get_interface(config, interface, alt_setting)
            .and_then(|i| {
                i.endpoints
                    .iter()
                    .find(|e| e.address.address == endpoint_address)
            })
    }

    /// Get the mutable [`Endpoint`] with number `endpoint` from the device's extra data
    pub fn get_endpoint_mut(
        &mut self,
        config: u8,
        interface: u8,
        alt_setting: u8,
        endpoint_address: u8,
    ) -> Option<&mut Endpoint> {
        self.get_interface_mut(config, interface, alt_setting)
            .and_then(|i| {
                i.endpoints
                    .iter_mut()
                    .find(|e| e.address.address == endpoint_address)
            })
    }

    /// Returns position on branch (parent), which is the last number in `tree_positions` also sometimes referred to as port
    pub fn get_branch_position(&self) -> u8 {
        // root hub could be [] or [0] but we want to return 0
        self.location_id.tree_positions.last().copied().unwrap_or(0)
    }

    /// The number of [`Device`] deep; branch depth
    pub fn get_depth(&self) -> usize {
        self.location_id.tree_positions.len()
    }

    /// Returns `true` if device is a hub based on device name - not perfect but most hubs advertise as a hub in name - or class code if it has one
    pub fn is_hub(&self) -> bool {
        self.name.to_lowercase().contains("hub")
            || self.class.as_ref().is_some_and(|c| *c == BaseClass::Hub)
    }

    /// [`PortPath`] of the [`Device`]
    pub fn port_path(&self) -> PortPath {
        self.location_id.clone().into()
    }

    /// Linux style port path where it can be found on system device path - normally /sys/bus/usb/devices
    pub fn sysfs_path(&self) -> PathBuf {
        UsbPath::from(self.port_path()).to_sysfs_path()
    }

    /// [`PortPath`] of parent [`Device`]; one above in tree
    pub fn parent_port_path(&self) -> Option<PortPath> {
        self.port_path().parent()
    }

    /// [`PortPath`] of trunk [`Device`]; first in tree
    pub fn trunk_port_path(&self) -> PortPath {
        self.port_path().trunk()
    }

    /// Linux devpath to [`Device`]
    pub fn dev_path(&self) -> PathBuf {
        get_dev_path(self.location_id.bus, Some(self.location_id.number))
    }

    /// Linux sysfs name of [`Device`]
    pub fn sysfs_name(&self) -> String {
        get_sysfs_name(self.location_id.bus, &self.location_id.tree_positions)
    }

    /// Trunk device is first in tree
    pub fn is_trunk_device(&self) -> bool {
        self.location_id.tree_positions.len() == 1
    }

    /// Root hub is a specific device on Linux, essentially the bus but sits in device tree because of system_profiler legacy
    pub fn is_root_hub(&self) -> bool {
        self.location_id.tree_positions.is_empty() || self.location_id.tree_positions == [0]
    }

    /// From lsusb.c: Attempt to get friendly vendor and product names from the udev hwdb. If either or both are not present, instead populate those from the device's own string descriptors
    pub fn get_vendor_product_with_fallback(&self) -> (String, String) {
        match &self.extra {
            Some(v) => (
                v.vendor
                    .to_owned()
                    .unwrap_or(self.manufacturer.to_owned().unwrap_or_default()),
                v.product_name
                    .to_owned()
                    .unwrap_or(self.name.trim().to_owned()),
            ),
            None => (
                self.manufacturer.to_owned().unwrap_or_default(),
                self.name.trim().to_owned(),
            ),
        }
    }

    /// Generate a String from self like lsusb default list device
    /// ```
    /// let d = cyme::profiler::Device{
    ///     name: String::from("Test device"),
    ///     manufacturer: Some(String::from("Test Devices Inc.")),
    ///     vendor_id: Some(0x1234),
    ///     product_id: Some(0x4321),
    ///     location_id: cyme::profiler::DeviceLocation { bus: 1, number: 4, tree_positions: vec![1, 2, 3] },
    ///     ..Default::default()
    ///     };
    /// assert_eq!(d.to_lsusb_string(), "Bus 001 Device 004: ID 1234:4321 Test Devices Inc. Test device");
    /// ```
    pub fn to_lsusb_string(&self) -> String {
        let (vendor, product) = self.get_vendor_product_with_fallback();
        format!(
            "Bus {:03} Device {:03}: ID {:04x}:{:04x} {} {}",
            self.location_id.bus,
            self.location_id.number,
            self.vendor_id.unwrap_or(0xffff),
            self.product_id.unwrap_or(0xffff),
            vendor,
            product,
        )
    }

    /// Generate a Vec of Vec<Strings> from self like lsusb tree mode in order of verbosity
    ///
    /// Each device contains of Vec<String> of interfaces
    pub fn to_lsusb_tree_strings(&self, verbosity: u8) -> Vec<Vec<String>> {
        let mut ret = Vec::new();
        let speed = match &self.device_speed {
            Some(v) => match v {
                DeviceSpeed::SpeedValue(v) => v.to_lsusb_speed(),
                DeviceSpeed::Description(_) => String::new(),
            },
            None => String::from(""),
        };

        // no fallback for lsusb tree mode
        let (driver, vendor, product) = match &self.extra {
            Some(v) => (
                v.driver.to_owned().unwrap_or(String::from("[none]")),
                v.vendor.to_owned().unwrap_or(String::from("[unknown]")),
                v.product_name
                    .to_owned()
                    .unwrap_or(String::from("[unknown]")),
            ),
            None => (
                String::from("[none]"),
                String::from("[unknown]"),
                String::from("[unknown]"),
            ),
        };

        let get_istrings = || match (
            &self.name,
            self.manufacturer.as_ref(),
            self.serial_num.as_ref(),
        ) {
            (n, Some(m), Some(s)) => {
                format!("Manufactuer={} Product={} Serial={}", n, m, s)
            }
            (n, Some(m), None) => {
                format!("Manufactuer={} Product={}", n, m)
            }
            (n, None, Some(s)) => {
                format!("Product={} Serial={}", n, s)
            }
            (n, None, None) => {
                format!("Product={}", n)
            }
        };

        if let Some(extra) = self.extra.as_ref() {
            let ports = extra.hub.as_ref().map(|hub| hub.num_ports);
            for config in &extra.configurations {
                for interface in &config.interfaces {
                    let mut device_strings = Vec::with_capacity(verbosity as usize);
                    let interface_driver = interface
                        .driver
                        .as_ref()
                        .map_or(String::from("[none]"), |d| d.to_string());
                    // if there are ports (device is hub), add them to the driver string
                    let driver_string = if let Some(p) = ports {
                        format!("{}/{}p", interface_driver, p)
                    } else {
                        interface_driver
                    };
                    device_strings.push(format!(
                        "Port {:03}: Dev {:03}, If {}, Class={}, Driver={}, {}",
                        self.get_branch_position(),
                        self.location_id.number,
                        interface.number,
                        interface.class.to_lsusb_string(),
                        driver_string,
                        speed
                    ));
                    if verbosity > 0 {
                        device_strings.push(format!(
                            "ID {:04x}:{:04x} {} {}",
                            self.vendor_id.unwrap_or(0xFFFF),
                            self.product_id.unwrap_or(0xFFFF),
                            vendor,
                            product,
                        ));
                    }
                    if verbosity > 1 {
                        device_strings.push(format!(
                            "{}/{}  {}",
                            "/sys/bus/usb/devices",
                            self.port_path(),
                            self.dev_path().display(),
                        ));
                    }
                    if verbosity > 2 {
                        device_strings.push(get_istrings());
                    }
                    ret.push(device_strings);
                }
            }
        } else {
            log::warn!("Rendering {} lsusb tree without extra data because it is missing. No configurations or interfaces will be shown", self);
            let mut device_strings = Vec::with_capacity(verbosity as usize);
            device_strings.push(format!(
                "Port {:03}: Dev {:03}, If {}, Class={}, Driver={}, {}",
                self.get_branch_position(),
                self.location_id.number,
                0,
                self.class
                    .as_ref()
                    .map_or(String::from("[unknown]"), |c| c.to_lsusb_string()),
                driver,
                speed
            ));
            if verbosity > 0 {
                device_strings.push(format!(
                    "ID {:04x}:{:04x} {} {}",
                    self.vendor_id.unwrap_or(0xFFFF),
                    self.product_id.unwrap_or(0xFFFF),
                    // these are actually usb_ids vendor/product but don't have those without extra
                    self.manufacturer
                        .as_ref()
                        .unwrap_or(&String::from("[unknown]")),
                    self.name,
                ));
            }
            if verbosity > 1 {
                device_strings.push(format!(
                    "{}/{}  {}",
                    "/sys/bus/usb/devices",
                    self.port_path(),
                    self.dev_path().display(),
                ));
            }
            if verbosity > 2 {
                device_strings.push(get_istrings());
            }
            ret.push(device_strings);
        }

        ret
    }

    /// Gets the base class code byte from [`BaseClass`]
    pub fn base_class_code(&self) -> Option<u8> {
        self.class.as_ref().map(|c| u8::from(*c))
    }

    /// Name of class from Linux USB IDs repository
    pub fn class_name(&self) -> Option<&str> {
        match self.base_class_code() {
            Some(cid) => usb_ids::Classes::iter()
                .find(|c| c.id() == cid)
                .map(|c| c.name()),
            None => None,
        }
    }

    /// Name of sub class from Linux USB IDs repository
    pub fn sub_class_name(&self) -> Option<&str> {
        match (self.base_class_code(), self.sub_class) {
            (Some(cid), Some(sid)) => {
                usb_ids::SubClass::from_cid_scid(cid, sid).map(|sc| sc.name())
            }
            _ => None,
        }
    }

    /// Name of protocol from Linux USB IDs repository
    pub fn protocol_name(&self) -> Option<&str> {
        match (self.base_class_code(), self.sub_class, self.protocol) {
            (Some(cid), Some(sid), Some(pid)) => {
                usb_ids::Protocol::from_cid_scid_pid(cid, sid, pid).map(|p| p.name())
            }
            _ => None,
        }
    }

    /// Returns fully defined USB [`Class`] based on base_class, sub_class and protocol triplet
    pub fn fully_defined_class(&self) -> Option<ClassCode> {
        self.class
            .map(|c| (c, self.sub_class.unwrap_or(0), self.protocol.unwrap_or(0)).into())
    }

    /// Recursively gets all devices in a [`Device`] and flattens them into a Vec of references, including self
    pub fn flatten(&self) -> Vec<&Device> {
        let mut ret: Vec<&Device> = Vec::with_capacity(self.len());
        ret.push(self);
        if let Some(d) = self.devices.as_ref() {
            for child in d {
                ret.extend(child.flatten());
            }
        }

        ret
    }

    /// Recursively gets all devices in a [`Device`] and flattens them into a Vec of mutable references, **excluding** self
    pub fn flatten_mut(&mut self) -> Vec<&mut Device> {
        if let Some(d) = self.devices.as_mut() {
            d.iter_mut()
                .flat_map(|d| d.flatten_mut())
                .collect::<Vec<&mut Device>>()
        } else {
            Vec::new()
        }
    }

    /// Recursively gets all devices in a [`Device`] and flattens them into a Vec, including self
    ///
    /// Similar to `flatten` but flattens in place rather than returning references so is destructive
    pub fn into_flattened(mut self) -> Vec<Device> {
        let mut ret: Vec<Device> = Vec::with_capacity(self.len());
        if let Some(mut d) = self.devices.take() {
            while let Some(child) = d.pop() {
                ret.extend(child.into_flattened());
            }
        }

        ret.insert(0, self);

        ret
    }

    /// Recursively searches for a device with a specific [`::nusb::DeviceId`] and returns a reference
    #[cfg(feature = "nusb")]
    pub fn get_id(&self, id: &::nusb::DeviceId) -> Option<&Self> {
        if self.id == Some(*id) {
            return Some(self);
        }
        if let Some(devices) = self.devices.as_ref() {
            for dev in devices {
                if let Some(node) = dev.get_id(id) {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Recursively searches for a device with a specific [`::nusb::DeviceId`] and returns a mutable reference
    #[cfg(feature = "nusb")]
    pub fn get_id_mut(&mut self, id: &::nusb::DeviceId) -> Option<&mut Self> {
        if self.id == Some(*id) {
            return Some(self);
        }
        if let Some(devices) = self.devices.as_mut() {
            for dev in devices {
                if let Some(node) = dev.get_id_mut(id) {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Get last event that occurred on device
    pub fn last_event(&self) -> Option<DeviceEvent> {
        self.last_event
    }

    /// Has the device disconnected based last event being disconnected
    ///
    /// Logic rather than is_connected since Profiled event is not certain still present
    pub fn is_disconnected(&self) -> bool {
        matches!(self.last_event, Some(DeviceEvent::Disconnected(_)))
    }

    /// Should the device be hidden when printing
    pub fn is_hidden(&self) -> bool {
        self.internal.hidden
    }

    /// Should the device be displayed expanded in a tree
    pub fn is_expanded(&self) -> bool {
        self.internal.expanded
    }

    /// Toggle the expanded state of the device
    pub fn toggle_expanded(&mut self) {
        self.internal.expanded = !self.internal.expanded;
    }

    /// Set the expanded state of the device and all configurations, interfaces and endpoints
    pub fn set_all_expanded(&mut self, expanded: bool) {
        self.internal.expanded = expanded;
        if let Some(extra) = self.extra.as_mut() {
            for config in &mut extra.configurations {
                config.set_all_expanded(expanded);
            }
        }
    }
}

impl fmt::Display for Device {
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
        } else if f.alternate() {
            "\u{2514}\u{2500}\u{2500} "
        } else {
            "|__ "
        };

        // alternate for coloured, slightly different format to lsusb
        if f.alternate() {
            write!(
                f,
                "{:>spaces$}{}/{} {}:{} {} {} {}",
                tree.bright_black(),
                format!("{:03}", self.location_id.bus).cyan(),
                format!("{:03}", self.location_id.number).magenta(),
                format!("0x{:04x}", self.vendor_id.unwrap_or(0))
                    .yellow()
                    .bold(),
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
                let interface_strs: Vec<String> = self
                    .to_lsusb_tree_strings(0)
                    .iter()
                    .map(|s| format!("{:>spaces$}{}", tree, s[0]))
                    .collect();
                write!(f, "{}", interface_strs.join("\n\r"))
            } else {
                write!(f, "{}", self.to_lsusb_string())
            }
        }
    }
}

impl<'a> IntoIterator for &'a Device {
    type Item = &'a Device;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> std::vec::IntoIter<Self::Item> {
        if let Some(d) = self.devices.as_ref() {
            d.iter()
                .flat_map(|d| d.flatten())
                .collect::<Vec<&Device>>()
                .into_iter()
        } else {
            Vec::new().into_iter()
        }
    }
}

impl<'a> IntoIterator for &'a mut Device {
    type Item = &'a mut Device;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> std::vec::IntoIter<Self::Item> {
        self.flatten_mut().into_iter()
    }
}

/// Used to filter devices within buses
///
/// The tree to a [`Device`] is kept even if parent branches are not matches. To avoid this, one must flatten the devices first.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Filter {
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
    /// retain only devices with serial.contains(serial)
    pub serial: Option<String>,
    /// retain only device of BaseClass class
    pub class: Option<BaseClass>,
    /// Exclude empty buses in the tree
    pub exclude_empty_bus: bool,
    /// Exclude empty hubs in the tree
    pub exclude_empty_hub: bool,
    /// Don't exclude Linux root_hub devices - this is inverse because they are pseudo [`Bus`]'s in the tree
    pub no_exclude_root_hub: bool,
    /// Case sensitive matching for strings. False will be unless capital letter in query
    pub case_sensitive: bool,
}

/// Deprecated alias for [`Filter`]
#[deprecated(since = "2.0.0", note = "Use Filter instead")]
pub type USBFilter = Filter;

/// Filter devices with name
///
/// ```
/// use cyme::profiler::*;
///
/// # let mut spusb = read_json_dump(&"./tests/data/system_profiler_dump.json").unwrap();
/// let filter = Filter {
///     name: Some(String::from("Black Magic Probe")),
///     ..Default::default()
/// };
/// filter.retain_buses(&mut spusb.buses);
/// let flattened = spusb.flattened_devices();
/// // node was on a hub so that will remain with it
/// assert_eq!(flattened.len(), 2);
/// // get the node from path known before for purpose of test
/// let device = spusb.get_node_by_str("20-3.3");
/// assert_eq!(device.unwrap().name, "Black Magic Probe  v1.8.2");
/// ```
///
/// Filter devices with vid and pid
/// ```
/// use cyme::profiler::*;
///
/// # let mut spusb = read_json_dump(&"./tests/data/system_profiler_dump.json").unwrap();
/// let filter = Filter {
///     vid: Some(0x1d50),
///     pid: Some(0x6018),
///     ..Default::default()
/// };
/// filter.retain_buses(&mut spusb.buses);
/// let flattened = spusb.flattened_devices();
/// // node was on a hub so that will remain with it
/// assert_eq!(flattened.len(), 2);
/// // get the node from path known before for purpose of test
/// let device = spusb.get_node_by_str("20-3.3");
/// assert_eq!(device.unwrap().vendor_id.unwrap(), 0x1d50);
/// ```
///
/// Filter a flattened tree to exclude hubs
///
/// ```
/// use cyme::profiler::*;
///
/// # let mut spusb = read_json_dump(&"./tests/data/system_profiler_dump.json").unwrap();
/// let filter = Filter {
///     number: Some(6),
///     bus: Some(20),
///     ..Default::default()
/// };
/// let mut flattened = spusb.flattened_devices();
/// filter.retain_flattened_devices_ref(&mut flattened);
/// // now no hub
/// assert_eq!(flattened.len(), 1);
/// assert_eq!(flattened.first().unwrap().name, "Black Magic Probe  v1.8.2");
/// ```
///
/// Filter devices with class
///
/// ```
/// use cyme::profiler::*;
///
/// # let mut spusb = read_json_dump(&"./tests/data/cyme_libusb_merge_macos_tree.json").unwrap();
/// let filter = Filter {
///     class: Some(cyme::usb::BaseClass::CdcCommunications),
///     ..Default::default()
/// };
/// let mut flattened = spusb.flattened_devices();
/// filter.retain_flattened_devices_ref(&mut flattened);
/// // black magic probe has CDCCommunications serial
/// let device = spusb.get_node_by_str("20-3.3");
/// assert_eq!(device.unwrap().name, "Black Magic Probe  v1.8.2");
/// ```
///
impl Filter {
    /// Creates a new filter with defaults
    pub fn new() -> Self {
        Default::default()
    }

    fn string_match(&self, pattern: &Option<String>, query: Option<&String>) -> bool {
        if let (Some(p), Some(q)) = (pattern, query) {
            let case_sensitive = if !self.case_sensitive {
                p.chars().any(|c| c.is_uppercase())
            } else {
                self.case_sensitive
            };

            if !case_sensitive {
                q.to_lowercase().contains(p.to_lowercase().as_str())
            } else {
                q.contains(p.as_str())
            }
        } else {
            true
        }
    }

    /// Checks whether `device` passes through filter
    pub fn is_match(&self, device: &Device) -> bool {
        (Some(device.location_id.bus) == self.bus || self.bus.is_none())
            && (Some(device.location_id.number) == self.number || self.number.is_none())
            && (device.vendor_id == self.vid || self.vid.is_none())
            && (device.product_id == self.pid || self.pid.is_none())
            && (self.string_match(&self.name, Some(&device.name)))
            && (self.string_match(&self.serial, device.serial_num.as_ref()))
            && self.class.as_ref().is_none_or(|fc| {
                device.class.as_ref() == Some(fc) || device.has_interface_class(fc)
            })
            && !(self.exclude_empty_hub && device.is_hub() && !device.has_devices())
            && (!device.is_root_hub() || self.no_exclude_root_hub)
    }

    /// Checks whether `bus` passes through filter
    pub fn is_bus_match(&self, bus: &Bus) -> bool {
        (bus.usb_bus_number == self.bus || self.bus.is_none() || bus.usb_bus_number.is_none())
            && !(self.exclude_empty_bus && bus.is_empty())
    }

    /// Recursively looks down tree for any `device` matching filter
    ///
    /// Important because a simple check of trunk device might remove a matching device further down the tree
    pub fn exists_in_tree(&self, device: &Device) -> bool {
        // if device itself is a match, just return now and don't bother going keeper
        if self.is_match(device) {
            return true;
        }

        match &device.devices {
            Some(devs) => devs.iter().any(|d| self.exists_in_tree(d)),
            None => false,
        }
    }

    /// Recursively retain only `Bus` in `buses` with `Device` matching filter
    pub fn retain_buses(&self, buses: &mut Vec<Bus>) {
        // filter any empty or number matches
        buses.retain(|b| self.is_bus_match(b));

        for bus in buses.iter_mut() {
            bus.devices.iter_mut().for_each(|d| self.retain_devices(d));
        }

        // check bus match again in case empty after device filter
        buses.retain(|b| self.is_bus_match(b));
    }

    /// Recursively hide `Bus` in `buses` with `Device` matching filter
    pub fn hide_buses(&self, buses: &mut [Bus]) {
        buses
            .iter_mut()
            .for_each(|b| b.internal.hidden = !self.is_bus_match(b));

        for bus in buses.iter_mut() {
            bus.devices.iter_mut().for_each(|d| self.hide_devices(d));
        }

        buses
            .iter_mut()
            .for_each(|b| b.internal.hidden = !self.is_bus_match(b));
    }

    /// Recursively retain only `Device` in `devices` matching filter
    ///
    /// Note that non-matching parents will still be retained if they have a matching `Device` within their branches
    pub fn retain_devices(&self, devices: &mut Vec<Device>) {
        devices.retain(|d| self.exists_in_tree(d));

        for d in devices {
            d.devices.iter_mut().for_each(|d| self.retain_devices(d));
        }
    }

    /// Recursively retain only `Device` in `devices` matching filter
    pub fn hide_devices(&self, devices: &mut [Device]) {
        devices
            .iter_mut()
            .for_each(|d| d.internal.hidden = !self.exists_in_tree(d));

        for d in devices {
            d.devices.iter_mut().for_each(|d| self.hide_devices(d));
        }
    }

    /// Retains only `&Device` in `devices` which match filter
    ///
    /// Does not check down tree so should be used to flattened devices only (`get_all_devices`). Will remove hubs if `hide_hubs` since when flattened they will have no devices
    pub fn retain_flattened_devices_ref(&self, devices: &mut Vec<&Device>) {
        devices.retain(|d| self.is_match(d))
    }
}

/// Reads a json dump at `file_path` with serde deserializer - either from `system_profiler` or from `cyme --json`
///
/// Must be a full tree including buses. Use `read_flat_json_dump` for devices only
pub fn read_json_dump<P: AsRef<Path>>(file_path: P) -> Result<SystemProfile> {
    let mut file = fs::File::options().read(true).open(file_path.as_ref())?;

    let mut data = String::new();
    file.read_to_string(&mut data)?;

    let json_dump: SystemProfile = serde_json::from_str(&data).map_err(|e| {
        Error::new(
            ErrorKind::Parsing,
            &format!(
                "Failed to parse dump at {:?}; Error({})",
                file_path.as_ref().display(),
                e
            ),
        )
    })?;

    Ok(json_dump)
}

/// Reads a flat json dump (devices no buses) at `file_path` with serde deserializer - either from `system_profiler` or from `cyme --json`
pub fn read_flat_json_dump<P: AsRef<Path>>(file_path: P) -> Result<Vec<Device>> {
    let mut file = fs::File::options().read(true).open(file_path.as_ref())?;

    let mut data = String::new();
    file.read_to_string(&mut data)?;

    let json_dump: Vec<Device> = serde_json::from_str(&data).map_err(|e| {
        Error::new(
            ErrorKind::Parsing,
            &format!(
                "Failed to parse dump at {:?}; Error({})",
                file_path.as_ref().display(),
                e
            ),
        )
    })?;

    Ok(json_dump)
}

/// Reads a flat json dump (devices no buses) at `file_path` with serde deserializer from `cyme --json` and converts to `SPUSBDataType`
///
/// This is useful for converting a flat json dump to a full tree for use with `Filter`. Bus information is phony however.
pub fn read_flat_json_to_phony_bus<P: AsRef<Path>>(file_path: P) -> Result<SystemProfile> {
    let devices = read_flat_json_dump(file_path)?;
    let bus = Bus {
        name: String::from("Phony Flat JSON Import Bus"),
        host_controller: String::from("Phony Host Controller"),
        host_controller_vendor: None,
        host_controller_device: None,
        pci_device: None,
        pci_vendor: None,
        pci_revision: None,
        usb_bus_number: None,
        devices: Some(devices),
        ..Default::default()
    };

    Ok(SystemProfile { buses: vec![bus] })
}

/// Deserializes an option number from String (base10 or base16 encoding) or a number
///
/// Modified from https://github.com/vityafx/serde-aux/blob/master/src/field_attributes.rs with addition of base16 encoding
fn deserialize_option_number_from_string<'de, T, D>(
    deserializer: D,
) -> core::result::Result<Option<T>, D::Error>
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
                let vendor_vec: Vec<&str> = s.split(' ').collect();

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

fn deserialize_option_version_from_string<'de, D>(
    deserializer: D,
) -> core::result::Result<Option<Version>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum VersionOrNull {
        #[serde(deserialize_with = "deserialize_version")]
        From(Version),
        Null,
    }

    match VersionOrNull::deserialize(deserializer)? {
        VersionOrNull::From(i) => Ok(Some(i)),
        VersionOrNull::Null => Ok(None),
    }
}

fn deserialize_version<'de, D>(deserializer: D) -> core::result::Result<Version, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct VersionVisitor;
    impl serde::de::Visitor<'_> for VersionVisitor {
        type Value = Version;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("BCD version base16 encoding [MM.mP] where MM is Major, m is Minor and P is sub-minor")
        }

        fn visit_f32<E>(self, value: f32) -> core::result::Result<Version, E>
        where
            E: serde::de::Error,
        {
            Version::try_from(value)
                .map_err(|_| E::invalid_value(serde::de::Unexpected::Float(value.into()), &self))
        }

        fn visit_str<E>(self, value: &str) -> core::result::Result<Version, E>
        where
            E: serde::de::Error,
        {
            Version::from_str(value)
                .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
        }
    }

    deserializer.deserialize_any(VersionVisitor)
}

fn version_serializer<S>(version: &Option<Version>, s: S) -> core::result::Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    match version {
        Some(v) => s.serialize_str(&v.to_string()),
        None => s.serialize_none(),
    }
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

        let device: Device = serde_json::from_str(device_json).unwrap();

        assert_eq!(device.name, "Arduino Zero");
        assert_eq!(device.bcd_device, Some(Version(1, 0, 0)));
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

        let device: Bus = serde_json::from_str(device_json).unwrap();

        assert_eq!(device.name, "USB31Bus");
        assert_eq!(device.host_controller, "AppleUSBXHCITR");
        assert_eq!(device.pci_device, Some(0x15f0));
        assert_eq!(device.pci_revision, Some(0x0006));
        assert_eq!(device.pci_vendor, Some(0x8086));
        assert_eq!(device.usb_bus_number, Some(0x00));
    }

    #[test]
    fn test_json_dump_read_not_panic() {
        read_json_dump("./tests/data/system_profiler_dump.json").unwrap();
    }
}
