//! Where the magic happens for `cyme` binary!
use clap::Parser;
use colored::*;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::env;
use terminal_size::terminal_size;

use cyme::config::Config;
use cyme::display;
use cyme::error::{Error, ErrorKind, Result};
use cyme::lsusb;
use cyme::usb;
use cyme::system_profiler;
use cyme::usb::ClassCode;

#[derive(Parser, Debug, Default, Serialize, Deserialize)]
#[skip_serializing_none]
#[command(author, version, about, long_about = None, max_term_width=80)]
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

    /// Filter on USB class code
    #[arg(long)]
    filter_class: Option<ClassCode>,

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

    /// Hide empty buses when printing tree; those with no devices. When listing will hide Linux root_hubs
    // these are a bit confusing, could make value enum with hide_empty, hide...
    #[arg(long, default_value_t = false)]
    hide_buses: bool,

    /// Hide empty hubs when printing tree; those with no devices. When listing will hide hubs regardless of whether empty of not
    #[arg(long, default_value_t = false)]
    hide_hubs: bool,

    /// Show base16 values as base10 decimal instead
    #[arg(long, default_value_t = false)]
    decimal: bool,

    /// Disable padding to align blocks - will cause --headings to become maligned
    #[arg(long, default_value_t = false)]
    no_padding: bool,

    /// Output coloring mode
    #[arg(long, value_enum, default_value_t = display::ColorWhen::Always, aliases = &["colour"])]
    color: display::ColorWhen,

    /// Disable coloured output, can also use NO_COLOR environment variable
    #[arg(long, default_value_t = false, hide = true, aliases = &["no_colour"])]
    no_color: bool,

    /// Output charactor encoding
    #[arg(long, value_enum, default_value_t = display::Encoding::Glyphs)]
    encoding: display::Encoding,

    /// Disables icons and utf-8 charactors
    #[arg(long, default_value_t = false, hide = true)]
    ascii: bool,

    /// Disables all Block icons by not using any IconTheme. Providing custom XxxxBlocks without any icons is a nicer way to do this
    #[arg(long, default_value_t = false, hide = true)]
    no_icons: bool,

    /// When to print icon blocks
    #[arg(long, value_enum, default_value_t = display::IconWhen::Auto)]
    icon: display::IconWhen,

    /// Show block headings
    #[arg(long, default_value_t = false)]
    headings: bool,

    /// Output as json format after sorting, filters and tree settings are applied; without -tree will be flattened dump of devices
    #[arg(long, default_value_t = false, overrides_with = "lsusb")]
    json: bool,

    /// Read from json output rather than profiling system - must use --tree json dump
    #[arg(long)]
    from_json: Option<String>,

    /// Force libusb profiler on macOS rather than using/combining system_profiler output
    #[arg(short = 'F', long, default_value_t = false)]
    force_libusb: bool,

    /// Path to user config file to use for custom icons, colours and default settings
    #[arg(short = 'c', long)]
    config: Option<String>,

    /// Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE
    #[arg(short = 'z', long, action = clap::ArgAction::Count)]
    // short -d taken by lsusb compat vid:pid
    debug: u8,

    /// Mask serial numbers with '*' or random chars
    #[arg(long)]
    mask_serials: Option<display::MaskSerial>,

    /// Generate cli completions and man page
    #[arg(long, hide = true, exclusive = true)]
    gen: bool,
}

/// Print in bold red and exit with error
macro_rules! eprintexit {
    ($error:expr) => {
        // `stringify!` will convert the expression *as it is* into a string.
        eprintln!(
            "{}\n{}",
            "cyme encounted a runtime error:".bold().red(),
            $error.to_string().bold().red()
        );
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

/// Merges non-Option Config with passed `Args`
fn merge_config(c: &Config, a: &mut Args) {
    a.lsusb |= c.lsusb;
    a.tree |= c.tree;
    a.more |= c.more;
    a.hide_buses |= c.hide_buses;
    a.hide_hubs |= c.hide_hubs;
    a.decimal |= c.decimal;
    a.no_padding |= c.no_padding;
    a.ascii |= c.ascii;
    a.headings |= c.headings;
    a.force_libusb |= c.force_libusb;
    a.no_icons |= c.no_icons;
    if a.verbose == 0 {
        a.verbose = c.verbose;
    }
}

/// Parse the vidpid filter lsusb format: vid:Option<pid>
fn parse_vidpid(s: &str) -> Result<(Option<u16>, Option<u16>)> {
    if s.contains(':') {
        let vid_split: Vec<&str> = s.split(':').collect();
        let vid: Option<u16> =
            vid_split
                .first()
                .filter(|v| !v.is_empty())
                .map_or(Ok(None), |v| {
                    u32::from_str_radix(v.trim().trim_start_matches("0x"), 16)
                        .map(|v| Some(v as u16))
                        .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))
                })?;
        let pid: Option<u16> =
            vid_split
                .last()
                .filter(|v| !v.is_empty())
                .map_or(Ok(None), |v| {
                    u32::from_str_radix(v.trim().trim_start_matches("0x"), 16)
                        .map(|v| Some(v as u16))
                        .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))
                })?;

        Ok((vid, pid))
    } else {
        let vid: Option<u16> = u32::from_str_radix(s.trim().trim_start_matches("0x"), 16)
            .map(|v| Some(v as u16))
            .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))?;

        Ok((vid, None))
    }
}

/// Parse the show Option<bus>:device lsusb format
fn parse_show(s: &str) -> Result<(Option<u8>, Option<u8>)> {
    if s.contains(':') {
        let split: Vec<&str> = s.split(':').collect();
        let bus: Option<u8> = split
            .first()
            .filter(|v| !v.is_empty())
            .map_or(Ok(None), |v| {
                v.parse::<u8>()
                    .map(Some)
                    .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))
            })?;
        let device = split
            .last()
            .filter(|v| !v.is_empty())
            .map_or(Ok(None), |v| {
                v.parse::<u8>()
                    .map(Some)
                    .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))
            })?;

        Ok((bus, device))
    } else {
        let device: Option<u8> = s
            .trim()
            .parse::<u8>()
            .map(Some)
            .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))?;

        Ok((None, device))
    }
}

/// Parse devpath supplied by --device into a show format
///
/// Could be a regex match r"^[\/|\w+\/]+(?'bus'\d{3})\/(?'devno'\d{3})$" but this saves another crate
fn parse_devpath(s: &str) -> Result<(Option<u8>, Option<u8>)> {
    if s.contains('/') {
        let split: Vec<&str> = s.split('/').collect();
        // second to last
        let bus: Option<u8> = split.get(split.len() - 2).map_or(Ok(None), |v| {
            v.parse::<u8>()
                .map(Some)
                .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))
        })?;
        // last
        let device = split.last().map_or(Ok(None), |v| {
            v.parse::<u8>()
                .map(Some)
                .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))
        })?;

        Ok((bus, device))
    } else {
        Err(Error::new(
            ErrorKind::InvalidArg,
            &format!("Invalid device path {}", s),
        ))
    }
}

/// Abort with exit code before trying to call libusb feature if not present
#[cfg(not(feature = "libusb"))]
fn get_libusb_spusb(_args: &Args) -> Result<system_profiler::SPUSBDataType> {
    Err(Error::new(
        ErrorKind::Unsupported,
        "libusb feature is required to do this, install with `cargo install --features libusb`",
    ))
}

#[cfg(feature = "libusb")]
fn get_libusb_spusb(args: &Args, print_stderr: bool) -> Result<system_profiler::SPUSBDataType> {
    if args.verbose > 0
        || args.tree
        || args.device.is_some()
        || args.lsusb
        || args.json
        || args.more
        || args.filter_class.is_none()
    // class filter requires extra
    {
        usb::profiler::get_spusb_with_extra(print_stderr).map_err(|e| {
            Error::new(
                ErrorKind::LibUSB,
                &format!(
                    "Failed to gather system USB data with extra from libusb, Error({})",
                    e
                ),
            )
        })
    } else {
        usb::profiler::get_spusb(print_stderr).map_err(|e| {
            Error::new(
                ErrorKind::LibUSB,
                &format!("Failed to gather system USB data from libusb, Error({})", e),
            )
        })
    }
}

fn print_lsusb(
    sp_usb: &system_profiler::SPUSBDataType,
    device: &Option<String>,
    settings: &display::PrintSettings,
) -> Result<()> {
    // device specific overrides tree on lsusb
    if settings.tree && device.is_none() {
        if !cfg!(target_os = "linux") {
            log::warn!("Most of the data in a lsusb style tree is applicable to Linux only!");
        }
        if !cfg!(feature = "udev") {
            log::warn!("Without udev, lsusb style tree content will not match lsusb: driver and syspath will be missing");
        }
        lsusb::display::print_tree(sp_usb, settings)
    } else {
        // can't print verbose if not using libusb
        if !cfg!(feature = "libusb") && (settings.verbosity > 0 || device.is_some()) {
            return Err(Error::new(ErrorKind::Unsupported, "libusb feature is required to do this, install with `cargo install --features libusb`"));
        }

        let devices = sp_usb.flatten_devices();
        // even though we filtered using filter.show and using prepare, keep this here because it will match the exact Linux dev path and exit error if it doesn't match like lsusb
        if let Some(dev_path) = &device {
            lsusb::display::dump_one_device(&devices, dev_path)?
        } else {
            let sorted = settings.sort_devices.sort_devices_ref(&devices);
            lsusb::display::print(&sorted, settings.verbosity > 0);
        }
    };

    Ok(())
}

/// Generates extra CLI information for packaging
#[cfg(feature = "cli_generate")]
#[cold]
fn print_man() -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::generate_to;
    use clap_complete::shells::*;
    use std::fs;
    use std::path::PathBuf;

    let outdir = std::env::var_os("BUILD_SCRIPT_DIR")
        .or_else(|| std::env::var_os("OUT_DIR"))
        .unwrap_or_else(|| "./doc".into());
    fs::create_dir_all(&outdir)?;
    println!("Generating CLI info to {:?}", outdir);

    let mut app = Args::command();

    // completions
    let bin_name = "cyme";
    generate_to(Bash, &mut app, bin_name, &outdir).expect("Failed to generate Bash completions");
    generate_to(Fish, &mut app, bin_name, &outdir).expect("Failed to generate Fish completions");
    generate_to(Zsh, &mut app, bin_name, &outdir).expect("Failed to generate Zsh completions");
    generate_to(PowerShell, &mut app, bin_name, &outdir)
        .expect("Failed to generate PowerShell completions");

    // man page
    let man = clap_mangen::Man::new(app);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(PathBuf::from(&outdir).join("cyme.1"), buffer)?;

    // example config
    std::fs::write(
        PathBuf::from(&outdir).join("cyme_example_config.json"),
        serde_json::to_string_pretty(&Config::example())?,
    )?;

    Ok(())
}

fn cyme() -> Result<()> {
    let mut args = Args::parse();

    #[cfg(feature = "cli_generate")]
    if args.gen {
        print_man()?;
        std::process::exit(0);
    }

    // set the module debug level, will also check env if args.debug == 0
    cyme::set_log_level(args.debug)?;

    #[cfg(feature = "libusb")]
    usb::profiler::set_log_level(args.debug);

    let mut config = if let Some(path) = args.config.as_ref() {
        let config = Config::from_file(path)?;
        log::info!("Using user config {:?}", config);
        config
    } else {
        Config::sys()?
    };

    // add any config ENV override
    config.print_non_critical_profiler_stderr =
        std::env::var_os("CYME_PRINT_NON_CRITICAL_PROFILER_STDERR")
            .map_or(config.print_non_critical_profiler_stderr, |_| true);

    merge_config(&config, &mut args);

    // legacy arg, hidden but still suport with new format
    if args.no_color {
        args.color = display::ColorWhen::Never;
    }

    // set the output colouring
    let colours = match args.color {
        display::ColorWhen::Auto => {
            // colored crate manages coloring
            Some(config.colours)
        }
        display::ColorWhen::Always => {
            env::set_var("NO_COLOR", "0");
            colored::control::set_override(true);
            Some(config.colours)
        }
        display::ColorWhen::Never => {
            // set env to be sure too
            env::set_var("NO_COLOR", "1");
            colored::control::set_override(false);
            None
        }
    };

    // legacy arg, hidden but still suport with new format
    if args.ascii {
        args.encoding = display::Encoding::Ascii;
    }

    // support hidden no_icons arg
    let icons = if args.no_icons {
        // For the tree, the display crate falls back to the static defaults for the encoding
        None
    } else {
        // Default icons and any user supplied
        Some(config.icons)
    };

    let mut spusb = if let Some(file_path) = args.from_json {
        system_profiler::read_json_dump(file_path.as_str())?
    } else if cfg!(target_os = "macos") 
        && !args.force_libusb
        && args.device.is_none() // device path requires extra
        && args.filter_class.is_none() // class filter requires extra
        && !((args.tree && args.lsusb) || args.verbose > 0 || args.more)
    {
        system_profiler::get_spusb()
            .map_or_else(|e| {
                // For non-zero return, report but continue in this case
                if e.kind() == ErrorKind::SystemProfiler {
                    eprintln!("Failed to run 'system_profiler -json SPUSBDataType', fallback to pure libusb; Error({})", e);
                    get_libusb_spusb(&args, config.print_non_critical_profiler_stderr)
                // parsing error abort
                } else {
                    Err(e)
                }
            }, Ok)?
    } else {
        // if not forcing libusb, get system_profiler and the merge with libusb
        if cfg!(target_os = "macos") && !args.force_libusb {
            log::warn!("Merging macOS system_profiler output with libusb for verbose data. Apple internal devices will not be obtained");
            system_profiler::get_spusb_with_extra().map_or_else(|e| {
                // For non-zero return, report but continue in this case
                if e.kind() == ErrorKind::SystemProfiler {
                    eprintln!("Failed to run 'system_profiler -json SPUSBDataType', fallback to pure libusb; Error({})", e);
                    get_libusb_spusb(&args, config.print_non_critical_profiler_stderr)
                } else {
                    Err(e)
                }
            }, Ok)?
        } else {
            get_libusb_spusb(&args, config.print_non_critical_profiler_stderr)?
        }
    };

    log::trace!("Returned system_profiler data\n\r{:#?}", spusb);

    let filter = if args.hide_hubs
        || args.vidpid.is_some()
        || args.show.is_some()
        || args.device.is_some()
        || args.filter_name.is_some()
        || args.filter_serial.is_some()
        || args.filter_class.is_some()
    {
        let mut f = system_profiler::USBFilter::new();

        if let Some(vidpid) = &args.vidpid {
            let (vid, pid) = parse_vidpid(vidpid.as_str()).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidArg,
                    &format!("Failed to parse vidpid '{}'; Error({})", vidpid, e),
                )
            })?;
            f.vid = vid;
            f.pid = pid;
        }

        // decode device devpath into the show filter since that is what it essentially will do
        if let Some(devpath) = &args.device {
            let (bus, number) = parse_devpath(devpath.as_str()).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidArg,
                    &format!(
                        "Failed to parse devpath '{}', should end with 'BUS/DEVNO'; Error({})",
                        devpath, e
                    ),
                )
            })?;
            f.bus = bus;
            f.number = number;
        } else if let Some(show) = &args.show {
            let (bus, number) = parse_show(show.as_str()).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidArg,
                    &format!("Failed to parse show parameter '{}'; Error({})", show, e),
                )
            })?;
            f.bus = bus;
            f.number = number;
        }

        // no need to unwrap as these are Option
        f.name = args.filter_name;
        f.serial = args.filter_serial;
        f.class = args.filter_class;
        f.exclude_empty_hub = args.hide_hubs;
        // exclude root hubs unless dumping a list or json
        f.no_exclude_root_hub =
            args.lsusb || args.json || !(args.tree || args.group_devices == display::Group::Bus);

        Some(f)
    } else {
        // default filter with exlcude root_hubs on linux if printing new tree as they are buses in system_profiler
        // always include if lsusb compat
        if cfg!(target_os = "linux") {
            Some(system_profiler::USBFilter {
                no_exclude_root_hub: args.lsusb
                    || args.json
                    || !(args.tree || args.group_devices == display::Group::Bus),
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

    let group_devices = if args.group_devices == display::Group::Bus && args.tree {
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
        group_devices,
        json: args.json,
        headings: args.headings,
        verbosity: args.verbose,
        more: args.more,
        encoding: args.encoding,
        mask_serials: args.mask_serials.map_or(config.mask_serials, Some),
        device_blocks: args.blocks.map_or(config.blocks, Some),
        bus_blocks: args.bus_blocks.map_or(config.bus_blocks, Some),
        config_blocks: args.config_blocks.map_or(config.config_blocks, Some),
        interface_blocks: args.interface_blocks.map_or(config.interface_blocks, Some),
        endpoint_blocks: args.endpoint_blocks.map_or(config.endpoint_blocks, Some),
        icons,
        colours,
        max_variable_string_len: config.max_variable_string_len,
        auto_width: !config.no_auto_width,
        terminal_size: terminal_size(),
        icon_when: args.icon,
    };

    display::prepare(&mut spusb, filter, &settings);

    if args.lsusb {
        print_lsusb(&spusb, &args.device, &settings)?;
    } else {
        // check and report if was looking for args.device
        if args.device.is_some() && !spusb.buses.iter().any(|b| b.has_devices()) {
            return Err(Error::new(
                ErrorKind::NotFound,
                &format!("Unable to find device at {:?}", args.device.unwrap()),
            ));
        }
        display::print(&spusb, &settings);
    }

    Ok(())
}

fn main() {
    cyme().unwrap_or_else(|e| {
        eprintexit!(e);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[test]
    fn test_output_args() {
        let mut args = Args {
            ..Default::default()
        };
        args.blocks = Some(vec![display::DeviceBlocks::BusNumber]);
        println!("{}", serde_json::to_string_pretty(&args).unwrap());
    }

    #[test]
    fn test_parse_vidpid() {
        assert_eq!(
            parse_vidpid("000A:0x000b").unwrap(),
            (Some(0x0A), Some(0x0b))
        );
        assert_eq!(parse_vidpid("000A:1").unwrap(), (Some(0x0A), Some(1)));
        assert_eq!(parse_vidpid("000A:").unwrap(), (Some(0x0A), None));
        assert_eq!(parse_vidpid("0x000A").unwrap(), (Some(0x0A), None));
        assert!(parse_vidpid("dfg:sdfd").is_err());
    }

    #[test]
    fn test_parse_show() {
        assert_eq!(parse_show("1").unwrap(), (None, Some(1)));
        assert_eq!(parse_show("1:124").unwrap(), (Some(1), Some(124)));
        assert_eq!(parse_show("1:").unwrap(), (Some(1), None));
        // too big
        assert!(parse_show("55233:12323").is_err());
        assert!(parse_show("dfg:sdfd").is_err());
    }

    #[test]
    fn test_parse_devpath() {
        assert_eq!(
            parse_devpath("/dev/bus/usb/001/003").unwrap(),
            (Some(1), Some(3))
        );
        assert_eq!(
            parse_devpath("/dev/bus/usb/004/003").unwrap(),
            (Some(4), Some(3))
        );
        assert_eq!(
            parse_devpath("/dev/bus/usb/004/3").unwrap(),
            (Some(4), Some(3))
        );
        assert_eq!(parse_devpath("004/3").unwrap(), (Some(4), Some(3)));
        assert!(parse_devpath("004/").is_err());
        assert!(parse_devpath("sas/ssas").is_err());
    }
}
