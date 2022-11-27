//! List system USB buses and devices; a modern `lsusb` that attempts to maintain compatibility with, but also add new features. 
//! Includes a macOS `system_profiler` parser module and `lsusb` for non-macOS systems/gathering more verbose information.
#![allow(dead_code)]
#![warn(missing_docs)]

#[macro_use]
extern crate lazy_static;
pub mod display;
pub mod icon;
pub mod system_profiler;
pub mod types;
pub mod usb;

#[cfg(feature = "libusb")]
pub mod lsusb;
#[cfg(target_os = "linux")]
#[cfg(feature = "udev")]
pub mod udev;
