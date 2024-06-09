//! Utilities to get device information using udev - only supported on Linux. Requires 'udev' feature.
use udevrs::{udev_new, UdevDevice, UdevHwdb};

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
    let device = UdevDevice::new_from_syspath(udev_new(), &path).map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!(
                "Failed to get udev info for device at {}: Error({})",
                path, e
            ),
        )
    })?;

    Ok({
        UdevInfo {
            driver: Some(device.driver().to_string()),
            syspath: Some(device.syspath().to_string()),
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
    let device = UdevDevice::new_from_syspath(udev_new(), &path).map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!(
                "Failed to get udev info for device at {}: Error({})",
                path, e
            ),
        )
    })?;

    Ok(Some(device.driver().to_owned()))
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
    let device = UdevDevice::new_from_syspath(udev_new(), &path).map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!(
                "Failed to get udev info for device at {}: Error({})",
                path, e
            ),
        )
    })?;

    Ok(Some(device.syspath().to_owned()))
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
pub fn get_udev_attribute<T: AsRef<std::ffi::OsStr> + std::fmt::Display + Into<String>>(
    port_path: &str,
    attribute: T,
) -> Result<Option<String>, Error> {
    let path: String = format!("/sys/bus/usb/devices/{}", port_path);
    let mut device = UdevDevice::new_from_syspath(udev_new(), &path).map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!(
                "Failed to get udev attribute {} for device at {}: Error({})",
                attribute, path, e
            ),
        )
    })?;

    Ok(device.get_sysattr_value(&attribute.into()))
}

/// udev hwdb lookup functions
///
/// Protected by the `udev_hwdb` feature because 'libudev-sys' excludes hwdb ffi bindings if native udev does not support hwdb
//#[cfg(feature = "udev_hwdb")]
pub mod hwdb {
    use super::*;
    /// Lookup an entry in the udev hwdb given the `modalias` and `key`.
    ///
    /// Should act like https://github.com/gregkh/usbutils/blob/master/names.c#L115
    ///
    /// ```
    /// use cyme::udev;
    ///
    /// let modalias = "usb:v1D6Bp0001";
    /// let vendor = udev::hwdb::get(&modalias, "ID_VENDOR_FROM_DATABASE").unwrap();
    ///
    /// assert_eq!(vendor, Some("Linux Foundation".into()));
    ///
    /// let modalias = "usb:v*p*d*dc03dsc01dp01*";
    /// let vendor = udev::hwdb::get(&modalias, "ID_USB_PROTOCOL_FROM_DATABASE").unwrap();
    ///
    /// assert_eq!(vendor, Some("Keyboard".into()));
    /// ```
    pub fn get(modalias: &str, _key: &'static str) -> Result<Option<String>, Error> {
        let mut hwdb = UdevHwdb::new(udev_new()).map_err(|e| {
            Error::new(
                ErrorKind::Udev,
                &format!("Failed to get hwdb: Error({})", e),
            )
        })?;

        Ok(hwdb
            .get_properties_list_entry(&modalias.to_string(), 0)
            .map(|entry| entry.value().to_owned()))
    }
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
        assert!(udevi.syspath.unwrap().contains("usb1/1-0:1.0"));
    }

    /// Tests can lookup bInterfaceClass of the root hub, which is always 09
    #[cfg_attr(not(feature = "usb_test"), ignore)]
    #[test]
    fn test_udev_attribute() {
        let interface_class = get_udev_attribute("1-0:1.0", "bInterfaceClass").unwrap();
        assert_eq!(interface_class, Some("09".into()));
    }
}
