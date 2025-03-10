//! Uses nusb (pure Rust) to get system USB information. Requires 'nusb' feature. Uses [`crate::profiler::types`] types to hold data so that it is cross-compatible with macOS system_profiler command.
use super::*;
use crate::error::{Error, ErrorKind};
use crate::lsusb::names;
use crate::types::NumericalUnit;
use ::nusb::{self, MaybeFuture};
use usb_ids::{self, FromId};

#[derive(Debug)]
pub(crate) struct NusbProfiler {
    #[cfg(target_os = "windows")]
    bus_id_map: HashMap<String, u8>,
}

pub(crate) struct UsbDevice {
    handle: nusb::Device,
    language: u16,
    vidpid: (u16, u16),
    location: DeviceLocation,
    timeout: std::time::Duration,
}

impl std::fmt::Debug for UsbDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UsbDevice {{ vidpid: {:#04x}:{:#04x}, location: {} }}",
            self.vidpid.0,
            self.vidpid.1,
            self.location.port_path().display()
        )
    }
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

/// Convert a bus into a root hub device - following the same sort of abstraction as Linux
impl From<&nusb::BusInfo> for Device {
    fn from(bus: &nusb::BusInfo) -> Self {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            // should use profiler for extra data not this into
            bus.root_hub().into()
        }

        #[cfg(target_os = "windows")]
        {
            let (bcd_device, protocol) = match bus.controller_type() {
                Some(nusb::UsbControllerType::XHCI) => (Some(0x300), Some(0x03)),
                Some(nusb::UsbControllerType::EHCI) => (Some(0x200), Some(0x01)),
                Some(nusb::UsbControllerType::OHCI) => (Some(0x110), Some(0x00)),
                Some(nusb::UsbControllerType::VHCI) => (Some(0x000), Some(0x00)),
                _ => (None, None),
            };

            let (vendor_id, product_id) = if let Some(pci_info) = platform::pci_info_from_bus(bus) {
                (Some(pci_info.vendor_id), Some(pci_info.product_id))
            } else {
                (None, None)
            };

            Device {
                vendor_id,
                product_id,
                device_speed: None,
                location_id: DeviceLocation {
                    bus: 0,
                    number: 0,
                    tree_positions: vec![],
                },
                bcd_device: bcd_device.map(usb::Version::from_bcd),
                bcd_usb: None,
                class: Some(usb::BaseClass::Hub),
                sub_class: Some(0),
                protocol,
                name: bus.system_name().map(|s| s.to_string()).unwrap_or_default(),
                manufacturer: None,
                // serial number is the PCI instance on Linux
                serial_num: Some(bus.parent_instance_id().to_string_lossy().to_string()),
                ..Default::default()
            }
        }

        #[cfg(target_os = "macos")]
        {
            let (bcd_device, protocol) = match bus.controller_type() {
                Some(nusb::UsbControllerType::XHCI) => (Some(0x300), Some(0x03)),
                Some(nusb::UsbControllerType::EHCI) => (Some(0x200), Some(0x01)),
                Some(nusb::UsbControllerType::OHCI) => (Some(0x110), Some(0x00)),
                Some(nusb::UsbControllerType::VHCI) => (Some(0x000), Some(0x00)),
                _ => (None, None),
            };

            Device {
                vendor_id: None,
                product_id: None,
                device_speed: None,
                location_id: DeviceLocation {
                    // macOS bus_id is a hex string
                    bus: u8::from_str_radix(bus.bus_id(), 16).expect(
                        "Failed to parse bus_id: macOS bus_id should be a hex string and not None",
                    ),
                    number: 0,
                    tree_positions: vec![],
                },
                bcd_device: bcd_device.map(usb::Version::from_bcd),
                bcd_usb: None,
                class: Some(usb::BaseClass::Hub),
                sub_class: Some(0),
                protocol,
                name: bus.class_name().to_string(),
                manufacturer: Some(bus.provider_class_name().to_string()),
                serial_num: bus.name().map(|s| s.to_string()),
                ..Default::default()
            }
        }
    }
}

impl From<&nusb::BusInfo> for Bus {
    fn from(bus: &nusb::BusInfo) -> Self {
        platform::from(bus)
    }
}

impl From<&nusb::DeviceInfo> for Device {
    fn from(device_info: &nusb::DeviceInfo) -> Self {
        let device_speed = device_info.speed().map(|s| {
            let s = usb::Speed::from(s);
            DeviceSpeed::SpeedValue(s)
        });

        let manufacturer = device_info
            .manufacturer_string()
            .map(|s| s.to_string())
            .or_else(|| names::vendor(device_info.vendor_id()))
            .or_else(|| {
                usb_ids::Vendor::from_id(device_info.vendor_id()).map(|v| v.name().to_string())
            });
        let name = device_info
            .product_string()
            .map(|s| s.to_string())
            .or_else(|| names::product(device_info.vendor_id(), device_info.product_id()))
            .or_else(|| {
                usb_ids::Device::from_vid_pid(device_info.vendor_id(), device_info.product_id())
                    .map(|d| d.name().to_string())
            })
            .unwrap_or_default();
        let serial_num = device_info.serial_number().map(|s| s.to_string());

        let bus_no = if cfg!(target_os = "macos") {
            // macOS bus_id is a hex string
            u8::from_str_radix(device_info.bus_id(), 16)
                .expect("Failed to parse bus_id: macOS bus_id should be a hex string and not None")
        } else if cfg!(target_os = "linux") || cfg!(target_os = "android") {
            // Linux bus_id is a string decimal
            device_info.bus_id().parse::<u8>().expect(
                "Failed to parse bus_id: Linux bus_id should be a decimal string and not None",
            )
        } else {
            // Windows bus_id is a string string so 0
            0
        };

        Device {
            vendor_id: Some(device_info.vendor_id()),
            product_id: Some(device_info.product_id()),
            device_speed,
            location_id: DeviceLocation {
                bus: bus_no,
                number: device_info.device_address(),
                tree_positions: device_info.port_chain().to_vec(),
            },
            bcd_device: Some(usb::Version::from_bcd(device_info.device_version())),
            // gets added on the extra read
            bcd_usb: None,
            class: Some(usb::BaseClass::from(device_info.class())),
            sub_class: Some(device_info.subclass()),
            protocol: Some(device_info.protocol()),
            id: Some(device_info.id()),
            name,
            manufacturer,
            serial_num,
            ..Default::default()
        }
    }
}

impl UsbDevice {
    fn control_in(
        &self,
        control_request: &ControlRequest,
        data: &mut Vec<u8>,
        clear_halt: bool,
    ) -> Result<usize> {
        let nusb_control: nusb::transfer::Control = (*control_request).into();
        // Windows *ALWAYS* needs to claim the interface and self.handle.control_in_blocking isn't defined
        #[cfg(target_os = "windows")]
        let ret = {
            // requires detech_and_claim_interface on Linux if mod is loaded
            // not nice though just for profiling - maybe add a flag to claim or not?
            let interface = self
                .handle
                .claim_interface(control_request.index as u8)
                .wait()?;
            if clear_halt {
                interface.clear_halt(0).wait()?;
            }
            interface.control_in_blocking(nusb_control, data.as_mut_slice(), self.timeout)
        };

        #[cfg(not(target_os = "windows"))]
        let ret = {
            if control_request.claim_interface | clear_halt {
                // requires detech_and_claim_interface on Linux if mod is loaded
                // not nice though just for profiling - maybe add a flag to claim or not?
                let interface = self
                    .handle
                    .claim_interface(control_request.index as u8)
                    .wait()?;
                if clear_halt {
                    interface.clear_halt(0).wait()?;
                }
                interface.control_in_blocking(nusb_control, data.as_mut_slice(), self.timeout)
            } else {
                self.handle
                    .control_in_blocking(nusb_control, data.as_mut_slice(), self.timeout)
            }
        };

        ret.map_err(|e| match e {
            nusb::transfer::TransferError::Stall => Error {
                kind: ErrorKind::TransferStall,
                message: "Endpoint in a STALL condition".to_string(),
            },
            _ => Error {
                kind: ErrorKind::Nusb,
                message: format!("Failed to get control message: {}", e),
            },
        })
    }

    /// Retry control request if it fails due to STALL - following a claim interface and clear halt
    fn control_in_retry(
        &self,
        control_request: &ControlRequest,
        data: &mut Vec<u8>,
    ) -> Result<usize> {
        match self.control_in(control_request, data, false) {
            Ok(n) => Ok(n),
            Err(Error {
                kind: ErrorKind::TransferStall,
                ..
            }) => self
                .control_in(control_request, data, true)
                .map_err(|e| Error {
                    kind: ErrorKind::Nusb,
                    message: format!("Failed to get control message: {}", e),
                }),
            Err(e) => Err(Error {
                kind: ErrorKind::Nusb,
                message: format!("Failed to get control message: {}", e),
            }),
        }
    }
}

impl UsbOperations for UsbDevice {
    fn get_descriptor_string(&self, string_index: u8) -> Option<String> {
        if string_index == 0 {
            return None;
        }
        self.handle
            .get_string_descriptor(string_index, self.language, self.timeout)
            .map(|s| s.chars().filter(|c| !c.is_control()).collect())
            .ok()
    }

    fn get_control_msg(&self, control_request: ControlRequest) -> Result<Vec<u8>> {
        let mut data = vec![0; control_request.length];
        let n = self.control_in_retry(&control_request, &mut data)?;

        if n < control_request.length {
            log::debug!(
                "{:?} Failed to get full control message: read {} of {} bytes",
                self,
                n,
                control_request.length
            );
            return Err(Error {
                kind: ErrorKind::Nusb,
                message: format!(
                    "{:?} Failed to get full control message: read {} of {} bytes",
                    self, n, control_request.length
                ),
            });
        }

        Ok(data)
    }
}

impl NusbProfiler {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            bus_id_map: HashMap::new(),
        }
    }

    fn build_endpoints(
        &self,
        device: &UsbDevice,
        interface_desc: &nusb::descriptors::InterfaceDescriptor,
    ) -> Vec<usb::Endpoint> {
        let mut ret: Vec<usb::Endpoint> = Vec::new();

        for endpoint in interface_desc.endpoints() {
            let endpoint_desc = endpoint.descriptors().next().unwrap();
            let endpoint_extra = endpoint
                .descriptors()
                .skip(1)
                // no filter as all _should_ be endpoint descriptors at this point
                .flat_map(|d| d.to_vec())
                .collect::<Vec<u8>>();

            ret.push(usb::Endpoint {
                address: usb::EndpointAddress::from(endpoint.address()),
                transfer_type: usb::TransferType::from(endpoint.attributes()),
                sync_type: usb::SyncType::from(endpoint.attributes()),
                usage_type: usb::UsageType::from(endpoint.attributes()),
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
                internal: InternalData::default(),
            });
        }

        ret
    }

    fn build_interfaces(
        &self,
        device: &UsbDevice,
        config: &nusb::descriptors::ConfigurationDescriptor,
    ) -> Result<Vec<usb::Interface>> {
        let mut ret: Vec<usb::Interface> = Vec::new();

        for interface in config.interfaces() {
            for interface_alt in interface.alt_settings() {
                let path = usb::get_interface_path(
                    device.location.bus,
                    &device.location.tree_positions,
                    config.configuration_value(),
                    interface_alt.interface_number(),
                );
                let path = path.to_str().unwrap();

                let interface_desc = interface_alt.descriptors().next().unwrap();
                let interface_extra = interface_alt
                    .descriptors()
                    .skip(1)
                    // only want device and interface descriptors - nusb has everything trailing including endpoint
                    .take_while(|d| d.descriptor_type() != 0x05)
                    .flat_map(|d| d.to_vec())
                    .collect::<Vec<u8>>();

                let interface = usb::Interface {
                    name: get_sysfs_string(path, "interface").or_else(|| {
                        interface_alt
                            .string_index()
                            .and_then(|i| device.get_descriptor_string(i))
                    }),
                    string_index: interface_alt.string_index().unwrap_or(0),
                    number: interface_alt.interface_number(),
                    class: usb::BaseClass::from(interface_alt.class()),
                    sub_class: interface_alt.subclass(),
                    protocol: interface_alt.protocol(),
                    alt_setting: interface_alt.alternate_setting(),
                    driver: get_sysfs_readlink(path, "driver")
                        .or_else(|| get_udev_driver_name(path).ok().flatten()),
                    syspath: get_syspath(&path).or_else(|| get_udev_syspath(&path).ok().flatten()),
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
                    path: path.to_string(),
                    internal: InternalData::default(),
                };

                ret.push(interface);
            }
        }

        Ok(ret)
    }

    fn build_configurations(&self, device: &UsbDevice) -> Result<Vec<usb::Configuration>> {
        let mut ret: Vec<usb::Configuration> = Vec::new();

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
                // nusb has everything trailing so take until interfaces
                .take_while(|d| d.descriptor_type() != 0x04)
                .flat_map(|d| d.to_vec())
                .collect::<Vec<u8>>();
            let total_length = u16::from_le_bytes(config_desc[2..4].try_into().unwrap());

            ret.push(usb::Configuration {
                name: c
                    .string_index()
                    .and_then(|i| device.get_descriptor_string(i))
                    .unwrap_or_default(),
                string_index: c.string_index().unwrap_or(0),
                number: c.configuration_value(),
                attributes,
                max_power: NumericalUnit {
                    // *2 because nusb returns in 2mA units
                    value: (c.max_power() as u32 * 2),
                    unit: String::from("mA"),
                    description: None,
                },
                length: config_desc.len() as u8,
                total_length,
                interfaces: self.build_interfaces(device, &c)?,
                extra: self
                    .build_config_descriptor_extra(device, config_extra)
                    .ok(),
                ..Default::default()
            });
        }

        Ok(ret)
    }

    fn build_spdevice_extra(
        &self,
        device: &UsbDevice,
        sp_device: &mut Device,
    ) -> Result<usb::DeviceExtra> {
        // nusb has this cached in handle.device_descriptor - convert to our type
        let device_desc: usb::DeviceDescriptor =
            usb::DeviceDescriptor::try_from(device.handle.device_descriptor().as_bytes())?;
        sp_device.bcd_usb = Some(device_desc.usb_version);

        // try to get strings from device descriptors
        // if missing
        if sp_device.name.is_empty() {
            if let Some(name) = device.get_descriptor_string(device_desc.product_string_index) {
                sp_device.name = name;
            }
        }

        if sp_device.manufacturer.is_none() {
            if let Some(manufacturer) =
                device.get_descriptor_string(device_desc.manufacturer_string_index)
            {
                sp_device.manufacturer = Some(manufacturer);
            }
        }

        if sp_device.serial_num.is_none() {
            if let Some(serial) =
                device.get_descriptor_string(device_desc.serial_number_string_index)
            {
                sp_device.serial_num = Some(serial);
            }
        }

        let sysfs_name = sp_device.sysfs_name().display().to_string();
        let mut extra = usb::DeviceExtra {
            max_packet_size: device_desc.max_packet_size,
            string_indexes: (
                device_desc.product_string_index,
                device_desc.manufacturer_string_index,
                device_desc.serial_number_string_index,
            ),
            driver: get_sysfs_readlink(&sysfs_name, "driver")
                .or_else(|| get_udev_driver_name(&sysfs_name).ok().flatten()),
            syspath: get_syspath(&sysfs_name)
                .or_else(|| get_udev_syspath(&sysfs_name).ok().flatten()),
            // These are idProduct, idVendor in lsusb - from udev_hwdb/usb-ids - not device descriptor
            vendor: names::vendor(device_desc.vendor_id).or_else(|| {
                usb_ids::Vendor::from_id(device_desc.vendor_id).map(|v| v.name().to_owned())
            }),
            product_name: names::product(device_desc.vendor_id, device_desc.product_id).or_else(
                || {
                    usb_ids::Device::from_vid_pid(device_desc.vendor_id, device_desc.product_id)
                        .map(|v| v.name().to_owned())
                },
            ),
            configurations: self.build_configurations(device)?,
            status: Self::get_device_status(device).ok(),
            debug: Self::get_debug_descriptor(device).ok(),
            binary_object_store: None,
            qualifier: None,
            hub: None,
        };

        // Get device specific stuff: bos, hub, dualspeed, debug and status
        if device_desc.usb_version >= usb::Version::from_bcd(0x0201) {
            extra.binary_object_store = Self::get_bos_descriptor(device).ok();
        }
        if device_desc.usb_version >= usb::Version::from_bcd(0x0200) {
            extra.qualifier = Self::get_device_qualifier(device).ok();
        }

        if device_desc.device_class == usb::BaseClass::Hub as u8 {
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

    pub(crate) fn build_spdevice(
        &mut self,
        device_info: &nusb::DeviceInfo,
        with_extra: bool,
    ) -> Result<Device> {
        let mut sp_device: Device = device_info.into();

        let generic_extra = |sysfs_name: &str| {
            usb::DeviceExtra {
                max_packet_size: device_info.max_packet_size_0(),
                // nusb doesn't have these cached
                string_indexes: (0, 0, 0),
                driver: get_sysfs_readlink(sysfs_name, "driver")
                    .or_else(|| get_udev_driver_name(sysfs_name).ok().flatten()),
                syspath: get_syspath(sysfs_name)
                    .or_else(|| get_udev_syspath(sysfs_name).ok().flatten()),
                vendor: names::vendor(device_info.vendor_id()).or_else(|| {
                    usb_ids::Vendor::from_id(device_info.vendor_id()).map(|v| v.name().to_owned())
                }),
                product_name: names::product(device_info.vendor_id(), device_info.product_id())
                    .or_else(|| {
                        usb_ids::Device::from_vid_pid(
                            device_info.vendor_id(),
                            device_info.product_id(),
                        )
                        .map(|v| v.name().to_owned())
                    }),
                configurations: vec![],
                status: None,
                debug: None,
                binary_object_store: None,
                qualifier: None,
                hub: None,
            }
        };

        if with_extra {
            if let Ok(device) = device_info.open().wait() {
                // get the first language - probably US English
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

                    match self.build_spdevice_extra(&usb_device, &mut sp_device) {
                        Ok(extra) => {
                            sp_device.extra = Some(extra);
                            None
                        }
                        Err(e) => {
                            sp_device.extra =
                                Some(generic_extra(&sp_device.sysfs_name().display().to_string()));
                            Some(format!("Failed to get some extra data for {}, probably requires elevated permissions: {}", sp_device, e))
                        }
                    }
                };
            } else {
                log::warn!("Failed to open device for extra data: {:04x}:{:04x}. Ensure user has USB access permissions: https://docs.rs/nusb/latest/nusb", device_info.vendor_id(), device_info.product_id());
                sp_device.profiler_error = Some(
                    "Failed to open device, extra data incomplete and possibly inaccurate"
                        .to_string(),
                );
                sp_device.extra =
                    Some(generic_extra(&sp_device.sysfs_name().display().to_string()));
            }
        }

        Ok(sp_device)
    }
}

impl Profiler<UsbDevice> for NusbProfiler {
    fn get_devices(&mut self, with_extra: bool) -> Result<Vec<Device>> {
        let mut devices = Vec::new();
        for device in nusb::list_devices().wait()? {
            match self.build_spdevice(&device, with_extra) {
                #[allow(unused_mut)]
                Ok(mut sp_device) => {
                    #[cfg(target_os = "windows")]
                    {
                        // Windows doesn't have a bus number for root hubs, so we use the index
                        // and assign devices based on serial number
                        if let Some(existing_no) = self.bus_id_map.get(device.bus_id()) {
                            sp_device.location_id.bus = *existing_no;
                        } else {
                            let bus = self.bus_id_map.len() as u8;
                            self.bus_id_map.insert(device.bus_id().to_owned(), bus);
                            sp_device.location_id.bus = bus;
                        }
                    }
                    devices.push(sp_device.to_owned());

                    let print_stderr =
                        std::env::var_os("CYME_PRINT_NON_CRITICAL_PROFILER_STDERR").is_some();

                    // print any non-critical error during extra capture
                    sp_device.profiler_error.iter().for_each(|e| {
                        if print_stderr {
                            eprintln!("{}", e);
                        } else {
                            log::warn!("Non-critical error during profile of {:?}: {}", device, e);
                        }
                    });
                }
                Err(e) => eprintln!("Failed to get data for {:?}: {}", device, e),
            }
        }

        Ok(devices)
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn get_root_hubs(&mut self) -> Result<HashMap<u8, Device>> {
        let mut root_hubs = HashMap::new();
        for bus in nusb::list_buses().wait()? {
            let device = bus.root_hub();
            // get with extra data only on Linux as others _really_ don't exist
            match self.build_spdevice(device, true) {
                #[allow(unused_mut)]
                Ok(mut sp_device) => {
                    if !sp_device.is_root_hub() {
                        return Err(Error::new(
                            ErrorKind::InvalidDevice,
                            &format!(
                                "Device {} returned by nusb::list_root_hubs is not a root hub!",
                                sp_device
                            ),
                        ));
                    }
                    let print_stderr =
                        std::env::var_os("CYME_PRINT_NON_CRITICAL_PROFILER_STDERR").is_some();

                    // print any non-critical error during extra capture
                    sp_device.profiler_error.iter().for_each(|e| {
                        if print_stderr {
                            eprintln!("{}", e);
                        } else {
                            log::warn!("Non-critical error during profile of {}: {}", sp_device, e);
                        }
                    });

                    root_hubs.insert(sp_device.location_id.bus, sp_device);
                }
                Err(e) => eprintln!("Failed to get data for {:?}: {}", device, e),
            }
        }

        Ok(root_hubs)
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    fn get_root_hubs(&mut self) -> Result<HashMap<u8, Device>> {
        let mut root_hubs = HashMap::new();
        for bus in nusb::list_buses().wait()? {
            #[allow(unused_mut)]
            let mut device: Device = Device::from(&bus);

            #[cfg(target_os = "windows")]
            {
                if let Some(existing_no) = self.bus_id_map.get(bus.bus_id()) {
                    device.location_id.bus = *existing_no;
                } else {
                    let bus_no = self.bus_id_map.len() as u8;
                    self.bus_id_map.insert(bus.bus_id().to_owned(), bus_no);
                    device.location_id.bus = bus_no;
                }
            }

            root_hubs.insert(device.location_id.bus, device);
        }

        Ok(root_hubs)
    }

    fn get_buses(&mut self) -> Result<HashMap<u8, Bus>> {
        let mut buses = HashMap::new();
        for nusb_bus in nusb::list_buses().wait()? {
            #[allow(unused_mut)]
            let mut bus: Bus = Bus::from(&nusb_bus);

            // Windows doesn't have a bus number for root hubs, so we track the bus_id string
            #[cfg(target_os = "windows")]
            {
                if let Some(existing_no) = self.bus_id_map.get(nusb_bus.bus_id()) {
                    bus.usb_bus_number = Some(*existing_no);
                } else {
                    let bus_no = self.bus_id_map.len() as u8;
                    self.bus_id_map.insert(nusb_bus.bus_id().to_owned(), bus_no);
                    bus.usb_bus_number = Some(bus_no);
                }
            }

            // add root hub to devices like lsusb on Linux since they are displayed like devices
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                let sp_device = self.build_spdevice(nusb_bus.root_hub(), true)?;
                bus.devices = Some(vec![sp_device]);
            }

            buses.insert(
                bus.usb_bus_number
                    .expect("Bus has no usb_bus_number, unable to use as key"),
                bus,
            );
        }

        Ok(buses)
    }
}

pub(crate) fn fill_spusb(spusb: &mut SystemProfile) -> Result<()> {
    let mut profiler = NusbProfiler::new();
    profiler.fill_spusb(spusb)
}
