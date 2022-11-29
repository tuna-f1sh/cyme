//! Defines for USB, mainly thosed covered at [usb.org](https://www.usb.org)
//!
//! Also refering to [beyondlogic](https://beyondlogic.org/usbnutshell/usb5.shtml)
//!
//! There are some repeated/copied Enum defines from rusb in order to control Serialize/Deserialize and add impl
use itertools::Itertools;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::fmt;
use std::str::FromStr;

use crate::types::NumericalUnit;

/// Configuration attributes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigAttributes {
    /// Device powers itself not from bus
    SelfPowered,
    /// Supports remote wake-up
    RemoteWakeup,
}

impl fmt::Display for ConfigAttributes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ConfigAttributes {
    /// Converts a HashSet of [`ConfigAttributes`] into a ';' separated string
    pub fn attributes_to_string(attributes: &HashSet<ConfigAttributes>) -> String {
        let vec: Vec<String> = attributes.iter().map(|a| a.to_string()).collect();
        vec.join(";")
    }
}

/// Explains how the `ClassCode` is used
#[derive(Debug)]
pub enum DescriptorUsage {
    /// Describes device
    Device,
    /// Describes interface
    Interface,
    /// Can be used to describe both
    Both,
}

/// USB class code defines [ref](https://www.usb.org/defined-class-codes)
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum ClassCode {
    #[default]
    UseInterfaceDescriptor,
    Audio,
    CDCCommunications,
    HID,
    Physical,
    Image,
    Printer,
    MassStorage,
    Hub,
    CDCData,
    SmartCart,
    ContentSecurity,
    Video,
    PersonalHealthcare,
    AudioVideo,
    Billboard,
    USBTypeCBridge,
    I3CDevice,
    Diagnostic,
    WirelessController,
    Miscellaneous,
    ApplicationSpecificInterface,
    VendorSpecific,
}

impl fmt::Display for ClassCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<u8> for ClassCode {
    fn from(b: u8) -> ClassCode {
        match b {
            0 => ClassCode::UseInterfaceDescriptor,
            1 => ClassCode::Audio,
            2 => ClassCode::CDCCommunications,
            3 => ClassCode::HID,
            5 => ClassCode::Physical,
            6 => ClassCode::Image,
            7 => ClassCode::Printer,
            8 => ClassCode::MassStorage,
            9 => ClassCode::Hub,
            0x0a => ClassCode::CDCData,
            0x0b => ClassCode::SmartCart,
            0x0d => ClassCode::ContentSecurity,
            0x0e => ClassCode::Video,
            0x0f => ClassCode::PersonalHealthcare,
            0x10 => ClassCode::AudioVideo,
            0x11 => ClassCode::Billboard,
            0x12 => ClassCode::USBTypeCBridge,
            0x3c => ClassCode::I3CDevice,
            0xdc => ClassCode::Diagnostic,
            0xe0 => ClassCode::WirelessController,
            0xef => ClassCode::Miscellaneous,
            0xfe => ClassCode::ApplicationSpecificInterface,
            0xff => ClassCode::VendorSpecific,
            _ => ClassCode::UseInterfaceDescriptor,
        }
    }
}

impl ClassCode {
    /// How the ClassCode is used [`DescriptorUsage`]
    pub fn usage(&self) -> DescriptorUsage {
        match self {
            ClassCode::UseInterfaceDescriptor | ClassCode::Hub | ClassCode::Billboard => {
                DescriptorUsage::Device
            }
            ClassCode::CDCCommunications
            | ClassCode::Diagnostic
            | ClassCode::Miscellaneous
            | ClassCode::VendorSpecific => DescriptorUsage::Both,
            _ => DescriptorUsage::Interface,
        }
    }

    /// Converts Pascal case enum to space separated on capitals
    /// ```
    /// use cyme::usb::ClassCode;
    ///
    /// assert_eq!(ClassCode::UseInterfaceDescriptor.to_title_case(), "Use Interface Descriptor");
    /// assert_eq!(ClassCode::CDCData.to_title_case(), "CDC Data");
    /// ```
    pub fn to_title_case(&self) -> String {
        let title = heck::AsTitleCase(self.to_string()).to_string();
        let split: Vec<&str> = title.split(" ").collect();
        let first = split.first().unwrap_or(&"");

        // keep capitalised abbreviations
        match first.to_owned() {
            "Cdc"|"Usb"|"I3c"|"Hid" => title.replace(first, &first.to_uppercase()),
            _ => title
        }
    }
}

impl From<ClassCode> for DescriptorUsage {
    fn from(c: ClassCode) -> DescriptorUsage {
        return c.usage();
    }
}

/// USB Speed is also defined in libusb but this one allows us to provide updates and custom impl
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(untagged, rename_all = "snake_case")]
#[allow(missing_docs)]
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

/// Convert from byte returned from device descriptor
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
                value: 10.0,
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

impl Speed {
    /// lsusb speed is always in Mb/s and shown just a M prefix
    pub fn to_lsusb_speed(&self) -> String {
        let dv = NumericalUnit::<f32>::from(self);
        let prefix = dv.unit.chars().next().unwrap_or('M');
        match prefix {
            'G' => format!("{:.0}{}", dv.value * 1000.0, 'M'),
            _ => format!("{:.0}{}", dv.value, prefix)
        }
    }
}

/// Transfer and [`USBEndpoint`] direction
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Direction for write (host to device) transfers.
    Out,
    /// Direction for read (device to host) transfers.
    In
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Transfer type  for [`USBEndpoint`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferType {
    /// Control endpoint.
    Control,
    /// Isochronous endpoint.
    Isochronous,
    /// Bulk endpoint.
    Bulk,
    /// Interrupt endpoint.
    Interrupt,
}

impl fmt::Display for TransferType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Isochronous synchronization mode for [`USBEndpoint`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncType {
    /// No synchronisation.
    None,
    /// Asynchronous.
    Asynchronous,
    /// Adaptive.
    Adaptive,
    /// Synchronous.
    Synchronous,
}

impl fmt::Display for SyncType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Isochronous usage type for [`USBEndpoint`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsageType {
    /// Data endpoint.
    Data,
    /// Feedback endpoint.
    Feedback,
    /// Explicit feedback data endpoint.
    FeedbackData,
    /// Reserved.
    Reserved,
}

impl fmt::Display for UsageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Address information for a [`USBEndpoint`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointAddress {
    /// Endpoint address byte
    pub address: u8,
    /// Endpoint number on [`USBInterface`] 0..3b
    pub number: u8,
    /// Data transfer direction 7b
    pub direction: Direction,
}

/// Endpoint for a [`USBInterface`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBEndpoint {
    /// Address information for endpoint
    pub address: EndpointAddress,
    /// Type of data transfer endpoint accepts
    pub transfer_type: TransferType,
    /// Synchronisation type (Iso mode)
    pub sync_type: SyncType,
    /// Usage type (Iso mode)
    pub usage_type: UsageType,
    /// Maximum packet size in bytes endpoint can send/recieve
    pub max_packet_size: u16,
    /// Interval for polling endpoint data transfers. Value in frame counts. Ignored for Bulk & Control Endpoints. Isochronous must equal 1 and field may range from 1 to 255 for interrupt endpoints.
    pub interval: u8,
}

/// Interface within a [`USBConfiguration`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBInterface {
    /// Name from descriptor
    pub name: String,
    /// Index of name string in descriptor - only useful for lsusb verbose print
    #[serde(skip_serializing)]
    pub string_index: u8,
    /// Interface number
    pub number: u8,
    /// Interface port path - could be generated from device but stored here for ease
    pub path: String,
    /// Class of interface provided by USB IF
    pub class: ClassCode,
    /// Sub-class of interface provided by USB IF
    pub sub_class: u8,
    /// Prototol code for interface provided by USB IF
    pub protocol: u8,
    /// Interfaces can have the same number but an alternate settings defined here
    pub alt_setting: u8,
    /// Driver obtained from udev on Linux only
    pub driver: Option<String>,
    /// syspath obtained from udev on Linux only
    pub syspath: Option<String>,
    /// An interface can have many endpoints
    pub endpoints: Vec<USBEndpoint>,
}

impl USBInterface {
    /// Linux syspath to interface
    pub fn path(&self, bus: u8, ports: &Vec<u8>, config: u8) -> String {
        get_interface_path(bus, ports, config, self.number)
    }
}

/// Devices can have multiple configurations, each with different attributes and interfaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBConfiguration {
    /// Name from string descriptor
    pub name: String,
    /// Index of name string in descriptor - only useful for lsusb verbose print
    #[serde(skip_serializing)]
    pub string_index: u8,
    /// Number of config, bConfigurationValue; value to set to enable to configuration
    pub number: u8,
    /// Interfaces available for this configuruation
    pub interfaces: Vec<USBInterface>,
    /// Attributes of configuration, bmAttributes
    pub attributes: HashSet<ConfigAttributes>,
    /// Maximum power consumption in mA
    pub max_power: NumericalUnit<u32>,
}

impl USBConfiguration {
    /// Converts attributes into a ';' separated String
    pub fn attributes_string(&self) -> String {
        ConfigAttributes::attributes_to_string(&self.attributes)
    }

    /// Convert attibutes back to reg value
    pub fn attributes_value(&self) -> u8 {
        let mut ret: u8 = 0;
        for attr in self.attributes.iter() {
            match attr {
                ConfigAttributes::SelfPowered => ret |= 0x40,
                ConfigAttributes::RemoteWakeup => ret |= 0x20,
            }
        }

        ret
    }
}

/// Extra USB device data for verbose printing
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBDeviceExtra {
    /// Maximum packet size in bytes
    pub max_packet_size: u8,
    /// Driver obtained from udev on Linux only
    pub driver: Option<String>,
    /// syspath obtained from udev on Linux only
    pub syspath: Option<String>,
    /// Vendor name from usb_ids VID lookup
    pub vendor: Option<String>,
    /// Product name from usb_ids VIDPID lookup
    pub product_name: Option<String>,
    /// Tuple of indexes to strings (iProduct, iManufacturer, iSerialNumber) - only useful for the lsbusb verbose print
    #[serde(skip_serializing)]
    pub string_indexes: (u8, u8, u8),
    /// USB devices can be have a number of configurations
    pub configurations: Vec<USBConfiguration>,
}

/// Builds a replica of sysfs path; excludes config.interface
///
/// ```
/// use cyme::usb::get_port_path;
///
/// assert_eq!(get_port_path(1, &vec![1, 3, 2]), String::from("1-1.3.2"));
/// assert_eq!(get_port_path(1, &vec![2]), String::from("1-2"));
/// // special case for root_hub
/// assert_eq!(get_port_path(2, &vec![]), String::from("2-0"));
/// ```
///
/// [ref](http://gajjarpremal.blogspot.com/2015/04/sysfs-structures-for-linux-usb.html)
/// The names that begin with "usb" refer to USB controllers. More accurately, they refer to the "root hub" associated with each controller. The number is the USB bus number. In the example there is only one controller, so its bus is number 1. Hence the name "usb1".
///
/// "1-0:1.0" is a special case. It refers to the root hub's interface. This acts just like the interface in an actual hub an almost every respect; see below.
/// All the other entries refer to genuine USB devices and their interfaces. The devices are named by a scheme like this:
///
///  bus-port.port.port ...
pub fn get_port_path(bus: u8, ports: &Vec<u8>) -> String {
    if ports.len() <= 1 {
        get_trunk_path(bus, ports)
    } else {
        format!("{:}-{}", bus, ports.into_iter().format("."))
    }
}

/// Parent path is path to parent device
/// ```
/// use cyme::usb::get_parent_path;
///
/// assert_eq!(get_parent_path(1, &vec![1, 3, 4, 5]).unwrap(), String::from("1-1.3.4"));
/// ```
pub fn get_parent_path(bus: u8, ports: &Vec<u8>) -> Result<String, String> {
    if ports.len() == 0 {
        Err("Cannot get parent path for root device".to_string())
    } else {
        Ok(get_port_path(bus, &ports[..ports.len() - 1].to_vec()))
    }
}

/// Trunk path is path to trunk device on bus
/// ```
/// use cyme::usb::get_trunk_path;
///
/// assert_eq!(get_trunk_path(1, &vec![1, 3, 5, 6]), String::from("1-1"));
/// // special case for root_hub
/// assert_eq!(get_trunk_path(1, &vec![]), String::from("1-0"));
/// ```
pub fn get_trunk_path(bus: u8, ports: &Vec<u8>) -> String {
    if ports.len() == 0 {
        // special case for root_hub
        format!("{:}-{}", bus, 0)
    } else {
        format!("{:}-{}", bus, ports[0])
    }
}

/// Build replica of sysfs path with interface
///
/// ```
/// use cyme::usb::get_interface_path;
///
/// assert_eq!(get_interface_path(1, &vec![1, 3], 1, 0), String::from("1-1.3:1.0"));
/// // bus
/// assert_eq!(get_interface_path(1, &vec![], 1, 0), String::from("1-0:1.0"));
/// ```
pub fn get_interface_path(bus: u8, ports: &Vec<u8>, config: u8, interface: u8) -> String {
    format!("{}:{}.{}", get_port_path(bus, ports), config, interface)
}
