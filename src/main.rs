///! Where the magic happens for `cyme` binary!
use clap::Parser;
use colored::*;
use simple_logger::SimpleLogger;
use std::env;
use std::io::{Error, ErrorKind};

use cyme::display;
use cyme::icon::IconTheme;
#[cfg(feature = "libusb")]
use cyme::lsusb;
use cyme::system_profiler;

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
    blocks: Option<Vec<display::DeviceBlocks>>,

    /// Specify the blocks which will be displayed for each bus and in what order
    #[arg(long, value_enum)]
    bus_blocks: Option<Vec<display::BusBlocks>>,

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
    sort_devices: Option<display::Sort>,

    /// Sort devices by bus number
    #[arg(long, default_value_t = false)]
    sort_buses: bool,

    /// Group devices by value when listing
    #[arg(long, value_enum, default_value_t = Default::default())]
    group_devices: display::Group,

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

macro_rules! eprintexit {
    ($error:expr) => {
        // `stringify!` will convert the expression *as it is* into a string.
        eprintln!("{}", $error.to_string().bold().red());
        std::process::exit(1);
    };
}

fn parse_vidpid(s: &str) -> (Option<u16>, Option<u16>) {
    if s.contains(":") {
        let vid_split: Vec<&str> = s.split(":").collect();
        let vid: Option<u16> = vid_split.first().filter(|v| v.len() > 0).map_or(None, |v| {
            u32::from_str_radix(v.trim().trim_start_matches("0x"), 16)
                .map(|v| Some(v as u16))
                .unwrap_or(None)
        });
        let pid: Option<u16> = vid_split.last().filter(|v| v.len() > 0).map_or(None, |v| {
            u32::from_str_radix(v.trim().trim_start_matches("0x"), 16)
                .map(|v| Some(v as u16))
                .unwrap_or(None)
        });

        (vid, pid)
    } else {
        let vid: Option<u16> = u32::from_str_radix(s.trim().trim_start_matches("0x"), 16)
            .map(|v| Some(v as u16))
            .unwrap_or(None);

        (vid, None)
    }
}

fn parse_show(s: &str) -> Result<(Option<u8>, Option<u8>), Error> {
    if s.contains(":") {
        let split: Vec<&str> = s.split(":").collect();
        // TODO this unwrap should return as the result but I struggle with all this chaining...
        let bus: Option<u8> = split.first().filter(|v| v.len() > 0).map_or(None, |v| {
            v.parse::<u8>()
                .map(Some)
                .map_err(|e| Error::new(ErrorKind::Other, e))
                .unwrap_or(None)
        });
        let device = split.last().filter(|v| v.len() > 0).map_or(None, |v| {
            v.parse::<u8>()
                .map(Some)
                .map_err(|e| Error::new(ErrorKind::Other, e))
                .unwrap_or(None)
        });

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

/// Abort with exit code before trying to call libusb feature if not present
fn abort_not_libusb() {
    if !cfg!(feature = "libusb") {
        eprintexit!(Error::new(
            ErrorKind::Other,
            "libusb feature is required to do this, install with `cargo install --features libusb`"
        ));
    }
}

fn print_flat_lsusb(
    devices: &Vec<&system_profiler::USBDevice>,
    filter: &Option<system_profiler::USBFilter>,
) {
    if let Some(f) = filter {
        for d in devices {
            if f.is_match(&d) {
                println!("{:}", d);
            }
        }
    } else {
        for d in devices {
            println!("{:}", d);
        }
    }
}

fn main() {
    let mut args = Args::parse();

    match args.debug {
        0 => (),
        // just use env if not passed
        // 0 => SimpleLogger::new()
        //     .with_utc_timestamps()
        //     .env()
        //     .init()
        //     .unwrap(),
        1 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Info.to_level_filter())
            .init()
            .unwrap(),
        2 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Debug.to_level_filter())
            .init()
            .unwrap(),
        3 | _ => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Trace.to_level_filter())
            .init()
            .unwrap(),
    }

    // just set the env for this process
    if args.no_colour {
        env::set_var("NO_COLOR", "1");
    }

    // TODO use use system_profiler but add extra from libusb for verbose
    let mut sp_usb = if cfg!(target_os = "macos") && !(args.force_libusb || args.verbose > 0) {
        system_profiler::get_spusb().unwrap_or_else(|e| {
            eprintexit!(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to parse system_profiler output: {}", e)
            ));
        })
    } else {
        if cfg!(target_os = "macos") {
            eprintln!("Forcing libusb use for verbose output on macOS");
            args.force_libusb = true;
        }
        abort_not_libusb();
        #[cfg(feature = "libusb")]
        lsusb::set_log_level(args.debug);
        #[cfg(feature = "libusb")]
        lsusb::get_spusb(args.verbose > 0).unwrap_or_else(|e| {
            eprintexit!(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to gather system USB data: {}", e)
            ));
        })
    };

    log::debug!("Returned system_profiler data\n\r{}", sp_usb);

    let filter = if args.hide_hubs
        || args.vidpid.is_some()
        || args.show.is_some()
        || args.filter_name.is_some()
        || args.filter_serial.is_some()
    {
        let mut f = system_profiler::USBFilter::new();

        if let Some(vidpid) = &args.vidpid {
            let (vid, pid) = parse_vidpid(&vidpid.as_str());
            f.vid = vid;
            f.pid = pid;
        }

        if let Some(show) = &args.show {
            let (bus, number) = parse_show(&show.as_str()).unwrap_or_else(|e| {
                eprintexit!(Error::new(
                    ErrorKind::Other,
                    format!("Failed to parse show parameter: {}", e)
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
        f.no_exclude_root_hub = !(args.tree || args.group_devices == display::Group::Bus);

        log::info!("Filtering with {:?}", f);
        Some(f)
    } else {
        Some(system_profiler::USBFilter {
          no_exclude_root_hub: !(args.tree || args.group_devices == display::Group::Bus),
          ..Default::default()
        })
    };

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

    let print_settings = display::PrintSettings {
        no_padding: args.no_padding,
        decimal: args.decimal,
        tree: args.tree,
        hide_buses: args.hide_buses,
        sort_devices,
        sort_buses: args.sort_buses,
        group_devices: group_devies,
        json: args.json,
        headings: args.headings,
        icons: Some(IconTheme::new()),
        ..Default::default()
    };

    // TODO verbose only supported by lsusb mode at the moment
    if args.verbose > 0 && !(args.lsusb || args.force_libusb) {
        eprintln!("Forcing '--lsusb' compatibility mode, supply --lsusb to avoid this");
        args.lsusb = true;
    }

    // TODO do this in main cyme_print so that sorting each is done too
    if args.lsusb {
        if args.tree { 
            eprintln!("lsusb compatible tree is styling only; content is not the same!");
            print!("{:+}", sp_usb);
        } else {
            if args.verbose > 0 {
                abort_not_libusb();
                #[cfg(feature = "libusb")]
                lsusb::lsusb_verbose(&filter).unwrap_or_else(|e| {
                    eprintexit!(Error::new(
                        ErrorKind::Other,
                        format!("Failed to use lsusb verbose mode: {}", e)
                    ));
                });
            } else {
                print_flat_lsusb(&sp_usb.flatten_devices(), &filter);
            }
        }
    } else {
        display::cyme_print(&mut sp_usb, filter, args.blocks, args.bus_blocks, &print_settings);
    }
}
