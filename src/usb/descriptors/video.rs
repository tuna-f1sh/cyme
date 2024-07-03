//! Defines for the USB Video Class (UVC) interface descriptors
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use super::*;
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
                _ => write!(f, "{}", heck::AsShoutySnakeCase(self.to_string()))
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
                _ => write!(f, "{}", heck::AsShoutySnakeCase(self.to_string()))
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
        match self {
            UvcType::Control(c) => write!(f, "{}", c),
            UvcType::Streaming(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UvcDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub subtype: ControlSubtype,
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

        let video_control_subtype = ControlSubtype::from(value[2]);

        let string_index = match video_control_subtype {
            ControlSubtype::InputTerminal => value.get(7).copied(),
            ControlSubtype::OutputTerminal => value.get(8).copied(),
            ControlSubtype::SelectorUnit => {
                if let Some(p) = value.get(4) {
                    value.get(5 + *p as usize).copied()
                } else {
                    None
                }
            }
            ControlSubtype::ProcessingUnit => {
                if let Some(n) = value.get(7) {
                    value.get(8 + *n as usize).copied()
                } else {
                    None
                }
            }
            ControlSubtype::ExtensionUnit => {
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
            ControlSubtype::EncodingUnit => value.get(5).copied(),
            _ => None,
        };

        Ok(UvcDescriptor {
            length,
            descriptor_type: value[1],
            subtype: video_control_subtype,
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
        ret.push(vcd.subtype as u8);
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
