//! Defines for USB parsed device descriptors; extends the `usb` module.
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

use crate::error::{self, Error, ErrorKind};
use crate::usb::*;

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
    Midi(MidiDescriptor, u8),
    /// USB Video extra descriptor
    Video(UvcDescriptor, u8),
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
            ClassDescriptor::Video(vd, _) => vd.into(),
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
                    *self = ClassDescriptor::Communication(CommunicationDescriptor::try_from(
                        gd.to_owned(),
                    )?)
                }
                (ClassCode::Audio, 3, p) => {
                    *self = ClassDescriptor::Midi(MidiDescriptor::try_from(gd.to_owned())?, p)
                }
                (ClassCode::Video, 1, p) => {
                    *self = ClassDescriptor::Video(UvcDescriptor::try_from(gd.to_owned())?, p)
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
            if descriptors_vec.len() < 2 {
                return Err(Error::new(
                    ErrorKind::InvalidArg,
                    "HID report descriptor too short",
                ));
            }

            let len = u16::from_le_bytes([descriptors_vec[1], descriptors_vec[2]]) as usize;

            if len > descriptors_vec.len() {
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
#[repr(u8)]
#[non_exhaustive]
pub enum UvcInterface {
    Undefined = 0x00,
    Header = 0x01,
    InputTerminal = 0x02,
    OutputTerminal = 0x03,
    SelectorUnit = 0x04,
    ProcessingUnit = 0x05,
    ExtensionUnit = 0x06,
    EncodingUnit = 0x07,
}

impl From<u8> for UvcInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UvcInterface::Undefined,
            0x01 => UvcInterface::Header,
            0x02 => UvcInterface::InputTerminal,
            0x03 => UvcInterface::OutputTerminal,
            0x04 => UvcInterface::SelectorUnit,
            0x05 => UvcInterface::ProcessingUnit,
            0x06 => UvcInterface::ExtensionUnit,
            0x07 => UvcInterface::EncodingUnit,
            _ => UvcInterface::Undefined,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UvcDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_subtype: u8,
    pub string_index: Option<u8>,
    pub string: Option<String>,
    pub data: Vec<u8>,
}

impl TryFrom<&[u8]> for UvcDescriptor {
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

        let video_control_subtype = UvcInterface::from(value[2]);

        let string_index = match video_control_subtype {
            UvcInterface::InputTerminal => value.get(7).copied(),
            UvcInterface::OutputTerminal => value.get(8).copied(),
            UvcInterface::SelectorUnit => {
                if let Some(p) = value.get(4) {
                    value.get(5 + *p as usize).copied()
                } else {
                    None
                }
            }
            UvcInterface::ProcessingUnit => {
                if let Some(n) = value.get(7) {
                    value.get(8 + *n as usize).copied()
                } else {
                    None
                }
            }
            UvcInterface::ExtensionUnit => {
                if let Some(p) = value.get(21) {
                    if let Some(n) = value.get(22 + *p as usize) {
                        value.get(23 + *n as usize + *p as usize).copied()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            UvcInterface::EncodingUnit => value.get(5).copied(),
            _ => None,
        };

        Ok(UvcDescriptor {
            length,
            descriptor_type: value[1],
            descriptor_subtype: value[2],
            string_index,
            string: None,
            data: value[3..].to_vec(),
        })
    }
}

impl From<UvcDescriptor> for Vec<u8> {
    fn from(vcd: UvcDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(vcd.length);
        ret.push(vcd.descriptor_type);
        ret.push(vcd.descriptor_subtype);
        ret.extend(vcd.data);

        ret
    }
}

impl TryFrom<GenericDescriptor> for UvcDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        UvcDescriptor::try_from(&gd_vec[..])
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
    pub midi_type: MidiInterface,
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

        let midi_type = MidiInterface::from(value[2]);

        let string_index = match midi_type {
            MidiInterface::InputJack => value.get(5).copied(),
            MidiInterface::OutputJack => value.get(5).map(|v| 6 + *v * 2),
            MidiInterface::Element => {
                // don't ask...
                if let Some(j) = value.get(4) {
                    if let Some(capsize) = value.get((5 + *j as usize * 2) + 3) {
                        value.get(9 + 2 * *j as usize + *capsize as usize).copied()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        Ok(MidiDescriptor {
            length,
            descriptor_type: value[1],
            midi_type,
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
        ret.push(md.midi_type as u8);
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

/// USB Audio Class (UAC) interface Audio Control (AC) types based on bDescriptorSubtype
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum UacAcInterface {
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

impl std::fmt::Display for UacAcInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            // uppercase with _ instead of space for lsusb dump
            match self {
                UacAcInterface::Undefined => write!(f, "unknown"),
                UacAcInterface::Header => write!(f, "HEADER"),
                UacAcInterface::InputTerminal => write!(f, "INPUT_TERMINAL"),
                UacAcInterface::OutputTerminal => write!(f, "OUTPUT_TERMINAL"),
                UacAcInterface::ExtendedTerminal => write!(f, "EXTENDED_TERMINAL"),
                UacAcInterface::MixerUnit => write!(f, "MIXER_UNIT"),
                UacAcInterface::SelectorUnit => write!(f, "SELECTOR_UNIT"),
                UacAcInterface::FeatureUnit => write!(f, "FEATURE_UNIT"),
                UacAcInterface::EffectUnit => write!(f, "EFFECT_UNIT"),
                UacAcInterface::ProcessingUnit => write!(f, "PROCESSING_UNIT"),
                UacAcInterface::ExtensionUnit => write!(f, "EXTENSION_UNIT"),
                UacAcInterface::ClockSource => write!(f, "CLOCK_SOURCE"),
                UacAcInterface::ClockSelector => write!(f, "CLOCK_SELECTOR"),
                UacAcInterface::ClockMultiplier => write!(f, "CLOCK_MULTIPLIER"),
                UacAcInterface::SampleRateConverter => write!(f, "SAMPLE_RATE_CONVERTER"),
                UacAcInterface::Connectors => write!(f, "CONNECTORS"),
                UacAcInterface::PowerDomain => write!(f, "POWER_DOMAIN"),
            }
        } else {
            match self {
                UacAcInterface::Undefined => write!(f, "Undefined"),
                UacAcInterface::Header => write!(f, "Header"),
                UacAcInterface::InputTerminal => write!(f, "Input Terminal"),
                UacAcInterface::OutputTerminal => write!(f, "Output Terminal"),
                UacAcInterface::ExtendedTerminal => write!(f, "Extended Terminal"),
                UacAcInterface::MixerUnit => write!(f, "Mixer Unit"),
                UacAcInterface::SelectorUnit => write!(f, "Selector Unit"),
                UacAcInterface::FeatureUnit => write!(f, "Feature Unit"),
                UacAcInterface::EffectUnit => write!(f, "Effect Unit"),
                UacAcInterface::ProcessingUnit => write!(f, "Processing Unit"),
                UacAcInterface::ExtensionUnit => write!(f, "Extension Unit"),
                UacAcInterface::ClockSource => write!(f, "Clock Source"),
                UacAcInterface::ClockSelector => write!(f, "Clock Selector"),
                UacAcInterface::ClockMultiplier => write!(f, "Clock Multiplier"),
                UacAcInterface::SampleRateConverter => write!(f, "Sample Rate Converter"),
                UacAcInterface::Connectors => write!(f, "Connectors"),
                UacAcInterface::PowerDomain => write!(f, "Power Domain"),
            }
        }
    }
}

impl From<u8> for UacAcInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UacAcInterface::Undefined,
            0x01 => UacAcInterface::Header,
            0x02 => UacAcInterface::InputTerminal,
            0x03 => UacAcInterface::OutputTerminal,
            0x04 => UacAcInterface::ExtendedTerminal,
            0x05 => UacAcInterface::MixerUnit,
            0x06 => UacAcInterface::SelectorUnit,
            0x07 => UacAcInterface::FeatureUnit,
            0x08 => UacAcInterface::EffectUnit,
            0x09 => UacAcInterface::ProcessingUnit,
            0x0a => UacAcInterface::ExtensionUnit,
            0x0b => UacAcInterface::ClockSource,
            0x0c => UacAcInterface::ClockSelector,
            0x0d => UacAcInterface::ClockMultiplier,
            0x0e => UacAcInterface::SampleRateConverter,
            0x0f => UacAcInterface::Connectors,
            0x10 => UacAcInterface::PowerDomain,
            _ => UacAcInterface::Undefined,
        }
    }
}

impl UacAcInterface {
    /// UAC1, UAC2, and UAC3 define bDescriptorSubtype differently for the
    /// AudioControl interface, so we need to do some ugly remapping:
    pub fn get_uac_subtype(subtype: u8, protocol: u8) -> Self {
        match protocol {
            // UAC1
            0x00 => match subtype {
                0x04 => UacAcInterface::MixerUnit,
                0x05 => UacAcInterface::SelectorUnit,
                0x06 => UacAcInterface::FeatureUnit,
                0x07 => UacAcInterface::ProcessingUnit,
                0x08 => UacAcInterface::ExtensionUnit,
                _ => Self::from(subtype),
            },
            // UAC2
            0x20 => match subtype {
                0x04 => UacAcInterface::MixerUnit,
                0x05 => UacAcInterface::SelectorUnit,
                0x06 => UacAcInterface::FeatureUnit,
                0x07 => UacAcInterface::EffectUnit,
                0x08 => UacAcInterface::ProcessingUnit,
                0x09 => UacAcInterface::ExtensionUnit,
                0x0a => UacAcInterface::ClockSource,
                0x0b => UacAcInterface::ClockSelector,
                0x0c => UacAcInterface::ClockMultiplier,
                0x0d => UacAcInterface::SampleRateConverter,
                _ => Self::from(subtype),
            },
            // no re-map for UAC3..
            _ => Self::from(subtype),
        }
    }

    /// Get the UAC interface descriptor from the UAC interface
    pub fn get_descriptor(
        &self,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<UacInterfaceDescriptor, Error> {
        UacInterfaceDescriptor::from_uac_ac_interface(self, protocol, data)
    }
}

/// USB Audio Class (UAC) interface Audio Streaming (AS) types based on bDescriptorSubtype
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum UacAsInterface {
    Undefined = 0x00,
    General = 0x01,
    FormatType = 0x02,
    FormatSpecific = 0x03,
}

impl From<u8> for UacAsInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UacAsInterface::Undefined,
            0x01 => UacAsInterface::General,
            0x02 => UacAsInterface::FormatType,
            0x03 => UacAsInterface::FormatSpecific,
            _ => UacAsInterface::Undefined,
        }
    }
}

impl fmt::Display for UacAsInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // uppercase with _ instead of space for lsusb dump
            match self {
                UacAsInterface::Undefined => write!(f, "UNDEFINED"),
                UacAsInterface::General => write!(f, "GENERAL"),
                UacAsInterface::FormatType => write!(f, "FORMAT_TYPE"),
                UacAsInterface::FormatSpecific => write!(f, "FORMAT_SPECIFIC"),
            }
        } else {
            match self {
                UacAsInterface::Undefined => write!(f, "Undefined"),
                UacAsInterface::General => write!(f, "General"),
                UacAsInterface::FormatType => write!(f, "Format Type"),
                UacAsInterface::FormatSpecific => write!(f, "Format Specific"),
            }
        }
    }
}

impl UacAsInterface {
    /// Get the UAC interface descriptor from the UAC interface
    pub fn get_descriptor(
        &self,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<UacInterfaceDescriptor, Error> {
        UacInterfaceDescriptor::from_uac_as_interface(self, protocol, data)
    }
}

/// USB Audio Class (UAC) interface descriptors
///
/// Ported from https://github.com/gregkh/usbutils/blob/master/desc-defs.c
///
/// I think there is a much nicer way to define all these for more generic printing; enum types like desc-def.c wrapping the int values so they can be acted on in a more generic way
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum UacInterfaceDescriptor {
    // Audio Controls bSubClass
    AudioHeader1(AudioHeader1),
    AudioHeader2(AudioHeader2),
    AudioHeader3(AudioHeader3),
    AudioInputTerminal1(AudioInputTerminal1),
    AudioInputTerminal2(AudioInputTerminal2),
    AudioInputTerminal3(AudioInputTerminal3),
    AudioOutputTerminal1(AudioOutputTerminal1),
    AudioOutputTerminal2(AudioOutputTerminal2),
    AudioOutputTerminal3(AudioOutputTerminal3),
    ExtendedTerminalHeader(ExtendedTerminalHeader),
    AudioPowerDomain(AudioPowerDomain),
    AudioMixerUnit1(AudioMixerUnit1),
    AudioMixerUnit2(AudioMixerUnit2),
    AudioMixerUnit3(AudioMixerUnit3),
    AudioSelectorUnit1(AudioSelectorUnit1),
    AudioSelectorUnit2(AudioSelectorUnit2),
    AudioSelectorUnit3(AudioSelectorUnit3),
    // TODO EffectUnit
    // TODO ProcessingUnit
    // TODO FeatureUnit
    AudioExtensionUnit1(AudioExtensionUnit1),
    AudioExtensionUnit2(AudioExtensionUnit2),
    AudioExtensionUnit3(AudioExtensionUnit3),
    AudioClockSource2(AudioClockSource2),
    AudioClockSource3(AudioClockSource3),
    AudioClockSelector2(AudioClockSelector2),
    AudioClockSelector3(AudioClockSelector3),
    AudioClockMultiplier2(AudioClockMultiplier2),
    AudioClockMultiplier3(AudioClockMultiplier3),
    AudioSampleRateConverter2(AudioSampleRateConverter2),
    AudioSampleRateConverter3(AudioSampleRateConverter3),
    // Audio Streaming bSubClass
    AudioStreamingInterface1(AudioStreamingInterface1),
    AudioStreamingInterface2(AudioStreamingInterface2),
    AudioStreamingInterface3(AudioStreamingInterface3),
    // Isochronous Audio Data Stream Endpoint
    AudioDataStreamingEndpoint1(AudioDataStreamingEndpoint1),
    AudioDataStreamingEndpoint2(AudioDataStreamingEndpoint2),
    AudioDataStreamingEndpoint3(AudioDataStreamingEndpoint3),
    /// Generic descriptor for unsupported descriptors
    Generic(Vec<u8>),
    /// Undefined descriptor
    Undefined(Vec<u8>),
}

impl UacInterfaceDescriptor {
    const UAC1_CHANNEL_NAMES: [&'static str; 12] = [
        "Left Front (L)",
        "Right Front (R)",
        "Center Front (C)",
        "Low Frequency Enhancement (LFE)",
        "Left Surround (LS)",
        "Right Surround (RS)",
        "Left of Center (LC)",
        "Right of Center (RC)",
        "Surround (S)",
        "Side Left (SL)",
        "Side Right (SR)",
        "Top (T)",
    ];

    const UAC2_CHANNEL_NAMES: [&'static str; 27] = [
        "Front Left (FL)",
        "Front Right (FR)",
        "Front Center (FC)",
        "Low Frequency Effects (LFE)",
        "Back Left (BL)",
        "Back Right (BR)",
        "Front Left of Center (FLC)",
        "Front Right of Center (FRC)",
        "Back Center (BC)",
        "Side Left (SL)",
        "Side Right (SR)",
        "Top Center (TC)",
        "Top Front Left (TFL)",
        "Top Front Center (TFC)",
        "Top Front Right (TFR)",
        "Top Back Left (TBL)",
        "Top Back Center (TBC)",
        "Top Back Right (TBR)",
        "Top Front Left of Center (TFLC)",
        "Top Front Right of Center (TFRC)",
        "Left Low Frequency Effects (LLFE)",
        "Right Low Frequency Effects (RLFE)",
        "Top Side Left (TSL)",
        "Top Side Right (TSR)",
        "Bottom Center (BC)",
        "Back Left of Center (BLC)",
        "Back Right of Center (BRC)",
    ];

    /// Get the UAC AC interface descriptor from the UAC AC interface
    pub fn from_uac_ac_interface(
        uac_interface: &UacAcInterface,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match uac_interface {
            UacAcInterface::Header => match protocol {
                UacProtocol::Uac1 => {
                    AudioHeader1::try_from(data).map(UacInterfaceDescriptor::AudioHeader1)
                }
                UacProtocol::Uac2 => {
                    AudioHeader2::try_from(data).map(UacInterfaceDescriptor::AudioHeader2)
                }
                UacProtocol::Uac3 => {
                    AudioHeader3::try_from(data).map(UacInterfaceDescriptor::AudioHeader3)
                }
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::InputTerminal => match protocol {
                UacProtocol::Uac1 => AudioInputTerminal1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioInputTerminal1),
                UacProtocol::Uac2 => AudioInputTerminal2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioInputTerminal2),
                UacProtocol::Uac3 => AudioInputTerminal3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioInputTerminal3),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::OutputTerminal => match protocol {
                UacProtocol::Uac1 => AudioOutputTerminal1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioOutputTerminal1),
                UacProtocol::Uac2 => AudioOutputTerminal2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioOutputTerminal2),
                UacProtocol::Uac3 => AudioOutputTerminal3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioOutputTerminal3),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::ExtendedTerminal => match protocol {
                UacProtocol::Uac3 => ExtendedTerminalHeader::try_from(data)
                    .map(UacInterfaceDescriptor::ExtendedTerminalHeader),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::PowerDomain => match protocol {
                UacProtocol::Uac3 => {
                    AudioPowerDomain::try_from(data).map(UacInterfaceDescriptor::AudioPowerDomain)
                }
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::MixerUnit => match protocol {
                UacProtocol::Uac1 => {
                    AudioMixerUnit1::try_from(data).map(UacInterfaceDescriptor::AudioMixerUnit1)
                }
                UacProtocol::Uac2 => {
                    AudioMixerUnit2::try_from(data).map(UacInterfaceDescriptor::AudioMixerUnit2)
                }
                UacProtocol::Uac3 => {
                    AudioMixerUnit3::try_from(data).map(UacInterfaceDescriptor::AudioMixerUnit3)
                }
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::SelectorUnit => match protocol {
                UacProtocol::Uac1 => AudioSelectorUnit1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSelectorUnit1),
                UacProtocol::Uac2 => AudioSelectorUnit2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSelectorUnit2),
                UacProtocol::Uac3 => AudioSelectorUnit3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSelectorUnit3),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::ExtensionUnit => match protocol {
                UacProtocol::Uac1 => AudioExtensionUnit1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioExtensionUnit1),
                UacProtocol::Uac2 => AudioExtensionUnit2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioExtensionUnit2),
                UacProtocol::Uac3 => AudioExtensionUnit3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioExtensionUnit3),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::ClockSource => {
                match protocol {
                    UacProtocol::Uac2 => AudioClockSource2::try_from(data)
                        .map(UacInterfaceDescriptor::AudioClockSource2),
                    UacProtocol::Uac3 => AudioClockSource3::try_from(data)
                        .map(UacInterfaceDescriptor::AudioClockSource3),
                    _ => Err(Error::new(
                        ErrorKind::InvalidArg,
                        "Protocol not supported for this interface",
                    )),
                }
            }
            UacAcInterface::ClockSelector => match protocol {
                UacProtocol::Uac2 => AudioClockSelector2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioClockSelector2),
                UacProtocol::Uac3 => AudioClockSelector3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioClockSelector3),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::ClockMultiplier => match protocol {
                UacProtocol::Uac2 => AudioClockMultiplier2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioClockMultiplier2),
                UacProtocol::Uac3 => AudioClockMultiplier3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioClockMultiplier3),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::SampleRateConverter => match protocol {
                UacProtocol::Uac2 => AudioSampleRateConverter2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSampleRateConverter2),
                UacProtocol::Uac3 => AudioSampleRateConverter3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSampleRateConverter3),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAcInterface::Undefined => Ok(UacInterfaceDescriptor::Undefined(data.to_vec())),
            _ => Ok(UacInterfaceDescriptor::Generic(data.to_vec())),
            //_ => Err(Error::new(
            //    ErrorKind::InvalidArg,
            //    "Interface not supported for this descriptor",
            //)),
        }
    }

    /// Get the UAC AS interface descriptor from the UAC AS interface
    pub fn from_uac_as_interface(
        uac_interface: &UacAsInterface,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match uac_interface {
            UacAsInterface::General => match protocol {
                UacProtocol::Uac1 => AudioStreamingInterface1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioStreamingInterface1),
                UacProtocol::Uac2 => AudioStreamingInterface2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioStreamingInterface2),
                UacProtocol::Uac3 => AudioStreamingInterface3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioStreamingInterface3),
                _ => Err(Error::new(
                    ErrorKind::InvalidArg,
                    "Protocol not supported for this interface",
                )),
            },
            UacAsInterface::Undefined => Ok(UacInterfaceDescriptor::Undefined(data.to_vec())),
            _ => Ok(UacInterfaceDescriptor::Generic(data.to_vec())),
        }
    }

    /// Get the UAC Audio Data Endpoint descriptor from the UAC AS interface
    pub fn from_uac_as_iso_data_endpoint(
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match protocol {
            UacProtocol::Uac1 => AudioDataStreamingEndpoint1::try_from(data)
                .map(UacInterfaceDescriptor::AudioDataStreamingEndpoint1),
            UacProtocol::Uac2 => AudioDataStreamingEndpoint2::try_from(data)
                .map(UacInterfaceDescriptor::AudioDataStreamingEndpoint2),
            UacProtocol::Uac3 => AudioDataStreamingEndpoint3::try_from(data)
                .map(UacInterfaceDescriptor::AudioDataStreamingEndpoint3),
            _ => Err(Error::new(
                ErrorKind::InvalidArg,
                "Protocol not supported for this interface",
            )),
        }
    }

    /// Get bitmap string from the descriptor bit field; each bit corresponds to a string in the array
    pub fn get_bitmap_string<T: Into<u32> + Copy>(bitmap: T, strings: &[&str]) -> Vec<String> {
        let mut ret = Vec::new();
        for (i, s) in strings.iter().enumerate() {
            if bitmap.into() & (1 << i) != 0 {
                ret.push(s.to_string());
            }
        }
        ret
    }

    /// Get USB Audio Device Class channel names from the descriptor "wChannelConfig" field bitmap string based on the protocol
    pub fn get_channel_names<T: Into<u32> + Copy>(
        protocol: &UacProtocol,
        channel_config: T,
    ) -> Vec<String> {
        match protocol {
            UacProtocol::Uac1 => Self::get_bitmap_string(channel_config, &Self::UAC1_CHANNEL_NAMES),
            UacProtocol::Uac2 => Self::get_bitmap_string(channel_config, &Self::UAC2_CHANNEL_NAMES),
            _ => Vec::new(),
        }
    }

    /// Get the lock delay units from the descriptor
    pub fn get_lock_delay_units(&self) -> Option<LockDelayUnits> {
        match self {
            UacInterfaceDescriptor::AudioDataStreamingEndpoint1(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            UacInterfaceDescriptor::AudioDataStreamingEndpoint2(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            UacInterfaceDescriptor::AudioDataStreamingEndpoint3(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            _ => None,
        }
    }
}

/// USB Audio Class (UAC) protocol byte defines the version of the UAC
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum UacProtocol {
    Uac1 = 0x00,
    Uac2 = 0x20,
    Uac3 = 0x30,
    Unknown,
}

impl From<u8> for UacProtocol {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UacProtocol::Uac1,
            0x20 => UacProtocol::Uac2,
            0x30 => UacProtocol::Uac3,
            _ => UacProtocol::Unknown,
        }
    }
}

impl std::fmt::Display for UacProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UacProtocol::Uac1 => write!(f, "UAC1"),
            UacProtocol::Uac2 => write!(f, "UAC2"),
            UacProtocol::Uac3 => write!(f, "UAC3"),
            UacProtocol::Unknown => write!(f, "Unknown"),
        }
    }
}

/// The control setting for a UAC bmControls byte
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum ControlSetting {
    ReadOnly = 0b01,
    IllegalValue = 0b10,
    ReadWrite = 0b11,
}

impl From<u8> for ControlSetting {
    fn from(b: u8) -> Self {
        match b {
            0b01 => ControlSetting::ReadOnly,
            0b10 => ControlSetting::IllegalValue,
            0b11 => ControlSetting::ReadWrite,
            _ => ControlSetting::IllegalValue,
        }
    }
}

impl fmt::Display for ControlSetting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ControlSetting::ReadOnly => write!(f, "read-only"),
            ControlSetting::IllegalValue => write!(f, "ILLEGAL VALUE (0b10)"),
            ControlSetting::ReadWrite => write!(f, "read/write"),
        }
    }
}

/// UAC bmControl can be 1 bit for just the control type or 2 bits for control type and whether it's read-only
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ControlType {
    BmControl1,
    BmControl2,
}

/// UAC1: 4.3.2 Class-Specific AC Interface Descriptor; Table 4-2.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioHeader1 {
    pub version: Version,
    pub total_length: u16,
    pub collection_bytes: u8,
    pub interfaces: Vec<u8>,
}

impl TryFrom<&[u8]> for AudioHeader1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Header 1 descriptor too short",
            ));
        }

        let total_length = u16::from_le_bytes([value[2], value[3]]);
        let collection_bytes = value[4];
        let interfaces = value[5..].to_vec();

        Ok(AudioHeader1 {
            version: Version::from_bcd(u16::from_le_bytes([value[0], value[1]])),
            total_length,
            collection_bytes,
            interfaces,
        })
    }
}

/// UAC2: 4.7.2 Class-Specific AC Interface Descriptor; Table 4-5.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioHeader2 {
    pub version: Version,
    pub category: u8,
    pub total_length: u16,
    pub controls: u8,
}

impl TryFrom<&[u8]> for AudioHeader2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Header 2 descriptor too short",
            ));
        }

        let total_length = u16::from_le_bytes([value[3], value[4]]);
        let controls = value[5];

        Ok(AudioHeader2 {
            version: Version::from_bcd(u16::from_le_bytes([value[0], value[1]])),
            category: value[2],
            total_length,
            controls,
        })
    }
}

/// UAC3: 4.5.2 Class-Specific AC Interface Descriptor; Table 4-15.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioHeader3 {
    pub category: u8,
    pub total_length: u16,
    pub controls: u32,
}

impl TryFrom<&[u8]> for AudioHeader3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Header 3 descriptor too short",
            ));
        }

        let total_length = u16::from_le_bytes([value[1], value[2]]);
        let controls = u32::from_le_bytes([value[3], value[4], value[5], value[6]]);

        Ok(AudioHeader3 {
            category: value[0],
            total_length,
            controls,
        })
    }
}

/// UAC1: 4.3.2.1 Input Terminal Descriptor; Table 4-3.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioInputTerminal1 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub nr_channels: u8,
    pub channel_config: u16,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for AudioInputTerminal1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Input Terminal 1 descriptor too short",
            ));
        }

        Ok(AudioInputTerminal1 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            nr_channels: value[4],
            channel_config: u16::from_le_bytes([value[5], value[6]]),
            channel_names_index: value[7],
            channel_names: None,
            terminal_index: value[8],
            terminal: None,
        })
    }
}

/// UAC2: 4.7.2.4 Input Terminal Descriptor; Table 4-9.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioInputTerminal2 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub csource_id: u8,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
    pub controls: u16,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for AudioInputTerminal2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 14 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Input Terminal 2 descriptor too short",
            ));
        }

        Ok(AudioInputTerminal2 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            csource_id: value[4],
            nr_channels: value[5],
            channel_config: u32::from_le_bytes([value[6], value[7], value[8], value[9]]),
            channel_names_index: value[10],
            channel_names: None,
            controls: u16::from_le_bytes([value[11], value[12]]),
            terminal_index: value[13],
            terminal: None,
        })
    }
}

/// UAC3: 4.5.2.1 Input Terminal Descriptor; Table 4-16.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioInputTerminal3 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub csource_id: u8,
    pub controls: u32,
    pub cluster_descr_id: u16,
    pub ex_terminal_descr_id: u16,
    pub connectors_descr_id: u16,
    pub terminal_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioInputTerminal3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 17 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Input Terminal 3 descriptor too short",
            ));
        }

        Ok(AudioInputTerminal3 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            csource_id: value[4],
            controls: u32::from_le_bytes([value[5], value[6], value[7], value[8]]),
            cluster_descr_id: u16::from_le_bytes([value[9], value[10]]),
            ex_terminal_descr_id: u16::from_le_bytes([value[11], value[12]]),
            connectors_descr_id: u16::from_le_bytes([value[13], value[14]]),
            terminal_descr_str: u16::from_le_bytes([value[15], value[16]]),
        })
    }
}

/// UAC1: 4.3.2.2 Output Terminal Descriptor; Table 4-4.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioOutputTerminal1 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub source_id: u8,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for AudioOutputTerminal1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Output Terminal 1 descriptor too short",
            ));
        }

        Ok(AudioOutputTerminal1 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            source_id: value[4],
            terminal_index: value[5],
            terminal: None,
        })
    }
}

/// UAC2: 4.7.2.5 Output Terminal Descriptor; Table 4-10.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioOutputTerminal2 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub source_id: u8,
    pub c_source_id: u8,
    pub controls: u16,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for AudioOutputTerminal2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Output Terminal 2 descriptor too short",
            ));
        }

        Ok(AudioOutputTerminal2 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            source_id: value[4],
            c_source_id: value[5],
            controls: u16::from_le_bytes([value[6], value[7]]),
            terminal_index: value[8],
            terminal: None,
        })
    }
}

/// UAC3: 4.5.2.2 Output Terminal Descriptor; Table 4-17.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioOutputTerminal3 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub source_id: u8,
    pub c_source_id: u8,
    pub controls: u32,
    pub ex_terminal_descr_id: u16,
    pub connectors_descr_id: u16,
    pub terminal_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioOutputTerminal3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 17 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Output Terminal 3 descriptor too short",
            ));
        }

        Ok(AudioOutputTerminal3 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            source_id: value[4],
            c_source_id: value[5],
            controls: u32::from_le_bytes([value[6], value[7], value[8], value[9]]),
            ex_terminal_descr_id: u16::from_le_bytes([value[10], value[11]]),
            connectors_descr_id: u16::from_le_bytes([value[12], value[13]]),
            terminal_descr_str: u16::from_le_bytes([value[14], value[15]]),
        })
    }
}

/// UAC3: 4.5.2.3.1 Extended Terminal Header Descriptor; Table 4-18.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ExtendedTerminalHeader {
    pub descriptor_id: u8,
    pub nr_channels: u8,
}

impl TryFrom<&[u8]> for ExtendedTerminalHeader {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 2 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Extended Terminal Header descriptor too short",
            ));
        }

        Ok(ExtendedTerminalHeader {
            descriptor_id: value[0],
            nr_channels: value[1],
        })
    }
}

/// UAC3: 4.5.2.15 Power Domain Descriptor; Table 4-46. */
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioPowerDomain {
    pub power_domain_id: u8,
    pub recovery_time_1: u16,
    pub recovery_time_2: u16,
    pub nr_entities: u8,
    pub entity_ids: Vec<u8>,
    pub domain_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioPowerDomain {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Power Domain 3 descriptor too short",
            ));
        }

        let nr_entities = value[5] as usize;
        let expected_len = 8 + nr_entities;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Power Domain 3 descriptor too short for the number of entities",
            ));
        }

        Ok(AudioPowerDomain {
            power_domain_id: value[0],
            recovery_time_1: u16::from_le_bytes([value[1], value[2]]),
            recovery_time_2: u16::from_le_bytes([value[3], value[4]]),
            nr_entities: value[5],
            entity_ids: value[6..6 + nr_entities].to_vec(),
            domain_descr_str: u16::from_le_bytes([value[6 + nr_entities], value[7 + nr_entities]]),
        })
    }
}

/// UAC1: 4.3.2.3 Mixer Unit Descriptor; Table 4-5.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioMixerUnit1 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u16,
    pub channel_names: u8,
    pub controls: Vec<u8>,
    pub mixer: u8,
}

impl TryFrom<&[u8]> for AudioMixerUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 1 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let nr_channels = value[3] as usize;
        let expected_len = 6 + nr_in_pins + nr_channels;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 1 descriptor too short for the number of pins and channels",
            ));
        }

        Ok(AudioMixerUnit1 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids: value[2..2 + nr_in_pins].to_vec(),
            nr_channels: value[2 + nr_in_pins],
            channel_config: u16::from_le_bytes([value[3 + nr_in_pins], value[4 + nr_in_pins]]),
            channel_names: value[5 + nr_in_pins],
            controls: value[6 + nr_in_pins..6 + nr_in_pins + nr_channels].to_vec(),
            mixer: value[6 + nr_in_pins + nr_channels],
        })
    }
}

/// UAC2: 4.7.2.6 Mixer Unit Descriptor; Table 4-11.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioMixerUnit2 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names: u8,
    pub mixer_controls: Vec<u8>,
    pub controls: u8,
    pub mixer: u8,
}

impl TryFrom<&[u8]> for AudioMixerUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let nr_channels = value[3] as usize;
        let expected_len = 10 + nr_in_pins + nr_channels;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 2 descriptor too short for the number of pins and channels",
            ));
        }

        Ok(AudioMixerUnit2 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids: value[2..2 + nr_in_pins].to_vec(),
            nr_channels: value[2 + nr_in_pins],
            channel_config: u32::from_le_bytes([
                value[3 + nr_in_pins],
                value[4 + nr_in_pins],
                value[5 + nr_in_pins],
                value[6 + nr_in_pins],
            ]),
            channel_names: value[7 + nr_in_pins],
            mixer_controls: value[8 + nr_in_pins..8 + nr_in_pins + nr_channels].to_vec(),
            controls: value[8 + nr_in_pins + nr_channels],
            mixer: value[9 + nr_in_pins + nr_channels],
        })
    }
}

/// UAC3: 4.5.2.5 Mixer Unit Descriptor; Table 4-29.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioMixerUnit3 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub cluster_descr_id: u16,
    pub mixer_controls: Vec<u8>,
    pub controls: u32,
    pub mixer_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioMixerUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_len = 8 + nr_in_pins;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 3 descriptor too short for the number of pins",
            ));
        }

        Ok(AudioMixerUnit3 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids: value[2..2 + nr_in_pins].to_vec(),
            cluster_descr_id: u16::from_le_bytes([value[2 + nr_in_pins], value[3 + nr_in_pins]]),
            mixer_controls: value[4 + nr_in_pins..4 + nr_in_pins + 1].to_vec(),
            controls: u32::from_le_bytes([
                value[5 + nr_in_pins],
                value[6 + nr_in_pins],
                value[7 + nr_in_pins],
                value[8 + nr_in_pins],
            ]),
            mixer_descr_str: u16::from_le_bytes([value[9 + nr_in_pins], value[10 + nr_in_pins]]),
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioStreamingInterface1 {
    pub terminal_link: u8,
    pub delay: u8,
    pub format_tag: u16,
}

impl TryFrom<&[u8]> for AudioStreamingInterface1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Streaming Interface 1 descriptor too short",
            ));
        }

        Ok(AudioStreamingInterface1 {
            terminal_link: value[0],
            delay: value[1],
            format_tag: u16::from_le_bytes([value[2], value[3]]),
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioStreamingInterface2 {
    pub terminal_link: u8,
    pub controls: u8, // BmControl2
    pub format_type: u8,
    pub formats: u32,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
}

impl TryFrom<&[u8]> for AudioStreamingInterface2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 13 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Streaming Interface 2 descriptor too short",
            ));
        }

        Ok(AudioStreamingInterface2 {
            terminal_link: value[0],
            controls: value[1],
            format_type: value[2],
            formats: u32::from_le_bytes([value[3], value[4], value[5], value[6]]),
            nr_channels: value[7],
            channel_config: u32::from_le_bytes([value[8], value[9], value[10], value[11]]),
            channel_names_index: value[12],
            channel_names: None,
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioStreamingInterface3 {
    pub terminal_link: u8,
    pub controls: u32, // BmControl2
    pub cluster_descr_id: u16,
    pub formats: u64,
    pub sub_slot_size: u8,
    pub bit_resolution: u8,
    pub aux_protocols: u16,
    pub control_size: u8,
}

impl TryFrom<&[u8]> for AudioStreamingInterface3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 20 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Streaming Interface 3 descriptor too short",
            ));
        }

        Ok(AudioStreamingInterface3 {
            terminal_link: value[0],
            controls: u32::from_le_bytes([value[1], value[2], value[3], value[4]]),
            cluster_descr_id: u16::from_le_bytes([value[5], value[6]]),
            formats: u64::from_le_bytes([
                value[7], value[8], value[9], value[10], value[11], value[12], value[13], value[14],
            ]),
            sub_slot_size: value[15],
            bit_resolution: value[16],
            aux_protocols: u16::from_le_bytes([value[17], value[18]]),
            control_size: value[19],
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum LockDelayUnits {
    Undefined,
    Milliseconds,
    DecodedPcmSamples,
}

impl From<u8> for LockDelayUnits {
    fn from(b: u8) -> Self {
        match b {
            0 => LockDelayUnits::Undefined,
            1 => LockDelayUnits::Milliseconds,
            2 => LockDelayUnits::DecodedPcmSamples,
            _ => LockDelayUnits::Undefined,
        }
    }
}

impl fmt::Display for LockDelayUnits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LockDelayUnits::Undefined => write!(f, "Undefined"),
            LockDelayUnits::Milliseconds => write!(f, "Milliseconds"),
            LockDelayUnits::DecodedPcmSamples => write!(f, "Decoded PCM samples"),
        }
    }
}

/// Isochronous Audio Data Stream Endpoint for UAC1
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioDataStreamingEndpoint1 {
    pub attributes: u8,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl TryFrom<&[u8]> for AudioDataStreamingEndpoint1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Data Streaming Endpoint 1 descriptor too short",
            ));
        }

        Ok(AudioDataStreamingEndpoint1 {
            attributes: value[0],
            lock_delay_units: value[1],
            lock_delay: u16::from_le_bytes([value[2], value[3]]),
        })
    }
}

/// Isochronous Audio Data Stream Endpoint for UAC2
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioDataStreamingEndpoint2 {
    pub attributes: u8,
    pub controls: u8,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl TryFrom<&[u8]> for AudioDataStreamingEndpoint2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Data Streaming Endpoint 2 descriptor too short",
            ));
        }

        Ok(AudioDataStreamingEndpoint2 {
            attributes: value[0],
            controls: value[1],
            lock_delay_units: value[2],
            lock_delay: u16::from_le_bytes([value[3], value[4]]),
        })
    }
}

/// Isochronous Audio Data Stream Endpoint for UAC3
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioDataStreamingEndpoint3 {
    pub controls: u32,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl TryFrom<&[u8]> for AudioDataStreamingEndpoint3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Data Streaming Endpoint 3 descriptor too short",
            ));
        }

        Ok(AudioDataStreamingEndpoint3 {
            controls: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
            lock_delay_units: value[4],
            lock_delay: u16::from_le_bytes([value[5], value[6]]),
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSelectorUnit1 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub selector_index: u8,
}

impl TryFrom<&[u8]> for AudioSelectorUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 1 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 3 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 1 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(AudioSelectorUnit1 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids,
            selector_index: value[expected_length - 1],
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSelectorUnit2 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub controls: u8,
    pub selector_index: u8,
}

impl TryFrom<&[u8]> for AudioSelectorUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 4 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 2 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(AudioSelectorUnit2 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids,
            controls: value[2 + nr_in_pins],
            selector_index: value[expected_length - 1],
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSelectorUnit3 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub controls: u32,
    pub selector_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioSelectorUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 6 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 3 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();
        let controls = u32::from_le_bytes([
            value[2 + nr_in_pins],
            value[3 + nr_in_pins],
            value[4 + nr_in_pins],
            value[5 + nr_in_pins],
        ]);

        Ok(AudioSelectorUnit3 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids,
            controls,
            selector_descr_str: u16::from_le_bytes([
                value[expected_length - 2],
                value[expected_length - 1],
            ]),
        })
    }
}

/// UAC1: 4.3.2.7 Extension Unit Descriptor; Table 4-15.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioExtensionUnit1 {
    pub unit_id: u8,
    pub extension_code: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u16,
    pub channel_names_index: u8,
    pub control_size: u8,
    pub controls: Vec<u8>,
    pub extension_index: u8,
}

impl TryFrom<&[u8]> for AudioExtensionUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 1 descriptor too short",
            ));
        }

        let nr_in_pins = value[3] as usize;
        let control_size = value[8 + nr_in_pins];
        let expected_length = 10 + nr_in_pins + control_size as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 1 descriptor too short",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();
        let controls = value[(9 + nr_in_pins)..(9 + nr_in_pins + control_size as usize)].to_vec();

        Ok(AudioExtensionUnit1 {
            unit_id: value[0],
            extension_code: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins: value[3],
            source_ids,
            nr_channels: value[4 + nr_in_pins],
            channel_config: u16::from_le_bytes([value[5 + nr_in_pins], value[6 + nr_in_pins]]),
            channel_names_index: value[7 + nr_in_pins],
            control_size,
            controls,
            extension_index: value[expected_length - 1],
        })
    }
}

/// UAC2: 4.7.2.12 Extension Unit Descriptor; Table 4-24.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioExtensionUnit2 {
    pub unit_id: u8,
    pub extension_code: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names_index: u8,
    pub controls: u8,
    pub extension_index: u8,
}

impl TryFrom<&[u8]> for AudioExtensionUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 11 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[3] as usize;
        let expected_length = 10 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 2 descriptor too short",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();

        Ok(AudioExtensionUnit2 {
            unit_id: value[0],
            extension_code: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins: value[3],
            source_ids,
            nr_channels: value[4 + nr_in_pins],
            channel_config: u32::from_le_bytes([
                value[5 + nr_in_pins],
                value[6 + nr_in_pins],
                value[7 + nr_in_pins],
                value[8 + nr_in_pins],
            ]),
            channel_names_index: value[9 + nr_in_pins],
            controls: value[10 + nr_in_pins],
            extension_index: value[11 + nr_in_pins],
        })
    }
}

/// UAC3: 4.5.2.11 Extension Unit Descriptor; Table 4-42.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioExtensionUnit3 {
    pub unit_id: u8,
    pub extension_code: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub extension_descr_str: u16,
    pub controls: u32,
    pub cluster_descr_id: u16,
}

impl TryFrom<&[u8]> for AudioExtensionUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[3] as usize;
        let expected_length = 9 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 3 descriptor too short",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();

        Ok(AudioExtensionUnit3 {
            unit_id: value[0],
            extension_code: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins: value[3],
            source_ids,
            extension_descr_str: u16::from_le_bytes([value[4 + nr_in_pins], value[5 + nr_in_pins]]),
            controls: u32::from_le_bytes([
                value[6 + nr_in_pins],
                value[7 + nr_in_pins],
                value[8 + nr_in_pins],
                value[9 + nr_in_pins],
            ]),
            cluster_descr_id: u16::from_le_bytes([value[10 + nr_in_pins], value[11 + nr_in_pins]]),
        })
    }
}

/// UAC2: 4.7.2.1 Clock Source Descriptor; Table 4-6.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockSource2 {
    pub clock_id: u8,
    pub attributes: u8,
    pub controls: u8,
    pub assoc_terminal: u8,
    pub clock_source_index: u8,
}

impl TryFrom<&[u8]> for AudioClockSource2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Source 2 descriptor too short",
            ));
        }

        Ok(AudioClockSource2 {
            clock_id: value[0],
            attributes: value[1],
            controls: value[2],
            assoc_terminal: value[3],
            clock_source_index: value[4],
        })
    }
}

/// UAC3: 4.5.2.12 Clock Source Descriptor; Table 4-43.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockSource3 {
    pub clock_id: u8,
    pub attributes: u8,
    pub controls: u32,
    pub reference_terminal: u8,
    pub clock_source_str: u16,
}

impl TryFrom<&[u8]> for AudioClockSource3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Source 3 descriptor too short",
            ));
        }

        Ok(AudioClockSource3 {
            clock_id: value[0],
            attributes: value[1],
            controls: u32::from_le_bytes([value[2], value[3], value[4], value[5]]),
            reference_terminal: value[6],
            clock_source_str: u16::from_le_bytes([value[7], value[8]]),
        })
    }
}

/// UAC2: 4.7.2.2 Clock Selector Descriptor; Table 4-7.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockSelector2 {
    pub clock_id: u8,
    pub nr_in_pins: u8,
    pub csource_ids: Vec<u8>,
    pub controls: u8,
    pub clock_selector_index: u8,
}

impl TryFrom<&[u8]> for AudioClockSelector2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Selector 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 3 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Selector 2 descriptor too short",
            ));
        }

        let csource_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(AudioClockSelector2 {
            clock_id: value[0],
            nr_in_pins: value[1],
            csource_ids,
            controls: value[2 + nr_in_pins],
            clock_selector_index: value[expected_length - 1],
        })
    }
}

/// UAC3: 4.5.2.13 Clock Selector Descriptor; Table 4-44.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockSelector3 {
    pub clock_id: u8,
    pub nr_in_pins: u8,
    pub csource_ids: Vec<u8>,
    pub controls: u32,
    pub cselector_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioClockSelector3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Selector 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 5 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Selector 3 descriptor too short",
            ));
        }

        let csource_ids = value[2..(2 + nr_in_pins)].to_vec();
        let controls = u32::from_le_bytes([
            value[2 + nr_in_pins],
            value[3 + nr_in_pins],
            value[4 + nr_in_pins],
            value[5 + nr_in_pins],
        ]);

        Ok(AudioClockSelector3 {
            clock_id: value[0],
            nr_in_pins: value[1],
            csource_ids,
            controls,
            cselector_descr_str: u16::from_le_bytes([
                value[expected_length - 2],
                value[expected_length - 1],
            ]),
        })
    }
}

/// UAC2: 4.7.2.3 Clock Multiplier Descriptor; Table 4-8.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockMultiplier2 {
    pub clock_id: u8,
    pub csource_id: u8,
    pub controls: u8,
    pub clock_multiplier_index: u8,
}

impl TryFrom<&[u8]> for AudioClockMultiplier2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Multiplier 2 descriptor too short",
            ));
        }

        Ok(AudioClockMultiplier2 {
            clock_id: value[0],
            csource_id: value[1],
            controls: value[2],
            clock_multiplier_index: value[3],
        })
    }
}

/// UAC3: 4.5.2.14 Clock Multiplier Descriptor; Table 4-45.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockMultiplier3 {
    pub clock_id: u8,
    pub csource_id: u8,
    pub controls: u32,
    pub cmultiplier_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioClockMultiplier3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Multiplier 3 descriptor too short",
            ));
        }

        Ok(AudioClockMultiplier3 {
            clock_id: value[0],
            csource_id: value[1],
            controls: u32::from_le_bytes([value[2], value[3], value[4], value[5]]),
            cmultiplier_descr_str: u16::from_le_bytes([value[6], value[7]]),
        })
    }
}

/// UAC2: 4.7.2.9 Sampling Rate Converter Descriptor; Table 4-14.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSampleRateConverter2 {
    pub unit_id: u8,
    pub source_id: u8,
    pub csource_in_id: u8,
    pub csource_out_id: u8,
    pub src_index: u8,
}

impl TryFrom<&[u8]> for AudioSampleRateConverter2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Sample Rate Converter 2 descriptor too short",
            ));
        }

        Ok(AudioSampleRateConverter2 {
            unit_id: value[0],
            source_id: value[1],
            csource_in_id: value[2],
            csource_out_id: value[3],
            src_index: value[4],
        })
    }
}

/// UAC3: 4.5.2.8 Sampling Rate Converter Descriptor; Table 4-32.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSampleRateConverter3 {
    pub unit_id: u8,
    pub source_id: u8,
    pub csource_in_id: u8,
    pub csource_out_id: u8,
    pub src_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioSampleRateConverter3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Sample Rate Converter 3 descriptor too short",
            ));
        }

        Ok(AudioSampleRateConverter3 {
            unit_id: value[0],
            source_id: value[1],
            csource_in_id: value[2],
            csource_out_id: value[3],
            src_descr_str: u16::from_le_bytes([value[4], value[5]]),
        })
    }
}
