//! Parser for macOS `system_profiler` command -json output with SPUSBDataType. Merged with libusb or nusb for extra data.
//!
//! Bus and Device structs are used as deserializers for serde. The JSON output with the -json flag is not really JSON; all values are String regardless of contained data so it requires some extra work. Additionally, some values differ slightly from the non json output such as the speed - it is a description rather than numerical.
use super::*;
use std::process::Command;

/// Runs the system_profiler command for SPUSBDataType and parses the json stdout into a [`SystemProfile`].
///
/// Ok result not contain [`usb::DeviceExtra`] because system_profiler does not provide this. Use `get_spusb_with_extra` to combine with libusb output for [`Device`]s with `extra`
pub fn get_spusb() -> Result<SystemProfile> {
    let output = Command::new("system_profiler")
        .args(["-timeout", "5", "-json", "SPUSBDataType"])
        .output()?;

    if output.status.success() {
        serde_json::from_str(String::from_utf8(output.stdout)?.as_str())
            .map_err(|e| {
                Error::new(
                    ErrorKind::Parsing,
                    &format!(
                        "Failed to parse 'system_profiler -json SPUSBDataType'; Error({})",
                        e
                    ),
                )
                // map to get pci.ids host controller data
            })
            .map(|mut sp: SystemProfile| {
                for bus in sp.buses.iter_mut() {
                    bus.fill_host_controller_from_ids();
                }
                sp
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
/// `system_profiler` captures Apple buses (essentially root_hubs) that are not captured by libusb (but are captured by nusb); this method merges the two to so the bus information is kept.
pub fn get_spusb_with_extra() -> Result<SystemProfile> {
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
