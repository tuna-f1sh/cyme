//! Utilities to get device information using udev - only supported on Linux. Requires 'udev' feature.
use std::path::Path;
use udev as udevlib;

use crate::error::{Error, ErrorKind};

/// Contains data returned by [`get_udev_info()`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UdevInfo {
    /// The driver name for the device
    pub driver: Option<String>,
    /// The syspath for the device
    pub syspath: Option<String>,
}

/// Lookup the driver and syspath for a device given the `port_path`. Returns [`UdevInfo`] containing both.
///
/// ```no_run
/// use cyme::udev::get_udev_info;
///
/// let udevi = get_udev_info("1-0:1.0").unwrap();
/// assert_eq!(udevi.driver, Some("hub".into()));
/// assert_eq!(udevi.syspath.unwrap().contains("usb1/1-0:1.0"), true);
/// ```
pub fn get_udev_info(port_path: &str) -> Result<UdevInfo, Error> {
    let path: String = format!("/sys/bus/usb/devices/{}", port_path);
    let device = udevlib::Device::from_syspath(&Path::new(&path)).map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!(
                "Failed to get udev info for device at {}: Error({})",
                path,
                e.to_string()
            ),
        )
    })?;

    Ok({
        UdevInfo {
            driver: device
                .driver()
                .map(|s| s.to_str().unwrap_or("").to_string()),
            syspath: device.syspath().to_str().map(|s| s.to_string()),
        }
    })
}

/// Lookup the driver name for a device given the `port_path`.
///
/// ```no_run
/// use cyme::udev::get_udev_driver_name;
/// let driver = get_udev_driver_name("1-0:1.0").unwrap();
/// assert_eq!(driver, Some("hub".into()));
/// ```
pub fn get_udev_driver_name(port_path: &str) -> Result<Option<String>, Error> {
    let path: String = format!("/sys/bus/usb/devices/{}", port_path);
    let device = udevlib::Device::from_syspath(&Path::new(&path)).map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!(
                "Failed to get udev info for device at {}: Error({})",
                path,
                e.to_string()
            ),
        )
    })?;

    Ok(device
        .driver()
        .map(|s| s.to_str().unwrap_or("").to_string()))
}

/// Lookup the syspath for a device given the `port_path`.
///
/// ```no_run
/// use cyme::udev::get_udev_syspath;
/// let syspath = get_udev_syspath("1-0:1.0").unwrap();
/// assert_eq!(syspath.unwrap().contains("usb1/1-0:1.0"), true);
/// ```
pub fn get_udev_syspath(port_path: &str) -> Result<Option<String>, Error> {
    let path: String = format!("/sys/bus/usb/devices/{}", port_path);
    let device = udevlib::Device::from_syspath(&Path::new(&path)).map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!(
                "Failed to get udev info for device at {}: Error({})",
                path,
                e.to_string()
            ),
        )
    })?;

    Ok(device.syspath().to_str().map(|s| s.to_string()))
}

/// Lookup a udev attribute given the `port_path` and `attribute`.
///
/// This only works on Linux and not all devices have all attributes.
/// These attributes are generally readable by all users.
///
/// NOTE: In general you should read from sysfs directly as it does not
///       depend on the udev feature. See `get_sysfs_string()` in lsusb.rs
///
/// ```no_run
/// use cyme::udev::get_udev_attribute;
///
/// let interface_class = get_udev_attribute("1-0:1.0", "bInterfaceClass").unwrap();
/// assert_eq!(interface_class, Some("09".into()));
/// ```
pub fn get_udev_attribute<T: AsRef<std::ffi::OsStr> + std::fmt::Display>(
    port_path: &str,
    attribute: T,
) -> Result<Option<String>, Error> {
    let path: String = format!("/sys/bus/usb/devices/{}", port_path);
    let device = udevlib::Device::from_syspath(&Path::new(&path)).map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!(
                "Failed to get udev attribute {} for device at {}: Error({})",
                attribute,
                path,
                e.to_string()
            ),
        )
    })?;

    Ok(device
        .attribute_value(attribute)
        .map(|s| s.to_str().unwrap_or("").to_string()))
}

/// Lookup an entry in the udev hwdb given the `modalias` and `key`.
///
/// Should act like https://github.com/gregkh/usbutils/blob/master/names.c#L115
///
/// ```
/// use cyme::udev::hwdb_get;
///
/// let modalias = "usb:v1D6Bp0001";
/// let vendor = hwdb_get(&modalias, "ID_VENDOR_FROM_DATABASE").unwrap();
///
/// assert_eq!(vendor, Some("Linux Foundation".into()));
///
/// let modalias = "usb:v*p*d*dc03dsc01dp01*";
/// let vendor = hwdb_get(&modalias, "ID_USB_PROTOCOL_FROM_DATABASE").unwrap();
///
/// assert_eq!(vendor, Some("Keyboard".into()));
/// ```
pub fn hwdb_get(modalias: &str, key: &'static str) -> Result<Option<String>, Error> {
    let hwdb = udevlib::Hwdb::new().map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!("Failed to get hwdb: Error({})", e.to_string()),
        )
    })?;

    Ok(hwdb
        .query_one(&modalias.to_string(), &key.to_string())
        .map(|s| s.to_str().unwrap_or("").to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests can obtain driver and syspath for root_hub on bus 1 - only do if we have USB
    #[cfg_attr(not(feature = "usb_test"), ignore)]
    #[test]
    fn test_udev_info() {
        let udevi = get_udev_info("1-0:1.0").unwrap();
        assert_eq!(udevi.driver, Some("hub".into()));
        assert_eq!(udevi.syspath.unwrap().contains("usb1/1-0:1.0"), true);
    }

    /// Tests can lookup bInterfaceClass of the root hub, which is always 09
    #[cfg_attr(not(feature = "usb_test"), ignore)]
    #[test]
    fn test_udev_attribute() {
        let interface_class = get_udev_attribute("1-0:1.0", "bInterfaceClass").unwrap();
        assert_eq!(interface_class, Some("09".into()));
    }
}
