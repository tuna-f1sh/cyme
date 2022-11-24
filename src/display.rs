///! Provides the main utilities to display USB types within this crate - primarily used by `cyme` binary.
use clap::ValueEnum;
use colored::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::icon;
use crate::system_profiler;
use crate::system_profiler::{USBBus, USBDevice};

/// Info that can be printed about a `USBDevice`
#[non_exhaustive]
#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize)]
pub enum DeviceBlocks {
    BusNumber,
    DeviceNumber,
    BranchPosition,
    PortPath,
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
    BcdDevice,
    BcdUsb,
}

/// Info that can be printed about a `USBBus`
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

/// Intended to be `impl` by a xxxBlocks `enum`
pub trait Block<B, T> {
    fn default_blocks() -> Vec<Self>
    where
        Self: Sized;
    fn colour(&self, s: &String) -> ColoredString;
    fn heading(&self, settings: &PrintSettings, pad: &PrintPadding) -> String;
    fn value_is_string(&self) -> bool;

    fn format_value(&self, d: &T, pad: &PrintPadding, settings: &PrintSettings) -> Option<String>;

    fn format_base(v: u16, settings: &PrintSettings) -> String {
        if settings.decimal {
            format!("{:6}", v)
        } else {
            format!("0x{:04x}", v)
        }
    }
}

impl DeviceBlocks {
    /// Default `DeviceBlocks` for tree printing are different to list, get them here
    pub fn default_device_tree_blocks() -> Vec<DeviceBlocks> {
        vec![
            DeviceBlocks::Icon,
            DeviceBlocks::PortPath,
            DeviceBlocks::Name,
            DeviceBlocks::Serial,
        ]
    }
}

impl Block<DeviceBlocks, USBDevice> for DeviceBlocks {
    fn default_blocks() -> Vec<DeviceBlocks> {
        vec![
            DeviceBlocks::BusNumber,
            DeviceBlocks::DeviceNumber,
            DeviceBlocks::Icon,
            DeviceBlocks::VendorID,
            DeviceBlocks::ProductID,
            DeviceBlocks::Name,
            DeviceBlocks::Serial,
            DeviceBlocks::Speed,
        ]
    }

    fn value_is_string(&self) -> bool {
        match self {
            DeviceBlocks::Name|DeviceBlocks::Serial|DeviceBlocks::PortPath|DeviceBlocks::Manufacturer => true,
            _ => false
        }
    }

    fn format_value(
        &self,
        d: &USBDevice,
        pad: &PrintPadding,
        settings: &PrintSettings,
    ) -> Option<String> {
        match self {
            DeviceBlocks::BusNumber => Some(format!("{:3}", d.location_id.bus)),
            DeviceBlocks::DeviceNumber => Some(format!("{:3}", d.location_id.number)),
            DeviceBlocks::BranchPosition => Some(format!("{:3}", d.get_branch_position())),
            DeviceBlocks::PortPath => Some(format!("{:pad$}", d.location_id.port_path(), pad = 2 + pad.tree_positions)),
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
            DeviceBlocks::Name => Some(format!("{:pad$}", d.name, pad = pad.name)),
            DeviceBlocks::Manufacturer => Some(match d.manufacturer.as_ref() {
                Some(v) => format!("{:pad$}", v, pad = pad.manufacturer),
                None => format!("{:pad$}", "-", pad = pad.manufacturer),
            }),
            DeviceBlocks::Serial => Some(match d.serial_num.as_ref() {
                Some(v) => format!("{:pad$}", v, pad = pad.serial),
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
            DeviceBlocks::BcdDevice => Some(match d.bcd_device {
                Some(v) => format!("{:>5.2}", v),
                None => format!("{:>8}", "-"),
            }),
            DeviceBlocks::BcdUsb => Some(match d.bcd_usb {
                Some(v) => format!("{:>5.2}", v),
                None => format!("{:>5}", "-"),
            }),
            // _ => None,
        }
    }

    fn colour(&self, s: &String) -> ColoredString {
        match self {
            DeviceBlocks::BusNumber => s.cyan(),
            DeviceBlocks::DeviceNumber => s.bright_magenta(),
            DeviceBlocks::BranchPosition => s.magenta(),
            DeviceBlocks::PortPath => s.cyan(),
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
            DeviceBlocks::BcdDevice => s.purple(),
            _ => s.normal(),
        }
    }

    fn heading(&self, _settings: &PrintSettings, pad: &PrintPadding) -> String {
        match self {
            DeviceBlocks::BusNumber => "Bus".into(),
            DeviceBlocks::DeviceNumber => " # ".into(),
            DeviceBlocks::BranchPosition => "Prt".into(),
            DeviceBlocks::PortPath => format!("{:^pad$}", "Path", pad = 2 + pad.tree_positions),
            DeviceBlocks::VendorID => format!("{:^6}", "VID"),
            DeviceBlocks::ProductID => format!("{:^6}", "PID"),
            DeviceBlocks::Name => format!("{:^pad$}", "Name", pad = pad.name),
            DeviceBlocks::Manufacturer => {
                format!("{:^pad$}", "Manufacturer", pad = pad.manufacturer)
            }
            DeviceBlocks::Serial => format!("{:^pad$}", "Serial", pad = pad.serial),
            DeviceBlocks::Speed => format!("{:^10}", "Speed"),
            DeviceBlocks::TreePositions => format!("{:^pad$}", "TreePos", pad = pad.tree_positions),
            // will be 000 mA = 6
            DeviceBlocks::BusPower => "BusPwr".into(),
            DeviceBlocks::BusPowerUsed => "PwrUsd".into(),
            DeviceBlocks::ExtraCurrentUsed => "PwrExr".into(),
            // 00.00 = 5
            DeviceBlocks::BcdDevice => "Dev V".into(),
            DeviceBlocks::BcdUsb => "USB V".into(),
            DeviceBlocks::Icon => " ".into(),
            // _ => "",
        }
    }
}

impl Block<BusBlocks, USBBus> for BusBlocks {
    fn default_blocks() -> Vec<BusBlocks> {
        vec![BusBlocks::Name, BusBlocks::HostController]
    }

    fn value_is_string(&self) -> bool {
        match self {
            BusBlocks::Name|BusBlocks::HostController => true,
            _ => false
        }
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

    fn heading(&self, _settings: &PrintSettings, pad: &PrintPadding) -> String {
        match self {
            BusBlocks::BusNumber => "Bus".into(),
            BusBlocks::PCIDevice => " PID ".into(),
            BusBlocks::PCIVendor => " VID ".into(),
            BusBlocks::PCIRevision => " Rev ".into(),
            BusBlocks::Name => format!("{:^pad$}", "Name", pad = pad.name),
            BusBlocks::HostController => {
                format!("{:^pad$}", "Host Controller", pad = pad.host_controller)
            }
            BusBlocks::Icon => " ".into(),
            // _ => "",
        }
    }
}

/// Structure passed when printing list of devices that provides inner device amount to pad values so that they all align
///
/// Requires parent device to fill with max length of each value in list of its devices
///
/// TODO convert to HashMap<Block<B,T>, usize> and populate string blocks at parent rather than having to manually extend this struct
#[derive(Debug, Default)]
pub struct PrintPadding {
    pub name: usize,
    pub manufacturer: usize,
    pub serial: usize,
    pub tree_positions: usize,
    pub host_controller: usize,
}

/// Value to sort `USBDevice`
#[derive(Default, Debug, ValueEnum, Clone, Serialize, Deserialize)]
pub enum Sort {
    #[default]
    BranchPosition,
    DeviceNumber,
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
            Sort::DeviceNumber => sorted.sort_by_key(|d| d.location_id.number),
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
            Sort::DeviceNumber => sorted.sort_by_key(|d| d.location_id.number),
            _ => (),
        }

        sorted
    }
}

/// Value to group `USBDevice`
#[derive(Default, Debug, ValueEnum, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Group {
    #[default]
    NoGroup,
    Bus,
}

/// Passed to printing functions allows default args
#[derive(Debug, Default)]
pub struct PrintSettings {
    /// Don't pad in order to align blocks
    pub no_padding: bool,
    /// Print in decimal not base16
    pub decimal: bool,
    /// No tree printing
    pub tree: bool,
    /// Hide empty buses
    pub hide_buses: bool,
    /// Sort devices
    pub sort_devices: Sort,
    /// Sort buses by bus number
    pub sort_buses: bool,
    /// Group devices
    pub group_devices: Group,
    /// Print headings for blocks
    pub headings: bool,
    /// Print as json
    pub json: bool,
    /// `IconTheme` to apply - None to not print any icons
    pub icons: Option<icon::IconTheme>,
}

pub fn render_value<B, T>(
    d: &T,
    blocks: &Vec<impl Block<B, T>>,
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

pub fn render_heading<B, T>(
    blocks: &Vec<impl Block<B, T>>,
    pad: &PrintPadding,
    settings: &PrintSettings,
) -> Vec<String> {
    let mut ret = Vec::new();

    for b in blocks {
        ret.push(b.heading(settings, pad).to_string())
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

    // get prefix from icons if tree - maybe should cache these before build rather than lookup each time...
    if settings.tree {
        pass_tree.prefix = if pass_tree.depth > 0 {
            if index + 1 != pass_tree.branch_length {
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
    }

    pass_tree.depth += 1;
    pass_tree.branch_length = branch_length;
    pass_tree.trunk_index = index as u8;

    return pass_tree;
}

/// Print `devices` `USBDevice` references without looking down each device's devices!
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

    if settings.headings {
        let heading = render_heading(db, &pad, settings).join(" ");
        println!("{}", heading);
        println!("{}", "\u{2508}".repeat(heading.len())); // ┈
    }

    for device in sorted {
        println!("{}", render_value(device, db, &pad, settings).join(" "));
    }
}

/// A way of printing a reference flattened `SPUSBDataType` rather than hard flatten
///
/// Prints each `&USBBus` and tuple pair `Vec<&USBDevice>`
pub fn print_bus_grouped(
    bus_devices: Vec<(&system_profiler::USBBus, Vec<&system_profiler::USBDevice>)>,
    db: &Vec<DeviceBlocks>,
    bb: &Vec<BusBlocks>,
    settings: &PrintSettings,
) {
    let pad: PrintPadding = Default::default();

    for (bus, devices) in bus_devices {
        if settings.headings {
            let heading = render_heading(bb, &pad, settings).join(" ");
            println!("{}", heading);
            println!("{}", "\u{2508}".repeat(heading.len())); // ┈
        }
        println!("{}", render_value(bus, bb, &pad, settings).join(" "));
        print_flattened_devices(&devices, db, settings);
        // new line for each group
        println!();
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

/// Recursively print `devices`; will call for each `USBDevice` devices if `Some`
///
/// Will draw tree if `settings.tree`, otherwise it will be flat
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
        if settings.tree {
            let device_prefix = if tree.depth > 0 {
                if i + 1 != tree.branch_length {
                    format!(
                        "{}{}",
                        tree.prefix,
                        settings
                            .icons
                            .as_ref()
                            .map_or(icon::get_default_tree_icon(icon::Icon::TreeEdge), |i| i
                                .get_tree_icon(icon::Icon::TreeEdge))
                    )
                } else {
                    format!(
                        "{}{}",
                        tree.prefix,
                        settings
                            .icons
                            .as_ref()
                            .map_or(icon::get_default_tree_icon(icon::Icon::TreeCorner), |i| i
                                .get_tree_icon(icon::Icon::TreeCorner))
                    )
                }
            // TODO print not from bus
            } else {
                format!(
                    "{}{}",
                    tree.prefix,
                    settings
                        .icons
                        .as_ref()
                        .map_or(icon::get_default_tree_icon(icon::Icon::TreeBlank), |i| i
                            .get_tree_icon(icon::Icon::TreeBlank))
                )
            };

            // TODO this is not nice with fix but .len() device_prefix is num bytes so not correct for utf-8, nor is chars().count()
            // maybe should just do once at start of bus
            if settings.headings && i == 0 {
                let heading = render_heading(db, &pad, settings).join(" ");
                println!("{:>spaces$}{} ", "", heading, spaces=4 * tree.depth);
                println!("{:>spaces$}{} ", "", "\u{2508}".repeat(heading.len()), spaces=4 * tree.depth); // ┈
            }
            // render and print tree if doing it
            print!(
                "{}{} ",
                device_prefix,
                settings
                    .icons
                    .as_ref()
                    .map_or(icon::get_default_tree_icon(icon::Icon::TreeCorner), |i| i
                        .get_tree_icon(icon::Icon::TreeDeviceTerminator))
            );
        }
        // print the device
        println!("{}", render_value(device, db, &pad, settings).join(" "));

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

pub fn print_sp_usb(
    sp_usb: &system_profiler::SPUSBDataType,
    db: &Vec<DeviceBlocks>,
    bb: &Vec<BusBlocks>,
    settings: &PrintSettings,
) {
    let pad: PrintPadding = if !settings.no_padding {
        let longest_name = sp_usb.buses.iter().max_by_key(|x| x.name.len());
        let longest_host_controller = sp_usb.buses.iter().max_by_key(|x| x.host_controller.len());

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

    for (i, bus) in sp_usb.buses.iter().enumerate() {
        if settings.headings {
            let heading = render_heading(bb, &pad, settings).join(" ");
            println!("{}", heading);
            println!("{}", "\u{2508}".repeat(heading.len())); // ┈
        }
        if settings.tree {
            print!(
                "{}{} ",
                base_tree.prefix,
                settings
                    .icons
                    .as_ref()
                    .map_or(icon::get_default_tree_icon(icon::Icon::TreeBusStart), |i| i
                        .get_tree_icon(icon::Icon::TreeBusStart))
            );
        }
        println!("{}", render_value(bus, bb, &pad, settings).join(" "));

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

        // separate bus groups with line
        println!();
    }
}

pub fn cyme_print(
    sp_usb: &mut system_profiler::SPUSBDataType,
    filter: Option<system_profiler::USBFilter>,
    db: Option<Vec<DeviceBlocks>>,
    bb: Option<Vec<BusBlocks>>,
    settings: &PrintSettings,
) {
    // if not printing tree, hard flatten now before filtering as filter will retain non-matching parents with matching devices in tree
    // but only do it if there is a filter, grouping by bus (which uses tree print without tree...) or json
    if !settings.tree && (filter.is_some() || settings.group_devices == Group::Bus || settings.json) {
        sp_usb.flatten();
    }

    // do the filter if present; will keep parents of matched devices even if they do not match
    filter
        .as_ref()
        .map_or((), |f| f.retain_buses(&mut sp_usb.buses));

    // hide any empty buses and hubs now we've filtered
    if settings.hide_buses {
        sp_usb.buses.retain(|b| b.has_devices());
        // may still be empty hubs if the hub had an empty hub!
        if let Some(f) = filter.as_ref() {
            if f.exclude_empty_hub {
                sp_usb.buses.retain(|b| !b.has_empty_hubs());
            }
        }
    }

    // sort the buses if asked
    if settings.sort_buses {
        sp_usb.buses.sort_by_key(|d| d.get_bus_number());
    }

    // default blocks or those passed
    let bus_blocks = bb.unwrap_or(Block::<BusBlocks, system_profiler::USBBus>::default_blocks());

    if settings.tree || settings.group_devices == Group::Bus {
        let device_blocks = db.unwrap_or(DeviceBlocks::default_device_tree_blocks());
        if settings.json {
            println!("{}", serde_json::to_string_pretty(&sp_usb).unwrap());
        } else {
            print_sp_usb(sp_usb, &device_blocks, &bus_blocks, settings);
        }
    } else {
        let device_blocks = db.unwrap_or(DeviceBlocks::default_blocks());
        match settings.group_devices {
            // completely flatten the bus and only print devices
            _ => {
                // get a list of all devices
                let devs = sp_usb.flatten_devices();

                if settings.json {
                    println!("{}", serde_json::to_string_pretty(&devs).unwrap());
                } else {
                    print_flattened_devices(&devs, &device_blocks, settings);
                }
            }
        }
    }
}
