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
    use itertools::Itertools;
    use rusb as libusb;
    use std::collections::HashMap;
    use std::time::Duration;
    use usb_ids::{self, FromId};
    use crate::error::{self, Error, ErrorKind};

    #[cfg(all(target_os = "linux", feature = "udev"))]
    use crate::udev;
    use crate::{system_profiler, types::NumericalUnit, usb};

    impl From<libusb::Error> for Error {
        fn from(error: libusb::Error) -> Self {
            Error {
                kind: ErrorKind::LibUSB,
                message: format!("Failed to gather system USB data from libusb: Error({})", &error.to_string()),
            }
        }
    }

    struct UsbDevice<T: libusb::UsbContext> {
        handle: libusb::DeviceHandle<T>,
        language: libusb::Language,
        timeout: Duration,
    }

    /// Set log level for rusb
    pub fn set_log_level(debug: u8) -> () {
        let log_level = match debug {
            0 => rusb::LogLevel::None,
            1 => rusb::LogLevel::Info,
            2 | _ => rusb::LogLevel::Debug,
        };

        rusb::set_log_level(log_level);
    }

    fn get_product_string<T: libusb::UsbContext>(
        device_desc: &libusb::DeviceDescriptor,
        handle: &mut Option<UsbDevice<T>>,
    ) -> Option<String> {
        handle.as_mut().map_or(None, |h| {
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
        handle.as_mut().map_or(None, |h| {
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
        handle.as_mut().map_or(None, |h| {
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
        handle.as_mut().map_or(None, |h| {
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
        handle.as_mut().map_or(None, |h| {
            match h
                .handle
                .read_interface_string(h.language, interface_desc, h.timeout)
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

    fn build_endpoints(
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
            });
        }

        ret
    }

    fn build_interfaces<T: libusb::UsbContext>(
        device: &libusb::Device<T>,
        handle: &mut Option<UsbDevice<T>>,
        config_desc: &libusb::ConfigDescriptor,
        _with_udev: bool,
    ) -> error::Result<Vec<usb::USBInterface>> {
        let mut ret: Vec<usb::USBInterface> = Vec::new();

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                let mut _interface = usb::USBInterface {
                    name: get_interface_string(&interface_desc, handle).unwrap_or(String::new()),
                    string_index: interface_desc.description_string_index().unwrap_or(0),
                    number: interface_desc.interface_number(),
                    path: usb::get_interface_path(
                        device.bus_number(),
                        &device.port_numbers()?,
                        config_desc.number(),
                        interface_desc.interface_number(),
                    ),
                    class: usb::ClassCode::from(interface_desc.class_code()),
                    sub_class: interface_desc.sub_class_code(),
                    protocol: interface_desc.protocol_code(),
                    alt_setting: interface_desc.setting_number(),
                    driver: None,
                    syspath: None,
                    endpoints: build_endpoints(&interface_desc),
                };

                #[cfg(all(target_os = "linux", feature = "udev"))]
                if _with_udev {
                    udev::get_udev_info(
                        &mut _interface.driver,
                        &mut _interface.syspath,
                        &_interface.path,
                    )?;
                }

                ret.push(_interface);
            }
        }

        Ok(ret)
    }

    fn build_configurations<T: libusb::UsbContext>(
        device: &libusb::Device<T>,
        handle: &mut Option<UsbDevice<T>>,
        device_desc: &libusb::DeviceDescriptor,
        with_udev: bool,
    ) -> error::Result<Vec<usb::USBConfiguration>> {
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

            ret.push(usb::USBConfiguration {
                name: get_configuration_string(&config_desc, handle).unwrap_or(String::new()),
                string_index: config_desc.description_string_index().unwrap_or(0),
                number: config_desc.number(),
                attributes,
                max_power: NumericalUnit {
                    value: config_desc.max_power() as u32,
                    unit: String::from("mA"),
                    description: None,
                },
                interfaces: build_interfaces(device, handle, &config_desc, with_udev)?,
            });
        }

        Ok(ret)
    }

    fn build_spdevice_extra<T: libusb::UsbContext>(
        device: &libusb::Device<T>,
        handle: &mut Option<UsbDevice<T>>,
        device_desc: &libusb::DeviceDescriptor,
        _sp_device: &system_profiler::USBDevice,
        _with_udev: bool,
    ) -> error::Result<usb::USBDeviceExtra> {
        let mut _extra = usb::USBDeviceExtra {
            max_packet_size: device_desc.max_packet_size(),
            string_indexes: (
                device_desc.product_string_index().unwrap_or(0),
                device_desc.manufacturer_string_index().unwrap_or(0),
                device_desc.serial_number_string_index().unwrap_or(0),
            ),
            driver: None,
            syspath: None,
            vendor: usb_ids::Vendor::from_id(device_desc.vendor_id())
                .map_or(None, |v| Some(v.name().to_owned())),
            product_name: usb_ids::Device::from_vid_pid(
                device_desc.vendor_id(),
                device_desc.product_id(),
            )
            .map_or(None, |v| Some(v.name().to_owned())),
            configurations: build_configurations(device, handle, device_desc, _with_udev)?,
        };

        #[cfg(all(target_os = "linux", feature = "udev"))]
        if _with_udev {
            udev::get_udev_info(
                &mut _extra.driver,
                &mut _extra.syspath,
                &_sp_device.port_path(),
            )?;
        }

        Ok(_extra)
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
                        if l.len() > 0 {
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

        // lookup manufacturer and device name from Linux list if empty
        let manufacturer = get_manufacturer_string(&device_desc, &mut usb_device).or(
            match usb_ids::Vendor::from_id(device_desc.vendor_id()) {
                Some(vendor) => Some(vendor.name().to_owned()),
                None => None,
            },
        );

        let name = match get_product_string(&device_desc, &mut usb_device) {
            Some(n) => n,
            None => {
                match usb_ids::Device::from_vid_pid(
                    device_desc.vendor_id(),
                    device_desc.product_id(),
                ) {
                    Some(product) => product.name().to_owned(),
                    None => String::new(),
                }
            }
        };

        let mut sp_device = system_profiler::USBDevice {
            name,
            manufacturer,
            serial_num: get_serial_string(&device_desc, &mut usb_device),
            vendor_id: Some(device_desc.vendor_id()),
            product_id: Some(device_desc.product_id()),
            device_speed: speed,
            location_id: system_profiler::DeviceLocation {
                bus: device.bus_number(),
                number: device.address(),
                tree_positions: device.port_numbers()?,
                ..Default::default()
            },
            bcd_device: Some(device_desc.device_version().into()),
            bcd_usb: Some(device_desc.usb_version().into()),
            class: Some(usb::ClassCode::from(device_desc.class_code())),
            sub_class: Some(device_desc.sub_class_code()),
            protocol: Some(device_desc.protocol_code()),
            ..Default::default()
        };

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
                    sp_device.profiler_error.as_ref().map_or((), |e| {
                        if print_stderr {
                            eprintln!("{}", e);
                        } else {
                            log::warn!("Non-critical error during profile: {}", e);
                        }
                    });

                    // save it if it's a root_hub for assigning to bus data
                    if !cfg!(target_os = "macos") {
                        if sp_device.is_root_hub() {
                            root_hubs.insert(sp_device.location_id.bus, sp_device);
                        }
                    }
                }
                Err(e) => eprintln!("Failed to get data for {:?}: {}", device, e.to_string()),
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
                new_bus.name = root_hub.name.to_owned();
                new_bus.host_controller = root_hub
                    .manufacturer
                    .as_ref()
                    .unwrap_or(&String::new())
                    .to_owned();
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
                if let Some(mut existing) = spusb
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

pub mod display {
    //! Printing functions for lsusb style output of USB data
    use crate::display::PrintSettings;
    use crate::{system_profiler, usb};
    use crate::error::{Error, ErrorKind};

    const TREE_LSUSB_BUS: &'static str = "/:  ";
    const TREE_LSUSB_DEVICE: &'static str = "|__ ";
    const TREE_LSUSB_SPACE: &'static str = "    ";

    /// Print [`system_profiler::SPUSBDataType`] as a lsusb style tree with the two optional `verbosity` levels
    pub fn print_tree(spusb: &system_profiler::SPUSBDataType, settings: &PrintSettings) -> () {
        fn print_tree_devices(devices: &Vec<system_profiler::USBDevice>, settings: &PrintSettings) {
            let sorted = settings.sort_devices.sort_devices(&devices);

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
                    return Err(Error::new(ErrorKind::Opening, &format!("Unable to open {}", dev_path)));
                } else {
                    print(&vec![device], true);
                    return Ok(());
                }
            }
        }

        Err(Error::new(ErrorKind::NotFound, &format!("Unable to find {}", dev_path)))
    }

    /// Print USB devices in lsusb style flat dump
    ///
    /// `verbose` flag enables verbose printing like lsusb (configs, interfaces and endpoints) - a huge dump!
    pub fn print(devices: &Vec<&system_profiler::USBDevice>, verbose: bool) -> () {
        if !verbose {
            for device in devices {
                println!("{}", device.to_lsusb_string());
            }
        } else {
            for device in devices {
                match device.extra.as_ref() {
                    None => log::warn!("Skipping {} because it does not contain extra data required for verbose print", device),
                    Some(device_extra) => {
                        println!(""); // new lines separate in verbose lsusb
                        println!("{}", device.to_lsusb_string());
                        // print error regarding open if non-critcal during probe like lsusb --verbose
                        if device.profiler_error.is_some() {
                            eprintln!("Couldn't open device, some information will be missing");
                        }
                        print_device(&device);

                        for config in &device_extra.configurations {
                            print_config(&config);

                            for interface in &config.interfaces {
                                print_interface(&interface);

                                for endpoint in &interface.endpoints {
                                    print_endpoint(&endpoint);
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

        println!("Device Descriptor:");
        println!(
            "  bcdUSB              {}",
            device
                .bcd_usb
                .as_ref()
                .map_or(String::new(), |v| v.to_string())
        );
        println!(
            "  bDeviceClass         {:3} {}",
            device.class.as_ref().map_or(0, |c| c.to_owned() as u8),
            device
                .class
                .as_ref()
                .map_or(String::new(), |c| c.to_string())
        );
        println!("  bDeviceSubClass      {:3}", device.sub_class.unwrap_or(0),);
        println!("  bDeviceProtocol      {:3}", device.protocol.unwrap_or(0));
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
        println!("  Config Descriptor:");
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
        println!(
            "    bMaxPower           {:4}{}",
            config.max_power.value, config.max_power.unit
        )
    }

    fn print_interface(interface: &usb::USBInterface) {
        println!("    Interface Descriptor:");
        println!("      bInterfaceNumber     {:3}", interface.number);
        println!("      bAlternateSetting    {:3}", interface.alt_setting);
        println!("      bNumEndpoints        {:3}", interface.endpoints.len());
        println!(
            "      bInterfaceClass      {:3} {}",
            interface.class.to_owned() as u8,
            interface.class.to_string()
        );
        println!("      bInterfaceSubClass   {:3}", interface.sub_class);
        println!("      bInterfaceProtocol   {:3}", interface.protocol);
        println!(
            "      iInterface           {:3} {}",
            interface.string_index, interface.name
        );
    }

    fn print_endpoint(endpoint: &usb::USBEndpoint) {
        println!("      Endpoint Descriptor:");
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
        println!("          Sync Type             {:?}", endpoint.sync_type);
        println!("          Usage Type             {:?}", endpoint.usage_type);
        println!(
            "        wMaxPacketSize    {:#06x} {} bytes",
            endpoint.max_packet_size,
            endpoint.max_packet_string()
        );
        println!("        bInterval            {:3}", endpoint.interval);
    }
}
