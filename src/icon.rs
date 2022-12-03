//! Icons and themeing of cyme output
use std::str::FromStr;
use std::collections::HashMap;
use std::fmt;
use std::io;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::system_profiler::{USBBus, USBDevice};
use crate::usb::{ClassCode, Direction};

/// Icon type enum is used as key in `HashMaps`
/// TODO FromStr and ToStr serialize/deserialize so that can merge with user defined
#[derive(Debug, Clone, Hash, PartialEq, Eq, SerializeDisplay, DeserializeFromStr)]
pub enum Icon {
    /// Vendor ID lookup
    Vid(u16),
    /// Vendor ID and Product ID exact match
    VidPid((u16, u16)),
    /// Use to mask on msb of product ID
    VidPidMsb((u16, u8)),
    /// Class classifier icon
    Classifier(ClassCode),
    /// Class classifier lookup with SubClass and Protocol
    ClassifierSubProtocol((ClassCode, u8, u8)),
    /// Icon for unknown vendors
    UnknownVendor,
    /// Icon for undefined classifier
    UndefinedClassifier,
    /// Icon to use when tree is being printed within an extending branch
    TreeEdge,
    /// Icon to use for non-last list item
    TreeLine,
    /// Icon to use at last item in list
    TreeCorner,
    /// Blanking icon for inset without edge
    TreeBlank,
    /// Icon at prepended before printing `USBBus`
    TreeBusStart,
    /// Icon printed at end of tree before printing `USBDevice`
    TreeDeviceTerminator,
    /// Icon printed at end of tree before printing configuration
    TreeConfigurationTerminiator,
    /// Icon printed at end of tree before printing interface
    TreeInterfaceTerminiator,
    /// Icon for endpoint direction
    Endpoint(Direction),
}

impl FromStr for Icon {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value_split: Vec<&str> = s.split("#").collect();
        let enum_name = value_split[0];

        // no value in string, match kebab-case
        if value_split.len() == 1 {
            match enum_name {
                "unknown-vendor" => Ok(Icon::UnknownVendor),
                "undefined-classifier" => Ok(Icon::UndefinedClassifier),
                "tree-edge" => Ok(Icon::TreeEdge),
                "tree-line" => Ok(Icon::TreeLine),
                "tree-corner" => Ok(Icon::TreeCorner),
                "tree-bus-start" => Ok(Icon::TreeBusStart),
                "tree-device-terminator" => Ok(Icon::TreeDeviceTerminator),
                "tree-configuration-terminator" => Ok(Icon::TreeConfigurationTerminiator),
                "tree-interface-terminator" => Ok(Icon::TreeInterfaceTerminiator),
                "endpoint_in" => Ok(Icon::Endpoint(Direction::In)),
                "endpoint_out" => Ok(Icon::Endpoint(Direction::Out)),
                _ => Err(io::Error::new(io::ErrorKind::Other, "Invalid Icon enum name or valued enum without value"))
            }
        // enum contains value
        } else {
            let (parse_ints, errors): (Vec<_>, Vec<_>) = value_split[1..].into_iter()
                .map(|vs| vs.parse::<u16>())
                .partition(Result::is_ok);
            let numbers: Vec<_> = parse_ints.into_iter().map(Result::unwrap).collect();

            if !errors.is_empty() {
                return Err(io::Error::new(io::ErrorKind::Other, "Invalid value in enum string after #"));
            }

            match value_split[0] {
                "vid" => match numbers.get(0) {
                    Some(i) => Ok(Icon::Vid(*i)),
                    None => Err(io::Error::new(io::ErrorKind::Other, "No value for enum after $"))
                },
                "vid-pid" => match numbers.get(0..1) {
                    Some(slice) => Ok(Icon::VidPid((slice[0], slice[1]))),
                    None => Err(io::Error::new(io::ErrorKind::Other, "No value for enum after $"))
                },
                "vid-pid-msb" => match numbers.get(0..1) {
                    Some(slice) => Ok(Icon::VidPidMsb((slice[0], slice[1] as u8))),
                    None => Err(io::Error::new(io::ErrorKind::Other, "No value for enum after $"))
                },
                "classifier" => match numbers.get(0) {
                    Some(i) => Ok(Icon::Classifier(ClassCode::from(*i as u8))),
                    None => Err(io::Error::new(io::ErrorKind::Other, "No value for enum after $"))
                },
                "classifier-sub-protocol" => match numbers.get(0..2) {
                    Some(slice) => Ok(Icon::ClassifierSubProtocol((ClassCode::from(slice[0] as u8), slice[1] as u8, slice[2] as u8))),
                    None => Err(io::Error::new(io::ErrorKind::Other, "No value for enum after $"))
                },
                _ => Err(io::Error::new(io::ErrorKind::Other, "Invalid Icon enum value holder"))
            }
        }
    }
}

impl fmt::Display for Icon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Icon::Vid(v) => write!(f, "vid#{:04x}", v),
            Icon::VidPid((v, p)) => write!(f, "vid-pid#{:04x}:{:04x}", v, p),
            Icon::VidPidMsb((v, p)) => write!(f, "vid-pid-msb#{:04x}:{:02x}", v, p),
            Icon::Classifier(c) => write!(f, "classifier#{:02x}", c.to_owned() as u8),
            Icon::ClassifierSubProtocol(c) => write!(f, "classifier-sub-protocol#{}:{}:{}", c.0.to_owned() as u8, c.1, c.2),
            Icon::Endpoint(Direction::In) => write!(f, "endpoint_in"),
            Icon::Endpoint(Direction::Out) => write!(f, "endpoint_out"),
            _ => {
                let dbg_str = format!("{:?}", self);
                write!(f, "{}", heck::AsKebabCase(dbg_str))
            }
        }
    }
}

/// Allows user supplied icons to replace or add to `DEFAULT_ICONS` and `DEFAULT_TREE`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct IconTheme {
    /// Will merge with `DEFAULT_ICONS` for user supplied
    pub icons: Option<HashMap<Icon, String>>,
    /// Will merge with `DEFAULT_TREE` for user supplied tree drawing
    pub tree: Option<HashMap<Icon, String>>,
}

/// Make default icons lazy_static and outside of IconTheme keeps them static but can be overridden user HashMap<Icon, String> at runtime
impl Default for IconTheme {
    fn default() -> Self {
        IconTheme {
            icons: None,
            tree: None,
        }
    }
}

lazy_static! {
    /// Default icons to draw tree can be overridden by user icons with IconTheme `tree`
    static ref DEFAULT_TREE: HashMap<Icon, &'static str> = {
        HashMap::from([
            (Icon::TreeEdge, "\u{251c}\u{2500}\u{2500}".into()), // "├──"
            (Icon::TreeLine, "\u{2502}  ".into()), // "│  "
            (Icon::TreeCorner, "\u{2514}\u{2500}\u{2500}".into()), // "└──"
            (Icon::TreeBlank, "   ".into()), // should be same char width as above
            (Icon::TreeBusStart, "\u{25CF}".into()), // "●"
            (Icon::TreeDeviceTerminator, "\u{25CB}".into()), // "○"
            (Icon::TreeConfigurationTerminiator, "\u{2022}".into()), // "•"
            (Icon::TreeInterfaceTerminiator, "\u{25E6}".into()), // "◦"
            // (Icon::Endpoint(Direction::In), "\u{2192}".into()), // →
            // (Icon::Endpoint(Direction::Out), "\u{2190}".into()), // ←
            (Icon::Endpoint(Direction::In), ">".into()), // →
            (Icon::Endpoint(Direction::Out), "<".into()), // ←
        ])
    };

    /// Ascii chars used by lsusb compatible mode or no utf-8
    static ref ASCII_TREE: HashMap<Icon, &'static str> = {
        HashMap::from([
            (Icon::TreeEdge, "|__".into()), // same as corner
            (Icon::TreeLine, "|  ".into()), // no outside line but inset so starts under parent device
            (Icon::TreeCorner, "|__".into()),
            (Icon::TreeBlank, "   ".into()), // inset like line
            (Icon::TreeBusStart, "/: ".into()),
            (Icon::TreeDeviceTerminator, "O".into()), // null
            (Icon::TreeConfigurationTerminiator, "o".into()), // null
            (Icon::TreeInterfaceTerminiator, ".".into()), // null
            (Icon::Endpoint(Direction::In), ">".into()), //
            (Icon::Endpoint(Direction::Out), "<".into()), //
        ])
    };

    /// Default icon lookup can be overridden by user icons with IconTheme `icons`
    ///
    /// Should probably keep fairly short but I've added things I use like debuggers, mcus as examples
    pub static ref DEFAULT_ICONS: HashMap<Icon, &'static str> = {
        HashMap::from([
            (Icon::UnknownVendor, "\u{f287}".into()), // usb plug default 
            (Icon::Vid(0x05ac), "\u{f179}".into()), // apple 
            (Icon::Vid(0x8086), "\u{f179}".into()), // apple bus 
            (Icon::Vid(0x045e), "\u{f871}".into()), // microsoft 
            (Icon::Vid(0x18d1), "\u{f1a0}".into()), // google 
            (Icon::Vid(0x1D6B), "\u{f17c}".into()), // linux foundation 
            (Icon::Vid(0x1d50), "\u{e771}".into()), // open source VID 
            (Icon::Vid(0x1915), "\u{f5a2}".into()), // specialized 
            (Icon::Vid(0x0483), "\u{f5a2}".into()), // specialized 
            (Icon::Vid(0x046d), "\u{f87c}".into()), // logitech 
            (Icon::Vid(0x091e), "\u{e2a6}".into()), // garmin 
            (Icon::VidPid((0x1d50, 0x6018)), "\u{f188}".into()), // black magic probe 
            (Icon::Vid(0x1366), "\u{f188}".into()), // segger 
            (Icon::Vid(0xf1a0), "\u{f188}".into()), // arm 
            (Icon::VidPidMsb((0x0483, 0x37)), "\u{f188}".into()), // st-link 
            (Icon::VidPid((0x0483, 0xdf11)), "\u{f019}".into()), // STM DFU 
            (Icon::VidPid((0x1d50, 0x6017)), "\u{f188}".into()), // black magic probe DFU 
            (Icon::ClassifierSubProtocol((ClassCode::ApplicationSpecificInterface, 0x01, 0x01)), "\u{f188}".into()), // DFU 
            (Icon::ClassifierSubProtocol((ClassCode::WirelessController, 0x01, 0x01)), "\u{f188}".into()), // bluetooth DFU 
            (Icon::Vid(0x2341), "\u{f2db}".into()), // arduino 
            (Icon::Vid(0x239A), "\u{f2db}".into()), // adafruit 
            (Icon::Vid(0x2e8a), "\u{f315}".into()), // raspberry pi foundation 
            (Icon::Vid(0x0483), "\u{f2db}".into()), // stm 
            (Icon::Vid(0x1915), "\u{f2db}".into()), // nordic 
            (Icon::Vid(0x1fc9), "\u{f2db}".into()), // nxp 
            (Icon::Vid(0x1050), "\u{f805}".into()), // yubikey 
            (Icon::VidPid((0x18D1, 0x2D05)), "\u{e70e}".into()), // android dev 
            (Icon::VidPid((0x18D1, 0xd00d)), "\u{e70e}".into()), // android 
            (Icon::VidPid((0x1d50, 0x606f)), "\u{f5e6}".into()), // candlelight_fw gs_can 
            (Icon::VidPidMsb((0x043e, 0x9a)), "\u{f878}".into()), // lg monitor 
            (Icon::VidPid((0x0781, 0xf7c9)), "\u{f878}".into()), // sandisk external disk 
            (Icon::Classifier(ClassCode::Audio), "\u{f001}".into()), // 
            (Icon::Classifier(ClassCode::Image), "\u{f03e}".into()), // 
            (Icon::Classifier(ClassCode::Video), "\u{f03d}".into()), // 
            (Icon::Classifier(ClassCode::Printer), "\u{fc05}".into()), // ﰅ
            // (Icon::Classifier(ClassCode::MassStorage), "\u{fc05}".into()),
            (Icon::Classifier(ClassCode::Hub), "\u{f126}".into()), // 
            (Icon::Classifier(ClassCode::ContentSecurity), "\u{f805}".into()), // 
            (Icon::Classifier(ClassCode::SmartCart), "\u{f805}".into()), // 
            (Icon::Classifier(ClassCode::PersonalHealthcare), "\u{fbeb}".into()), // ﯭ
            (Icon::Classifier(ClassCode::Physical), "\u{f5cd}".into()), // 
            (Icon::Classifier(ClassCode::AudioVideo), "\u{fd3f}".into()), // ﴿
            (Icon::Classifier(ClassCode::Billboard), "\u{f05a}".into()), // 
            (Icon::Classifier(ClassCode::I3CDevice), "\u{f493}".into()), // 
            (Icon::Classifier(ClassCode::Diagnostic), "\u{f489}".into()), // 
            (Icon::Classifier(ClassCode::WirelessController), "\u{f1eb}".into()), // 
            (Icon::Classifier(ClassCode::Miscellaneous), "\u{f074}".into()), // 
            (Icon::Classifier(ClassCode::CDCCommunications), "\u{e795}".into()), // serial 
            (Icon::Classifier(ClassCode::CDCData), "\u{e795}".into()), // serial 
            (Icon::Classifier(ClassCode::HID), "\u{f80b}".into()), // 
            (Icon::UndefinedClassifier, "\u{2636}".into()), //☶
        ])
    };
}

impl IconTheme {
    /// New theme with defaults
    pub fn new() -> Self {
        Default::default()
    }

    /// Get tree building icon checks `Self` for user `tree` and tries to find `icon` there, otherwise uses `DEFAULT_TREE`
    pub fn get_tree_icon(&self, icon: &Icon) -> String {
        // unwrap on DEFAULT_TREE is ok here since should panic if missing from static list
        if let Some(user_tree) = self.tree.as_ref() {
            user_tree
                .get(icon)
                .unwrap_or(&DEFAULT_TREE.get(icon).unwrap().to_string())
                .to_string()
                .to_owned()
        } else {
            get_default_tree_icon(&icon)
        }
    }

    /// Drill through `DEFAULT_ICONS` first looking for `VidPid` -> `VidPidMsb` -> `Vid` -> `UnknownVendor` -> ""
    pub fn get_default_vidpid_icon(vid: u16, pid: u16) -> String {
        // try vid pid first
        DEFAULT_ICONS
            .get(&Icon::VidPid((vid, pid)))
            .unwrap_or(
                DEFAULT_ICONS
                    .get(&Icon::VidPidMsb((vid, (pid >> 8) as u8)))
                    .unwrap_or(
                        DEFAULT_ICONS
                            .get(&Icon::Vid(vid))
                            .unwrap_or(DEFAULT_ICONS.get(&Icon::UnknownVendor).unwrap_or(&"")),
                    ),
            )
            .to_string()
    }

    /// Drill through `Self` `icons` if present first looking for `VidPid` -> `VidPidMsb` -> `Vid` -> `UnknownVendor` -> `get_default_vidpid_icon`
    pub fn get_vidpid_icon(&self, vid: u16, pid: u16) -> String {
        if let Some(user_icons) = self.icons.as_ref() {
            // try vid pid first
            user_icons
                .get(&Icon::VidPid((vid, pid)))
                .unwrap_or(
                    user_icons
                        .get(&Icon::VidPidMsb((vid, (pid >> 8) as u8)))
                        .unwrap_or(
                            user_icons.get(&Icon::Vid(vid)).unwrap_or(
                                user_icons
                                    .get(&Icon::UnknownVendor)
                                    .unwrap_or(&IconTheme::get_default_vidpid_icon(vid, pid)),
                            ),
                        ),
                )
                .to_owned()
        } else {
            IconTheme::get_default_vidpid_icon(vid, pid)
        }
    }

    /// Get icon for device from static default lookup
    pub fn get_default_device_icon(d: &USBDevice) -> String {
        if let (Some(vid), Some(pid)) = (d.vendor_id, d.product_id) {
            IconTheme::get_default_vidpid_icon(vid, pid)
        } else {
            String::new()
        }
    }

    /// Get icon for USBDevice `d` by checking `Self` using Vendor ID and Product ID
    pub fn get_device_icon(&self, d: &USBDevice) -> String {
        if let (Some(vid), Some(pid)) = (d.vendor_id, d.product_id) {
            self.get_vidpid_icon(vid, pid)
        } else {
            String::new()
        }
    }

    /// Get icon for USBBus `d` by checking `Self` using PCI Vendor and PCI Device
    pub fn get_bus_icon(&self, d: &USBBus) -> String {
        if let (Some(vid), Some(pid)) = (d.pci_vendor, d.pci_device) {
            self.get_vidpid_icon(vid, pid)
        } else {
            String::new()
        }
    }

    /// Drill through `DEFAULT_ICONS` first looking for `ClassifierSubProtocol` -> `Classifier` -> `UndefinedClassifier` -> ""
    pub fn get_default_classifier_icon(class: &ClassCode, sub: u8, protocol: u8) -> String {
        // try vid pid first
        DEFAULT_ICONS
            .get(&Icon::ClassifierSubProtocol((class.to_owned(), sub, protocol)))
            .unwrap_or(
                DEFAULT_ICONS
                    .get(&Icon::Classifier(class.to_owned()))
                    .unwrap_or(DEFAULT_ICONS.get(&Icon::UndefinedClassifier).unwrap_or(&"")),
                ).to_string()
    }

    /// Drill through `Self` icons first looking for `ClassifierSubProtocol` -> `Classifier` -> `UndefinedClassifier` -> get_default_classifier_icon
    pub fn get_classifier_icon(&self, class: &ClassCode, sub: u8, protocol: u8) -> String {
        if let Some(user_icons) = self.icons.as_ref() {
            user_icons
            .get(&Icon::ClassifierSubProtocol((class.to_owned(), sub, protocol)))
            .unwrap_or(
                user_icons
                    .get(&Icon::Classifier(class.to_owned()))
                    .unwrap_or(&IconTheme::get_default_classifier_icon(class, sub, protocol)),
                )
                .to_owned()
        } else {
            IconTheme::get_default_classifier_icon(class, sub, protocol)
        }
    }
}

/// Gets tree icon from `DEFAULT_TREE` as `String` with `unwrap` because should panic if missing from there
pub fn get_default_tree_icon(i: &Icon) -> String {
    DEFAULT_TREE.get(i).unwrap().to_string()
}

/// Gets tree icon from `LSUSB_TREE` as `String` with `unwrap` because should panic if missing from there
pub fn get_ascii_tree_icon(i: &Icon) -> String {
    ASCII_TREE.get(i).unwrap().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[ignore]
    #[test]
    fn test_serialize_theme() {
        let theme = IconTheme{
            icons: Some(HashMap::from([
                (Icon::UnknownVendor, "\u{f287}".into()), // usb plug default 
            ])),
            ..Default::default()
        };
        assert_eq!(serde_json::to_string(&theme).unwrap(), "{\"icons\":{\"unknown-vendor\":\"\"},\"tree\":null}");
    }

    #[test]
    fn test_deserialize_theme() {
        let theme: IconTheme = serde_json::from_str("{\"icons\":{\"unknown-vendor\":\"\"},\"tree\":null}").unwrap();
        let actual_theme = IconTheme{
            icons: Some(HashMap::from([
                (Icon::UnknownVendor, "\u{f287}".into()), // usb plug default 
            ])),
            ..Default::default()
        };
        assert_eq!(theme, actual_theme);
    }

    #[test]
    fn test_serialize_defaults() {
        let theme = IconTheme{
            icons: Some(HashMap::from([
                (Icon::UnknownVendor, "\u{f287}".into()), // usb plug default 
                // (Icon::Classifier(ClassCode::HID), "\u{f80b}".into()), // 
            ])),
            ..Default::default()
        };
        println!("{}", serde_json::to_string(&theme).unwrap());
    }
}
