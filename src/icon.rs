//! Icons and themeing of cyme output
#[cfg(feature = "regex_icon")]
use regex;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use crate::display::Encoding;
use crate::error::{Error, ErrorKind};
use crate::profiler::{Bus, Device};
use crate::usb::{BaseClass, Direction};

/// If only standard UTF-8 characters are used, this is the default icon for a device
// const UTF8_DEFAULT_DEVICE_ICON: &str = "\u{2023}"; // ‣

/// Serialize alphabetically for HashMaps so they don't change each generation
fn sort_alphabetically<T: Serialize, S: serde::Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let value = serde_json::to_value(value).map_err(serde::ser::Error::custom)?;
    value.serialize(serializer)
}

/// Icon type enum is used as key in `HashMaps`
#[derive(Debug, Clone, Hash, PartialEq, Eq, SerializeDisplay, DeserializeFromStr)]
pub enum Icon {
    /// Vendor ID lookup
    Vid(u16),
    /// Vendor ID and Product ID exact match
    VidPid((u16, u16)),
    /// Use to mask on msb of product ID
    VidPidMsb((u16, u8)),
    /// Class classifier icon
    Classifier(BaseClass),
    /// Class classifier lookup with SubClass and Protocol
    ClassifierSubProtocol((BaseClass, u8, u8)),
    /// Pattern match device name icon
    Name(String),
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
    /// Icon at prepended before printing `Bus`
    TreeBusStart,
    /// Icon printed at end of tree before printing `Device`
    TreeDeviceTerminator,
    /// Icon printed at end of tree before printing configuration
    TreeConfigurationTerminator,
    /// Icon printed at end of tree before printing interface
    TreeInterfaceTerminator,
    /// Icon for endpoint direction
    Endpoint(Direction),
}

impl FromStr for Icon {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value_split: Vec<&str> = s.split('#').collect();
        let enum_name = value_split[0];

        // no value in string, match kebab-case
        if value_split.len() == 1 {
            match enum_name {
                "unknown-vendor" => Ok(Icon::UnknownVendor),
                "undefined-classifier" => Ok(Icon::UndefinedClassifier),
                "tree-edge" => Ok(Icon::TreeEdge),
                "tree-blank" => Ok(Icon::TreeBlank),
                "tree-line" => Ok(Icon::TreeLine),
                "tree-corner" => Ok(Icon::TreeCorner),
                "tree-bus-start" => Ok(Icon::TreeBusStart),
                "tree-device-terminator" => Ok(Icon::TreeDeviceTerminator),
                "tree-configuration-terminator" => Ok(Icon::TreeConfigurationTerminator),
                "tree-interface-terminator" => Ok(Icon::TreeInterfaceTerminator),
                "endpoint_in" => Ok(Icon::Endpoint(Direction::In)),
                "endpoint_out" => Ok(Icon::Endpoint(Direction::Out)),
                _ => Err(Error::new(
                    ErrorKind::Parsing,
                    "Invalid Icon enum name or valued enum without value",
                )),
            }
        // name#pattern
        } else if matches!(enum_name, "name") {
            #[cfg(feature = "regex_icon")]
            match regex::Regex::new(value_split[1]) {
                Ok(_) => Ok(Icon::Name(value_split[1].to_string())),
                Err(_) => Err(Error::new(
                    ErrorKind::Parsing,
                    &format!(
                        "Invalid regex pattern in Icon::Name enum string: {}",
                        value_split[1]
                    ),
                )),
            }
            #[cfg(not(feature = "regex_icon"))]
            Err(Error::new(
                ErrorKind::Parsing,
                "regex_icon feature not enabled for Icon::Name matching",
            ))
        // enum contains value
        } else {
            let (parse_ints, errors): (Vec<Result<u32, _>>, Vec<_>) = value_split[1]
                .split(':')
                .map(|vs| u32::from_str_radix(vs.trim_start_matches("0x"), 16))
                .partition(Result::is_ok);
            let numbers: Vec<u16> = parse_ints.into_iter().map(|v| v.unwrap() as u16).collect();

            if !errors.is_empty() {
                return Err(Error::new(
                    ErrorKind::Parsing,
                    "Invalid value in enum string after #",
                ));
            }

            match value_split[0] {
                "vid" => match numbers.first() {
                    Some(i) => Ok(Icon::Vid(*i)),
                    None => Err(Error::new(ErrorKind::Parsing, "No value for enum after $")),
                },
                "vid-pid" => match numbers.get(0..2) {
                    Some(slice) => Ok(Icon::VidPid((slice[0], slice[1]))),
                    None => Err(Error::new(ErrorKind::Parsing, "No value for enum after $")),
                },
                "vid-pid-msb" => match numbers.get(0..2) {
                    Some(slice) => Ok(Icon::VidPidMsb((slice[0], slice[1] as u8))),
                    None => Err(Error::new(ErrorKind::Parsing, "No value for enum after $")),
                },
                "classifier" => match numbers.first() {
                    Some(i) => Ok(Icon::Classifier(BaseClass::from(*i as u8))),
                    None => Err(Error::new(ErrorKind::Parsing, "No value for enum after $")),
                },
                "classifier-sub-protocol" => match numbers.get(0..3) {
                    Some(slice) => Ok(Icon::ClassifierSubProtocol((
                        BaseClass::from(slice[0] as u8),
                        slice[1] as u8,
                        slice[2] as u8,
                    ))),
                    None => Err(Error::new(ErrorKind::Parsing, "No value for enum after $")),
                },
                _ => Err(Error::new(
                    ErrorKind::Parsing,
                    "Invalid Icon enum value holder",
                )),
            }
        }
    }
}

impl fmt::Display for Icon {
    /// Output is a Enum kebab case with # separating base16 : separated values, _ if String value
    ///
    /// ```
    /// use cyme::icon::*;
    ///
    /// let icon: Icon = Icon::VidPid((0x1d50, 0x6018));
    /// assert_eq!(format!("{}", icon), "vid-pid#1d50:6018");
    ///
    /// let icon: Icon = Icon::UnknownVendor;
    /// assert_eq!(format!("{}", icon), "unknown-vendor");
    /// ```
    ///
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Icon::Vid(v) => write!(f, "vid#{:04x}", v),
            Icon::VidPid((v, p)) => write!(f, "vid-pid#{:04x}:{:04x}", v, p),
            Icon::VidPidMsb((v, p)) => write!(f, "vid-pid-msb#{:04x}:{:02x}", v, p),
            Icon::Classifier(c) => write!(f, "classifier#{:02x}", u8::from(c.to_owned())),
            Icon::ClassifierSubProtocol(c) => write!(
                f,
                "classifier-sub-protocol#{:02x}:{:02x}:{:02x}",
                u8::from(c.0.to_owned()),
                c.1,
                c.2
            ),
            Icon::Name(s) => write!(f, "name#{}", s),
            Icon::Endpoint(Direction::In) => write!(f, "endpoint_in"),
            Icon::Endpoint(Direction::Out) => write!(f, "endpoint_out"),
            _ => {
                let dbg_str = format!("{:?}", self);
                write!(f, "{}", heck::AsKebabCase(dbg_str))
            }
        }
    }
}

/// Allows user supplied icons to replace or add to [`static@DEFAULT_ICONS`] and [`static@DEFAULT_UTF8_TREE`]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct IconTheme {
    /// Will merge with [`static@DEFAULT_ICONS`] for user supplied
    #[serde(serialize_with = "sort_alphabetically")]
    pub user: Option<HashMap<Icon, String>>,
    /// Will merge with [`static@DEFAULT_UTF8_TREE`] for user supplied tree drawing
    #[serde(serialize_with = "sort_alphabetically")]
    pub tree: Option<HashMap<Icon, String>>,
}

/// Make default icons lazy_static and outside of IconTheme keeps them static but can be overridden user HashMap<Icon, String> at runtime
impl Default for IconTheme {
    fn default() -> Self {
        IconTheme {
            user: None,
            tree: None,
        }
    }
}

lazy_static! {
    /// Default icons to draw tree can be overridden by user icons with IconTheme `tree`
    pub static ref DEFAULT_UTF8_TREE: HashMap<Icon, &'static str> = {
        HashMap::from([
            (Icon::TreeEdge, "\u{251c}\u{2500}\u{2500}"), // "├──"
            (Icon::TreeLine, "\u{2502}  "), // "│  "
            (Icon::TreeCorner, "\u{2514}\u{2500}\u{2500}"), // "└──"
            (Icon::TreeBlank, "   "), // should be same char width as above
            (Icon::TreeBusStart, "\u{25CF}"), // "●"
            (Icon::TreeDeviceTerminator, "\u{25CB}"), // "○"
            (Icon::TreeConfigurationTerminator, "\u{2022}"), // "•"
            (Icon::TreeInterfaceTerminator, "\u{25E6}"), // "◦"
            (Icon::Endpoint(Direction::In), "\u{2192}"), // →
            (Icon::Endpoint(Direction::Out), "\u{2190}"), // ←
            // (Icon::Endpoint(Direction::In), ">".into()), // →
            // (Icon::Endpoint(Direction::Out), "<".into()), // ←
        ])
    };

    /// Ascii chars used by lsusb compatible mode or no utf-8
    pub static ref DEFAULT_ASCII_TREE: HashMap<Icon, &'static str> = {
        HashMap::from([
            (Icon::TreeEdge, "|__"), // same as corner
            (Icon::TreeLine, "|  "), // no outside line but inset so starts under parent device
            (Icon::TreeCorner, "|__"),
            (Icon::TreeBlank, "   "), // inset like line
            (Icon::TreeBusStart, "/: "),
            (Icon::TreeDeviceTerminator, "O"), // null
            (Icon::TreeConfigurationTerminator, "o"), // null
            (Icon::TreeInterfaceTerminator, "."), // null
            (Icon::Endpoint(Direction::In), ">"), //
            (Icon::Endpoint(Direction::Out), "<"), //
        ])
    };

    /// Default icon lookup can be overridden by user icons with IconTheme `icons`
    ///
    /// Should probably keep fairly short but I've added things I use like debuggers, mcus as examples
    pub static ref DEFAULT_ICONS: HashMap<Icon, &'static str> = {
        HashMap::from([
            (Icon::UnknownVendor, "\u{f287}"), // usb plug default 
            (Icon::Vid(0x05ac), "\u{f179}"), // apple 
            (Icon::Vid(0x045e), "\u{f0372}"), // microsoft 󰍲
            (Icon::Vid(0x18d1), "\u{f1a0}"), // google 
            (Icon::Vid(0x1D6B), "\u{f17c}"), // linux foundation 
            (Icon::Vid(0x1d50), "\u{e771}"), // open source VID 
            (Icon::VidPid((0x1915, 0x520c)), "\u{f00a3}"), // specialized 󰂣
            (Icon::VidPid((0x1915, 0x520d)), "\u{f00a3}"), // specialized 󰂣
            (Icon::VidPid((0x0483, 0x572B)), "\u{f00a3}"), // specialized 󰂣
            (Icon::Vid(0x046d), "\u{f037d}"), // logitech 󰍽
            (Icon::Vid(0x091e), "\u{e2a6}"), // garmin 
            (Icon::VidPid((0x1d50, 0x6018)), "\u{f188}"), // black magic probe 
            (Icon::Vid(0x1366), "\u{f188}"), // segger 
            (Icon::Vid(0xf1a0), "\u{f188}"), // arm 
            (Icon::VidPidMsb((0x0483, 0x37)), "\u{f188}"), // st-link 
            (Icon::VidPid((0x0483, 0xdf11)), "\u{f019}"), // STM DFU 
            (Icon::VidPid((0x1d50, 0x6017)), "\u{f188}"), // black magic probe DFU 
            (Icon::ClassifierSubProtocol((BaseClass::ApplicationSpecificInterface, 0x01, 0x01)), "\u{f188}"), // DFU 
            (Icon::ClassifierSubProtocol((BaseClass::WirelessController, 0x01, 0x01)), "\u{f188}"), // bluetooth DFU 
            (Icon::Vid(0x2341), "\u{f2db}"), // arduino 
            (Icon::Vid(0x239A), "\u{f2db}"), // adafruit 
            (Icon::Vid(0x2e8a), "\u{f315}"), // raspberry pi foundation 
            (Icon::Vid(0x0483), "\u{f2db}"), // stm 
            (Icon::Vid(0x1915), "\u{f2db}"), // nordic 
            (Icon::Vid(0x1fc9), "\u{f2db}"), // nxp 
            (Icon::Vid(0x1050), "\u{f084}"), // yubikey 
            (Icon::Vid(0x0781), "\u{f129e}"), // sandisk 󱊞
            #[cfg(feature = "regex_icon")]
            (Icon::Name(r".*^[sS][dD]\s[cC]ard\s[rR]eader.*".to_string()), "\u{ef61}"), // sd card reader 
            (Icon::VidPid((0x18D1, 0x2D05)), "\u{e70e}"), // android dev 
            (Icon::VidPid((0x18D1, 0xd00d)), "\u{e70e}"), // android 
            (Icon::VidPid((0x1d50, 0x606f)), "\u{f191d}"), // candlelight_fw gs_can 󱤝
            (Icon::VidPidMsb((0x043e, 0x9a)), "\u{f0379}"), // lg monitor 󰍹
            (Icon::Classifier(BaseClass::Audio), "\u{f001}"), // 
            (Icon::Classifier(BaseClass::Image), "\u{f03e}"), // 
            (Icon::Classifier(BaseClass::Video), "\u{f03d}"), // 
            (Icon::Classifier(BaseClass::Printer), "\u{f02f}"), // 
            (Icon::Classifier(BaseClass::MassStorage), "\u{f0a0}"), // 
            (Icon::Classifier(BaseClass::Hub), "\u{f126}"), // 
            (Icon::Classifier(BaseClass::ContentSecurity), "\u{f084}"), // 
            (Icon::Classifier(BaseClass::SmartCard), "\u{f084}"), // 
            (Icon::Classifier(BaseClass::PersonalHealthcare), "\u{f21e}"), // 
            (Icon::Classifier(BaseClass::AudioVideo), "\u{f0841}"), // 󰡁
            (Icon::Classifier(BaseClass::Billboard), "\u{f05a}"), // 
            (Icon::Classifier(BaseClass::I3cDevice), "\u{f493}"), // 
            (Icon::Classifier(BaseClass::Diagnostic), "\u{f489}"), // 
            (Icon::Classifier(BaseClass::WirelessController), "\u{f1eb}"), // 
            (Icon::Classifier(BaseClass::Miscellaneous), "\u{f074}"), // 
            (Icon::Classifier(BaseClass::CdcCommunications), "\u{e795}"), // serial 
            (Icon::Classifier(BaseClass::CdcData), "\u{e795}"), // serial 
            (Icon::Classifier(BaseClass::Hid), "\u{f030c}"), // 󰌌
            (Icon::UndefinedClassifier, "\u{2636}"), //☶
        ])
    };
}

impl IconTheme {
    /// New theme with defaults
    pub fn new() -> Self {
        Default::default()
    }

    /// Get tree building icon checks `Self` for user `tree` and tries to find `icon` there, otherwise uses [`static@DEFAULT_UTF8_TREE`]
    ///
    /// Also checks if user icon is valid for encoding, if not will return default for that encoding
    pub fn get_tree_icon(&self, icon: &Icon, encoding: &Encoding) -> String {
        // unwrap on DEFAULT_UTF8_TREE is ok here since should panic if missing from static list
        if let Some(user_tree) = self.tree.as_ref() {
            user_tree
                .get(icon)
                .map(|s| match encoding.str_is_valid(s) {
                    true => s.to_owned(),
                    false => get_default_tree_icon(icon, encoding),
                })
                .unwrap_or(get_default_tree_icon(icon, encoding))
        } else {
            get_default_tree_icon(icon, encoding)
        }
    }

    /// Drill through [`static@DEFAULT_ICONS`] first looking for `VidPid` -> `VidPidMsb` -> `Vid` -> `UnknownVendor` -> ""
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
        if let Some(user_icons) = self.user.as_ref() {
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
    pub fn get_default_device_icon(d: &Device) -> String {
        if let (Some(vid), Some(pid)) = (d.vendor_id, d.product_id) {
            IconTheme::get_default_vidpid_icon(vid, pid)
        } else {
            String::new()
        }
    }

    /// Get icon for Device `d` by checking `Self` using Name, Vendor ID and Product ID
    #[cfg(feature = "regex_icon")]
    pub fn get_device_icon(&self, d: &Device) -> String {
        // try name first since vidpid will return UnknownVendor default icon if not found
        // does mean regex will be built/checked for every device
        match self.get_name_icon(&d.name) {
            s if !s.is_empty() => s,
            _ => {
                if let (Some(vid), Some(pid)) = (d.vendor_id, d.product_id) {
                    self.get_vidpid_icon(vid, pid)
                } else {
                    String::new()
                }
            }
        }
    }

    /// Get icon for Device `d` by checking `Self` using Vendor ID and Product ID
    #[cfg(not(feature = "regex_icon"))]
    pub fn get_device_icon(&self, d: &Device) -> String {
        if let (Some(vid), Some(pid)) = (d.vendor_id, d.product_id) {
            self.get_vidpid_icon(vid, pid)
        } else {
            DEFAULT_ICONS
                .get(&Icon::UnknownVendor)
                .unwrap_or(&"")
                .to_string()
        }
    }

    /// Get icon for Bus `d` by checking `Self` using PCI Vendor and PCI Device
    pub fn get_bus_icon(&self, d: &Bus) -> String {
        if let (Some(vid), Some(pid)) = (d.pci_vendor, d.pci_device) {
            self.get_vidpid_icon(vid, pid)
        } else {
            DEFAULT_ICONS
                .get(&Icon::UnknownVendor)
                .unwrap_or(&"")
                .to_string()
        }
    }

    /// Drill through `DEFAULT_ICONS` first looking for `ClassifierSubProtocol` -> `Classifier` -> `UndefinedClassifier` -> ""
    pub fn get_default_classifier_icon(class: &BaseClass, sub: u8, protocol: u8) -> String {
        // try vid pid first
        DEFAULT_ICONS
            .get(&Icon::ClassifierSubProtocol((
                class.to_owned(),
                sub,
                protocol,
            )))
            .unwrap_or(
                DEFAULT_ICONS
                    .get(&Icon::Classifier(class.to_owned()))
                    .unwrap_or(DEFAULT_ICONS.get(&Icon::UndefinedClassifier).unwrap_or(&"")),
            )
            .to_string()
    }

    /// Drill through `Self` icons first looking for `ClassifierSubProtocol` -> `Classifier` -> `UndefinedClassifier` -> get_default_classifier_icon
    pub fn get_classifier_icon(&self, class: &BaseClass, sub: u8, protocol: u8) -> String {
        if let Some(user_icons) = self.user.as_ref() {
            user_icons
                .get(&Icon::ClassifierSubProtocol((
                    class.to_owned(),
                    sub,
                    protocol,
                )))
                .unwrap_or(
                    user_icons
                        .get(&Icon::Classifier(class.to_owned()))
                        .unwrap_or(&IconTheme::get_default_classifier_icon(
                            class, sub, protocol,
                        )),
                )
                .to_owned()
        } else {
            IconTheme::get_default_classifier_icon(class, sub, protocol)
        }
    }

    /// Get default icon for device based on descriptor name pattern `[Icon::Name]` pattern match
    #[cfg(feature = "regex_icon")]
    pub fn get_default_name_icon(name: &str) -> String {
        DEFAULT_ICONS
            .iter()
            .find(|(k, _)| {
                if let Icon::Name(s) = k {
                    regex::Regex::new(s).map_or(false, |r| r.is_match(name))
                } else {
                    false
                }
            })
            .map(|(_, v)| v.to_owned())
            .unwrap_or("")
            .to_string()
    }

    /// Get icon for device based on descriptor name pattern `[Icon::Name]` pattern match
    #[cfg(feature = "regex_icon")]
    pub fn get_name_icon(&self, name: &str) -> String {
        if let Some(user_icons) = self.user.as_ref() {
            user_icons
                .iter()
                .find(|(k, _)| {
                    if let Icon::Name(s) = k {
                        regex::Regex::new(s).map_or(false, |r| r.is_match(name))
                    } else {
                        false
                    }
                })
                .map(|(_, v)| v.to_owned())
                .unwrap_or(String::new())
        } else {
            IconTheme::get_default_name_icon(name)
        }
    }
}

/// Gets tree icon from [`static@DEFAULT_UTF8_TREE`] or [`static@DEFAULT_ASCII_TREE`] (depanding on [`Encoding`]) as `String` with `unwrap` because should panic if missing from there
pub fn get_default_tree_icon(i: &Icon, encoding: &Encoding) -> String {
    match encoding {
        Encoding::Utf8 | Encoding::Glyphs => DEFAULT_UTF8_TREE.get(i).unwrap().to_string(),
        Encoding::Ascii => DEFAULT_ASCII_TREE.get(i).unwrap().to_string(),
    }
}

/// Gets tree icon from [`static@DEFAULT_ASCII_TREE`] as `String` with `unwrap` because should panic if missing from there
pub fn get_ascii_tree_icon(i: &Icon) -> String {
    DEFAULT_ASCII_TREE.get(i).unwrap().to_string()
}

/// Returns clone of lazy_static defaults
pub fn defaults() -> HashMap<Icon, &'static str> {
    DEFAULT_ICONS.clone()
}

/// Returns example list of icons with all [`Icon`] types
pub fn example() -> HashMap<Icon, String> {
    HashMap::from([
        (Icon::UnknownVendor, "\u{f287}".into()), // usb plug default 
        (Icon::Vid(0x05ac), "\u{f179}".into()),   // apple 
        (Icon::VidPid((0x1d50, 0x6018)), "\u{f188}".into()), // black magic probe 
        (Icon::VidPidMsb((0x0483, 0x37)), "\u{f188}".into()), // st-link 
        (
            Icon::ClassifierSubProtocol((BaseClass::ApplicationSpecificInterface, 0x01, 0x01)),
            "\u{f188}".into(),
        ), // DFU 
        (Icon::Vid(0x2e8a), "\u{f315}".into()),   // raspberry pi foundation 
        (
            Icon::Classifier(BaseClass::CdcCommunications),
            "\u{e795}".into(),
        ), // serial 
        (Icon::UndefinedClassifier, "\u{2636}".into()), //☶
        #[cfg(feature = "regex_icon")]
        (
            Icon::Name(r".*^[sS][dD]\s[cC]ard\s[rR]eader.*".to_string()),
            "\u{ef61}".into(),
        ), // sd card reader 
    ])
}

/// Returns example theme with [`Icon`] types and default tree
pub fn example_theme() -> IconTheme {
    let tree_strings: HashMap<Icon, String> = DEFAULT_UTF8_TREE
        .iter()
        .map(|(k, v)| (k.to_owned(), v.to_string()))
        .collect();

    IconTheme {
        user: Some(example()),
        tree: Some(tree_strings),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_theme() {
        let theme = IconTheme {
            user: Some(HashMap::from([
                (Icon::UnknownVendor, "\u{f287}".into()), // usb plug default 
            ])),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&theme).unwrap(),
            "{\"user\":{\"unknown-vendor\":\"\"},\"tree\":null}"
        );
    }

    #[test]
    fn test_deserialize_theme() {
        let theme: IconTheme =
            serde_json::from_str("{\"user\":{\"unknown-vendor\":\"\"},\"tree\":null}").unwrap();
        let actual_theme = IconTheme {
            user: Some(HashMap::from([
                (Icon::UnknownVendor, "\u{f287}".into()), // usb plug default 
            ])),
            ..Default::default()
        };
        assert_eq!(theme, actual_theme);
    }

    #[test]
    fn test_serialize_defaults() {
        serde_json::to_string(&defaults()).unwrap();
    }

    #[test]
    fn test_serialize_example() {
        println!("{}", serde_json::to_string_pretty(&example()).unwrap());
    }

    #[test]
    fn test_deserialize_icon_tuples() {
        let item: (Icon, &'static str) = (Icon::VidPid((0x1d50, 0x6018)), "\u{f188}");
        let item_ser = serde_json::to_string(&item).unwrap();
        assert_eq!(item_ser, r#"["vid-pid#1d50:6018",""]"#);

        let item: (Icon, &'static str) = (Icon::Endpoint(Direction::In), ">");
        let item_ser = serde_json::to_string(&item).unwrap();
        assert_eq!(item_ser, r#"["endpoint_in",">"]"#);

        let item: (Icon, &'static str) = (
            Icon::ClassifierSubProtocol((BaseClass::Hid, 0x01, 0x0a)),
            "K",
        );
        let item_ser = serde_json::to_string(&item).unwrap();
        assert_eq!(item_ser, r#"["classifier-sub-protocol#03:01:0a","K"]"#);
    }

    #[test]
    fn icon_from_str() {
        let str = "vid#1d50";
        let icon = Icon::from_str(str);
        assert_eq!(icon.unwrap(), Icon::Vid(7504));

        let str = "vid-pid#1d50:6018";
        let icon = Icon::from_str(str);
        assert_eq!(icon.unwrap(), Icon::VidPid((7504, 24600)));

        let str = "classifier#03";
        let icon = Icon::from_str(str);
        assert_eq!(icon.unwrap(), Icon::Classifier(BaseClass::Hid));

        let str = "classifier-sub-protocol#03:01:0a";
        let icon = Icon::from_str(str);
        assert_eq!(
            icon.unwrap(),
            Icon::ClassifierSubProtocol((BaseClass::Hid, 1, 10))
        );

        let str = "endpoint_in";
        let icon = Icon::from_str(str);
        assert_eq!(icon.unwrap(), Icon::Endpoint(Direction::In));

        let str = "unknown-vendor";
        let icon = Icon::from_str(str);
        assert_eq!(icon.unwrap(), Icon::UnknownVendor);

        if cfg!(feature = "regex_icon") {
            let str = "name#test";
            let icon = Icon::from_str(str);
            assert_eq!(icon.unwrap(), Icon::Name("test".to_string()));

            let str = r"name#.*^[sS][dD]\s[cC]ard\s[rR]eader.*";
            let icon = Icon::from_str(str);
            assert_eq!(
                icon.unwrap(),
                Icon::Name(r".*^[sS][dD]\s[cC]ard\s[rR]eader.*".to_string())
            );
        }
    }

    #[test]
    #[cfg(feature = "regex_icon")]
    fn icon_match_name() {
        let mut device = Device {
            name: "SD Card Reader".to_string(),
            ..Default::default()
        };

        let theme = IconTheme {
            user: Some(HashMap::from([(
                Icon::Name(r".*^[sS][dD]\s[cC]ard\s[rR]eader.*".to_string()),
                "\u{ef61}".into(),
            )])),
            ..Default::default()
        };

        let icon = theme.get_device_icon(&device);
        assert_eq!(icon, "\u{ef61}");

        device.name = "sD Card reader 2".to_string();
        let icon = theme.get_device_icon(&device);
        assert_eq!(icon, "\u{ef61}");
    }
}
