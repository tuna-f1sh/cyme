//! List system USB buses and devices; a modern `lsusb` that attempts to maintain compatibility with, but also add new features.
//!
//! # Examples
//!
//! Profile USB devices on cross-platform systems:
//!
//! ```no_run
//! use cyme::profiler;
//! let sp_usb = profiler::get_spusb().unwrap();
//! ```
//!
//! Profile USB devices with all extra descriptor data (requires opening devices) on cross-platform systems:
//!
//! ```no_run
//! use cyme::profiler;
//! let sp_usb = profiler::get_spusb_with_extra().unwrap();
//! ```
//!
//! It's often useful to then flatten this into a list of devices ([`profiler::Device`]):
//!
//! ```no_run
//! # use cyme::profiler;
//! # let sp_usb = profiler::get_spusb().unwrap();
//! // flatten since we don't care tree/buses
//! let devices = sp_usb.flattened_devices();
//!
//! for device in devices {
//!    format!("{}", device);
//! }
//! ```
//!
//! One can then print with the cyme display module:
//!
//! ```no_run
//! # use cyme::profiler;
//! # let sp_usb = profiler::get_spusb().unwrap();
//! # let devices = sp_usb.flattened_devices();
//! use cyme::display;
//! // print with default [`display::PrintSettings`]
//! display::DisplayWriter::default().print_flattened_devices(&devices, &display::PrintSettings::default());
//! ```
//!
//! The [`profiler::SystemProfile`] struct contains system [`profiler::Bus`]s, which contain [`profiler::Device`]s as a USB tree.
#![allow(dead_code)]
#![warn(missing_docs)]
use simple_logger::SimpleLogger;
use std::collections::HashSet;

pub mod colour;
pub mod config;
pub mod display;
pub mod error;
pub mod icon;
pub mod lsusb;
pub mod profiler;
pub mod types;
#[cfg(all(target_os = "linux", feature = "udev"))]
pub mod udev;
#[cfg(all(all(target_os = "linux", feature = "udevlib"), not(feature = "udev")))]
#[path = "udev_ffi.rs"]
pub mod udev;
pub mod usb;

/// Set cyme module and binary log level
/// TODO move from mod with bin feature for simple_logger, dependant can configure log in their own way
pub fn set_log_level(debug: u8) -> crate::error::Result<()> {
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
            crate::error::Error::new(
                crate::error::ErrorKind::Other("logger"),
                &format!("Failed to set log level: {}", e),
            )
        })?;

    Ok(())
}

// run any Rust code as doctest
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
