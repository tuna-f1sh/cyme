//! Where the magic happens for `cyme` binary!
use clap::Parser;
use colored::*;
use std::env;
use std::io::{Error, ErrorKind};
// use lazy_static::lazy_static;
// use regex::Regex;

use cyme::display;
use cyme::icon::IconTheme;
use cyme::system_profiler;
#[cfg(feature = "libusb")]
use cyme::lsusb;

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

    /// Selects which device lsusb will examine - supplied as Linux /dev/bus/usb/BBB/DDD style path
    #[arg(short = 'D', long)]
    device: Option<String>,

    /// Filter on string contained in name
    #[arg(long)]
    filter_name: Option<String>,

    /// Filter on string contained in serial
    #[arg(long)]
    filter_serial: Option<String>,

    /// Verbosity level: 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints; 4 prints everything and all blocks
    #[arg(short = 'v', long, default_value_t = 0, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Specify the blocks which will be displayed for each device and in what order
    #[arg(short, long, value_enum)]
    blocks: Option<Vec<display::DeviceBlocks>>,

    /// Specify the blocks which will be displayed for each bus and in what order
    #[arg(long, value_enum)]
    bus_blocks: Option<Vec<display::BusBlocks>>,

    /// Specify the blocks which will be displayed for each configuration and in what order
    #[arg(long, value_enum)]
    config_blocks: Option<Vec<display::ConfigurationBlocks>>,

    /// Specify the blocks which will be displayed for each interface and in what order
    #[arg(long, value_enum)]
    interface_blocks: Option<Vec<display::InterfaceBlocks>>,

    /// Specify the blocks which will be displayed for each endpoint and in what order
    #[arg(long, value_enum)]
    endpoint_blocks: Option<Vec<display::EndpointBlocks>>,

    /// Sort devices by value
    #[arg(long, value_enum)]
    sort_devices: Option<display::Sort>,

    /// Sort devices by bus number
    #[arg(long, default_value_t = false)]
    sort_buses: bool,

    /// Group devices by value when listing
    #[arg(long, value_enum, default_value_t = Default::default())]
    group_devices: display::Group,

    /// Hide empty buses; those with no devices
    #[arg(long, default_value_t = false)]
    hide_buses: bool,

    /// Hide empty hubs; those with no devices
    #[arg(long, default_value_t = false)]
    hide_hubs: bool,

    /// Show base16 values as base10 decimal instead
    #[arg(long, default_value_t = false)]
    decimal: bool,

    /// Disable padding to align blocks
    #[arg(long, default_value_t = false)]
    no_padding: bool,

    /// Disable coloured output, can also use NO_COLOR environment variable
    #[arg(long, default_value_t = false)]
    no_colour: bool,

    /// Show block headings
    #[arg(long, default_value_t = false)]
    headings: bool,

    /// Output as json format after sorting, filters and tree settings are applied
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Force libusb mode on macOS rather than using system_profiler output
    #[arg(long, default_value_t = false)]
    force_libusb: bool,

    /// Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE
    #[arg(long, action = clap::ArgAction::Count)]
    debug: u8,
}

/// Print in bold red and exit with error
macro_rules! eprintexit {
    ($error:expr) => {
        // `stringify!` will convert the expression *as it is* into a string.
        eprintln!("{}", $error.to_string().bold().red());
        std::process::exit(1);
    };
}

/// Print in bold orange warning and log
#[allow(unused_macros)]
macro_rules! wprintln {
    ($error:expr) => {
        // `stringify!` will convert the expression *as it is* into a string.
        println!("{}", $error.to_string().bold().yellow());
        log::warn!($error)
    };
}

/// Parse the vidpid filter lsusb format: vid:Option<pid>
fn parse_vidpid(s: &str) -> Result<(Option<u16>, Option<u16>), Error> {
    if s.contains(":") {
        let vid_split: Vec<&str> = s.split(":").collect();
        let vid: Option<u16> = vid_split.first().filter(|v| v.len() > 0).map_or(Ok(None), |v| {
            u32::from_str_radix(v.trim().trim_start_matches("0x"), 16)
                .map(|v| Some(v as u16))
                .map_err(|e| Error::new(ErrorKind::Other, e))
        })?;
        let pid: Option<u16> = vid_split.last().filter(|v| v.len() > 0).map_or(Ok(None), |v| {
            u32::from_str_radix(v.trim().trim_start_matches("0x"), 16)
                .map(|v| Some(v as u16))
                .map_err(|e| Error::new(ErrorKind::Other, e))
        })?;

        Ok((vid, pid))
    } else {
        let vid: Option<u16> = u32::from_str_radix(s.trim().trim_start_matches("0x"), 16)
            .map(|v| Some(v as u16))
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok((vid, None))
    }
}

/// Parse the show Option<bus>:device lsusb format
fn parse_show(s: &str) -> Result<(Option<u8>, Option<u8>), Error> {
    if s.contains(":") {
        let split: Vec<&str> = s.split(":").collect();
        let bus: Option<u8> = split.first().filter(|v| v.len() > 0).map_or(Ok(None), |v| {
            v.parse::<u8>()
                .map(Some)
                .map_err(|e| Error::new(ErrorKind::Other, e))
        })?;
        let device = split.last().filter(|v| v.len() > 0).map_or(Ok(None), |v| {
            v.parse::<u8>()
                .map(Some)
                .map_err(|e| Error::new(ErrorKind::Other, e))
        })?;

        Ok((bus, device))
    } else {
        let device: Option<u8> = s
            .trim()
            .parse::<u8>()
            .map(Some)
            .map_err(|e| Error::new(ErrorKind::Other, e))
            .unwrap_or(None);

        Ok((None, device))
    }
}

/// Parse devpath supplied by --device into a show format
///
/// Could be a regex match r"^[\/|\w+\/]+(?'bus'\d{3})\/(?'devno'\d{3})$" but this saves another crate
fn parse_devpath(s: &str) -> Result<(Option<u8>, Option<u8>), Error> {
    if s.contains("/") {
        let split: Vec<&str> = s.split("/").collect();
        // second to last
        let bus: Option<u8> = split.get(split.len()-2).map_or(Ok(None), |v| {
            v.parse::<u8>()
                .map(Some)
                .map_err(|e| Error::new(ErrorKind::Other, e))
        })?;
        // last
        let device = split.last().map_or(Ok(None), |v| {
            v.parse::<u8>()
                .map(Some)
                .map_err(|e| Error::new(ErrorKind::Other, e))
        })?;

        Ok((bus, device))
    } else {
        Err(Error::new(ErrorKind::Other, format!("Invalid device path {}", s)))
    }
}

/// Abort with exit code before trying to call libusb feature if not present
fn abort_not_libusb() {
    if !cfg!(feature = "libusb") {
        eprintexit!(Error::new(
            ErrorKind::Other,
            "libusb feature is required to do this, install with `cargo install --features libusb`"
        ));
    }
}

fn main() {
    let mut args = Args::parse();

    cyme::set_log_level(args.debug).unwrap_or_else(|e| {
        eprintexit!(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to configure logging: {}", e)
        ));
    });

    // just set the env for this process
    if args.no_colour {
        env::set_var("NO_COLOR", "1");
    }

    if args.json && args.lsusb {
        eprintln!("Disabling --lsusb flag because --json flag present");
        args.lsusb = false;
    }

    // TODO use use system_profiler but merge with extra from libusb for verbose to retain Apple buses which libusb cannot list
    let mut sp_usb = if cfg!(target_os = "macos") 
        && !args.force_libusb
        && args.device.is_none() // device path requires extra
        && !((args.tree && args.lsusb) || args.verbose > 0) {
        system_profiler::get_spusb().unwrap_or_else(|e| {
            eprintexit!(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to parse system_profiler output: {}", e)
            ));
        })
    } else {
        if cfg!(target_os = "macos") && !args.force_libusb {
            log::warn!("Forcing libusb for supplied arguments on macOS");
            args.force_libusb = true;
        }
        abort_not_libusb();
        // TODO this won't compile without udev due to no return with these compiled out...
        #[cfg(feature = "libusb")]
        lsusb::set_log_level(args.debug);
        #[cfg(feature = "libusb")]
        // verbose, tree and devpath require extra data
        lsusb::get_spusb(args.verbose > 0 || args.tree || args.device.is_some()).unwrap_or_else(|e| {
            eprintexit!(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to gather system USB data: {}", e)
            ));
        })
    };

    log::trace!("Returned system_profiler data\n\r{:#}", sp_usb);

    let filter = if args.hide_hubs
        || args.vidpid.is_some()
        || args.show.is_some()
        || args.device.is_some()
        || args.filter_name.is_some()
        || args.filter_serial.is_some()
    {
        let mut f = system_profiler::USBFilter::new();

        if let Some(vidpid) = &args.vidpid {
            let (vid, pid) = parse_vidpid(&vidpid.as_str()).unwrap_or_else(|e| {
                eprintexit!(Error::new(
                    ErrorKind::Other,
                    format!("Failed to parse vidpid '{}': {}", vidpid, e)
                ));
            });
            f.vid = vid;
            f.pid = pid;
        }

        // decode device devpath into the show filter since that is what it essentially will do
        if let Some(devpath) = &args.device {
            let (bus, number) = parse_devpath(&devpath.as_str()).unwrap_or_else(|e| {
                eprintexit!(Error::new(
                    ErrorKind::Other,
                    format!("Failed to parse devpath '{}', should end with 'BUS/DEVNO': {}", devpath, e)
                ));
            });
            f.bus = bus;
            f.number = number;
        } else if let Some(show) = &args.show {
            let (bus, number) = parse_show(&show.as_str()).unwrap_or_else(|e| {
                eprintexit!(Error::new(
                    ErrorKind::Other,
                    format!("Failed to parse show parameter '{}': {}", show, e)
                ));
            });
            f.bus = bus;
            f.number = number;
        }

        // no need to unwrap as these are Option
        f.name = args.filter_name;
        f.serial = args.filter_serial;
        f.exclude_empty_hub = args.hide_hubs;
        // exclude root hubs unless dumping a list
        f.no_exclude_root_hub = args.lsusb || !(args.tree || args.group_devices == display::Group::Bus);

        Some(f)
    } else {
        // default filter with exlcude root_hubs on linux if printing new tree as they are buses in system_profiler
        // always include if lsusb compat
        if cfg!(target_os = "linux") {
            Some(system_profiler::USBFilter {
                no_exclude_root_hub: args.lsusb || !(args.tree || args.group_devices == display::Group::Bus),
                ..Default::default()
            })
        } else {
            None
        }
    };

    log::info!("Filtering with {:?}", filter);

    // no sort if just dumping because it looks wierd with buses out of order
    let sort_devices = match args.sort_devices {
        Some(v) => v,
        None => {
            if args.tree || args.group_devices != display::Group::NoGroup {
                display::Sort::default()
            } else {
                display::Sort::NoSort
            }
        }
    };

    let group_devies = if args.group_devices == display::Group::Bus && args.tree {
        eprintln!("--group-devices with --tree is ignored; will print as tree");
        display::Group::NoGroup
    } else {
        args.group_devices
    };

    let settings = display::PrintSettings {
        no_padding: args.no_padding,
        decimal: args.decimal,
        tree: args.tree,
        hide_buses: args.hide_buses,
        sort_devices,
        sort_buses: args.sort_buses,
        group_devices: group_devies,
        json: args.json,
        headings: args.headings,
        verbosity: args.verbose,
        device_blocks: args.blocks,
        bus_blocks: args.bus_blocks,
        config_blocks: args.config_blocks,
        interface_blocks: args.interface_blocks,
        endpoint_blocks: args.endpoint_blocks,
        icons: Some(IconTheme::new()),
        ..Default::default()
    };

    display::prepare(&mut sp_usb, filter, &settings);

    if args.lsusb {
        // device specific overrides tree on lsusb
        if args.tree && args.device.is_none() { 
            if !cfg!(target_os = "linux") {
                log::warn!("Most of the data in a lsusb style tree is applicable to Linux only!");
            }
            if !cfg!(feature = "udev") {
                log::warn!("Without udev, lsusb style tree content will not match lsusb: driver and syspath will be missing");
            }
            lsusb::print_tree(&sp_usb, &settings)
        } else {
            let devices = sp_usb.flatten_devices();
            // even though we filtered using filter.show and using prepare, keep this here because it will match the exact Linux dev path and exit error if it doesn't match like lsusb
            if let Some(dev_path) = args.device {
                lsusb::dump_one_device(&devices, dev_path).unwrap_or_else(|e| {
                    eprintexit!(std::io::Error::new(std::io::ErrorKind::Other, e));
                });
            } else {
                let sorted = settings.sort_devices.sort_devices_ref(&devices);
                lsusb::print(&sorted, args.verbose > 0);
            }
        }
    } else {
        if args.device.is_some() && !sp_usb.buses.iter().any(|b| b.has_devices()) {
            eprintexit!(std::io::Error::new(std::io::ErrorKind::Other, format!("Unable to find {:?}", args.device.unwrap())));
        }
        display::print(&mut sp_usb, &settings);
    }
}
