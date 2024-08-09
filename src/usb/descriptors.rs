//! Defines for USB parsed device descriptors; extends the `usb` module.
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use super::*;
use crate::error::{self, Error, ErrorKind};

pub mod audio;
pub mod bos;
pub mod cdc;
pub mod video;

/// USB descritor types
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[repr(u8)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum DescriptorType {
    Device = 0x01,
    Config = 0x02,
    String = 0x03,
    Interface = 0x04,
    Endpoint = 0x05,
    DeviceQualifier = 0x06,
    OtherSpeedConfiguration = 0x07,
    InterfacePower = 0x08,
    Otg = 0x09,
    Debug = 0x0a,
    InterfaceAssociation = 0x0b,
    Security = 0x0c,
    Key = 0x0d,
    Encrypted = 0x0e,
    Bos = 0x0f,
    DeviceCapability = 0x10,
    WirelessEndpointCompanion = 0x11,
    WireAdaptor = 0x21,
    Report = 0x22,
    Physical = 0x23,
    Pipe = 0x24,
    Hub = 0x29,
    SuperSpeedHub = 0x2a,
    SsEndpointCompanion = 0x30,
    SsIsocEndpointCompanion = 0x31,
    Unknown(u8),
}

impl From<u8> for DescriptorType {
    fn from(b: u8) -> Self {
        match b {
            0x01 => DescriptorType::Device,
            0x02 => DescriptorType::Config,
            0x03 => DescriptorType::String,
            0x04 => DescriptorType::Interface,
            0x05 | 0x25 => DescriptorType::Endpoint,
            0x06 => DescriptorType::DeviceQualifier,
            0x07 => DescriptorType::OtherSpeedConfiguration,
            0x08 => DescriptorType::InterfacePower,
            0x09 => DescriptorType::Otg,
            0x0a => DescriptorType::Debug,
            0x0b => DescriptorType::InterfaceAssociation,
            0x0c => DescriptorType::Security,
            0x0d => DescriptorType::Key,
            0x0e => DescriptorType::Encrypted,
            0x0f => DescriptorType::Bos,
            0x10 => DescriptorType::DeviceCapability,
            0x11 => DescriptorType::WirelessEndpointCompanion,
            0x21 => DescriptorType::WireAdaptor,
            0x22 => DescriptorType::Report,
            0x23 => DescriptorType::Physical,
            0x24 => DescriptorType::Pipe,
            0x29 => DescriptorType::Hub,
            0x2a => DescriptorType::SuperSpeedHub,
            0x30 => DescriptorType::SsEndpointCompanion,
            0x31 => DescriptorType::SsIsocEndpointCompanion,
            _ => DescriptorType::Unknown(b),
        }
    }
}

impl From<DescriptorType> for u8 {
    fn from(dt: DescriptorType) -> Self {
        match dt {
            DescriptorType::Device => 0x01,
            DescriptorType::Config => 0x02,
            DescriptorType::String => 0x03,
            DescriptorType::Interface => 0x04,
            DescriptorType::Endpoint => 0x05,
            DescriptorType::DeviceQualifier => 0x06,
            DescriptorType::OtherSpeedConfiguration => 0x07,
            DescriptorType::InterfacePower => 0x08,
            DescriptorType::Otg => 0x09,
            DescriptorType::Debug => 0x0a,
            DescriptorType::InterfaceAssociation => 0x0b,
            DescriptorType::Security => 0x0c,
            DescriptorType::Key => 0x0d,
            DescriptorType::Encrypted => 0x0e,
            DescriptorType::Bos => 0x0f,
            DescriptorType::DeviceCapability => 0x10,
            DescriptorType::WirelessEndpointCompanion => 0x11,
            DescriptorType::WireAdaptor => 0x21,
            DescriptorType::Report => 0x22,
            DescriptorType::Physical => 0x23,
            DescriptorType::Pipe => 0x24,
            DescriptorType::Hub => 0x29,
            DescriptorType::SuperSpeedHub => 0x2a,
            DescriptorType::SsEndpointCompanion => 0x30,
            DescriptorType::SsIsocEndpointCompanion => 0x31,
            DescriptorType::Unknown(b) => b,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub usb_version: Version,
    pub device_class: u8,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_version: Version,
    pub manufacturer_string_index: u8,
    pub product_string_index: u8,
    pub serial_number_string_index: u8,
    pub num_configurations: u8,
}

impl TryFrom<&[u8]> for DeviceDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 18 {
            return Err(Error::new_descriptor_len(
                "DeviceDescriptor",
                18,
                value.len(),
            ));
        }

        Ok(DeviceDescriptor {
            length: value[0],
            descriptor_type: value[1],
            usb_version: Version::from_bcd(u16::from_le_bytes([value[2], value[3]])),
            device_class: value[4],
            device_sub_class: value[5],
            device_protocol: value[6],
            max_packet_size: value[7],
            vendor_id: u16::from_le_bytes([value[8], value[9]]),
            product_id: u16::from_le_bytes([value[10], value[11]]),
            device_version: Version::from_bcd(u16::from_le_bytes([value[12], value[13]])),
            manufacturer_string_index: value[14],
            product_string_index: value[15],
            serial_number_string_index: value[16],
            num_configurations: value[17],
        })
    }
}

impl From<DeviceDescriptor> for Vec<u8> {
    fn from(dd: DeviceDescriptor) -> Self {
        let mut ret = vec![dd.length, dd.descriptor_type];

        ret.extend(u16::from(dd.usb_version).to_le_bytes());
        ret.push(dd.device_class);
        ret.push(dd.device_sub_class);
        ret.push(dd.device_protocol);
        ret.push(dd.max_packet_size);
        ret.extend(dd.vendor_id.to_le_bytes());
        ret.extend(dd.product_id.to_le_bytes());
        ret.extend(u16::from(dd.device_version).to_le_bytes());
        ret.push(dd.manufacturer_string_index);
        ret.push(dd.product_string_index);
        ret.push(dd.serial_number_string_index);
        ret.push(dd.num_configurations);

        ret
    }
}

/// USB descriptor encloses type specific descriptor structs
///
/// Not all descriptors are implemented
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum Descriptor {
    Device(ClassDescriptor),
    Config(ClassDescriptor),
    String(String),
    Interface(ClassDescriptor),
    Endpoint(ClassDescriptor),
    DeviceQualifier(DeviceQualifierDescriptor),
    Otg(OnTheGoDescriptor),
    Debug(DebugDescriptor),
    InterfaceAssociation(InterfaceAssociationDescriptor),
    Security(SecurityDescriptor),
    Encrypted(EncryptionDescriptor),
    Bos(bos::BinaryObjectStoreDescriptor),
    Report(HidReportDescriptor),
    Hub(HubDescriptor),
    SuperSpeedHub(HubDescriptor),
    SsEndpointCompanion(SsEndpointCompanionDescriptor),
    // these are internal
    Unknown(Vec<u8>),
    Junk(Vec<u8>),
}

impl Descriptor {
    /// Returns the [`DescriptorType`] of the descriptor
    pub fn descriptor_type(&self) -> DescriptorType {
        match self {
            Descriptor::Device(_) => DescriptorType::Device,
            Descriptor::Config(_) => DescriptorType::Config,
            Descriptor::String(_) => DescriptorType::String,
            Descriptor::Interface(_) => DescriptorType::Interface,
            Descriptor::Endpoint(_) => DescriptorType::Endpoint,
            Descriptor::DeviceQualifier(_) => DescriptorType::DeviceQualifier,
            Descriptor::Otg(_) => DescriptorType::Otg,
            Descriptor::Debug(_) => DescriptorType::Debug,
            Descriptor::InterfaceAssociation(_) => DescriptorType::InterfaceAssociation,
            Descriptor::Security(_) => DescriptorType::Security,
            Descriptor::Encrypted(_) => DescriptorType::Encrypted,
            Descriptor::Bos(_) => DescriptorType::Bos,
            Descriptor::Report(_) => DescriptorType::Report,
            Descriptor::Hub(_) => DescriptorType::Hub,
            Descriptor::SuperSpeedHub(_) => DescriptorType::SuperSpeedHub,
            Descriptor::SsEndpointCompanion(_) => DescriptorType::SsEndpointCompanion,
            Descriptor::Unknown(d) => DescriptorType::Unknown(d.get(1).copied().unwrap_or(0)),
            Descriptor::Junk(d) => DescriptorType::Unknown(d.get(1).copied().unwrap_or(0)),
        }
    }
}

impl TryFrom<&[u8]> for Descriptor {
    type Error = Error;

    fn try_from(v: &[u8]) -> error::Result<Self> {
        if v.len() < 2 {
            return Err(Error::new_descriptor_len("Descriptor", 2, v.len()));
        }

        // junk length
        if v[0] < 2 {
            return Ok(Descriptor::Junk(v.to_vec()));
        }

        match v[1].into() {
            DescriptorType::Device => Ok(Descriptor::Device(ClassDescriptor::try_from(v)?)),
            DescriptorType::Config => Ok(Descriptor::Config(ClassDescriptor::try_from(v)?)),
            DescriptorType::String => {
                Ok(Descriptor::String(String::from_utf8_lossy(v).to_string()))
            }
            DescriptorType::Interface => Ok(Descriptor::Interface(ClassDescriptor::try_from(v)?)),
            DescriptorType::Endpoint => Ok(Descriptor::Endpoint(ClassDescriptor::try_from(v)?)),
            DescriptorType::DeviceQualifier => Ok(Descriptor::DeviceQualifier(
                DeviceQualifierDescriptor::try_from(v)?,
            )),
            DescriptorType::Otg => Ok(Descriptor::Otg(OnTheGoDescriptor::try_from(v)?)),
            DescriptorType::Debug => Ok(Descriptor::Debug(DebugDescriptor::try_from(v)?)),
            DescriptorType::InterfaceAssociation => Ok(Descriptor::InterfaceAssociation(
                InterfaceAssociationDescriptor::try_from(v)?,
            )),
            DescriptorType::Security => Ok(Descriptor::Security(SecurityDescriptor::try_from(v)?)),
            DescriptorType::Encrypted => {
                Ok(Descriptor::Encrypted(EncryptionDescriptor::try_from(v)?))
            }
            DescriptorType::Bos => Ok(Descriptor::Bos(bos::BinaryObjectStoreDescriptor::try_from(
                v,
            )?)),
            DescriptorType::Report => Ok(Descriptor::Report(HidReportDescriptor::try_from(v)?)),
            DescriptorType::Hub => Ok(Descriptor::Hub(HubDescriptor::try_from(v)?)),
            DescriptorType::SuperSpeedHub => {
                Ok(Descriptor::SuperSpeedHub(HubDescriptor::try_from(v)?))
            }
            DescriptorType::SsEndpointCompanion => Ok(Descriptor::SsEndpointCompanion(
                SsEndpointCompanionDescriptor::try_from(v)?,
            )),
            _ => Ok(Descriptor::Unknown(v.to_vec())),
        }
    }
}

impl From<Descriptor> for Vec<u8> {
    fn from(dt: Descriptor) -> Self {
        match dt {
            Descriptor::Device(d) => d.into(),
            Descriptor::Config(c) => c.into(),
            Descriptor::String(s) => s.into_bytes(),
            Descriptor::Interface(i) => i.into(),
            Descriptor::Endpoint(e) => e.into(),
            Descriptor::DeviceQualifier(dq) => dq.into(),
            Descriptor::Debug(d) => d.into(),
            Descriptor::InterfaceAssociation(ia) => ia.into(),
            Descriptor::Security(s) => s.into(),
            Descriptor::Encrypted(e) => e.into(),
            Descriptor::Bos(b) => b.into(),
            Descriptor::Report(r) => r.into(),
            Descriptor::Hub(h) => h.into(),
            Descriptor::Otg(o) => o.into(),
            Descriptor::SuperSpeedHub(h) => h.into(),
            Descriptor::SsEndpointCompanion(s) => s.into(),
            Descriptor::Unknown(u) => u,
            Descriptor::Junk(j) => j,
        }
    }
}

impl Descriptor {
    /// Uses [`ClassCodeTriplet`] to update the [`ClassDescriptor`] with [`ClassCode`] for class specific descriptors
    pub fn update_with_class_context<T: Into<ClassCode> + Copy>(
        &mut self,
        class_triplet: ClassCodeTriplet<T>,
    ) -> Result<(), Error> {
        match self {
            Descriptor::Device(d) => d.update_with_class_context(class_triplet),
            Descriptor::Config(c) => c.update_with_class_context(class_triplet),
            Descriptor::Interface(i) => i.update_with_class_context(class_triplet),
            Descriptor::Endpoint(e) => e.update_with_class_context(class_triplet),
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
            return Err(Error::new_descriptor_len(
                "InterfaceAssociationDescriptor",
                8,
                value.len(),
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

impl From<InterfaceAssociationDescriptor> for Vec<u8> {
    fn from(iad: InterfaceAssociationDescriptor) -> Self {
        vec![
            iad.length,
            iad.descriptor_type,
            iad.first_interface,
            iad.interface_count,
            iad.function_class,
            iad.function_sub_class,
            iad.function_protocol,
            iad.function_string_index,
        ]
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
        if value.len() < 4 {
            return Err(Error::new_descriptor_len(
                "SsEndpointCompanionDescriptor",
                4,
                value.len(),
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

impl From<SsEndpointCompanionDescriptor> for Vec<u8> {
    fn from(sec: SsEndpointCompanionDescriptor) -> Self {
        vec![
            sec.length,
            sec.descriptor_type,
            sec.max_burst,
            sec.attributes,
        ]
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
            return Err(Error::new_descriptor_len(
                "SecurityDescriptor",
                5,
                value.len(),
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

impl From<SecurityDescriptor> for Vec<u8> {
    fn from(sd: SecurityDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(sd.length);
        ret.push(sd.descriptor_type);
        ret.extend(sd.total_length.to_le_bytes());
        ret.push(sd.encryption_types);

        ret
    }
}

/// Encryption type for [`SecurityDescriptor`]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
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

impl From<EncryptionType> for u8 {
    fn from(et: EncryptionType) -> Self {
        match et {
            EncryptionType::Unsecure => 0x00,
            EncryptionType::Wired => 0x01,
            EncryptionType::Ccm1 => 0x02,
            EncryptionType::Rsa1 => 0x03,
            EncryptionType::Reserved => 0xff,
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
            return Err(Error::new_descriptor_len(
                "EncryptionDescriptor",
                5,
                value.len(),
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

impl From<EncryptionDescriptor> for Vec<u8> {
    fn from(ed: EncryptionDescriptor) -> Self {
        vec![
            ed.length,
            ed.descriptor_type,
            u8::from(ed.encryption_type),
            ed.encryption_value,
            ed.auth_key_index,
        ]
    }
}

/// USB base class descriptor
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClassDescriptor {
    /// USB HID extra descriptor
    Hid(HidDescriptor),
    /// USB Communication extra descriptor
    Communication(cdc::CommunicationDescriptor),
    /// USB CCID (Smart Card) extra descriptor
    Ccid(CcidDescriptor),
    /// USB Printer extra descriptor
    Printer(PrinterDescriptor),
    /// USB MIDI extra descriptor
    ///
    /// For legacy purposes, MIDI is defined as a SubClass (3) of Audio Class [1](https://www.usb.org/sites/default/files/USB%20MIDI%20v2_0.pdf) but we define at as a separate ClassDescriptor
    Midi(audio::MidiDescriptor, u8),
    /// USB Audio extra descriptor
    Audio(audio::UacDescriptor, audio::UacProtocol),
    /// USB Video extra descriptor
    Video(video::UvcDescriptor, u8),
    /// Device Firmware Upgrade (DFU) descriptor
    Dfu(DfuDescriptor),
    /// Generic descriptor with Option<ClassCode>
    ///
    /// Used for most descriptors and allows for TryFrom without knowing the [`ClassCode`]
    Generic(Option<ClassCodeTriplet<ClassCode>>, GenericDescriptor),
}

impl TryFrom<&[u8]> for ClassDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new_descriptor_len("ClassDescriptor", 3, value.len()));
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
            ClassDescriptor::Communication(cd) => cd.into(),
            ClassDescriptor::Midi(md, _) => md.into(),
            ClassDescriptor::Audio(ad, _) => ad.into(),
            ClassDescriptor::Video(vd, _) => vd.into(),
            ClassDescriptor::Dfu(dd) => dd.into(),
        }
    }
}

impl ClassDescriptor {
    /// Uses [`ClassCodeTriplet`] to update the [`ClassDescriptor`] with [`ClassCode`] and descriptor if it is not [`GenericDescriptor`]
    pub fn update_with_class_context<T: Into<ClassCode> + Copy>(
        &mut self,
        triplet: ClassCodeTriplet<T>,
    ) -> Result<(), Error> {
        if let ClassDescriptor::Generic(_, gd) = self {
            match (triplet.0.into(), triplet.1, triplet.2) {
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
                    *self = ClassDescriptor::Communication(cdc::CommunicationDescriptor::try_from(
                        gd.to_owned(),
                    )?)
                }
                // For legacy purposes, MIDI is defined as a SubClass of Audio Class
                // but we define at as a separate ClassDescriptor
                (ClassCode::Audio, 3, p) => {
                    *self =
                        ClassDescriptor::Midi(audio::MidiDescriptor::try_from(gd.to_owned())?, p)
                }
                // UAC
                (ClassCode::Audio, s, p) => {
                    *self = ClassDescriptor::Audio(
                        // endpoint is included in UacInterfaceDescriptor::try_from
                        audio::UacDescriptor::try_from((gd.to_owned(), s, p))?,
                        audio::UacProtocol::from(p),
                    )
                }
                (ClassCode::Video, s, p) => {
                    *self = ClassDescriptor::Video(
                        video::UvcDescriptor::try_from((gd.to_owned(), s, p))?,
                        p,
                    )
                }
                (ClassCode::ApplicationSpecificInterface, 1, _) => {
                    *self = ClassDescriptor::Dfu(DfuDescriptor::try_from(gd.to_owned())?)
                }
                ct => *self = ClassDescriptor::Generic(Some(ct), gd.to_owned()),
            }
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
            return Err(Error::new_descriptor_len(
                "HidReportDescriptor",
                3,
                value.len(),
            ));
        }

        if value[0] != 0x22 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "HID report descriptor must have descriptor type 0x22",
            ));
        }

        let length = u16::from_le_bytes([value[1], value[2]]);

        Ok(HidReportDescriptor {
            descriptor_type: value[0],
            length,
            data: value.get(3..3 + length as usize).map(|d| d.to_vec()),
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
            return Err(Error::new_descriptor_len(
                "GenericDescriptor",
                3,
                value.len(),
            ));
        }

        let length = value[0];
        if length as usize > value.len() {
            return Err(Error::new_descriptor_len(
                "GenericDescriptor reported",
                length as usize,
                value.len(),
            ));
        }

        Ok(GenericDescriptor {
            length,
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

    /// Returns true if the reported length of the data is 0
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
            return Err(Error::new_descriptor_len("HidDescriptor", 6, value.len()));
        }

        let num_descriptors = value[5] as usize;
        let mut descriptors_vec = value[6..].to_vec();
        let mut descriptors = Vec::<HidReportDescriptor>::with_capacity(num_descriptors);

        for _ in 0..num_descriptors {
            if descriptors_vec.len() < 3 {
                return Err(Error::new_descriptor_len(
                    "HidReportDescriptor",
                    3,
                    descriptors_vec.len(),
                ));
            }
            // Report data requires read of report from device so allow HidReportDescriptor creation but with no data
            descriptors.push(descriptors_vec.drain(..3).as_slice().try_into()?);
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
            return Err(Error::new_descriptor_len("CcidDescriptor", 54, value.len()));
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
        if value.len() < 5 {
            return Err(Error::new_descriptor_len(
                "PrinterDescriptor",
                5,
                value.len(),
            ));
        }

        let num_descriptors = value[3] as usize;
        let mut descriptors_vec = value[4..].to_vec();
        let mut descriptors = Vec::<PrinterReportDescriptor>::with_capacity(num_descriptors);

        for _ in 0..num_descriptors {
            if descriptors_vec.len() < 2 {
                return Err(Error::new_descriptor_len(
                    "PrinterReportDescriptor",
                    2,
                    descriptors_vec.len(),
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
    pub length: u8,
    pub descriptor_type: u8,
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
            return Err(Error::new_descriptor_len(
                "PrinterReportDescriptor",
                6,
                value.len(),
            ));
        }

        Ok(PrinterReportDescriptor {
            length: value[0],
            descriptor_type: value[1],
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct HubDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub num_ports: u8,
    pub characteristics: u16,
    pub power_on_to_power_good: u8,
    pub control_current: u8,
    pub data: Vec<u8>,
    pub port_statuses: Option<Vec<[u8; 8]>>,
}

impl TryFrom<&[u8]> for HubDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new_descriptor_len("HubDescriptor", 9, value.len()));
        }

        Ok(HubDescriptor {
            length: value[0],
            descriptor_type: value[1],
            num_ports: value[2],
            characteristics: u16::from_le_bytes([value[3], value[4]]),
            power_on_to_power_good: value[5],
            control_current: value[6],
            data: value[7..].to_vec(),
            port_statuses: None,
        })
    }
}

impl From<HubDescriptor> for Vec<u8> {
    fn from(hd: HubDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(hd.length);
        ret.push(hd.descriptor_type);
        ret.push(hd.num_ports);
        ret.extend(hd.characteristics.to_le_bytes());
        ret.push(hd.power_on_to_power_good);
        ret.push(hd.control_current);
        ret.extend(hd.data);

        ret
    }
}

impl HubDescriptor {
    /// Type 3 devices have a delay field, which is a combination of latency in nano seconds
    pub fn delay(&self) -> Option<u16> {
        match (self.latency(), self.data.get(1)) {
            (Some(l), Some(d)) => Some((*d as u16) << (4 + l)),
            _ => None,
        }
    }

    /// Type 3 devices have a latency field
    pub fn latency(&self) -> Option<u8> {
        self.data.first().copied()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DfuDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub attributes: u8,
    pub detach_timeout: u16,
    pub transfer_size: u16,
    // not all have version
    pub dfu_version: Option<Version>,
}

impl TryFrom<&[u8]> for DfuDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new_descriptor_len("DfuDescriptor", 7, value.len()));
        }

        let dfu_version = if value.len() >= 9 {
            Some(Version::from_bcd(u16::from_le_bytes([value[7], value[8]])))
        } else {
            None
        };

        Ok(DfuDescriptor {
            length: value[0],
            descriptor_type: value[1],
            attributes: value[2],
            detach_timeout: u16::from_le_bytes([value[3], value[4]]),
            transfer_size: u16::from_le_bytes([value[5], value[6]]),
            dfu_version,
        })
    }
}

impl From<DfuDescriptor> for Vec<u8> {
    fn from(dd: DfuDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(dd.length);
        ret.push(dd.descriptor_type);
        ret.push(dd.attributes);
        ret.extend(dd.detach_timeout.to_le_bytes());
        ret.extend(dd.transfer_size.to_le_bytes());
        if let Some(v) = dd.dfu_version {
            ret.extend(u16::from(v).to_le_bytes());
        }

        ret
    }
}

impl TryFrom<GenericDescriptor> for DfuDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        DfuDescriptor::try_from(&gd_vec[..])
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DebugDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub debug_in_endpoint: u8,
    pub debug_out_endpoint: u8,
}

impl TryFrom<&[u8]> for DebugDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("DebugDescriptor", 4, value.len()));
        }

        if value[1] != 0x0a {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Debug descriptor must have descriptor type 0x0a",
            ));
        }

        Ok(DebugDescriptor {
            length: value[0],
            descriptor_type: value[1],
            debug_in_endpoint: value[2],
            debug_out_endpoint: value[3],
        })
    }
}

impl From<DebugDescriptor> for Vec<u8> {
    fn from(dd: DebugDescriptor) -> Self {
        vec![
            dd.length,
            dd.descriptor_type,
            dd.debug_in_endpoint,
            dd.debug_out_endpoint,
        ]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DeviceQualifierDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub version: Version,
    pub device_class: ClassCode,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub max_packet_size: u8,
    pub num_configurations: u8,
}

impl TryFrom<&[u8]> for DeviceQualifierDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new_descriptor_len(
                "DeviceQualifierDescriptor",
                9,
                value.len(),
            ));
        }

        if value[1] != 0x06 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Device Qualifier descriptor must have descriptor type 0x06",
            ));
        }

        Ok(DeviceQualifierDescriptor {
            length: value[0],
            descriptor_type: value[1],
            version: Version::from_bcd(u16::from_le_bytes([value[2], value[3]])),
            device_class: ClassCode::from(value[4]),
            device_subclass: value[5],
            device_protocol: value[6],
            max_packet_size: value[7],
            num_configurations: value[8],
        })
    }
}

impl From<DeviceQualifierDescriptor> for Vec<u8> {
    fn from(dqd: DeviceQualifierDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(dqd.length);
        ret.push(dqd.descriptor_type);
        ret.extend(u16::from(dqd.version).to_le_bytes());
        ret.push(dqd.device_class.into());
        ret.push(dqd.device_subclass);
        ret.push(dqd.device_protocol);
        ret.push(dqd.max_packet_size);
        ret.push(dqd.num_configurations);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct OnTheGoDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub attributes: u8,
}

impl TryFrom<&[u8]> for OnTheGoDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() != 3 {
            return Err(Error::new_descriptor_len(
                "OnTheGoDescriptor",
                3,
                value.len(),
            ));
        }

        if value[1] != 0x09 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "On-The-Go descriptor must have descriptor type 0x09",
            ));
        }

        Ok(OnTheGoDescriptor {
            length: value[0],
            descriptor_type: value[1],
            attributes: value[2],
        })
    }
}

impl From<OnTheGoDescriptor> for Vec<u8> {
    fn from(otg: OnTheGoDescriptor) -> Self {
        vec![otg.length, otg.descriptor_type, otg.attributes]
    }
}
