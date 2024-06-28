//! Defines for the USB Video Class (UVC) interface descriptors
use std::convert::TryFrom;
use serde::{Deserialize, Serialize};

use super::*;
use crate::error::{self, Error, ErrorKind};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[non_exhaustive]
pub enum UvcSubtype {
    Undefined = 0x00,
    Header = 0x01,
    InputTerminal = 0x02,
    OutputTerminal = 0x03,
    SelectorUnit = 0x04,
    ProcessingUnit = 0x05,
    ExtensionUnit = 0x06,
    EncodingUnit = 0x07,
}

impl From<u8> for UvcSubtype {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UvcSubtype::Undefined,
            0x01 => UvcSubtype::Header,
            0x02 => UvcSubtype::InputTerminal,
            0x03 => UvcSubtype::OutputTerminal,
            0x04 => UvcSubtype::SelectorUnit,
            0x05 => UvcSubtype::ProcessingUnit,
            0x06 => UvcSubtype::ExtensionUnit,
            0x07 => UvcSubtype::EncodingUnit,
            _ => UvcSubtype::Undefined,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UvcDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub subtype: UvcSubtype,
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

        let video_control_subtype = UvcSubtype::from(value[2]);

        let string_index = match video_control_subtype {
            UvcSubtype::InputTerminal => value.get(7).copied(),
            UvcSubtype::OutputTerminal => value.get(8).copied(),
            UvcSubtype::SelectorUnit => {
                if let Some(p) = value.get(4) {
                    value.get(5 + *p as usize).copied()
                } else {
                    None
                }
            }
            UvcSubtype::ProcessingUnit => {
                if let Some(n) = value.get(7) {
                    value.get(8 + *n as usize).copied()
                } else {
                    None
                }
            }
            UvcSubtype::ExtensionUnit => {
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
            UvcSubtype::EncodingUnit => value.get(5).copied(),
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
