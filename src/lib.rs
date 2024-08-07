//! List system USB buses and devices; a modern `lsusb` that attempts to maintain compatibility with, but also add new features.
//! Includes a macOS `system_profiler` parser module and `lsusb` for non-macOS systems/gathering more verbose information.
//!
//! # Examples
//!
//! To get all the USB devices on cross-platform systems using libusb:
//!
//! ```ignore
//! use cyme::lsusb::profiler;
//! let sp_usb = profiler::get_spusb(false).unwrap();
//! ```
//!
//! It's often useful to then flatten this into a list of devices ([`system_profiler::USBDevice`]):
//!
//! ```ignore
//! // flatten since we don't care tree/buses
//! let devices = sp_usb.flatten_devices();
//!
//! for device in devices {
//!    format!("{}");
//! }
//! ```
//!
//! One can then print with the cyme display module:
//!
//! ```ignore
//! use cyme::display;
//! // print with default [`display::PrintSettings`]
//! display::print_flattened_devices(&devices, &display::PrintSettings::default());
//! ```
//!
//! The [`system_profiler::SPUSBDataType`] struct contains system [`system_profiler::USBBus`]s, which contain [`system_profiler::USBDevice`]s as a USB tree.
//!
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
#[cfg(all(target_os = "linux", feature = "udev"))]
pub mod udev;
#[cfg(all(all(target_os = "linux", feature = "udevlib"), not(feature = "udev")))]
#[path = "udev_ffi.rs"]
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
            .with_module_level("udevrs", log::Level::Warn.to_level_filter())
            .with_module_level("nusb", log::Level::Warn.to_level_filter())
            .with_module_level("cyme", log::Level::Info.to_level_filter()),
        2 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_module_level("udevrs", log::Level::Info.to_level_filter())
            .with_module_level("nusb", log::Level::Info.to_level_filter())
            .with_module_level("cyme", log::Level::Debug.to_level_filter()),
        3 => SimpleLogger::new()
            .with_utc_timestamps()
            .with_module_level("udevrs", log::Level::Debug.to_level_filter())
            .with_module_level("nusb", log::Level::Debug.to_level_filter())
            .with_module_level("cyme", log::Level::Trace.to_level_filter()),
        // all modules
        _ => SimpleLogger::new()
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
