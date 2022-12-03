//! Where the magic happens for `cyme` binary!
use std::fs;
use std::io::Read;
use std::env;
use std::io::{Error, ErrorKind};
use clap::Parser;
use colored::*;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[cfg(feature = "cli_generate")]
use clap_complete::generate_to;
#[cfg(feature = "cli_generate")]
use clap::CommandFactory;
#[cfg(feature = "cli_generate")]
use clap_complete::shells::*;
#[cfg(feature = "cli_generate")]
use cyme::icon::example;
#[cfg(feature = "cli_generate")]
use std::path::PathBuf;

use cyme::display;
use cyme::icon::IconTheme;
use cyme::colour::ColourTheme;
use cyme::system_profiler;
use cyme::lsusb;

#[derive(Parser, Debug, Default, Serialize, Deserialize)]
#[skip_serializing_none]
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

    /// Print more blocks by default at each verbosity
    #[arg(short, long, default_value_t = false)]
    more: bool,

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

    /// Disables icons and utf-8 charactors
    #[arg(long, default_value_t = false)]
    ascii: bool,

    /// Show block headings
    #[arg(long, default_value_t = false)]
    headings: bool,

    /// Output as json format after sorting, filters and tree settings are applied; without -tree will be flattened dump of devices
    #[arg(long, default_value_t = false, overrides_with="lsusb")]
    json: bool,

    /// Read from json output rather than profiling system - must use --tree json dump
    #[arg(long)]
    from_json: Option<String>,

    /// Force libusb profiler on macOS rather than using/combining system_profiler output
    #[arg(short='F', long, default_value_t = false)]
    force_libusb: bool,

    /// Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE
    #[arg(short = 'c', long, action = clap::ArgAction::Count)] // short -d taken by lsusb compat vid:pid
    debug: u8,

    /// Generate cli completions and man page
    #[arg(long, hide=true, exclusive=true)]
    gen: bool,
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
#[cfg(not(feature = "libusb"))]
fn get_libusb_spusb(_args: &Args) -> system_profiler::SPUSBDataType {
    eprintexit!(Error::new(ErrorKind::Other, "libusb feature is required to do this, install with `cargo install --features libusb`"));
}

#[cfg(not(feature = "libusb"))]
fn merge_libusb_spusb(_spdata: &mut system_profiler::SPUSBDataType, _args: &Args) -> () {
    eprintexit!(Error::new(ErrorKind::Other, "libusb feature is required to do this, install with `cargo install --features libusb`"));
}

#[cfg(feature = "libusb")]
fn get_libusb_spusb(args: &Args) -> system_profiler::SPUSBDataType {
    lsusb::profiler::set_log_level(args.debug);
    lsusb::profiler::get_spusb(args.verbose > 0 || args.tree || args.device.is_some() || args.lsusb || args.json || args.more).unwrap_or_else(|e| {
        eprintexit!(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to gather system USB data from libusb: Error({})", e)
        ));
    })
}

#[cfg(feature = "libusb")]
fn merge_libusb_spusb(spdata: &mut system_profiler::SPUSBDataType, args: &Args) -> () {
    lsusb::profiler::set_log_level(args.debug);
    lsusb::profiler::fill_spusb(spdata, true).unwrap_or_else(|e| {
        eprintexit!(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to gather system USB data from libusb: Error({})", e)
        ));
    })
}

fn print_lsusb(sp_usb: &system_profiler::SPUSBDataType, device: &Option<String>, settings: &display::PrintSettings) {
    // device specific overrides tree on lsusb
    if settings.tree && device.is_none() { 
        if !cfg!(target_os = "linux") {
            log::warn!("Most of the data in a lsusb style tree is applicable to Linux only!");
        }
        if !cfg!(feature = "udev") {
            log::warn!("Without udev, lsusb style tree content will not match lsusb: driver and syspath will be missing");
        }
        lsusb::display::print_tree(&sp_usb, &settings)
    } else {
        // can't print verbose if not using libusb
        if !cfg!(feature = "libusb") && (settings.verbosity > 0 || device.is_some()) {
            eprintexit!(Error::new(ErrorKind::Other, "libusb feature is required to do this, install with `cargo install --features libusb`"));
        }
        let devices = sp_usb.flatten_devices();
        // even though we filtered using filter.show and using prepare, keep this here because it will match the exact Linux dev path and exit error if it doesn't match like lsusb
        if let Some(dev_path) = &device {
            lsusb::display::dump_one_device(&devices, dev_path).unwrap_or_else(|e| {
                eprintexit!(std::io::Error::new(std::io::ErrorKind::Other, e));
            });
        } else {
            let sorted = settings.sort_devices.sort_devices_ref(&devices);
            lsusb::display::print(&sorted, settings.verbosity > 0);
        }
    }
}

/// Generates extra CLI information for packaging
#[cfg(feature = "cli_generate")]
#[cold]
fn print_man() -> Result<(), Error> {
    let outdir = std::env::var_os("BUILD_SCRIPT_DIR")
        .or_else(|| std::env::var_os("OUT_DIR"))
            .unwrap_or_else(|| "./doc".into());
    fs::create_dir_all(&outdir).unwrap();
    println!("Generating CLI info to {:?}", outdir);

    let mut app = Args::command();

    // completions
    let bin_name = "cyme";
    generate_to(Bash, &mut app, bin_name, &outdir).expect("Failed to generate Bash completions");
    generate_to(Fish, &mut app, bin_name, &outdir).expect("Failed to generate Fish completions");
    generate_to(Zsh, &mut app, bin_name, &outdir).expect("Failed to generate Zsh completions");
    generate_to(PowerShell, &mut app, bin_name, &outdir).expect("Failed to generate PowerShell completions");

    // man page
    let man = clap_mangen::Man::new(app);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(PathBuf::from(&outdir).join("cyme.1"), buffer)?;

    // TODO example config
    std::fs::write(PathBuf::from(&outdir).join("cyme_example_config.json"), serde_json::to_string_pretty(&example()).unwrap())?;

    Ok(())
}

/// Reads a json dump with serde deserializer
///
/// Must be a full tree including buses
fn read_json_dump(file_path: &str) -> Result<system_profiler::SPUSBDataType, Error> {
    let mut file = fs::File::options().read(true).open(file_path)?;

    let mut data = String::new();
    file.read_to_string(&mut data)?;

    let json_dump: system_profiler::SPUSBDataType =
        serde_json::from_str(&data).map_err(|e| Error::new(ErrorKind::Other, e))?;

    Ok(json_dump)
}

fn main() {
    let args = Args::parse();

    #[cfg(feature = "cli_generate")]
    if args.gen {
        print_man().expect("Failed to generate extra CLI material");
        std::process::exit(0);
    }

    // set the module debug level, will also check env if args.debug == 0
    cyme::set_log_level(args.debug).unwrap_or_else(|e| {
        eprintexit!(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to configure logging: Error({})", e)
        ));
    });

    let colours = if args.no_colour {
        // set env to be sure too
        env::set_var("NO_COLOR", "1");
        None
    } else {
        Some(ColourTheme::new())
    };

    let icons = if args.ascii {
        None
    } else {
        Some(IconTheme::new())
    };

    let mut spusb = if let Some(file_path) = args.from_json {
        read_json_dump(&file_path.as_str()).unwrap_or_else(|e| {
            eprintexit!(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to parse system_profiler dump: Error({})", e)
            ));
        })
    } else if cfg!(target_os = "macos") 
        && !args.force_libusb
        && args.device.is_none() // device path requires extra
        && !((args.tree && args.lsusb) || args.verbose > 0 || args.more) {
        system_profiler::get_spusb().unwrap_or_else(|e| {
            eprintexit!(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to parse system_profiler output: Error({})", e)
            ));
        })
    } else {
        // if not forcing libusb, get system_profiler and the merge with libusb
        if cfg!(target_os = "macos") && !args.force_libusb {
            let mut spdata = system_profiler::get_spusb().unwrap_or_else(|e| {
                eprintexit!(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to parse system_profiler output: Error({})", e)
                ));
            });
            log::warn!("Merging macOS system_profiler output with libusb for verbose data. Apple internal devices will not be obtained");
            merge_libusb_spusb(&mut spdata, &args);
            spdata
        } else {
            get_libusb_spusb(&args)
        }
    };

    log::trace!("Returned system_profiler data\n\r{:#?}", spusb);

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
                    format!("Failed to parse vidpid '{}': Error({})", vidpid, e)
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
                    format!("Failed to parse devpath '{}', should end with 'BUS/DEVNO': Error({})", devpath, e)
                ));
            });
            f.bus = bus;
            f.number = number;
        } else if let Some(show) = &args.show {
            let (bus, number) = parse_show(&show.as_str()).unwrap_or_else(|e| {
                eprintexit!(Error::new(
                    ErrorKind::Other,
                    format!("Failed to parse show parameter '{}': Error({})", show, e)
                ));
            });
            f.bus = bus;
            f.number = number;
        }

        // no need to unwrap as these are Option
        f.name = args.filter_name;
        f.serial = args.filter_serial;
        f.exclude_empty_hub = args.hide_hubs;
        // exclude root hubs unless dumping a list or json
        f.no_exclude_root_hub = args.lsusb || args.json || !(args.tree || args.group_devices == display::Group::Bus);

        Some(f)
    } else {
        // default filter with exlcude root_hubs on linux if printing new tree as they are buses in system_profiler
        // always include if lsusb compat
        if cfg!(target_os = "linux") {
            Some(system_profiler::USBFilter {
                no_exclude_root_hub: args.lsusb || args.json || !(args.tree || args.group_devices == display::Group::Bus),
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
        more: args.more,
        device_blocks: args.blocks,
        bus_blocks: args.bus_blocks,
        config_blocks: args.config_blocks,
        interface_blocks: args.interface_blocks,
        endpoint_blocks: args.endpoint_blocks,
        icons,
        colours,
        ..Default::default()
    };

    display::prepare(&mut spusb, filter, &settings);

    if args.lsusb {
        print_lsusb(&spusb, &args.device, &settings);
    } else {
        // check and report if was looking for args.device
        if args.device.is_some() && !spusb.buses.iter().any(|b| b.has_devices()) {
            eprintexit!(std::io::Error::new(std::io::ErrorKind::Other, format!("Unable to find {:?}", args.device.unwrap())));
        }
        display::print(&mut spusb, &settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_config() {
        let mut args = Args{ ..Default::default() };
        args.blocks = Some(vec![display::DeviceBlocks::BusNumber]);
        println!("{}", serde_json::to_string_pretty(&args).unwrap());
    }

    #[test]
    fn test_parse_vidpid() {
        assert_eq!(parse_vidpid("000A:0x000b").unwrap(), (Some(0x0A), Some(0x0b)));
        assert_eq!(parse_vidpid("000A:1").unwrap(), (Some(0x0A), Some(1)));
        assert_eq!(parse_vidpid("000A:").unwrap(), (Some(0x0A), None));
        assert_eq!(parse_vidpid("0x000A").unwrap(), (Some(0x0A), None));
        assert_eq!(parse_vidpid("dfg:sdfd").is_err(), true);
    }

    #[test]
    fn test_parse_show() {
        assert_eq!(parse_show("1").unwrap(), (None, Some(1)));
        assert_eq!(parse_show("1:124").unwrap(), (Some(1), Some(124)));
        assert_eq!(parse_show("1:").unwrap(), (Some(1), None));
        // too big
        assert_eq!(parse_show("55233:12323").is_err(), true);
        assert_eq!(parse_show("dfg:sdfd").is_err(), true);
    }

    #[test]
    fn test_parse_devpath() {
        assert_eq!(parse_devpath("/dev/bus/usb/001/003").unwrap(), (Some(1), Some(3)));
        assert_eq!(parse_devpath("/dev/bus/usb/004/003").unwrap(), (Some(4), Some(3)));
        assert_eq!(parse_devpath("/dev/bus/usb/004/3").unwrap(), (Some(4), Some(3)));
        assert_eq!(parse_devpath("004/3").unwrap(), (Some(4), Some(3)));
        assert_eq!(parse_devpath("004/").is_err(), true);
        assert_eq!(parse_devpath("sas/ssas").is_err(), true);
    }
}
