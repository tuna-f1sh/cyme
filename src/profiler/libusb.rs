//! Uses rusb (upto date libusb fork) to get system USB information - same lib as lsusb. Requires 'libusb' feature. Uses [`crate::profiler::types`] types to hold data so that it is cross-compatible with macOS system_profiler command.
use super::*;
use crate::error::{Error, ErrorKind};
use crate::lsusb::names;
use rusb as libusb;
use usb_ids::{self, FromId};

#[derive(Debug)]
pub(crate) struct LibUsbProfiler;

pub(crate) struct UsbDevice<T: libusb::UsbContext> {
    handle: libusb::DeviceHandle<T>,
    language: libusb::Language,
    vidpid: (u16, u16),
    location: DeviceLocation,
    timeout: std::time::Duration,
}

/// Set log level for rusb
pub fn set_log_level(debug: u8) {
    let log_level = match debug {
        0 => rusb::LogLevel::None,
        1 => rusb::LogLevel::Warning,
        2 => rusb::LogLevel::Info,
        _ => rusb::LogLevel::Debug,
    };

    rusb::set_log_level(log_level);
}

impl ControlRequest {
    fn get_request_type_in(&self) -> u8 {
        libusb::request_type(
            libusb::Direction::In,
            self.control_type.into(),
            self.recipient.into(),
        )
    }
}

impl From<ControlType> for libusb::RequestType {
    fn from(ct: ControlType) -> Self {
        match ct {
            ControlType::Standard => libusb::RequestType::Standard,
            ControlType::Class => libusb::RequestType::Class,
            ControlType::Vendor => libusb::RequestType::Vendor,
        }
    }
}

impl From<Recipient> for libusb::Recipient {
    fn from(r: Recipient) -> Self {
        match r {
            Recipient::Device => libusb::Recipient::Device,
            Recipient::Interface => libusb::Recipient::Interface,
            Recipient::Endpoint => libusb::Recipient::Endpoint,
            Recipient::Other => libusb::Recipient::Other,
        }
    }
}

impl From<libusb::Error> for Error {
    fn from(error: libusb::Error) -> Self {
        Error {
            kind: ErrorKind::LibUSB,
            message: format!(
                "Failed to gather system USB data from libusb: Error({})",
                &error.to_string()
            ),
        }
    }
}

/// Covert to our crate speed
impl From<libusb::Speed> for usb::Speed {
    fn from(libusb: libusb::Speed) -> Self {
        match libusb {
            libusb::Speed::SuperPlus => usb::Speed::SuperSpeedPlus,
            libusb::Speed::Super => usb::Speed::SuperSpeed,
            libusb::Speed::High => usb::Speed::HighSpeed,
            libusb::Speed::Full => usb::Speed::FullSpeed,
            libusb::Speed::Low => usb::Speed::LowSpeed,
            _ => usb::Speed::Unknown,
        }
    }
}

impl From<libusb::Direction> for usb::Direction {
    fn from(libusb: libusb::Direction) -> Self {
        match libusb {
            libusb::Direction::Out => usb::Direction::Out,
            libusb::Direction::In => usb::Direction::In,
        }
    }
}

impl From<libusb::TransferType> for usb::TransferType {
    fn from(libusb: libusb::TransferType) -> Self {
        match libusb {
            libusb::TransferType::Control => usb::TransferType::Control,
            libusb::TransferType::Isochronous => usb::TransferType::Isochronous,
            libusb::TransferType::Bulk => usb::TransferType::Bulk,
            libusb::TransferType::Interrupt => usb::TransferType::Interrupt,
        }
    }
}

impl From<libusb::UsageType> for usb::UsageType {
    fn from(libusb: libusb::UsageType) -> Self {
        match libusb {
            libusb::UsageType::Data => usb::UsageType::Data,
            libusb::UsageType::Feedback => usb::UsageType::Feedback,
            libusb::UsageType::FeedbackData => usb::UsageType::FeedbackData,
            libusb::UsageType::Reserved => usb::UsageType::Reserved,
        }
    }
}

impl From<libusb::SyncType> for usb::SyncType {
    fn from(libusb: libusb::SyncType) -> Self {
        match libusb {
            libusb::SyncType::NoSync => usb::SyncType::None,
            libusb::SyncType::Asynchronous => usb::SyncType::Asynchronous,
            libusb::SyncType::Adaptive => usb::SyncType::Adaptive,
            libusb::SyncType::Synchronous => usb::SyncType::Synchronous,
        }
    }
}

impl From<libusb::Version> for usb::Version {
    fn from(libusb: libusb::Version) -> Self {
        usb::Version(libusb.major(), libusb.minor(), libusb.sub_minor())
    }
}

/// Attempt to retrieve the current bConfigurationValue and iConfiguration for a device
/// This will only return the current configuration, not all possible configurations
/// If there are any failures in retrieving the data, None is returned
#[allow(unused_variables)]
fn get_sysfs_configuration_string(sysfs_name: &str) -> Option<(u8, String)> {
    #[cfg(target_os = "linux")]
    // Determine bConfigurationValue value on linux
    match get_sysfs_string(sysfs_name, "bConfigurationValue") {
        Some(s) => match s.parse::<u8>() {
            Ok(v) => {
                // Determine iConfiguration
                get_sysfs_string(sysfs_name, "configuration").map(|s| (v, s))
            }
            Err(_) => None,
        },
        None => None,
    }

    #[cfg(not(target_os = "linux"))]
    None
}

impl<T: libusb::UsbContext> UsbOperations for UsbDevice<T> {
    /// Get string descriptor from device
    ///
    /// Returns None if string_index is 0 - reserved for language codes
    fn get_descriptor_string(&self, string_index: u8) -> Option<String> {
        if string_index == 0 {
            return None;
        }
        self.handle
            .read_string_descriptor(self.language, string_index, self.timeout)
            .map(|s| s.trim().trim_end_matches('\0').to_string())
            .ok()
    }

    /// Get control message from device, ensuring message of [`ControlRequest`] length is read
    fn get_control_msg(&self, control_request: &ControlRequest) -> Result<Vec<u8>> {
        let mut buf = vec![0; control_request.length];
        let n = self
            .handle
            .read_control(
                control_request.get_request_type_in(),
                control_request.request,
                control_request.value,
                control_request.index,
                &mut buf,
                self.timeout,
            )
            .map_err(|e| Error {
                kind: ErrorKind::LibUSB,
                message: format!("Failed to get control message: {}", e),
            })?;
        if n < control_request.length {
            log::warn!(
                "Failed to read full control message for {}: {} < {}",
                control_request.request,
                n,
                control_request.length
            );
            Err(Error {
                kind: ErrorKind::LibUSB,
                message: "Control message too short".to_string(),
            })
        } else {
            Ok(buf)
        }
    }
}

impl LibUsbProfiler {
    fn build_endpoints<T: libusb::UsbContext>(
        &self,
        handle: &UsbDevice<T>,
        interface_desc: &libusb::InterfaceDescriptor,
    ) -> Vec<usb::Endpoint> {
        let mut ret: Vec<usb::Endpoint> = Vec::new();

        for endpoint_desc in interface_desc.endpoint_descriptors() {
            let extra_desc = if let Some(extra) = endpoint_desc.extra() {
                self.build_endpoint_descriptor_extra(
                    handle,
                    (
                        interface_desc.class_code(),
                        interface_desc.sub_class_code(),
                        interface_desc.protocol_code(),
                    ),
                    interface_desc.interface_number(),
                    extra.to_vec(),
                )
                .ok()
                .flatten()
            } else {
                None
            };

            ret.push(usb::Endpoint {
                address: usb::EndpointAddress {
                    address: endpoint_desc.address(),
                    number: endpoint_desc.number(),
                    direction: usb::Direction::from(endpoint_desc.direction()),
                },
                transfer_type: usb::TransferType::from(endpoint_desc.transfer_type()),
                sync_type: usb::SyncType::from(endpoint_desc.sync_type()),
                usage_type: usb::UsageType::from(endpoint_desc.usage_type()),
                max_packet_size: endpoint_desc.max_packet_size(),
                interval: endpoint_desc.interval(),
                length: endpoint_desc.length(),
                extra: extra_desc,
            });
        }

        ret
    }

    fn build_interfaces<T: libusb::UsbContext>(
        &self,
        handle: &UsbDevice<T>,
        config_desc: &libusb::ConfigDescriptor,
        with_udev: bool,
    ) -> Result<Vec<usb::Interface>> {
        let mut ret: Vec<usb::Interface> = Vec::new();

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                let path = usb::get_interface_path(
                    handle.location.bus,
                    &handle.location.tree_positions,
                    config_desc.number(),
                    interface_desc.interface_number(),
                );

                let mut interface = usb::Interface {
                    name: get_sysfs_string(&path, "interface")
                        .or(interface_desc
                            .description_string_index()
                            .and_then(|i| handle.get_descriptor_string(i)))
                        .unwrap_or_default(),
                    string_index: interface_desc.description_string_index().unwrap_or(0),
                    number: interface_desc.interface_number(),
                    path,
                    class: usb::ClassCode::from(interface_desc.class_code()),
                    sub_class: interface_desc.sub_class_code(),
                    protocol: interface_desc.protocol_code(),
                    alt_setting: interface_desc.setting_number(),
                    driver: None,
                    syspath: None,
                    length: interface_desc.length(),
                    endpoints: self.build_endpoints(handle, &interface_desc),
                    extra: self
                        .build_interface_descriptor_extra(
                            handle,
                            (
                                interface_desc.class_code(),
                                interface_desc.sub_class_code(),
                                interface_desc.protocol_code(),
                            ),
                            interface_desc.interface_number(),
                            interface_desc.extra().to_vec(),
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

    fn build_configurations<T: libusb::UsbContext>(
        &self,
        device: &libusb::Device<T>,
        handle: &UsbDevice<T>,
        device_desc: &libusb::DeviceDescriptor,
        sp_device: &Device,
        with_udev: bool,
    ) -> Result<Vec<usb::Configuration>> {
        // Retrieve the current configuration (if available)
        let cur_config = get_sysfs_configuration_string(&sp_device.sysfs_name());
        let mut ret: Vec<usb::Configuration> = Vec::new();

        for n in 0..device_desc.num_configurations() {
            let config_desc = match device.config_descriptor(n) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut attributes = Vec::new();
            if config_desc.remote_wakeup() {
                attributes.push(usb::ConfigAttributes::RemoteWakeup);
            }

            if config_desc.self_powered() {
                attributes.push(usb::ConfigAttributes::SelfPowered);
            }

            // Check if we have a cached iConfiguration string
            let config_name = if let Some((config_num, ref config_name)) = cur_config {
                // Configs start from 1, not 0
                if config_num - 1 == n {
                    Some(config_name.clone())
                } else {
                    None
                }
            } else {
                None
            };

            ret.push(usb::Configuration {
                name: config_desc
                    .description_string_index()
                    .and_then(|i| handle.get_descriptor_string(i))
                    .or(config_name)
                    .unwrap_or(String::new()),
                string_index: config_desc.description_string_index().unwrap_or(0),
                number: config_desc.number(),
                attributes,
                max_power: NumericalUnit {
                    value: config_desc.max_power() as u32,
                    unit: String::from("mA"),
                    description: None,
                },
                length: config_desc.length(),
                total_length: config_desc.total_length(),
                interfaces: self.build_interfaces(handle, &config_desc, with_udev)?,
                extra: self
                    .build_config_descriptor_extra(handle, config_desc.extra().to_vec())
                    .ok(),
            });
        }

        Ok(ret)
    }

    #[allow(unused_variables)]
    fn build_spdevice_extra<T: libusb::UsbContext>(
        &self,
        device: &libusb::Device<T>,
        handle: &UsbDevice<T>,
        device_desc: &libusb::DeviceDescriptor,
        sp_device: &mut Device,
        with_udev: bool,
    ) -> Result<usb::DeviceExtra> {
        // attempt to get manufacturer and product strings from device itself
        sp_device.manufacturer = device_desc
            .manufacturer_string_index()
            .and_then(|i| handle.get_descriptor_string(i));

        if let Some(name) = device_desc
            .product_string_index()
            .and_then(|i| handle.get_descriptor_string(i))
        {
            sp_device.name = name;
        }

        sp_device.serial_num = device_desc
            .serial_number_string_index()
            .and_then(|i| handle.get_descriptor_string(i));

        let mut extra = usb::DeviceExtra {
            max_packet_size: device_desc.max_packet_size(),
            string_indexes: (
                device_desc.product_string_index().unwrap_or(0),
                device_desc.manufacturer_string_index().unwrap_or(0),
                device_desc.serial_number_string_index().unwrap_or(0),
            ),
            driver: None,
            syspath: None,
            // These are idProduct, idVendor in lsusb - from udev_hwdb/usb-ids
            vendor: names::vendor(device_desc.vendor_id())
                .or(usb_ids::Vendor::from_id(device_desc.vendor_id()).map(|v| v.name().to_owned())),
            product_name: names::product(device_desc.vendor_id(), device_desc.product_id()).or(
                usb_ids::Device::from_vid_pid(device_desc.vendor_id(), device_desc.product_id())
                    .map(|v| v.name().to_owned()),
            ),
            configurations: self.build_configurations(
                device,
                handle,
                device_desc,
                sp_device,
                with_udev,
            )?,
            status: Self::get_device_status(handle).ok(),
            debug: Self::get_debug_descriptor(handle).ok(),
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
        if device_desc.usb_version() >= rusb::Version::from_bcd(0x0201) {
            extra.binary_object_store = Self::get_bos_descriptor(handle).ok();
        }
        if device_desc.usb_version() >= rusb::Version::from_bcd(0x0200) {
            extra.qualifier = Self::get_device_qualifier(handle).ok();
        }
        if device_desc.class_code() == usb::ClassCode::Hub as u8 {
            let has_ssp = if let Some(bos) = &extra.binary_object_store {
                bos.capabilities
                    .iter()
                    .any(|c| matches!(c, usb::descriptors::bos::BosCapability::SuperSpeedPlus(_)))
            } else {
                false
            };
            let bcd = sp_device.bcd_usb.map_or(0x0100, |v| v.into());
            extra.hub =
                Self::get_hub_descriptor(handle, device_desc.protocol_code(), bcd, has_ssp).ok();
        }

        Ok(extra)
    }

    fn open_device<T: libusb::UsbContext>(
        &self,
        device: &libusb::Device<T>,
        device_desc: &libusb::DeviceDescriptor,
    ) -> Result<UsbDevice<T>> {
        let timeout = std::time::Duration::from_secs(1);
        let handle = device.open()?;
        let language = match handle.read_languages(timeout) {
            Ok(l) => {
                if l.is_empty() {
                    return Err(Error {
                        kind: ErrorKind::LibUSB,
                        message: format!(
                            "Languages for {:?} are empty, will be unable to obtain all data",
                            device
                        ),
                    });
                }
                l[0]
            }
            Err(e) => {
                return Err(Error {
                    kind: ErrorKind::LibUSB,
                    message: format!(
                        "Could not read languages for {:?}, will be unable to obtain all data: {}",
                        device, e
                    ),
                });
            }
        };

        Ok(UsbDevice {
            handle,
            language,
            vidpid: (device_desc.vendor_id(), device_desc.product_id()),
            location: DeviceLocation {
                bus: device.bus_number(),
                number: device.address(),
                tree_positions: device.port_numbers()?,
            },
            timeout,
        })
    }

    /// Builds a [`Device`] from a [`libusb::Device`] by using `device_descriptor()` and intrograting for configuration strings. Optionally with `with_extra` will gather full device information, including from udev if feature is present.
    ///
    /// [`Device.profiler_error`] `Option<String>` will contain any non-critical error during gather of `with_extra` data - normally due to permissions preventing open of device descriptors.
    fn build_spdevice<T: libusb::UsbContext>(
        &self,
        device: &libusb::Device<T>,
        with_extra: bool,
    ) -> Result<Device> {
        let speed = match usb::Speed::from(device.speed()) {
            usb::Speed::Unknown => None,
            v => Some(DeviceSpeed::SpeedValue(v)),
        };

        let device_desc = device.device_descriptor()?;

        let mut sp_device = Device {
            vendor_id: Some(device_desc.vendor_id()),
            product_id: Some(device_desc.product_id()),
            device_speed: speed,
            location_id: DeviceLocation {
                bus: device.bus_number(),
                number: device.address(),
                tree_positions: device.port_numbers()?,
            },
            bcd_device: Some(device_desc.device_version().into()),
            bcd_usb: Some(device_desc.usb_version().into()),
            class: Some(usb::ClassCode::from(device_desc.class_code())),
            sub_class: Some(device_desc.sub_class_code()),
            protocol: Some(device_desc.protocol_code()),
            ..Default::default()
        };

        // sysfs cache
        sp_device.name = get_sysfs_string(&sp_device.sysfs_name(), "product")
            // udev-hwdb
            .or(names::product(
                device_desc.vendor_id(),
                device_desc.product_id(),
            ))
            // usb-ids
            .or(
                usb_ids::Device::from_vid_pid(device_desc.vendor_id(), device_desc.product_id())
                    .map(|device| device.name().to_owned()),
            )
            // empty
            .unwrap_or_default();

        // sysfs cache
        sp_device.manufacturer = get_sysfs_string(&sp_device.sysfs_name(), "manufacturer")
            // udev-hwdb
            .or(names::vendor(device_desc.vendor_id())) // udev, usb-ids if error
            // usb-ids
            .or(usb_ids::Vendor::from_id(device_desc.vendor_id())
                .map(|vendor| vendor.name().to_owned()));

        sp_device.serial_num = get_sysfs_string(&sp_device.sysfs_name(), "serial");

        if with_extra {
            if let Ok(handle) = self.open_device(device, &device_desc) {
                sp_device.profiler_error = {
                    match self.build_spdevice_extra(
                        device,
                        &handle,
                        &device_desc,
                        &mut sp_device,
                        true,
                    ) {
                        Ok(extra) => {
                            sp_device.extra = Some(extra);
                            None
                        }
                        Err(e) => {
                            // try again without udev if we have that feature but return message so device still added
                            if cfg!(feature = "udev") && e.kind() == ErrorKind::Udev {
                                sp_device.extra = Some(self.build_spdevice_extra(
                                    device,
                                    &handle,
                                    &device_desc,
                                    &mut sp_device,
                                    false,
                                )?);
                                Some(format!(
                                    "Failed to get udev data for {}, probably requires elevated permissions",
                                    sp_device
                                ))
                            } else {
                                Some(format!(
                                    "Failed to get some extra data for {}, probably requires elevated permissions: {}",
                                    sp_device, e
                                ))
                            }
                        }
                    }
                }
            } else {
                log::warn!("Failed to open device {:?} for extra data", device);
                sp_device.profiler_error = Some("Failed to open device for extra data".to_string());
                sp_device.extra = Some(usb::DeviceExtra {
                    max_packet_size: device_desc.max_packet_size(),
                    string_indexes: (
                        device_desc.product_string_index().unwrap_or(0),
                        device_desc.manufacturer_string_index().unwrap_or(0),
                        device_desc.serial_number_string_index().unwrap_or(0),
                    ),
                    driver: None,
                    syspath: None,
                    vendor: names::vendor(device_desc.vendor_id())
                        .or(usb_ids::Vendor::from_id(device_desc.vendor_id())
                            .map(|v| v.name().to_owned())),
                    product_name: names::product(device_desc.vendor_id(), device_desc.product_id())
                        .or(usb_ids::Device::from_vid_pid(
                            device_desc.vendor_id(),
                            device_desc.product_id(),
                        )
                        .map(|v| v.name().to_owned())),
                    configurations: Vec::new(),
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

impl<C: libusb::UsbContext> Profiler<UsbDevice<C>> for LibUsbProfiler {
    fn get_devices(&mut self, with_extra: bool) -> Result<Vec<Device>> {
        let mut devices = Vec::new();
        // run through devices building Device types - not root_hubs (port number 0)
        for device in libusb::DeviceList::new()?
            .iter()
            .filter(|d| d.port_number() != 0)
        {
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
    fn get_root_hubs(&mut self) -> Result<HashMap<u8, Device>> {
        let mut ret = HashMap::new();

        for device in libusb::DeviceList::new()?
            .iter()
            .filter(|d| d.port_number() == 0)
        {
            if let Ok(sp_device) = self.build_spdevice(&device, true) {
                ret.insert(sp_device.location_id.bus, sp_device);
            }
        }

        Ok(ret)
    }

    #[cfg(not(target_os = "linux"))]
    fn get_root_hubs(&mut self) -> Result<HashMap<u8, Device>> {
        Ok(HashMap::new())
    }

    fn get_buses(&mut self) -> Result<HashMap<u8, Bus>> {
        <LibUsbProfiler as Profiler<UsbDevice<rusb::Context>>>::get_root_hubs(self).map(|hubs| {
            hubs.into_iter()
                .filter_map(|(k, d)| match Bus::try_from(d) {
                    Ok(b) => Some((k, b)),
                    Err(e) => {
                        eprintln!("Failed to convert root hub to bus: {}", e);
                        None
                    }
                })
                .collect()
        })
    }
}

pub(crate) fn fill_spusb(spusb: &mut SystemProfile) -> Result<()> {
    let mut profiler = LibUsbProfiler;
    <LibUsbProfiler as Profiler<UsbDevice<rusb::Context>>>::fill_spusb(&mut profiler, spusb)
}
