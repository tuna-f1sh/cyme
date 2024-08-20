//! Parser for macOS `system_profiler` command -json output with SPUSBDataType. Merged with libusb or nusb for extra data.
//!
//! USBBus and USBDevice structs are used as deserializers for serde. The JSON output with the -json flag is not really JSON; all values are String regardless of contained data so it requires some extra work. Additionally, some values differ slightly from the non json output such as the speed - it is a description rather than numerical.
use super::*;
use std::process::Command;

/// Runs the system_profiler command for SPUSBDataType and parses the json stdout into a [`SPUSBDataType`]
///
/// Ok result not contain [`usb::USBDeviceExtra`] because system_profiler does not provide this. Use `get_spusb_with_extra` to combine with libusb output for [`USBDevice`]s with `extra`
pub fn get_spusb() -> Result<SPUSBDataType> {
    let output = if cfg!(target_os = "macos") {
        Command::new("system_profiler")
            .args(["-json", "SPUSBDataType"])
            .output()?
    } else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "system_profiler is only supported on macOS",
        ));
    };

    if output.status.success() {
        serde_json::from_str(String::from_utf8(output.stdout)?.as_str()).map_err(|e| {
            Error::new(
                ErrorKind::Parsing,
                &format!(
                    "Failed to parse 'system_profiler -json SPUSBDataType'; Error({})",
                    e
                ),
            )
        })
    } else {
        log::error!(
            "system_profiler returned non-zero stderr: {:?}, stdout: {:?}",
            String::from_utf8(output.stderr)?,
            String::from_utf8(output.stdout)?
        );
        Err(Error::new(
            ErrorKind::SystemProfiler,
            "system_profiler returned non-zero, use '--force-libusb' to bypass",
        ))
    }
}

/// Runs `get_spusb` and then adds in data obtained from libusb. Requires 'libusb' feature.
///
/// `system_profiler` captures Apple buses (essentially root_hubs) that are not captured by libusb or nusb; this method merges the two to so the bus information is kept.
// TODO capture the Apple buses with IOKit directly not through system_profiler by impl Profiler::get_root_hubs
#[cfg(any(feature = "libusb", feature = "nusb"))]
pub fn get_spusb_with_extra() -> Result<SPUSBDataType> {
    #[cfg(all(feature = "libusb", not(feature = "nusb")))]
    {
        get_spusb().and_then(|mut spusb| {
            crate::profiler::libusb::fill_spusb(&mut spusb)?;
            Ok(spusb)
        })
    }

    #[cfg(feature = "nusb")]
    {
        get_spusb().and_then(|mut spusb| {
            crate::profiler::nusb::fill_spusb(&mut spusb)?;
            Ok(spusb)
        })
    }

    #[cfg(all(not(feature = "libusb"), not(feature = "nusb")))]
    {
        Err(Error::new(
            ErrorKind::Unsupported,
            "nusb or libusb feature is required to do this, install with `cargo install --features nusb/libusb`",
        ))
    }
}
