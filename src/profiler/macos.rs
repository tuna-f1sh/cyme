//! macOS specific code for USB device profiling.
//!
//! Includes parser for macOS `system_profiler` command -json output with SPUSBDataType. Merged with libusb or nusb for extra data. Also includes IOKit functions for obtaining host controller data - helper code taken from [nusb](https://github.com/kevinmehall/nusb).
//!
//! `system_profiler`: Bus and Device structs are used as deserializers for serde. The JSON output with the -json flag is not really JSON; all values are String regardless of contained data so it requires some extra work. Additionally, some values differ slightly from the non json output such as the speed - it is a description rather than numerical.
use super::*;
use std::process::Command;

use core_foundation::{
    base::{CFType, TCFType},
    data::CFData,
    string::CFString,
    ConcreteCFType,
};
use io_kit_sys::{
    kIOMasterPortDefault, kIORegistryIterateParents, kIORegistryIterateRecursively,
    keys::kIOServicePlane, ret::kIOReturnSuccess, IOIteratorNext, IOObjectRelease,
    IORegistryEntryGetRegistryEntryID, IORegistryEntrySearchCFProperty,
    IOServiceGetMatchingServices, IOServiceNameMatching,
};

pub(crate) struct IoObject(u32);

impl IoObject {
    // Safety: `handle` must be an IOObject handle. Ownership is transferred.
    pub unsafe fn new(handle: u32) -> IoObject {
        IoObject(handle)
    }
    pub fn get(&self) -> u32 {
        self.0
    }
}

impl Drop for IoObject {
    fn drop(&mut self) {
        unsafe {
            IOObjectRelease(self.0);
        }
    }
}

pub(crate) struct IoService(IoObject);

impl IoService {
    // Safety: `handle` must be an IOService handle. Ownership is transferred.
    pub unsafe fn new(handle: u32) -> IoService {
        IoService(IoObject(handle))
    }
    pub fn get(&self) -> u32 {
        self.0 .0
    }
}

pub(crate) struct IoServiceIterator(IoObject);

impl IoServiceIterator {
    // Safety: `handle` must be an IoIterator of IoService. Ownership is transferred.
    pub unsafe fn new(handle: u32) -> IoServiceIterator {
        IoServiceIterator(IoObject::new(handle))
    }
}

impl Iterator for IoServiceIterator {
    type Item = IoService;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let handle = IOIteratorNext(self.0.get());
            if handle != 0 {
                Some(IoService::new(handle))
            } else {
                None
            }
        }
    }
}

pub(crate) struct HostControllerInfo {
    pub(crate) name: String,
    pub(crate) class_name: String,
    pub(crate) io_name: String,
    pub(crate) registry_id: u64,
    pub(crate) vendor_id: u16,
    pub(crate) device_id: u16,
    pub(crate) revision_id: u16,
    pub(crate) class_code: u32,
    pub(crate) subsystem_vendor_id: Option<u16>,
    pub(crate) subsystem_id: Option<u16>,
}

impl std::fmt::Debug for HostControllerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PciControllerInfo")
            .field("name", &self.name)
            .field("class_name", &self.class_name)
            .field("io_name", &self.io_name)
            .field("registry_id", &format!("{:08x}", self.registry_id))
            .field("vendor_id", &format!("{:04x}", self.vendor_id))
            .field("device_id", &format!("{:04x}", self.device_id))
            .field("revision_id", &format!("{:04x}", self.revision_id))
            .field("class_code", &format!("{:08x}", self.class_code))
            .field("subsystem_vendor_id", &self.subsystem_vendor_id)
            .field("subsystem_id", &self.subsystem_id)
            .finish()
    }
}

pub(crate) fn get_registry_id(device: &IoService) -> Option<u64> {
    unsafe {
        let mut out = 0;
        let r = IORegistryEntryGetRegistryEntryID(device.get(), &mut out);

        if r == kIOReturnSuccess {
            Some(out)
        } else {
            // not sure this can actually fail.
            log::debug!("IORegistryEntryGetRegistryEntryID failed with {r}");
            None
        }
    }
}

fn get_property<T: ConcreteCFType>(device: &IoService, property: &'static str) -> Option<T> {
    unsafe {
        let cf_property = CFString::from_static_string(property);

        let raw = IORegistryEntrySearchCFProperty(
            device.get(),
            kIOServicePlane as *mut i8,
            cf_property.as_CFTypeRef() as *const _,
            std::ptr::null(),
            kIORegistryIterateRecursively | kIORegistryIterateParents,
        );

        if raw.is_null() {
            log::debug!("Device does not have property `{property}`");
            return None;
        }

        let res = CFType::wrap_under_create_rule(raw).downcast_into();

        if res.is_none() {
            log::debug!("Failed to convert device property `{property}`");
        }

        res
    }
}

fn get_string_property(device: &IoService, property: &'static str) -> Option<String> {
    get_property::<CFString>(device, property).map(|s| s.to_string())
}

fn get_byte_array_property(device: &IoService, property: &'static str) -> Option<Vec<u8>> {
    let d = get_property::<CFData>(device, property)?;
    Some(d.bytes().to_vec())
}

fn get_ascii_array_property(device: &IoService, property: &'static str) -> Option<String> {
    let d = get_property::<CFData>(device, property)?;
    Some(
        d.bytes()
            .iter()
            .map(|b| *b as char)
            .filter(|c| *c != '\0')
            .collect(),
    )
}

pub(crate) fn probe_controller(device: IoService) -> Option<HostControllerInfo> {
    let registry_id = get_registry_id(&device)?;
    log::debug!("Probing controller {registry_id:08x}");

    // name is a CFData of ASCII characters
    let name = get_ascii_array_property(&device, "name")?;

    let class_name = get_string_property(&device, "IOClass")?;
    let io_name = get_string_property(&device, "IOName")?;

    let vendor_id =
        get_byte_array_property(&device, "vendor-id").map(|v| u16::from_le_bytes([v[0], v[1]]))?;
    let device_id =
        get_byte_array_property(&device, "device-id").map(|v| u16::from_le_bytes([v[0], v[1]]))?;
    let revision_id = get_byte_array_property(&device, "revision-id")
        .map(|v| u16::from_le_bytes([v[0], v[1]]))?;
    let class_code = get_byte_array_property(&device, "class-code")
        .map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))?;
    let subsystem_vendor_id = get_byte_array_property(&device, "subsystem-vendor-id")
        .map(|v| u16::from_le_bytes([v[0], v[1]]));
    let subsystem_id =
        get_byte_array_property(&device, "subsystem-id").map(|v| u16::from_le_bytes([v[0], v[1]]));

    Some(HostControllerInfo {
        name,
        class_name,
        io_name,
        registry_id,
        vendor_id,
        device_id,
        revision_id,
        class_code,
        subsystem_vendor_id,
        subsystem_id,
    })
}

pub(crate) fn get_controller(name: &str) -> Result<HostControllerInfo> {
    unsafe {
        let dictionary = IOServiceNameMatching(name.as_ptr() as *const i8);
        if dictionary.is_null() {
            return Err(Error::new(ErrorKind::IoKit, "IOServiceMatching failed"));
        }

        let mut iterator = 0;
        let r = IOServiceGetMatchingServices(kIOMasterPortDefault, dictionary, &mut iterator);
        if r != kIOReturnSuccess {
            return Err(Error::new(ErrorKind::IoKit, &r.to_string()));
        }

        IoServiceIterator::new(iterator)
            .next()
            .and_then(probe_controller)
            .ok_or(Error::new(
                ErrorKind::IoKit,
                &format!("No controller found for {}", name),
            ))
    }
}

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
