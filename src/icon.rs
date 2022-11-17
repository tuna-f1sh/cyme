use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::usb::ClassCode;
use crate::system_profiler::{USBDevice, USBBus};

// TODO FromStr and ToStr serialize/deserialize so that can merge with user defined
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Icon {
    /// vendor id lookup
    Vid(u16),
    /// vendor id and product id exact match
    VidPid((u16, u16)),
    /// Use to mask on msb of product ID
    VidPidMsb((u16, u8)),
    Classifier(ClassCode),
    UnknownVendor,
    TreeEdge,
    TreeLine,
    TreeCorner,
    TreeBlank,
    TreeBusStart,
    TreeDeviceTerminator,
    TreeClassifierTerminiator,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct IconTheme {
    pub icons: HashMap<Icon, String>,
}

impl Default for IconTheme {
    fn default() -> Self {
        IconTheme {
            icons: Self::get_default_icons(),
        }
    }
}

impl IconTheme {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_default_icons() -> HashMap<Icon, String> {
        HashMap::from([
            (Icon::TreeEdge, "\u{251c}\u{2500}\u{2500}".into()), // "├──"
            (Icon::TreeLine, "\u{2502}  ".into()), // "│  "
            (Icon::TreeCorner, "\u{2514}\u{2500}\u{2500}".into()), // "└──"
            (Icon::TreeBlank, "   ".into()), // should be same char width as above
            (Icon::TreeBusStart, "\u{25CF}".into()), // "●"
            (Icon::TreeDeviceTerminator, "\u{25CB}".into()), // "○"
            (Icon::TreeClassifierTerminiator, "\u{25E6}".into()), // "◦"
            (Icon::Vid(0x05ac), "\u{f179}".into()), // apple 
            (Icon::Vid(0x8086), "\u{f179}".into()), // apple bus 
            (Icon::Vid(0x045e), "\u{f871}".into()), // microsoft 
            (Icon::Vid(0x1D6B), "\u{f17c}".into()), // linux foundation 
            (Icon::Vid(0x1915), "\u{f5a2}".into()), // specialized 
            (Icon::Vid(0x0483), "\u{f5a2}".into()), // specialized 
            (Icon::Vid(0x091e), "\u{e2a6}".into()), // garmin 
            (Icon::Vid(0x1d50), "\u{f188}".into()), // black magic probe 
            (Icon::Vid(0x1366), "\u{f188}".into()), // segger 
            (Icon::UnknownVendor, "\u{f287}".into()), // usb plug default 
            (Icon::VidPidMsb((0x043e, 0x9a)), "\u{f878}".into()), // monitor 
            (Icon::VidPid((0x0781, 0xf7c9)), "\u{f878}".into()), // external disk 
        ])
    }

    pub fn get_tree_icon(&self, icon: Icon) -> String {
        self.icons.get(&icon).unwrap_or(&String::from("   ")).to_owned()
    }

    pub fn get_device_icon(&self, d: &USBDevice) -> String {
        if let (Some(vid), Some(pid)) = (d.vendor_id, d.product_id) {
            // try vid pid first
            self.icons.get(&Icon::VidPid((vid, pid)))
                .unwrap_or(self.icons.get(&Icon::VidPidMsb((vid, (pid >> 8) as u8)))
                   .unwrap_or(self.icons.get(&Icon::Vid(vid))
                        .unwrap_or(self.icons.get(&Icon::UnknownVendor).unwrap_or(&String::new())))).to_owned()
        } else {
            String::new()
        }
    }

    pub fn get_bus_icon(&self, d: &USBBus) -> String {
        if let (Some(vid), Some(pid)) = (d.pci_vendor, d.pci_device) {
            // try vid pid first
            self.icons.get(&Icon::VidPid((vid, pid)))
                .unwrap_or(self.icons.get(&Icon::VidPidMsb((vid, (pid >> 8) as u8)))
                   .unwrap_or(self.icons.get(&Icon::Vid(vid))
                        .unwrap_or(self.icons.get(&Icon::UnknownVendor).unwrap_or(&String::new())))).to_owned()
        } else {
            String::new()
        }
    }
}
