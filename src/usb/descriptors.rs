//! Defines for USB parsed device descriptors; extends the `usb` module.
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use super::*;
use crate::error::{self, Error, ErrorKind};

pub mod audio;
pub mod bos;
pub mod video;

/// Get the GUID String from a descriptor buffer slice
pub fn get_guid(buf: &[u8]) -> Result<String, Error> {
    if buf.len() < 16 {
        return Err(Error::new(
            ErrorKind::InvalidArg,
            "GUID buffer too short, must be at least 16 bytes",
        ));
    }

    Ok(format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", 
        buf[3], buf[2], buf[1], buf[0],
        buf[5], buf[4],
        buf[7], buf[6],
        buf[8], buf[9],
        buf[10], buf[11], buf[12], buf[13], buf[14], buf[15]))
}

/// Convert a GUID string back to a byte array
pub fn guid_to_bytes(guid: &str) -> Result<[u8; 16], Error> {
    let guid = guid.replace('-', "");

    if guid.len() != 32 {
        return Err(Error::new(
            ErrorKind::InvalidArg,
            "GUID string must be 32 characters long",
        ));
    }

    let bytes = (0..16)
        .map(|i| u8::from_str_radix(&guid[i * 2..i * 2 + 2], 16).unwrap_or(0))
        .collect::<Vec<u8>>();

    let mut array = [0; 16];
    array.copy_from_slice(&bytes);
    Ok(array)
}

/// USB Descriptor Types
///
/// Can enclose struct of descriptor data
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[repr(u8)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum DescriptorType {
    Device(ClassDescriptor) = 0x01,
    Config(ClassDescriptor) = 0x02,
    String(String) = 0x03,
    Interface(ClassDescriptor) = 0x04,
    Endpoint(ClassDescriptor) = 0x05,
    DeviceQualifier(DeviceQualifierDescriptor) = 0x06,
    OtherSpeedConfiguration = 0x07,
    InterfacePower = 0x08,
    // TODO do_otg
    Otg = 0x09,
    Debug(DebugDescriptor) = 0x0a,
    InterfaceAssociation(InterfaceAssociationDescriptor) = 0x0b,
    Security(SecurityDescriptor) = 0x0c,
    Key = 0x0d,
    Encrypted(EncryptionDescriptor) = 0x0e,
    Bos(bos::BinaryObjectStoreDescriptor) = 0x0f,
    DeviceCapability = 0x10,
    WirelessEndpointCompanion = 0x11,
    WireAdaptor = 0x21,
    Report(HidReportDescriptor) = 0x22,
    Physical = 0x23,
    Pipe = 0x24,
    Hub(HubDescriptor) = 0x29,
    SuperSpeedHub(HubDescriptor) = 0x2a,
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
            0x06 => Ok(DescriptorType::DeviceQualifier(
                DeviceQualifierDescriptor::try_from(v)?,
            )),
            0x07 => Ok(DescriptorType::OtherSpeedConfiguration),
            0x08 => Ok(DescriptorType::InterfacePower),
            0x09 => Ok(DescriptorType::Otg),
            0x0a => Ok(DescriptorType::Debug(DebugDescriptor::try_from(v)?)),
            0x0b => Ok(DescriptorType::InterfaceAssociation(
                InterfaceAssociationDescriptor::try_from(v)?,
            )),
            0x0c => Ok(DescriptorType::Security(SecurityDescriptor::try_from(v)?)),
            0x0d => Ok(DescriptorType::Key),
            0x0e => Ok(DescriptorType::Encrypted(EncryptionDescriptor::try_from(
                v,
            )?)),
            0x0f => Ok(DescriptorType::Bos(
                bos::BinaryObjectStoreDescriptor::try_from(v)?,
            )),
            0x10 => Ok(DescriptorType::DeviceCapability),
            0x11 => Ok(DescriptorType::WirelessEndpointCompanion),
            0x21 => Ok(DescriptorType::WireAdaptor),
            0x22 => Ok(DescriptorType::Report(HidReportDescriptor::try_from(v)?)),
            0x23 => Ok(DescriptorType::Physical),
            0x24 => Ok(DescriptorType::Pipe),
            0x29 => Ok(DescriptorType::Hub(HubDescriptor::try_from(v)?)),
            0x2a => Ok(DescriptorType::SuperSpeedHub(HubDescriptor::try_from(v)?)),
            0x30 => Ok(DescriptorType::SsEndpointCompanion(
                SsEndpointCompanionDescriptor::try_from(v)?,
            )),
            0x31 => Ok(DescriptorType::SsIsocEndpointCompanion),
            _ => Ok(DescriptorType::Unknown(v.to_vec())),
        }
    }
}

impl From<DescriptorType> for Vec<u8> {
    fn from(dt: DescriptorType) -> Self {
        match dt {
            DescriptorType::Device(d) => d.into(),
            DescriptorType::Config(c) => c.into(),
            DescriptorType::String(s) => s.into_bytes(),
            DescriptorType::Interface(i) => i.into(),
            DescriptorType::Endpoint(e) => e.into(),
            DescriptorType::DeviceQualifier(dq) => dq.into(),
            DescriptorType::OtherSpeedConfiguration => vec![],
            DescriptorType::InterfacePower => vec![],
            DescriptorType::Otg => vec![],
            DescriptorType::Debug(d) => d.into(),
            DescriptorType::InterfaceAssociation(ia) => ia.into(),
            DescriptorType::Security(s) => s.into(),
            DescriptorType::Key => vec![],
            DescriptorType::Encrypted(e) => e.into(),
            DescriptorType::Bos(b) => b.into(),
            DescriptorType::DeviceCapability => vec![],
            DescriptorType::WirelessEndpointCompanion => vec![],
            DescriptorType::WireAdaptor => vec![],
            DescriptorType::Report(r) => r.into(),
            DescriptorType::Physical => vec![],
            DescriptorType::Pipe => vec![],
            DescriptorType::Hub(h) => h.into(),
            DescriptorType::SuperSpeedHub(h) => h.into(),
            DescriptorType::SsEndpointCompanion(s) => s.into(),
            DescriptorType::SsIsocEndpointCompanion => vec![],
            DescriptorType::Unknown(u) => u,
            DescriptorType::Junk(j) => j,
        }
    }
}

//impl From<DescriptorType> for u8 {
//    fn from(dt: DescriptorType) -> Self {
//        match dt {
//            DescriptorType::Device(_) => 0x01,
//            DescriptorType::Config(_) => 0x02,
//            DescriptorType::String(_) => 0x03,
//            DescriptorType::Interface(_) => 0x04,
//            DescriptorType::Endpoint(_) => 0x05,
//            DescriptorType::DeviceQualifier(_) => 0x06,
//            DescriptorType::OtherSpeedConfiguration => 0x07,
//            DescriptorType::InterfacePower => 0x08,
//            DescriptorType::Otg => 0x09,
//            DescriptorType::Debug(_) => 0x0a,
//            DescriptorType::InterfaceAssociation(_) => 0x0b,
//            DescriptorType::Security(_) => 0x0c,
//            DescriptorType::Key => 0x0d,
//            DescriptorType::Encrypted(_) => 0x0e,
//            DescriptorType::Bos(_) => 0x0f,
//            DescriptorType::DeviceCapability => 0x10,
//            DescriptorType::WirelessEndpointCompanion => 0x11,
//            DescriptorType::WireAdaptor => 0x21,
//            DescriptorType::Report(_) => 0x22,
//            DescriptorType::Physical => 0x23,
//            DescriptorType::Pipe => 0x24,
//            DescriptorType::Hub(_) => 0x29,
//            DescriptorType::SuperSpeedHub(_) => 0x2a,
//            DescriptorType::SsEndpointCompanion(_) => 0x30,
//            DescriptorType::SsIsocEndpointCompanion => 0x31,
//            DescriptorType::Unknown(_) => 0xfe,
//            DescriptorType::Junk(_) => 0xff,
//        }
//    }
//}

impl DescriptorType {
    /// Uses [`ClassCodeTriplet`] to update the [`ClassDescriptor`] with [`ClassCode`] for class specific descriptors
    pub fn update_with_class_context<T: Into<ClassCode> + Copy>(
        &mut self,
        class_triplet: ClassCodeTriplet<T>,
    ) -> Result<(), Error> {
        let dt = self.clone();
        match self {
            DescriptorType::Device(d) => d.update_with_class_context(&dt, class_triplet),
            DescriptorType::Config(c) => c.update_with_class_context(&dt, class_triplet),
            DescriptorType::Interface(i) => i.update_with_class_context(&dt, class_triplet),
            DescriptorType::Endpoint(e) => e.update_with_class_context(&dt, class_triplet),
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
    Communication(CommunicationDescriptor),
    /// USB CCID (Smart Card) extra descriptor
    Ccid(CcidDescriptor),
    /// USB Printer extra descriptor
    Printer(PrinterDescriptor),
    /// USB MIDI extra descriptor (AudioVideoAVDataAudio)
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
        descriptor_type: &DescriptorType,
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
                    *self = ClassDescriptor::Communication(CommunicationDescriptor::try_from(
                        gd.to_owned(),
                    )?)
                }
                // MIDI - TODO include in UAC
                (ClassCode::Audio, 3, p) => {
                    // leave generic for Midi Endpoint - TODO should add MidiEndpointDescriptor
                    if !matches!(descriptor_type, DescriptorType::Endpoint(_)) {
                        *self = ClassDescriptor::Midi(
                            audio::MidiDescriptor::try_from(gd.to_owned())?,
                            p,
                        )
                    }
                }
                // UAC
                (ClassCode::Audio, s, p) => {
                    *self = ClassDescriptor::Audio(
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Generic descriptor too short",
            ));
        }

        let length = value[0];
        if length as usize > value.len() {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Generic descriptor reported length too long for data returned",
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "HID descriptor too short",
            ));
        }

        let num_descriptors = value[5] as usize;
        let mut descriptors_vec = value[6..].to_vec();
        let mut descriptors = Vec::<HidReportDescriptor>::with_capacity(num_descriptors);

        for _ in 0..num_descriptors {
            if descriptors_vec.len() < 3 {
                return Err(Error::new(
                    ErrorKind::InvalidArg,
                    "HID report descriptor too short",
                ));
            }

            // Report data requires read of report from device so allow HidReportDescriptor creation but with no data
            //let len = u16::from_le_bytes([descriptors_vec[1], descriptors_vec[2]]) as usize;
            //if len > descriptors_vec.len() {
            //    return Err(Error::new(
            //        ErrorKind::InvalidArg,
            //        &format!("HID report descriptor reported length too long for available data! Expected {} but only have {}", len, descriptors_vec.len()),
            //    ));
            //}

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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Printer report descriptor too short",
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

/// USB Communication Device Class (CDC) types
///
/// Used to differentiate between different CDC descriptors
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
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

impl std::fmt::Display for CdcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // lsusb style
        if f.alternate() {
            match self {
                CdcType::Header => write!(f, "Header"),
                CdcType::CallManagement => write!(f, "Call Management"),
                CdcType::AbstractControlManagement => write!(f, "ACM"),
                CdcType::DirectLineManagement => write!(f, "DLM"),
                CdcType::TelephoneRinger => write!(f, "Telephone Ringer"),
                CdcType::TelephoneCall => write!(f, "Telephone Call"),
                CdcType::Union => write!(f, "Union"),
                CdcType::CountrySelection => write!(f, "Country Selection"),
                CdcType::TelephoneOperationalModes => write!(f, "Telephone Operations"),
                CdcType::UsbTerminal => write!(f, "USB Terminal"),
                CdcType::NetworkChannel => write!(f, "Network Channel Terminal"),
                CdcType::ProtocolUnit => write!(f, "Protocol Unit"),
                CdcType::ExtensionUnit => write!(f, "Extension Unit"),
                CdcType::MultiChannel => write!(f, "Multi Channel"),
                CdcType::CapiControl => write!(f, "CAPI Control"),
                CdcType::EthernetNetworking => write!(f, "Ethernet"),
                CdcType::AtmNetworking => write!(f, "ATM Networking"),
                CdcType::WirelessHandsetControlModel => write!(f, "WHCM version"),
                CdcType::MobileDirectLineModelFunctional => {
                    write!(f, "MDLM")
                }
                CdcType::MobileDirectLineModelDetail => write!(f, "MDLM detail"),
                CdcType::DeviceManagement => write!(f, "Device Management"),
                CdcType::Obex => write!(f, "OBEX"),
                CdcType::CommandSet => write!(f, "Command Set"),
                CdcType::CommandSetDetail => write!(f, "Command Set Detail"),
                CdcType::TelephoneControlModel => write!(f, "Telephone Control Model"),
                CdcType::ObexCommandSet => write!(f, "OBEX Command Set"),
                CdcType::Ncm => write!(f, "NCM"),
                CdcType::Mbim => write!(f, "MBIM"),
                CdcType::MbimExtended => write!(f, "MBIM Extended"),
                CdcType::Unknown => write!(f, ""),
            }
        } else {
            write!(f, "{:?}", self)
        }
    }
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
            CdcType::EthernetNetworking | CdcType::CountrySelection => {
                value.get(3).map(|v| v.to_owned())
            }
            CdcType::NetworkChannel => value.get(4).map(|v| v.to_owned()),
            CdcType::CommandSet => value.get(5).map(|v| v.to_owned()),
            _ => None,
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
#[allow(missing_docs)]
pub struct HubDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub num_ports: u8,
    pub characteristics: u16,
    pub power_on_to_power_good: u8,
    pub control_current: u8,
    pub latancy: u8,
    pub delay: u8,
}

impl TryFrom<&[u8]> for HubDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Hub descriptor too short",
            ));
        }

        Ok(HubDescriptor {
            length: value[0],
            descriptor_type: value[1],
            num_ports: value[2],
            characteristics: u16::from_le_bytes([value[3], value[4]]),
            power_on_to_power_good: value[5],
            control_current: value[6],
            latancy: value[7],
            delay: value[8],
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
        ret.push(hd.latancy);
        ret.push(hd.delay);

        ret
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "DFU descriptor too short",
            ));
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Debug descriptor too short",
            ));
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
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Device Qualifier descriptor too short",
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