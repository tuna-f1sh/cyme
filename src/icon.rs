use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::system_profiler::{USBBus, USBDevice};
use crate::usb::ClassCode;

/// Icon type enum is used as key in `HashMaps`
/// TODO FromStr and ToStr serialize/deserialize so that can merge with user defined
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Icon {
    /// Vendor ID lookup
    Vid(u16),
    /// Vendor ID and Product ID exact match
    VidPid((u16, u16)),
    /// Use to mask on msb of product ID
    VidPidMsb((u16, u8)),
    /// Class classifier icon
    Classifier(ClassCode),
    UnknownVendor,
    TreeEdge,
    TreeLine,
    TreeCorner,
    TreeBlank,
    /// Icon at prepended before printing `USBBus`
    TreeBusStart,
    /// Icon printed at end of tree before printing `USBDevice`
    TreeDeviceTerminator,
    /// Icon printed at end of tree before printing classifier
    TreeClassifierTerminiator,
}

/// Allows user supplied icons to replace or add to `DEFAULT_ICONS` and `DEFAULT_TREE`
/// TODO FromStr deserialize so that we can import user file
#[derive(Debug, Deserialize, PartialEq, Eq)]
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
            (Icon::TreeClassifierTerminiator, "\u{25E6}".into()), // "◦"
        ])
    };

    /// Default icon lookup can be overridden by user icons with IconTheme `icons`
    ///
    /// Should probably keep fairly short but I've added things I use like debuggers, mcus as examples
    static ref DEFAULT_ICONS: HashMap<Icon, &'static str> = {
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
            (Icon::Vid(0x2341), "\u{f2db}".into()), // arduino 
            (Icon::Vid(0x239A), "\u{f2db}".into()), // adafruit 
            (Icon::Vid(0x2e8a), "\u{f315}".into()), // raspberry pi foundation 
            (Icon::Vid(0x0483), "\u{f2db}".into()), // stm 
            (Icon::Vid(0x1915), "\u{f2db}".into()), // nordic 
            (Icon::Vid(0x1fc9), "\u{f2db}".into()), // nxp 
            (Icon::Vid(0x1050), "\u{f805}".into()), // yubikey 
            (Icon::VidPid((0x1d50, 0x606f)), "\u{f5e6}".into()), // candlelight_fw gs_can 
            (Icon::VidPidMsb((0x043e, 0x9a)), "\u{f878}".into()), // lg monitor 
            (Icon::VidPid((0x0781, 0xf7c9)), "\u{f878}".into()), // sandisk external disk 
        ])
    };
}

impl IconTheme {
    pub fn new() -> Self {
        Default::default()
    }

    /// Get tree building icon checks `Self` for user `tree` and tries to find `icon` there, otherwise uses `DEFAULT_TREE`
    pub fn get_tree_icon(&self, icon: Icon) -> String {
        // unwrap on DEFAULT_TREE is ok here since should panic if missing from static list
        if let Some(user_tree) = self.tree.as_ref() {
            user_tree
                .get(&icon)
                .unwrap_or(&DEFAULT_TREE.get(&icon).unwrap().to_string())
                .bright_black()
                .to_string()
                .to_owned()
        } else {
            DEFAULT_TREE.get(&icon).unwrap().bright_black().to_string()
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
}
