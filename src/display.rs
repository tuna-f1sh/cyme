//! Provides the main utilities to display USB types within this crate - primarily used by `cyme` binary.
use std::collections::HashMap;
use std::cmp;
use clap::ValueEnum;
use colored::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::icon;
use crate::system_profiler;
use crate::system_profiler::{USBBus, USBDevice};

/// Info that can be printed about a `USBDevice`
#[non_exhaustive]
#[derive(Debug, ValueEnum, Eq, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub enum DeviceBlocks {
    /// Number of bus device is attached
    BusNumber,
    /// Bus issued device number
    DeviceNumber,
    /// Position of device in parent branch
    BranchPosition,
    /// Linux style port path
    PortPath,
    /// Linux udev reported syspath
    SysPath,
    /// Icon based on VID/PID
    Icon,
    /// Unique vendor identifier - purchased from USB IF
    VendorID,
    /// Vendor unique product identifier
    ProductID,
    /// The device product name as reported in descriptor or using usb_ids if None
    Name,
    /// The device manufacturer as provided in descriptor or using usb_ids if None
    Manufacturer,
    /// Device serial string as reported by descriptor
    Serial,
    /// Advertised device capable speed
    Speed,
    /// Position along all branches back to trunk device
    TreePositions,
    /// macOS system_profiler only - actually bus current in mA not power!
    BusPower,
    /// macOS system_profiler only - actually bus current used in mA not power!
    BusPowerUsed,
    /// macOS system_profiler only - actually bus current used in mA not power!
    ExtraCurrentUsed,
    /// The device version
    BcdDevice,
    /// The supported USB version
    BcdUsb,
    /// USB device class
    ClassCode,
}

/// Info that can be printed about a `USBBus`
#[non_exhaustive]
#[derive(Debug, ValueEnum, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub enum BusBlocks {
    /// System bus number identifier
    BusNumber,
    /// Icon based on VID/PID
    Icon,
    /// Bus name from descriptor or usb_ids
    Name,
    /// Host Controller on macOS, vendor put here when using libusb
    HostController,
    /// Understood to be vendor ID - it is when using libusb
    PCIVendor,
    /// Understood to be product ID - it is when using libusb
    PCIDevice,
    /// Revsision of hardware
    PCIRevision,
    /// syspath style port path to bus, applicable to Linux only
    PortPath,
}

/// Intended to be `impl` by a xxxBlocks `enum`
pub trait Block<B, T> {
    /// List of default blocks to use for printing T
    fn default_blocks() -> Vec<Self>
    where
        Self: Sized;

    /// Creates a HashMap of B keys to usize of longest value for that key in the `d` Vec; values can then be padded to match this
    fn generate_padding(d: &Vec<&T>) -> HashMap<B, usize>;

    /// Colour the block String
    fn colour(&self, s: &String) -> ColoredString;

    /// Creates the heading for the block value, for use with the heading flag
    fn heading(&self, pad: &HashMap<B, usize>) -> String;

    /// Returns whether the value intended for the block is a String type
    fn value_is_string(&self) -> bool;

    /// Formats the value associated with the block into a display String
    fn format_value(&self, d: &T, pad: &HashMap<B, usize>, settings: &PrintSettings) -> Option<String>;

    /// Formats u16 values like VID as base16 or base10 depending on decimal setting
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

    fn generate_padding(d: &Vec<&system_profiler::USBDevice>) -> HashMap<Self, usize> {
        HashMap::from([
            (DeviceBlocks::Name, cmp::max(DeviceBlocks::Name.heading(&Default::default()).len(), d.iter().map(|d| d.name.len()).max().unwrap_or(0))),
            (DeviceBlocks::Serial, cmp::max(DeviceBlocks::Serial.heading(&Default::default()).len(), d.iter().map(|d| d.serial_num.as_ref().unwrap_or(&String::new()).len()).max().unwrap_or(0))),
            (DeviceBlocks::Manufacturer, cmp::max(DeviceBlocks::Manufacturer.heading(&Default::default()).len(), d.iter().map(|d| d.manufacturer.as_ref().unwrap_or(&String::new()).len()).max().unwrap_or(0))),
            (DeviceBlocks::TreePositions, cmp::max(DeviceBlocks::TreePositions.heading(&Default::default()).len(), d.iter().map(|d| d.location_id.tree_positions.len()).max().unwrap_or(0))),
            (DeviceBlocks::PortPath, cmp::max(DeviceBlocks::PortPath.heading(&Default::default()).len(), d.iter().map(|d| d.port_path().len()).max().unwrap_or(0))),
            (DeviceBlocks::ClassCode, cmp::max(DeviceBlocks::ClassCode.heading(&Default::default()).len(), d.iter().map(|d| d.class.as_ref().map_or(String::new(), |c| c.to_string()).len()).max().unwrap_or(0))),
        ])
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
        pad: &HashMap<Self, usize>,
        settings: &PrintSettings,
    ) -> Option<String> {
        match self {
            DeviceBlocks::BusNumber => Some(format!("{:3}", d.location_id.bus)),
            DeviceBlocks::DeviceNumber => Some(format!("{:3}", d.location_id.number)),
            DeviceBlocks::BranchPosition => Some(format!("{:3}", d.get_branch_position())),
            DeviceBlocks::PortPath => Some(format!("{:pad$}", d.port_path(), pad = pad.get(self).unwrap_or(&0))),
            DeviceBlocks::SysPath => Some(match d.extra.as_ref() {
                Some(e) => format!("{:pad$}", e.syspath.as_ref().unwrap_or(&format!("{:pad$}", "-", pad = pad.get(self).unwrap_or(&0))), pad = pad.get(self).unwrap_or(&0)),
                None => format!("{:pad$}", "-", pad = pad.get(self).unwrap_or(&0))
            }),
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
            DeviceBlocks::Name => Some(format!("{:pad$}", d.name, pad = pad.get(self).unwrap_or(&0))),
            DeviceBlocks::Manufacturer => Some(match d.manufacturer.as_ref() {
                Some(v) => format!("{:pad$}", v, pad = pad.get(self).unwrap_or(&0)),
                None => format!("{:pad$}", "-", pad = pad.get(self).unwrap_or(&0)),
            }),
            DeviceBlocks::Serial => Some(match d.serial_num.as_ref() {
                Some(v) => format!("{:pad$}", v, pad = pad.get(self).unwrap_or(&0)),
                None => format!("{:pad$}", "-", pad = pad.get(self).unwrap_or(&0)),
            }),
            DeviceBlocks::Speed => Some(match d.device_speed.as_ref() {
                Some(v) => format!("{:>10}", v.to_string()),
                None => format!("{:>10}", "-"),
            }),
            DeviceBlocks::TreePositions => Some(format!(
                "{:pad$}",
                format!("{:}", d.location_id.tree_positions.iter().format("╌")),
                pad = pad.get(self).unwrap_or(&0)
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
            DeviceBlocks::ClassCode => Some(match d.class.as_ref() {
                Some(v) => format!("{:pad$}", v, pad = pad.get(self).unwrap_or(&0)),
                None => format!("{:pad$}", "-", pad = pad.get(self).unwrap_or(&0)),
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
            DeviceBlocks::SysPath => s.bright_cyan(),
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

    fn heading(&self, pad: &HashMap<Self, usize>) -> String {
        match self {
            DeviceBlocks::BusNumber => "Bus".into(),
            DeviceBlocks::DeviceNumber => " # ".into(),
            DeviceBlocks::BranchPosition => "Prt".into(),
            DeviceBlocks::PortPath => format!("{:^pad$}", "PortPath", pad = 2 + pad.get(self).unwrap_or(&0)),
            DeviceBlocks::SysPath => format!("{:^pad$}", "SysPath", pad = 2 + pad.get(self).unwrap_or(&0)),
            DeviceBlocks::VendorID => format!("{:^6}", "VID"),
            DeviceBlocks::ProductID => format!("{:^6}", "PID"),
            DeviceBlocks::Name => format!("{:^pad$}", "Name", pad = pad.get(self).unwrap_or(&0)),
            DeviceBlocks::Manufacturer => {
                format!("{:^pad$}", "Manufacturer", pad = pad.get(self).unwrap_or(&0))
            }
            DeviceBlocks::Serial => format!("{:^pad$}", "Serial", pad = pad.get(self).unwrap_or(&0)),
            DeviceBlocks::Speed => format!("{:^10}", "Speed"),
            DeviceBlocks::TreePositions => format!("{:^pad$}", "TreePos", pad = pad.get(self).unwrap_or(&0)),
            // will be 000 mA = 6
            DeviceBlocks::BusPower => "BusPwr".into(),
            DeviceBlocks::BusPowerUsed => "PwrUsd".into(),
            DeviceBlocks::ExtraCurrentUsed => "PwrExr".into(),
            // 00.00 = 5
            DeviceBlocks::BcdDevice => "Dev V".into(),
            DeviceBlocks::BcdUsb => "USB V".into(),
            DeviceBlocks::ClassCode => "Class".into(),
            DeviceBlocks::Icon => " ".into(),
            // _ => "",
        }
    }
}

impl Block<BusBlocks, USBBus> for BusBlocks {
    fn default_blocks() -> Vec<BusBlocks> {
        vec![BusBlocks::Name, BusBlocks::HostController]
    }

    fn generate_padding(d: &Vec<&system_profiler::USBBus>) -> HashMap<Self, usize> {
        HashMap::from([
            (BusBlocks::Name, cmp::max(BusBlocks::Name.heading(&Default::default()).len(), d.iter().map(|d| d.name.len()).max().unwrap_or(0))),
            (BusBlocks::HostController, cmp::max(BusBlocks::HostController.heading(&Default::default()).len(), d.iter().map(|d| d.host_controller.len()).max().unwrap_or(0))),
            (BusBlocks::PortPath, cmp::max(BusBlocks::PortPath.heading(&Default::default()).len(), d.iter().map(|d| d.path().len()).max().unwrap_or(0))),
        ])
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
        pad: &HashMap<Self, usize>,
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
            BusBlocks::Name => Some(format!("{:pad$}", bus.name, pad = pad.get(self).unwrap_or(&0))),
            BusBlocks::HostController => Some(format!(
                "{:pad$}",
                bus.host_controller,
                pad = pad.get(self).unwrap_or(&0)
            )),
            BusBlocks::PortPath => Some(format!(
                "{:pad$}",
                bus.path(),
                pad = pad.get(self).unwrap_or(&0)
            )),
            // _ => None,
        }
    }

    fn heading(&self, pad: &HashMap<Self, usize>) -> String {
        match self {
            BusBlocks::BusNumber => "Bus".into(),
            BusBlocks::PortPath => "PortPath".into(),
            BusBlocks::PCIDevice => " PID ".into(),
            BusBlocks::PCIVendor => " VID ".into(),
            BusBlocks::PCIRevision => " Rev ".into(),
            BusBlocks::Name => format!("{:^pad$}", "Name", pad = pad.get(self).unwrap_or(&0)),
            BusBlocks::HostController => {
                format!("{:^pad$}", "Host Controller", pad = pad.get(self).unwrap_or(&0))
            }
            BusBlocks::Icon => " ".into(),
            // _ => "",
        }
    }
}

/// Value to sort [`USBDevice`]
#[derive(Default, Debug, ValueEnum, Clone, Serialize, Deserialize)]
pub enum Sort {
    #[default]
    /// Sort by position in parent branch
    BranchPosition,
    /// Sort by bus device number
    DeviceNumber,
    /// No sorting; whatever order it was parsed
    NoSort,
}

impl Sort {
    /// The clone and sort the [`USBDevice`]s `d`
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

    /// The clone and sort the references to [`USBDevice`]s `d`
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

/// Value to group [`USBDevice`]
#[derive(Default, Debug, ValueEnum, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Group {
    #[default]
    /// No grouping
    NoGroup,
    /// Group into buses with bus info as heading - like a flat tree
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

/// Formats each [`Block`] value shown from a device `d`
pub fn render_value<B, T>(
    d: &T,
    blocks: &Vec<impl Block<B, T>>,
    pad: &HashMap<B, usize>,
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

/// Renders the headings for each [`Block`] being shown
pub fn render_heading<B, T>(
    blocks: &Vec<impl Block<B, T>>,
    pad: &HashMap<B, usize>,
) -> Vec<String> {
    let mut ret = Vec::new();

    for b in blocks {
        ret.push(b.heading(pad).to_string())
    }

    ret
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
    let pad = if !settings.no_padding {
        DeviceBlocks::generate_padding(devices)
    } else {
        HashMap::new()
    };
    log::debug!("Flattened devices padding {:?}", pad);

    let sorted = settings.sort_devices.sort_devices_ref(&devices);

    if settings.headings {
        let heading = render_heading(db, &pad).join(" ");
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
    let pad: HashMap<BusBlocks, usize> = if !settings.no_padding {
        BusBlocks::generate_padding(&bus_devices.iter().map(|bd| bd.0).collect())
    } else {
        HashMap::new()
    };

    for (bus, devices) in bus_devices {
        if settings.headings {
            let heading = render_heading(bb, &pad).join(" ");
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
    let pad = if !settings.no_padding {
        DeviceBlocks::generate_padding(&devices.iter().map(|d| d).collect())
    } else {
        HashMap::new()
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
                let heading = render_heading(db, &pad).join(" ");
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

/// Print SPUSBDataType
pub fn print_sp_usb(
    sp_usb: &system_profiler::SPUSBDataType,
    db: &Vec<DeviceBlocks>,
    bb: &Vec<BusBlocks>,
    settings: &PrintSettings,
) {
    let base_tree = TreeData {
        ..Default::default()
    };

    let pad: HashMap<BusBlocks, usize> = if !settings.no_padding {
        BusBlocks::generate_padding(&sp_usb.buses.iter().map(|b| b).collect())
    } else {
        HashMap::new()
    };

    log::debug!(
        "SPUSBDataType settings, {:?}, padding {:?}, tree {:?}",
        settings,
        pad,
        base_tree
    );

    for (i, bus) in sp_usb.buses.iter().enumerate() {
        if settings.headings {
            let heading = render_heading(bb, &pad).join(" ");
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

/// Main cyme bin print function - changes mutable SPUSBDataType during print
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
