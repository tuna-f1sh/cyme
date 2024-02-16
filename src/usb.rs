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

use crate::error::{self, Error, ErrorKind};
use crate::types::NumericalUnit;

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
    pub extra: Option<Vec<DescriptorType>>,
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
    ///     length: 7,
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
    /// Size of interface descriptor in bytes
    #[serde(default = "default_interface_desc_length")]
    pub length: u8,
    /// Extra descriptors for interface based on type
    #[serde(default)] // default for legacy json
    pub extra: Option<Vec<DescriptorType>>,
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
    pub extra: Option<Vec<DescriptorType>>,
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

/// USB Descriptor Types
///
/// Can enclose struct of descriptor data
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[repr(u8)]
#[allow(missing_docs)]
// TODO structs for others
pub enum DescriptorType {
    Device(ClassDescriptor) = 0x01,
    Config(ClassDescriptor) = 0x02,
    String(String) = 0x03,
    Interface(ClassDescriptor) = 0x04,
    Endpoint(ClassDescriptor) = 0x05,
    DeviceQualifier = 0x06,
    OtherSpeedConfiguration = 0x07,
    InterfacePower = 0x08,
    // TODO do_otg
    Otg = 0x09,
    Debug = 0x0a,
    InterfaceAssociation(InterfaceAssociationDescriptor) = 0x0b,
    Security(SecurityDescriptor) = 0x0c,
    Key = 0x0d,
    Encrypted(EncryptionDescriptor) = 0x0e,
    Bos = 0x0f,
    DeviceCapability = 0x10,
    WirelessEndpointCompanion = 0x11,
    WireAdaptor = 0x21,
    Report(HidReportDescriptor) = 0x22,
    Physical = 0x23,
    Pipe = 0x24,
    // TODO do_hub
    Hub = 0x29,
    SuperSpeedHub = 0x2a,
    SsEndpointCompanion(SsEndpointCompanionDescriptor) = 0x30,
    SsIsocEndpointCompanion = 0x31,
    // these are internal
    Unknown(Vec<u8>) = 0xfe,
    Junk(Vec<u8>) = 0xff,
}

impl TryFrom<&[u8]> for DescriptorType {
    type Error = Error;

    fn try_from(v: &[u8]) -> error::Result<Self> {
        if v.len() < 2 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Descriptor type too short, must be at least 2 bytes",
            ));
        }

        // junk length
        if v[0] < 2 {
            return Ok(DescriptorType::Junk(v.to_vec()));
        }

        match v[1] {
            0x01 => Ok(DescriptorType::Device(ClassDescriptor::try_from(v)?)),
            0x02 => Ok(DescriptorType::Config(ClassDescriptor::try_from(v)?)),
            0x03 => Ok(DescriptorType::String(
                String::from_utf8_lossy(v).to_string(),
            )),
            0x04 => Ok(DescriptorType::Interface(ClassDescriptor::try_from(v)?)),
            0x05 => Ok(DescriptorType::Endpoint(ClassDescriptor::try_from(v)?)),
            0x06 => Ok(DescriptorType::DeviceQualifier),
            0x07 => Ok(DescriptorType::OtherSpeedConfiguration),
            0x08 => Ok(DescriptorType::InterfacePower),
            0x09 => Ok(DescriptorType::Otg),
            0x0a => Ok(DescriptorType::Debug),
            0x0b => Ok(DescriptorType::InterfaceAssociation(
                InterfaceAssociationDescriptor::try_from(v)?,
            )),
            0x0c => Ok(DescriptorType::Security(SecurityDescriptor::try_from(v)?)),
            0x0d => Ok(DescriptorType::Key),
            0x0e => Ok(DescriptorType::Encrypted(EncryptionDescriptor::try_from(
                v,
            )?)),
            0x0f => Ok(DescriptorType::Bos),
            0x10 => Ok(DescriptorType::DeviceCapability),
            0x11 => Ok(DescriptorType::WirelessEndpointCompanion),
            0x21 => Ok(DescriptorType::WireAdaptor),
            0x22 => Ok(DescriptorType::Report(HidReportDescriptor::try_from(v)?)),
            0x23 => Ok(DescriptorType::Physical),
            0x24 => Ok(DescriptorType::Pipe),
            0x29 => Ok(DescriptorType::Hub),
            0x2a => Ok(DescriptorType::SuperSpeedHub),
            0x30 => Ok(DescriptorType::SsEndpointCompanion(
                SsEndpointCompanionDescriptor::try_from(v)?,
            )),
            0x31 => Ok(DescriptorType::SsIsocEndpointCompanion),
            _ => Ok(DescriptorType::Unknown(v.to_vec())),
        }
    }
}

impl DescriptorType {
    /// Uses [`ClassCodeTriplet`] to update the [`ClassDescriptor`] with [`ClassCode`] for class specific descriptors
    pub fn update_with_class_context<T: Into<ClassCode> + Copy>(
        &mut self,
        class_triplet: ClassCodeTriplet<T>,
    ) -> Result<(), Error> {
        match self {
            DescriptorType::Device(d) => d.update_with_class_context(class_triplet),
            DescriptorType::Config(c) => c.update_with_class_context(class_triplet),
            DescriptorType::Interface(i) => i.update_with_class_context(class_triplet),
            DescriptorType::Endpoint(e) => e.update_with_class_context(class_triplet),
            _ => Ok(()),
        }
    }
}

/// Device Capability Type Codes (Wireless USB spec and USB 3.0 bus spec)
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(missing_docs)]
#[repr(u8)]
pub enum DeviceCapability {
    WirelessUsb = 0x01,
    Usb20Extension = 0x02,
    Superspeed = 0x03,
    ContainerId = 0x04,
    Platform = 0x05,
    SuperSpeedPlus = 0x0a,
    BillBoard = 0x0d,
    BillboardAltMode = 0x0f,
    ConfigurationSummary = 0x10,
}

/// Extra USB device data for unknown descriptors
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescriptorData(pub Vec<u8>);

/// The Interface Association Descriptor is a specific type of USB descriptor used to associate a group of interfaces with a particular function or feature of a USB device
///
/// It helps organize and convey the relationship between different interfaces within a single device configuration.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct InterfaceAssociationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub first_interface: u8,
    pub interface_count: u8,
    pub function_class: u8,
    pub function_sub_class: u8,
    pub function_protocol: u8,
    pub function_string_index: u8,
    pub function_string: Option<String>,
}

impl TryFrom<&[u8]> for InterfaceAssociationDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Interface Association descriptor too short",
            ));
        }

        Ok(InterfaceAssociationDescriptor {
            length: value[0],
            descriptor_type: value[1],
            first_interface: value[2],
            interface_count: value[3],
            function_class: value[4],
            function_sub_class: value[5],
            function_protocol: value[6],
            function_string_index: value[7],
            function_string: None,
        })
    }
}

/// USB SS Endpoint Companion descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SsEndpointCompanionDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub max_burst: u8,
    pub attributes: u8,
}

impl TryFrom<&[u8]> for SsEndpointCompanionDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "SS Endpoint Companion descriptor too short",
            ));
        }

        Ok(SsEndpointCompanionDescriptor {
            length: value[0],
            descriptor_type: value[1],
            max_burst: value[2],
            attributes: value[3],
        })
    }
}

/// USB security descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SecurityDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub encryption_types: u8,
}

impl TryFrom<&[u8]> for SecurityDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Security descriptor too short",
            ));
        }

        Ok(SecurityDescriptor {
            length: value[0],
            descriptor_type: value[1],
            total_length: u16::from_le_bytes([value[2], value[3]]),
            encryption_types: value[4],
        })
    }
}

/// Encryption type for [`SecurityDescriptor`]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum EncryptionType {
    Unsecure,
    Wired,
    Ccm1,
    Rsa1,
    Reserved,
}

impl From<u8> for EncryptionType {
    fn from(b: u8) -> Self {
        match b {
            0x00 => EncryptionType::Unsecure,
            0x01 => EncryptionType::Wired,
            0x02 => EncryptionType::Ccm1,
            0x03 => EncryptionType::Rsa1,
            _ => EncryptionType::Reserved,
        }
    }
}

/// USB encryption descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct EncryptionDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub encryption_type: EncryptionType,
    pub encryption_value: u8,
    pub auth_key_index: u8,
}

impl TryFrom<&[u8]> for EncryptionDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Encryption Type descriptor too short",
            ));
        }

        Ok(EncryptionDescriptor {
            length: value[0],
            descriptor_type: value[1],
            encryption_type: EncryptionType::from(value[2]),
            encryption_value: value[3],
            auth_key_index: value[4],
        })
    }
}

/// USB base class descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassDescriptor {
    /// USB HID extra descriptor
    Hid(HidDescriptor),
    /// USB Communication extra descriptor
    Communication(CommunicationDescriptor),
    /// USB CCID (Smart Card) extra descriptor
    Ccid(CcidDescriptor),
    /// USB Printer extra descriptor
    Printer(PrinterDescriptor),
    /// USB MIDI extra descriptor (AudioVideoAVDataAudio)
    Midi(MidiDescriptor),
    /// Generic descriptor with Option<ClassCode>
    ///
    /// Used for most descriptors and allows for TryFrom without knowing the [`ClassCode`]
    Generic(Option<ClassCodeTriplet<ClassCode>>, GenericDescriptor),
}

impl TryFrom<&[u8]> for ClassDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Class descriptor too short",
            ));
        }

        Ok(ClassDescriptor::Generic(
            None,
            GenericDescriptor::try_from(value)?,
        ))
    }
}

impl From<ClassDescriptor> for Vec<u8> {
    fn from(cd: ClassDescriptor) -> Self {
        match cd {
            ClassDescriptor::Generic(_, gd) => gd.into(),
            ClassDescriptor::Hid(hd) => hd.into(),
            ClassDescriptor::Ccid(cd) => cd.into(),
            ClassDescriptor::Printer(pd) => pd.into(),
            _ => Vec::new(),
        }
    }
}

impl ClassDescriptor {
    /// Uses [`ClassCodeTriplet`] to update the [`ClassDescriptor`] with [`ClassCode`] and descriptor if it is not [`GenericDescriptor`]
    pub fn update_with_class_context<T: Into<ClassCode> + Copy>(
        &mut self,
        triplet: ClassCodeTriplet<T>,
    ) -> Result<(), Error> {
        match self {
            ClassDescriptor::Generic(None, gd) => match (triplet.0.into(), triplet.1, triplet.2) {
                (ClassCode::HID, _, _) => {
                    *self = ClassDescriptor::Hid(HidDescriptor::try_from(gd.to_owned())?)
                }
                (ClassCode::SmartCart, _, _) => {
                    *self = ClassDescriptor::Ccid(CcidDescriptor::try_from(gd.to_owned())?)
                }
                (ClassCode::Printer, _, _) => {
                    *self = ClassDescriptor::Printer(PrinterDescriptor::try_from(gd.to_owned())?)
                }
                (ClassCode::CDCCommunications, _, _) | (ClassCode::CDCData, _, _) => {
                    *self = ClassDescriptor::Communication(CommunicationDescriptor::try_from(gd.to_owned())?)
                }
                (ClassCode::Audio, 3, _) => {
                    *self = ClassDescriptor::Midi(MidiDescriptor::try_from(gd.to_owned())?)
                }
                ct => *self = ClassDescriptor::Generic(Some(ct), gd.to_owned()),
            },
            _ => (),
        }

        Ok(())
    }
}

/// USB HID report descriptor
///
/// Similar to [`GenericDescriptor`] but with a wLength rather than bLength and no sub-type
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct HidReportDescriptor {
    pub descriptor_type: u8,
    pub length: u16,
    pub data: Option<Vec<u8>>,
}

impl TryFrom<&[u8]> for HidReportDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "HID report descriptor too short",
            ));
        }

        if value[0] != 0x22 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "HID report descriptor must have descriptor type 0x22",
            ));
        }

        Ok(HidReportDescriptor {
            descriptor_type: value[0],
            length: u16::from_le_bytes([value[1], value[2]]),
            data: value.get(3..).map(|d| d.to_vec()),
        })
    }
}

impl From<HidReportDescriptor> for Vec<u8> {
    fn from(hd: HidReportDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(hd.descriptor_type);
        ret.extend(hd.length.to_le_bytes());
        if let Some(data) = hd.data {
            ret.extend(data);
        }

        ret
    }
}

/// USB generic descriptor
///
/// Used for most [`ClassDescriptor`]s
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct GenericDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_subtype: u8,
    pub data: Option<Vec<u8>>,
}

impl TryFrom<&[u8]> for GenericDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Generic descriptor too short",
            ));
        }

        Ok(GenericDescriptor {
            length: value[0],
            descriptor_type: value[1],
            descriptor_subtype: value[2],
            data: value.get(3..).map(|d| d.to_vec()),
        })
    }
}

impl From<GenericDescriptor> for Vec<u8> {
    fn from(gd: GenericDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(gd.length);
        ret.push(gd.descriptor_type);
        ret.push(gd.descriptor_subtype);
        if let Some(data) = gd.data {
            ret.extend(data);
        }

        ret
    }
}

impl GenericDescriptor {
    /// Returns the reported length of the data
    pub fn len(&self) -> usize {
        self.length as usize
    }

    /// Returns the expected length of the data based on the length field minus the bytes taken by struct fields
    pub fn expected_data_length(&self) -> usize {
        if self.len() < 3 {
            0
        } else {
            self.len() - 3
        }
    }

    /// Returns the (cloned) data as a Vec<u8>
    pub fn to_vec(&self) -> Vec<u8> {
        self.clone().into()
    }
}

/// USB HID descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct HidDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub bcd_hid: Version,
    pub country_code: u8,
    pub descriptors: Vec<HidReportDescriptor>,
}

impl TryFrom<&[u8]> for HidDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "HID descriptor too short",
            ));
        }

        let num_descriptors = value[5] as usize;
        let mut descriptors_vec = value[6..].to_vec();
        let mut descriptors = Vec::<HidReportDescriptor>::with_capacity(num_descriptors);

        for _ in 0..num_descriptors {
            if descriptors_vec.len() < 2 {
                return Err(Error::new(
                    ErrorKind::InvalidArg,
                    "HID report descriptor too short",
                ));
            }

            let len = u16::from_le_bytes([descriptors_vec[1], descriptors_vec[2]]) as usize;

            if len > descriptors_vec.len()  {
                return Err(Error::new(
                    ErrorKind::InvalidArg,
                    "HID report descriptor too long for available data!",
                ));
            }

            descriptors.push(descriptors_vec.drain(..len).as_slice().try_into()?);
        }

        Ok(HidDescriptor {
            length: value[0],
            descriptor_type: value[1],
            bcd_hid: Version::from_bcd(u16::from_le_bytes([value[2], value[3]])),
            country_code: value[4],
            descriptors,
        })
    }
}

impl TryFrom<GenericDescriptor> for HidDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        HidDescriptor::try_from(&gd_vec[..])
    }
}

impl From<HidDescriptor> for Vec<u8> {
    fn from(hd: HidDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(hd.length);
        ret.push(hd.descriptor_type);
        ret.extend(u16::from(hd.bcd_hid).to_le_bytes());
        ret.push(hd.country_code);
        for desc in hd.descriptors {
            ret.extend(Vec::<u8>::from(desc));
        }

        ret
    }
}

/// USB CCID (Smart Card) descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CcidDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub version: Version,
    pub max_slot_index: u8,
    pub voltage_support: u8,
    pub protocols: u32,
    pub default_clock: u32,
    pub max_clock: u32,
    pub num_clock_supported: u8,
    pub data_rate: u32,
    pub max_data_rate: u32,
    pub num_data_rates_supp: u8,
    pub max_ifsd: u32,
    pub sync_protocols: u32,
    pub mechanical: u32,
    pub features: u32,
    pub max_ccid_msg_len: u32,
    pub class_get_response: u8,
    pub class_envelope: u8,
    pub lcd_layout: (u8, u8),
    pub pin_support: u8,
    pub max_ccid_busy_slots: u8,
}

impl TryFrom<&[u8]> for CcidDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 54 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "CCID descriptor too short",
            ));
        }

        let lcd_layout = (value[50], value[51]);

        Ok(CcidDescriptor {
            length: value[0],
            descriptor_type: value[1],
            version: Version::from_bcd(u16::from_le_bytes([value[2], value[3]])),
            max_slot_index: value[4],
            voltage_support: value[5],
            protocols: u32::from_le_bytes([value[6], value[7], value[8], value[9]]),
            default_clock: u32::from_le_bytes([value[10], value[11], value[12], value[13]]),
            max_clock: u32::from_le_bytes([value[14], value[15], value[16], value[17]]),
            num_clock_supported: value[18],
            data_rate: u32::from_le_bytes([value[19], value[20], value[21], value[22]]),
            max_data_rate: u32::from_le_bytes([value[23], value[24], value[25], value[26]]),
            num_data_rates_supp: value[27],
            max_ifsd: u32::from_le_bytes([value[28], value[29], value[30], value[31]]),
            sync_protocols: u32::from_le_bytes([value[32], value[33], value[34], value[35]]),
            mechanical: u32::from_le_bytes([value[36], value[37], value[38], value[39]]),
            features: u32::from_le_bytes([value[40], value[41], value[42], value[43]]),
            max_ccid_msg_len: u32::from_le_bytes([value[44], value[45], value[46], value[47]]),
            class_get_response: value[48],
            class_envelope: value[49],
            lcd_layout,
            pin_support: value[52],
            max_ccid_busy_slots: value[53],
        })
    }
}

impl TryFrom<GenericDescriptor> for CcidDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        CcidDescriptor::try_from(&gd_vec[..])
    }
}

impl From<CcidDescriptor> for Vec<u8> {
    fn from(cd: CcidDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(cd.length);
        ret.push(cd.descriptor_type);
        ret.extend(u16::from(cd.version).to_le_bytes());
        ret.push(cd.max_slot_index);
        ret.push(cd.voltage_support);
        ret.extend(cd.protocols.to_le_bytes());
        ret.extend(cd.default_clock.to_le_bytes());
        ret.extend(cd.max_clock.to_le_bytes());
        ret.push(cd.num_clock_supported);
        ret.extend(cd.data_rate.to_le_bytes());
        ret.extend(cd.max_data_rate.to_le_bytes());
        ret.push(cd.num_data_rates_supp);
        ret.extend(cd.max_ifsd.to_le_bytes());
        ret.extend(cd.sync_protocols.to_le_bytes());
        ret.extend(cd.mechanical.to_le_bytes());
        ret.extend(cd.features.to_le_bytes());
        ret.extend(cd.max_ccid_msg_len.to_le_bytes());
        ret.push(cd.class_get_response);
        ret.push(cd.class_envelope);
        ret.push(cd.lcd_layout.0);
        ret.push(cd.lcd_layout.1);
        ret.push(cd.pin_support);
        ret.push(cd.max_ccid_busy_slots);

        ret
    }
}

/// USB printer descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct PrinterDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub release_number: u8,
    pub descriptors: Vec<PrinterReportDescriptor>,
}

impl TryFrom<&[u8]> for PrinterDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Printer descriptor too short",
            ));
        }

        let num_descriptors = value[3] as usize;
        let mut descriptors_vec = value[4..].to_vec();
        let mut descriptors = Vec::<PrinterReportDescriptor>::with_capacity(num_descriptors);

        for _ in 0..num_descriptors {
            if descriptors_vec.len() < 2 {
                return Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Printer report descriptor too short",
                ));
            }

            // +2 for length and descriptor type
            let len = descriptors_vec[1] as usize + 2;

            if descriptors_vec.len() < len {
                break;
            }

            descriptors.push(descriptors_vec.drain(..len).as_slice().try_into()?);
        }

        Ok(PrinterDescriptor {
            length: value[0],
            descriptor_type: value[1],
            release_number: value[2],
            descriptors,
        })
    }
}

impl TryFrom<GenericDescriptor> for PrinterDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        PrinterDescriptor::try_from(&gd_vec[..])
    }
}

impl From<PrinterDescriptor> for Vec<u8> {
    fn from(pd: PrinterDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(pd.length);
        ret.push(pd.descriptor_type);
        ret.push(pd.release_number);
        for desc in pd.descriptors {
            ret.extend(Vec::<u8>::from(desc));
        }

        ret
    }
}

/// USB printer report descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct PrinterReportDescriptor {
    pub descriptor_type: u8,
    pub length: u8,
    pub capabilities: u16,
    pub versions_supported: u8,
    pub uuid_string_index: u8,
    pub uuid_string: Option<String>,
    pub data: Option<Vec<u8>>,
}

impl TryFrom<&[u8]> for PrinterReportDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Printer report descriptor too short",
            ));
        }

        Ok(PrinterReportDescriptor {
            descriptor_type: value[0],
            length: value[1],
            capabilities: u16::from_le_bytes([value[2], value[3]]),
            versions_supported: value[4],
            uuid_string_index: value[5],
            uuid_string: None,
            data: value.get(6..).map(|d| d.to_vec()),
        })
    }
}

impl From<PrinterReportDescriptor> for Vec<u8> {
    fn from(prd: PrinterReportDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(prd.descriptor_type);
        ret.push(prd.length);
        ret.extend(prd.capabilities.to_le_bytes());
        ret.push(prd.versions_supported);
        ret.push(prd.uuid_string_index);

        ret
    }
}

/// USB Communication Device Class (CDC) types
///
/// Used to differentiate between different CDC descriptors
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum CdcType {
    Header = 0x00,
    CallManagement = 0x01,
    AbstractControlManagement = 0x02,
    DirectLineManagement = 0x03,
    TelephoneRinger = 0x04,
    TelephoneCall = 0x05,
    Union = 0x06,
    CountrySelection = 0x07,
    TelephoneOperationalModes = 0x08,
    UsbTerminal = 0x09,
    NetworkChannel = 0x0a,
    ProtocolUnit = 0x0b,
    ExtensionUnit = 0x0c,
    MultiChannel = 0x0d,
    CapiControl = 0x0e,
    EthernetNetworking = 0x0f,
    AtmNetworking = 0x10,
    WirelessHandsetControlModel = 0x11,
    MobileDirectLineModelFunctional = 0x12,
    MobileDirectLineModelDetail = 0x13,
    DeviceManagement = 0x14,
    Obex = 0x15,
    CommandSet = 0x16,
    CommandSetDetail = 0x17,
    TelephoneControlModel = 0x18,
    ObexCommandSet = 0x19,
    Ncm = 0x1a,
    Mbim = 0x1b,
    MbimExtended = 0x1c,
    Unknown = 0xff,
}

impl From<u8> for CdcType {
    fn from(b: u8) -> Self {
        match b {
            0x00 => CdcType::Header,
            0x01 => CdcType::CallManagement,
            0x02 => CdcType::AbstractControlManagement,
            0x03 => CdcType::DirectLineManagement,
            0x04 => CdcType::TelephoneRinger,
            0x05 => CdcType::TelephoneCall,
            0x06 => CdcType::Union,
            0x07 => CdcType::CountrySelection,
            0x08 => CdcType::TelephoneOperationalModes,
            0x09 => CdcType::UsbTerminal,
            0x0a => CdcType::NetworkChannel,
            0x0b => CdcType::ProtocolUnit,
            0x0c => CdcType::ExtensionUnit,
            0x0d => CdcType::MultiChannel,
            0x0e => CdcType::CapiControl,
            0x0f => CdcType::EthernetNetworking,
            0x10 => CdcType::AtmNetworking,
            0x11 => CdcType::WirelessHandsetControlModel,
            0x12 => CdcType::MobileDirectLineModelFunctional,
            0x13 => CdcType::MobileDirectLineModelDetail,
            0x14 => CdcType::DeviceManagement,
            0x15 => CdcType::Obex,
            0x16 => CdcType::CommandSet,
            0x17 => CdcType::CommandSetDetail,
            0x18 => CdcType::TelephoneControlModel,
            0x19 => CdcType::ObexCommandSet,
            0x1a => CdcType::Ncm,
            0x1b => CdcType::Mbim,
            0x1c => CdcType::MbimExtended,
            _ => CdcType::Unknown,
        }
    }
}

/// USB Communication Device Class (CDC) descriptor
///
/// Can be used by CDCData and CDCCommunications
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CommunicationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub communication_type: CdcType,
    pub string_index: Option<u8>,
    pub string: Option<String>,
    pub data: Vec<u8>,
}

impl TryFrom<&[u8]> for CommunicationDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Communication descriptor too short",
            ));
        }

        let length = value[0];
        if length as usize > value.len() {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Communication descriptor reported length too long for buffer",
            ));
        }

        let communication_type = CdcType::from(value[2]);
        // some CDC types have descriptor strings with index in the data
        let string_index = match communication_type {
            CdcType::EthernetNetworking | CdcType::CountrySelection => value.get(3).map(|v| v.to_owned()),
            CdcType::NetworkChannel => value.get(4).map(|v| v.to_owned()),
            CdcType::CommandSet => value.get(5).map(|v| v.to_owned()),
            _ => None
        };

        Ok(CommunicationDescriptor {
            length,
            descriptor_type: value[1],
            communication_type,
            string_index,
            string: None,
            data: value[3..].to_vec(),
        })
    }
}

impl From<CommunicationDescriptor> for Vec<u8> {
    fn from(cd: CommunicationDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(cd.length);
        ret.push(cd.descriptor_type);
        ret.push(cd.communication_type as u8);
        ret.extend(cd.data);

        ret
    }
}

impl TryFrom<GenericDescriptor> for CommunicationDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        CommunicationDescriptor::try_from(&gd_vec[..])
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum UacInterface {
    Undefined = 0x00,
    Header = 0x01,
    InputTerminal = 0x02,
    OutputTerminal = 0x03,
    ExtendedTerminal = 0x04,
    MixerUnit = 0x05,
    SelectorUnit = 0x06,
    FeatureUnit = 0x07,
    EffectUnit = 0x08,
    ProcessingUnit = 0x09,
    ExtensionUnit = 0x0a,
    ClockSource = 0x0b,
    ClockSelector = 0x0c,
    ClockMultiplier = 0x0d,
    SampleRateConverter = 0x0e,
    Connectors = 0x0f,
    PowerDomain = 0x10,
}

impl From<u8> for UacInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UacInterface::Undefined,
            0x01 => UacInterface::Header,
            0x02 => UacInterface::InputTerminal,
            0x03 => UacInterface::OutputTerminal,
            0x04 => UacInterface::ExtendedTerminal,
            0x05 => UacInterface::MixerUnit,
            0x06 => UacInterface::SelectorUnit,
            0x07 => UacInterface::FeatureUnit,
            0x08 => UacInterface::EffectUnit,
            0x09 => UacInterface::ProcessingUnit,
            0x0a => UacInterface::ExtensionUnit,
            0x0b => UacInterface::ClockSource,
            0x0c => UacInterface::ClockSelector,
            0x0d => UacInterface::ClockMultiplier,
            0x0e => UacInterface::SampleRateConverter,
            0x0f => UacInterface::Connectors,
            0x10 => UacInterface::PowerDomain,
            _ => UacInterface::Undefined,
        }
    }
}

impl UacInterface {
    /// UAC1, UAC2, and UAC3 define bDescriptorSubtype differently for the
    /// AudioControl interface, so we need to do some ugly remapping:
    pub fn get_uac_subtype(subtype: u8, protocol: u8) -> Self {
        match protocol {
            // UAC1
            0x00 => {
                match subtype {
                    0x04 => UacInterface::MixerUnit,
                    0x05 => UacInterface::SelectorUnit,
                    0x06 => UacInterface::FeatureUnit,
                    0x07 => UacInterface::ProcessingUnit,
                    0x08 => UacInterface::ExtensionUnit,
                    _ => Self::from(subtype),
                }
            },
            // UAC2
            0x20 => {
                match subtype {
                    0x04 => UacInterface::MixerUnit,
                    0x05 => UacInterface::SelectorUnit,
                    0x06 => UacInterface::FeatureUnit,
                    0x07 => UacInterface::EffectUnit,
                    0x08 => UacInterface::ProcessingUnit,
                    0x09 => UacInterface::ExtensionUnit,
                    0x0a => UacInterface::ClockSource,
                    0x0b => UacInterface::ClockSelector,
                    0x0c => UacInterface::ClockMultiplier,
                    0x0d => UacInterface::SampleRateConverter,
                    _ => Self::from(subtype),
                }
            },
            // no re-map for UAC3..
            _ => {
                Self::from(subtype)
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[non_exhaustive]
pub enum VideoControlInterface {
    Undefined = 0x00,
    Header = 0x01,
    InputTerminal = 0x02,
    OutputTerminal = 0x03,
    SelectorUnit = 0x04,
    ProcessingUnit = 0x05,
    ExtensionUnit = 0x06,
    EncodingUnit = 0x07,
}

impl From<u8> for VideoControlInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => VideoControlInterface::Undefined,
            0x01 => VideoControlInterface::Header,
            0x02 => VideoControlInterface::InputTerminal,
            0x03 => VideoControlInterface::OutputTerminal,
            0x04 => VideoControlInterface::SelectorUnit,
            0x05 => VideoControlInterface::ProcessingUnit,
            0x06 => VideoControlInterface::ExtensionUnit,
            0x07 => VideoControlInterface::EncodingUnit,
            _ => VideoControlInterface::Undefined,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct VideoControlDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_subtype: u8,
    pub string_index: Option<u8>,
    pub string: Option<String>,
    pub data: Vec<u8>,
}

impl TryFrom<&[u8]> for VideoControlDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Video Control descriptor too short",
            ));
        }

        let length = value[0];
        if length as usize > value.len() {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Video Control descriptor reported length too long for buffer",
            ));
        }

        let video_control_subtype = VideoControlInterface::from(value[2]);

        let string_index = match video_control_subtype {
            VideoControlInterface::InputTerminal => value.get(7).map(|v| *v),
            VideoControlInterface::OutputTerminal => value.get(8).map(|v| *v),
            VideoControlInterface::SelectorUnit => {
                if let Some(p) = value.get(4) {
                    value.get(5 + *p as usize).map(|v| *v)
                } else {
                    None
                }
            }
            VideoControlInterface::ProcessingUnit => {
                if let Some(n) = value.get(7) {
                    value.get(8 + *n as usize).map(|v| *v)
                } else {
                    None
                }
            }
            VideoControlInterface::ExtensionUnit => {
                if let Some(p) = value.get(21) {
                    if let Some(n) = value.get(22 + *p as usize) {
                        value.get(23 + *n as usize + *p as usize).map(|v| *v)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            VideoControlInterface::EncodingUnit => value.get(5).map(|v| *v),
            _ => None
        };

        Ok(VideoControlDescriptor {
            length,
            descriptor_type: value[1],
            descriptor_subtype: value[2],
            string_index,
            string: None,
            data: value[3..].to_vec(),
        })
    }
}

impl From<VideoControlDescriptor> for Vec<u8> {
    fn from(vcd: VideoControlDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(vcd.length);
        ret.push(vcd.descriptor_type);
        ret.push(vcd.descriptor_subtype);
        ret.extend(vcd.data);

        ret
    }
}

impl TryFrom<GenericDescriptor> for VideoControlDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        VideoControlDescriptor::try_from(&gd_vec[..])
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[non_exhaustive]
pub enum MidiInterface {
    Undefined = 0x00,
    Header = 0x01,
    InputJack = 0x02,
    OutputJack = 0x03,
    Element = 0x04,
}

impl From<u8> for MidiInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => MidiInterface::Undefined,
            0x01 => MidiInterface::Header,
            0x02 => MidiInterface::InputJack,
            0x03 => MidiInterface::OutputJack,
            0x04 => MidiInterface::Element,
            _ => MidiInterface::Undefined,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MidiDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub midi_subtype: MidiInterface,
    pub string_index: Option<u8>,
    pub string: Option<String>,
    pub data: Vec<u8>,
}

impl TryFrom<&[u8]> for MidiDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "MidiDescriptor descriptor too short",
            ));
        }

        let length = value[0];
        if length as usize > value.len() {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "MidiDescriptor descriptor reported length too long for buffer",
            ));
        }

        let midi_subtype = MidiInterface::from(value[2]);

        let string_index = match midi_subtype {
            MidiInterface::InputJack => value.get(5).map(|v| *v),
            MidiInterface::OutputJack => value.get(5).map(|v| 6 + *v * 2),
            MidiInterface::Element => {
                // don't ask...
                if let Some(j) = value.get(4) {
                    if let Some(capsize) = value.get((5 + *j as usize * 2) + 3) {
                        value.get(9 + 2 * *j as usize + *capsize as usize).map(|v| *v)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None
        };

        Ok(MidiDescriptor {
            length,
            descriptor_type: value[1],
            midi_subtype,
            string_index,
            string: None,
            data: value[3..].to_vec(),
        })
    }
}

impl From<MidiDescriptor> for Vec<u8> {
    fn from(md: MidiDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(md.length);
        ret.push(md.descriptor_type);
        ret.push(md.midi_subtype as u8);
        ret.extend(md.data);

        ret
    }
}

impl TryFrom<GenericDescriptor> for MidiDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        MidiDescriptor::try_from(&gd_vec[..])
    }
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
