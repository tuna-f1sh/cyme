//! Defines for USB, mainly thosed covered at [usb.org](https://www.usb.org)
//!
//! Also refering to [beyondlogic](https://beyondlogic.org/usbnutshell/usb5.shtml)
//!
//! There are some repeated/copied Enum defines from rusb in order to control Serialize/Deserialize and add impl
use clap::ValueEnum;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

pub mod descriptors;
pub mod profiler;

use crate::error::{self, Error, ErrorKind};
use crate::types::NumericalUnit;
use descriptors::*;

const WEBUSB_GET_URL: u8 = 0x02;
const USB_DT_WEBUSB_URL: u8 = 0x03;

/// The version value (for BCD and USB) is in binary coded decimal with a format of 0xJJMN where JJ is the major version number, M is the minor version number and N is the sub minor version number. e.g. USB 2.0 is reported as 0x0200, USB 1.1 as 0x0110 and USB 1.0 as 0x0100. The type is a mirror of the one from [rusb](https://docs.rs/rusb/latest/rusb/) in order to impl Display, From etc.
///
///
/// ```
/// let version = cyme::usb::Version(2, 0, 1);
/// ```
///
/// Represents the version 2.0.1, or in `String` representation it is base16 encoded:
///
/// ```
/// # let version = cyme::usb::Version(2, 0, 1);
/// assert_eq!(version.to_string(), "2.01");
/// let version = cyme::usb::Version(155, 15, 1);
/// assert_eq!(version.to_string(), "9b.f1");
/// ```
///
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version(pub u8, pub u8, pub u8);

impl Version {
    /// Extracts a version from a binary coded decimal (BCD) field. BCD fields exist in USB
    /// descriptors as 16-bit integers encoding a version as `0xJJMN`, where `JJ` is the major
    /// version, `M` is the minor version, and `N` is the sub minor version. For example, 2.0 is
    /// encoded as `0x0200` and 1.1 is encoded as `0x0110`.
    pub fn from_bcd(mut raw: u16) -> Self {
        let sub_minor: u8 = (raw & 0x000F) as u8;
        raw >>= 4;

        let minor: u8 = (raw & 0x000F) as u8;
        raw >>= 4;

        let mut major: u8 = (raw & 0x000F) as u8;
        raw >>= 4;

        major += (10 * raw) as u8;

        Version(major, minor, sub_minor)
    }

    /// Returns the major version.
    pub fn major(self) -> u8 {
        let Version(major, _, _) = self;
        major
    }

    /// Returns the minor version.
    pub fn minor(self) -> u8 {
        let Version(_, minor, _) = self;
        minor
    }

    /// Returns the sub minor version.
    pub fn sub_minor(self) -> u8 {
        let Version(_, _, sub_minor) = self;
        sub_minor
    }
}

impl std::fmt::Display for Version {
    /// Output is a base16 encoding of Major.MinorSub
    ///
    /// ```
    /// assert_eq!(cyme::usb::Version(155, 0, 0).to_string(), "9b.00");
    /// assert_eq!(cyme::usb::Version(2, 0, 1).to_string(), "2.01");
    /// ```
    ///
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:x}.{:x}{:x}",
            self.major(),
            self.minor() & 0x0F,
            self.sub_minor() & 0x0F
        )
    }
}

impl FromStr for Version {
    type Err = Error;
    fn from_str(s: &str) -> error::Result<Self> {
        let (parse_ints, _): (Vec<Result<u8, _>>, Vec<_>) = s
            .split('.')
            .map(|vs| u8::from_str_radix(vs, 16))
            .partition(Result::is_ok);
        let numbers: Vec<u8> = parse_ints.into_iter().map(|v| v.unwrap()).collect();

        match numbers.get(0..2) {
            Some(slice) => Ok(Version(slice[0], (slice[1] & 0xF0) >> 4, slice[1] & 0x0F)),
            None => Err(Error::new(
                ErrorKind::Decoding,
                &format!("No two base16 encoded versions in {}", s),
            )),
        }
    }
}

/// For legacy import where I thought the value was a f32...
impl TryFrom<f32> for Version {
    type Error = Error;

    fn try_from(f: f32) -> error::Result<Self> {
        let s = format!("{:2.2}", f);
        let (parse_ints, _): (Vec<Result<u8, _>>, Vec<_>) = s
            .split('.')
            .map(|vs| vs.parse::<u8>())
            .partition(Result::is_ok);
        let numbers: Vec<u8> = parse_ints.into_iter().map(|v| v.unwrap()).collect();

        match numbers.get(0..2) {
            Some(slice) => Ok(Version(slice[0], (slice[1] & 0xF0) >> 4, slice[1] & 0x0F)),
            None => Err(Error::new(
                ErrorKind::Decoding,
                &format!("Failed to parse float into MM.mP {}", f),
            )),
        }
    }
}

impl From<Version> for u16 {
    fn from(v: Version) -> Self {
        let Version(major, minor, sub_minor) = v;
        ((major as u16) << 8) | ((minor as u16) << 4) | (sub_minor as u16)
    }
}

/// Configuration attributes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ConfigAttributes {
    /// Device powers itself not from bus
    SelfPowered,
    /// Supports remote wake-up
    RemoteWakeup,
    /// Device is battery powered
    BatteryPowered,
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
    pub fn attributes_to_string(attributes: &[ConfigAttributes]) -> String {
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
///
/// Technically this is the 'Base Class' - the 'Class Code' is the full triplet of (Base Class, Sub Class, Protocol). TODO rename in 2.0 release
#[derive(Debug, ValueEnum, Default, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
#[repr(u8)]
pub enum ClassCode {
    #[default]
    /// Device class is unspecified, interface descriptors are used to determine needed drivers
    UseInterfaceDescriptor = 0x00,
    /// Speaker, microphone, sound card, MIDI
    Audio = 0x01,
    /// The modern serial interface; appears as a UART/RS232 port on most systems
    CDCCommunications = 0x02,
    /// Human Interface Device; game controllers, keyboards, mice etc. Also commonly used as a device data interface rather then creating something from scratch
    HID = 0x03,
    /// Force feedback joystick
    Physical = 0x05,
    /// Still imaging device; scanners, cameras
    Image = 0x06,
    /// Laser printer, inkjet printer, CNC machine
    Printer = 0x07,
    /// Mass storage devices (MSD): USB flash drive, memory card reader, digital audio player, digital camera, external drive
    MassStorage = 0x08,
    /// High speed USB hub
    Hub = 0x09,
    /// Used together with class 02h (Communications and CDC Control) above
    CDCData = 0x0a,
    /// USB smart card reader
    SmartCart = 0x0b,
    /// Fingerprint reader
    ContentSecurity = 0x0d,
    /// Webcam
    Video = 0x0e,
    /// Pulse monitor (watch)
    PersonalHealthcare = 0x0f,
    /// Webcam, TV
    AudioVideo = 0x10,
    /// Describes USB-C alternate modes supported by device
    Billboard = 0x11,
    /// An interface to expose and configure the USB Type-C capabilities of Connectors on USB Hubs or Alternate Mode Adapters
    USBTypeCBridge = 0x12,
    /// This base class is defined for devices that conform to the “VESA USB BDP Device Specification” found at the VESA website. This specification defines the usable set of SubClass and Protocol values. Values outside of this defined spec are reserved. These class codes can only be used in Interface Descriptors.
    BDP = 0x13,
    /// This base class is defined for devices that conform to the “MCTP over USB” found at the DMTF website as DSP0283. This specification defines the usable set of SubClass and Protocol values. Values outside of this defined spec are reserved. These class codes can only be used in Interface Descriptors.
    MCTP = 0x14,
    /// An interface to expose and configure I3C function within a USB device to allow interaction between host software and the I3C device, to drive transaction on the I3C bus to/from target devices
    I3CDevice = 0x3c,
    /// Trace and debugging equipment
    Diagnostic = 0xdc,
    /// Wireless controllers: Bluetooth adaptors, Microsoft RNDIS
    WirelessController = 0xe0,
    /// This base class is defined for miscellaneous device definitions. Some matching SubClass and Protocols are defined on the USB-IF website
    Miscellaneous = 0xef,
    /// This base class is defined for devices that conform to several class specifications found on the USB-IF website
    ApplicationSpecificInterface = 0xfe,
    /// This base class is defined for vendors to use as they please
    VendorSpecificClass = 0xff,
}

impl fmt::Display for ClassCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<u8> for ClassCode {
    fn from(b: u8) -> ClassCode {
        match b {
            0x00 => ClassCode::UseInterfaceDescriptor,
            0x01 => ClassCode::Audio,
            0x02 => ClassCode::CDCCommunications,
            0x03 => ClassCode::HID,
            0x05 => ClassCode::Physical,
            0x06 => ClassCode::Image,
            0x07 => ClassCode::Printer,
            0x08 => ClassCode::MassStorage,
            0x09 => ClassCode::Hub,
            0x0a => ClassCode::CDCData,
            0x0b => ClassCode::SmartCart,
            0x0d => ClassCode::ContentSecurity,
            0x0e => ClassCode::Video,
            0x0f => ClassCode::PersonalHealthcare,
            0x10 => ClassCode::AudioVideo,
            0x11 => ClassCode::Billboard,
            0x12 => ClassCode::USBTypeCBridge,
            0x13 => ClassCode::BDP,
            0x14 => ClassCode::MCTP,
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

impl From<ClassCode> for u8 {
    fn from(val: ClassCode) -> Self {
        // set as repr(u8) so this will do the conversion
        val as u8
    }
}

impl From<Class> for ClassCode {
    fn from(c: Class) -> Self {
        match c {
            Class::Generic(c) => c,
            Class::FullSpeedHub => ClassCode::Hub,
            Class::HighSpeedHubSingleTT => ClassCode::Hub,
            Class::HighSpeedHubMultiTT => ClassCode::Hub,
            Class::AudioVideoAVControlInterface => ClassCode::Audio,
            Class::AudioVideoAVDataVideo => ClassCode::Audio,
            Class::AudioVideoAVDataAudio => ClassCode::Audio,
            Class::MCTPManagementController => ClassCode::MCTP,
            Class::MCTPHostInterfaceEndpoint => ClassCode::MCTP,
            Class::USB2CompliaceDevice => ClassCode::Diagnostic,
            Class::DebugTargetVendorDefined => ClassCode::Diagnostic,
            Class::GNURemoteDebugCommandSet => ClassCode::Diagnostic,
            Class::VendorDefinedTraceDbC => ClassCode::Diagnostic,
            Class::VendorDefinedDfxDbC => ClassCode::Diagnostic,
            Class::VendorDefinedTraceGPDvC => ClassCode::Diagnostic,
            Class::GNUProtocolGPDvC => ClassCode::Diagnostic,
            Class::VendorDefinedDfxDvC => ClassCode::Diagnostic,
            Class::VendorDefinedTraceDvC => ClassCode::Diagnostic,
            Class::BluetoothProgrammingInterface => ClassCode::WirelessController,
            Class::UWBRadioControlInterace => ClassCode::WirelessController,
            Class::RemoteNDIS => ClassCode::WirelessController,
            Class::BluetoothAMPController => ClassCode::WirelessController,
            Class::HostWireAdaptor => ClassCode::WirelessController,
            Class::DeviceWireAdaptor => ClassCode::WirelessController,
            Class::DeviceWireAdaptorIsochronous => ClassCode::WirelessController,
            Class::ActiveSync => ClassCode::Miscellaneous,
            Class::PalmSync => ClassCode::Miscellaneous,
            Class::InterfaceAssociationDescriptor => ClassCode::Miscellaneous,
            Class::WireAdaptorMultifunctionPeripheral => ClassCode::Miscellaneous,
            Class::CableBasedAssociationFramework => ClassCode::Miscellaneous,
            Class::RNDISOverEthernet => ClassCode::Miscellaneous,
            Class::RNDISOverWiFi => ClassCode::Miscellaneous,
            Class::RNDISOverWiMAX => ClassCode::Miscellaneous,
            Class::RNDISOverWWAN => ClassCode::Miscellaneous,
            Class::RNDISforRawIPv4 => ClassCode::Miscellaneous,
            Class::RNDISforRawIPv6 => ClassCode::Miscellaneous,
            Class::RNDISforGPRS => ClassCode::Miscellaneous,
            Class::USB3VisionControlInterface => ClassCode::Miscellaneous,
            Class::USB3VisionEventInterface => ClassCode::Miscellaneous,
            Class::USB3VisionStreamingInterface => ClassCode::Miscellaneous,
            Class::STEPStreamTransport => ClassCode::Miscellaneous,
            Class::STEPRAWStreamTransport => ClassCode::Miscellaneous,
            Class::CommandInterfaceIAD => ClassCode::Miscellaneous,
            Class::CommandInterfaceID => ClassCode::Miscellaneous,
            Class::MediaInterfaceID => ClassCode::Miscellaneous,
            Class::DeviceFirmwareUpgrade => ClassCode::ApplicationSpecificInterface,
            Class::IRDABridge => ClassCode::ApplicationSpecificInterface,
            Class::USBTestMeasurement => ClassCode::ApplicationSpecificInterface,
            Class::USBTestMeasurementUSBTMC488 => ClassCode::ApplicationSpecificInterface,
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
        let split: Vec<&str> = title.split(' ').collect();
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
        c.usage()
    }
}

/// Fully defined USB-IF class based on (Base Class, Sub Class, Protocol) Class Code triplet
///
/// https://www.usb.org/defined-class-codes
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Class {
    /// Generic devices just have a 'Base Class'. It is a device without a defining SubClass or Protocol
    Generic(ClassCode),
    /// Full speed Hub
    FullSpeedHub,
    /// Hi-speed hub with single TT
    HighSpeedHubSingleTT,
    /// Hi-speed hub with multiple TTs
    HighSpeedHubMultiTT,
    /// Audio/Video Device – AVControl Interface
    AudioVideoAVControlInterface,
    /// Audio/Video Device – AVData Video Streaming Interface
    AudioVideoAVDataVideo,
    /// Audio/Video Device – AVData Audio Streaming Interface
    AudioVideoAVDataAudio,
    /// MCTP Management-controller and Managed-Device endpoints
    MCTPManagementController,
    /// MCTP Host Interface endpoint
    MCTPHostInterfaceEndpoint,
    /// USB2 Compliance Device. Definition for this device can be found at http://www.intel.com/technology/usb/spec.htm
    USB2CompliaceDevice,
    /// Debug Target vendor defined. Please see http://www.intel.com/content/www/us/en/io/universal-serial-bus/extensible-host-controler-interface-usb-xhci.html for more info.
    DebugTargetVendorDefined,
    /// GNU Remote Debug Command Set. Please see http://www.intel.com/content/www/us/en/io/universal-serial-bus/extensible-host-controler-interface-usb-xhci.html for more info.
    GNURemoteDebugCommandSet,
    /// Vendor defined Trace protocol on DbC.
    VendorDefinedTraceDbC,
    /// Vendor defined Dfx protocol on DbC.
    VendorDefinedDfxDbC,
    /// Vendor defined Trace protocol over General Purpose (GP) endpoint on DvC.
    VendorDefinedTraceGPDvC,
    /// GNU Protocol protocol over General Purpose (GP) endpoint on DvC.
    ///
    /// http://www.gnu.org/software/gdb/
    GNUProtocolGPDvC,
    /// Vendor defined Dfx protocol on DvC.
    VendorDefinedDfxDvC,
    /// Vendor defined Trace protocol on DvC.
    VendorDefinedTraceDvC,
    /// Bluetooth Programming Interface. Get specific information from www.bluetooth.com.
    BluetoothProgrammingInterface,
    /// UWB Radio Control Interface. Definition for this is found in the Wireless USB Specification in Chapter 8.
    UWBRadioControlInterace,
    /// Remote NDIS. Information can be found at: http://www.microsoft.com/windowsmobile/mobileoperators/default.mspx
    RemoteNDIS,
    /// Bluetooth AMP Controller. Get specific information from www.bluetooth.com.
    BluetoothAMPController,
    /// Host Wire Adapter Control/Data interface. Definition can be found in the Wireless USB Specification in Chapter 8.
    HostWireAdaptor,
    /// Device Wire Adapter Control/Data interface. Definition can be found in the Wireless USB Specification in Chapter 8.
    DeviceWireAdaptor,
    /// Device Wire Adapter Isochronous interface. Definition can be found in the Wireless USB Specification in Chapter 8.
    DeviceWireAdaptorIsochronous,
    /// Active Sync device. This class code can be used in either Device or Interface Descriptors. Contact Microsoft for more information on this class.
    ActiveSync,
    /// Palm Sync. This class code can be used in either Device or Interface Descriptors.
    PalmSync,
    /// Interface Association Descriptor. The usage of this class code triple is defined in the Interface Association Descriptor ECN that is provided on www.usb.org . This class code may only be used in Device Descriptors.
    InterfaceAssociationDescriptor,
    /// Wire Adapter Multifunction Peripheral programming interface. Definition can be found in the Wireless USB Specification in Chapter 8. This class code may only be used in Device Descriptors.
    WireAdaptorMultifunctionPeripheral,
    /// Cable Based Association Framework. This is defined in the Association Model addendum to the Wireless USB specification. This class code may only be used in Interface Descriptors.
    CableBasedAssociationFramework,
    /// RNDIS over Ethernet.
    ///
    /// Connecting a host to the Internet via Ethernet mobile device. The device appears to the host as an Ethernet gateway device. This class code may only be used in Interface Descriptors.
    RNDISOverEthernet,
    /// RNDIS over WiFi.
    ///
    /// Connecting a host to the Internet via WiFi enabled mobile device. The device represents itself to the host as an 802.11 compliant network device. This class code may only be used in Interface Descriptors.
    RNDISOverWiFi,
    /// RNDIS over WiMAX
    ///
    /// Connecting a host to the Internet via WiMAX enabled mobile device. The device is represented to the host as an 802.16 network device.
    ///
    /// This class code may only be used in Interface Descriptors.
    RNDISOverWiMAX,
    /// RNDIS over WWAN
    ///
    /// Connecting a host to the Internet via a device using mobile broadband, i.e. WWAN (GSM/CDMA).
    ///
    /// This class code may only be used in Interface Descriptors.
    RNDISOverWWAN,
    /// RNDIS for Raw IPv4
    ///
    /// Connecting a host to the Internet using raw IPv4 via non-Ethernet mobile device. Devices that provide raw IPv4, not in an Ethernet packet, may use this form to in lieu of other stock types.
    ///
    /// This class code may only be used in Interface Descriptors.
    RNDISforRawIPv4,
    /// RNDIS for Raw IPv6
    ///
    /// Connecting a host to the Internet using raw IPv6 via non-Ethernet mobile device. Devices that provide raw IPv6, not in an Ethernet packet, may use this form to in lieu of other stock types.
    ///
    /// This class code may only be used in Interface Descriptors.
    RNDISforRawIPv6,
    /// RNDIS for GPRS
    ///
    /// Connecting a host to the Internet over GPRS mobile device using the device’s cellular radio
    RNDISforGPRS,
    /// USB3 Vision Control Interface
    USB3VisionControlInterface,
    /// USB3 Vision Event Interface
    USB3VisionEventInterface,
    /// USB3 Vision Streaming Interface
    USB3VisionStreamingInterface,
    /// STEP. Stream Transport Efficient Protocol for content protection.
    STEPStreamTransport,
    /// STEP RAW. Stream Transport Efficient Protocol for Raw content protection.
    STEPRAWStreamTransport,
    /// Command Interface in IAD
    CommandInterfaceIAD,
    /// Command Interface in Interface Descriptor
    CommandInterfaceID,
    /// Media Interface in Interface Descriptor
    MediaInterfaceID,
    /// Device Firmware Upgrade. Device class definition provided on www.usb.org .
    DeviceFirmwareUpgrade,
    /// IRDA Bridge device. Device class definition provided on www.usb.org .
    IRDABridge,
    /// USB Test and Measurement Device. Definition provided in the USB Test and Measurement Class spec found on www.usb.org .
    USBTestMeasurement,
    /// USB Test and Measurement Device conforming to the USBTMC USB488 Subclass Specification found on www.usb.org.
    USBTestMeasurementUSBTMC488,
}

/// A fully defined Class Code requires a (Base Class, Sub Class, Protocol) triplet
pub type ClassCodeTriplet<T> = (T, u8, u8);

impl<T> From<ClassCodeTriplet<T>> for Class
where
    T: Into<ClassCode>,
{
    fn from(triplet: ClassCodeTriplet<T>) -> Self {
        match (triplet.0.into(), triplet.1, triplet.2) {
            (ClassCode::Hub, 0x00, 0x00) => Class::FullSpeedHub,
            (ClassCode::Hub, 0x00, 0x01) => Class::HighSpeedHubSingleTT,
            (ClassCode::Hub, 0x00, 0x02) => Class::HighSpeedHubMultiTT,
            (ClassCode::Audio, 0x01, 0x00) => Class::AudioVideoAVControlInterface,
            (ClassCode::Audio, 0x02, 0x00) => Class::AudioVideoAVDataVideo,
            (ClassCode::Audio, 0x03, 0x00) => Class::AudioVideoAVDataAudio,
            (ClassCode::MCTP, 0x00, 0x01) => Class::MCTPManagementController,
            (ClassCode::MCTP, 0x00, 0x02) => Class::MCTPHostInterfaceEndpoint,
            (ClassCode::Diagnostic, 0x01, 0x01) => Class::USB2CompliaceDevice,
            (ClassCode::Diagnostic, 0x02, 0x00) => Class::DebugTargetVendorDefined,
            (ClassCode::Diagnostic, 0x02, 0x01) => Class::GNURemoteDebugCommandSet,
            (ClassCode::Diagnostic, 0x03, 0x01) => Class::VendorDefinedTraceDbC,
            (ClassCode::Diagnostic, 0x04, 0x01) => Class::VendorDefinedDfxDbC,
            (ClassCode::Diagnostic, 0x05, 0x00) => Class::VendorDefinedTraceGPDvC,
            (ClassCode::Diagnostic, 0x05, 0x01) => Class::GNUProtocolGPDvC,
            (ClassCode::Diagnostic, 0x06, 0x01) => Class::VendorDefinedDfxDvC,
            (ClassCode::Diagnostic, 0x07, 0x01) => Class::VendorDefinedTraceDvC,
            (ClassCode::WirelessController, 0x01, 0x01) => Class::BluetoothProgrammingInterface,
            (ClassCode::WirelessController, 0x01, 0x02) => Class::UWBRadioControlInterace,
            (ClassCode::WirelessController, 0x01, 0x03) => Class::RemoteNDIS,
            (ClassCode::WirelessController, 0x01, 0x04) => Class::BluetoothAMPController,
            (ClassCode::WirelessController, 0x02, 0x01) => Class::HostWireAdaptor,
            (ClassCode::WirelessController, 0x02, 0x02) => Class::DeviceWireAdaptor,
            (ClassCode::WirelessController, 0x02, 0x03) => Class::DeviceWireAdaptorIsochronous,
            (ClassCode::Miscellaneous, 0x01, 0x01) => Class::ActiveSync,
            (ClassCode::Miscellaneous, 0x01, 0x02) => Class::PalmSync,
            (ClassCode::Miscellaneous, 0x02, 0x01) => Class::InterfaceAssociationDescriptor,
            (ClassCode::Miscellaneous, 0x02, 0x02) => Class::WireAdaptorMultifunctionPeripheral,
            (ClassCode::Miscellaneous, 0x03, 0x01) => Class::CableBasedAssociationFramework,
            (ClassCode::Miscellaneous, 0x04, 0x01) => Class::RNDISOverEthernet,
            (ClassCode::Miscellaneous, 0x04, 0x02) => Class::RNDISOverWiFi,
            (ClassCode::Miscellaneous, 0x04, 0x03) => Class::RNDISOverWiMAX,
            (ClassCode::Miscellaneous, 0x04, 0x04) => Class::RNDISOverWWAN,
            (ClassCode::Miscellaneous, 0x04, 0x05) => Class::RNDISforRawIPv4,
            (ClassCode::Miscellaneous, 0x04, 0x06) => Class::RNDISforRawIPv6,
            (ClassCode::Miscellaneous, 0x04, 0x07) => Class::RNDISforGPRS,
            (ClassCode::Miscellaneous, 0x05, 0x00) => Class::USB3VisionControlInterface,
            (ClassCode::Miscellaneous, 0x05, 0x01) => Class::USB3VisionEventInterface,
            (ClassCode::Miscellaneous, 0x05, 0x02) => Class::USB3VisionStreamingInterface,
            (ClassCode::Miscellaneous, 0x06, 0x01) => Class::STEPStreamTransport,
            (ClassCode::Miscellaneous, 0x06, 0x02) => Class::STEPRAWStreamTransport,
            // (ClassCode::Miscellaneous, 0x07, 0x01) => DeviceClass::CommandInterfaceIAD,
            (ClassCode::Miscellaneous, 0x07, 0x01) => Class::CommandInterfaceID,
            (ClassCode::Miscellaneous, 0x07, 0x02) => Class::MediaInterfaceID,
            (ClassCode::ApplicationSpecificInterface, 0x01, 0x01) => Class::DeviceFirmwareUpgrade,
            (ClassCode::ApplicationSpecificInterface, 0x02, 0x00) => Class::IRDABridge,
            (ClassCode::ApplicationSpecificInterface, 0x03, 0x00) => Class::USBTestMeasurement,
            (ClassCode::ApplicationSpecificInterface, 0x03, 0x01) => {
                Class::USBTestMeasurementUSBTMC488
            }
            (c, _, _) => Class::Generic(c),
        }
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ClassCode> for Class {
    fn from(class: ClassCode) -> Self {
        Class::Generic(class)
    }
}

impl Class {
    // TODO ensure this is correct
    fn usage(&self) -> DescriptorUsage {
        match self {
            Class::Generic(c) => c.usage(),
            _ => DescriptorUsage::Interface,
        }
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
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
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
                Speed::LowSpeed => "low_speed",
                Speed::Unknown => "unknown",
                // _ => todo!("Unsupported speed"),
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
        if f.alternate() {
            match self {
                Direction::Out => write!(f, "OUT"),
                Direction::In => write!(f, "IN"),
            }
        } else {
            write!(f, "{:?}", self)
        }
    }
}

/// Transfer type  for [`USBEndpoint`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
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

impl From<u8> for TransferType {
    fn from(b: u8) -> Self {
        match b & 0x03 {
            0 => TransferType::Control,
            1 => TransferType::Isochronous,
            2 => TransferType::Bulk,
            3 => TransferType::Interrupt,
            _ => unreachable!(),
        }
    }
}

/// Isochronous synchronization mode for [`USBEndpoint`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
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

impl From<u8> for SyncType {
    fn from(b: u8) -> Self {
        match (b & 0x0c) >> 2 {
            0 => SyncType::None,
            1 => SyncType::Asynchronous,
            2 => SyncType::Adaptive,
            3 => SyncType::Synchronous,
            _ => unreachable!(),
        }
    }
}

/// Isochronous usage type for [`USBEndpoint`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
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

impl From<u8> for UsageType {
    fn from(b: u8) -> Self {
        match (b & 0x30) >> 4 {
            0 => UsageType::Data,
            1 => UsageType::Feedback,
            2 => UsageType::FeedbackData,
            3 => UsageType::Reserved,
            _ => unreachable!(),
        }
    }
}

// these are for backwards compatible json defaults
/// The USB device descriptor is actually a fixed length
fn default_device_desc_length() -> u8 {
    18
}

/// The USB configuration descriptor is variable but most are 9 bytes
fn default_configuration_desc_length() -> u8 {
    9
}

/// The USB interface descriptor is variable but most are 9 bytes
fn default_interface_desc_length() -> u8 {
    9
}

/// True for most endpoints other than audio
fn default_endpoint_desc_length() -> u8 {
    7
}

/// Address information for a [`USBEndpoint`]
// This struct could be one byte with getters using mask but this saves a custom Serialize impl for system_profiler
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointAddress {
    /// Endpoint address byte
    pub address: u8,
    /// Endpoint number on [`USBInterface`] 0..3b
    pub number: u8,
    /// Data transfer direction 7b
    pub direction: Direction,
}

impl From<u8> for EndpointAddress {
    fn from(b: u8) -> Self {
        EndpointAddress {
            address: b,
            // 0..3b
            number: b & 0x0f,
            direction: if b & 0x80 == 0 {
                Direction::Out
            } else {
                Direction::In
            },
        }
    }
}

impl From<EndpointAddress> for u8 {
    fn from(addr: EndpointAddress) -> u8 {
        addr.address
    }
}

impl fmt::Display for EndpointAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "EP {} {}", self.number, self.direction)
    }
}

/// Endpoint for a [`USBInterface`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBEndpoint {
    /// Endpoint length in bytes
    #[serde(default = "default_endpoint_desc_length")] // for backwards compatible json
    pub length: u8,
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
    /// Extra descriptors data based on type
    #[serde(default)] // default for legacy json
    pub extra: Option<Vec<Descriptor>>,
}

impl USBEndpoint {
    /// Decodes the max packet value into a multipler and number of bytes like lsusb
    ///
    /// ```
    /// # use cyme::usb::*;
    ///
    /// let mut ep = USBEndpoint {
    ///     length: 7,
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
    ///     extra: None,
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

    /// Returns the attributes byte for the endpoint
    pub fn attributes(&self) -> u8 {
        self.transfer_type.to_owned() as u8
            | (self.sync_type.to_owned() as u8) << 2
            | (self.usage_type.to_owned() as u8) << 4
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
    /// Size of interface descriptor in bytes
    #[serde(default = "default_interface_desc_length")]
    pub length: u8,
    /// Extra descriptors for interface based on type
    #[serde(default)] // default for legacy json
    pub extra: Option<Vec<Descriptor>>,
}

impl USBInterface {
    /// Linux syspath to interface
    pub fn path(&self, bus: u8, ports: &[u8], config: u8) -> String {
        get_interface_path(bus, ports, config, self.number)
    }

    /// Name of class from Linux USB IDs repository
    pub fn class_name(&self) -> Option<&str> {
        usb_ids::Classes::iter()
            .find(|c| c.id() == u8::from(self.class))
            .map(|c| c.name())
    }

    /// Name of sub class from Linux USB IDs repository
    pub fn sub_class_name(&self) -> Option<&str> {
        usb_ids::SubClass::from_cid_scid(u8::from(self.class), self.sub_class).map(|sc| sc.name())
    }

    /// Name of protocol from Linux USB IDs repository
    pub fn protocol_name(&self) -> Option<&str> {
        usb_ids::Protocol::from_cid_scid_pid(u8::from(self.class), self.sub_class, self.protocol)
            .map(|p| p.name())
    }

    /// Returns fully defined USB [`Class`] based on base_class, sub_class and protocol triplet
    pub fn fully_defined_class(&self) -> Class {
        (self.class, self.sub_class, self.protocol).into()
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
    /// Size of configuration descriptor in bytes
    #[serde(default = "default_configuration_desc_length")]
    pub length: u8,
    /// Total length of configuration descriptor in bytes including all interfaces and endpoints
    #[serde(default)]
    pub total_length: u16,
    /// Extra descriptors for configuration based on type
    #[serde(default)] // default for legacy json
    pub extra: Option<Vec<Descriptor>>,
}

impl USBConfiguration {
    /// Converts attributes into a ';' separated String
    pub fn attributes_string(&self) -> String {
        ConfigAttributes::attributes_to_string(&self.attributes)
    }

    /// Convert attibutes back to reg value
    pub fn attributes_value(&self) -> u8 {
        let mut ret: u8 = 0x80; // always set reserved bit
        for attr in self.attributes.iter() {
            match attr {
                ConfigAttributes::SelfPowered => ret |= 0x40,
                ConfigAttributes::RemoteWakeup => ret |= 0x20,
                ConfigAttributes::BatteryPowered => ret |= 0x10,
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
    /// Device status
    pub status: Option<u16>,
    /// Debug descriptor if present
    pub debug: Option<DebugDescriptor>,
    /// Binary Object Store (BOS) descriptor if present
    pub binary_object_store: Option<bos::BinaryObjectStoreDescriptor>,
    /// Device qualifier descriptor if present
    pub qualifier: Option<DeviceQualifierDescriptor>,
    /// Hub descriptor if present (is a hub)
    pub hub: Option<HubDescriptor>,
}

/// Builds a replica of sysfs path; excludes config.interface
///
/// ```
/// use cyme::usb::get_port_path;
///
/// assert_eq!(get_port_path(1, &[1, 3, 2]), String::from("1-1.3.2"));
/// assert_eq!(get_port_path(1, &[2]), String::from("1-2"));
/// // special case for root_hub
/// assert_eq!(get_port_path(2, &[]), String::from("2-0"));
/// ```
///
/// [ref](http://gajjarpremal.blogspot.com/2015/04/sysfs-structures-for-linux-usb.html)
/// The names that begin with "usb" refer to USB controllers. More accurately, they refer to the "root hub" associated with each controller. The number is the USB bus number. In the example there is only one controller, so its bus is number 1. Hence the name "usb1".
///
/// "1-0:1.0" is a special case. It refers to the root hub's interface. This acts just like the interface in an actual hub an almost every respect; see below.
/// All the other entries refer to genuine USB devices and their interfaces. The devices are named by a scheme like this:
///
///  bus-port.port.port ...
pub fn get_port_path(bus: u8, ports: &[u8]) -> String {
    if ports.len() <= 1 {
        get_trunk_path(bus, ports)
    } else {
        format!("{:}-{}", bus, ports.iter().format("."))
    }
}

/// Parent path is path to parent device
/// ```
/// use cyme::usb::get_parent_path;
///
/// assert_eq!(get_parent_path(1, &[1, 3, 4, 5]).unwrap(), String::from("1-1.3.4"));
/// ```
pub fn get_parent_path(bus: u8, ports: &[u8]) -> error::Result<String> {
    if ports.is_empty() {
        Err(Error::new(
            ErrorKind::InvalidArg,
            "Cannot get parent path for root device",
        ))
    } else {
        Ok(get_port_path(bus, &ports[..ports.len() - 1]))
    }
}

/// Trunk path is path to trunk device on bus
/// ```
/// use cyme::usb::get_trunk_path;
///
/// assert_eq!(get_trunk_path(1, &[1, 3, 5, 6]), String::from("1-1"));
/// // special case for root_hub
/// assert_eq!(get_trunk_path(1, &[]), String::from("1-0"));
/// ```
pub fn get_trunk_path(bus: u8, ports: &[u8]) -> String {
    if ports.is_empty() {
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
/// assert_eq!(get_interface_path(1, &[1, 3], 1, 0), String::from("1-1.3:1.0"));
/// // bus
/// assert_eq!(get_interface_path(1, &[], 1, 0), String::from("1-0:1.0"));
/// ```
pub fn get_interface_path(bus: u8, ports: &[u8], config: u8, interface: u8) -> String {
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

/// Builds a replica of sysfs name for reading sysfs_props ala: https://github.com/gregkh/usbutils/blob/master/sysfs.c#L29
///
/// Like `get_port_path` but root_hubs use the USB controller name (usbX) rather than interface
///
/// ```
/// use cyme::usb::get_sysfs_name;
///
/// assert_eq!(get_sysfs_name(1, &vec![1, 3, 2]), String::from("1-1.3.2"));
/// assert_eq!(get_sysfs_name(1, &vec![2]), String::from("1-2"));
/// // special case for root_hub
/// assert_eq!(get_sysfs_name(2, &vec![]), String::from("usb2"));
/// ```
pub fn get_sysfs_name(bus: u8, ports: &[u8]) -> String {
    if ports.is_empty() {
        // special cae for root_hub
        format!("usb{}", bus)
    } else {
        get_port_path(bus, ports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_to_string() {
        assert_eq!(Version(155, 0, 0).to_string(), "9b.00");
        // leading not padded
        assert_eq!(Version(10, 4, 15).to_string(), "a.4f");
        assert_eq!(Version(2, 0, 1).to_string(), "2.01");
    }

    #[test]
    fn test_version_from_f32() {
        assert_eq!(Version::try_from(155.0).unwrap(), Version(155, 0, 0));
        assert_eq!(Version::try_from(101.0).unwrap(), Version(101, 0, 0));
        assert_eq!(Version::try_from(2.01).unwrap(), Version(2, 0, 1));
        assert_eq!(Version::try_from(2.31).unwrap(), Version(2, 1, 15));
    }
}
