//! Where the magic happens for `cyme` binary!
#[cfg(not(feature = "watch"))]
use clap::Parser;
#[cfg(feature = "watch")]
use clap::{Parser, Subcommand};
use colored::*;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use simple_logger::SimpleLogger;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use terminal_size::terminal_size;

use cyme::config::Config;
use cyme::display::{self, Block, DeviceBlocks};
use cyme::error::{Error, ErrorKind, Result};
use cyme::lsusb;
use cyme::profiler;
use cyme::usb::BaseClass;

#[cfg(feature = "watch")]
mod watch;

const MAX_VERBOSITY: u8 = 4;

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
    filter_class: Option<BaseClass>,

    /// Verbosity level (repeat provides count): 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints; 4 prints everything and more blocks
    #[arg(short = 'v', long, default_value_t = 0, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Specify the blocks which will be displayed for each device and in what order. Supply arg multiple times or csv to specify multiple blocks.
    ///
    /// [default: bus-number,device-number,icon,vendor-id,product-id,name,serial,speed]
    #[arg(short, long, value_enum, value_delimiter = ',', num_args = 1..)]
    blocks: Option<Vec<display::DeviceBlocks>>,

    /// Specify the blocks which will be displayed for each bus and in what order. Supply arg multiple times or csv to specify multiple blocks.
    ///
    /// [default: port-path,name,host-controller,host-controller-device]
    #[arg(long, value_enum, value_delimiter = ',', num_args = 1..)]
    bus_blocks: Option<Vec<display::BusBlocks>>,

    /// Specify the blocks which will be displayed for each configuration and in what order. Supply arg multiple times or csv to specify multiple blocks.
    ///
    /// [default: number,icon-attributes,max-power,name]
    #[arg(long, value_enum, value_delimiter = ',', num_args = 1..)]
    config_blocks: Option<Vec<display::ConfigurationBlocks>>,

    /// Specify the blocks which will be displayed for each interface and in what order. Supply arg multiple times or csv to specify multiple blocks.
    ///
    /// [default: port-path,icon,alt-setting,base-class,sub-class]
    #[arg(long, value_enum, value_delimiter = ',', num_args = 1..)]
    interface_blocks: Option<Vec<display::InterfaceBlocks>>,

    /// Specify the blocks which will be displayed for each endpoint and in what order. Supply arg multiple times or csv to specify multiple blocks.
    ///
    /// [default: number,direction,transfer-type,sync-type,usage-type,max-packet-size]
    #[arg(long, value_enum, value_delimiter = ',', num_args = 1..)]
    endpoint_blocks: Option<Vec<display::EndpointBlocks>>,

    /// Operation to perform on the blocks supplied via --blocks, --bus-blocks, --config-blocks, --interface-blocks and --endpoint-blocks
    ///
    /// Default is 'new' for legacy reasons but 'add' is probably more useful
    #[arg(long, value_enum, default_value_t = display::BlockOperation::New)]
    block_operation: display::BlockOperation,

    /// Print more blocks by default at each verbosity
    ///
    /// Only works if --blocks,--x--blocks not supplied as args or in config
    #[arg(short, long, default_value_t = false)]
    more: bool,

    /// Sort devices operation
    ///
    /// [default: device-number]
    #[arg(long, value_enum)]
    sort_devices: Option<display::Sort>,

    /// Sort devices by bus number. If using any sort-devices other than no-sort, this happens automatically
    #[arg(long, default_value_t = false)]
    sort_buses: bool,

    /// Group devices by value when listing
    ///
    /// [default: no-group]
    #[arg(long, value_enum)]
    group_devices: Option<display::Group>,

    /// Hide empty buses when printing tree; those with no devices.
    // these are a bit confusing, could make value enum with hide_empty, hide...
    #[arg(long, default_value_t = false)]
    hide_buses: bool,

    /// Hide empty hubs when printing tree; those with no devices. When listing will hide hubs regardless of whether empty of not
    #[arg(long, default_value_t = false)]
    hide_hubs: bool,

    /// Show root hubs when listing; Linux only
    #[arg(long, default_value_t = false)]
    list_root_hubs: bool,

    /// Show base16 values as base10 decimal instead
    #[arg(long, default_value_t = false)]
    decimal: bool,

    /// Disable padding to align blocks - will cause --headings to become maligned
    #[arg(long, default_value_t = false)]
    no_padding: bool,

    /// Output coloring mode
    ///
    /// [default: auto]
    #[arg(long, value_enum, aliases = &["colour"])]
    color: Option<display::ColorWhen>,

    /// Disable coloured output, can also use NO_COLOR environment variable
    #[arg(long, default_value_t = false, hide = true, aliases = &["no_colour"])]
    no_color: bool,

    /// Output character encoding
    ///
    /// [default: glyphs]
    #[arg(long, value_enum)]
    encoding: Option<display::Encoding>,

    /// Disables icons and utf-8 characters
    #[arg(long, default_value_t = false, hide = true)]
    ascii: bool,

    /// Disables all Block icons by not using any IconTheme. Providing custom XxxxBlocks without any icons is a nicer way to do this
    #[arg(long, default_value_t = false, hide = true)]
    no_icons: bool,

    /// When to print icon blocks
    ///
    /// [default: auto]
    #[arg(long, value_enum, aliases = &["icon_when"])]
    icon: Option<display::IconWhen>,

    /// Show block headings
    #[arg(long, default_value_t = false)]
    headings: bool,

    /// Output as json format after sorting, filters and tree settings are applied; without -tree will be flattened dump of devices
    #[arg(long, default_value_t = false, overrides_with = "lsusb")]
    json: bool,

    /// Read from json output rather than profiling system
    #[arg(long)]
    from_json: Option<PathBuf>,

    /// Force pure libusb profiler on macOS rather than combining system_profiler output
    ///
    /// Has no effect on other platforms or when using nusb
    #[arg(short = 'F', long, default_value_t = false)]
    force_libusb: bool,

    /// Path to user config file to use for custom icons, colours and default settings
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,

    /// Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE
    #[arg(short = 'z', long, action = clap::ArgAction::Count)]
    // short -d taken by lsusb compat vid:pid
    debug: u8,

    /// Mask serial numbers with '*' or random chars
    #[arg(long)]
    mask_serials: Option<display::MaskSerial>,

    /// Generate cli completions and man page
    #[cfg(feature = "cli_generate")]
    #[arg(long, hide = true, exclusive = true)]
    gen: bool,

    /// Use the system_profiler command on macOS to get USB data
    ///
    /// If not using nusb this is the default for macOS, merging with libusb data for verbose output. nusb uses IOKit directly so does not use system_profiler by default
    #[arg(long, default_value_t = false)]
    system_profiler: bool,

    /// Watch sub-command
    #[cfg(feature = "watch")]
    #[command(subcommand)]
    command: Option<SubCommand>,
}

#[cfg(feature = "watch")]
#[derive(Subcommand, Debug, Serialize, Deserialize)]
enum SubCommand {
    /// Watch for USB devices being connected and disconnected
    Watch,
}

/// Print in bold red and exit with error
macro_rules! eprintexit {
    ($error:expr) => {
        // `stringify!` will convert the expression *as it is* into a string.
        eprintln!(
            "{}\n{}",
            "cyme encountered a runtime error:".bold().red(),
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
///
/// Args will override Config if set
fn merge_config(c: &mut Config, a: &Args) {
    c.lsusb |= a.lsusb;
    c.tree |= a.tree;
    c.more |= a.more;
    c.hide_buses |= a.hide_buses;
    c.hide_hubs |= a.hide_hubs;
    c.list_root_hubs |= a.list_root_hubs;
    c.decimal |= a.decimal;
    c.no_padding |= a.no_padding;
    c.ascii |= a.ascii;
    c.headings |= a.headings;
    c.force_libusb |= a.force_libusb;
    c.no_icons |= a.no_icons;
    c.no_color |= a.no_color;
    c.json |= a.json;
    // override group devices if passed
    if a.group_devices.is_some() {
        c.group_devices = a.group_devices;
    }
    if a.encoding.is_some() {
        c.encoding = a.encoding;
    }
    if a.sort_devices.is_some() {
        c.sort_devices = a.sort_devices;
    }
    if a.icon.is_some() {
        c.icon_when = a.icon;
    }
    if a.color.is_some() {
        c.color_when = a.color;
    }
    if a.mask_serials.is_some() {
        c.mask_serials = a.mask_serials;
    }
    c.sort_buses |= a.sort_buses;
    // take larger debug level
    c.verbose = c.verbose.max(a.verbose);
}

/// Parse the vidpid filter lsusb format: vid:Option<pid>
fn parse_vidpid(s: &str) -> Result<(Option<u16>, Option<u16>)> {
    let vid_split: Vec<&str> = s.split(':').collect();
    if vid_split.len() >= 2 {
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
                .get(1)
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
            &format!("Invalid device path {s}"),
        ))
    }
}

/// macOS can use system_profiler to get USB data and merge with libusb so separate function
#[cfg(target_os = "macos")]
fn get_system_profile_macos(config: &Config, args: &Args) -> Result<profiler::SystemProfile> {
    // if requested or only have libusb, use system_profiler and merge with libusb
    if args.system_profiler || !cfg!(feature = "nusb") {
        if !config.force_libusb
            && args.device.is_none() // device path requires extra
                && args.filter_class.is_none() // class filter requires extra
                && !((config.tree && config.lsusb) || config.verbose > 0 || config.more)
        {
            profiler::macos::get_spusb()
                .map_or_else(|e| {
                    // For non-zero return, report but continue in this case
                    if e.kind() == ErrorKind::SystemProfiler {
                        eprintln!("Failed to run 'system_profiler -json SPUSBDataType', fallback to cyme profiler; Error({e})");
                        get_system_profile(config, args)
                    } else {
                        Err(e)
                    }
                }, Ok)
        } else if !config.force_libusb {
            if cfg!(feature = "libusb") {
                log::warn!("Merging macOS system_profiler output with libusb for verbose data. Apple internal devices will not be obtained");
            }
            profiler::macos::get_spusb_with_extra().map_or_else(|e| {
                // For non-zero return, report but continue in this case
                if e.kind() == ErrorKind::SystemProfiler {
                    eprintln!("Failed to run 'system_profiler -json SPUSBDataType', fallback to cyme profiler; Error({e})");
                    get_system_profile(config, args)
                } else {
                    Err(e)
                }
            }, Ok)
        } else {
            get_system_profile(config, args)
        }
    } else {
        get_system_profile(config, args)
    }
}

/// Detects and switches between verbose profiler (extra) and normal profiler
fn get_system_profile(config: &Config, args: &Args) -> Result<profiler::SystemProfile> {
    if config.verbose > 0
        || config.tree
        || args.device.is_some()
        || config.lsusb
        || config.more
        || args.filter_class.is_some()
    // class filter requires extra
    {
        profiler::get_spusb_with_extra()
    } else {
        profiler::get_spusb()
    }
}

fn print_lsusb(
    sp_usb: &profiler::SystemProfile,
    device: &Option<String>,
    settings: &display::PrintSettings,
) -> Result<()> {
    // device specific overrides tree on lsusb
    if settings.tree && device.is_none() {
        if !cfg!(target_os = "linux") {
            log::warn!("Most of the data in a lsusb style tree is applicable to Linux only!");
        }
        lsusb::print_tree(sp_usb, settings)
    } else {
        // can't print verbose if not using libusb
        if !(cfg!(feature = "libusb") || cfg!(feature = "nusb"))
            && (settings.verbosity > 0 || device.is_some())
        {
            return Err(Error::new(ErrorKind::Unsupported, "nusb or libusb feature is required to do this, install with `cargo install --features nusb/libusb`"));
        }

        let devices = sp_usb.flattened_devices();
        // even though we filtered using filter.show and using prepare, keep this here because it will match the exact Linux dev path and exit error if it doesn't match like lsusb
        if let Some(dev_path) = &device {
            lsusb::dump_one_device(&devices, dev_path)?
        } else {
            lsusb::print(&devices, settings.verbosity > 0);
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
    println!("Generating CLI info to {outdir:?}");

    let mut app = Args::command();

    // completions
    let bin_name = app.get_name().to_string();
    generate_to(Bash, &mut app, &bin_name, &outdir).expect("Failed to generate Bash completions");
    generate_to(Fish, &mut app, &bin_name, &outdir).expect("Failed to generate Fish completions");
    generate_to(Zsh, &mut app, &bin_name, &outdir).expect("Failed to generate Zsh completions");
    generate_to(PowerShell, &mut app, &bin_name, &outdir)
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

fn load_config<P: AsRef<Path>>(path: Option<P>) -> Result<Config> {
    if let Some(p) = path {
        let config = Config::from_file(p);
        log::info!("Using user config {config:?}");
        config
    } else {
        Config::sys()
    }
}

/// Set log level
pub fn set_log_level(debug: u8) -> Result<()> {
    let mut builder = SimpleLogger::new();
    let mut env_levels: HashSet<(String, log::LevelFilter)> = HashSet::new();

    let global_level = match debug {
        0 => {
            env_levels.insert(("udevrs".to_string(), log::LevelFilter::Off));
            env_levels.insert(("nusb".to_string(), log::LevelFilter::Off));
            log::LevelFilter::Error
        }
        1 => {
            env_levels.insert(("udevrs".to_string(), log::LevelFilter::Warn));
            env_levels.insert(("nusb".to_string(), log::LevelFilter::Warn));
            env_levels.insert(("cyme".to_string(), log::LevelFilter::Info));
            log::LevelFilter::Error
        }
        2 => {
            env_levels.insert(("udevrs".to_string(), log::LevelFilter::Info));
            env_levels.insert(("nusb".to_string(), log::LevelFilter::Info));
            env_levels.insert(("cyme".to_string(), log::LevelFilter::Debug));
            log::LevelFilter::Error
        }
        3 => {
            env_levels.insert(("udevrs".to_string(), log::LevelFilter::Debug));
            env_levels.insert(("nusb".to_string(), log::LevelFilter::Debug));
            env_levels.insert(("cyme".to_string(), log::LevelFilter::Trace));
            log::LevelFilter::Error
        }
        _ => log::LevelFilter::Trace,
    };

    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        rust_log
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut split = s.split('=');
                let k = split.next().unwrap();
                let v = split.next().and_then(|s| s.parse().ok());
                (k.to_string(), v)
            })
            .filter(|(_, v)| v.is_some())
            .map(|(k, v)| (k, v.unwrap()))
            .for_each(|(k, v)| {
                env_levels.replace((k, v));
            });
    }

    for (k, v) in env_levels {
        builder = builder.with_module_level(&k, v);
    }

    builder
        .with_utc_timestamps()
        .with_level(global_level)
        .env()
        .init()
        .map_err(|e| {
            Error::new(
                ErrorKind::Other("logger"),
                &format!("Failed to set log level: {e}"),
            )
        })?;

    #[cfg(feature = "libusb")]
    profiler::libusb::set_log_level(debug);

    Ok(())
}

/// Merge with arg blocks with config blocks (or default if None) depending on BlockOperation
fn merge_blocks(config: &Config, args: &Args, settings: &mut display::PrintSettings) -> Result<()> {
    if let Some(blocks) = &args.blocks {
        let mut device_blocks = config.blocks.to_owned().unwrap_or(if settings.more {
            DeviceBlocks::default_blocks(true)
        } else if settings.tree {
            DeviceBlocks::default_device_tree_blocks()
        } else {
            DeviceBlocks::default_blocks(false)
        });
        args.block_operation.run(&mut device_blocks, blocks)?;
        settings.device_blocks = Some(device_blocks);
    }

    if let Some(blocks) = &args.bus_blocks {
        settings.bus_blocks = Some(args.block_operation.new_or_op(
            config.bus_blocks.to_owned(),
            blocks,
            settings.more,
        )?);
    }

    if let Some(blocks) = &args.config_blocks {
        settings.config_blocks = Some(args.block_operation.new_or_op(
            settings.config_blocks.to_owned(),
            blocks,
            settings.more,
        )?);
    }

    if let Some(blocks) = &args.interface_blocks {
        settings.interface_blocks = Some(args.block_operation.new_or_op(
            settings.interface_blocks.to_owned(),
            blocks,
            settings.more,
        )?);
    }

    if let Some(blocks) = &args.endpoint_blocks {
        settings.endpoint_blocks = Some(args.block_operation.new_or_op(
            settings.endpoint_blocks.to_owned(),
            blocks,
            settings.more,
        )?);
    }

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
    set_log_level(args.debug)?;

    let mut config = load_config(args.config.as_deref())?;

    // add any config ENV override
    if config.print_non_critical_profiler_stderr {
        std::env::set_var("CYME_PRINT_NON_CRITICAL_PROFILER_STDERR", "1");
    }

    // legacy arg, hidden but still support with new format
    if args.ascii {
        args.encoding = Some(display::Encoding::Ascii);
    }

    // legacy arg, hidden but still support with new format
    if args.no_color {
        args.color = Some(display::ColorWhen::Never);
    }

    if args.verbose >= MAX_VERBOSITY {
        args.more = true;
    }

    merge_config(&mut config, &args);

    // set the output colouring mode
    // display::print will check based on print settings but let's ensure
    match config.color_when {
        Some(display::ColorWhen::Always) => {
            env::set_var("NO_COLOR", "0");
            colored::control::set_override(true);
            config.no_color = false;
        }
        Some(display::ColorWhen::Never) => {
            // set env to be sure too
            env::set_var("NO_COLOR", "1");
            colored::control::set_override(false);
            config.no_color = true;
        }
        _ => (),
    };

    let mut spusb = if let Some(file_path) = args.from_json.clone() {
        match profiler::read_json_dump(&file_path) {
            Ok(s) => s,
            Err(e) => {
                log::warn!(
                    "Failed to read json dump, attempting as flattened with phony bus: Error({e})"
                );
                profiler::read_flat_json_to_phony_bus(&file_path)?
            }
        }
    } else {
        #[cfg(target_os = "macos")]
        {
            get_system_profile_macos(&config, &args)?
        }

        #[cfg(not(target_os = "macos"))]
        {
            get_system_profile(&config, &args)?
        }
    };

    let filter = if config.hide_hubs
        || config.hide_buses
        || args.vidpid.is_some()
        || args.show.is_some()
        || args.device.is_some()
        || args.filter_name.is_some()
        || args.filter_serial.is_some()
        || args.filter_class.is_some()
    {
        let mut f = profiler::Filter::new();

        if let Some(vidpid) = &args.vidpid {
            let (vid, pid) = parse_vidpid(vidpid.as_str()).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidArg,
                    &format!("Failed to parse vidpid '{vidpid}'; Error({e})"),
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
                        "Failed to parse devpath '{devpath}', should end with 'BUS/DEVNO'; Error({e})"
                    ),
                )
            })?;
            f.bus = bus;
            f.number = number;
        } else if let Some(show) = &args.show {
            let (bus, number) = parse_show(show.as_str()).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidArg,
                    &format!("Failed to parse show parameter '{show}'; Error({e})"),
                )
            })?;
            f.bus = bus;
            f.number = number;
        }

        // no need to unwrap as these are Option
        f.name = args.filter_name.clone();
        f.serial = args.filter_serial.clone();
        f.class = args.filter_class;
        f.exclude_empty_hub = config.hide_hubs;
        f.exclude_empty_bus = config.hide_buses;
        // exclude root hubs unless:
        // * lsusb compat (shows root_hubs)
        // * json - for --from-json support
        // * list_root_hubs - user wants to see root hubs in list
        f.no_exclude_root_hub = config.lsusb || config.json || config.list_root_hubs;

        Some(f)
    } else {
        // exclude root hubs (on Linux) unless:
        // * lsusb compat (shows root_hubs)
        // * json - for --from-json support
        // * list_root_hubs - user wants to see root hubs in list
        if cfg!(target_os = "linux") {
            Some(profiler::Filter {
                no_exclude_root_hub: (config.lsusb || config.json || config.list_root_hubs),
                ..Default::default()
            })
        } else {
            None
        }
    };

    // create print settings from config - merged with arg flags above
    let mut settings = config.print_settings();
    settings.terminal_size = terminal_size().map(|(w, h)| (w.0, h.0));
    merge_blocks(&config, &args, &mut settings)?;

    log::trace!("Returned system_profiler data\n\r{spusb:#?}");

    #[cfg(feature = "watch")]
    if matches!(args.command, Some(SubCommand::Watch)) {
        if settings.json {
            watch::watch_usb_devices_json(spusb, filter, settings)?;
        } else {
            watch::watch_usb_devices(spusb, filter, settings, config)?;
        }
        return Ok(());
    }

    display::prepare(&mut spusb, filter.as_ref(), &settings);

    if config.lsusb {
        print_lsusb(&spusb, &args.device, &settings)?;
    } else {
        // check and report if was looking for args.device
        if args.device.is_some() && !spusb.buses.iter().any(|b| b.is_empty()) {
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
