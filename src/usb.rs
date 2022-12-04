//! Defines for USB, mainly thosed covered at [usb.org](https://www.usb.org)
//!
//! Also refering to [beyondlogic](https://beyondlogic.org/usbnutshell/usb5.shtml)
//!
//! There are some repeated/copied Enum defines from rusb in order to control Serialize/Deserialize and add impl
use itertools::Itertools;
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
    ///
    /// ```
    /// use cyme::usb::ConfigAttributes;
    ///
    /// assert_eq!(ConfigAttributes::attributes_to_string(&vec![ConfigAttributes::RemoteWakeup, ConfigAttributes::SelfPowered]), "RemoteWakeup;SelfPowered");
    /// ```
    pub fn attributes_to_string(attributes: &Vec<ConfigAttributes>) -> String {
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
pub enum ClassCode {
    #[default]
    /// Device class is unspecified, interface descriptors are used to determine needed drivers
    UseInterfaceDescriptor,
    /// Speaker, microphone, sound card, MIDI
    Audio,
    /// The modern serial interface; appears as a UART/RS232 port on most systems
    CDCCommunications,
    /// Human Interface Device; game controllers, keyboards, mice etc. Also commonly used as a device data interface rather then creating something from scratch
    HID,
    /// Force feedback joystick
    Physical,
    /// Scanners, cameras
    Image,
    /// Laser printer, inkjet printer, CNC machine
    Printer,
    /// Mass storage devices (MSD): USB flash drive, memory card reader, digital audio player, digital camera, external drive
    MassStorage,
    /// High speed USB hub
    Hub,
    /// Used together with class 02h (Communications and CDC Control) above
    CDCData,
    /// USB smart card reader
    SmartCart,
    /// Fingerprint reader
    ContentSecurity,
    /// Webcam
    Video,
    /// Pulse monitor (watch)
    PersonalHealthcare,
    /// Webcam, TV
    AudioVideo,
    /// Describes USB-C alternate modes supported by device
    Billboard,
    /// An interface to expose and configure the USB Type-C capabilities of Connectors on USB Hubs or Alternate Mode Adapters
    USBTypeCBridge,
    /// An interface to expose and configure I3C function within a USB device to allow interaction between host software and the I3C device, to drive transaction on the I3C bus to/from target devices
    I3CDevice,
    /// Trace and debugging equipment
    Diagnostic,
    /// Wireless controllers: Bluetooth adaptors, Microsoft RNDIS
    WirelessController,
    /// This base class is defined for miscellaneous device definitions. Some matching SubClass and Protocols are defined on the USB-IF website
    Miscellaneous,
    /// This base class is defined for devices that conform to several class specifications found on the USB-IF website
    ApplicationSpecificInterface,
    /// This base class is defined for vendors to use as they please
    VendorSpecificClass,
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
            0xff => ClassCode::VendorSpecificClass,
            _ => ClassCode::UseInterfaceDescriptor,
        }
    }
}

impl Into<u8> for ClassCode {
    fn into(self) -> u8 {
        match self {
            ClassCode::UseInterfaceDescriptor => 0,
            ClassCode::Audio => 1,
            ClassCode::CDCCommunications => 2,
            ClassCode::HID => 3,
            ClassCode::Physical => 5,
            ClassCode::Image => 6,
            ClassCode::Printer => 7,
            ClassCode::MassStorage => 8,
            ClassCode::Hub => 9,
            ClassCode::CDCData => 0x0a,
            ClassCode::SmartCart => 0x0b,
            ClassCode::ContentSecurity => 0x0d,
            ClassCode::Video => 0x0e,
            ClassCode::PersonalHealthcare => 0x0f,
            ClassCode::AudioVideo => 0x10,
            ClassCode::Billboard => 0x11,
            ClassCode::USBTypeCBridge => 0x12,
            ClassCode::I3CDevice => 0x3c,
            ClassCode::Diagnostic => 0xdc,
            ClassCode::WirelessController => 0xe0,
            ClassCode::Miscellaneous => 0xef,
            ClassCode::ApplicationSpecificInterface => 0xfe,
            ClassCode::VendorSpecificClass => 0xff,
        }
    }
}

// TODO return device based on base class and subclass, protocol
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
            | ClassCode::VendorSpecificClass => DescriptorUsage::Both,
            _ => DescriptorUsage::Interface,
        }
    }

    /// lsusb is explicit for some in styling of tree
    /// ```
    /// # use cyme::usb::ClassCode;
    ///
    /// assert_eq!(ClassCode::HID.to_lsusb_string(), "Human Interface Device");
    /// ```
    pub fn to_lsusb_string(&self) -> String {
        match self {
            ClassCode::HID => "Human Interface Device".into(),
            ClassCode::CDCCommunications => "Communications".into(),
            _ => self.to_title_case(),
        }

    }

    /// Converts Pascal case enum to space separated on capitals
    /// ```
    /// # use cyme::usb::ClassCode;
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
            "Cdc" | "Usb" | "I3c" | "Hid" => title.replace(first, &first.to_uppercase()),
            _ => title,
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
            "10.0 Gb/s" | "super_speed_plus" => Speed::SuperSpeedPlus,
            "5.0 Gb/s" | "super_speed" => Speed::SuperSpeed,
            "480.0 Mb/s" | "high_speed" | "high_bandwidth" => Speed::HighSpeed,
            "12.0 Mb/s" | "full_speed" => Speed::FullSpeed,
            "1.5 Mb/s" | "low_speed" => Speed::LowSpeed,
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
    ///
    /// ```
    /// # use cyme::usb::Speed;
    ///
    /// assert_eq!(Speed::SuperSpeedPlus.to_lsusb_speed(), "10000M");
    /// assert_eq!(Speed::FullSpeed.to_lsusb_speed(), "12M");
    /// ```
    pub fn to_lsusb_speed(&self) -> String {
        let dv = NumericalUnit::<f32>::from(self);
        let prefix = dv.unit.chars().next().unwrap_or('M');
        match prefix {
            // see you when we have Tb/s buses :P
            'G' => format!("{:.0}{}", dv.value * 1000.0, 'M'),
            _ => format!("{:.0}{}", dv.value, prefix),
        }
    }
}

/// Transfer and [`USBEndpoint`] direction
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Direction for write (host to device) transfers.
    Out,
    /// Direction for read (device to host) transfers.
    In,
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
    /// Maximum packet size in bytes endpoint can send/recieve - encoded with multipler, use `max_packet_string` for packet information
    pub max_packet_size: u16,
    /// Interval for polling endpoint data transfers. Value in frame counts. Ignored for Bulk & Control Endpoints. Isochronous must equal 1 and field may range from 1 to 255 for interrupt endpoints.
    pub interval: u8,
}

impl USBEndpoint {
    /// Decodes the max packet value into a multipler and number of bytes like lsusb
    ///
    /// ```
    /// # use cyme::usb::*;
    ///
    /// let mut ep = USBEndpoint {
    ///     address: EndpointAddress {
    ///         address: 0,
    ///         number: 0,
    ///         direction: Direction::In
    ///     },
    ///     transfer_type: TransferType::Control,
    ///     sync_type: SyncType::None,
    ///     usage_type: UsageType::Data,
    ///     max_packet_size: 0xfff1,
    ///     interval: 3,
    /// };
    /// assert_eq!(ep.max_packet_string(), "4x 2033");
    /// ep.max_packet_size = 0x0064;
    /// assert_eq!(ep.max_packet_string(), "1x 100");
    /// ```
    pub fn max_packet_string(&self) -> String {
        format!(
            "{}x {}",
            ((self.max_packet_size >> 11) & 3) + 1,
            self.max_packet_size & 0x7ff
        )
    }
}

/// Interface within a [`USBConfiguration`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBInterface {
    /// Name from descriptor
    pub name: String,
    /// Index of name string in descriptor - only useful for lsusb verbose print
    #[serde(default)]
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
    #[serde(default)]
    pub string_index: u8,
    /// Number of config, bConfigurationValue; value to set to enable to configuration
    pub number: u8,
    /// Interfaces available for this configuruation
    pub interfaces: Vec<USBInterface>,
    /// Attributes of configuration, bmAttributes - was a HashSet since attributes should be unique but caused issues printing out of order
    pub attributes: Vec<ConfigAttributes>,
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
    #[serde(default)]
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

/// Build replica of Linux dev path from libusb.c *devbususb for getting device with -D
///
/// It's /dev/bus/usb/BUS/DEVNO
///
/// Supply `device_no` as None for bus
///
/// ```
/// use cyme::usb::get_dev_path;
///
/// assert_eq!(get_dev_path(1, Some(3)), String::from("/dev/bus/usb/001/003"));
/// assert_eq!(get_dev_path(1, Some(2)), String::from("/dev/bus/usb/001/002"));
/// // special case for bus
/// assert_eq!(get_dev_path(1, None), String::from("/dev/bus/usb/001/001"));
/// ```
pub fn get_dev_path(bus: u8, device_no: Option<u8>) -> String {
    if let Some(devno) = device_no {
        format!("/dev/bus/usb/{:03}/{:03}", bus, devno)
    } else {
        format!("/dev/bus/usb/{:03}/001", bus)
    }
}
