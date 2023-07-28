//! List system USB buses and devices; a modern `lsusb` that attempts to maintain compatibility with, but also add new features.
//! Includes a macOS `system_profiler` parser module and `lsusb` for non-macOS systems/gathering more verbose information.
#![allow(dead_code)]
#![warn(missing_docs)]
use simple_logger::SimpleLogger;

#[macro_use]
extern crate lazy_static;
pub mod colour;
pub mod config;
pub mod display;
pub mod error;
pub mod icon;
pub mod lsusb;
pub mod system_profiler;
pub mod types;
#[cfg(target_os = "linux")]
#[cfg(feature = "udev")]
pub mod udev;
pub mod usb;

/// Set cyme module and binary log level
pub fn set_log_level(debug: u8) -> crate::error::Result<()> {
    match debug {
        // just use env if not passed
        0 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Error.to_level_filter())
            .env(),
        1 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Info.to_level_filter()),
        2 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Debug.to_level_filter()),
        3 | _ => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Trace.to_level_filter()),
    }
    .init()
    .map_err(|e| {
        crate::error::Error::new(
            crate::error::ErrorKind::Other("simple_logger"),
            &format!("Failed to set log level: {}", e),
        )
    })?;

    Ok(())
}

// run any Rust code as doctest
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
