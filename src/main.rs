use clap::Parser;
use std::env;
use colored::*;
use std::io::{Error, ErrorKind};
use simple_logger::SimpleLogger;

mod system_profiler;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Attempt to maintain compatibility with lsusb output
    #[arg(short, long, default_value_t = false)]
    lsusb: bool,

    /// Disable coloured output, can also use NO_COLOR environment variable
    #[arg(short, long, default_value_t = false)]
    no_colour: bool,

    /// Classic dump the physical USB device hierarchy as a tree - currently styling is the same but content is not
    #[arg(short = 't', long, default_value_t = false)]
    lsusb_tree: bool,

    /// Modern dump the physical USB device hierarchy as a tree
    #[arg(short = 'T', long, default_value_t = true)]
    tree: bool,

    /// Show only devices with the specified vendor and product ID numbers (in hexadecimal) in format VID:[PID]
    #[arg(short = 'd', long)]
    vidpid: Option<String>,

    /// Show only devices with specified device and/or bus numbers (in decimal) in format [[bus]:][devnum]
    #[arg(short, long)]
    show: Option<String>,

    /// Increase verbosity (show descriptors) TODO
    // #[arg(short, long, default_value_t = false)]
    // verbose: bool,

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

fn main() {
    let args = Args::parse();

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

    let sp_usb = system_profiler::get_spusb().unwrap_or_else(|e| {
        eprintexit!(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to parse system_profiler output: {}", e)));
    });

    log::debug!("{:#?}", sp_usb);

    let mut filter = system_profiler::USBFilter {
        vid: None,
        pid: None,
        bus: None,
        port: None,
    };

    if let Some(vidpid) = &args.vidpid {
        let (vid, pid) = parse_vidpid(&vidpid.as_str());
        filter.vid = vid;
        filter.pid = pid;
    }

    if let Some(show) = &args.show {
        let (bus, port) = parse_show(&show.as_str()).unwrap_or_else(|e| {
            eprintexit!(Error::new(ErrorKind::Other, format!("Failed to parse show parameter: {}", e)));
        });
        filter.bus = bus;
        filter.port = port;
    }

    log::info!("{:?}", filter);

    if args.vidpid.is_some() || args.show.is_some() {
        let mut devs = sp_usb.get_all_devices();
        devs = filter.filter_devices_ref(devs);
        for d in devs {
            if args.lsusb {
                println!("{:}", d);
            } else {
                println!("{:#}", d);
            }
        }
    } else {
        if args.lsusb {
            if args.lsusb_tree {
                eprintln!("lsusb tree is styling only; content is not the same!");
                print!("{:+}", sp_usb);
            } else {
                print!("{:}", sp_usb);
            }
        } else {
            if args.tree {
                print!("{:+#}", sp_usb);
            } else {
                print!("{:#}", sp_usb);
            }
        }
    }
}
