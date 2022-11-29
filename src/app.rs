use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

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

/// Value to group [`USBDevice`]
#[derive(Default, Debug, ValueEnum, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Group {
    #[default]
    /// No grouping
    NoGroup,
    /// Group into buses with bus info as heading - like a flat tree
    Bus,
}

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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Attempt to maintain compatibility with lsusb output
    #[arg(short, long, default_value_t = false)]
    lsusb: bool,

    /// Dump USB device hierarchy as a tree
    #[arg(short, long, default_value_t = false)]
    tree: bool,

    /// Show only devices with the specified vendor and product ID numbers (in hexadecimal) in format VID:[PID]
    #[arg(short = 'd', long)]
    vidpid: Option<String>,

    /// Show only devices with specified device and/or bus numbers (in decimal) in format [[bus]:][devnum]
    #[arg(short, long)]
    show: Option<String>,

    /// Specify the blocks which will be displayed for each device and in what order
    #[arg(short, long, value_enum)]
    blocks: Option<Vec<DeviceBlocks>>,

    /// Specify the blocks which will be displayed for each bus and in what order
    #[arg(long, value_enum)]
    bus_blocks: Option<Vec<BusBlocks>>,

    /// Hide empty buses; those with no devices
    #[arg(long, default_value_t = false)]
    hide_buses: bool,

    /// Hide empty hubs; those with no devices
    #[arg(long, default_value_t = false)]
    hide_hubs: bool,

    /// Filter on string contained in name
    #[arg(long)]
    filter_name: Option<String>,

    /// Filter on string contained in serial
    #[arg(long)]
    filter_serial: Option<String>,

    /// Verbosity level: 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints
    #[arg(short = 'v', long, default_value_t = 0, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable coloured output, can also use NO_COLOR environment variable
    #[arg(short, long, default_value_t = false)]
    no_colour: bool,

    /// Disable padding to align blocks
    #[arg(long, default_value_t = false)]
    no_padding: bool,

    /// Show base16 values as base10 decimal instead
    #[arg(long, default_value_t = false)]
    decimal: bool,

    /// Sort devices by value
    #[arg(long, value_enum)]
    sort_devices: Option<Sort>,

    /// Sort devices by bus number
    #[arg(long, default_value_t = false)]
    sort_buses: bool,

    /// Group devices by value when listing
    #[arg(long, value_enum, default_value_t = Default::default())]
    group_devices: Group,

    /// Show block headings
    #[arg(long, default_value_t = false)]
    headings: bool,

    /// Output as json format after sorting, filters and tree settings are applied
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Force libusb mode on macOS rather than using system_profiler output
    #[arg(long, default_value_t = false)]
    force_libusb: bool,

    /// Turn debugging information on
    #[arg(short = 'D', long, action = clap::ArgAction::Count)]
    debug: u8,
}
