use clap::ValueEnum;
use colored::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::system_profiler;

// utf-8 boxes for drawing tree
const EDGE: &str = "\u{251c}\u{2500}\u{2500}"; // "├──"
const LINE: &str = "\u{2502}  "; // "│  "
const CORNER: &str = "\u{2514}\u{2500}\u{2500}"; // "└──"
const BLANK: &str = "   "; // should be same char width as above

#[non_exhaustive]
#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize)]
pub enum Blocks {
    BusNumber,
    PortNumber,
    DeviceNumber,
    BranchPosition,
    VendorID,
    ProductID,
    Name,
    Manufacturer,
    Serial,
    Speed,
    HostController,
    PCIRevision,
    TreePositions,
    BusPower,
    BusPowerUsed,
    ExtraCurrentUsed,
    Bcd,
}

pub type DeviceBlocks = Vec<Blocks>;
pub type BusBlocks = Vec<Blocks>;

impl Blocks {
    pub fn default_device_blocks() -> DeviceBlocks {
        vec![
            Blocks::BusNumber,
            Blocks::PortNumber,
            Blocks::VendorID,
            Blocks::ProductID,
            Blocks::Name,
            Blocks::Serial,
            Blocks::Speed,
        ]
    }

    pub fn default_bus_blocks() -> BusBlocks {
        vec![
            Blocks::Name,
            Blocks::HostController,
            Blocks::VendorID,
            Blocks::ProductID,
            Blocks::PCIRevision,
        ]
    }

    pub fn colour(&self, s: &String) -> ColoredString {
        match self {
            Blocks::BusNumber => s.cyan(),
            Blocks::PortNumber => s.magenta(),
            Blocks::DeviceNumber => s.bright_magenta(),
            Blocks::BranchPosition => s.bright_magenta(),
            Blocks::VendorID => s.bold().yellow(),
            Blocks::ProductID => s.yellow(),
            Blocks::Name => s.bold().blue(),
            Blocks::Manufacturer => s.blue(),
            Blocks::Serial => s.green(),
            Blocks::Speed => s.purple(),
            Blocks::HostController => s.green(),
            Blocks::PCIRevision => s.normal(),
            Blocks::TreePositions => s.magenta(),
            Blocks::BusPower => s.purple(),
            Blocks::BusPowerUsed => s.bright_purple(),
            Blocks::ExtraCurrentUsed => s.red(),
            Blocks::Bcd => s.purple(),
            // _ => todo!("Add colour for new block"),
        }
    }

    pub fn icon(&self, d: &system_profiler::USBDevice) -> Option<String> {
        match self {
            // TODO separate icons for Vendor and Product; some can match from just vendor id like apple, microsoft others should be vendor and product lookup like harddisk etc.
            // make struct Icon with impl for get_vendor_icon, get_product_icon, get_global_icon (tree, usb device etc.) - can be merged with load from file
            // HashMap<String, String> where key is vendor base16, vendor:product base16 and global ref
            Blocks::VendorID => match d.vendor_id {
                Some(v) => match v {
                    0x05ac => Some("\u{f179}".into()),          // apple 
                    0x045e => Some("\u{f871}".into()),          // microsoft 
                    0x1D6B => Some("\u{f17c}".into()),          // linux foundation 
                    0x1915 | 0x0483 => Some("\u{f5a2}".into()), // specialized 
                    0x091e => Some("\u{e2a6}".into()),          // garmin 
                    0x1d50 | 0x1366 => Some("\u{f188}".into()), // debuggers 
                    0x043e => Some("\u{f878}".into()),          // monitor 
                    0x0781 => Some("\u{f7c9}".into()),          // external disk 
                    // _ => Some("\u{f287}".into()), // usb plug default
                    _ => Some("".into()),
                },
                None => None,
            },
            _ => None,
        }
    }

    pub fn format_device_value(
        &self,
        d: &system_profiler::USBDevice,
        pad: &PrintPadding,
    ) -> Option<String> {
        match self {
            Blocks::BusNumber => Some(format!("{:3}", d.location_id.bus)),
            Blocks::DeviceNumber => Some(match d.location_id.number {
                Some(v) => format!("{:3}", v),
                None => format!("{:>3}", "-"),
            }),
            Blocks::PortNumber => Some(match d.location_id.port {
                Some(v) => format!("{:3}", v),
                None => format!("{:>3}", "-"),
            }),
            Blocks::BranchPosition => Some(format!(
                "{:3}",
                d.location_id.tree_positions.last().unwrap_or(&0)
            )),
            Blocks::VendorID => Some(match d.vendor_id {
                Some(v) => format!("0x{:04x}", v),
                None => format!("{:>6}", "-"),
            }),
            Blocks::ProductID => Some(match d.product_id {
                Some(v) => format!("0x{:04x}", v),
                None => format!("{:>6}", "-"),
            }),
            Blocks::Name => Some(format!("{:pad$}", d.name.trim(), pad = pad.name)),
            Blocks::Manufacturer => Some(match d.manufacturer.as_ref() {
                Some(v) => format!("{:pad$}", v.trim(), pad = pad.manufacturer),
                None => format!("{:pad$}", "-", pad = pad.manufacturer),
            }),
            Blocks::Serial => Some(match d.serial_num.as_ref() {
                Some(v) => format!("{:pad$}", v.trim(), pad = pad.serial),
                None => format!("{:pad$}", "-", pad = pad.serial),
            }),
            Blocks::Speed => Some(match d.device_speed.as_ref() {
                Some(v) => format!("{:>10}", v.to_string()),
                None => format!("{:>10}", "-"),
            }),
            Blocks::TreePositions => Some(format!(
                "{:pad$}",
                format!("{:}", d.location_id.tree_positions.iter().format("╌")),
                pad = pad.tree_positions
            )),
            Blocks::BusPower => Some(match d.bus_power {
                Some(v) => format!("{:3} mA", v),
                None => format!("{:>6}", "-"),
            }),
            Blocks::BusPowerUsed => Some(match d.bus_power_used {
                Some(v) => format!("{:3} mA", v),
                None => format!("{:>6}", "-"),
            }),
            Blocks::ExtraCurrentUsed => Some(match d.extra_current_used {
                Some(v) => format!("{:3} mA", v),
                None => format!("{:>6}", "-"),
            }),
            Blocks::Bcd => Some(match d.bcd_device {
                Some(v) => format!("{:>5.2}", v),
                None => format!("{:>8}", "-"),
            }),
            _ => None,
        }
    }

    fn format_bus_value(
        &self,
        bus: &system_profiler::USBBus,
        pad: &PrintPadding,
    ) -> Option<String> {
        match self {
            Blocks::BusNumber => Some(format!("{:3}", bus.get_bus_number())),
            Blocks::VendorID => Some(match bus.pci_vendor {
                Some(v) => format!("0x{:04x}", v),
                None => format!("{:>6}", "-"),
            }),
            Blocks::ProductID => Some(match bus.pci_device {
                Some(v) => format!("0x{:04x}", v),
                None => format!("{:>6}", "-"),
            }),
            Blocks::PCIRevision => Some(match bus.pci_revision {
                Some(v) => format!("0x{:04x}", v),
                None => format!("{:>6}", "-"),
            }),
            Blocks::Name => Some(format!("{:pad$}", bus.name, pad = pad.name)),
            Blocks::HostController => Some(format!(
                "{:pad$}",
                bus.host_controller,
                pad = pad.host_controller
            )),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct PrintPadding {
    pub name: usize,
    pub manufacturer: usize,
    pub serial: usize,
    pub tree_positions: usize,
    pub host_controller: usize,
}

#[derive(Debug, Default)]
pub struct PrintSettings {
    pub no_padding: bool,
    pub base10: bool,
    pub tree: bool,
    pub icons: bool,
}

pub fn print_device(d: &system_profiler::USBDevice, blocks: &DeviceBlocks, pad: &PrintPadding) {
    for b in blocks {
        if let Some(string) = b.format_device_value(d, pad) {
            if let Some(icon) = b.icon(d) {
                print!("{:2} ", icon);
            }
            print!("{} ", b.colour(&string));
        }
    }
    println!();
}

pub fn print_bus(bus: &system_profiler::USBBus, blocks: &DeviceBlocks, pad: &PrintPadding) {
    for b in blocks {
        if let Some(string) = b.format_bus_value(bus, pad) {
            print!("{} ", b.colour(&string));
        }
    }
    println!();
}

pub fn get_devices_padding_required(devices: &Vec<&system_profiler::USBDevice>) -> PrintPadding {
    let longest_name = devices.iter().max_by_key(|x| x.name.len());
    let longest_serial = devices
        .iter()
        .max_by_key(|x| x.serial_num.as_ref().unwrap_or(&String::new()).len());
    let longest_manufacturer = devices
        .iter()
        .max_by_key(|x| x.manufacturer.as_ref().unwrap_or(&String::new()).len());
    let longest_tree = devices
        .iter()
        .max_by_key(|x| x.location_id.tree_positions.len());

    PrintPadding {
        name: longest_name.map_or(0, |d| d.name.len()),
        serial: longest_serial.map_or(0, |d| d.serial_num.as_ref().unwrap_or(&String::new()).len()),
        manufacturer: longest_manufacturer.map_or(0, |d| {
            d.manufacturer.as_ref().unwrap_or(&String::new()).len()
        }),
        tree_positions: longest_tree.map_or(0, |d| d.location_id.tree_positions.len() * 2),
        ..Default::default()
    }
}

pub fn print_flattened_devices(
    devices: &Vec<&system_profiler::USBDevice>,
    db: &DeviceBlocks,
    settings: &PrintSettings,
) {
    let pad: PrintPadding = if !settings.no_padding {
        get_devices_padding_required(devices)
    } else {
        Default::default()
    };
    log::debug!("Flattened devices padding {:?}", pad);

    for device in devices {
        print_device(device, db, &pad);
    }
}

pub fn print_devices(
    devices: &Vec<system_profiler::USBDevice>,
    db: &DeviceBlocks,
    settings: &PrintSettings,
) {
    let pad: PrintPadding = if !settings.no_padding {
        let refs: Vec<&system_profiler::USBDevice> = devices.iter().map(|v| v).collect();
        get_devices_padding_required(&refs)
    } else {
        Default::default()
    };
    log::debug!("Print devices padding {:?}", pad);

    for device in devices {
        print_device(device, db, &pad);
        device
            .devices
            .as_ref()
            .map(|d| print_devices(d, db, settings));
    }
}

pub fn print_spdata(
    spdata: &system_profiler::SPUSBDataType,
    db: &DeviceBlocks,
    bb: &BusBlocks,
    settings: &PrintSettings,
) {
    let pad: PrintPadding = if !settings.no_padding {
        let longest_name = spdata.buses.iter().max_by_key(|x| x.name.len());
        let longest_host_controller = spdata.buses.iter().max_by_key(|x| x.host_controller.len());

        PrintPadding {
            name: longest_name.map_or(0, |d| d.name.len()),
            host_controller: longest_host_controller.map_or(0, |d| d.host_controller.len()),
            ..Default::default()
        }
    } else {
        Default::default()
    };
    log::debug!("SPUSBDataType padding {:?}", pad);

    for bus in &spdata.buses {
        print_bus(bus, bb, &pad);
        bus.devices.as_ref().map(|d| print_devices(d, db, settings));
    }
}
