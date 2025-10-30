//! Defines for USB, mainly thosed covered at [usb.org](https://www.usb.org)
//!
//! Also referring to [beyondlogic](https://beyondlogic.org/usbnutshell/usb5.shtml)
//!
//! There are some repeated/copied Enum defines from rusb in order to control Serialize/Deserialize and add impl
use clap::ValueEnum;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::convert::TryFrom;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

pub mod descriptors;
pub use descriptors::*;
pub mod path;
pub use path::*;

use crate::error::{self, Error, ErrorKind};
use crate::profiler::InternalData;
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
                &format!("No two base16 encoded versions in {s}"),
            )),
        }
    }
}

/// For legacy import where I thought the value was a f32...
impl TryFrom<f32> for Version {
    type Error = Error;

    fn try_from(f: f32) -> error::Result<Self> {
        let s = format!("{f:2.2}");
        let (parse_ints, _): (Vec<Result<u8, _>>, Vec<_>) = s
            .split('.')
            .map(|vs| vs.parse::<u8>())
            .partition(Result::is_ok);
        let numbers: Vec<u8> = parse_ints.into_iter().map(|v| v.unwrap()).collect();

        match numbers.get(0..2) {
            Some(slice) => Ok(Version(slice[0], (slice[1] & 0xF0) >> 4, slice[1] & 0x0F)),
            None => Err(Error::new(
                ErrorKind::Decoding,
                &format!("Failed to parse float into MM.mP {f}"),
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
    /// Device is bus powered
    BusPowered,
    /// Device powers itself not from bus
    SelfPowered,
    /// Supports remote wake-up
    RemoteWakeup,
    /// Device is battery powered
    BatteryPowered,
}

impl fmt::Display for ConfigAttributes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
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

/// Explains how the `BaseClass` is used
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
/// Technically this is the 'Base Class' - the 'Class Code' is the full triplet of (Base Class, Sub Class, Protocol).
#[derive(Debug, ValueEnum, Default, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
#[repr(u8)]
pub enum BaseClass {
    #[default]
    /// Device class is unspecified, interface descriptors are used to determine needed drivers
    UseInterfaceDescriptor = 0x00,
    /// Speaker, microphone, sound card, MIDI
    Audio = 0x01,
    /// The modern serial interface; appears as a UART/RS232 port on most systems
    #[serde(alias = "c-d-c-communications", alias = "CDC Communications")]
    CdcCommunications = 0x02,
    /// Human Interface Device; game controllers, keyboards, mice etc. Also commonly used as a device data interface rather then creating something from scratch
    #[serde(alias = "h-i-d", alias = "HID")]
    Hid = 0x03,
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
    #[serde(alias = "c-d-c-data", alias = "CDC Data")]
    CdcData = 0x0a,
    /// USB smart card reader
    SmartCard = 0x0b,
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
    #[serde(alias = "u-s-b-type-c-bridge", alias = "USB TypeC Bridge")]
    UsbTypeCBridge = 0x12,
    /// This base class is defined for devices that conform to the “VESA USB BDP Device Specification” found at the VESA website. This specification defines the usable set of SubClass and Protocol values. Values outside of this defined spec are reserved. These class codes can only be used in Interface Descriptors.
    #[serde(alias = "b-d-p", alias = "BDP")]
    Bdp = 0x13,
    /// This base class is defined for devices that conform to the “MCTP over USB” found at the DMTF website as DSP0283. This specification defines the usable set of SubClass and Protocol values. Values outside of this defined spec are reserved. These class codes can only be used in Interface Descriptors.
    #[serde(alias = "m-c-t-p", alias = "MCTP")]
    Mctp = 0x14,
    /// An interface to expose and configure I3C function within a USB device to allow interaction between host software and the I3C device, to drive transaction on the I3C bus to/from target devices
    #[serde(alias = "i-3-c-device", alias = "I3C Device")]
    I3cDevice = 0x3c,
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

impl fmt::Display for BaseClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<u8> for BaseClass {
    fn from(b: u8) -> BaseClass {
        match b {
            0x00 => BaseClass::UseInterfaceDescriptor,
            0x01 => BaseClass::Audio,
            0x02 => BaseClass::CdcCommunications,
            0x03 => BaseClass::Hid,
            0x05 => BaseClass::Physical,
            0x06 => BaseClass::Image,
            0x07 => BaseClass::Printer,
            0x08 => BaseClass::MassStorage,
            0x09 => BaseClass::Hub,
            0x0a => BaseClass::CdcData,
            0x0b => BaseClass::SmartCard,
            0x0d => BaseClass::ContentSecurity,
            0x0e => BaseClass::Video,
            0x0f => BaseClass::PersonalHealthcare,
            0x10 => BaseClass::AudioVideo,
            0x11 => BaseClass::Billboard,
            0x12 => BaseClass::UsbTypeCBridge,
            0x13 => BaseClass::Bdp,
            0x14 => BaseClass::Mctp,
            0x3c => BaseClass::I3cDevice,
            0xdc => BaseClass::Diagnostic,
            0xe0 => BaseClass::WirelessController,
            0xef => BaseClass::Miscellaneous,
            0xfe => BaseClass::ApplicationSpecificInterface,
            0xff => BaseClass::VendorSpecificClass,
            _ => BaseClass::UseInterfaceDescriptor,
        }
    }
}

impl From<BaseClass> for u8 {
    fn from(val: BaseClass) -> Self {
        // set as repr(u8) so this will do the conversion
        val as u8
    }
}

impl From<ClassCode> for BaseClass {
    fn from(c: ClassCode) -> Self {
        match c {
            ClassCode::Generic(c) => c,
            ClassCode::FullSpeedHub => BaseClass::Hub,
            ClassCode::HighSpeedHubSingleTt => BaseClass::Hub,
            ClassCode::HighSpeedHubMultiTt => BaseClass::Hub,
            ClassCode::AudioVideoControlInterface => BaseClass::Audio,
            ClassCode::AudioVideoDataVideo => BaseClass::Audio,
            ClassCode::AudioVideoDataAudio => BaseClass::Audio,
            ClassCode::MctpManagementController => BaseClass::Mctp,
            ClassCode::MctpHostInterfaceEndpoint => BaseClass::Mctp,
            ClassCode::Usb2ComplianceDevice => BaseClass::Diagnostic,
            ClassCode::DebugTargetVendorDefined => BaseClass::Diagnostic,
            ClassCode::GnuRemoteDebugCommandSet => BaseClass::Diagnostic,
            ClassCode::VendorDefinedTraceDbC => BaseClass::Diagnostic,
            ClassCode::VendorDefinedDfxDbC => BaseClass::Diagnostic,
            ClassCode::VendorDefinedTraceGPDvC => BaseClass::Diagnostic,
            ClassCode::GnuProtocolGpDvC => BaseClass::Diagnostic,
            ClassCode::VendorDefinedDfxDvC => BaseClass::Diagnostic,
            ClassCode::VendorDefinedTraceDvC => BaseClass::Diagnostic,
            ClassCode::BluetoothProgrammingInterface => BaseClass::WirelessController,
            ClassCode::UwbRadioControlInterface => BaseClass::WirelessController,
            ClassCode::RemoteNdis => BaseClass::WirelessController,
            ClassCode::BluetoothAmpController => BaseClass::WirelessController,
            ClassCode::HostWireAdaptor => BaseClass::WirelessController,
            ClassCode::DeviceWireAdaptor => BaseClass::WirelessController,
            ClassCode::DeviceWireAdaptorIsochronous => BaseClass::WirelessController,
            ClassCode::ActiveSync => BaseClass::Miscellaneous,
            ClassCode::PalmSync => BaseClass::Miscellaneous,
            ClassCode::InterfaceAssociationDescriptor => BaseClass::Miscellaneous,
            ClassCode::WireAdaptorMultifunctionPeripheral => BaseClass::Miscellaneous,
            ClassCode::CableBasedAssociationFramework => BaseClass::Miscellaneous,
            ClassCode::RndisOverEthernet => BaseClass::Miscellaneous,
            ClassCode::RndisOverWifi => BaseClass::Miscellaneous,
            ClassCode::RndisOverWiMax => BaseClass::Miscellaneous,
            ClassCode::RndisOverWwan => BaseClass::Miscellaneous,
            ClassCode::RndisForRawIpv4 => BaseClass::Miscellaneous,
            ClassCode::RndisForRawIpv6 => BaseClass::Miscellaneous,
            ClassCode::RndisForGprs => BaseClass::Miscellaneous,
            ClassCode::Usb3VisionControlInterface => BaseClass::Miscellaneous,
            ClassCode::Usb3VisionEventInterface => BaseClass::Miscellaneous,
            ClassCode::Usb3VisionStreamingInterface => BaseClass::Miscellaneous,
            ClassCode::StepStreamTransport => BaseClass::Miscellaneous,
            ClassCode::StepRawStreamTransport => BaseClass::Miscellaneous,
            ClassCode::CommandInterfaceIad => BaseClass::Miscellaneous,
            ClassCode::CommandInterfaceId => BaseClass::Miscellaneous,
            ClassCode::MediaInterfaceId => BaseClass::Miscellaneous,
            ClassCode::DeviceFirmwareUpgrade => BaseClass::ApplicationSpecificInterface,
            ClassCode::IrdaBridge => BaseClass::ApplicationSpecificInterface,
            ClassCode::UsbTestMeasurement => BaseClass::ApplicationSpecificInterface,
            ClassCode::UsbTestMeasurementUsbTmc488 => BaseClass::ApplicationSpecificInterface,
        }
    }
}

impl BaseClass {
    /// How the BaseClass is used [`DescriptorUsage`]
    pub fn usage(&self) -> DescriptorUsage {
        match self {
            BaseClass::UseInterfaceDescriptor | BaseClass::Hub | BaseClass::Billboard => {
                DescriptorUsage::Device
            }
            BaseClass::CdcCommunications
            | BaseClass::Diagnostic
            | BaseClass::Miscellaneous
            | BaseClass::VendorSpecificClass => DescriptorUsage::Both,
            _ => DescriptorUsage::Interface,
        }
    }

    /// lsusb is explicit for some in styling of tree
    /// ```
    /// # use cyme::usb::BaseClass;
    ///
    /// assert_eq!(BaseClass::Hid.to_lsusb_string(), "Human Interface Device");
    /// ```
    pub fn to_lsusb_string(&self) -> String {
        match self {
            BaseClass::Hid => "Human Interface Device".into(),
            BaseClass::CdcCommunications => "Communications".into(),
            _ => self.to_title_case(),
        }
    }

    /// Converts Pascal case enum to space separated on capitals
    /// ```
    /// # use cyme::usb::BaseClass;
    ///
    /// assert_eq!(BaseClass::UseInterfaceDescriptor.to_title_case(), "Use Interface Descriptor");
    /// assert_eq!(BaseClass::CdcData.to_title_case(), "CDC Data");
    /// ```
    pub fn to_title_case(&self) -> String {
        let title = heck::AsTitleCase(self.to_string()).to_string();
        let split: Vec<&str> = title.split(' ').collect();
        let first = split.first().unwrap_or(&"");

        // keep capitalised abbreviations
        match first.to_owned() {
            "Cdc" | "Usb" | "I3c" | "Hid" | "Bdp" | "Mctp" => {
                title.replace(first, &first.to_uppercase())
            }
            _ => title,
        }
    }
}

impl From<BaseClass> for DescriptorUsage {
    fn from(c: BaseClass) -> DescriptorUsage {
        c.usage()
    }
}

/// Fully defined USB-IF class based on (Base Class, Sub Class, Protocol) Class Code triplet
///
/// <https://www.usb.org/defined-class-codes>
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ClassCode {
    /// Generic devices just have a 'Base Class'. It is a device without a defining SubClass or Protocol
    Generic(BaseClass),
    /// Full speed Hub
    FullSpeedHub,
    /// Hi-speed hub with single TT
    HighSpeedHubSingleTt,
    /// Hi-speed hub with multiple TTs
    HighSpeedHubMultiTt,
    /// Audio/Video Device – AVControl Interface
    AudioVideoControlInterface,
    /// Audio/Video Device – AVData Video Streaming Interface
    AudioVideoDataVideo,
    /// Audio/Video Device – AVData Audio Streaming Interface
    AudioVideoDataAudio,
    /// MCTP Management-controller and Managed-Device endpoints
    MctpManagementController,
    /// MCTP Host Interface endpoint
    MctpHostInterfaceEndpoint,
    /// USB2 Compliance Device. Definition for this device can be found at <http://www.intel.com/technology/usb/spec.htm>
    Usb2ComplianceDevice,
    /// Debug Target vendor defined. Please see <http://www.intel.com/content/www/us/en/io/universal-serial-bus/extensible-host-controler-interface-usb-xhci.html> for more info.
    DebugTargetVendorDefined,
    /// GNU Remote Debug Command Set. Please see <http://www.intel.com/content/www/us/en/io/universal-serial-bus/extensible-host-controler-interface-usb-xhci.html> for more info.
    GnuRemoteDebugCommandSet,
    /// Vendor defined Trace protocol on DbC.
    VendorDefinedTraceDbC,
    /// Vendor defined Dfx protocol on DbC.
    VendorDefinedDfxDbC,
    /// Vendor defined Trace protocol over General Purpose (GP) endpoint on DvC.
    VendorDefinedTraceGPDvC,
    /// GNU Protocol protocol over General Purpose (GP) endpoint on DvC.
    ///
    /// <http://www.gnu.org/software/gdb/>
    GnuProtocolGpDvC,
    /// Vendor defined Dfx protocol on DvC.
    VendorDefinedDfxDvC,
    /// Vendor defined Trace protocol on DvC.
    VendorDefinedTraceDvC,
    /// Bluetooth Programming Interface. Get specific information from www.bluetooth.com.
    BluetoothProgrammingInterface,
    /// UWB Radio Control Interface. Definition for this is found in the Wireless USB Specification in Chapter 8.
    UwbRadioControlInterface,
    /// Remote NDIS. Information can be found at: <http://www.microsoft.com/windowsmobile/mobileoperators/default.mspx>
    RemoteNdis,
    /// Bluetooth AMP Controller. Get specific information from www.bluetooth.com.
    BluetoothAmpController,
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
    RndisOverEthernet,
    /// RNDIS over WiFi.
    ///
    /// Connecting a host to the Internet via WiFi enabled mobile device. The device represents itself to the host as an 802.11 compliant network device. This class code may only be used in Interface Descriptors.
    RndisOverWifi,
    /// RNDIS over WiMAX
    ///
    /// Connecting a host to the Internet via WiMAX enabled mobile device. The device is represented to the host as an 802.16 network device.
    ///
    /// This class code may only be used in Interface Descriptors.
    RndisOverWiMax,
    /// RNDIS over WWAN
    ///
    /// Connecting a host to the Internet via a device using mobile broadband, i.e. WWAN (GSM/CDMA).
    ///
    /// This class code may only be used in Interface Descriptors.
    RndisOverWwan,
    /// RNDIS for Raw IPv4
    ///
    /// Connecting a host to the Internet using raw IPv4 via non-Ethernet mobile device. Devices that provide raw IPv4, not in an Ethernet packet, may use this form to in lieu of other stock types.
    ///
    /// This class code may only be used in Interface Descriptors.
    RndisForRawIpv4,
    /// RNDIS for Raw IPv6
    ///
    /// Connecting a host to the Internet using raw IPv6 via non-Ethernet mobile device. Devices that provide raw IPv6, not in an Ethernet packet, may use this form to in lieu of other stock types.
    ///
    /// This class code may only be used in Interface Descriptors.
    RndisForRawIpv6,
    /// RNDIS for GPRS
    ///
    /// Connecting a host to the Internet over GPRS mobile device using the device’s cellular radio
    RndisForGprs,
    /// USB3 Vision Control Interface
    Usb3VisionControlInterface,
    /// USB3 Vision Event Interface
    Usb3VisionEventInterface,
    /// USB3 Vision Streaming Interface
    Usb3VisionStreamingInterface,
    /// STEP. Stream Transport Efficient Protocol for content protection.
    StepStreamTransport,
    /// STEP RAW. Stream Transport Efficient Protocol for Raw content protection.
    StepRawStreamTransport,
    /// Command Interface in IAD
    CommandInterfaceIad,
    /// Command Interface in Interface Descriptor
    CommandInterfaceId,
    /// Media Interface in Interface Descriptor
    MediaInterfaceId,
    /// Device Firmware Upgrade. Device class definition provided on www.usb.org .
    DeviceFirmwareUpgrade,
    /// IRDA Bridge device. Device class definition provided on www.usb.org .
    IrdaBridge,
    /// USB Test and Measurement Device. Definition provided in the USB Test and Measurement Class spec found on www.usb.org .
    UsbTestMeasurement,
    /// USB Test and Measurement Device conforming to the USBTMC USB488 Subclass Specification found on www.usb.org.
    UsbTestMeasurementUsbTmc488,
}

/// A fully defined Class Code requires a (Base Class, Sub Class, Protocol) triplet
pub type ClassCodeTriplet<T> = (T, u8, u8);

impl<T> From<ClassCodeTriplet<T>> for ClassCode
where
    T: Into<BaseClass>,
{
    fn from(triplet: ClassCodeTriplet<T>) -> Self {
        match (triplet.0.into(), triplet.1, triplet.2) {
            (BaseClass::Hub, 0x00, 0x00) => ClassCode::FullSpeedHub,
            (BaseClass::Hub, 0x00, 0x01) => ClassCode::HighSpeedHubSingleTt,
            (BaseClass::Hub, 0x00, 0x02) => ClassCode::HighSpeedHubMultiTt,
            (BaseClass::Audio, 0x01, 0x00) => ClassCode::AudioVideoControlInterface,
            (BaseClass::Audio, 0x02, 0x00) => ClassCode::AudioVideoDataVideo,
            (BaseClass::Audio, 0x03, 0x00) => ClassCode::AudioVideoDataAudio,
            (BaseClass::Mctp, 0x00, 0x01) => ClassCode::MctpManagementController,
            (BaseClass::Mctp, 0x00, 0x02) => ClassCode::MctpHostInterfaceEndpoint,
            (BaseClass::Diagnostic, 0x01, 0x01) => ClassCode::Usb2ComplianceDevice,
            (BaseClass::Diagnostic, 0x02, 0x00) => ClassCode::DebugTargetVendorDefined,
            (BaseClass::Diagnostic, 0x02, 0x01) => ClassCode::GnuRemoteDebugCommandSet,
            (BaseClass::Diagnostic, 0x03, 0x01) => ClassCode::VendorDefinedTraceDbC,
            (BaseClass::Diagnostic, 0x04, 0x01) => ClassCode::VendorDefinedDfxDbC,
            (BaseClass::Diagnostic, 0x05, 0x00) => ClassCode::VendorDefinedTraceGPDvC,
            (BaseClass::Diagnostic, 0x05, 0x01) => ClassCode::GnuProtocolGpDvC,
            (BaseClass::Diagnostic, 0x06, 0x01) => ClassCode::VendorDefinedDfxDvC,
            (BaseClass::Diagnostic, 0x07, 0x01) => ClassCode::VendorDefinedTraceDvC,
            (BaseClass::WirelessController, 0x01, 0x01) => ClassCode::BluetoothProgrammingInterface,
            (BaseClass::WirelessController, 0x01, 0x02) => ClassCode::UwbRadioControlInterface,
            (BaseClass::WirelessController, 0x01, 0x03) => ClassCode::RemoteNdis,
            (BaseClass::WirelessController, 0x01, 0x04) => ClassCode::BluetoothAmpController,
            (BaseClass::WirelessController, 0x02, 0x01) => ClassCode::HostWireAdaptor,
            (BaseClass::WirelessController, 0x02, 0x02) => ClassCode::DeviceWireAdaptor,
            (BaseClass::WirelessController, 0x02, 0x03) => ClassCode::DeviceWireAdaptorIsochronous,
            (BaseClass::Miscellaneous, 0x01, 0x01) => ClassCode::ActiveSync,
            (BaseClass::Miscellaneous, 0x01, 0x02) => ClassCode::PalmSync,
            (BaseClass::Miscellaneous, 0x02, 0x01) => ClassCode::InterfaceAssociationDescriptor,
            (BaseClass::Miscellaneous, 0x02, 0x02) => ClassCode::WireAdaptorMultifunctionPeripheral,
            (BaseClass::Miscellaneous, 0x03, 0x01) => ClassCode::CableBasedAssociationFramework,
            (BaseClass::Miscellaneous, 0x04, 0x01) => ClassCode::RndisOverEthernet,
            (BaseClass::Miscellaneous, 0x04, 0x02) => ClassCode::RndisOverWifi,
            (BaseClass::Miscellaneous, 0x04, 0x03) => ClassCode::RndisOverWiMax,
            (BaseClass::Miscellaneous, 0x04, 0x04) => ClassCode::RndisOverWwan,
            (BaseClass::Miscellaneous, 0x04, 0x05) => ClassCode::RndisForRawIpv4,
            (BaseClass::Miscellaneous, 0x04, 0x06) => ClassCode::RndisForRawIpv6,
            (BaseClass::Miscellaneous, 0x04, 0x07) => ClassCode::RndisForGprs,
            (BaseClass::Miscellaneous, 0x05, 0x00) => ClassCode::Usb3VisionControlInterface,
            (BaseClass::Miscellaneous, 0x05, 0x01) => ClassCode::Usb3VisionEventInterface,
            (BaseClass::Miscellaneous, 0x05, 0x02) => ClassCode::Usb3VisionStreamingInterface,
            (BaseClass::Miscellaneous, 0x06, 0x01) => ClassCode::StepStreamTransport,
            (BaseClass::Miscellaneous, 0x06, 0x02) => ClassCode::StepRawStreamTransport,
            // (BaseClass::Miscellaneous, 0x07, 0x01) => DeviceClass::CommandInterfaceIAD,
            (BaseClass::Miscellaneous, 0x07, 0x01) => ClassCode::CommandInterfaceId,
            (BaseClass::Miscellaneous, 0x07, 0x02) => ClassCode::MediaInterfaceId,
            (BaseClass::ApplicationSpecificInterface, 0x01, 0x01) => {
                ClassCode::DeviceFirmwareUpgrade
            }
            (BaseClass::ApplicationSpecificInterface, 0x02, 0x00) => ClassCode::IrdaBridge,
            (BaseClass::ApplicationSpecificInterface, 0x03, 0x00) => ClassCode::UsbTestMeasurement,
            (BaseClass::ApplicationSpecificInterface, 0x03, 0x01) => {
                ClassCode::UsbTestMeasurementUsbTmc488
            }
            (c, _, _) => ClassCode::Generic(c),
        }
    }
}

impl fmt::Display for ClassCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<BaseClass> for ClassCode {
    fn from(class: BaseClass) -> Self {
        ClassCode::Generic(class)
    }
}

impl ClassCode {
    // TODO ensure this is correct
    fn usage(&self) -> DescriptorUsage {
        match self {
            ClassCode::Generic(c) => c.usage(),
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
    SuperSpeedPlusX2,
}

impl FromStr for Speed {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        Ok(match s {
            "20000" | "20.0 Gb/s" | "super_speed_plus_plus" | "super++" => Speed::SuperSpeedPlusX2,
            "10000" | "10.0 Gb/s" | "super_speed_plus" | "super+" => Speed::SuperSpeedPlus,
            "5000" | "5.0 Gb/s" | "super_speed" | "super" => Speed::SuperSpeed,
            "480" | "480.0 Mb/s" | "high_speed" | "high_bandwidth" | "high" => Speed::HighSpeed,
            "12" | "12.0 Mb/s" | "full_speed" | "full" => Speed::FullSpeed,
            "1.5" | "1.5 Mb/s" | "low_speed" | "low" => Speed::LowSpeed,
            _ => Speed::Unknown,
        })
    }
}

/// Convert from byte returned from device descriptor
impl From<u8> for Speed {
    fn from(b: u8) -> Self {
        match b {
            6 => Speed::SuperSpeedPlusX2,
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
                Speed::SuperSpeedPlusX2 => "super_speed_plus_plus",
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
            Speed::SuperSpeedPlusX2 => NumericalUnit {
                value: 20.0,
                unit: String::from("Gb/s"),
                description: Some(speed.to_string()),
            },
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

/// Transfer and [`Endpoint`] direction
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
            write!(f, "{self:?}")
        }
    }
}

/// Transfer type  for [`Endpoint`]
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
        write!(f, "{self:?}")
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

/// Isochronous synchronization mode for [`Endpoint`]
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
        write!(f, "{self:?}")
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

/// Isochronous usage type for [`Endpoint`]
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
        write!(f, "{self:?}")
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

/// Address information for a [`Endpoint`]
// This struct could be one byte with getters using mask but this saves a custom Serialize impl for system_profiler
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointAddress {
    /// Endpoint address byte
    pub address: u8,
    /// Endpoint number on [`Interface`] 0..3b
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

impl From<EndpointPath> for EndpointAddress {
    fn from(path: EndpointPath) -> Self {
        path.endpoint_address()
    }
}

impl fmt::Display for EndpointAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "EP {} {}", self.number, self.direction)
    }
}

/// Endpoint for a [`Interface`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
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
    /// Raw maximum packet size value of endpoint 'wMaxPacketSize' field - encoded with multiplier, use `max_packet_string` for packet information
    pub max_packet_size: u16,
    /// Interval for polling endpoint data transfers. Value in frame counts. Ignored for Bulk & Control Endpoints. Isochronous must equal 1 and field may range from 1 to 255 for interrupt endpoints.
    pub interval: u8,
    /// Extra descriptors data based on type
    #[serde(default)] // default for legacy json
    pub extra: Option<Vec<Descriptor>>,
    #[serde(skip)]
    pub(crate) internal: InternalData,
    /// Option because of legacy json de compatibility
    ///
    /// Allows lookup back to parent
    pub(crate) endpoint_path: Option<EndpointPath>,
}

/// Deprecated alias for [`Endpoint`]
#[deprecated(since = "2.0.0", note = "Use Endpoint instead")]
pub type USBEndpoint = Endpoint;

impl Endpoint {
    /// Decodes the max packet value into a multiplier and number of bytes like lsusb
    pub fn max_packet_string(&self) -> String {
        format!(
            "{}x {}",
            // packets per microframe
            self.packets_per_microframe(),
            self.max_packet_size & 0x7ff
        )
    }

    /// Returns the maximum packet size in bytes for the endpoint
    pub fn max_packet_size(&self) -> usize {
        (self.max_packet_size & ((1 << 11) - 1)) as usize
    }

    /// For isochronous endpoints at high speed, get the number of packets per microframe (1, 2, or 3).
    pub fn packets_per_microframe(&self) -> u8 {
        ((self.max_packet_size >> 11) & 0b11) as u8 + 1
    }

    /// Returns the attributes byte for the endpoint
    pub fn attributes(&self) -> u8 {
        self.transfer_type.to_owned() as u8
            | ((self.sync_type.to_owned() as u8) << 2)
            | ((self.usage_type.to_owned() as u8) << 4)
    }

    /// Should the endpoint be displayed expanded in a tree
    pub fn is_expanded(&self) -> bool {
        self.internal.expanded
    }

    /// Set the expanded state of the endpoint
    pub fn set_expanded(&mut self, expanded: bool) {
        self.internal.expanded = expanded;
    }

    /// Toggle the expanded state of the endpoint
    pub fn toggle_expanded(&mut self) {
        self.internal.expanded = !self.internal.expanded;
    }

    /// Get [`EndpointPath`] for endpoint which includes [`DevicePath`]
    pub fn endpoint_path(&self) -> Option<EndpointPath> {
        self.endpoint_path.to_owned()
    }
}

/// Interface within a [`Configuration`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    /// Name from descriptor
    pub name: Option<String>,
    /// Index of name string in descriptor - only useful for lsusb verbose print
    #[serde(default)]
    pub string_index: u8,
    /// Interface number
    pub number: u8,
    /// Interface port path - could be generated from device but stored here for ease
    pub path: String,
    /// Class of interface provided by USB IF
    pub class: BaseClass,
    /// Sub-class of interface provided by USB IF
    pub sub_class: u8,
    /// Prototol code for interface provided by USB IF
    pub protocol: u8,
    /// Interfaces can have the same number/path but an alternate setting defined here
    pub alt_setting: u8,
    /// Driver obtained from udev on Linux only
    pub driver: Option<String>,
    /// syspath obtained from udev on Linux only
    pub syspath: Option<String>,
    /// An interface can have many endpoints
    pub endpoints: Vec<Endpoint>,
    /// Size of interface descriptor in bytes
    #[serde(default = "default_interface_desc_length")]
    pub length: u8,
    /// Extra descriptors for interface based on type
    #[serde(default)] // default for legacy json
    pub extra: Option<Vec<Descriptor>>,
    #[serde(skip)]
    pub(crate) internal: InternalData,
    /// [`DevicePath`] to interface
    ///
    /// This is option for legacy json de compatibility. In hindsight this would have been used and syspath, path derived from it
    pub(crate) device_path: Option<DevicePath>,
}

/// Deprecated alias for [`Interface`]
#[deprecated(since = "2.0.0", note = "Use Interface instead")]
pub type USBInterface = Interface;

impl Interface {
    /// Linux sysfs name of [`Interface`]
    ///
    /// The port path with config.interface, for example '1-1.2:1.0'
    pub fn sysfs_name(&self) -> String {
        self.path.to_owned()
    }

    /// Linux sysfs path to [`Interface`]
    ///
    /// The [`sysfs_name`] with the sysfs path prefix from udev on Linux, else None
    pub fn sysfs_path(&self) -> Option<PathBuf> {
        self.syspath.as_ref().map(PathBuf::from)
    }

    /// [`DevicePath`] to interface
    ///
    /// Option for legacy json deserialize compatibility - should be present in > 2.1.3. Will attempt to parse from `path` if not present
    pub fn device_path(&self) -> Option<DevicePath> {
        // will be present unless legacy json import
        if let Some(ref dp) = self.device_path {
            Some(dp.to_owned())
        } else {
            // try to parse from path
            let mut dp = DevicePath::from_str(&self.path).ok()?;
            // set alt setting since not in str path
            dp.set_alt_setting(self.alt_setting);
            Some(dp)
        }
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
    pub fn fully_defined_class(&self) -> ClassCode {
        (self.class, self.sub_class, self.protocol).into()
    }

    /// Should the interface be displayed expanded in a tree
    pub fn is_expanded(&self) -> bool {
        self.internal.expanded
    }

    /// Toggle the expanded state of the interface
    pub fn toggle_expanded(&mut self) {
        self.internal.expanded = !self.internal.expanded;
    }

    /// Set the expanded state of the interface and all its endpoints
    pub fn set_all_expanded(&mut self, expanded: bool) {
        self.internal.expanded = expanded;
        for endpoint in self.endpoints.iter_mut() {
            endpoint.set_expanded(expanded);
        }
    }
}

/// Devices can have multiple configurations, each with different attributes and interfaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    /// Name from string descriptor
    pub name: String,
    /// Index of name string in descriptor - only useful for lsusb verbose print
    #[serde(default)]
    pub string_index: u8,
    /// Number of config, bConfigurationValue; value to set to enable to configuration
    pub number: u8,
    /// Number of interfaces available for this configuruation
    #[serde(skip)]
    pub num_interfaces: u8,
    /// Interfaces available for this configuruation, including alt settings
    pub interfaces: Vec<Interface>,
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
    #[serde(skip)]
    pub(crate) internal: InternalData,
}

/// Deprecated alias for [`Configuration`]
#[deprecated(since = "2.0.0", note = "Use Configuration instead")]
pub type USBConfiguration = Configuration;

impl Configuration {
    /// Converts attributes into a ';' separated String
    pub fn attributes_string(&self) -> String {
        ConfigAttributes::attributes_to_string(&self.attributes)
    }

    /// Convert attributes back to reg value
    pub fn attributes_value(&self) -> u8 {
        let mut ret: u8 = 0x80; // always set reserved bit
        for attr in self.attributes.iter() {
            match attr {
                ConfigAttributes::SelfPowered => ret |= 0x40,
                ConfigAttributes::RemoteWakeup => ret |= 0x20,
                ConfigAttributes::BatteryPowered => ret |= 0x10,
                _ => (),
            }
        }

        ret
    }

    /// Should the configuration be displayed expanded in a tree
    pub fn is_expanded(&self) -> bool {
        self.internal.expanded
    }

    /// Toggle the expanded state of the configuration
    pub fn toggle_expanded(&mut self) {
        self.internal.expanded = !self.internal.expanded;
    }

    /// Set the expanded state of the configuration and all its interfaces
    pub fn set_all_expanded(&mut self, expanded: bool) {
        self.internal.expanded = expanded;
        for interface in self.interfaces.iter_mut() {
            interface.set_all_expanded(expanded);
        }
    }

    /// Gets the [`DevicePath`] for the configuration based on first [`Interface`]
    fn device_path(&self) -> Option<DevicePath> {
        self.interfaces.first().and_then(|i| i.device_path())
    }

    /// Gets the [`PortPath`] for the configuration based on first [`Interface`]
    pub fn parent_port_path(&self) -> Option<PortPath> {
        self.device_path().map(|p| p.port_path().to_owned())
    }

    /// Gets the [`ConfigurationPath`] for the configuration based on [`Self::parent_port_path`] and configuration number
    pub fn configuration_path(&self) -> Option<ConfigurationPath> {
        self.parent_port_path().map(|p| (p, self.number))
    }
}

/// Extra USB device data for verbose printing
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceExtra {
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
    /// Tuple of indexes to strings (iProduct, iManufacturer, iSerialNumber) - only useful for the lsusb verbose print
    #[serde(default)]
    pub string_indexes: (u8, u8, u8),
    /// USB devices can be have a number of configurations
    pub configurations: Vec<Configuration>,
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
    /// Speed that the device is operating at
    pub negotiated_speed: Option<Speed>,
}

/// Deprecated alias for [`DeviceExtra`]
#[deprecated(since = "2.0.0", note = "Use DeviceExtra instead")]
pub type USBDeviceExtra = DeviceExtra;

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
