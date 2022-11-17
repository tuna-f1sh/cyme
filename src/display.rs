use clap::ValueEnum;
use colored::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::icon;
use crate::system_profiler;

#[non_exhaustive]
#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize)]
pub enum Blocks {
    BusNumber,
    PortNumber,
    DeviceNumber,
    BranchPosition,
    Icon,
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
            Blocks::Icon,
            Blocks::VendorID,
            Blocks::ProductID,
            Blocks::Name,
            Blocks::Serial,
            Blocks::Speed,
        ]
    }

    pub fn default_device_tree_blocks() -> DeviceBlocks {
        vec![
            Blocks::Icon,
            Blocks::Name,
            Blocks::Serial,
        ]
    }

    pub fn default_bus_blocks() -> BusBlocks {
        vec![
            Blocks::Name,
            Blocks::HostController,
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
            _ => s.normal(),
        }
    }

    pub fn device_icon(&self, d: &system_profiler::USBDevice) -> Option<String> {
        match self {
            // TODO separate icons for Vendor and Product; some can match from just vendor id like apple, microsoft others should be vendor and product lookup like harddisk etc.
            // make struct Icon with impl for get_vendor_icon, get_product_icon, get_global_icon (tree, usb device etc.) - can be merged with load from file
            // HashMap<String, String> where key is vendor base16, vendor:product base16 and global ref
            Blocks::Icon => match d.vendor_id {
                Some(v) => match v {
                    0x05ac => Some("\u{f179}".into()),          // apple 
                    0x045e => Some("\u{f871}".into()),          // microsoft 
                    0x1D6B => Some("\u{f17c}".into()),          // linux foundation 
                    0x1915 | 0x0483 => Some("\u{f5a2}".into()), // specialized 
                    0x091e => Some("\u{e2a6}".into()),          // garmin 
                    0x1d50 | 0x1366 => Some("\u{f188}".into()), // debuggers 
                    0x043e => Some("\u{f878}".into()),          // monitor 
                    0x0781 => Some("\u{f7c9}".into()),          // external disk 
                    _ => Some("\u{f287}".into()), // usb plug default
                    // _ => Some(" ".into()),
                },
                None => None,
            },
            _ => None,
        }
    }

    pub fn bus_icon(&self, d: &system_profiler::USBBus) -> Option<String> {
        match self {
            Blocks::Icon => match d.pci_vendor {
                Some(v) => match v {
                    0x8086 => Some("\u{f179}".into()),          // apple 
                    0x045e => Some("\u{f871}".into()),          // microsoft 
                    0x1D6B => Some("\u{f17c}".into()),          // linux foundation 
                    _ => Some("\u{f287}".into()), // usb plug default
                },
                _ => Some("\u{f287}".into()), // usb plug default
            },
            _ => None,
        }
    }

    pub fn format_base(&self, v: u16, settings: &PrintSettings) -> String {
        if settings.base10 {
            format!("{:6}", v)
        } else {
            format!("0x{:04x}", v)
        }
    }

    pub fn format_device_value(
        &self,
        d: &system_profiler::USBDevice,
        pad: &PrintPadding,
        settings: &PrintSettings,
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
                d.get_branch_position()
            )),
            Blocks::Icon => settings.icons.as_ref().map_or(None, |i| Some(i.get_device_icon(d))),
            Blocks::VendorID => Some(match d.vendor_id {
                Some(v) => self.format_base(v, settings),
                None => format!("{:>6}", "-"),
            }),
            Blocks::ProductID => Some(match d.product_id {
                Some(v) => self.format_base(v, settings),
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
        settings: &PrintSettings,
    ) -> Option<String> {
        match self {
            Blocks::BusNumber => Some(format!("{:3}", bus.get_bus_number())),
            Blocks::Icon => settings.icons.as_ref().map_or(None, |i| Some(i.get_bus_icon(bus))),
            Blocks::VendorID => Some(match bus.pci_vendor {
                Some(v) => self.format_base(v, settings),
                None => format!("{:>6}", "-"),
            }),
            Blocks::ProductID => Some(match bus.pci_device {
                Some(v) => self.format_base(v, settings),
                None => format!("{:>6}", "-"),
            }),
            Blocks::PCIRevision => Some(match bus.pci_revision {
                Some(v) => self.format_base(v, settings),
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

#[derive(Default, Debug, ValueEnum, Clone, Serialize, Deserialize)]
pub enum Sort {
    #[default]
    BranchPosition,
    DeviceNumber,
    PortNumber,
    NoSort,
}

impl Sort {
    pub fn sort_devices(&self, d: &Vec<system_profiler::USBDevice>) -> Vec<system_profiler::USBDevice> {
        let mut sorted = d.to_owned();
        match self {
            Sort::BranchPosition => sorted.sort_by_key(|d| d.get_branch_position()),
            Sort::DeviceNumber => sorted.sort_by_key(|d| d.location_id.number.unwrap_or(0)),
            Sort::PortNumber => sorted.sort_by_key(|d| d.location_id.port.unwrap_or(0)),
            _ => ()
        }

        sorted
    }

    pub fn sort_devices_ref<'a>(&self, d: &Vec<&'a system_profiler::USBDevice>) -> Vec<&'a system_profiler::USBDevice> {
        let mut sorted = d.to_owned();
        match self {
            Sort::BranchPosition => sorted.sort_by_key(|d| d.get_branch_position()),
            Sort::DeviceNumber => sorted.sort_by_key(|d| d.location_id.number.unwrap_or(0)),
            Sort::PortNumber => sorted.sort_by_key(|d| d.location_id.port.unwrap_or(0)),
            _ => ()
        }

        sorted
    }
}

#[derive(Debug, Default)]
pub struct PrintSettings {
    pub no_padding: bool,
    pub base10: bool,
    pub tree: bool,
    pub sort_devices: Sort,
    pub sort_buses: bool,
    pub icons: Option<icon::IconTheme>,
}

pub fn render_device(d: &system_profiler::USBDevice, blocks: &DeviceBlocks, pad: &PrintPadding, settings: &PrintSettings) -> Vec<String> {
    let mut ret = Vec::new();
    for b in blocks {
        if let Some(string) = b.format_device_value(d, pad, settings) {
            ret.push(format!("{}", b.colour(&string)));
        }
    }

    ret
}

pub fn render_bus(bus: &system_profiler::USBBus, blocks: &DeviceBlocks, pad: &PrintPadding, settings: &PrintSettings) -> Vec<String> {
    let mut ret = Vec::new();

    for b in blocks {
        if let Some(string) = b.format_bus_value(bus, pad, settings) {
           ret.push(format!("{} ", b.colour(&string)));
        }
    }

    ret
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

fn generate_tree_data(current_tree: &TreeData, branch_length: usize, index: usize, settings: &PrintSettings) -> TreeData {
    let mut pass_tree = current_tree.clone();

    pass_tree.prefix = if pass_tree.depth > 0 {
        if index + 1 != pass_tree.branch_length {
            // format!("{}{}", pass_tree.prefix, LINE)
            format!("{}{}", pass_tree.prefix, settings.icons.as_ref().map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeLine)))
        } else {
            format!("{}{}", pass_tree.prefix, settings.icons.as_ref().map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeBlank)))
        }
    } else {
        format!("{}", pass_tree.prefix)
    };

    pass_tree.depth += 1;
    pass_tree.branch_length = branch_length;
    pass_tree.trunk_index = index as u8;

    return pass_tree;
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

    let sorted = settings.sort_devices.sort_devices_ref(&devices);

    for device in sorted {
        println!("{}", render_device(device, db, &pad, settings).join(" "));
    }
}

/// Passed to print functions to support tree building
#[derive(Debug, Default, Clone)]
pub struct TreeData {
    /// Length of the branch sitting on
    branch_length: usize,
    /// Index within parent list of devices
    trunk_index: u8,
    /// Depth of tree being built - normally len() tree_positions but might not be if printing inner
    depth: usize,
    /// Prefix to apply, builds up as depth increases
    prefix: String,
}

pub fn print_devices(
    devices: &Vec<system_profiler::USBDevice>,
    db: &DeviceBlocks,
    settings: &PrintSettings,
    tree: &TreeData,
) {
    let pad: PrintPadding = if !settings.no_padding {
        let refs: Vec<&system_profiler::USBDevice> = devices.iter().map(|v| v).collect();
        get_devices_padding_required(&refs)
    } else {
        Default::default()
    };
    log::debug!("Print devices padding {:?}, tree {:?}", pad, tree);

    // sort so that can be ascending along branch
    let sorted = settings.sort_devices.sort_devices(&devices);

    for (i, device) in sorted.iter().enumerate() {
        // get current prefix based on if last in tree and whether we are within the tree
        let device_prefix = if tree.depth > 0 {
            if i + 1 != tree.branch_length {
                format!("{}{}", tree.prefix, settings.icons.as_ref().map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeEdge)))
            } else {
                format!("{}{}", tree.prefix, settings.icons.as_ref().map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeCorner)))
            }
        } else {
            format!("{}{}", tree.prefix, settings.icons.as_ref().map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeBlank)))
        };

        // print the device
        print!("{}{} ", device_prefix, settings.icons.as_ref().map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeDeviceTerminator)));
        println!("{}", render_device(device, db, &pad, settings).join(" "));

        match device.devices.as_ref() {
            Some(d) => {
                // and then walk down devices printing them too
                print_devices(&d, db, settings, &generate_tree_data(&tree, d.len(), i, settings));
            },
            None => ()
        }
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

    let base_tree = TreeData {
        ..Default::default()
    };
    log::debug!("SPUSBDataType settings, {:?}, padding {:?}, tree {:?}", settings, pad, base_tree);

    for (i, bus) in spdata.buses.iter().enumerate() {
        print!("{}{} ", base_tree.prefix, settings.icons.as_ref().map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeBusStart)));
        println!("{}", render_bus(bus, bb, &pad, settings).join(" "));

        match bus.devices.as_ref() {
            Some(d) => {
                // and then walk down devices printing them too
                print_devices(&d, db, settings, &generate_tree_data(&base_tree, d.len(), i, settings));
            },
            None => ()
        }

        println!();
    }
}
