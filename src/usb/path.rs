//! Helper functions for USB sysfs style paths
//!
//! Used for Linux sysfs but also cyme retrieval of USB device information within the [`crate::profiler`] module and [`crate::usb`] module
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

use super::*;
use crate::profiler::SYSFS_USB_PREFIX;

/// Represents a USB path in sysfs but used cross-platform to represent paths to USB devices, interfaces and endpoints
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

impl From<PathBuf> for UsbPath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl From<&Path> for UsbPath {
    fn from(path: &Path) -> Self {
        Self(path.to_path_buf())
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
    /// let path = UsbPath::new("/sys/bus/usb/devices/usb1");
    /// assert_eq!(path.parent(), Some(Path::new("/sys/bus/usb/devices")));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<&Path> {
        // find : in path, work backwards from index to find next / and return path 0..index
        let path_str = self.path().to_str()?;
        let index = path_str.rfind('-').or_else(|| path_str.rfind("usb"))?;
        let index = path_str[..index].rfind('/')?;
        Some(Path::new(&path_str[..index]))
    }

    /// Is the path to a bus controller (root hub); device starts with "usb" in sysfs
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/usb1");
    /// assert!(path.is_bus_controller());
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0");
    /// assert!(!path.is_bus_controller());
    pub fn is_bus_controller(&self) -> bool {
        self.sysfs_device()
            .map(|f| f.starts_with("usb"))
            .unwrap_or(false)
    }

    /// Does the path belong to a root hub
    ///
    /// Trunk ends with "-0" in sysfs, e.g. "1-0" or usbX (bus controller). Only root hubs can be port 0
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
        self.sysfs_trunk()
            .map(|f| f.ends_with("-0"))
            .unwrap_or(self.is_bus_controller())
    }

    /// Extract bus number from path
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.bus(), Some(1));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.bus(), Some(1));
    /// let path = UsbPath::new("usb1");
    /// assert_eq!(path.bus(), Some(1));
    /// ```
    pub fn bus(&self) -> Option<u8> {
        self.port_path().map(|f| f.bus)
    }

    /// Extract trunk path from path
    ///
    /// The trunk path is the path to the first device on the bus
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.sysfs_trunk(), Some("1-1"));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.sysfs_trunk(), Some("1-1"));
    /// let path = UsbPath::new("1-2");
    /// assert_eq!(path.sysfs_trunk(), Some("1-2"));
    /// let path = UsbPath::new("usb1");
    /// assert_eq!(path.sysfs_trunk(), Some("usb1"));
    pub fn sysfs_trunk(&self) -> Option<&str> {
        self.sysfs_name()
            .and_then(|f| f.split_once('.').map(|f| f.0).or(Some(f)))
    }

    /// The device path could be a base device or a device interface representing a device
    ///
    /// It is the path to the device on the bus with any configuration.interface but without any endpoint. On Linux sysfs, this should be a directory with descriptors etc for the interface or device.
    ///
    /// If one is looking for the the device without any the interface, use [`UsbPath::sysfs_port_path`]
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// use std::path::Path;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.sysfs_device_path(), Some(Path::new("/sys/bus/usb/devices/1-1.3:1.0")));
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3");
    /// assert_eq!(path.sysfs_device_path(), Some(Path::new("/sys/bus/usb/devices/1-1.3")));
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1");
    /// assert_eq!(path.sysfs_device_path(), Some(Path::new("/sys/bus/usb/devices/1-1")));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.sysfs_device_path(), Some(Path::new("1-1.3:1.0")));
    /// ```
    pub fn sysfs_device_path(&self) -> Option<&Path> {
        let path_str = self.path().to_str().unwrap();
        let index = path_str.rfind('-').or_else(|| path_str.rfind("usb"))?;
        // now look for next / from index
        let end_index = path_str[index..]
            .find('/')
            .map(|f| f.saturating_add(index))
            .unwrap_or(path_str.len());
        Some(Path::new(&path_str[..end_index]))
    }

    fn sysfs_device_str(&self) -> Option<&str> {
        self.sysfs_device_path()?
            .file_name()
            .and_then(|f| f.to_str())
    }

    /// Extract just the device filename from [`UsbPath::sysfs_device_path`]
    ///
    /// Unlike [`UsbPath::sysfs_name`] this could be a fully described configuration.interface directory
    pub fn sysfs_device(&self) -> Option<&str> {
        // validate is valid sysfs USB device
        if self.device_path().is_some() {
            self.sysfs_device_str()
        } else {
            None
        }
    }

    /// Extract [`DevicePath`] from path
    pub fn device_path(&self) -> Option<DevicePath> {
        self.sysfs_device_str().and_then(|f| f.parse().ok())
    }

    /// Extract device port path from path
    ///
    /// The port path is the path to the device on the bus without any configuration.interface. On Linux sysfs, this is the path to the device directory with base device descriptors etc. Use [`UsbPath::sysfs_device_path`] to get the full described path if present.
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// use std::path::Path;
    ///
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.sysfs_port_path(), Some(Path::new("/sys/bus/usb/devices/1-1.3")));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.sysfs_port_path(), Some(Path::new("1-1.3")));
    /// let path = UsbPath::new("1-2");
    /// assert_eq!(path.sysfs_port_path(), Some(Path::new("1-2")));
    /// // root hub
    /// let path = UsbPath::new("/sys/bus/usb/devices/usb1");
    /// assert_eq!(path.sysfs_port_path(), Some(Path::new("/sys/bus/usb/devices/usb1")));
    /// ```
    pub fn sysfs_port_path(&self) -> Option<&Path> {
        self.sysfs_device_path()
            .and_then(|f| f.to_str())
            .and_then(|f| f.split_once(':').map(|f| f.0).or(Some(f)))
            .map(Path::new)
    }

    fn sysfs_port_str(&self) -> Option<&str> {
        self.sysfs_port_path()
            .and_then(|f| f.file_name().and_then(|f| f.to_str()))
    }

    /// Extract port path from path
    ///
    /// On Linux sysfs, this is the path to the device directory with base device descriptors etc - the device sysfs name.
    pub fn sysfs_name(&self) -> Option<&str> {
        if self.port_path().is_some() {
            self.sysfs_port_str()
        } else {
            None
        }
    }

    /// Extract [`PortPath`] from path
    pub fn port_path(&self) -> Option<PortPath> {
        self.sysfs_port_str().and_then(|f| f.parse().ok())
    }

    /// Extract configuration number from path
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    ///
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0");
    /// assert_eq!(path.configuration(), Some(1));
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3");
    /// assert_eq!(path.configuration(), None);
    /// ```
    pub fn configuration(&self) -> Option<u8> {
        self.device_path().and_then(|f| f.config)
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
        self.device_path().and_then(|f| f.interface)
    }

    fn endpoint_path_str(&self) -> Option<&str> {
        let path_str = self.path().to_str()?;
        let index = path_str.rfind("ep_")?;
        let end_index = path_str[index..]
            .find('/')
            .map(|f| f.saturating_add(index))
            .unwrap_or(path_str.len());
        Some(&path_str[index..end_index])
    }

    /// Extract the [`EndpointPath`] from path
    pub fn endpoint_path(&self) -> Option<EndpointPath> {
        Some(EndpointPath::new_with_device_path(
            self.device_path()?,
            self.endpoint()?,
        ))
    }

    /// Extract endpoint number from path
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.endpoint(), Some(81));
    pub fn endpoint(&self) -> Option<u8> {
        self.endpoint_path_str()
            .and_then(|f| f.strip_prefix("ep_"))
            .and_then(|f| f.parse().ok())
    }

    /// Convert to sysfs path for reading sysfs properties
    ///
    /// If the path is already a sysfs path, it will be returned as is
    ///
    /// ```
    /// use cyme::usb::UsbPath;
    /// use std::path::Path;
    /// let path = UsbPath::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81");
    /// assert_eq!(path.to_sysfs_path(), Path::new("/sys/bus/usb/devices/1-1.3:1.0/ep_81"));
    /// let path = UsbPath::new("1-1.3:1.0");
    /// assert_eq!(path.to_sysfs_path(), Path::new("/sys/bus/usb/devices/1-1.3:1.0"));
    pub fn to_sysfs_path(&self) -> PathBuf {
        if self.path().starts_with(SYSFS_USB_PREFIX) {
            self.path().to_path_buf()
        } else {
            Path::new(SYSFS_USB_PREFIX).join(self.path())
        }
    }
}

/// Port path to a device
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortPath {
    /// Bus number
    bus: u8,
    /// Port numbers through bus to device
    ports: Vec<u8>,
}

impl FromStr for PortPath {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        // root hub
        if let Some(s) = s.strip_prefix("usb") {
            let num = s
                .parse::<u8>()
                .map_err(|_| Error::new(ErrorKind::Parsing, &format!("Invalid bus number: {s}")))?;
            Ok(Self {
                bus: num,
                ports: vec![],
            })
        } else {
            // strip out any config.interface, then split by '-'
            let mut parts = s.split(':').next().unwrap_or(s).split('-');
            let bus = parts
                .next()
                .ok_or_else(|| Error::new(ErrorKind::Parsing, &format!("No bus number: {s}")))?
                .parse()
                .map_err(|_| Error::new(ErrorKind::Parsing, &format!("Invalid bus number: {s}")))?;
            let ports = parts
                .next()
                .ok_or_else(|| Error::new(ErrorKind::Parsing, &format!("No port number: {s}")))?
                .split('.')
                .map(|p| p.parse())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    Error::new(ErrorKind::Parsing, &format!("Invalid port number: {s}"))
                })?;
            Ok(Self { bus, ports })
        }
    }
}

impl fmt::Display for PortPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // root hut
        if self.ports.is_empty() {
            if f.alternate() {
                write!(f, "usb{}", self.bus)
            } else {
                write!(f, "{}-0", self.bus)
            }
        } else {
            write!(f, "{}-{}", self.bus, self.ports.iter().format("."))
        }
    }
}

impl TryFrom<&Path> for PortPath {
    type Error = Error;

    fn try_from(s: &Path) -> error::Result<Self> {
        s.file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| Error::new(ErrorKind::InvalidPath, "Invalid path"))
            .and_then(|f| f.parse())
    }
}

impl TryFrom<&str> for PortPath {
    type Error = Error;

    fn try_from(s: &str) -> error::Result<Self> {
        s.parse()
    }
}

impl From<PortPath> for UsbPath {
    fn from(p: PortPath) -> Self {
        UsbPath::new(p.to_string())
    }
}

impl From<&PortPath> for UsbPath {
    fn from(p: &PortPath) -> Self {
        UsbPath::new(p.to_string())
    }
}

impl PortPath {
    /// Create a new port path from bus number and port tree positions
    pub fn new(bus: u8, ports: Vec<u8>) -> Self {
        Self { bus, ports }
    }

    /// Get the bus number
    pub fn bus(&self) -> u8 {
        self.bus
    }

    /// Get the port tree positions
    pub fn ports(&self) -> &[u8] {
        &self.ports
    }

    /// Get the parent port path; one branch up the tree
    ///
    /// ```
    /// use cyme::usb::PortPath;
    /// let path = PortPath::new(1, vec![1, 3, 4, 5]);
    /// assert_eq!(path.parent(), Some(PortPath::new(1, vec![1, 3, 4])));
    /// let path = PortPath::new(1, vec![]);
    /// assert_eq!(path.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<Self> {
        if self.ports.is_empty() {
            None
        } else {
            Some(Self {
                bus: self.bus,
                ports: self.ports[..self.ports.len() - 1].to_vec(),
            })
        }
    }

    /// Get the trunk port path; the first device in the tree
    ///
    /// ```
    /// use cyme::usb::PortPath;
    /// let path = PortPath::new(1, vec![1, 3, 5, 6]);
    /// assert_eq!(path.trunk(), PortPath::new(1, vec![1]));
    /// let path = PortPath::new(1, vec![2]);
    /// assert_eq!(path.trunk(), PortPath::new(1, vec![2]));
    /// // root hub
    /// let path = PortPath::new(2, vec![]);
    /// assert_eq!(path.trunk(), PortPath::new(2, vec![]));
    /// ```
    pub fn trunk(&self) -> Self {
        if self.ports.is_empty() {
            Self {
                bus: self.bus,
                ports: vec![],
            }
        } else {
            Self {
                bus: self.bus,
                ports: vec![self.ports[0]],
            }
        }
    }

    /// Get the branch depth in the port tree - length of ports vector
    pub fn depth(&self) -> usize {
        self.ports.len()
    }

    /// Is the port path to a root hub
    pub fn is_root_hub(&self) -> bool {
        self.ports.is_empty() || self.ports == [0]
    }
}

/// Helper type for defining the location of a [`Configuration`]
///
/// A configuration does not exist in sysfs without an interface so this is used internally to get a [`Configuration`] from a [`PortPath`]
pub type ConfigurationPath = (PortPath, u8);

/// Device path to a device
///
/// A device path is a port path with optional configuration and interface numbers. It is used to represent a device in sysfs, which could mean a base device or a device interface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevicePath {
    port_path: PortPath,
    config: Option<u8>,
    interface: Option<u8>,
    alt_setting: Option<u8>,
}

impl FromStr for DevicePath {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        let (p, ci) = s.split_once(':').unwrap_or((s, ""));
        let port_path = p.parse()?;
        let mut parts = ci.split('.');
        // make sure both config and interface are present if one is
        match (
            parts.next().and_then(|f| f.parse::<u8>().ok()),
            parts.next().and_then(|f| f.parse::<u8>().ok()),
        ) {
            (Some(config), Some(interface)) => Ok(Self {
                port_path,
                config: Some(config),
                interface: Some(interface),
                alt_setting: None,
            }),
            _ => Ok(Self {
                port_path,
                config: None,
                interface: None,
                alt_setting: None,
            }),
        }
    }
}

impl fmt::Display for DevicePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.port_path)?;
        // only write config.interface if both are present
        // one does not get path with just config or interface
        if let (Some(config), Some(interface)) = (self.config, self.interface) {
            write!(f, ":{config}")?;
            write!(f, ".{interface}")?;
        }
        Ok(())
    }
}

impl TryFrom<&Path> for DevicePath {
    type Error = Error;

    fn try_from(s: &Path) -> error::Result<Self> {
        s.file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| Error::new(ErrorKind::InvalidPath, "Invalid path"))
            .and_then(|f| f.parse())
    }
}

impl From<DevicePath> for PortPath {
    fn from(d: DevicePath) -> Self {
        d.port_path
    }
}

impl From<PortPath> for DevicePath {
    fn from(p: PortPath) -> Self {
        Self {
            port_path: p,
            config: None,
            interface: None,
            alt_setting: None,
        }
    }
}

impl From<&DevicePath> for UsbPath {
    fn from(d: &DevicePath) -> Self {
        UsbPath::new(d.to_string())
    }
}

impl From<DevicePath> for UsbPath {
    fn from(d: DevicePath) -> Self {
        UsbPath::new(d.to_string())
    }
}

impl DevicePath {
    /// Create a new device path from [`PortPath`], configuration and interface
    pub fn new_with_port_path(
        port_path: PortPath,
        config: Option<u8>,
        interface: Option<u8>,
        alt_setting: Option<u8>,
    ) -> Self {
        Self {
            port_path,
            config,
            interface,
            alt_setting,
        }
    }

    /// Create a new device path from bus number, port tree positions, configuration and interface
    pub fn new(
        bus: u8,
        ports: Vec<u8>,
        config: Option<u8>,
        interface: Option<u8>,
        alt_setting: Option<u8>,
    ) -> Self {
        Self {
            port_path: PortPath::new(bus, ports),
            config,
            interface,
            alt_setting,
        }
    }

    /// Get the port path
    pub fn port_path(&self) -> &PortPath {
        &self.port_path
    }

    /// Get the configuration number
    pub fn configuration(&self) -> Option<u8> {
        self.config
    }

    /// Get the interface number
    pub fn interface(&self) -> Option<u8> {
        self.interface
    }

    /// Get the interface alternate setting
    ///
    /// If not set, it defaults to 0
    pub fn alt_setting(&self) -> u8 {
        self.alt_setting.unwrap_or(0)
    }

    /// Set the interface alternate setting
    pub fn set_alt_setting(&mut self, alt: u8) {
        self.alt_setting = Some(alt);
    }
}

/// Path to an endpoint on a device
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointPath {
    device_path: DevicePath,
    endpoint: u8,
}

impl FromStr for EndpointPath {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        let mut parts = s.split("/ep_");
        if let (Some(d), Some(e)) = (parts.next(), parts.next()) {
            let device_path: DevicePath = d.parse()?;
            // thought about this but there are actually base device (1-1) with ep_00 so it is valid, just not used by cyme
            //if device_path.configuration().is_none() || device_path.interface().is_none() {
            //    return Err(Error::new(
            //        ErrorKind::Parsing,
            //        &format!("Invalid endpoint path {}: requires config.interface", s),
            //    ));
            //}
            let endpoint = e
                .parse()
                .map_err(|_| Error::new(ErrorKind::Parsing, &format!("Invalid endpoint: {e}")))?;
            Ok(Self {
                device_path,
                endpoint,
            })
        } else {
            Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid endpoint path: {s}"),
            ))
        }
    }
}

impl TryFrom<&Path> for EndpointPath {
    type Error = Error;

    fn try_from(s: &Path) -> error::Result<Self> {
        s.to_str()
            .ok_or_else(|| Error::new(ErrorKind::InvalidPath, "Invalid path"))
            .and_then(|f| f.parse())
    }
}

impl fmt::Display for EndpointPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.device_path)?;
        write!(f, "/ep_{}", self.endpoint)
    }
}

impl From<EndpointPath> for UsbPath {
    fn from(ep: EndpointPath) -> Self {
        UsbPath::new(ep.device_path.to_string())
    }
}

impl EndpointPath {
    /// Create a new endpoint path from [`PortPath`], configuration, interface and endpoint number
    pub fn new_with_device_path(device_path: DevicePath, endpoint: u8) -> Self {
        Self {
            device_path,
            endpoint,
        }
    }

    /// Create a new endpoint path from [`PortPath`], configuration, interface and endpoint number
    pub fn new_with_port_path(
        port_path: PortPath,
        config: u8,
        interface: u8,
        alt_setting: u8,
        endpoint: u8,
    ) -> Self {
        Self {
            device_path: DevicePath::new_with_port_path(
                port_path,
                Some(config),
                Some(interface),
                Some(alt_setting),
            ),
            endpoint,
        }
    }

    /// Create a new endpoint path from bus number, port tree positions, configuration, interface and endpoint number
    pub fn new(
        bus: u8,
        ports: Vec<u8>,
        config: u8,
        interface: u8,
        alt_setting: u8,
        endpoint: u8,
    ) -> Self {
        Self {
            device_path: DevicePath::new(
                bus,
                ports,
                Some(config),
                Some(interface),
                Some(alt_setting),
            ),
            endpoint,
        }
    }

    /// Get the device path
    pub fn device_path(&self) -> &DevicePath {
        &self.device_path
    }

    /// Get the endpoint address byte
    pub fn endpoint(&self) -> u8 {
        self.endpoint
    }

    /// Get the [`EndpointAddress`] from the endpoint number
    pub fn endpoint_address(&self) -> EndpointAddress {
        EndpointAddress::from(self.endpoint)
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
    if ports.is_empty() {
        // special case for root_hub
        format!("{bus:}-0")
    } else {
        format!("{:}-{}", bus, ports.iter().format("."))
    }
}

/// Parent path is path to parent device
/// ```
/// use cyme::usb::get_parent_path;
///
/// assert_eq!(get_parent_path(1, &[1, 3, 4, 5]), Some(String::from("1-1.3.4")));
/// ```
pub fn get_parent_path(bus: u8, ports: &[u8]) -> Option<String> {
    if ports.is_empty() {
        None
    } else {
        Some(get_port_path(bus, &ports[..ports.len() - 1]))
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
        format!("{bus:}-0")
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
        format!("/dev/bus/usb/{bus:03}/{devno:03}").into()
    } else {
        format!("/dev/bus/usb/{bus:03}/001").into()
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
        format!("usb{bus}")
    } else {
        get_port_path(bus, ports)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_port_path_parse() {
        assert_eq!(
            "1-1.3.4".parse::<PortPath>(),
            Ok(PortPath {
                bus: 1,
                ports: vec![1, 3, 4]
            })
        );

        assert_eq!(
            "1-1.3".parse::<PortPath>(),
            Ok(PortPath {
                bus: 1,
                ports: vec![1, 3]
            })
        );

        assert_eq!(
            "1-2".parse::<PortPath>(),
            Ok(PortPath {
                bus: 1,
                ports: vec![2]
            })
        );

        assert_eq!(
            "1-0:1-0".parse::<PortPath>(),
            Ok(PortPath {
                bus: 1,
                ports: vec![0]
            })
        );

        assert_eq!(
            "usb1".parse::<PortPath>(),
            Ok(PortPath {
                bus: 1,
                ports: vec![]
            })
        );
    }

    #[test]
    fn test_port_path_display() {
        assert_eq!(
            PortPath {
                bus: 1,
                ports: vec![1, 3, 4]
            }
            .to_string(),
            "1-1.3.4"
        );

        assert_eq!(
            PortPath {
                bus: 1,
                ports: vec![1, 3]
            }
            .to_string(),
            "1-1.3"
        );

        assert_eq!(
            PortPath {
                bus: 1,
                ports: vec![2]
            }
            .to_string(),
            "1-2"
        );

        assert_eq!(
            PortPath {
                bus: 1,
                ports: vec![0]
            }
            .to_string(),
            "1-0"
        );

        assert_eq!(
            PortPath {
                bus: 1,
                ports: vec![]
            }
            .to_string(),
            "1-0"
        );
    }
}
