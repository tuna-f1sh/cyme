use clap::ValueEnum;
use colored::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::icon;
use crate::system_profiler;
use crate::system_profiler::{USBBus, USBDevice};

#[non_exhaustive]
#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize)]
pub enum DeviceBlocks {
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
    TreePositions,
    BusPower,
    BusPowerUsed,
    ExtraCurrentUsed,
    Bcd,
}

#[non_exhaustive]
#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize)]
pub enum BusBlocks {
    BusNumber,
    Icon,
    Name,
    HostController,
    PCIVendor,
    PCIDevice,
    PCIRevision,
}

pub trait Block<B, T> {
    fn default_blocks() -> Vec<Self> where Self: Sized;
    fn colour(&self, s: &String) -> ColoredString;

    fn format_value(
        &self,
        d: &T,
        pad: &PrintPadding,
        settings: &PrintSettings,
    ) -> Option<String>;

    fn format_base(v: u16, settings: &PrintSettings) -> String {
        if settings.base10 {
            format!("{:6}", v)
        } else {
            format!("0x{:04x}", v)
        }
    }
}

impl DeviceBlocks {
    pub fn default_device_tree_blocks() -> Vec<DeviceBlocks> {
        vec![DeviceBlocks::Icon, DeviceBlocks::Name, DeviceBlocks::Serial]
    }
}

impl Block<DeviceBlocks, USBDevice> for DeviceBlocks {
    fn default_blocks() -> Vec<DeviceBlocks> {
        vec![
            DeviceBlocks::BusNumber,
            DeviceBlocks::PortNumber,
            DeviceBlocks::Icon,
            DeviceBlocks::VendorID,
            DeviceBlocks::ProductID,
            DeviceBlocks::Name,
            DeviceBlocks::Serial,
            DeviceBlocks::Speed,
        ]
    }

    fn format_value(
        &self,
        d: &USBDevice,
        pad: &PrintPadding,
        settings: &PrintSettings,
    ) -> Option<String> {
        match self {
            DeviceBlocks::BusNumber => Some(format!("{:3}", d.location_id.bus)),
            DeviceBlocks::DeviceNumber => Some(match d.location_id.number {
                Some(v) => format!("{:3}", v),
                None => format!("{:>3}", "-"),
            }),
            DeviceBlocks::PortNumber => Some(match d.location_id.port {
                Some(v) => format!("{:3}", v),
                None => format!("{:>3}", "-"),
            }),
            DeviceBlocks::BranchPosition => Some(format!("{:3}", d.get_branch_position())),
            DeviceBlocks::Icon => settings
                .icons
                .as_ref()
                .map_or(None, |i| Some(i.get_device_icon(d))),
            DeviceBlocks::VendorID => Some(match d.vendor_id {
                Some(v) => Self::format_base(v, settings),
                None => format!("{:>6}", "-"),
            }),
            DeviceBlocks::ProductID => Some(match d.product_id {
                Some(v) => Self::format_base(v, settings),
                None => format!("{:>6}", "-"),
            }),
            DeviceBlocks::Name => Some(format!("{:pad$}", d.name.trim(), pad = pad.name)),
            DeviceBlocks::Manufacturer => Some(match d.manufacturer.as_ref() {
                Some(v) => format!("{:pad$}", v.trim(), pad = pad.manufacturer),
                None => format!("{:pad$}", "-", pad = pad.manufacturer),
            }),
            DeviceBlocks::Serial => Some(match d.serial_num.as_ref() {
                Some(v) => format!("{:pad$}", v.trim(), pad = pad.serial),
                None => format!("{:pad$}", "-", pad = pad.serial),
            }),
            DeviceBlocks::Speed => Some(match d.device_speed.as_ref() {
                Some(v) => format!("{:>10}", v.to_string()),
                None => format!("{:>10}", "-"),
            }),
            DeviceBlocks::TreePositions => Some(format!(
                "{:pad$}",
                format!("{:}", d.location_id.tree_positions.iter().format("╌")),
                pad = pad.tree_positions
            )),
            DeviceBlocks::BusPower => Some(match d.bus_power {
                Some(v) => format!("{:3} mA", v),
                None => format!("{:>6}", "-"),
            }),
            DeviceBlocks::BusPowerUsed => Some(match d.bus_power_used {
                Some(v) => format!("{:3} mA", v),
                None => format!("{:>6}", "-"),
            }),
            DeviceBlocks::ExtraCurrentUsed => Some(match d.extra_current_used {
                Some(v) => format!("{:3} mA", v),
                None => format!("{:>6}", "-"),
            }),
            DeviceBlocks::Bcd => Some(match d.bcd_device {
                Some(v) => format!("{:>5.2}", v),
                None => format!("{:>8}", "-"),
            }),
            // _ => None,
        }
    }


    fn colour(&self, s: &String) -> ColoredString {
        match self {
            DeviceBlocks::BusNumber => s.cyan(),
            DeviceBlocks::PortNumber => s.magenta(),
            DeviceBlocks::DeviceNumber => s.bright_magenta(),
            DeviceBlocks::BranchPosition => s.bright_magenta(),
            DeviceBlocks::VendorID => s.bold().yellow(),
            DeviceBlocks::ProductID => s.yellow(),
            DeviceBlocks::Name => s.bold().blue(),
            DeviceBlocks::Manufacturer => s.blue(),
            DeviceBlocks::Serial => s.green(),
            DeviceBlocks::Speed => s.purple(),
            DeviceBlocks::TreePositions => s.magenta(),
            DeviceBlocks::BusPower => s.purple(),
            DeviceBlocks::BusPowerUsed => s.bright_purple(),
            DeviceBlocks::ExtraCurrentUsed => s.red(),
            DeviceBlocks::Bcd => s.purple(),
            _ => s.normal(),
        }
    }
}

impl Block<BusBlocks, USBBus> for BusBlocks {
    fn default_blocks() -> Vec<BusBlocks> {
        vec![
            BusBlocks::Name,
            BusBlocks::HostController,
        ]
    }

    fn colour(&self, s: &String) -> ColoredString {
        match self {
            BusBlocks::BusNumber => s.cyan(),
            BusBlocks::PCIVendor => s.bold().yellow(),
            BusBlocks::PCIDevice => s.yellow(),
            BusBlocks::Name => s.bold().blue(),
            BusBlocks::HostController => s.green(),
            BusBlocks::PCIRevision => s.normal(),
            _ => s.normal(),
        }
    }

    fn format_value(
        &self,
        bus: &system_profiler::USBBus,
        pad: &PrintPadding,
        settings: &PrintSettings,
    ) -> Option<String> {
        match self {
            BusBlocks::BusNumber => Some(format!("{:3}", bus.get_bus_number())),
            BusBlocks::Icon => settings
                .icons
                .as_ref()
                .map_or(None, |i| Some(i.get_bus_icon(bus))),
            BusBlocks::PCIVendor => Some(match bus.pci_vendor {
                Some(v) => Self::format_base(v, settings),
                None => format!("{:>6}", "-"),
            }),
            BusBlocks::PCIDevice => Some(match bus.pci_device {
                Some(v) => Self::format_base(v, settings),
                None => format!("{:>6}", "-"),
            }),
            BusBlocks::PCIRevision => Some(match bus.pci_revision {
                Some(v) => Self::format_base(v, settings),
                None => format!("{:>6}", "-"),
            }),
            BusBlocks::Name => Some(format!("{:pad$}", bus.name, pad = pad.name)),
            BusBlocks::HostController => Some(format!(
                "{:pad$}",
                bus.host_controller,
                pad = pad.host_controller
            )),
            // _ => None,
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
    pub fn sort_devices(
        &self,
        d: &Vec<system_profiler::USBDevice>,
    ) -> Vec<system_profiler::USBDevice> {
        let mut sorted = d.to_owned();
        match self {
            Sort::BranchPosition => sorted.sort_by_key(|d| d.get_branch_position()),
            Sort::DeviceNumber => sorted.sort_by_key(|d| d.location_id.number.unwrap_or(0)),
            Sort::PortNumber => sorted.sort_by_key(|d| d.location_id.port.unwrap_or(0)),
            _ => (),
        }

        sorted
    }

    pub fn sort_devices_ref<'a>(
        &self,
        d: &Vec<&'a system_profiler::USBDevice>,
    ) -> Vec<&'a system_profiler::USBDevice> {
        let mut sorted = d.to_owned();
        match self {
            Sort::BranchPosition => sorted.sort_by_key(|d| d.get_branch_position()),
            Sort::DeviceNumber => sorted.sort_by_key(|d| d.location_id.number.unwrap_or(0)),
            Sort::PortNumber => sorted.sort_by_key(|d| d.location_id.port.unwrap_or(0)),
            _ => (),
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

pub fn render_device(
    d: &system_profiler::USBDevice,
    blocks: &Vec<DeviceBlocks>,
    pad: &PrintPadding,
    settings: &PrintSettings,
) -> Vec<String> {
    let mut ret = Vec::new();
    for b in blocks {
        if let Some(string) = b.format_value(d, pad, settings) {
            ret.push(format!("{}", b.colour(&string)));
        }
    }

    ret
}

pub fn render_bus(
    bus: &system_profiler::USBBus,
    blocks: &Vec<BusBlocks>,
    pad: &PrintPadding,
    settings: &PrintSettings,
) -> Vec<String> {
    let mut ret = Vec::new();

    for b in blocks {
        if let Some(string) = b.format_value(bus, pad, settings) {
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

fn generate_tree_data(
    current_tree: &TreeData,
    branch_length: usize,
    index: usize,
    settings: &PrintSettings,
) -> TreeData {
    let mut pass_tree = current_tree.clone();

    // get prefix from icons - maybe should cache these before build rather than lookup each time...
    pass_tree.prefix = if pass_tree.depth > 0 {
        if index + 1 != pass_tree.branch_length {
            // format!("{}{}", pass_tree.prefix, LINE)
            format!(
                "{}{}",
                pass_tree.prefix,
                settings
                    .icons
                    .as_ref()
                    .map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeLine))
            )
        } else {
            format!(
                "{}{}",
                pass_tree.prefix,
                settings
                    .icons
                    .as_ref()
                    .map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeBlank))
            )
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
    db: &Vec<DeviceBlocks>,
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
    db: &Vec<DeviceBlocks>,
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
                format!(
                    "{}{}",
                    tree.prefix,
                    settings
                        .icons
                        .as_ref()
                        .map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeEdge))
                )
            } else {
                format!(
                    "{}{}",
                    tree.prefix,
                    settings
                        .icons
                        .as_ref()
                        .map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeCorner))
                )
            }
        } else {
            format!(
                "{}{}",
                tree.prefix,
                settings
                    .icons
                    .as_ref()
                    .map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeBlank))
            )
        };

        // print the device
        print!(
            "{}{} ",
            device_prefix,
            settings.icons.as_ref().map_or(String::new(), |i| i
                .get_tree_icon(icon::Icon::TreeDeviceTerminator))
        );
        println!("{}", render_device(device, db, &pad, settings).join(" "));

        match device.devices.as_ref() {
            Some(d) => {
                // and then walk down devices printing them too
                print_devices(
                    &d,
                    db,
                    settings,
                    &generate_tree_data(&tree, d.len(), i, settings),
                );
            }
            None => (),
        }
    }
}

pub fn print_spdata(
    spdata: &system_profiler::SPUSBDataType,
    db: &Vec<DeviceBlocks>,
    bb: &Vec<BusBlocks>,
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
    log::debug!(
        "SPUSBDataType settings, {:?}, padding {:?}, tree {:?}",
        settings,
        pad,
        base_tree
    );

    for (i, bus) in spdata.buses.iter().enumerate() {
        print!(
            "{}{} ",
            base_tree.prefix,
            settings
                .icons
                .as_ref()
                .map_or(String::new(), |i| i.get_tree_icon(icon::Icon::TreeBusStart))
        );
        println!("{}", render_bus(bus, bb, &pad, settings).join(" "));

        match bus.devices.as_ref() {
            Some(d) => {
                // and then walk down devices printing them too
                print_devices(
                    &d,
                    db,
                    settings,
                    &generate_tree_data(&base_tree, d.len(), i, settings),
                );
            }
            None => (),
        }

        println!();
    }
}
