//! Defines for the USB Video Class (UVC) interface descriptors
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use super::*;
use super::audio;
use crate::error::{self, Error, ErrorKind};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[non_exhaustive]
pub enum ControlSubtype {
    Undefined = 0x00,
    Header = 0x01,
    InputTerminal = 0x02,
    OutputTerminal = 0x03,
    SelectorUnit = 0x04,
    ProcessingUnit = 0x05,
    ExtensionUnit = 0x06,
    EncodingUnit = 0x07,
}

impl std::fmt::Display for ControlSubtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // lsusb style
        if f.alternate() {
            match self {
                ControlSubtype::Undefined => write!(f, "unknown"),
                _ => write!(f, "{}", heck::AsShoutySnakeCase(format!("{:?}", self))),
            }
        } else {
            write!(f, "{:?}", self)
        }
    }
}

impl From<u8> for ControlSubtype {
    fn from(b: u8) -> Self {
        match b {
            0x00 => ControlSubtype::Undefined,
            0x01 => ControlSubtype::Header,
            0x02 => ControlSubtype::InputTerminal,
            0x03 => ControlSubtype::OutputTerminal,
            0x04 => ControlSubtype::SelectorUnit,
            0x05 => ControlSubtype::ProcessingUnit,
            0x06 => ControlSubtype::ExtensionUnit,
            0x07 => ControlSubtype::EncodingUnit,
            _ => ControlSubtype::Undefined,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[non_exhaustive]
pub enum StreamingSubtype {
    Undefined = 0x00,
    InputHeader = 0x01,
    OutputHeader = 0x02,
    StillImageFrame = 0x03,
    FormatUncompressed = 0x04,
    FrameUncompressed = 0x05,
    FormatMJPEG = 0x06,
    FrameMJPEG = 0x07,
    FormatFrameBased = 0x10,
    FrameFrameBased = 0x11,
    FormatStreamBased = 0x12,
    FormatMPEG2TS = 0x0a,
    ColorFormat = 0x0d,
}

impl std::fmt::Display for StreamingSubtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // lsusb style
        if f.alternate() {
            match self {
                StreamingSubtype::Undefined => write!(f, "unknown"),
                _ => write!(f, "{}", heck::AsShoutySnakeCase(format!("{:?}", self))),
            }
        } else {
            write!(f, "{:?}", self)
        }
    }
}

impl From<u8> for StreamingSubtype {
    fn from(b: u8) -> Self {
        match b {
            0x00 => StreamingSubtype::Undefined,
            0x01 => StreamingSubtype::InputHeader,
            0x02 => StreamingSubtype::OutputHeader,
            0x03 => StreamingSubtype::StillImageFrame,
            0x04 => StreamingSubtype::FormatUncompressed,
            0x05 => StreamingSubtype::FrameUncompressed,
            0x06 => StreamingSubtype::FormatMJPEG,
            0x07 => StreamingSubtype::FrameMJPEG,
            0x10 => StreamingSubtype::FormatFrameBased,
            0x11 => StreamingSubtype::FrameFrameBased,
            0x12 => StreamingSubtype::FormatStreamBased,
            0x0a => StreamingSubtype::FormatMPEG2TS,
            0x0d => StreamingSubtype::ColorFormat,
            _ => StreamingSubtype::Undefined,
        }
    }
}

/// USB Video Class (UVC) subtype based on the bDescriptorSubtype
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum UvcType {
    /// Video Control Interface
    Control(ControlSubtype),
    /// Video Streaming Interface
    Streaming(StreamingSubtype),
}

impl std::fmt::Display for UvcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            match self {
                UvcType::Control(c) => write!(f, "{:#}", c),
                UvcType::Streaming(s) => write!(f, "{:#}", s),
            }
        } else {
            match self {
                UvcType::Control(c) => write!(f, "{}", c),
                UvcType::Streaming(s) => write!(f, "{}", s),
            }
        }
    }
}

/// From a tuple of (SubClass, DescriptorSub, Protocol) get the UAC subtype
impl TryFrom<(u8, u8, u8)> for UvcType {
    type Error = Error;

    fn try_from(value: (u8, u8, u8)) -> error::Result<Self> {
        match value.0 {
            1 => Ok(UvcType::Control(ControlSubtype::from(value.1))),
            2 => Ok(UvcType::Streaming(StreamingSubtype::from(value.1))),
            _ => Err(Error::new(
                ErrorKind::InvalidArg,
                "Invalid UVC descriptor type",
            )),
        }
    }
}

impl From<UvcType> for u8 {
    fn from(uvc: UvcType) -> Self {
        match uvc {
            UvcType::Control(c) => c as u8,
            UvcType::Streaming(s) => s as u8,
        }
    }
}

impl UvcType {
    /// Get the UVC descriptor from protocol and descriptor interface data
    pub fn get_uvc_descriptor(
        &self,
        _protocol: u8,
        data: &[u8],
    ) -> error::Result<UvcInterfaceDescriptor> {
        match self {
            UvcType::Control(c) => {
                match c {
                    ControlSubtype::Header => {
                        Ok(UvcInterfaceDescriptor::Header(Header::try_from(data)?))
                    }
                    ControlSubtype::InputTerminal => {
                        Ok(UvcInterfaceDescriptor::InputTerminal(InputTerminal::try_from(data)?))
                    }
                    ControlSubtype::OutputTerminal => {
                        Ok(UvcInterfaceDescriptor::OutputTerminal(OutputTerminal::try_from(data)?))
                    }
                    ControlSubtype::SelectorUnit => {
                        Ok(UvcInterfaceDescriptor::SelectorUnit(SelectorUnit::try_from(data)?))
                    }
                    ControlSubtype::ProcessingUnit => {
                        Ok(UvcInterfaceDescriptor::ProcessingUnit(ProcessingUnit::try_from(data)?))
                    }
                    ControlSubtype::ExtensionUnit => {
                        Ok(UvcInterfaceDescriptor::ExtensionUnit(ExtensionUnit::try_from(data)?))
                    }
                    ControlSubtype::EncodingUnit => {
                        Ok(UvcInterfaceDescriptor::EncodingUnit(EncodingUnit::try_from(data)?))
                    }
                    ControlSubtype::Undefined => Ok(UvcInterfaceDescriptor::Undefined(data.to_vec())),
                    //_ => Ok(UvcInterfaceDescriptor::Generic(data.to_vec())),
                }
            }
            UvcType::Streaming(_s) => {
                Ok(UvcInterfaceDescriptor::Generic(data.to_vec()))
            }
        }
    }
    /// Get the UVC descriptor from a generic descriptor and protocol
    pub fn uvc_descriptor_from_generic(
        &self,
        gd: GenericDescriptor,
        _protocol: u8,
    ) -> error::Result<UvcInterfaceDescriptor> {
        match gd.data {
            Some(data) => match self.get_uvc_descriptor(_protocol, &data) {
                Ok(v) => Ok(v),
                Err(e) => {
                    log::error!("Error parsing UVC descriptor: {}", e);
                    Ok(UvcInterfaceDescriptor::Invalid(data))
                }
            }
            None => Err(Error::new(
                ErrorKind::InvalidArg,
                "No data in generic descriptor",
            )),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UvcDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub subtype: UvcType,
    pub interface: UvcInterfaceDescriptor,
}

/// Try from ([`GenericDescriptor`], SubClass, Protocol)
impl TryFrom<(GenericDescriptor, u8, u8)> for UvcDescriptor {
    type Error = Error;

    fn try_from((gd, subc, p): (GenericDescriptor, u8, u8)) -> error::Result<Self> {
        let length = gd.length;
        let descriptor_type = gd.descriptor_type;
        let subtype: UvcType = (subc, gd.descriptor_subtype, p).try_into()?;
        let interface = subtype.uvc_descriptor_from_generic(gd.to_owned(), p)?;

        Ok(UvcDescriptor {
            length,
            descriptor_type,
            subtype,
            interface,
        })
    }
}

impl From<UvcDescriptor> for Vec<u8> {
    fn from(vcd: UvcDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(vcd.length);
        ret.push(vcd.descriptor_type);
        ret.push(u8::from(vcd.subtype));
        let data: Vec<u8> = vcd.interface.into();
        ret.extend(data);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum UvcInterfaceDescriptor {
    // Control
    Header(Header),
    InputTerminal(InputTerminal),
    OutputTerminal(OutputTerminal),
    SelectorUnit(SelectorUnit),
    ProcessingUnit(ProcessingUnit),
    ExtensionUnit(ExtensionUnit),
    EncodingUnit(EncodingUnit),
    // Streaming
    /// Invalid descriptor for failing to parse matched
    Invalid(Vec<u8>),
    /// Generic descriptor for known but unsupported descriptors
    Generic(Vec<u8>),
    /// Undefined descriptor
    Undefined(Vec<u8>),
}

impl From<UvcInterfaceDescriptor> for Vec<u8> {
    fn from(uvc: UvcInterfaceDescriptor) -> Self {
        match uvc {
            UvcInterfaceDescriptor::Header(h) => h.into(),
            UvcInterfaceDescriptor::InputTerminal(it) => it.into(),
            UvcInterfaceDescriptor::OutputTerminal(ot) => ot.into(),
            UvcInterfaceDescriptor::SelectorUnit(su) => su.into(),
            UvcInterfaceDescriptor::ProcessingUnit(pu) => pu.into(),
            UvcInterfaceDescriptor::ExtensionUnit(eu) => eu.into(),
            UvcInterfaceDescriptor::EncodingUnit(eu) => eu.into(),
            UvcInterfaceDescriptor::Invalid(data) => data,
            UvcInterfaceDescriptor::Generic(data) => data,
            UvcInterfaceDescriptor::Undefined(data) => data,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Header {
    pub version: Version,
    pub total_length: u16,
    pub clock_frequency: u32,
    pub collection_bytes: u8,
    pub interfaces: Vec<u8>,
}

impl TryFrom<&[u8]> for Header {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Video Control descriptor too short",
            ));
        }

        let version = Version::from_bcd(u16::from_le_bytes([value[0], value[1]]));
        let total_length = u16::from_le_bytes([value[2], value[3]]);
        let clock_frequency = u32::from_le_bytes([value[4], value[5], value[6], value[7]]);
        let collection_bytes = value[8];
        let interfaces = value[9..].to_vec();

        Ok(Header {
            version,
            total_length,
            clock_frequency,
            collection_bytes,
            interfaces,
        })
    }
}

impl From<Header> for Vec<u8> {
    fn from(h: Header) -> Self {
        let mut ret = Vec::new();
        ret.extend_from_slice(&(u16::from(h.version)).to_le_bytes());
        ret.extend_from_slice(&h.total_length.to_le_bytes());
        ret.extend_from_slice(&h.clock_frequency.to_le_bytes());
        ret.push(h.collection_bytes);
        ret.extend(h.interfaces);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TerminalExtra {
    pub objective_focal_length_min: u16,
    pub objective_focal_length_max: u16,
    pub ocular_focal_length: u16,
    pub control_size: u8,
    pub controls: u32,
}

impl TryFrom<&[u8]> for TerminalExtra {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!(
                    "Terminal Extra descriptor too short {} < {}",
                    value.len(),
                    8
                )
            ));
        }

        let objective_focal_length_min = u16::from_le_bytes([value[0], value[1]]);
        let objective_focal_length_max = u16::from_le_bytes([value[2], value[3]]);
        let ocular_focal_length = u16::from_le_bytes([value[4], value[5]]);
        let control_size = value[6];

        if value.len() < 7 + control_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!(
                    "Terminal Extra descriptor too short for control size {} < {}",
                    value.len(),
                    7 + control_size
                )
            ));
        }

        let mut controls: u32 = 0;
        for i in 0..control_size.min(3) as usize {
            controls |= (value[7 + i] as u32) << (i * 8);
        }

        Ok(TerminalExtra {
            objective_focal_length_min,
            objective_focal_length_max,
            ocular_focal_length,
            control_size,
            controls,
        })
    }
}

impl From<TerminalExtra> for Vec<u8> {
    fn from(te: TerminalExtra) -> Self {
        let mut ret = Vec::new();
        ret.extend_from_slice(&te.objective_focal_length_min.to_le_bytes());
        ret.extend_from_slice(&te.objective_focal_length_max.to_le_bytes());
        ret.extend_from_slice(&te.ocular_focal_length.to_le_bytes());
        ret.push(te.control_size);

        for i in 0..te.control_size.min(3) {
            ret.push((te.controls >> (i * 8)) as u8);
        }

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct InputTerminal {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub associated_terminal: u8,
    pub terminal_index: u8,
    pub terminal: Option<String>,
    pub extra: Option<TerminalExtra>,
}

impl TryFrom<&[u8]> for InputTerminal {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!("Input Terminal descriptor too short {} < {}", value.len(), 5),
            ));
        }

        let terminal_id = value[0];
        let terminal_type = u16::from_le_bytes([value[1], value[2]]);
        let associated_terminal = value[3];
        let terminal_string_index = value[4];
        let terminal_string = None;

        let extra = if terminal_type == 0x0201 && value.len() > 5 {
            Some(TerminalExtra::try_from(&value[5..])?)
        } else {
            None
        };

        Ok(InputTerminal {
            terminal_id,
            terminal_type,
            associated_terminal,
            terminal_index: terminal_string_index,
            terminal: terminal_string,
            extra,
        })
    }
}

impl From<InputTerminal> for Vec<u8> {
    fn from(it: InputTerminal) -> Self {
        let mut ret = Vec::new();
        ret.push(it.terminal_id);
        ret.extend_from_slice(&it.terminal_type.to_le_bytes());
        ret.push(it.associated_terminal);
        ret.push(it.terminal_index);

        if let Some(extra) = it.extra {
            let extra: Vec<u8> = extra.into();
            ret.extend(extra);
        }

        ret
    }
}

/// Output Terminal descriptor; same as [`audio::OutputTerminal1`]
pub type OutputTerminal = audio::OutputTerminal1;
/// Selector Unit descriptor; same as [`audio::SelectorUnit1`]
pub type SelectorUnit = audio::SelectorUnit1;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ProcessingUnit {
    pub unit_id: u8,
    pub source_id: u8,
    pub max_multiplier: u16,
    pub control_size: u8,
    pub controls: u32,
    pub processing_index: u8,
    pub processing: Option<String>,
    pub video_standards: u8,
}

impl TryFrom<&[u8]> for ProcessingUnit {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!("Processing Unit descriptor too short {} < {}", value.len(), 9),
            ));
        }

        let unit_id = value[0];
        let source_id = value[1];
        let max_multiplier = u16::from_le_bytes([value[2], value[3]]);
        let control_size = value[4];

        if value.len() < 7 + control_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!(
                    "Processing Unit descriptor too short for control size {} < {}",
                    value.len(),
                    7 + control_size
                ),
            ));
        }

        let mut controls: u32 = 0;
        for i in 0..control_size.min(3) as usize {
            controls |= (value[5 + i] as u32) << (i * 8);
        }

        let processing_string_index = value[5 + control_size as usize];
        let video_standards = value[6 + control_size as usize];

        Ok(ProcessingUnit {
            unit_id,
            source_id,
            max_multiplier,
            control_size,
            controls,
            processing_index: processing_string_index,
            processing: None,
            video_standards,
        })
    }
}

impl From<ProcessingUnit> for Vec<u8> {
    fn from(pu: ProcessingUnit) -> Self {
        let mut ret = Vec::new();
        ret.push(pu.unit_id);
        ret.push(pu.source_id);
        ret.extend_from_slice(&pu.max_multiplier.to_le_bytes());
        ret.push(pu.control_size);

        for i in 0..pu.control_size.min(3) {
            ret.push((pu.controls >> (i * 8)) as u8);
        }

        ret.push(pu.processing_index);
        ret.push(pu.video_standards);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ExtensionUnit {
    pub unit_id: u8,
    pub guid_extension_code: String,
    pub num_controls: u8,
    pub num_input_pins: u8,
    pub source_ids: Vec<u8>,
    pub control_size: u8,
    pub controls: Vec<u8>,
    pub extension_index: u8,
    pub extension: Option<String>,
}

impl TryFrom<&[u8]> for ExtensionUnit {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 21 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!("Extension Unit descriptor too short {} < {}", value.len(), 21),
            ));
        }

        let unit_id = value[0];
        let guid_extension_code = get_guid(&value[1..17])?;
        let num_controls = value[17];
        let num_input_pins = value[18];
        let p = num_input_pins as usize;

        if value.len() < 19 + p + 1 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!(
                    "Extension Unit descriptor too short for input pins {} < {}",
                    value.len(),
                    19 + p  + 1
                ),
            ));
        }

        let source_ids = value[19..19 + p].to_vec();
        let control_size = value[19 + p];

        if value.len() < 20 + p + 1 + control_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!(
                    "Extension Unit descriptor too short for control size {} < {}",
                    value.len(),
                    20 + p + 1 + control_size as usize
                ),
            ));
        }

        let controls = value[20 + p..20 + p + control_size as usize].to_vec();
        let extension_string_index = value[20 + p + control_size as usize];

        Ok(ExtensionUnit {
            unit_id,
            guid_extension_code,
            num_controls,
            num_input_pins,
            source_ids,
            control_size,
            controls,
            extension_index: extension_string_index,
            extension: None,
        })
    }
}

impl From<ExtensionUnit> for Vec<u8> {
    fn from(eu: ExtensionUnit) -> Self {
        let mut ret = Vec::new();
        ret.push(eu.unit_id);
        ret.extend(eu.guid_extension_code.into_bytes());
        ret.push(eu.num_controls);
        ret.push(eu.num_input_pins);
        ret.extend_from_slice(&eu.source_ids);
        ret.push(eu.control_size);
        ret.extend_from_slice(&eu.controls);
        ret.push(eu.extension_index);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct EncodingUnit {
    pub unit_id: u8,
    pub source_id: u8,
    pub encoding_index: u8,
    pub encoding: Option<String>,
    pub control_size: u8,
    pub controls: u32,
    pub controls_runtime: u32,
}

impl TryFrom<&[u8]> for EncodingUnit {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!("Encoding Unit descriptor too short {} < 4", value.len()),
            ));
        }

        let unit_id = value[0];
        let source_id = value[1];
        let encoding_string_index = value[2];
        let control_size = value[3] as usize;

        if value.len() < 4 + 2 * control_size {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                &format!(
                    "Encoding Unit descriptor too short for control size {} < {}",
                    value.len(),
                    4 + 2 * control_size
                ),
            ));
        }

        let mut controls: u32 = 0;
        for i in 0..control_size.min(3) {
            controls |= (value[4 + i] as u32) << (i * 8);
        }

        let mut controls_runtime: u32 = 0;
        for i in 0..control_size.min(3) {
            controls_runtime |= (value[4 + i + control_size] as u32) << (i * 8);
        }

        Ok(EncodingUnit {
            unit_id,
            source_id,
            encoding_index: encoding_string_index,
            encoding: None,
            control_size: value[3],
            controls,
            controls_runtime,
        })
    }
}

impl From<EncodingUnit> for Vec<u8> {
    fn from(eu: EncodingUnit) -> Self {
        let mut ret = Vec::new();
        ret.push(eu.unit_id);
        ret.push(eu.source_id);
        ret.push(eu.encoding_index);
        ret.push(eu.control_size);

        for i in 0..eu.control_size.min(3) {
            ret.push((eu.controls >> (i * 8)) as u8);
        }

        for i in 0..eu.control_size.min(3) {
            ret.push((eu.controls_runtime >> (i * 8)) as u8);
        }

        ret
    }
}
