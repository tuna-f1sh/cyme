//! Utilities to get device information using udev - only supported on Linux. Requires 'udev' feature.
use std::path::Path;
use udev as udevlib;

use crate::error::{Error, ErrorKind};

/// Get and assign `driver_ref` the driver and `syspath_ref` the syspath for device at the `port_path`
///
/// The struct members are supplied as references to allow macro attributes calling this only on Linux with udev feature
///
/// ```no_run
/// use cyme::udev::get_udev_info;
///
/// let mut driver: Option<String> = None;
/// let mut syspath: Option<String> = None;
///
/// get_udev_info(&mut driver, &mut syspath, &String::from("1-0:1.0"));
/// assert_eq!(driver, Some("hub".into()));
/// assert_eq!(syspath.unwrap().contains("usb1/1-0:1.0"), true);
///
/// ```
pub fn get_udev_info(
    driver_ref: &mut Option<String>,
    syspath_ref: &mut Option<String>,
    port_path: &String,
) -> Result<(), Error> {
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
    log::trace!("Got device driver {:?}", device.driver());
    *driver_ref = device
        .driver()
        .map(|s| s.to_str().unwrap_or("").to_string());
    *syspath_ref = Some(device.syspath().to_str().unwrap_or("").to_string());

    Ok(())
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
/// let interface_class = get_udev_attribute(&String::from("1-0:1.0"),"bInterfaceClass").unwrap();
/// assert_eq!(interface_class, Some("09".into()));
/// ```
pub fn get_udev_attribute<T: AsRef<std::ffi::OsStr> + std::fmt::Display>(
    port_path: &String,
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
/// let modalias = String::from("usb:v1D6Bp0001");
/// let key = String::from("ID_VENDOR_FROM_DATABASE");
/// let vendor = hwdb_get(&modalias, &key).unwrap();
///
/// assert_eq!(vendor, Some("Linux Foundation".into()));
///
/// let modalias = String::from("usb:v*p*d*dc03dsc01dp01*");
/// let key = String::from("ID_USB_PROTOCOL_FROM_DATABASE");
/// let vendor = hwdb_get(&modalias, &key).unwrap();
///
/// assert_eq!(vendor, Some("Keyboard".into()));
/// ```
pub fn hwdb_get(modalias: &String, key: &String) -> Result<Option<String>, Error> {
    let hwdb = udevlib::Hwdb::new().map_err(|e| {
        Error::new(
            ErrorKind::Udev,
            &format!("Failed to get hwdb: Error({})", e.to_string()),
        )
    })?;

    Ok(hwdb
        .query_one(modalias, key)
        .map(|s| s.to_str().unwrap_or("").to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests can obtain driver and syspath for root_hub on bus 1 - only do if we have USB
    #[cfg_attr(not(feature = "usb_test"), ignore)]
    #[test]
    fn test_udev_info() {
        let mut driver: Option<String> = None;
        let mut syspath: Option<String> = None;

        get_udev_info(&mut driver, &mut syspath, &String::from("1-0:1.0")).unwrap();
        assert_eq!(driver, Some("hub".into()));
        assert_eq!(syspath.unwrap().contains("usb1/1-0:1.0"), true);
    }

    /// Tests can lookup bInterfaceClass of the root hub, which is always 09
    #[cfg_attr(not(feature = "usb_test"), ignore)]
    #[test]
    fn test_udev_attribute() {
        let interface_class =
            get_udev_attribute(&String::from("1-0:1.0"), "bInterfaceClass").unwrap();
        assert_eq!(interface_class, Some("09".into()));
    }
}
