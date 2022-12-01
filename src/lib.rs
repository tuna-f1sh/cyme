//! List system USB buses and devices; a modern `lsusb` that attempts to maintain compatibility with, but also add new features. 
//! Includes a macOS `system_profiler` parser module and `lsusb` for non-macOS systems/gathering more verbose information.
#![allow(dead_code)]
#![warn(missing_docs)]
use std::io::Error;
use simple_logger::SimpleLogger;

#[macro_use]
extern crate lazy_static;
pub mod display;
pub mod icon;
pub mod system_profiler;
pub mod types;
pub mod usb;
pub mod lsusb;
#[cfg(target_os = "linux")]
#[cfg(feature = "udev")]
pub mod udev;

/// Set cyme module and binary log level
pub fn set_log_level(debug: u8) ->  Result<(), Error> {
    match debug {
        // just use env if not passed
        0 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Error.to_level_filter())
            .env()
            .init()
            .or(Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Failed to create logger"))))?,
        1 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Info.to_level_filter())
            .init()
            .or(Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Failed to create logger"))))?,
        2 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Debug.to_level_filter())
            .init()
            .or(Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Failed to create logger"))))?,
        3 | _ => SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(log::Level::Trace.to_level_filter())
            .init()
            .or(Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Failed to create logger"))))?,
    }

    Ok(())
}
