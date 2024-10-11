//! Defines for the USB Video Class (UVC) interface descriptors
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::Uuid;

use super::audio;
use super::*;
use crate::error::{self, Error, ErrorKind};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
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
#[serde(rename_all = "kebab-case")]
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
#[serde(rename_all = "kebab-case")]
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
            UvcType::Control(c) => match c {
                ControlSubtype::Header => {
                    Ok(UvcInterfaceDescriptor::Header(Header::try_from(data)?))
                }
                ControlSubtype::InputTerminal => Ok(UvcInterfaceDescriptor::InputTerminal(
                    InputTerminal::try_from(data)?,
                )),
                ControlSubtype::OutputTerminal => Ok(UvcInterfaceDescriptor::OutputTerminal(
                    OutputTerminal::try_from(data)?,
                )),
                ControlSubtype::SelectorUnit => Ok(UvcInterfaceDescriptor::SelectorUnit(
                    SelectorUnit::try_from(data)?,
                )),
                ControlSubtype::ProcessingUnit => Ok(UvcInterfaceDescriptor::ProcessingUnit(
                    ProcessingUnit::try_from(data)?,
                )),
                ControlSubtype::ExtensionUnit => Ok(UvcInterfaceDescriptor::ExtensionUnit(
                    ExtensionUnit::try_from(data)?,
                )),
                ControlSubtype::EncodingUnit => Ok(UvcInterfaceDescriptor::EncodingUnit(
                    EncodingUnit::try_from(data)?,
                )),
                ControlSubtype::Undefined => Ok(UvcInterfaceDescriptor::Undefined(data.to_vec())),
            },
            UvcType::Streaming(s) => match s {
                StreamingSubtype::InputHeader => Ok(UvcInterfaceDescriptor::InputHeader(
                    InputHeader::try_from(data)?,
                )),
                StreamingSubtype::OutputHeader => Ok(UvcInterfaceDescriptor::OutputHeader(
                    OutputHeader::try_from(data)?,
                )),
                StreamingSubtype::StillImageFrame => Ok(UvcInterfaceDescriptor::StillImageFrame(
                    StillImageFrame::try_from(data)?,
                )),
                StreamingSubtype::FrameUncompressed => Ok(
                    UvcInterfaceDescriptor::FrameUncompressed(FrameUncompressed::try_from(data)?),
                ),
                StreamingSubtype::FrameMJPEG => Ok(UvcInterfaceDescriptor::FrameMJPEG(
                    FrameMJPEG::try_from(data)?,
                )),
                StreamingSubtype::FrameFrameBased => Ok(UvcInterfaceDescriptor::FrameFrameBased(
                    FrameFrameBased::try_from(data)?,
                )),
                StreamingSubtype::FormatMJPEG => Ok(UvcInterfaceDescriptor::FormatMJPEG(
                    FormatMJPEG::try_from(data)?,
                )),
                StreamingSubtype::FormatFrameBased => Ok(UvcInterfaceDescriptor::FormatFrameBased(
                    FormatFrame::try_from(data)?,
                )),
                StreamingSubtype::FormatUncompressed => Ok(
                    UvcInterfaceDescriptor::FormatUncompressed(FormatFrame::try_from(data)?),
                ),
                StreamingSubtype::FormatStreamBased => Ok(
                    UvcInterfaceDescriptor::FormatStreamBased(FormatStreamBased::try_from(data)?),
                ),
                StreamingSubtype::FormatMPEG2TS => Ok(UvcInterfaceDescriptor::FormatMPEG2TS(
                    FormatMPEG2TS::try_from(data)?,
                )),
                StreamingSubtype::ColorFormat => Ok(UvcInterfaceDescriptor::ColorFormat(
                    ColorFormat::try_from(data)?,
                )),
                StreamingSubtype::Undefined => Ok(UvcInterfaceDescriptor::Undefined(data.to_vec())),
            },
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
                    log::warn!("Error parsing UVC descriptor: {}", e);
                    Ok(UvcInterfaceDescriptor::Invalid(data))
                }
            },
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
    pub descriptor_subtype: UvcType,
    pub interface: UvcInterfaceDescriptor,
}

/// Try from ([`GenericDescriptor`], SubClass, Protocol)
impl TryFrom<(GenericDescriptor, u8, u8)> for UvcDescriptor {
    type Error = Error;

    fn try_from((gd, subc, p): (GenericDescriptor, u8, u8)) -> error::Result<Self> {
        let length = gd.length;
        let descriptor_type = gd.descriptor_type;
        let descriptor_subtype: UvcType = (subc, gd.descriptor_subtype, p).try_into()?;
        let interface = descriptor_subtype.uvc_descriptor_from_generic(gd.to_owned(), p)?;

        Ok(UvcDescriptor {
            length,
            descriptor_type,
            descriptor_subtype,
            interface,
        })
    }
}

impl From<UvcDescriptor> for Vec<u8> {
    fn from(vcd: UvcDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(vcd.length);
        ret.push(vcd.descriptor_type);
        ret.push(u8::from(vcd.descriptor_subtype));
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
    InputHeader(InputHeader),
    OutputHeader(OutputHeader),
    StillImageFrame(StillImageFrame),
    FrameUncompressed(FrameUncompressed),
    FrameMJPEG(FrameMJPEG),
    FrameFrameBased(FrameFrameBased),
    FormatUncompressed(FormatFrame),
    FormatFrameBased(FormatFrame),
    FormatStreamBased(FormatStreamBased),
    FormatMJPEG(FormatMJPEG),
    FormatMPEG2TS(FormatMPEG2TS),
    ColorFormat(ColorFormat),
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
            UvcInterfaceDescriptor::InputHeader(ih) => ih.into(),
            UvcInterfaceDescriptor::OutputHeader(oh) => oh.into(),
            UvcInterfaceDescriptor::StillImageFrame(sif) => sif.into(),
            UvcInterfaceDescriptor::FrameFrameBased(ff) => ff.into(),
            UvcInterfaceDescriptor::FrameUncompressed(fu)
            | UvcInterfaceDescriptor::FrameMJPEG(fu) => fu.into(),
            UvcInterfaceDescriptor::FormatUncompressed(fmt)
            | UvcInterfaceDescriptor::FormatFrameBased(fmt) => fmt.into(),
            UvcInterfaceDescriptor::FormatStreamBased(fsb) => fsb.into(),
            UvcInterfaceDescriptor::FormatMJPEG(fmt) => fmt.into(),
            UvcInterfaceDescriptor::FormatMPEG2TS(fmts) => fmts.into(),
            UvcInterfaceDescriptor::ColorFormat(cf) => cf.into(),
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
            return Err(Error::new_descriptor_len("Video Control", 10, value.len()));
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
            return Err(Error::new_descriptor_len("TerminalExtra", 8, value.len()));
        }

        let objective_focal_length_min = u16::from_le_bytes([value[0], value[1]]);
        let objective_focal_length_max = u16::from_le_bytes([value[2], value[3]]);
        let ocular_focal_length = u16::from_le_bytes([value[4], value[5]]);
        let control_size = value[6];

        if value.len() < 7 + control_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                &format!(
                    "Terminal Extra descriptor too short for control size {} < {}",
                    value.len(),
                    7 + control_size
                ),
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
            return Err(Error::new_descriptor_len("InputTerminal", 5, value.len()));
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
        if value.len() < 7 {
            return Err(Error::new_descriptor_len("ProcessingUnit", 7, value.len()));
        }

        let unit_id = value[0];
        let source_id = value[1];
        let max_multiplier = u16::from_le_bytes([value[2], value[3]]);
        let control_size = value[4];

        // 5 + 2 for bytes after
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
    pub guid_extension_code: Uuid,
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
            return Err(Error::new_descriptor_len("ExtensionUnit", 21, value.len()));
        }

        let unit_id = value[0];
        let guid_extension_code = Uuid::from_slice_le(&value[1..17]).map_err(|e| {
            Error::new(
                ErrorKind::InvalidDescriptor,
                &format!("Invalid GUID Extension Code: {}", e),
            )
        })?;
        let num_controls = value[17];
        let num_input_pins = value[18];
        let p = num_input_pins as usize;

        if value.len() < 19 + p + 1 {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                &format!(
                    "Extension Unit descriptor too short for input pins {} < {}",
                    value.len(),
                    19 + p + 1
                ),
            ));
        }

        let source_ids = value[19..19 + p].to_vec();
        let control_size = value[19 + p];

        if value.len() < 20 + p + 1 + control_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
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
        ret.extend(eu.guid_extension_code.to_bytes_le());
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
            return Err(Error::new_descriptor_len("EncodingUnit", 4, value.len()));
        }

        let unit_id = value[0];
        let source_id = value[1];
        let encoding_string_index = value[2];
        let control_size = value[3] as usize;

        if value.len() < 3 + 2 * control_size {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                &format!(
                    "Encoding Unit descriptor too short for control size {} < {}",
                    value.len(),
                    3 + 2 * control_size
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
        let mut ret = vec![eu.unit_id, eu.source_id, eu.encoding_index, eu.control_size];

        for i in 0..eu.control_size.min(3) {
            ret.push((eu.controls >> (i * 8)) as u8);
        }

        for i in 0..eu.control_size.min(3) {
            ret.push((eu.controls_runtime >> (i * 8)) as u8);
        }

        ret
    }
}

/* Streaming Interface Descriptors */

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct InputHeader {
    pub num_formats: u8,
    pub total_length: u16,
    pub endpoint_address: EndpointAddress,
    pub info: u8,
    pub terminal_link: u8,
    pub still_capture_method: u8,
    pub trigger_support: u8,
    pub trigger_usage: u8,
    pub control_size: u8,
    pub controls: Vec<u8>,
}

impl TryFrom<&[u8]> for InputHeader {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new_descriptor_len("InputHeader", 10, value.len()));
        }

        let num_formats = value[0];
        let total_length = u16::from_le_bytes([value[1], value[2]]);
        let endpoint_address = EndpointAddress::from(value[3]);
        let info = value[4];
        let terminal_link = value[5];
        let still_capture_method = value[6];
        let trigger_support = value[7];
        let trigger_usage = value[8];
        let control_size = value[9];

        if value.len() < 10 + num_formats as usize * control_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Input Header descriptor too short for reported formats",
            ));
        }

        let controls = value[10..].to_vec();

        Ok(InputHeader {
            num_formats,
            total_length,
            endpoint_address,
            info,
            terminal_link,
            still_capture_method,
            trigger_support,
            trigger_usage,
            control_size,
            controls,
        })
    }
}

impl From<InputHeader> for Vec<u8> {
    fn from(ih: InputHeader) -> Self {
        let mut ret = Vec::new();
        ret.push(ih.num_formats);
        ret.extend_from_slice(&ih.total_length.to_le_bytes());
        ret.push(ih.endpoint_address.into());
        ret.push(ih.info);
        ret.push(ih.terminal_link);
        ret.push(ih.still_capture_method);
        ret.push(ih.trigger_support);
        ret.push(ih.trigger_usage);
        ret.push(ih.control_size);
        ret.extend(ih.controls);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct OutputHeader {
    pub num_formats: u8,
    pub total_length: u16,
    pub endpoint_address: EndpointAddress,
    pub terminal_link: u8,
    pub control_size: u8,
    pub controls: Vec<u8>,
}

impl TryFrom<&[u8]> for OutputHeader {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len("OutputHeader", 6, value.len()));
        }

        let num_formats = value[0];
        let total_length = u16::from_le_bytes([value[1], value[2]]);
        let endpoint_address = EndpointAddress::from(value[3]);
        let terminal_link = value[4];
        let control_size = value[5];

        if value.len() < 6 + num_formats as usize * control_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                &format!(
                    "Output Header descriptor too short for formats {} < {}",
                    value.len(),
                    7 + num_formats as usize * control_size as usize
                ),
            ));
        }

        let controls = value[6..].to_vec();

        Ok(OutputHeader {
            num_formats,
            total_length,
            endpoint_address,
            terminal_link,
            control_size,
            controls,
        })
    }
}

impl From<OutputHeader> for Vec<u8> {
    fn from(oh: OutputHeader) -> Self {
        let mut ret = Vec::new();
        ret.push(oh.num_formats);
        ret.extend_from_slice(&oh.total_length.to_le_bytes());
        ret.push(oh.endpoint_address.into());
        ret.push(oh.terminal_link);
        ret.push(oh.control_size);
        ret.extend(oh.controls);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct StillImageFrame {
    pub endpoint_address: EndpointAddress,
    pub num_image_size_patterns: u8,
    pub image_size_patterns: Vec<(u16, u16)>,
    pub num_compression_patterns: u8,
    pub compression_patterns: Vec<u8>,
}

impl TryFrom<&[u8]> for StillImageFrame {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new_descriptor_len("StillImageFrame", 3, value.len()));
        }

        let endpoint_address = EndpointAddress::from(value[0]);
        let num_image_size_patterns = value[1];
        let mut image_size_patterns = Vec::new();
        let mut offset = 2;

        if offset + num_image_size_patterns as usize * 4 > value.len() {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Still Image Frame descriptor too short for image size patterns",
            ));
        }

        for b in value[offset..].chunks_exact(4) {
            let width = u16::from_le_bytes([b[0], b[1]]);
            let height = u16::from_le_bytes([b[2], b[3]]);
            image_size_patterns.push((width, height));
            offset += 4;
        }

        let num_compression_patterns = value[offset];
        offset += 1;

        if offset + num_compression_patterns as usize > value.len() {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Still Image Frame descriptor too short for compression patterns",
            ));
        }

        let compression_patterns =
            value[offset..offset + num_compression_patterns as usize].to_vec();

        Ok(StillImageFrame {
            endpoint_address,
            num_image_size_patterns,
            image_size_patterns,
            num_compression_patterns,
            compression_patterns,
        })
    }
}

impl From<StillImageFrame> for Vec<u8> {
    fn from(sif: StillImageFrame) -> Self {
        let mut ret = Vec::new();
        ret.push(sif.endpoint_address.into());
        ret.push(sif.num_image_size_patterns);
        for (width, height) in sif.image_size_patterns {
            ret.extend_from_slice(&width.to_le_bytes());
            ret.extend_from_slice(&height.to_le_bytes());
        }
        ret.push(sif.num_compression_patterns);
        ret.extend(sif.compression_patterns);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ColorFormat {
    pub color_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
}

impl TryFrom<&[u8]> for ColorFormat {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new_descriptor_len("ColorFormat", 3, value.len()));
        }

        let color_primaries = value[0];
        let transfer_characteristics = value[1];
        let matrix_coefficients = value[2];

        Ok(ColorFormat {
            color_primaries,
            transfer_characteristics,
            matrix_coefficients,
        })
    }
}

impl From<ColorFormat> for Vec<u8> {
    fn from(cf: ColorFormat) -> Self {
        vec![
            cf.color_primaries,
            cf.transfer_characteristics,
            cf.matrix_coefficients,
        ]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatStreamBased {
    pub format_index: u8,
    pub guid_format: Uuid,
    pub packet_length: u8,
}

impl TryFrom<&[u8]> for FormatStreamBased {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 18 {
            return Err(Error::new_descriptor_len(
                "FormatStreamBased",
                18,
                value.len(),
            ));
        }

        let format_index = value[0];
        let guid_format = Uuid::from_slice_le(&value[1..17]).map_err(|e| {
            Error::new(
                ErrorKind::InvalidDescriptor,
                &format!("Invalid GUID Format: {}", e),
            )
        })?;
        let packet_length = value[17];

        Ok(FormatStreamBased {
            format_index,
            guid_format,
            packet_length,
        })
    }
}

impl From<FormatStreamBased> for Vec<u8> {
    fn from(fsb: FormatStreamBased) -> Self {
        let mut ret = Vec::new();
        ret.push(fsb.format_index);
        ret.extend(fsb.guid_format.to_bytes_le());
        ret.push(fsb.packet_length);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatMPEG2TS {
    pub format_index: u8,
    pub data_offset: u8,
    pub packet_length: u8,
    pub stride_length: u8,
    pub guid_stride_format: Option<Uuid>,
}

impl TryFrom<&[u8]> for FormatMPEG2TS {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("FormatMPEG2TS", 4, value.len()));
        }

        let format_index = value[0];
        let data_offset = value[1];
        let packet_length = value[2];
        let stride_length = value[3];

        let guid_stride_format = if value.len() < 20 {
            None
        } else {
            Uuid::from_slice_le(&value[4..20]).ok()
        };

        Ok(FormatMPEG2TS {
            format_index,
            data_offset,
            packet_length,
            stride_length,
            guid_stride_format,
        })
    }
}

impl From<FormatMPEG2TS> for Vec<u8> {
    fn from(fmts: FormatMPEG2TS) -> Self {
        let mut ret = vec![
            fmts.format_index,
            fmts.data_offset,
            fmts.packet_length,
            fmts.stride_length,
        ];
        if let Some(guid) = fmts.guid_stride_format {
            ret.extend(guid.to_bytes_le());
        }
        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatMJPEG {
    pub format_index: u8,
    pub num_frame_descriptors: u8,
    pub flags: u8,
    pub default_frame_index: u8,
    pub aspect_ratio_x: u8,
    pub aspect_ratio_y: u8,
    pub interlace_flags: u8,
    pub copy_protect: u8,
}

impl TryFrom<&[u8]> for FormatMJPEG {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new_descriptor_len("FormatMJPEG", 8, value.len()));
        }

        let format_index = value[0];
        let num_frame_descriptors = value[1];
        let flags = value[2];
        let default_frame_index = value[3];
        let aspect_ratio_x = value[4];
        let aspect_ratio_y = value[5];
        let interlace_flags = value[6];
        let copy_protect = value[7];

        Ok(FormatMJPEG {
            format_index,
            num_frame_descriptors,
            flags,
            default_frame_index,
            aspect_ratio_x,
            aspect_ratio_y,
            interlace_flags,
            copy_protect,
        })
    }
}

impl From<FormatMJPEG> for Vec<u8> {
    fn from(fmjpeg: FormatMJPEG) -> Self {
        vec![
            fmjpeg.format_index,
            fmjpeg.num_frame_descriptors,
            fmjpeg.flags,
            fmjpeg.default_frame_index,
            fmjpeg.aspect_ratio_x,
            fmjpeg.aspect_ratio_y,
            fmjpeg.interlace_flags,
            fmjpeg.copy_protect,
        ]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatFrame {
    pub format_index: u8,
    pub num_frame_descriptors: u8,
    pub guid_format: Uuid,
    pub bits_per_pixel: u8,
    pub default_frame_index: u8,
    pub aspect_ratio_x: u8,
    pub aspect_ratio_y: u8,
    pub interlace_flags: u8,
    pub copy_protect: u8,
    pub variable_size: Option<u8>,
}

impl TryFrom<&[u8]> for FormatFrame {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 24 {
            return Err(Error::new_descriptor_len("FormatFrame", 24, value.len()));
        }

        let format_index = value[0];
        let num_frame_descriptors = value[1];
        let guid_format = Uuid::from_slice_le(&value[2..18]).map_err(|e| {
            Error::new(
                ErrorKind::InvalidDescriptor,
                &format!("Invalid GUID Format: {}", e),
            )
        })?;
        let bits_per_pixel = value[18];
        let default_frame_index = value[19];
        let aspect_ratio_x = value[20];
        let aspect_ratio_y = value[21];
        let interlace_flags = value[22];
        let copy_protect = value[23];
        // only present on frame based
        let variable_size = value.get(24).copied();

        Ok(FormatFrame {
            format_index,
            num_frame_descriptors,
            guid_format,
            bits_per_pixel,
            default_frame_index,
            aspect_ratio_x,
            aspect_ratio_y,
            interlace_flags,
            copy_protect,
            variable_size,
        })
    }
}

impl From<FormatFrame> for Vec<u8> {
    fn from(fufb: FormatFrame) -> Self {
        let mut ret = Vec::new();
        ret.push(fufb.format_index);
        ret.push(fufb.num_frame_descriptors);
        ret.extend_from_slice(&fufb.guid_format.to_bytes_le());
        ret.push(fufb.bits_per_pixel);
        ret.push(fufb.default_frame_index);
        ret.push(fufb.aspect_ratio_x);
        ret.push(fufb.aspect_ratio_y);
        ret.push(fufb.interlace_flags);
        ret.push(fufb.copy_protect);
        if let Some(variable_size) = fufb.variable_size {
            ret.push(variable_size);
        }
        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FrameCommon {
    pub frame_index: u8,
    pub capabilities: u8,
    pub width: u16,
    pub height: u16,
    pub min_bit_rate: u32,
    pub max_bit_rate: u32,
}

impl TryFrom<&[u8]> for FrameCommon {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 14 {
            return Err(Error::new_descriptor_len("FrameCommon", 14, value.len()));
        }

        let frame_index = value[0];
        let capabilities = value[1];
        let width = u16::from_le_bytes([value[2], value[3]]);
        let height = u16::from_le_bytes([value[4], value[5]]);
        let min_bit_rate = u32::from_le_bytes([value[6], value[7], value[8], value[9]]);
        let max_bit_rate = u32::from_le_bytes([value[10], value[11], value[12], value[13]]);

        Ok(FrameCommon {
            frame_index,
            capabilities,
            width,
            height,
            min_bit_rate,
            max_bit_rate,
        })
    }
}

impl From<FrameCommon> for Vec<u8> {
    fn from(fc: FrameCommon) -> Self {
        let mut ret = Vec::new();
        ret.push(fc.frame_index);
        ret.push(fc.capabilities);
        ret.extend_from_slice(&fc.width.to_le_bytes());
        ret.extend_from_slice(&fc.height.to_le_bytes());
        ret.extend_from_slice(&fc.min_bit_rate.to_le_bytes());
        ret.extend_from_slice(&fc.max_bit_rate.to_le_bytes());
        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FrameUncompressed {
    pub common: FrameCommon,
    pub max_video_frame_buffer_size: u32,
    pub default_frame_interval: u32,
    pub frame_interval_type: u8,
    pub frame_intervals: Vec<u32>,
}

impl TryFrom<&[u8]> for FrameUncompressed {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        let common: FrameCommon = FrameCommon::try_from(value)?;

        if value.len() < 24 {
            return Err(Error::new_descriptor_len(
                "FrameUncompressed",
                24,
                value.len(),
            ));
        }

        let max_video_frame_buffer_size =
            u32::from_le_bytes([value[14], value[15], value[16], value[17]]);
        let default_frame_interval =
            u32::from_le_bytes([value[18], value[19], value[20], value[21]]);
        let frame_interval_type = value[22];

        let frame_intervals = if frame_interval_type == 0 && value.len() >= 35 {
            vec![
                u32::from_le_bytes([value[23], value[24], value[25], value[26]]),
                u32::from_le_bytes([value[27], value[28], value[29], value[30]]),
                u32::from_le_bytes([value[31], value[32], value[33], value[34]]),
            ]
        } else {
            if value.len() < 23 + frame_interval_type as usize * 4 {
                return Err(Error::new(
                    ErrorKind::InvalidDescriptor,
                    &format!(
                        "FrameUncompressed descriptor too short for frame intervals {} < {}",
                        value.len(),
                        22 + frame_interval_type as usize * 4
                    ),
                ));
            }
            value[23..]
                .chunks_exact(4)
                .take(frame_interval_type as usize)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect()
        };

        Ok(FrameUncompressed {
            common,
            default_frame_interval,
            frame_interval_type,
            max_video_frame_buffer_size,
            frame_intervals,
        })
    }
}

impl From<FrameUncompressed> for Vec<u8> {
    fn from(fu: FrameUncompressed) -> Self {
        let mut ret = Vec::from(fu.common);
        ret.extend_from_slice(&fu.max_video_frame_buffer_size.to_le_bytes());
        ret.extend_from_slice(&fu.default_frame_interval.to_le_bytes());
        ret.push(fu.frame_interval_type);
        for interval in fu.frame_intervals {
            ret.extend_from_slice(&interval.to_le_bytes());
        }
        ret
    }
}

#[allow(missing_docs)]
pub type FrameMJPEG = FrameUncompressed;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FrameFrameBased {
    pub common: FrameCommon,
    pub default_frame_interval: u32,
    pub frame_interval_type: u8,
    pub bytes_per_line: u32,
    pub frame_intervals: Vec<u32>,
}

impl TryFrom<&[u8]> for FrameFrameBased {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        let common: FrameCommon = FrameCommon::try_from(value)?;

        if value.len() < 24 {
            return Err(Error::new_descriptor_len(
                "FrameFrameBased",
                24,
                value.len(),
            ));
        }

        let default_frame_interval =
            u32::from_le_bytes([value[14], value[15], value[16], value[17]]);
        let frame_interval_type = value[18];
        let bytes_per_line = u32::from_le_bytes([value[19], value[20], value[21], value[22]]);

        let frame_intervals = if frame_interval_type == 0 && value.len() >= 35 {
            vec![
                u32::from_le_bytes([value[23], value[24], value[25], value[26]]),
                u32::from_le_bytes([value[27], value[28], value[29], value[30]]),
                u32::from_le_bytes([value[31], value[32], value[33], value[34]]),
            ]
        } else {
            if value.len() < 23 + frame_interval_type as usize * 4 {
                return Err(Error::new(
                    ErrorKind::InvalidDescriptor,
                    &format!(
                        "FrameFrameBased descriptor too short for frame intervals {} < {}",
                        value.len(),
                        23 + frame_interval_type as usize * 4
                    ),
                ));
            }
            value[23..]
                .chunks_exact(4)
                .take(frame_interval_type as usize)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect()
        };

        Ok(FrameFrameBased {
            common,
            default_frame_interval,
            frame_interval_type,
            bytes_per_line,
            frame_intervals,
        })
    }
}

impl From<FrameFrameBased> for Vec<u8> {
    fn from(ffb: FrameFrameBased) -> Self {
        let mut ret = Vec::from(ffb.common);
        ret.extend_from_slice(&ffb.default_frame_interval.to_le_bytes());
        ret.push(ffb.frame_interval_type);
        ret.extend_from_slice(&ffb.bytes_per_line.to_le_bytes());
        for interval in ffb.frame_intervals {
            ret.extend_from_slice(&interval.to_le_bytes());
        }

        ret
    }
}
