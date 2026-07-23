//! USB Type-C connector, partner and cable data read from the Linux `typec` sysfs class (`/sys/class/typec`)
//!
//! Field names and valid values are taken from the kernel ABI doc
//! (`Documentation/ABI/testing/sysfs-class-typec`) where documented; alt-mode `svid`/`active`
//! naming is not in that doc, only in `drivers/usb/typec/class.c`.
//!
//! Correlating a [`TypecPort`] back to an enumerated USB [`crate::profiler::Device`] is only
//! possible where the kernel has an ACPI companion for the port (`drivers/usb/typec/port-mapper.c`
//! matches on the shared `_PLD`). On Device Tree only systems (eg. Qualcomm UCSI laptops) the
//! kernel has no per-device link today, so [`TypecPort::port_path`] stays `None` there - see
//! `enumerate_typec_ports` for how the link is read when it does exist.
use std::fmt;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use super::PortPath;
use crate::error::{self, Error, ErrorKind};

/// Default sysfs location for the `typec` class
pub(crate) const SYSFS_TYPEC_PREFIX: &str = "/sys/class/typec/";

/// Extract the value currently selected from a sysfs "choice" attribute
///
/// The kernel typec ABI shows the active value of a multi-choice attribute in square brackets
/// amongst the other possible values, eg. `data_role` reads `"[host] device"` when the port is
/// acting as host.
///
/// ```
/// use cyme::usb::typec::current_choice;
/// assert_eq!(current_choice("[host] device"), Some("host"));
/// assert_eq!(current_choice("source [sink]"), Some("sink"));
/// assert_eq!(current_choice("no brackets here"), None);
/// ```
pub fn current_choice(s: &str) -> Option<&str> {
    let start = s.find('[')?;
    let end = start + s[start..].find(']')?;
    Some(&s[start + 1..end])
}

/// `data_role` of a [`TypecPort`] - see kernel ABI doc `data_role`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRole {
    /// Port is acting as USB host
    Host,
    /// Port is acting as USB device
    Device,
}

impl FromStr for DataRole {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "host" => Ok(Self::Host),
            "device" => Ok(Self::Device),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec data_role: {s}"),
            )),
        }
    }
}

/// `power_role` of a [`TypecPort`] - see kernel ABI doc `power_role`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerRole {
    /// Port is supplying VBUS power
    Source,
    /// Port is consuming VBUS power
    Sink,
}

impl FromStr for PowerRole {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "source" => Ok(Self::Source),
            "sink" => Ok(Self::Sink),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec power_role: {s}"),
            )),
        }
    }
}

/// `preferred_role` of a [`TypecPort`] - unlike [`PowerRole`] this has a third "no preference" state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferredRole {
    /// Port prefers Try.SRC
    Source,
    /// Port prefers Try.SNK
    Sink,
    /// No power role preference set
    None,
}

impl FromStr for PreferredRole {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "source" => Ok(Self::Source),
            "sink" => Ok(Self::Sink),
            "none" => Ok(Self::None),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec preferred_role: {s}"),
            )),
        }
    }
}

/// `port_type` of a [`TypecPort`] - see kernel ABI doc `port_type`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortType {
    /// Source only DFP port
    Source,
    /// Sink only UFP port
    Sink,
    /// Dual-role-data and dual-role-power port
    Dual,
}

impl FromStr for PortType {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "source" => Ok(Self::Source),
            "sink" => Ok(Self::Sink),
            "dual" => Ok(Self::Dual),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec port_type: {s}"),
            )),
        }
    }
}

/// `orientation` of a [`TypecPort`] - see kernel ABI doc `orientation`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    /// CC1 orientation
    Normal,
    /// CC2 orientation
    Reverse,
    /// Orientation cannot be determined
    Unknown,
}

impl FromStr for Orientation {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "normal" => Ok(Self::Normal),
            "reverse" => Ok(Self::Reverse),
            "unknown" => Ok(Self::Unknown),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec orientation: {s}"),
            )),
        }
    }
}

/// `power_operation_mode` of a [`TypecPort`] - see kernel ABI doc `power_operation_mode`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerOperationMode {
    /// Default USB current level for VBUS
    Default,
    /// 1.5A current level for VBUS
    #[serde(rename = "1.5A")]
    Amps1_5,
    /// 3.0A current level for VBUS
    #[serde(rename = "3.0A")]
    Amps3_0,
    /// Current level negotiated via USB Power Delivery
    UsbPowerDelivery,
}

impl FromStr for PowerOperationMode {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "default" => Ok(Self::Default),
            "1.5A" => Ok(Self::Amps1_5),
            "3.0A" => Ok(Self::Amps3_0),
            "usb_power_delivery" => Ok(Self::UsbPowerDelivery),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec power_operation_mode: {s}"),
            )),
        }
    }
}

/// `type` of a [`Partner`] - combines the UFP and DFP product type vocabularies since both are
/// read from the same `type` attribute file, only one active per current data role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductType {
    /// Product type not visible to the device driver
    Undefined,
    /// PDUSB Hub (UFP) or PDUSB Hub (DFP)
    Hub,
    /// PDUSB Peripheral (UFP role only)
    Peripheral,
    /// Power Bank (UFP role only)
    Psd,
    /// Alternate Mode Adapter (UFP role only)
    Ama,
    /// PDUSB Host (DFP role only)
    Host,
    /// Power Brick (DFP role only)
    PowerBrick,
    /// Alternate Mode Controller (DFP role only)
    Amc,
}

impl FromStr for ProductType {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "undefined" => Ok(Self::Undefined),
            "hub" => Ok(Self::Hub),
            "peripheral" => Ok(Self::Peripheral),
            "psd" => Ok(Self::Psd),
            "ama" => Ok(Self::Ama),
            "host" => Ok(Self::Host),
            "power_brick" => Ok(Self::PowerBrick),
            "amc" => Ok(Self::Amc),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec partner/cable product type: {s}"),
            )),
        }
    }
}

/// `type` of a [`Cable`] - see kernel ABI doc `<port>-cable/type`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CableType {
    /// Product type not visible to the device driver
    Undefined,
    /// Electronically marked active cable
    Active,
    /// Passive cable
    Passive,
}

impl FromStr for CableType {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "undefined" => Ok(Self::Undefined),
            "active" => Ok(Self::Active),
            "passive" => Ok(Self::Passive),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec cable type: {s}"),
            )),
        }
    }
}

/// `plug_type` of a [`Cable`] - see kernel ABI doc `<port>-cable/plug_type`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlugType {
    /// Standard-A plug
    TypeA,
    /// Standard-B plug
    TypeB,
    /// Type-C plug
    TypeC,
    /// Captive (permanently attached) plug
    Captive,
}

impl FromStr for PlugType {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "type-a" => Ok(Self::TypeA),
            "type-b" => Ok(Self::TypeB),
            "type-c" => Ok(Self::TypeC),
            "captive" => Ok(Self::Captive),
            _ => Err(Error::new(
                ErrorKind::Parsing,
                &format!("Invalid typec cable plug_type: {s}"),
            )),
        }
    }
}

/// A single USB Type-C alternate mode, eg. `/sys/class/typec/port0/port0.0/`
///
/// Naming (`<parent>.<index>`) is not in the kernel ABI doc, only in
/// `drivers/usb/typec/class.c` (`dev_set_name(&alt->adev.dev, "%s.%u", ...)`)
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AltMode {
    /// Mode index from the directory name suffix, eg. `1` for `port0.1`
    pub index: u8,
    /// Standard or Vendor ID for the alternate mode, eg. `0x8087` (Intel/TBT), `0xff01` (DisplayPort)
    pub svid: u16,
    /// Whether this alternate mode is the currently active one
    ///
    /// On a port's own alt-modes this is a static local capability slot, not proof a mode is
    /// actually running with a partner - a real negotiated mode is a separate node under the
    /// partner (see [`Partner::alt_modes`])
    pub active: Option<bool>,
}

/// USB Type-C partner device, eg. `/sys/class/typec/port0-partner/`
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Partner {
    /// Whether the partner supports USB Power Delivery communication
    pub supports_usb_power_delivery: Option<bool>,
    /// Product type of the partner if known
    pub product_type: Option<ProductType>,
    /// Number of alternate modes advertised by the partner during PD discovery
    pub number_of_alternate_modes: Option<u32>,
    /// Alternate modes actually negotiated with the partner
    pub alt_modes: Option<Vec<AltMode>>,
    /// [`PortPath`] of the enumerated USB device linked to this partner
    ///
    /// Only present where the kernel found an ACPI companion for the port (see module docs) -
    /// resolved from the reverse `typec` symlink the kernel creates under this partner's own
    /// sysfs directory, named after the linked device (eg. `2-2`), not from any attribute file
    pub port_path: Option<PortPath>,
}

/// USB Type-C cable device, eg. `/sys/class/typec/port0-cable/`
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Cable {
    /// Product type of the cable if known
    pub cable_type: Option<CableType>,
    /// Type of the plug on the cable
    pub plug_type: Option<PlugType>,
}

/// A single USB Type-C connector/port, eg. `/sys/class/typec/port0/`
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypecPort {
    /// sysfs class name, eg. `port0`
    pub name: String,
    /// [`PortPath`] of the enumerated USB device on this port, if the kernel could resolve it - see [`Partner::port_path`] for how
    pub port_path: Option<PortPath>,
    /// Current USB data role
    pub data_role: Option<DataRole>,
    /// Current USB power role
    pub power_role: Option<PowerRole>,
    /// Configured power role preference, if any
    pub preferred_role: Option<PreferredRole>,
    /// Configured port type
    pub port_type: Option<PortType>,
    /// Whether the port is the VCONN Source
    pub vconn_source: Option<bool>,
    /// Current power operation mode (VBUS current level)
    pub power_operation_mode: Option<PowerOperationMode>,
    /// Active cable orientation
    pub orientation: Option<Orientation>,
    /// Revision of supported USB Power Delivery spec, eg. `"3.0"`, or `"0.0"` if unsupported
    pub usb_power_delivery_revision: Option<String>,
    /// Space separated accessory modes the port supports, split into a list
    pub supported_accessory_modes: Option<Vec<String>>,
    /// Alternate modes the port itself is capable of - local capability, not necessarily negotiated with a partner
    pub alt_modes: Option<Vec<AltMode>>,
    /// Partner device currently attached, if any
    pub partner: Option<Partner>,
    /// Cable currently attached, if any
    pub cable: Option<Cable>,
}

impl fmt::Display for TypecPort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(target_os = "linux")]
mod sysfs {
    use super::*;
    use std::fs;

    fn read_attr(dir: &Path, name: &str) -> Option<String> {
        let content = fs::read_to_string(dir.join(name)).ok()?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn read_choice_attr<T: FromStr>(dir: &Path, name: &str) -> Option<T> {
        let raw = read_attr(dir, name)?;
        current_choice(&raw).and_then(|v| v.parse().ok())
    }

    fn read_bool_attr(dir: &Path, name: &str) -> Option<bool> {
        match read_attr(dir, name)?.as_str() {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        }
    }

    fn read_list_attr(dir: &Path, name: &str) -> Option<Vec<String>> {
        Some(
            read_attr(dir, name)?
                .split_whitespace()
                .map(String::from)
                .collect(),
        )
    }

    /// Parse a single alt-mode directory, eg. `port0.1` -> index `1`
    fn parse_alt_mode(dir_name: &str, path: &Path) -> Option<AltMode> {
        let index = dir_name.rsplit('.').next()?.parse().ok()?;
        let svid = u16::from_str_radix(read_attr(path, "svid")?.trim(), 16).ok()?;
        let active = read_bool_attr(path, "active");
        Some(AltMode {
            index,
            svid,
            active,
        })
    }

    /// Collect alt-mode subdirectories of `dir` named `<prefix>.<index>`
    fn collect_alt_modes(dir: &Path, prefix: &str) -> Option<Vec<AltMode>> {
        let entries = fs::read_dir(dir).ok()?;
        let dot_prefix = format!("{prefix}.");
        let modes: Vec<AltMode> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with(&dot_prefix) {
                    parse_alt_mode(&name, &e.path())
                } else {
                    None
                }
            })
            .collect();

        if modes.is_empty() {
            None
        } else {
            Some(modes)
        }
    }

    /// Find the reverse `typec` symlink the kernel creates inside a partner/cable directory
    /// pointing back at the enumerated USB device (named after the device, eg. `2-2` or `usb1`) -
    /// see `typec_partner_link_device()` in `drivers/usb/typec/class.c`. Only present when the
    /// port has an ACPI companion (see module docs).
    fn find_device_link(dir: &Path) -> Option<PortPath> {
        fs::read_dir(dir)
            .ok()?
            .filter_map(|e| e.ok())
            .find_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|name| name.parse::<PortPath>().ok())
            })
    }

    fn parse_partner(port_name: &str, class_root: &Path) -> Option<Partner> {
        let dir = class_root.join(format!("{port_name}-partner"));
        if !dir.is_dir() {
            return None;
        }

        Some(Partner {
            supports_usb_power_delivery: read_bool_attr(&dir, "supports_usb_power_delivery"),
            product_type: read_attr(&dir, "type").and_then(|v| v.parse().ok()),
            number_of_alternate_modes: read_attr(&dir, "number_of_alternate_modes")
                .and_then(|v| v.parse().ok()),
            alt_modes: collect_alt_modes(&dir, &format!("{port_name}-partner")),
            port_path: find_device_link(&dir),
        })
    }

    fn parse_cable(port_name: &str, class_root: &Path) -> Option<Cable> {
        let dir = class_root.join(format!("{port_name}-cable"));
        if !dir.is_dir() {
            return None;
        }

        Some(Cable {
            cable_type: read_attr(&dir, "type").and_then(|v| v.parse().ok()),
            plug_type: read_attr(&dir, "plug_type").and_then(|v| v.parse().ok()),
        })
    }

    fn parse_port(name: &str, dir: &Path, class_root: &Path) -> TypecPort {
        let partner = parse_partner(name, class_root);
        TypecPort {
            name: name.to_string(),
            // ports themselves are never the ACPI-linked device; the link is only ever
            // discoverable via the partner's reverse symlink
            port_path: partner.as_ref().and_then(|p| p.port_path.clone()),
            data_role: read_choice_attr(dir, "data_role"),
            power_role: read_choice_attr(dir, "power_role"),
            preferred_role: read_attr(dir, "preferred_role").and_then(|v| v.parse().ok()),
            port_type: read_attr(dir, "port_type").and_then(|v| v.parse().ok()),
            vconn_source: read_bool_attr(dir, "vconn_source"),
            power_operation_mode: read_attr(dir, "power_operation_mode")
                .and_then(|v| v.parse().ok()),
            orientation: read_attr(dir, "orientation").and_then(|v| v.parse().ok()),
            usb_power_delivery_revision: read_attr(dir, "usb_power_delivery_revision"),
            supported_accessory_modes: read_list_attr(dir, "supported_accessory_modes"),
            alt_modes: collect_alt_modes(dir, name),
            partner,
            cable: parse_cable(name, class_root),
        }
    }

    /// Enumerate all USB Type-C ports under `root` (a `/sys/class/typec`-style directory)
    ///
    /// Returns an empty `Vec` (not an error) if `root` does not exist - the typec class is
    /// optional kernel functionality, most systems and VMs will not have it.
    pub fn enumerate_typec_ports(root: &Path) -> Vec<TypecPort> {
        let Ok(entries) = fs::read_dir(root) else {
            return Vec::new();
        };

        let mut ports: Vec<TypecPort> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                // top level ports are named "portN" - partners/cables are "portN-partner" /
                // "portN-cable" siblings, alt-modes are "portN.M" children, skip both here
                if name.starts_with("port")
                    && name["port".len()..].chars().all(|c| c.is_ascii_digit())
                {
                    Some(parse_port(&name, &e.path(), root))
                } else {
                    None
                }
            })
            .collect();

        ports.sort_by(|a, b| a.name.cmp(&b.name));
        ports
    }
}

#[cfg(target_os = "linux")]
pub use sysfs::enumerate_typec_ports;

/// Enumerate USB Type-C ports from the default sysfs location (`/sys/class/typec`)
///
/// Always returns an empty `Vec` on non-Linux platforms - the typec sysfs class is Linux-only.
pub fn enumerate_default_typec_ports() -> Vec<TypecPort> {
    #[cfg(target_os = "linux")]
    {
        enumerate_typec_ports(Path::new(SYSFS_TYPEC_PREFIX))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

#[cfg(all(test, target_os = "linux"))]
mod test {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static FIXTURE_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Build an isolated fixture directory under the OS temp dir; not cleaned up automatically
    /// (left for inspection on failure) but named uniquely per call so parallel tests never clash
    fn fixture_root(test_name: &str) -> std::path::PathBuf {
        let n = FIXTURE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "cyme_typec_test_{}_{}_{}",
            std::process::id(),
            test_name,
            n
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_attr(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn test_current_choice() {
        assert_eq!(current_choice("[host] device"), Some("host"));
        assert_eq!(current_choice("source [sink]"), Some("sink"));
        assert_eq!(current_choice("no brackets here"), None);
    }

    #[test]
    fn test_role_from_str() {
        assert_eq!("host".parse::<DataRole>(), Ok(DataRole::Host));
        assert_eq!("device".parse::<DataRole>(), Ok(DataRole::Device));
        assert!("bogus".parse::<DataRole>().is_err());

        assert_eq!("source".parse::<PowerRole>(), Ok(PowerRole::Source));
        assert_eq!("sink".parse::<PowerRole>(), Ok(PowerRole::Sink));

        assert_eq!("none".parse::<PreferredRole>(), Ok(PreferredRole::None));
    }

    /// Real values from a soup<->dragon Snapdragon X Elite dump (issue #121, penguin42,
    /// 2026-07-17): port1 with data_role independent of power_role
    #[test]
    fn test_parse_port_real_values_port1() {
        let root = fixture_root("port1_real");
        let port1 = root.join("port1");
        fs::create_dir_all(&port1).unwrap();
        write_attr(&port1, "data_role", "[host] device");
        write_attr(&port1, "power_role", "source [sink]");
        write_attr(&port1, "orientation", "normal");
        write_attr(&port1, "vconn_source", "no");
        write_attr(&port1, "usb_power_delivery_revision", "3.0");

        let ports = enumerate_typec_ports(&root);
        assert_eq!(ports.len(), 1);
        let p = &ports[0];
        assert_eq!(p.name, "port1");
        assert_eq!(p.data_role, Some(DataRole::Host));
        // current value is the one NOT in brackets here - power source is the other role
        assert_eq!(p.power_role, Some(PowerRole::Sink));
        assert_eq!(p.orientation, Some(Orientation::Normal));
        assert_eq!(p.vconn_source, Some(false));
        assert_eq!(p.usb_power_delivery_revision.as_deref(), Some("3.0"));
        assert_eq!(p.port_path, None);
        assert!(p.partner.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    /// Real values from the same dump: port0 alt-modes, static local capability slots, no
    /// partner negotiated on this angle of the link (soup:L12565-12566/L6597-6598 per session notes)
    #[test]
    fn test_parse_port_alt_modes() {
        let root = fixture_root("port0_altmodes");
        let port0 = root.join("port0");
        let alt0 = port0.join("port0.0");
        let alt1 = port0.join("port0.1");
        fs::create_dir_all(&alt0).unwrap();
        fs::create_dir_all(&alt1).unwrap();
        write_attr(&alt0, "svid", "8087");
        write_attr(&alt0, "active", "yes");
        write_attr(&alt1, "svid", "ff01");
        write_attr(&alt1, "active", "no");

        let ports = enumerate_typec_ports(&root);
        assert_eq!(ports.len(), 1);
        let modes = ports[0].alt_modes.as_ref().expect("alt modes present");
        assert_eq!(modes.len(), 2);

        let intel = modes.iter().find(|m| m.svid == 0x8087).unwrap();
        assert_eq!(intel.index, 0);
        assert_eq!(intel.active, Some(true));

        let dp = modes.iter().find(|m| m.svid == 0xff01).unwrap();
        assert_eq!(dp.index, 1);
        assert_eq!(dp.active, Some(false));

        let _ = fs::remove_dir_all(&root);
    }

    /// Partner with a negotiated DisplayPort alt-mode, and PD support flag
    #[test]
    fn test_parse_partner_with_negotiated_alt_mode() {
        let root = fixture_root("partner_negotiated");
        let port0 = root.join("port0");
        let partner = root.join("port0-partner");
        let partner_alt0 = partner.join("port0-partner.0");
        fs::create_dir_all(&port0).unwrap();
        fs::create_dir_all(&partner_alt0).unwrap();
        write_attr(&partner, "supports_usb_power_delivery", "yes");
        write_attr(&partner, "type", "hub");
        write_attr(&partner, "number_of_alternate_modes", "1");
        write_attr(&partner_alt0, "svid", "ff01");
        write_attr(&partner_alt0, "active", "yes");

        let ports = enumerate_typec_ports(&root);
        let partner = ports[0].partner.as_ref().expect("partner present");
        assert_eq!(partner.supports_usb_power_delivery, Some(true));
        assert_eq!(partner.product_type, Some(ProductType::Hub));
        assert_eq!(partner.number_of_alternate_modes, Some(1));
        let modes = partner.alt_modes.as_ref().unwrap();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].svid, 0xff01);
        assert_eq!(partner.port_path, None);

        let _ = fs::remove_dir_all(&root);
    }

    /// ACPI-linked partner: the kernel's reverse `typec` symlink is represented in the fixture as
    /// a plain empty file named after the device (real sysfs would be a symlink, but the parser
    /// only reads the entry's *name*, so a plain file is equivalent for this test and avoids
    /// depending on the fixture filesystem supporting symlinks)
    #[test]
    fn test_parse_partner_acpi_device_link() {
        let root = fixture_root("partner_acpi_link");
        let port0 = root.join("port0");
        let partner = root.join("port0-partner");
        fs::create_dir_all(&port0).unwrap();
        fs::create_dir_all(&partner).unwrap();
        write_attr(&partner, "supports_usb_power_delivery", "no");
        // non-device entries that must NOT be mistaken for the device link
        write_attr(&partner, "type", "peripheral");
        fs::create_dir_all(partner.join("identity")).unwrap();
        // the actual device link: named after the linked USB device's sysfs name
        write_attr(&partner, "2-2", "");

        let ports = enumerate_typec_ports(&root);
        let partner = ports[0].partner.as_ref().unwrap();
        assert_eq!(partner.port_path, Some(PortPath::new(2, vec![2])));
        // and it propagates up to the port itself
        assert_eq!(ports[0].port_path, Some(PortPath::new(2, vec![2])));

        let _ = fs::remove_dir_all(&root);
    }

    /// Cable type/plug_type
    #[test]
    fn test_parse_cable() {
        let root = fixture_root("cable");
        let port0 = root.join("port0");
        let cable = root.join("port0-cable");
        fs::create_dir_all(&port0).unwrap();
        fs::create_dir_all(&cable).unwrap();
        write_attr(&cable, "type", "active");
        write_attr(&cable, "plug_type", "type-c");

        let ports = enumerate_typec_ports(&root);
        let cable = ports[0].cable.as_ref().expect("cable present");
        assert_eq!(cable.cable_type, Some(CableType::Active));
        assert_eq!(cable.plug_type, Some(PlugType::TypeC));

        let _ = fs::remove_dir_all(&root);
    }

    /// Multiple ports on the same host, sorted by name, missing attributes are None not an error
    #[test]
    fn test_enumerate_multiple_ports_missing_attrs_are_none() {
        let root = fixture_root("multi_port");
        fs::create_dir_all(root.join("port1")).unwrap();
        fs::create_dir_all(root.join("port0")).unwrap();
        // no attribute files written at all - every optional field should end up None

        let ports = enumerate_typec_ports(&root);
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].name, "port0");
        assert_eq!(ports[1].name, "port1");
        assert_eq!(ports[0].data_role, None);
        assert_eq!(ports[0].power_role, None);
        assert!(ports[0].alt_modes.is_none());
        assert!(ports[0].partner.is_none());
        assert!(ports[0].cable.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_enumerate_nonexistent_root_returns_empty() {
        let root = std::env::temp_dir().join("cyme_typec_test_definitely_does_not_exist_xyz");
        let _ = fs::remove_dir_all(&root);
        assert_eq!(enumerate_typec_ports(&root), Vec::new());
    }
}
