//! Port of names.c in usbutils that provides name lookups for USB data using udev, falling back to USB IDs repository.
//!
//! lsusb uses udev and the bundled hwdb (based on USB IDs) for name lookups. To attempt parity with lsusb, this module uses udev_hwdb if the feature is enabled, otherwise it will fall back to the USB IDs repository. Whilst they both get data from the same source, the bundled udev hwdb might be different due to release version/customisations.
//!
//! The function names match those found in the lsusb source code.
#[allow(unused_imports)]
use crate::error::{Error, ErrorKind};
use usb_ids::{self, FromId};

/// Get name of vendor from [`usb_ids::Vendor`] or [`hwdb_get`] if feature is enabled
///
/// ```
/// use cyme::lsusb::names;
/// assert_eq!(names::vendor(0x1d6b), Some("Linux Foundation".to_owned()));
/// ```
pub fn vendor(vid: u16) -> Option<String> {
    hwdb_get(&format!("usb:v{:04X}*", vid), "ID_VENDOR_FROM_DATABASE")
        .unwrap_or(usb_ids::Vendor::from_id(vid).map(|v| v.name().to_owned()))
}

/// Get name of product from [`usb_ids::Device`] or [`hwdb_get`] if feature is enabled
///
/// ```
/// use cyme::lsusb::names;
/// assert_eq!(names::product(0x1d6b, 0x0003), Some("3.0 root hub".to_owned()));
/// ```
pub fn product(vid: u16, pid: u16) -> Option<String> {
    hwdb_get(
        &format!("usb:v{:04X}p{:04X}*", vid, pid),
        "ID_MODEL_FROM_DATABASE",
    )
    .unwrap_or(usb_ids::Device::from_vid_pid(vid, pid).map(|v| v.name().to_owned()))
}

/// Get name of class from [`usb_ids::Class`] or [`hwdb_get`] if feature is enabled
///
/// ```
/// use cyme::lsusb::names;
/// assert_eq!(names::class(0x03), Some("Human Interface Device".to_owned()));
/// ```
pub fn class(id: u8) -> Option<String> {
    hwdb_get(
        &format!("usb:v*p*d*dc{:02X}*", id),
        "ID_USB_CLASS_FROM_DATABASE",
    )
    .unwrap_or(usb_ids::Class::from_id(id).map(|v| v.name().to_owned()))
}

/// Get name of sub class from [`usb_ids::SubClass`] or [`hwdb_get`] if feature is enabled
///
/// ```
/// use cyme::lsusb::names;
/// assert_eq!(names::subclass(0x02, 0x02), Some("Abstract (modem)".to_owned()));
/// ```
pub fn subclass(cid: u8, scid: u8) -> Option<String> {
    hwdb_get(
        &format!("usb:v*p*d*dc{:02X}dsc{:02X}*", cid, scid),
        "ID_USB_SUBCLASS_FROM_DATABASE",
    )
    .unwrap_or(usb_ids::SubClass::from_cid_scid(cid, scid).map(|v| v.name().to_owned()))
}

/// Get name of protocol from [`usb_ids::Protocol`] or [`hwdb_get`] if feature is enabled
///
/// ```
/// use cyme::lsusb::names;
/// assert_eq!(names::protocol(0x02, 0x02, 0x05), Some("AT-commands (3G)".to_owned()));
/// ```
pub fn protocol(cid: u8, scid: u8, pid: u8) -> Option<String> {
    hwdb_get(
        &format!("usb:v*p*d*dc{:02X}dsc{:02X}dp{:02X}*", cid, scid, pid),
        "ID_USB_PROTOCOL_FROM_DATABASE",
    )
    .unwrap_or(
        usb_ids::Protocol::from_cid_scid_pid(cid, scid, pid).map(|v| v.name().to_owned()),
    )
}

/// Get HID descriptor type name from [`usb_ids::Hid`]
pub fn hid(id: u8) -> Option<String> {
    usb_ids::Hid::from_id(id).map(|v| v.name().to_owned())
}

/// Get HID report tag name from [`usb_ids::HidItemType`]
pub fn report_tag(id: u8) -> Option<String> {
    usb_ids::HidItemType::from_id(id).map(|v| v.name().to_owned())
}

/// Get name of [`usb_ids::HidUsagePage`] from id
pub fn huts(id: u8) -> Option<String> {
    usb_ids::HidUsagePage::from_id(id).map(|v| v.name().to_owned())
}

/// Get name of [`usb_ids::HidUsage`] from page id and usage id
pub fn hutus(page_id: u8, id: u16) -> Option<String> {
    usb_ids::HidUsage::from_pageid_uid(page_id, id).map(|v| v.name().to_owned())
}

/// Get name of [`usb_ids::Language`] from id
pub fn langid(id: u16) -> Option<String> {
    usb_ids::Language::from_id(id).map(|v| v.name().to_owned())
}

/// Get name of [`usb_ids::Phy`] from id
pub fn physdes(id: u8) -> Option<String> {
    usb_ids::Phy::from_id(id).map(|v| v.name().to_owned())
}

/// Get name of [`usb_ids::Bias`] from id
pub fn bias(id: u8) -> Option<String> {
    usb_ids::Bias::from_id(id).map(|v| v.name().to_owned())
}

/// Get name of [`usb_ids::HidCountryCode`] from id
pub fn countrycode(id: u8) -> Option<String> {
    usb_ids::HidCountryCode::from_id(id).map(|v| v.name().to_owned())
}

/// Get name of [`usb_ids::VideoControl`] from id
pub fn videoterminal(id: u16) -> Option<String> {
    usb_ids::VideoTerminal::from_id(id).map(|v| v.name().to_owned())
}

/// Wrapper around [`crate::udev::hwdb_get`] so that it can be 'used' without feature
///
/// Returns `Err` not `None` if feature is not enabled so that with unwrap_or hwdb can still return `None` if no match in db
#[allow(unused_variables)]
fn hwdb_get(modalias: &str, key: &'static str) -> Result<Option<String>, Error> {
    #[cfg(all(target_os = "linux", feature = "udev_hwdb"))]
    return crate::udev::hwdb::get(modalias, key);

    #[cfg(not(all(target_os = "linux", feature = "udev_hwdb")))]
    return Err(Error::new(
        ErrorKind::Unsupported,
        "hwdb_get requires 'udev_hwdb' feature",
    ));
}
