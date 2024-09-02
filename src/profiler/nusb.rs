//! Uses nusb (pure Rust) to get system USB information. Requires 'nusb' feature. Uses [`crate::profiler::types`] types to hold data so that it is cross-compatible with macOS system_profiler command.
use super::*;
use crate::error::{Error, ErrorKind};
use crate::lsusb::names;
use ::nusb;
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
        #[cfg(target_os = "linux")]
        {
            bus.root_hub().into()
        }

        #[cfg(target_os = "windows")]
        {
            let (bcd_usb, protocol) = match bus.controller() {
                Some(nusb::UsbController::XHCI) => (Some(0x300), Some(0x03)),
                Some(nusb::UsbController::EHCI) => (Some(0x200), Some(0x01)),
                Some(nusb::UsbController::OHCI) => (Some(0x110), Some(0x00)),
                Some(nusb::UsbController::VHCI) => (Some(0x000), Some(0x00)),
                _ => (None, None),
            };

            Device {
                vendor_id: bus.pci_info().map(|i| i.vendor_id()),
                product_id: bus.pci_info().map(|i| i.device_id()),
                device_speed: None,
                location_id: DeviceLocation {
                    bus: bus.bus_id().parse::<u8>().unwrap_or(0),
                    number: 0,
                    tree_positions: vec![],
                },
                bcd_device: bus
                    .pci_info()
                    .map(|i| i.revision().map(|r| usb::Version::from_bcd(r)))
                    .flatten(),
                bcd_usb: bcd_usb.map(|v| usb::Version::from_bcd(v)),
                class: Some(usb::ClassCode::Hub),
                sub_class: Some(0),
                protocol,
                name: bus.class_name().map(|s| s.to_string()).unwrap_or_default(),
                manufacturer: bus.provider_class().map(|s| s.to_string()),
                // serial number is the PCI instance on Linux
                serial_num: bus.pci_info().map(|i| i.instance_id().to_string()),
                ..Default::default()
            }
        }

        #[cfg(target_os = "macos")]
        {
            let (bcd_usb, protocol) = match bus.controller() {
                Some(nusb::UsbController::XHCI) => (Some(0x300), Some(0x03)),
                Some(nusb::UsbController::EHCI) => (Some(0x200), Some(0x01)),
                Some(nusb::UsbController::OHCI) => (Some(0x110), Some(0x00)),
                Some(nusb::UsbController::VHCI) => (Some(0x000), Some(0x00)),
                _ => (None, None),
            };

            Device {
                vendor_id: bus.pci_info().map(|i| i.vendor_id()),
                product_id: bus.pci_info().map(|i| i.device_id()),
                device_speed: None,
                location_id: DeviceLocation {
                    bus: bus.bus_id().parse::<u8>().unwrap_or(0),
                    number: 0,
                    tree_positions: vec![],
                },
                bcd_device: bus
                    .pci_info()
                    .and_then(|i| i.revision().map(usb::Version::from_bcd)),
                bcd_usb: bcd_usb.map(usb::Version::from_bcd),
                class: Some(usb::ClassCode::Hub),
                sub_class: Some(0),
                protocol,
                name: bus.class_name().map(|s| s.to_string()).unwrap_or_default(),
                manufacturer: bus.provider_class().map(|s| s.to_string()),
                serial_num: bus.name().map(|s| s.to_string()),
                ..Default::default()
            }
        }
    }
}

impl From<&nusb::BusInfo> for Bus {
    fn from(bus: &nusb::BusInfo) -> Self {
        if let Some(pci_info) = bus.pci_info() {
            let (host_controller_vendor, host_controller_device) =
                match pci_ids::Device::from_vid_pid(pci_info.vendor_id(), pci_info.device_id()) {
                    Some(d) => (
                        Some(d.vendor().name().to_string()),
                        Some(d.name().to_string()),
                    ),
                    None => (None, None),
                };

            Bus {
                usb_bus_number: Some(bus.bus_id().parse::<u8>().unwrap_or(0)),
                name: bus.class_name().map(|s| s.to_string()).unwrap_or_default(),
                host_controller: bus
                    .provider_class()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                host_controller_vendor,
                host_controller_device,
                pci_vendor: Some(pci_info.vendor_id()),
                pci_device: Some(pci_info.device_id()),
                ..Default::default()
            }
        } else {
            Bus {
                usb_bus_number: Some(bus.bus_id().parse::<u8>().unwrap_or(0)),
                name: bus.class_name().map(|s| s.to_string()).unwrap_or_default(),
                host_controller: bus
                    .provider_class()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                ..Default::default()
            }
        }
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
            .or(names::vendor(device_info.vendor_id()))
            .or(usb_ids::Vendor::from_id(device_info.vendor_id()).map(|v| v.name().to_string()));
        let name = device_info
            .product_string()
            .map(|s| s.to_string())
            .or(names::product(
                device_info.vendor_id(),
                device_info.product_id(),
            ))
            .or(
                usb_ids::Device::from_vid_pid(device_info.vendor_id(), device_info.product_id())
                    .map(|d| d.name().to_string()),
            )
            .unwrap_or_default();
        let serial_num = device_info.serial_number().map(|s| s.to_string());

        Device {
            vendor_id: Some(device_info.vendor_id()),
            product_id: Some(device_info.product_id()),
            device_speed,
            location_id: DeviceLocation {
                // nusb bus_id is a string; busnum on Linux (number) will be 0 on Windows as it's a String
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
            name,
            manufacturer,
            serial_num,
            ..Default::default()
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
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            bus_id_map: HashMap::new(),
        }
    }

    fn build_endpoints(
        &self,
        device: &UsbDevice,
        interface_desc: &nusb::descriptors::InterfaceAltSetting,
    ) -> Vec<usb::Endpoint> {
        let mut ret: Vec<usb::Endpoint> = Vec::new();

        for endpoint in interface_desc.endpoints() {
            let endpoint_desc = endpoint.descriptors().next().unwrap();
            let endpoint_extra = endpoint
                .descriptors()
                .skip(1)
                .filter(|d| d.descriptor_type() == 0x05 || d.descriptor_type() == 0x25)
                .flat_map(|d| d.to_vec())
                .collect::<Vec<u8>>();

            ret.push(usb::Endpoint {
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

                let mut interface = usb::Interface {
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
    ) -> Result<Vec<usb::Configuration>> {
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
                // only config descriptors - nusb everything trailing
                .filter(|d| d.descriptor_type() == 0x02)
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
        sp_device: &mut Device,
        with_udev: bool,
    ) -> Result<usb::DeviceExtra> {
        // get the Device Descriptor since not all data is cached
        let device_desc_raw = device
            .handle
            .get_descriptor(0x01, 0x00, 0x00, device.timeout)?;
        let device_desc: usb::DeviceDescriptor =
            usb::DeviceDescriptor::try_from(device_desc_raw.as_slice())?;
        sp_device.bcd_usb = Some(device_desc.usb_version);

        // try to get strings from device descriptors
        if let Some(name) = device.get_descriptor_string(device_desc.product_string_index) {
            sp_device.name = name;
        }

        if let Some(manufacturer) =
            device.get_descriptor_string(device_desc.manufacturer_string_index)
        {
            sp_device.manufacturer = Some(manufacturer);
        }

        if let Some(serial) = device.get_descriptor_string(device_desc.serial_number_string_index) {
            sp_device.serial_num = Some(serial);
        }

        let mut extra = usb::DeviceExtra {
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
        &mut self,
        device_info: &nusb::DeviceInfo,
        with_extra: bool,
    ) -> Result<Device> {
        let mut sp_device: Device = device_info.into();

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
                sp_device.extra = Some(usb::DeviceExtra {
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
    fn get_devices(&mut self, with_extra: bool) -> Result<Vec<Device>> {
        let mut devices = Vec::new();
        for device in nusb::list_devices()? {
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
        let mut root_hubs = HashMap::new();
        for bus in nusb::list_buses()? {
            let device = bus.root_hub();
            // get with extra data only on Linux as others _really_ don't exist
            match self.build_spdevice(&device, true) {
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
                            log::warn!("Non-critical error during profile: {}", e);
                        }
                    });

                    root_hubs.insert(sp_device.location_id.bus, sp_device);
                }
                Err(e) => eprintln!("Failed to get data for {:?}: {}", device, e),
            }
        }

        Ok(root_hubs)
    }

    #[cfg(not(target_os = "linux"))]
    fn get_root_hubs(&mut self) -> Result<HashMap<u8, Device>> {
        let mut root_hubs = HashMap::new();
        for bus in nusb::list_buses()? {
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
        for nusb_bus in nusb::list_buses()? {
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
            #[cfg(target_os = "linux")]
            {
                bus.devices = Some(vec![nusb_bus.root_hub().into()]);
            }

            buses.insert(bus.usb_bus_number.unwrap(), bus);
        }

        Ok(buses)
    }
}

pub(crate) fn fill_spusb(spusb: &mut SystemProfile) -> Result<()> {
    let mut profiler = NusbProfiler::new();
    profiler.fill_spusb(spusb)
}
