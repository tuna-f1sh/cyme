//! Uses nusb (pure Rust) to get system USB information. Requires 'nusb' feature. Uses [`crate::profiler::types`] types to hold data so that it is cross-compatible with macOS system_profiler command.
use super::*;
use crate::error::{Error, ErrorKind};
use crate::lsusb::names;
use ::nusb;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use usb_ids::{self, FromId};

#[derive(Debug)]
pub(crate) struct NusbProfiler;

pub(crate) struct UsbDevice {
    handle: nusb::Device,
    language: u16,
    vidpid: (u16, u16),
    location: DeviceLocation,
    timeout: std::time::Duration,
}

impl From<ControlRequest> for nusb::transfer::Control {
    fn from(request: ControlRequest) -> Self {
        nusb::transfer::Control {
            control_type: request.control_type.into(),
            request: request.request,
            value: request.value,
            index: request.index,
            recipient: request.recipient.into(),
        }
    }
}

impl From<ControlType> for nusb::transfer::ControlType {
    fn from(control: ControlType) -> Self {
        match control {
            ControlType::Standard => nusb::transfer::ControlType::Standard,
            ControlType::Class => nusb::transfer::ControlType::Class,
            ControlType::Vendor => nusb::transfer::ControlType::Vendor,
        }
    }
}

impl From<Recipient> for nusb::transfer::Recipient {
    fn from(recipient: Recipient) -> Self {
        match recipient {
            Recipient::Device => nusb::transfer::Recipient::Device,
            Recipient::Interface => nusb::transfer::Recipient::Interface,
            Recipient::Endpoint => nusb::transfer::Recipient::Endpoint,
            Recipient::Other => nusb::transfer::Recipient::Other,
        }
    }
}

/// Covert to our crate speed
impl From<nusb::Speed> for usb::Speed {
    fn from(nusb: nusb::Speed) -> Self {
        match nusb {
            nusb::Speed::SuperPlus => usb::Speed::SuperSpeedPlus,
            nusb::Speed::Super => usb::Speed::SuperSpeed,
            nusb::Speed::High => usb::Speed::HighSpeed,
            nusb::Speed::Full => usb::Speed::FullSpeed,
            nusb::Speed::Low => usb::Speed::LowSpeed,
            _ => usb::Speed::Unknown,
        }
    }
}

// SysfsPath, parsing etc is taken from nusb crate (since not pub) and modified for our use
#[derive(Debug, Clone)]
struct SysfsPath(pub(crate) PathBuf);

impl SysfsPath {
    fn parse_attr<T>(&self, attr: &str, parse: impl FnOnce(&str) -> Result<T>) -> Result<T> {
        let attr_path = self.0.join(attr);
        fs::read_to_string(&attr_path)
            .map_err(|e| Error::new(ErrorKind::Io, &e.to_string()))
            .and_then(|v| parse(v.trim()))
    }

    pub(crate) fn read_attr<T: FromStr>(&self, attr: &str) -> Result<T> {
        self.parse_attr(attr, |s| {
            s.parse().map_err(|_| {
                Error::new(
                    ErrorKind::Parsing,
                    &format!("Failed to parse attr: {}", attr),
                )
            })
        })
    }

    fn read_attr_hex<T: FromHexStr>(&self, attr: &str) -> Result<T> {
        self.parse_attr(attr, |s| T::from_hex_str(s))
    }

    fn children(&self) -> impl Iterator<Item = SysfsPath> {
        fs::read_dir(&self.0)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|f| f.ok())
            .filter(|f| f.file_type().ok().is_some_and(|t| t.is_dir()))
            .map(|f| SysfsPath(f.path()))
    }
}

trait FromHexStr: Sized {
    fn from_hex_str(s: &str) -> Result<Self>;
}

impl FromHexStr for u8 {
    fn from_hex_str(s: &str) -> Result<Self> {
        u8::from_str_radix(s, 16).map_err(|_| Error::new(ErrorKind::Parsing, s))
    }
}

impl FromHexStr for u16 {
    fn from_hex_str(s: &str) -> Result<Self> {
        u16::from_str_radix(s, 16).map_err(|_| Error::new(ErrorKind::Parsing, s))
    }
}

const SYSFS_PREFIX: &str = "/sys/bus/usb/devices/";

#[cfg(target_os = "linux")]
fn probe_device(path: SysfsPath) -> Result<USBDevice> {
    let busnum: u8 = path.read_attr("busnum")?;
    let device_address = path.read_attr("devnum")?;
    let class: u8 = path.read_attr("bDeviceClass")?;
    let sub_class: u8 = path.read_attr("bDeviceSubClass")?;
    let protocol: u8 = path.read_attr("bDeviceProtocol")?;
    let speed: usb::Speed = path
        .read_attr("speed")
        .map(|s: String| s.parse().unwrap_or(usb::Speed::Unknown))
        .unwrap_or(usb::Speed::Unknown);

    Ok(USBDevice {
        name: path.read_attr("product")?,
        vendor_id: path.read_attr_hex("idVendor").ok(),
        product_id: path.read_attr_hex("idProduct").ok(),
        location_id: DeviceLocation {
            bus: busnum,
            number: device_address,
            tree_positions: vec![],
        },
        device_speed: Some(DeviceSpeed::SpeedValue(speed)),
        bcd_device: Some(usb::Version::from_bcd(path.read_attr_hex("bcdDevice")?)),
        bcd_usb: None,
        class: Some(usb::ClassCode::from(class)),
        sub_class: Some(sub_class),
        protocol: Some(protocol),
        manufacturer: path.read_attr("manufacturer").ok(),
        serial_num: path.read_attr("serial").ok(),
        ..Default::default()
    })
}

impl UsbOperations for UsbDevice {
    fn get_descriptor_string(&self, string_index: u8) -> Option<String> {
        if string_index == 0 {
            return None;
        }
        self.handle
            .get_string_descriptor(string_index, self.language, self.timeout)
            .map(|s| s.to_string())
            .ok()
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn get_control_msg(&self, control_request: &ControlRequest) -> Result<Vec<u8>> {
        let mut data = vec![0; control_request.length];
        let nusb_control: nusb::transfer::Control = (*control_request).into();
        let n = self
            .handle
            .control_in_blocking(nusb_control, data.as_mut_slice(), self.timeout)
            .map_err(|e| Error {
                kind: ErrorKind::Nusb,
                message: format!("Failed to get control message: {}", e),
            })?;

        if n < control_request.length {
            log::debug!(
                "Failed to get full control message, only read {} of {}",
                n,
                control_request.length
            );
            return Err(Error {
                kind: ErrorKind::Nusb,
                message: format!(
                    "Failed to get full control message, only read {} of {}",
                    n, control_request.length
                ),
            });
        }

        Ok(data)
    }

    #[cfg(target_os = "windows")]
    fn get_control_msg(&self, control_request: &ControlRequest) -> Result<Vec<u8>> {
        let mut data = vec![0; control_request.length];
        let nusb_control: nusb::transfer::Control = control_request.clone().into();
        // TODO this should probably be dependant on the interface being called?
        let interface = self.handle.claim_interface(0)?;
        let n = interface
            .control_in_blocking(nusb_control, data.as_mut_slice(), self.timeout)
            .map_err(|e| Error {
                kind: ErrorKind::Nusb,
                message: format!("Failed to get control message: {}", e),
            })?;

        if n < control_request.length {
            log::debug!(
                "Failed to get full control message, only read {} of {}",
                n,
                control_request.length
            );
            return Err(Error {
                kind: ErrorKind::Nusb,
                message: format!(
                    "Failed to get full control message, only read {} of {}",
                    n, control_request.length
                ),
            });
        }

        Ok(data)
    }
}

impl NusbProfiler {
    fn build_endpoints(
        &self,
        device: &UsbDevice,
        interface_desc: &nusb::descriptors::InterfaceAltSetting,
    ) -> Vec<usb::USBEndpoint> {
        let mut ret: Vec<usb::USBEndpoint> = Vec::new();

        for endpoint in interface_desc.endpoints() {
            let endpoint_desc = endpoint.descriptors().next().unwrap();
            let endpoint_extra = endpoint
                .descriptors()
                .skip(1)
                .filter(|d| d.descriptor_type() == 0x05 || d.descriptor_type() == 0x25)
                .flat_map(|d| d.to_vec())
                .collect::<Vec<u8>>();

            ret.push(usb::USBEndpoint {
                address: usb::EndpointAddress::from(endpoint.address()),
                transfer_type: usb::TransferType::from(endpoint.transfer_type() as u8),
                sync_type: usb::SyncType::from(endpoint.transfer_type() as u8),
                usage_type: usb::UsageType::from(endpoint.transfer_type() as u8),
                max_packet_size: endpoint.max_packet_size() as u16,
                interval: endpoint.interval(),
                length: endpoint_desc[0],
                extra: self
                    .build_endpoint_descriptor_extra(
                        device,
                        (
                            interface_desc.class(),
                            interface_desc.subclass(),
                            interface_desc.protocol(),
                        ),
                        interface_desc.interface_number(),
                        endpoint_extra,
                    )
                    .ok()
                    .flatten(),
            });
        }

        ret
    }

    fn build_interfaces(
        &self,
        device: &UsbDevice,
        config: &nusb::descriptors::Configuration,
        with_udev: bool,
    ) -> Result<Vec<usb::USBInterface>> {
        let mut ret: Vec<usb::USBInterface> = Vec::new();

        for interface in config.interfaces() {
            for interface_alt in interface.alt_settings() {
                let path = usb::get_interface_path(
                    device.location.bus,
                    &device.location.tree_positions,
                    config.configuration_value(),
                    interface_alt.interface_number(),
                );

                let interface_desc = interface_alt.descriptors().next().unwrap();
                let interface_extra = interface_alt
                    .descriptors()
                    .skip(1)
                    // only want device and interface descriptors - nusb everything trailing
                    .filter(|d| {
                        (d.descriptor_type() & 0x0F) == 0x04 || (d.descriptor_type() & 0x0F) == 0x01
                    })
                    .flat_map(|d| d.to_vec())
                    .collect::<Vec<u8>>();

                let mut interface = usb::USBInterface {
                    name: get_sysfs_string(&path, "interface")
                        .or(interface_alt
                            .string_index()
                            .and_then(|i| device.get_descriptor_string(i)))
                        .unwrap_or_default(),
                    string_index: interface_alt.string_index().unwrap_or(0),
                    number: interface_alt.interface_number(),
                    path,
                    class: usb::ClassCode::from(interface_alt.class()),
                    sub_class: interface_alt.subclass(),
                    protocol: interface_alt.subclass(),
                    alt_setting: interface_alt.alternate_setting(),
                    driver: None,
                    syspath: None,
                    length: interface_desc[0],
                    endpoints: self.build_endpoints(device, &interface_alt),
                    extra: self
                        .build_interface_descriptor_extra(
                            device,
                            (
                                interface_alt.class(),
                                interface_alt.subclass(),
                                interface_alt.protocol(),
                            ),
                            interface_alt.interface_number(),
                            interface_extra,
                        )
                        .ok(),
                };

                // flag allows us to try again without udev if it raises an error
                // but record the error for printing
                if with_udev {
                    interface.driver = get_udev_driver_name(&interface.path)?;
                    interface.syspath = get_udev_syspath(&interface.path)?;
                };

                ret.push(interface);
            }
        }

        Ok(ret)
    }

    fn build_configurations(
        &self,
        device: &UsbDevice,
        with_udev: bool,
    ) -> Result<Vec<usb::USBConfiguration>> {
        let mut ret: Vec<usb::USBConfiguration> = Vec::new();

        for c in device.handle.configurations() {
            let mut attributes = Vec::new();
            if c.attributes() & 0x10 != 0 {
                attributes.push(usb::ConfigAttributes::BatteryPowered);
            }
            if c.attributes() & 0x20 != 0 {
                attributes.push(usb::ConfigAttributes::RemoteWakeup);
            }
            if c.attributes() & 0x40 != 0 {
                attributes.push(usb::ConfigAttributes::SelfPowered);
            }

            let config_desc = c.descriptors().next().unwrap();
            let config_extra = c
                .descriptors()
                .skip(1)
                // only config descriptors - nusb everything trailing
                .filter(|d| d.descriptor_type() == 0x02)
                .flat_map(|d| d.to_vec())
                .collect::<Vec<u8>>();
            let total_length = u16::from_le_bytes(config_desc[2..4].try_into().unwrap());

            ret.push(usb::USBConfiguration {
                name: c
                    .string_index()
                    .and_then(|i| device.get_descriptor_string(i))
                    .unwrap_or_default(),
                string_index: c.string_index().unwrap_or(0),
                number: c.configuration_value(),
                attributes,
                max_power: NumericalUnit {
                    value: c.max_power() as u32,
                    unit: String::from("mA"),
                    description: None,
                },
                length: config_desc.len() as u8,
                total_length,
                interfaces: self.build_interfaces(device, &c, with_udev)?,
                extra: self
                    .build_config_descriptor_extra(device, config_extra)
                    .ok(),
            });
        }

        Ok(ret)
    }

    fn build_spdevice_extra(
        &self,
        device: &UsbDevice,
        sp_device: &mut USBDevice,
        with_udev: bool,
    ) -> Result<usb::USBDeviceExtra> {
        // get the Device Descriptor since not all data is cached
        let device_desc_raw = device
            .handle
            .get_descriptor(0x01, 0x00, 0x00, device.timeout)?;
        let device_desc: usb::DeviceDescriptor =
            usb::DeviceDescriptor::try_from(device_desc_raw.as_slice())?;
        sp_device.bcd_usb = Some(device_desc.usb_version);

        // try to get strings from device descriptors
        if let Ok(name) = device
            .handle
            .get_string_descriptor(device_desc.product_string_index, 0, device.timeout)
            .map(|s| s.to_string())
        {
            sp_device.name = name;
        }

        if let Ok(manufacturer) = device
            .handle
            .get_string_descriptor(device_desc.manufacturer_string_index, 0, device.timeout)
            .map(|s| s.to_string())
        {
            sp_device.manufacturer = Some(manufacturer);
        }

        if let Ok(serial) = device
            .handle
            .get_string_descriptor(device_desc.serial_number_string_index, 0, device.timeout)
            .map(|s| s.to_string())
        {
            sp_device.serial_num = Some(serial);
        }

        let mut extra = usb::USBDeviceExtra {
            max_packet_size: device_desc.max_packet_size,
            string_indexes: (
                device_desc.product_string_index,
                device_desc.manufacturer_string_index,
                device_desc.serial_number_string_index,
            ),
            driver: None,
            syspath: get_syspath(&sp_device.sysfs_name()),
            // These are idProduct, idVendor in lsusb - from udev_hwdb/usb-ids - not device descriptor
            vendor: names::vendor(device_desc.vendor_id)
                .or(usb_ids::Vendor::from_id(device_desc.vendor_id).map(|v| v.name().to_owned())),
            product_name: names::product(device_desc.vendor_id, device_desc.product_id).or(
                usb_ids::Device::from_vid_pid(device_desc.vendor_id, device_desc.product_id)
                    .map(|v| v.name().to_owned()),
            ),
            configurations: self.build_configurations(device, with_udev)?,
            status: Self::get_device_status(device).ok(),
            debug: Self::get_debug_descriptor(device).ok(),
            binary_object_store: None,
            qualifier: None,
            hub: None,
        };

        // flag allows us to try again without udev if it raises an nting
        // but record the error for printing
        if with_udev {
            let sysfs_name = sp_device.sysfs_name();
            extra.driver = get_udev_driver_name(&sysfs_name)?;
            extra.syspath = get_udev_syspath(&sysfs_name)?;
        }

        // Get device specific stuff: bos, hub, dualspeed, debug and status
        if device_desc.usb_version >= usb::Version::from_bcd(0x0201) {
            extra.binary_object_store = Self::get_bos_descriptor(device).ok();
        }
        if device_desc.usb_version >= usb::Version::from_bcd(0x0200) {
            extra.qualifier = Self::get_device_qualifier(device).ok();
        }

        if device_desc.device_class == usb::ClassCode::Hub as u8 {
            let has_ssp = if let Some(bos) = &extra.binary_object_store {
                bos.capabilities
                    .iter()
                    .any(|c| matches!(c, usb::descriptors::bos::BosCapability::SuperSpeedPlus(_)))
            } else {
                false
            };
            let bcd = sp_device.bcd_usb.map_or(0x0100, |v| v.into());
            extra.hub =
                Self::get_hub_descriptor(device, device_desc.device_protocol, bcd, has_ssp).ok();
        }

        Ok(extra)
    }

    fn build_spdevice(
        &self,
        device_info: &nusb::DeviceInfo,
        with_extra: bool,
    ) -> Result<USBDevice> {
        let speed = device_info.speed().map(|s| {
            let s = usb::Speed::from(s);
            DeviceSpeed::SpeedValue(s)
        });

        let mut sp_device = USBDevice {
            vendor_id: Some(device_info.vendor_id()),
            product_id: Some(device_info.product_id()),
            device_speed: speed,
            location_id: DeviceLocation {
                // nusb bus_id is a string; busnum on Linux (number)
                bus: device_info.bus_id().parse::<u8>().unwrap_or(0),
                number: device_info.device_address(),
                tree_positions: device_info.port_chain().to_vec(),
            },
            bcd_device: Some(usb::Version::from_bcd(device_info.device_version())),
            // gets added on the extra read
            bcd_usb: None,
            class: Some(usb::ClassCode::from(device_info.class())),
            sub_class: Some(device_info.subclass()),
            protocol: Some(device_info.protocol()),
            ..Default::default()
        };

        sp_device.manufacturer = device_info
            .manufacturer_string()
            .map(|s| s.to_string())
            .or(get_sysfs_string(&sp_device.sysfs_name(), "manufacturer"))
            .or(names::vendor(device_info.vendor_id()))
            .or(usb_ids::Vendor::from_id(device_info.vendor_id()).map(|v| v.name().to_string()));
        sp_device.name = device_info
            .product_string()
            .map(|s| s.to_string())
            .or(get_sysfs_string(&sp_device.sysfs_name(), "product"))
            .or(names::product(
                device_info.vendor_id(),
                device_info.product_id(),
            ))
            .or(
                usb_ids::Device::from_vid_pid(device_info.vendor_id(), device_info.product_id())
                    .map(|d| d.name().to_string()),
            )
            .unwrap_or_default();
        sp_device.serial_num = device_info
            .serial_number()
            .map(|s| s.to_string())
            .or(get_sysfs_string(&sp_device.sysfs_name(), "serial"));

        if with_extra {
            if let Ok(device) = device_info.open() {
                // get the first language - proably US English
                let languages: Vec<u16> = device
                    .get_string_descriptor_supported_languages(std::time::Duration::from_secs(1))
                    .map(|i| i.collect())
                    .unwrap_or_default();
                let language = languages
                    .first()
                    .copied()
                    .unwrap_or(nusb::descriptors::language_id::US_ENGLISH);

                sp_device.profiler_error = {
                    let usb_device = UsbDevice {
                        handle: device,
                        language,
                        vidpid: (device_info.vendor_id(), device_info.product_id()),
                        location: sp_device.location_id.clone(),
                        timeout: std::time::Duration::from_secs(1),
                    };

                    match self.build_spdevice_extra(&usb_device, &mut sp_device, true) {
                        Ok(extra) => {
                            sp_device.extra = Some(extra);
                            None
                        }
                        Err(e) => {
                            // try again without udev if we have that feature but return message so device still added
                            if cfg!(feature = "udev") && e.kind() == ErrorKind::Udev {
                                sp_device.extra = Some(self.build_spdevice_extra(
                                    &usb_device,
                                    &mut sp_device,
                                    false,
                                )?);
                                Some(format!(
                                        "Failed to get udev data for {}, probably requires elevated permissions",
                                        sp_device
                                ))
                            } else {
                                Some(format!("Failed to get some extra data for {}, probably requires elevated permissions: {}", sp_device, e))
                            }
                        }
                    }
                };
            } else {
                log::warn!("Failed to open device for extra data: {:04x}:{:04x}. Ensure user has USB access permissions: https://docs.rs/nusb/latest/nusb/#linux", device_info.vendor_id(), device_info.product_id());
                sp_device.profiler_error = Some(
                    "Failed to open device, extra data incomplete and possibly inaccurate"
                        .to_string(),
                );
                sp_device.extra = Some(usb::USBDeviceExtra {
                    max_packet_size: device_info.max_packet_size_0(),
                    // nusb doesn't have these cached
                    string_indexes: (0, 0, 0),
                    driver: None,
                    syspath: get_syspath(&sp_device.sysfs_name()),
                    vendor: names::vendor(device_info.vendor_id())
                        .or(usb_ids::Vendor::from_id(device_info.vendor_id())
                            .map(|v| v.name().to_owned())),
                    product_name: names::product(device_info.vendor_id(), device_info.product_id())
                        .or(usb_ids::Device::from_vid_pid(
                            device_info.vendor_id(),
                            device_info.product_id(),
                        )
                        .map(|v| v.name().to_owned())),
                    configurations: vec![],
                    status: None,
                    debug: None,
                    binary_object_store: None,
                    qualifier: None,
                    hub: None,
                });
            }
        }

        Ok(sp_device)
    }
}

impl Profiler<UsbDevice> for NusbProfiler {
    fn get_devices(&self, with_extra: bool) -> Result<Vec<USBDevice>> {
        let mut devices = Vec::new();
        for device in nusb::list_devices()? {
            match self.build_spdevice(&device, with_extra) {
                Ok(sp_device) => {
                    devices.push(sp_device.to_owned());
                    let print_stderr =
                        std::env::var_os("CYME_PRINT_NON_CRITICAL_PROFILER_STDERR").is_some();

                    // print any non-critical error during extra capture
                    sp_device.profiler_error.iter().for_each(|e| {
                        if print_stderr {
                            eprintln!("{}", e);
                        } else {
                            log::warn!("Non-critical error during profile: {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Failed to get data for {:?}: {}", device, e),
            }
        }

        Ok(devices)
    }

    #[cfg(target_os = "linux")]
    fn get_root_hubs(&self) -> Result<HashMap<u8, USBDevice>> {
        let ret = fs::read_dir(SYSFS_PREFIX)?
            .flat_map(|entry| {
                let path = entry.ok()?.path();
                let name = path.file_name()?;

                // just root_hubs
                if name.to_string_lossy().starts_with("usb") {
                    probe_device(SysfsPath(path)).ok()
                } else {
                    None
                }
            })
            .map(|hub| (hub.location_id.bus, hub))
            .collect::<HashMap<_, _>>();

        Ok(ret)
    }

    #[cfg(not(target_os = "linux"))]
    fn get_root_hubs(&self) -> Result<HashMap<u8, USBDevice>> {
        Ok(HashMap::new())
    }
}

pub(crate) fn fill_spusb(spusb: &mut SPUSBDataType) -> Result<()> {
    let profiler = NusbProfiler;
    profiler.fill_spusb(spusb)
}
