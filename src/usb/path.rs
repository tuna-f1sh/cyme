//! Helper functions for USB sysfs style paths
//!
//! Used for Linux sysfs but also cyme retrieval of USB device information within the [`crate::profiler`] module.
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

use super::*;

/// Represents a USB path in sysfs but used cross-platform to get part of device tree
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsbPath(PathBuf);

impl fmt::Display for UsbPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl AsRef<Path> for UsbPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl UsbPath {
    /// Create a new USB path
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self(path.as_ref().to_path_buf())
    }

    /// Get the inner path
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Get the length of the inner path
    pub fn len(self) -> usize {
        self.path().as_os_str().len()
    }

    /// Is the inner path empty
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Get the parent of the device
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// use std::path::Path;
    ///
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.parent(), Some(Path::new("/sys/bus/usb/devices")));
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0");
    /// assert_eq!(path.parent(), Some(Path::new("/sys/bus/usb/devices")));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.parent(), Some(Path::new("")));
    /// ```
    pub fn parent(&self) -> Option<&Path> {
        if self.endpoint().is_some() {
            self.path().parent().and_then(|p| p.parent())
        } else {
            self.path().parent()
        }
    }

    /// Is the path a bus controller; starts with "usb" in sysfs
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/usb1");
    /// assert!(path.is_bus_controller());
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0");
    /// assert!(!path.is_bus_controller());
    pub fn is_bus_controller(&self) -> bool {
        self.path()
            .file_name()
            .and_then(|f| f.to_str())
            .map(|f| f.starts_with("usb"))
            .unwrap_or(false)
    }

    /// Is the path a root hub
    ///
    /// Trunk ends with "-0" in sysfs, e.g. "1-0". Only root hubs can be port 0
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/2-0");
    /// assert!(path.is_root_hub());
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-0:1.0");
    /// assert!(path.is_root_hub());
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0");
    /// assert!(!path.is_root_hub());
    /// ```
    pub fn is_root_hub(&self) -> bool {
        self.trunk().map(|f| f.ends_with("-0")).unwrap_or(false)
    }

    /// Extract bus number from path
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.bus(), Some(1));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.bus(), Some(1));
    /// ```
    pub fn bus(&self) -> Option<u8> {
        if let Some(port) = self.port() {
            port.split('-').next().and_then(|f| f.parse().ok())
        // special case for bus controller: usbX
        } else {
            self.path()
                .file_name()
                .and_then(|f| f.to_str())
                .and_then(|f| f.strip_prefix("usb").and_then(|f| f.parse().ok()))
        }
    }

    /// Extract trunk path from path
    ///
    /// The trunk path is the path to the first device on the bus
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.trunk(), Some("1-1"));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.trunk(), Some("1-1"));
    /// let path = UsbPath::new("1-2");
    /// assert_eq!(path.trunk(), Some("1-2"));
    pub fn trunk(&self) -> Option<&str> {
        self.port()
            .and_then(|f| f.split_once('.').map(|f| f.0).or(Some(f)))
    }

    /// The device path could be a base device or a device interface representing a device
    ///
    /// It is the path to the device on the bus with any configuration.interface but without any endpoint. On Linux sysfs, this should be a directory with descriptors etc.
    ///
    /// If one is looking for the the device without any the interface, use [`UsbPath::port_path`]
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// use std::path::Path;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0");
    /// assert_eq!(path.device_path(), Path::new("/sys/bus/usb/devices/1-1.3:1.0"));
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1");
    /// assert_eq!(path.device_path(), Path::new("/sys/bus/usb/devices/1-1"));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.device_path(), Path::new("1-1.3:1.0"));
    /// ```
    pub fn device_path(&self) -> &Path {
        if self.endpoint().is_some() {
            self.path().parent().unwrap_or(self.path())
        } else {
            self.path()
        }
    }

    /// Extract just the device filename from [`UsbPath::device_path`]
    pub fn device(&self) -> Option<&str> {
        self.device_path().file_name().and_then(|f| f.to_str())
    }

    /// Extract device port path from path
    ///
    /// The port path is the path to the device on the bus without any configuration.interface. On Linux sysfs, this is the path to the device directory with base device descriptors etc.
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// use std::path::Path;
    ///
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.port_path(), Some(Path::new("/sys/bus/usb/devices/1-1.3")));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.port_path(), Some(Path::new("1-1.3")));
    /// let path = UsbPath::new("1-2");
    /// assert_eq!(path.port_path(), Some(Path::new("1-2")));
    /// ```
    pub fn port_path(&self) -> Option<&Path> {
        self.path()
            .to_str()
            .and_then(|f| f.split_once(':').map(|f| f.0).or(Some(f)))
            .map(Path::new)
    }

    /// Extract port path from path
    pub fn port(&self) -> Option<&str> {
        self.port_path()
            .and_then(|f| f.file_name().and_then(|f| f.to_str()))
    }

    /// Extract configuration number from path
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    ///
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0");
    /// assert_eq!(path.configuration(), Some(1));
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1");
    /// assert_eq!(path.configuration(), Some(1));
    /// ```
    pub fn configuration(&self) -> Option<u8> {
        self.path()
            .to_str()
            .and_then(|f| f.split_once(':'))
            .and_then(|f| f.1.split_once('.').or(Some((f.1, ""))))
            .and_then(|f| f.0.parse().ok())
    }

    /// Extract interface number from path
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    ///
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0");
    /// assert_eq!(path.interface(), Some(0));
    /// ```
    pub fn interface(&self) -> Option<u8> {
        self.path()
            .to_str()
            .and_then(|f| f.split_once(':'))
            .and_then(|f| f.1.split_once('.'))
            .and_then(|f| f.1.parse().ok())
    }

    /// Extract endpoint number from path
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.endpoint(), Some(81));
    pub fn endpoint(&self) -> Option<u8> {
        self.path()
            .file_name()
            .and_then(|f| f.to_str())
            .and_then(|f| f.strip_prefix("ep_"))
            .and_then(|f| f.parse().ok())
    }
}

/// Builds a replica of sysfs path; excludes config.interface
///
/// ```
/// use cyme::usb::get_port_path;
///
/// assert_eq!(get_port_path(1, &[1, 3, 2]), String::from("1-1.3.2"));
/// assert_eq!(get_port_path(1, &[2]), String::from("1-2"));
/// // special case for root_hub
/// assert_eq!(get_port_path(2, &[]), String::from("2-0"));
/// ```
///
/// [ref](http://gajjarpremal.blogspot.com/2015/04/sysfs-structures-for-linux-usb.html)
/// The names that begin with "usb" refer to USB controllers. More accurately, they refer to the "root hub" associated with each controller. The number is the USB bus number. In the example there is only one controller, so its bus is number 1. Hence the name "usb1".
///
/// "1-0:1.0" is a special case. It refers to the root hub's interface. This acts just like the interface in an actual hub an almost every respect; see below.
/// All the other entries refer to genuine USB devices and their interfaces. The devices are named by a scheme like this:
///
///  bus-port.port.port ...
pub fn get_port_path(bus: u8, ports: &[u8]) -> String {
    if ports.len() <= 1 {
        get_trunk_path(bus, ports)
    } else {
        format!("{:}-{}", bus, ports.iter().format("."))
    }
}

/// Parent path is path to parent device
/// ```
/// use cyme::usb::get_parent_path;
///
/// assert_eq!(get_parent_path(1, &[1, 3, 4, 5]).unwrap(), String::from("1-1.3.4"));
/// ```
pub fn get_parent_path(bus: u8, ports: &[u8]) -> error::Result<String> {
    if ports.is_empty() {
        Err(Error::new(
            ErrorKind::InvalidArg,
            "Cannot get parent path for root device",
        ))
    } else {
        Ok(get_port_path(bus, &ports[..ports.len() - 1]))
    }
}

/// Trunk path is path to trunk device on bus
/// ```
/// use cyme::usb::get_trunk_path;
///
/// assert_eq!(get_trunk_path(1, &[1, 3, 5, 6]), String::from("1-1"));
/// // special case for root_hub
/// assert_eq!(get_trunk_path(1, &[]), String::from("1-0"));
/// ```
pub fn get_trunk_path(bus: u8, ports: &[u8]) -> String {
    if ports.is_empty() {
        // special case for root_hub
        format!("{:}-0", bus)
    } else {
        format!("{:}-{}", bus, ports[0])
    }
}

/// Build replica of sysfs path with interface
///
/// ```
/// use cyme::usb::get_interface_path;
///
/// assert_eq!(get_interface_path(1, &[1, 3], 1, 0), String::from("1-1.3:1.0"));
/// // bus
/// assert_eq!(get_interface_path(1, &[], 1, 0), String::from("1-0:1.0"));
/// ```
pub fn get_interface_path(bus: u8, ports: &[u8], config: u8, interface: u8) -> String {
    format!("{}:{}.{}", get_port_path(bus, ports), config, interface)
}

/// Build replica of sysfs path to endpoint
///
/// ```
/// use cyme::usb::get_endpoint_path;
/// use std::path::PathBuf;
///
/// assert_eq!(get_endpoint_path(1, &[1, 3], 1, 0, 81), PathBuf::from("1-1.3:1.0/ep_81"));
/// ```
pub fn get_endpoint_path(
    bus: u8,
    ports: &[u8],
    config: u8,
    interface: u8,
    endpoint: u8,
) -> PathBuf {
    format!(
        "{}/ep_{}",
        get_interface_path(bus, ports, config, interface),
        endpoint
    )
    .into()
}

/// Build replica of Linux dev path from libusb.c *devbususb for getting device with -D
///
/// It's /dev/bus/usb/BUS/DEVNO
///
/// Supply `device_no` as None for bus
///
/// ```
/// use cyme::usb::get_dev_path;
/// use std::path::PathBuf;
///
/// assert_eq!(get_dev_path(1, Some(3)), PathBuf::from("/dev/bus/usb/001/003"));
/// assert_eq!(get_dev_path(1, Some(2)), PathBuf::from("/dev/bus/usb/001/002"));
/// // special case for bus
/// assert_eq!(get_dev_path(1, None), PathBuf::from("/dev/bus/usb/001/001"));
/// ```
pub fn get_dev_path(bus: u8, device_no: Option<u8>) -> PathBuf {
    if let Some(devno) = device_no {
        format!("/dev/bus/usb/{:03}/{:03}", bus, devno).into()
    } else {
        format!("/dev/bus/usb/{:03}/001", bus).into()
    }
}

/// Builds a replica of sysfs name for reading sysfs_props ala: <https://github.com/gregkh/usbutils/blob/master/sysfs.c#L29>
///
/// Like `get_port_path` but root_hubs use the USB controller name (usbX) rather than interface
///
/// ```
/// use cyme::usb::get_sysfs_name;
///
/// assert_eq!(get_sysfs_name(1, &vec![1, 3, 2]), String::from("1-1.3.2"));
/// assert_eq!(get_sysfs_name(1, &vec![2]), String::from("1-2"));
/// // special case for root_hub
/// assert_eq!(get_sysfs_name(2, &vec![]), String::from("usb2"));
/// ```
pub fn get_sysfs_name(bus: u8, ports: &[u8]) -> String {
    if ports.is_empty() {
        // special case for root_hub
        format!("usb{}", bus)
    } else {
        get_port_path(bus, ports)
    }
}
