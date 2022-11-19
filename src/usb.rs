///! Defines for USB, mainly thosed covered at [usb.org](https://www.usb.org)
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use itertools::Itertools;

use crate::types::NumericalUnit;

/// Explains how the `ClassCode` is used
#[derive(Debug)]
pub enum DescriptorUsage {
    Device,
    Interface,
    Both,
}

/// USB class code defines [ref](https://www.usb.org/defined-class-codes)
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClassCode {
    UseInterfaceDescriptor,
    Audio,
    CDCCommunications,
    HID,
    Physical,
    Image,
    Printer,
    MassStorage,
    Hub,
    CDCData,
    SmartCart,
    ContentSecurity,
    Video,
    PersonalHealthcare,
    AudioVideo,
    Billboard,
    USBTypeCBridge,
    I3CDevice,
    Diagnostic,
    WirelessController,
    Miscellaneous,
    ApplicationSpecific,
    VendorSpecific,
}

impl From<u8> for ClassCode {
    fn from(b: u8) -> ClassCode {
        match b {
            0 => ClassCode::UseInterfaceDescriptor,
            1 => ClassCode::Audio,
            2 => ClassCode::CDCCommunications,
            3 => ClassCode::HID,
            5 => ClassCode::Physical,
            6 => ClassCode::Image,
            7 => ClassCode::Printer,
            8 => ClassCode::MassStorage,
            9 => ClassCode::Hub,
            0x0a => ClassCode::CDCData,
            0x0b => ClassCode::SmartCart,
            0x0d => ClassCode::ContentSecurity,
            0x0e => ClassCode::Video,
            0x0f => ClassCode::PersonalHealthcare,
            0x10 => ClassCode::AudioVideo,
            0x11 => ClassCode::Billboard,
            0x12 => ClassCode::USBTypeCBridge,
            0x3c => ClassCode::I3CDevice,
            0xdc => ClassCode::Diagnostic,
            0xe0 => ClassCode::WirelessController,
            0xef => ClassCode::Miscellaneous,
            0xfe => ClassCode::ApplicationSpecific,
            0xff => ClassCode::VendorSpecific,
            _ => ClassCode::UseInterfaceDescriptor,
        }
    }
}

impl ClassCode {
    pub fn usage(&self) -> DescriptorUsage {
        match self {
            ClassCode::UseInterfaceDescriptor | ClassCode::Hub | ClassCode::Billboard => {
                DescriptorUsage::Device
            }
            ClassCode::CDCCommunications
            | ClassCode::Diagnostic
            | ClassCode::Miscellaneous
            | ClassCode::VendorSpecific => DescriptorUsage::Both,
            _ => DescriptorUsage::Interface,
        }
    }
}

impl From<ClassCode> for DescriptorUsage {
    fn from(c: ClassCode) -> DescriptorUsage {
        return c.usage();
    }
}

/// USB Speed is also defined in libusb but this one allows us to provide updates and custom impl
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(untagged, rename_all = "snake_case")]
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
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "super_speed_plus" => Speed::SuperSpeedPlus,
            "super_speed" => Speed::SuperSpeed,
            "high_speed" | "high_bandwidth" => Speed::HighSpeed,
            "full_speed" => Speed::FullSpeed,
            "low_speed" => Speed::LowSpeed,
            _ => Speed::Unknown,
        })
    }
}

/// Convert from byte returned from device
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
                Speed::Unknown => "unknown",
                _ => todo!("Unsupported speed"),
            }
        )
    }
}

impl From<&Speed> for NumericalUnit<f32> {
    fn from(speed: &Speed) -> NumericalUnit<f32> {
        match speed {
            Speed::SuperSpeedPlus => NumericalUnit {
                value: 20.0,
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

/// Builds a replica of sysfs path; excludes config.interface
///
/// [ref](http://gajjarpremal.blogspot.com/2015/04/sysfs-structures-for-linux-usb.html)
/// The names that begin with "usb" refer to USB controllers. More accurately, they refer to the "root hub" associated with each controller. The number is the USB bus number. In the example there is only one controller, so its bus is number 1. Hence the name "usb1".
/// 
/// "1-0:1.0" is a special case. It refers to the root hub's interface. This acts just like the interface in an actual hub an almost every respect; see below.
/// All the other entries refer to genuine USB devices and their interfaces. The devices are named by a scheme like this:
/// 
///  bus-port.port.port ...
pub fn get_port_path(bus: u8, ports: &Vec<u8>) -> String {
    format!("{:}-{}", bus, ports.into_iter().format("."))
}
