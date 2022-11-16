#![allow(dead_code)]
use clap::Parser;
use std::env;
use colored::*;
use std::io::{Error, ErrorKind};
use simple_logger::SimpleLogger;

mod app;
use cyme::system_profiler;
#[cfg(feature = "libusb")]
use cyme::lsusb;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Attempt to maintain compatibility with lsusb output
    #[arg(short, long, default_value_t = false)]
    lsusb: bool,

    /// Classic dump the physical USB device hierarchy as a tree - currently styling is the same but content is not
    #[arg(short = 't', long, default_value_t = false)]
    lsusb_tree: bool,

    /// Modern dump the physical USB device hierarchy as a tree
    #[arg(short = 'T', long, default_value_t = false)]
    tree: bool,

    /// Show only devices with the specified vendor and product ID numbers (in hexadecimal) in format VID:[PID]
    #[arg(short = 'd', long)]
    vidpid: Option<String>,

    /// Show only devices with specified device and/or bus numbers (in decimal) in format [[bus]:][devnum]
    #[arg(short, long)]
    show: Option<String>,

    /// Specify the blocks which will be displayed for each device and in what order
    #[arg(long, value_enum)]
    blocks: Option<Vec<app::Blocks>>,

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

    /// Classic increase verbosity (show descriptors)
    #[arg(short = 'v', long, default_value_t = false)]
    lsusb_verbose: bool,

    /// Disable coloured output, can also use NO_COLOR environment variable
    #[arg(short, long, default_value_t = false)]
    no_colour: bool,

    /// Disable padding to align blocks
    #[arg(long, default_value_t = false)]
    no_padding: bool,

    /// Show base16 values as base10 instead
    #[arg(long, default_value_t = false)]
    base10: bool,

    /// Output as json format after filters applied
    #[arg(long, default_value_t = false)]
    json: bool,

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
        let vid: Option<u16> = vid_split.first()
            .filter(|v| v.len() > 0)
            .map_or(None, |v| u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).map(|v| Some(v as u16)).unwrap_or(None));
        let pid: Option<u16> = vid_split.last()
            .filter(|v| v.len() > 0)
            .map_or(None, |v| u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).map(|v| Some(v as u16)).unwrap_or(None));

        (vid, pid)
    } else {
        let vid: Option<u16> = u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).map(|v| Some(v as u16)).unwrap_or(None);

        (vid, None)
    }
}

fn parse_show(s: &str) -> Result<(Option<u8>, Option<u8>), Error> {
    if s.contains(":") {
        let split: Vec<&str> = s.split(":").collect();
        // TODO this unwrap should return as the result but I struggle with all this chaining...
        let bus: Option<u8> = split.first()
            .filter(|v| v.len() > 0)
            .map_or(None, |v| v.parse::<u8>().map(Some).map_err(|e| Error::new(ErrorKind::Other, e)).unwrap_or(None));
        let device = split.last()
                .filter(|v| v.len() > 0)
                .map_or(None, |v| v.parse::<u8>().map(Some).map_err(|e| Error::new(ErrorKind::Other, e)).unwrap_or(None));

        Ok((bus, device))
    } else {
        let device: Option<u8> = s.trim().parse::<u8>().map(Some).map_err(|e| Error::new(ErrorKind::Other, e)).unwrap_or(None);

        Ok((None, device))
    }

}

/// Abort with exit code before trying to call libusb feature if not present
fn abort_not_libusb() {
    if !cfg!(feature = "libusb") {
        eprintexit!(Error::new(ErrorKind::Other, "libusb feature is required to do this, install with `cargo install --features libusb`"));
    }
}

fn print_flat_lsusb(devices: &Vec<&system_profiler::USBDevice>, filter: &Option<system_profiler::USBFilter>) {
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

    let mut sp_usb = system_profiler::get_spusb().unwrap_or_else(|e| {
        eprintexit!(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to parse system_profiler output: {}", e)));
    });
    log::debug!("{:#?}", sp_usb);

    let filter = if 
        args.hide_hubs ||
        args.vidpid.is_some() || 
        args.show.is_some() ||
        args.filter_name.is_some() ||
        args.filter_serial.is_some() {
        let mut f = system_profiler::USBFilter::new();

        if let Some(vidpid) = &args.vidpid {
            let (vid, pid) = parse_vidpid(&vidpid.as_str());
            f.vid = vid;
            f.pid = pid;
        }

        if let Some(show) = &args.show {
            let (bus, port) = parse_show(&show.as_str()).unwrap_or_else(|e| {
                eprintexit!(Error::new(ErrorKind::Other, format!("Failed to parse show parameter: {}", e)));
            });
            f.bus = bus;
            f.port = port;
        }

        // no need to unwrap as these are Option
        f.name = args.filter_name;
        f.serial = args.filter_serial;
        f.exclude_empty_hub = args.hide_hubs;

        log::info!("Filtering with {:?}", f);
        Some(f)
    } else {
        None
    };

    filter.as_ref().map_or((), |f| f.retain_buses(&mut sp_usb.buses));
    if args.hide_buses {
        sp_usb.buses.retain(|b| b.has_devices());
        // may still be empty hubs if the hub had an empty hub!
        if args.hide_hubs {
            sp_usb.buses.retain(|b| !b.has_empty_hubs());
        }
    }

    let print_settings = app::PrintSettings {
        no_padding: args.no_padding,
        base10: args.base10,
        ..Default::default()
    };

    // TODO verbose only supported by lsusb mode at the moment
    if args.lsusb_verbose && !args.lsusb {
        eprintln!("Forcing '--lsusb' compatibility mode, supply --lsusb to avoid this");
        args.lsusb = true;
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&sp_usb).unwrap());
    } else if !(args.lsusb_tree || args.tree) {
        // filter again on flattened tree because will have kept parent branches with previous
        let mut devs = sp_usb.flatten_devices();
        filter.as_ref().map_or((), |f| f.retain_flattened_devices_ref(&mut devs));
        let blocks = args.blocks.unwrap_or(app::Blocks::default_device_blocks());

        if args.lsusb {
            if args.lsusb_verbose {
                abort_not_libusb();
                #[cfg(feature = "libusb")]
                lsusb::lsusb_verbose(&filter).unwrap_or_else(|e| {
                    eprintexit!(Error::new(ErrorKind::Other, format!("Failed to use lsusb verbose mode: {}", e)));
                });
            } else {
                print_flat_lsusb(&devs, &filter);
            }
        } else {
            app::print_flattened_devices(&devs, &blocks, &print_settings);
        }
    } else {
        if args.lsusb {
            eprintln!("lsusb compatible tree is styling only; content is not the same!");
            print!("{:+}", sp_usb);
        } else {
            if args.tree {
                print!("{:+#}", sp_usb);
            } else {
                print!("{:#}", sp_usb);
            }
        }
    }
}
