//! Utilities to get device information using udev - only supported on Linux. Requires 'udev' feature.
use std::error::Error;
use std::path::Path;
use udev as udevlib;

/// Get and assign `driver_ref` the driver and `syspath_ref` the syspath for device at the `port_path`
///
/// The struct memebers are supplied as references to allow macro attributes calling this only on Linux with udev feature
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
) -> Result<(), Box<dyn Error>> {
    let path: String = format!("/sys/bus/usb/devices/{}", port_path);
    let device = udevlib::Device::from_syspath(&Path::new(&path))?;
    log::debug!("Got device driver {:?}", device.driver());
    *driver_ref = device
        .driver()
        .map(|s| s.to_str().unwrap_or("").to_string());
    *syspath_ref = Some(device.syspath().to_str().unwrap_or("").to_string());

    Ok(())
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
}
