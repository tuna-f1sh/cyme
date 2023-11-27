//! Originally based on [libusb list_devices.rs example](https://github.com/dcuddeback/libusb-rs/blob/master/examples/list_devices.rs), attempts to mimic lsusb output and provide cross-platform [`crate::system_profiler::SPUSBDataType`] getter
//!
//! The [lsusb source code](https://github.com/gregkh/usbutils/blob/master/lsusb.c) was used as a reference for a lot of the styling and content of the display module
#[cfg(feature = "libusb")]
pub mod profiler {
    //! Uses rusb (upto date libusb fork) to get system USB information, most of which has parity with lsusb. Requires 'libusb' feature. Uses [`crate::system_profiler`] types to hold data so that it is cross-compatible with macOS system_profiler command.
    //!
    //! lsusb uses udev for tree building, which libusb does not have access to and is Linux only. udev-rs is used on Linux to attempt to mirror the output of lsusb on Linux. On other platforms, certain information like driver used cannot be obtained.
    //!
    //! Get [`system_profiler::SPUSBDataType`] struct of system USB buses and devices with extra data like configs, interfaces and endpoints
    //! ```no_run
    //! use cyme::lsusb::profiler;
    //!
    //! let spusb = profiler::get_spusb_with_extra(true).unwrap();
    //! // print with alternative styling (#) is using utf-8 icons
    //! println!("{:#}", spusb);
    //! ```
    //!
    //! See [`system_profiler`] docs for what can be done with returned data, such as [`system_profiler::USBFilter`]
    use crate::error::{self, Error, ErrorKind};
    use itertools::Itertools;
    use rusb as libusb;
    use std::collections::HashMap;
    use std::time::Duration;
    use usb_ids::{self, FromId};

    #[cfg(all(target_os = "linux", feature = "udev"))]
    use crate::udev;
    use crate::{system_profiler, types::NumericalUnit, usb};

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

    struct UsbDevice<T: libusb::UsbContext> {
        handle: libusb::DeviceHandle<T>,
        language: libusb::Language,
        timeout: Duration,
    }

    /// Set log level for rusb
    pub fn set_log_level(debug: u8) {
        let log_level = match debug {
            0 => rusb::LogLevel::None,
            1 => rusb::LogLevel::Info,
            _ => rusb::LogLevel::Debug,
        };

        rusb::set_log_level(log_level);
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

    #[allow(unused_variables)]
    fn get_sysfs_string(sysfs_name: &str, name: &str) -> Option<String> {
        #[cfg(target_os = "linux")]
        match std::fs::read_to_string(format!("/sys/bus/usb/devices/{}/{}", sysfs_name, name)) {
            Ok(s) => Some(s.trim().to_string()),
            Err(_) => None,
        }

        #[cfg(not(target_os = "linux"))]
        None
    }

    #[allow(unused_variables)]
    fn get_udev_driver_name(port_path: &str) -> Result<Option<String>, Error> {
        #[cfg(all(target_os = "linux", feature = "udev"))]
        return udev::get_udev_driver_name(port_path);
        #[cfg(not(all(target_os = "linux", feature = "udev")))]
        return Ok(None);
    }

    #[allow(unused_variables)]
    fn get_udev_syspath(port_path: &str) -> Result<Option<String>, Error> {
        #[cfg(all(target_os = "linux", feature = "udev"))]
        return udev::get_udev_syspath(port_path);
        #[cfg(not(all(target_os = "linux", feature = "udev")))]
        return Ok(None);
    }

    fn get_product_string<T: libusb::UsbContext>(
        device_desc: &libusb::DeviceDescriptor,
        handle: &mut Option<UsbDevice<T>>,
    ) -> Option<String> {
        handle.as_mut().and_then(|h| {
            match h
                .handle
                .read_product_string(h.language, device_desc, h.timeout)
            {
                Ok(s) => Some(s.trim().trim_end_matches('\0').to_string()),
                Err(_) => None,
            }
        })
    }

    fn get_manufacturer_string<T: libusb::UsbContext>(
        device_desc: &libusb::DeviceDescriptor,
        handle: &mut Option<UsbDevice<T>>,
    ) -> Option<String> {
        handle.as_mut().and_then(|h| {
            match h
                .handle
                .read_manufacturer_string(h.language, device_desc, h.timeout)
            {
                Ok(s) => Some(s.trim().trim_end_matches('\0').to_string()),
                Err(_) => None,
            }
        })
    }

    fn get_serial_string<T: libusb::UsbContext>(
        device_desc: &libusb::DeviceDescriptor,
        handle: &mut Option<UsbDevice<T>>,
    ) -> Option<String> {
        handle.as_mut().and_then(|h| {
            match h
                .handle
                .read_serial_number_string(h.language, device_desc, h.timeout)
            {
                Ok(s) => Some(s.trim().trim_end_matches('\0').to_string()),
                Err(_) => None,
            }
        })
    }

    fn get_configuration_string<T: libusb::UsbContext>(
        config_desc: &libusb::ConfigDescriptor,
        handle: &mut Option<UsbDevice<T>>,
    ) -> Option<String> {
        handle.as_mut().and_then(|h| {
            match h
                .handle
                .read_configuration_string(h.language, config_desc, h.timeout)
            {
                Ok(s) => Some(s.trim().trim_end_matches('\0').to_string()),
                Err(_) => None,
            }
        })
    }

    fn get_interface_string<T: libusb::UsbContext>(
        interface_desc: &libusb::InterfaceDescriptor,
        handle: &mut Option<UsbDevice<T>>,
    ) -> Option<String> {
        handle.as_mut().and_then(|h| {
            match h
                .handle
                .read_interface_string(h.language, interface_desc, h.timeout)
            {
                Ok(s) => Some(s.trim().trim_end_matches('\0').to_string()),
                Err(_) => None,
            }
        })
    }

    fn get_descriptor_string<T: libusb::UsbContext>(
        string_index: u8,
        handle: &mut Option<UsbDevice<T>>,
    ) -> Option<String> {
        handle.as_mut().and_then(|h| {
            match h
                .handle
                .read_string_descriptor(h.language, string_index, h.timeout)
            {
                Ok(s) => Some(s.trim().trim_end_matches('\0').to_string()),
                Err(_) => None,
            }
        })
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

    fn build_descriptor_extra<T: libusb::UsbContext>(
        handle: &mut Option<UsbDevice<T>>,
        extra_bytes: &[u8],
    ) -> Result<usb::DescriptorType, Error> {
        // Get any extra descriptors into a known type and add any handle data while we have it
        let mut dt = usb::DescriptorType::try_from(extra_bytes)?;

        match dt {
            usb::DescriptorType::InterfaceAssociation(ref mut iad) => {
                iad.function_string = get_descriptor_string(iad.function_string_index, handle);
            }
            _ => (),
        }

        Ok(dt)
    }

    fn build_config_descriptor_extra<T: libusb::UsbContext>(
        handle: &mut Option<UsbDevice<T>>,
        config_desc: &libusb::ConfigDescriptor,
    ) -> Result<Vec<usb::DescriptorType>, Error> {
        let mut extra_bytes = config_desc.extra().to_owned();
        let extra_len = extra_bytes.len();
        let mut taken = 0;
        let mut ret = Vec::new();

        // Iterate on chunks of the header length
        while taken < extra_len && extra_len >= 2 {
            let dt_len = extra_bytes[0] as usize;
            let dt =
                build_descriptor_extra(handle, &extra_bytes.drain(..dt_len).collect::<Vec<u8>>())?;
            log::debug!("Config descriptor extra: {:?}", dt);
            ret.push(dt);
            taken += dt_len;
        }

        Ok(ret)
    }

    fn build_interface_descriptor_extra<T: libusb::UsbContext>(
        handle: &mut Option<UsbDevice<T>>,
        interface_desc: &libusb::InterfaceDescriptor,
    ) -> Result<Vec<usb::DescriptorType>, Error> {
        let mut extra_bytes = interface_desc.extra().to_owned();
        let extra_len = extra_bytes.len();
        let mut taken = 0;
        let mut ret = Vec::new();

        // Iterate on chunks of the header length
        while taken < extra_len && extra_len >= 2 {
            let dt_len = extra_bytes[0] as usize;
            extra_bytes.get_mut(1).map(|b| {
                // Mask request type LIBUSB_REQUEST_TYPE_CLASS
                *b &= !(0x01 << 5);
                // if not Device or Interface, force it to Interface
                if *b != 0x01 || *b != 0x04 {
                    *b = 0x04;
                }
            });

            let mut dt =
                build_descriptor_extra(handle, &extra_bytes.drain(..dt_len).collect::<Vec<u8>>())?;

            // Assign class context to interface since descriptor did not know it
            dt.update_with_class_context((
                interface_desc.class_code(),
                interface_desc.sub_class_code(),
                interface_desc.protocol_code(),
            ))?;

            log::debug!("Interface descriptor extra: {:?}", dt);
            ret.push(dt);
            taken += dt_len;
        }

        Ok(ret)
    }

    fn build_endpoint_descriptor_extra<T: libusb::UsbContext>(
        handle: &mut Option<UsbDevice<T>>,
        endpoint_desc: &libusb::EndpointDescriptor,
    ) -> Result<Option<Vec<usb::DescriptorType>>, Error> {
        match endpoint_desc.extra() {
            Some(extra_bytes) => {
                let extra_len = extra_bytes.len();
                let mut taken = 0;
                let mut ret = Vec::new();

                // Iterate on chunks of the header length
                while taken < extra_len && extra_len >= 2 {
                    let dt_len = extra_bytes[taken] as usize;
                    // TODO check device with mask to see if we need to do this
                    // extra_bytes.get_mut(taken+1).map(|b| {
                    //     if *b == 0x21 || *b == 0x22 || *b == 0x23 {
                    //         *b &= !(0x01 << 5);
                    //     }
                    // });

                    match extra_bytes.get(taken..dt_len) {
                        Some(dt_bytes) => {
                            let dt = build_descriptor_extra(handle, dt_bytes)?;
                            log::debug!("Endpoint descriptor extra: {:?}", dt);
                            ret.push(dt);
                            taken += dt_len;
                        }
                        None => break,
                    }
                }

                Ok(Some(ret))
            }
            None => Ok(None),
        }
    }

    fn build_endpoints<T: libusb::UsbContext>(
        handle: &mut Option<UsbDevice<T>>,
        interface_desc: &libusb::InterfaceDescriptor,
    ) -> Vec<usb::USBEndpoint> {
        let mut ret: Vec<usb::USBEndpoint> = Vec::new();

        for endpoint_desc in interface_desc.endpoint_descriptors() {
            ret.push(usb::USBEndpoint {
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
                extra: build_endpoint_descriptor_extra(handle, &endpoint_desc)
                    .ok()
                    .flatten(),
            });
        }

        ret
    }

    fn build_interfaces<T: libusb::UsbContext>(
        device: &libusb::Device<T>,
        handle: &mut Option<UsbDevice<T>>,
        config_desc: &libusb::ConfigDescriptor,
        with_udev: bool,
    ) -> error::Result<Vec<usb::USBInterface>> {
        let mut ret: Vec<usb::USBInterface> = Vec::new();

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                let path = usb::get_interface_path(
                    device.bus_number(),
                    &device.port_numbers()?,
                    config_desc.number(),
                    interface_desc.interface_number(),
                );

                let mut interface = usb::USBInterface {
                    name: get_sysfs_string(&path, "interface")
                        .or(get_interface_string(&interface_desc, handle))
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
                    endpoints: build_endpoints(handle, &interface_desc),
                    extra: build_interface_descriptor_extra(handle, &interface_desc).ok(),
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
        device: &libusb::Device<T>,
        handle: &mut Option<UsbDevice<T>>,
        device_desc: &libusb::DeviceDescriptor,
        sp_device: &system_profiler::USBDevice,
        with_udev: bool,
    ) -> error::Result<Vec<usb::USBConfiguration>> {
        // Retrieve the current configuration (if available)
        let cur_config = get_sysfs_configuration_string(&sp_device.sysfs_name());
        let mut ret: Vec<usb::USBConfiguration> = Vec::new();

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

            ret.push(usb::USBConfiguration {
                name: get_configuration_string(&config_desc, handle)
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
                interfaces: build_interfaces(device, handle, &config_desc, with_udev)?,
                extra: build_config_descriptor_extra(handle, &config_desc).ok(),
            });
        }

        Ok(ret)
    }

    #[allow(unused_variables)]
    fn build_spdevice_extra<T: libusb::UsbContext>(
        device: &libusb::Device<T>,
        handle: &mut Option<UsbDevice<T>>,
        device_desc: &libusb::DeviceDescriptor,
        sp_device: &system_profiler::USBDevice,
        with_udev: bool,
    ) -> error::Result<usb::USBDeviceExtra> {
        let mut extra = usb::USBDeviceExtra {
            max_packet_size: device_desc.max_packet_size(),
            string_indexes: (
                device_desc.product_string_index().unwrap_or(0),
                device_desc.manufacturer_string_index().unwrap_or(0),
                device_desc.serial_number_string_index().unwrap_or(0),
            ),
            driver: None,
            syspath: None,
            // These are idProduct, idVendor in lsusb - from udev_hwdb/usb-ids
            vendor: super::names::vendor(device_desc.vendor_id())
                .or(usb_ids::Vendor::from_id(device_desc.vendor_id()).map(|v| v.name().to_owned())),
            product_name: super::names::product(device_desc.vendor_id(), device_desc.product_id())
                .or(usb_ids::Device::from_vid_pid(
                    device_desc.vendor_id(),
                    device_desc.product_id(),
                )
                .map(|v| v.name().to_owned())),
            configurations: build_configurations(
                device,
                handle,
                device_desc,
                sp_device,
                with_udev,
            )?,
        };

        // flag allows us to try again without udev if it raises an nting
        // but record the error for printing
        if with_udev {
            let sysfs_name = sp_device.sysfs_name();
            extra.driver = get_udev_driver_name(&sysfs_name)?;
            extra.syspath = get_udev_syspath(&sysfs_name)?;
        }

        Ok(extra)
    }

    /// Builds a [`system_profiler::USBDevice`] from a [`libusb::Device`] by using `device_descriptor()` and intrograting for configuration strings. Optionally with `with_extra` will gather full device information, including from udev if feature is present.
    ///
    /// [`system_profiler::USBDevice.profiler_error`] `Option<String>` will contain any non-critical error during gather of `with_extra` data - normally due to permissions preventing open of device descriptors.
    pub fn build_spdevice<T: libusb::UsbContext>(
        device: &libusb::Device<T>,
        with_extra: bool,
    ) -> error::Result<system_profiler::USBDevice> {
        let timeout = Duration::from_secs(1);
        let speed = match usb::Speed::from(device.speed()) {
            usb::Speed::Unknown => None,
            v => Some(system_profiler::DeviceSpeed::SpeedValue(v)),
        };

        let mut error_str = None;
        let device_desc = device.device_descriptor()?;

        // try to get open device for strings but allowed to continue if this fails - get string functions will return empty
        let mut usb_device = {
            match device.open() {
                Ok(h) => match h.read_languages(timeout) {
                    Ok(l) => {
                        if !l.is_empty() {
                            Some(UsbDevice {
                                handle: h,
                                language: l[0],
                                timeout,
                            })
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        error_str = Some(format!(
                            "Failed to open {:?}, will be unable to obtain all data: {}",
                            device, e
                        ));
                        None
                    }
                },
                Err(e) => {
                    error_str = Some(format!(
                        "Failed to open {:?}, will be unable to obtain all data: {}",
                        device, e
                    ));
                    None
                }
            }
        };

        let mut sp_device = system_profiler::USBDevice {
            vendor_id: Some(device_desc.vendor_id()),
            product_id: Some(device_desc.product_id()),
            device_speed: speed,
            location_id: system_profiler::DeviceLocation {
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

        // Attempt to lookup 'i' strings (iManufacturer, iProduct, iSerialNumber) from device with
        // the following precedence
        // 1. Read directly from the device descriptor (usually requires root access)
        // 2. (on Linux) Read from sysfs, which is a cached copy of the device descriptor
        //    TODO (does macOS and Windows have an equivalent/similar way to retrieve this info?)
        // 3. Lookup iManufacturer and iProduct from the USB IDs list (iSerial has no alternative)

        sp_device.manufacturer =
            get_manufacturer_string(&device_desc, &mut usb_device) // descriptor
                // sysfs cache
                .or(get_sysfs_string(&sp_device.sysfs_name(), "manufacturer"))
                // udev-hwdb
                .or(super::names::vendor(device_desc.vendor_id())) // udev, usb-ids if error
                // usb-ids
                .or(usb_ids::Vendor::from_id(device_desc.vendor_id())
                    .map(|vendor| vendor.name().to_owned()));

        sp_device.name = get_product_string(&device_desc, &mut usb_device) // descriptor
            // sysfs cache
            .or(get_sysfs_string(&sp_device.sysfs_name(), "product"))
            // udev-hwdb
            .or(super::names::product(
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

        sp_device.serial_num = get_serial_string(&device_desc, &mut usb_device)
            .or(get_sysfs_string(&sp_device.sysfs_name(), "serial"));

        let extra_error_str = if with_extra {
            match build_spdevice_extra(device, &mut usb_device, &device_desc, &sp_device, true) {
                Ok(extra) => {
                    sp_device.extra = Some(extra);
                    None
                }
                Err(e) => {
                    // try again without udev if we have that feature but return message so device still added
                    if cfg!(feature = "udev") && e.kind() == ErrorKind::Udev {
                        sp_device.extra = Some(build_spdevice_extra(
                            device,
                            &mut usb_device,
                            &device_desc,
                            &sp_device,
                            false,
                        )?);
                        Some(format!( "Failed to get udev data for {}, probably requires elevated permissions", sp_device ))
                    } else {
                        Some(format!( "Failed to get some extra data for {}, probably requires elevated permissions: {}", sp_device, e ))
                    }
                }
            }
        } else {
            None
        };

        if error_str.is_none() {
            error_str = extra_error_str;
        }

        sp_device.profiler_error = error_str;
        Ok(sp_device)
    }

    fn _get_spusb(
        with_extra: bool,
        print_stderr: bool,
    ) -> Result<system_profiler::SPUSBDataType, Error> {
        let mut spusb = system_profiler::SPUSBDataType { buses: Vec::new() };
        // temporary store of devices created when iterating through DeviceList
        let mut cache: Vec<system_profiler::USBDevice> = Vec::new();
        // lookup for root hubs to assign info to bus on linux
        let mut root_hubs: HashMap<u8, system_profiler::USBDevice> = HashMap::new();

        log::info!("Building SPUSBDataType with libusb {:?}", libusb::version());

        // run through devices building USBDevice types
        for device in libusb::DeviceList::new()?.iter() {
            match build_spdevice(&device, with_extra) {
                Ok(sp_device) => {
                    cache.push(sp_device.to_owned());

                    // print any non-critical error during extra capture
                    sp_device.profiler_error.iter().for_each(|e| {
                        if print_stderr {
                            eprintln!("{}", e);
                        } else {
                            log::warn!("Non-critical error during profile: {}", e);
                        }
                    });

                    // save it if it's a root_hub for assigning to bus data
                    if !cfg!(target_os = "macos") && sp_device.is_root_hub() {
                        root_hubs.insert(sp_device.location_id.bus, sp_device);
                    }
                }
                Err(e) => eprintln!("Failed to get data for {:?}: {}", device, e),
            }
        }

        // ensure sort of bus so that grouping is not broken up
        cache.sort_by_key(|d| d.location_id.bus);
        log::trace!("Sorted devices {:#?}", cache);

        // group by bus number and then stick them into a bus in the returned SPUSBDataType
        for (key, group) in &cache.into_iter().group_by(|d| d.location_id.bus) {
            let root = if !cfg!(target_os = "macos") {
                root_hubs.get(&key)
            } else {
                None
            };
            log::debug!("Root device {:?}", root);

            // create the bus, we'll add devices at next step
            let mut new_bus = system_profiler::USBBus {
                name: "Unknown".into(),
                host_controller: "Unknown".into(),
                usb_bus_number: Some(key),
                ..Default::default()
            };

            if let Some(root_hub) = root {
                root_hub.name.clone_into(&mut new_bus.name);
                root_hub
                    .manufacturer
                    .as_ref()
                    .unwrap_or(&String::new())
                    .clone_into(&mut new_bus.host_controller);
                new_bus.pci_vendor = root_hub.vendor_id;
                new_bus.pci_device = root_hub.product_id;
            }

            // group into parent groups with parent path as key or trunk devices so they end up in same place
            let parent_groups = group.group_by(|d| d.parent_path().unwrap_or(d.trunk_path()));

            // now go through parent paths inserting devices owned by that parent
            // this is not perfect...if the sort of devices does not result in order of depth, it will panic because the parent of a device will not exist. But that won't happen, right...
            // sort key - ends_with to ensure root_hubs, which will have same str length as trunk devices will still be ahead
            for (parent_path, children) in parent_groups
                .into_iter()
                .sorted_by_key(|x| x.0.len() - x.0.ends_with("-0") as usize)
            {
                log::debug!("Adding devices to parent {}", parent_path);
                // if root devices, add them to bus
                if parent_path.ends_with("-0") {
                    // if parent_path == "-" {
                    let devices = std::mem::take(&mut new_bus.devices);
                    if let Some(mut d) = devices {
                        for new_device in children {
                            d.push(new_device);
                        }
                        new_bus.devices = Some(d);
                    } else {
                        new_bus.devices = Some(children.collect());
                    }
                    log::debug!("Updated bus {}", new_bus);
                    log::trace!("Updated bus devices {:?}", new_bus.devices);
                // else find and add parent - this should work because we are sorted to accend the tree so parents should be created before their children
                } else {
                    let parent_node = new_bus
                        .get_node_mut(&parent_path)
                        .expect("Parent node does not exist in new bus!");
                    let devices = std::mem::take(&mut parent_node.devices);
                    if let Some(mut d) = devices {
                        for new_device in children {
                            d.push(new_device);
                        }
                        parent_node.devices = Some(d);
                    } else {
                        parent_node.devices = Some(children.collect());
                    }
                    log::debug!("Updated parent {}", parent_node);
                    log::trace!("Updated parent devices {:?}", parent_node.devices);
                }
            }

            spusb.buses.push(new_bus);
        }

        Ok(spusb)
    }

    /// Get [`system_profiler::SPUSBDataType`] using `libusb`. Does not source [`usb::USBDeviceExtra`] - use [`get_spusb_with_extra`] for that; the extra operation is mostly moving data around so the only hit is to stack.
    ///
    /// Runs through `libusb::DeviceList` creating a cache of [`system_profiler::USBDevice`]. Then sorts into parent groups, accending in depth to build the [`system_profiler::USBBus`] tree.
    ///
    /// Building the [`system_profiler::SPUSBDataType`] depends on system; on Linux, the root devices are at buses where as macOS the buses are not listed
    pub fn get_spusb(print_stderr: bool) -> Result<system_profiler::SPUSBDataType, Error> {
        _get_spusb(false, print_stderr)
    }

    /// Get [`system_profiler::SPUSBDataType`] using `libusb` including [`usb::USBDeviceExtra`] - the main function to use for most use cases unless one does not want verbose data.
    ///
    /// Like `get_spusb`, runs through `libusb::DeviceList` creating a cache of [`system_profiler::USBDevice`]. On Linux and with the 'udev' feature enabled, the syspath and driver will attempt to be obtained.
    pub fn get_spusb_with_extra(
        print_stderr: bool,
    ) -> Result<system_profiler::SPUSBDataType, Error> {
        _get_spusb(true, print_stderr)
    }

    /// Fills a passed mutable `spusb` reference to fill using `get_spusb`. Will replace existing [`system_profiler::USBDevice`]s found in the libusb build but leave others and the buses.
    ///
    /// The main use case for this is to merge with macOS `system_profiler` data, so that [`usb::USBDeviceExtra`] can be obtained but internal buses kept. One could also use it to update a static .json dump.
    pub fn fill_spusb(spusb: &mut system_profiler::SPUSBDataType) -> Result<(), Error> {
        let libusb_spusb = get_spusb_with_extra(false)?;

        // merge if passed has any buses
        if !spusb.buses.is_empty() {
            for mut bus in libusb_spusb.buses {
                if let Some(existing) = spusb
                    .buses
                    .iter_mut()
                    .find(|b| b.get_bus_number() == bus.get_bus_number())
                {
                    // just take the devices and put them in since libusb will be more verbose
                    existing.devices = std::mem::take(&mut bus.devices);
                }
            }
        }

        Ok(())
    }
}

pub mod names {
    //! Port of names.c in usbutils that provides name lookups for USB data using udev, falling back to USB IDs repository.
    //!
    //! lsusb uses udev and the bundled hwdb (based on USB IDs) for name lookups. To attempt parity with lsusb, this module uses udev_hwdb if the feature is enabled, otherwise it will fall back to the USB IDs repository. Whilst they both get data from the same source, the bundled udev hwdb might be different due to release version/customisations.
    //!
    //! The function names match those found in the lsusb source code.
    //!
    //! TODO: use extra USB IDs for full descriptor dumping
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
}

pub mod display {
    //! Printing functions for lsusb style output of USB data
    use crate::display::PrintSettings;
    use crate::error::{Error, ErrorKind};
    use crate::{system_profiler, usb};

    const TREE_LSUSB_BUS: &str = "/:  ";
    const TREE_LSUSB_DEVICE: &str = "|__ ";
    const TREE_LSUSB_SPACE: &str = "    ";

    /// Print [`system_profiler::SPUSBDataType`] as a lsusb style tree with the two optional `verbosity` levels
    pub fn print_tree(spusb: &system_profiler::SPUSBDataType, settings: &PrintSettings) {
        fn print_tree_devices(devices: &Vec<system_profiler::USBDevice>, settings: &PrintSettings) {
            let sorted = settings.sort_devices.sort_devices(devices);

            for device in sorted {
                if device.is_root_hub() {
                    log::debug!("lsusb tree skipping root_hub {}", device);
                    continue;
                }
                // the const len should get compiled to const...
                let spaces =
                    (device.get_depth() * TREE_LSUSB_DEVICE.len()) + TREE_LSUSB_SPACE.len();
                let device_tree_strings: Vec<(String, String, String)> =
                    device.to_lsusb_tree_string();

                for strings in device_tree_strings {
                    println!("{:>spaces$}{}", TREE_LSUSB_DEVICE, strings.0);
                    if settings.verbosity >= 1 {
                        println!("{:>spaces$}{}", TREE_LSUSB_SPACE, strings.1);
                    }
                    if settings.verbosity >= 2 {
                        println!("{:>spaces$}{}", TREE_LSUSB_SPACE, strings.2);
                    }
                }
                // print all devices with this device - if hub for example
                device
                    .devices
                    .as_ref()
                    .map_or((), |d| print_tree_devices(d, settings))
            }
        }

        for bus in &spusb.buses {
            let bus_tree_strings: Vec<(String, String, String)> = bus.to_lsusb_tree_string();
            for strings in bus_tree_strings {
                println!("{}{}", TREE_LSUSB_BUS, strings.0);
                if settings.verbosity >= 1 {
                    println!("{}{}", TREE_LSUSB_SPACE, strings.1);
                }
                if settings.verbosity >= 2 {
                    println!("{}{}", TREE_LSUSB_SPACE, strings.2);
                }
            }

            // followed by devices if there are some
            bus.devices
                .as_ref()
                .map_or((), |d| print_tree_devices(d, settings))
        }
    }

    /// Dump a single [`system_profiler::USBDevice`] matching `dev_path` verbosely
    pub fn dump_one_device(
        devices: &Vec<&system_profiler::USBDevice>,
        dev_path: &String,
    ) -> Result<(), Error> {
        for device in devices {
            if &device.dev_path() == dev_path {
                // error if extra is none because we need it for vebose
                if device.extra.is_none() {
                    return Err(Error::new(
                        ErrorKind::Opening,
                        &format!("Unable to open {}", dev_path),
                    ));
                } else {
                    print(&vec![device], true);
                    return Ok(());
                }
            }
        }

        Err(Error::new(
            ErrorKind::NotFound,
            &format!("Unable to find {}", dev_path),
        ))
    }

    /// Print USB devices in lsusb style flat dump
    ///
    /// `verbose` flag enables verbose printing like lsusb (configs, interfaces and endpoints) - a huge dump!
    pub fn print(devices: &Vec<&system_profiler::USBDevice>, verbose: bool) {
        if !verbose {
            for device in devices {
                println!("{}", device.to_lsusb_string());
            }
        } else {
            for device in devices {
                match device.extra.as_ref() {
                    None => log::warn!("Skipping {} because it does not contain extra data required for verbose print", device),
                    Some(device_extra) => {
                        println!(); // new lines separate in verbose lsusb
                        println!("{}", device.to_lsusb_string());
                        // print error regarding open if non-critcal during probe like lsusb --verbose
                        if device.profiler_error.is_some() {
                            eprintln!("Couldn't open device, some information will be missing");
                        }
                        print_device(device);

                        for config in &device_extra.configurations {
                            print_config(config);

                            for interface in &config.interfaces {
                                print_interface(interface);

                                for endpoint in &interface.endpoints {
                                    print_endpoint(endpoint);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn print_device(device: &system_profiler::USBDevice) {
        let device_extra = device
            .extra
            .as_ref()
            .expect("Cannot print verbose without extra data");

        let (class_name, sub_class_name, protocol_name) =
            match (device.base_class_code(), device.sub_class, device.protocol) {
                (Some(bc), Some(scid), Some(pid)) => (
                    super::names::class(bc),
                    super::names::subclass(bc, scid),
                    super::names::protocol(bc, scid, pid),
                ),
                (Some(bc), Some(scid), None) => (
                    super::names::class(bc),
                    super::names::subclass(bc, scid),
                    None,
                ),
                (Some(bc), None, None) => (super::names::class(bc), None, None),
                (None, None, None) => (None, None, None),
                _ => unreachable!(),
            };

        println!("Device Descriptor:");
        // These are constants - length is 18 bytes for descriptor, type is 1
        println!("  bLength               18");
        println!("  bDescriptorType        1");
        println!(
            "  bcdUSB              {}",
            device
                .bcd_usb
                .as_ref()
                .map_or(String::new(), |v| v.to_string())
        );
        println!(
            "  bDeviceClass         {:3} {}",
            device.base_class_code().unwrap_or(0),
            class_name.unwrap_or_default()
        );
        println!(
            "  bDeviceSubClass      {:3} {}",
            device.sub_class.unwrap_or(0),
            sub_class_name.unwrap_or_default()
        );
        println!(
            "  bDeviceProtocol      {:3} {}",
            device.protocol.unwrap_or(0),
            protocol_name.unwrap_or_default()
        );
        println!("  bMaxPacketSize0      {:3}", device_extra.max_packet_size);
        println!(
            "  idVendor          {:#06x} {}",
            device.vendor_id.unwrap_or(0),
            device_extra.vendor.as_ref().unwrap_or(&String::new())
        );
        println!(
            "  idProduct         {:#06x} {}",
            device.product_id.unwrap_or(0),
            device_extra.product_name.as_ref().unwrap_or(&String::new())
        );
        println!(
            "  bcdDevice           {}",
            device
                .bcd_device
                .as_ref()
                .map_or(String::new(), |v| v.to_string())
        );
        println!(
            "  iManufacturer        {:3} {}",
            device_extra.string_indexes.0,
            device.manufacturer.as_ref().unwrap_or(&String::new())
        );
        println!(
            "  iProduct             {:3} {}",
            device_extra.string_indexes.1, device.name
        );
        println!(
            "  iSerialNumber        {:3} {}",
            device_extra.string_indexes.2,
            device.serial_num.as_ref().unwrap_or(&String::new())
        );
        println!(
            "  bNumConfigurations   {:3}",
            device_extra.configurations.len()
        );
    }

    fn print_config(config: &usb::USBConfiguration) {
        println!("  Configuration Descriptor:");
        println!("    bLength              {:3}", config.length);
        println!("    bDescriptorType        2"); // type 2 for configuration
        println!("    wTotalLength      {:#06x}", config.total_length);
        println!("    bNumInterfaces       {:3}", config.interfaces.len());
        println!("    bConfigurationValue  {:3}", config.number);
        println!(
            "    iConfiguration       {:3} {}",
            config.string_index, config.name
        );
        println!(
            "    bmAttributes:       0x{:02x}",
            config.attributes_value()
        );
        // no attributes is bus powered
        if config.attributes.is_empty() {
            println!("      (Bus Powered)");
        } else {
            if config
                .attributes
                .contains(&usb::ConfigAttributes::SelfPowered)
            {
                println!("      Self Powered");
            }
            if config
                .attributes
                .contains(&usb::ConfigAttributes::RemoteWakeup)
            {
                println!("      Remote Wakeup");
            }
        }
        println!(
            "    MaxPower           {:>5}{}",
            config.max_power.value, config.max_power.unit
        );

        // dump extra descriptors
        if let Some(dt_vec) = &config.extra {
            for dt in dt_vec {
                match dt {
                    usb::DescriptorType::InterfaceAssociation(iad) => {
                        dump_interface_association(iad);
                    }
                    usb::DescriptorType::Security(sec) => {
                        dump_security(sec);
                    }
                    usb::DescriptorType::Encrypted(enc) => {
                        dump_encryption_type(enc);
                    }
                    usb::DescriptorType::Unknown(junk) => {
                        dump_unrecognised(junk, 4);
                    }
                    usb::DescriptorType::Junk(junk) => {
                        dump_junk(junk, 4);
                    }
                    _ => (),
                }
            }
        }
    }

    fn print_interface(interface: &usb::USBInterface) {
        let interface_name = super::names::class(interface.class.into());
        let sub_class_name = super::names::subclass(interface.class.into(), interface.sub_class);
        let protocol_name = super::names::protocol(
            interface.class.into(),
            interface.sub_class,
            interface.protocol,
        );

        println!("    Interface Descriptor:");
        println!("      bLength              {:3}", interface.length);
        println!("      bDescriptorType        4"); // type 4 for interface
        println!("      bInterfaceNumber     {:3}", interface.number);
        println!("      bAlternateSetting    {:3}", interface.alt_setting);
        println!("      bNumEndpoints        {:3}", interface.endpoints.len());
        println!(
            "      bInterfaceClass      {:3} {}",
            u8::from(interface.class.to_owned()),
            interface_name.unwrap_or_default()
        );
        println!(
            "      bInterfaceSubClass   {:3} {}",
            interface.sub_class,
            sub_class_name.unwrap_or_default()
        );
        println!(
            "      bInterfaceProtocol   {:3} {}",
            interface.protocol,
            protocol_name.unwrap_or_default()
        );
        println!(
            "      iInterface           {:3} {}",
            interface.string_index, interface.name
        );

        // dump extra descriptors
        if let Some(dt_vec) = &interface.extra {
            for dt in dt_vec {
                match dt {
                    usb::DescriptorType::Device(cd) | usb::DescriptorType::Interface(cd) => {
                        match cd {
                            usb::ClassDescriptor::Hid(hidd) => dump_hid_device(hidd),
                            _ => (),
                        }
                    }
                    usb::DescriptorType::Unknown(junk) => {
                        dump_unrecognised(junk, 6);
                    }
                    usb::DescriptorType::Junk(junk) => {
                        dump_junk(junk, 6);
                    }
                    _ => (),
                }
            }
        }
    }

    fn print_endpoint(endpoint: &usb::USBEndpoint) {
        println!("      Endpoint Descriptor:");
        println!("        bLength              {:3}", endpoint.length);
        println!("        bDescriptorType        5"); // type 5 for endpoint
        println!(
            "        bEndpointAddress    {:#04x} EP {} {}",
            endpoint.address.address,
            endpoint.address.number,
            endpoint.address.direction.to_string().to_uppercase()
        );
        println!("        bmAttributes:");
        println!(
            "          Transfer Type          {:?}",
            endpoint.transfer_type
        );
        println!("          Sync Type              {:?}", endpoint.sync_type);
        println!("          Usage Type             {:?}", endpoint.usage_type);
        println!(
            "        wMaxPacketSize    {:#06x} {} bytes",
            endpoint.max_packet_size,
            endpoint.max_packet_string()
        );
        println!("        bInterval            {:3}", endpoint.interval);

        // dump extra descriptors
        if let Some(dt_vec) = &endpoint.extra {
            for dt in dt_vec {
                match dt {
                    usb::DescriptorType::Endpoint(_cd) => {
                        
                    }
                    // Misplaced descriptors
                    usb::DescriptorType::Device(cd) => {
                        // TODO dump depending on tagged class
                        println!(
                            "        DEVICE CLASS: {}",
                            Vec::<u8>::from(cd.to_owned())
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<String>>()
                                .join(" ")
                        );
                    }
                    usb::DescriptorType::Interface(cd) => {
                        // TODO dump depending on tagged class
                        println!(
                            "        INTERFACE CLASS: {}",
                            Vec::<u8>::from(cd.to_owned())
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<String>>()
                                .join(" ")
                        );
                    }
                    usb::DescriptorType::InterfaceAssociation(iad) => {
                        dump_interface_association(iad);
                    }
                    usb::DescriptorType::SsEndpointCompanion(ss) => {
                        println!("        bMaxBurst {:>15}", ss.max_burst);
                        match endpoint.transfer_type {
                            usb::TransferType::Bulk => {
                                if ss.attributes & 0x1f != 0 {
                                    println!("        MaxStreams {:>14}", 1 << ss.attributes);
                                }
                            }
                            usb::TransferType::Isochronous => {
                                if ss.attributes & 0x03 != 0 {
                                    println!("        Mult {:>20}", ss.attributes & 0x3);
                                }
                            }
                            _ => (),
                        }
                    }
                    usb::DescriptorType::Unknown(junk) => {
                        dump_unrecognised(junk, 8);
                    }
                    usb::DescriptorType::Junk(junk) => {
                        dump_junk(junk, 8);
                    }
                    _ => (),
                }
            }
        }
    }

    fn dump_junk(extra: &[u8], indent: usize) {
        println!(
            "{:^indent$}junk at descriptor end: {}",
            "",
            extra
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" ")
        )
    }

    fn dump_unrecognised(extra: &[u8], indent: usize) {
        println!(
            "{:^indent$}** UNRECOGNIZED: {}",
            "",
            extra
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" ")
        )
    }

    fn dump_security(sec: &usb::SecurityDescriptor) {
        println!("    Security Descriptor:");
        println!("      bLength              {:3}", sec.length);
        println!("      bDescriptorType      {:3}", sec.descriptor_type);
        println!("      wTotalLength      {:#04x}", sec.total_length);
        println!("      bNumEncryptionTypes  {:3}", sec.encryption_types);
    }

    fn dump_encryption_type(enc: &usb::EncryptionDescriptor) {
        let enct_string = match enc.encryption_type as u8 {
            0 => "UNSECURE",
            1 => "WIRED",
            2 => "CCM_1",
            3 => "RSA_1",
            _ => "RESERVED",
        };

        println!("     Encryption Type:");
        println!("      bLength              {:3}", enc.length);
        println!("      bDescriptorType      {:3}", enc.descriptor_type);
        println!(
            "      bEncryptionType      {:3} {}",
            enc.encryption_type as u8, enct_string
        );
        println!("      bEncryptionValue     {:3}", enc.encryption_value);
        println!("      bAuthKeyIndex        {:3}", enc.auth_key_index);
    }

    fn dump_interface_association(iad: &usb::InterfaceAssociationDescriptor) {
        println!("    Interface Association:");
        println!("      bLength              {:3}", iad.length);
        println!("      bDescriptorType      {:3}", iad.descriptor_type);
        println!("      bFirstInterface      {:3}", iad.first_interface);
        println!("      bInterfaceCount      {:3}", iad.interface_count);
        println!(
            "      bFunctionClass       {:3} {}",
            iad.function_class,
            super::names::class(iad.function_class).unwrap_or_default()
        );
        println!(
            "      bFunctionSubClass    {:3} {}",
            iad.function_sub_class,
            super::names::subclass(iad.function_class, iad.function_sub_class).unwrap_or_default()
        );
        println!(
            "      bFunctionProtocol    {:3} {}",
            iad.function_protocol,
            super::names::protocol(
                iad.function_class,
                iad.function_sub_class,
                iad.function_protocol
            )
            .unwrap_or_default()
        );
        println!(
            "      iFunction            {:3} {}",
            iad.function_string_index,
            iad.function_string.as_ref().unwrap_or(&String::new())
        );
    }

    fn dump_hid_device(hidd: &usb::HidDescriptor) {
        println!("        HID Descriptor:");
        println!("          bLength              {:3}", hidd.length);
        println!("          bDescriptorType      {:3}", hidd.descriptor_type);
        println!(
            "          bcdHID               {}",
            hidd.bcd_hid
        );
        println!(
            "          bCountryCode         {:3} {}",
            hidd.country_code,
            super::names::countrycode(hidd.country_code).unwrap_or_default()
        );
        println!(
            "          bNumDescriptors      {:3}",
            hidd.descriptors.len()
        );
        for desc in &hidd.descriptors {
            println!(
                "          bDescriptorType:      {:3} {}",
                desc.descriptor_type,
                super::names::hid(desc.descriptor_type).unwrap_or_default()
            );
            println!("          wDescriptorLength:    {:3}", desc.length);
        }

        for desc in &hidd.descriptors {
            // only print report descriptor
            if desc.descriptor_type != 0x22 {
                continue;
            }

            match desc.data.as_ref() {
                Some(d) => {
                    dump_report_desc(d, 28);
                }
                None => {
                    println!("          Report Descriptors:");
                    println!("            ** UNAVAILABLE **");
                }
            }
        }
    }

    // ported directly from lsusb - it's not pretty but works...
    fn dump_report_desc(desc: &Vec<u8>, indent: usize) {
        let types = |t: u8| match t {
            0x00 => "Main",
            0x01 => "Global",
            0x02 => "Local",
            _ => "reserved",
        };

        println!("          Report Descriptor: (length is {})", desc.len());

        let mut i = 0;

        while i < desc.len() {
            let b = desc[i];
            let mut data = 0xffff;
            let mut hut = 0xff;
            let mut bsize = (b & 0x03) as usize;
            if bsize == 3 {
                bsize = 4;
            }
            let btype = b & (0x03 << 2);
            let btag = b & !0x03;
            print!(
                "            Item({:>6}): {}, data=",
                types(btype >> 2),
                super::names::report_tag(btag).unwrap_or_default()
            );
            if bsize > 0 {
                print!(" [ ");
                data = 0;
                for j in 0..bsize {
                    data |= (desc[i + 1 + j] as u16) << (j * 8);
                    print!("{:02x} ", desc[i + 1 + j]);
                }
                println!("] {}", data);
            } else {
                println!("none");
            }

            match btag {
                // usage page
                0x04 => {
                    hut = data as u8;
                    println!(
                        "{:^indent$}",
                        super::names::huts(hut).unwrap_or_default(),
                        indent = indent
                    );
                }
                // usage, usage minimum, usage maximum
                0x08 | 0x18 | 0x28 => {
                    println!(
                        "{:^indent$}",
                        super::names::hutus(hut, data).unwrap_or_default(),
                        indent = indent
                    );
                }
                // unit exponent
                0x54 => {
                    println!(
                        "{:^indent$}: {}",
                        "Unit Exponent",
                        data as u8,
                        indent = indent
                    );
                }
                // unit
                // 0x64 => {
                //     println!("{:^indent$}" dump_unit(data, bsize), indent = indent);
                // }
                // collection
                0xa0 => match data {
                    0x00 => println!("{:^indent$}", "Physical", indent = indent),
                    0x01 => println!("{:^indent$}", "Application", indent = indent),
                    0x02 => println!("{:^indent$}", "Logical", indent = indent),
                    0x03 => println!("{:^indent$}", "Report", indent = indent),
                    0x04 => println!("{:^indent$}", "Named Array", indent = indent),
                    0x05 => println!("{:^indent$}", "Usage Switch", indent = indent),
                    0x06 => println!("{:^indent$}", "Usage Modifier", indent = indent),
                    _ => {
                        if (data & 0x80) == 0x80 {
                            println!("{:^indent$}", "Vendor defined", indent = indent)
                        } else {
                            println!("{:^indent$}", "Unknown", indent = indent)
                        }
                    }
                },
                // input, output, feature
                0x80 | 0x90 | 0xb0 => {}
                _ => (),
            }
            i += bsize;
        }
    }
}
