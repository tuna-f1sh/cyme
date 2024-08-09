//! System USB profilers
//!
//! Get [`system_profiler::SPUSBDataType`] struct of system USB buses and devices with extra data like configs, interfaces and endpoints
//!
//! ```no_run
//! use cyme::usb::profiler;
//!
//! let spusb = profiler::get_spusb_with_extra().unwrap();
//! // print with alternative styling (#) is using utf-8 icons
//! println!("{:#}", spusb);
//! ```
//!
//! See [`system_profiler`] docs for what can be done with returned data, such as [`system_profiler::USBFilter`]
use crate::error::Result;
use itertools::Itertools;
use std::collections::HashMap;

use crate::system_profiler;
#[cfg(all(target_os = "linux", any(feature = "udev", feature = "udevlib")))]
use crate::udev;

pub(crate) trait Profiler
where
    Self: std::fmt::Debug,
{
    fn profile_devices(
        &self,
        cache: &mut Vec<system_profiler::USBDevice>,
        root_hubs: &mut HashMap<u8, system_profiler::USBDevice>,
        with_extra: bool,
    ) -> Result<()>;

    fn get_spusb(&self, with_extra: bool) -> Result<system_profiler::SPUSBDataType> {
        let mut spusb = system_profiler::SPUSBDataType { buses: Vec::new() };
        // temporary store of devices created when iterating through DeviceList
        let mut cache: Vec<system_profiler::USBDevice> = Vec::new();
        // lookup for root hubs to assign info to bus on linux
        let mut root_hubs: HashMap<u8, system_profiler::USBDevice> = HashMap::new();

        log::info!("Building SPUSBDataType with {:?}", self);

        self.profile_devices(&mut cache, &mut root_hubs, with_extra)?;

        cache.sort_by_key(|d| d.location_id.bus);
        log::trace!("Sorted devices {:#?}", cache);

        // group by bus number and then stick them into a bus in the returned SPUSBDataType
        for (key, group) in &cache.into_iter().group_by(|d| d.location_id.bus) {
            let root = if !cfg!(target_os = "macos") {
                root_hubs.get(&key)
            } else {
                None
            };

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
                    log::trace!("Updated parent devices {:?}", parent_node.devices);
                }
            }

            spusb.buses.push(new_bus);
        }

        Ok(spusb)
    }

    /// Fills a passed mutable `spusb` reference to fill using `get_spusb`. Will replace existing [`system_profiler::USBDevice`]s found in the Profiler tree but leave others and the buses.
    ///
    /// The main use case for this is to merge with macOS `system_profiler` data, so that [`usb::USBDeviceExtra`] can be obtained but internal buses kept. One could also use it to update a static .json dump.
    fn fill_spusb(&self, spusb: &mut system_profiler::SPUSBDataType) -> Result<()> {
        let libusb_spusb = self.get_spusb(true)?;

        // merge if passed has any buses
        if !spusb.buses.is_empty() {
            for mut bus in libusb_spusb.buses {
                if let Some(existing) = spusb
                    .buses
                    .iter_mut()
                    .find(|b| b.get_bus_number() == bus.get_bus_number())
                {
                    // just take the devices and put them in since nusb/libusb will be more verbose
                    existing.devices = std::mem::take(&mut bus.devices);
                }
            }
        }

        Ok(())
    }

    /// Attempt to retrieve the current bConfigurationValue and iConfiguration for a device
    /// This will only return the current configuration, not all possible configurations
    /// If there are any failures in retrieving the data, None is returned
    #[allow(unused_variables)]
    fn get_sysfs_configuration_string(sysfs_name: &str) -> Option<(u8, String)> {
        #[cfg(target_os = "linux")]
        // Determine bConfigurationValue value on linux
        match Self::get_sysfs_string(sysfs_name, "bConfigurationValue") {
            Some(s) => match s.parse::<u8>() {
                Ok(v) => {
                    // Determine iConfiguration
                    Self::get_sysfs_string(sysfs_name, "configuration").map(|s| (v, s))
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
    fn get_udev_driver_name(port_path: &str) -> Result<Option<String>> {
        #[cfg(all(target_os = "linux", any(feature = "udev", feature = "udevlib")))]
        return udev::get_udev_driver_name(port_path);
        #[cfg(not(all(target_os = "linux", any(feature = "udev", feature = "udevlib"))))]
        return Ok(None);
    }

    #[allow(unused_variables)]
    fn get_udev_syspath(port_path: &str) -> Result<Option<String>> {
        #[cfg(all(target_os = "linux", any(feature = "udev", feature = "udevlib")))]
        return udev::get_udev_syspath(port_path);
        #[cfg(not(all(target_os = "linux", any(feature = "udev", feature = "udevlib"))))]
        return Ok(None);
    }

    #[allow(unused_variables)]
    fn get_syspath(port_path: &str) -> Option<String> {
        #[cfg(target_os = "linux")]
        return Some(format!("/sys/bus/usb/devices/{}", port_path));
        #[cfg(not(target_os = "linux"))]
        return None;
    }
}

#[cfg(feature = "libusb")]
pub mod libusb {
    //! Uses rusb (upto date libusb fork) to get system USB information - same lib as lsusb. Requires 'libusb' feature. Uses [`crate::system_profiler`] types to hold data so that it is cross-compatible with macOS system_profiler command.
    use super::*;
    use crate::error::{Error, ErrorKind};
    use crate::lsusb::names;
    use crate::usb::{self, NumericalUnit};
    use rusb as libusb;
    use usb_ids::{self, FromId};

    #[derive(Debug)]
    pub(crate) struct LibUsbProfiler;

    impl LibUsbProfiler {
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
            // 0 is reserved for language codes and we can assume we're not doing that for descriptor strings
            if string_index == 0 {
                return None;
            }

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

        fn get_control_msg<T: libusb::UsbContext>(
            handle: &mut Option<UsbDevice<T>>,
            request_type: u8,
            request: u8,
            value: u16,
            index: u16,
            length: usize,
        ) -> Result<Vec<u8>> {
            match handle.as_mut() {
                Some(h) => {
                    let mut buf = vec![0; length];
                    h.handle
                        .read_control(request_type, request, value, index, &mut buf, h.timeout)
                        .and_then(|n| {
                            if n < length {
                                log::warn!(
                                    "Failed to read full control message for {}: {} < {}",
                                    request,
                                    n,
                                    length
                                );
                                Err(libusb::Error::Io)
                            } else {
                                Ok(buf)
                            }
                        })
                        .map_err(|e| Error {
                            kind: ErrorKind::LibUSB,
                            message: format!("Failed to get control message: {}", e),
                        })
                }
                None => Err(Error {
                    kind: ErrorKind::LibUSB,
                    message: "Failed to get control message, no handle".to_string(),
                }),
            }
        }

        fn get_report_descriptor<T: libusb::UsbContext>(
            handle: &mut Option<UsbDevice<T>>,
            index: u16,
            length: u16,
        ) -> Result<Vec<u8>> {
            let request_type = libusb::request_type(
                libusb::Direction::In,
                libusb::RequestType::Standard,
                libusb::Recipient::Interface,
            );
            let request = libusb::constants::LIBUSB_REQUEST_GET_DESCRIPTOR;
            let value = (libusb::constants::LIBUSB_DT_REPORT as u16) << 8;
            Self::get_control_msg(handle, request_type, request, value, index, length as usize)
        }

        fn get_hub_descriptor<T: libusb::UsbContext>(
            handle: &mut Option<UsbDevice<T>>,
            protocol: u8,
            bcd: u16,
            has_ssp: bool,
        ) -> Result<usb::HubDescriptor> {
            let request_type = libusb::request_type(
                libusb::Direction::In,
                libusb::RequestType::Class,
                libusb::Recipient::Device,
            );
            let is_ext_status = protocol == 3 && bcd >= 0x0310 && has_ssp;
            let request = libusb::constants::LIBUSB_REQUEST_GET_DESCRIPTOR;
            let value = if bcd >= 0x0300 {
                (libusb::constants::LIBUSB_DT_SUPERSPEED_HUB as u16) << 8
            } else {
                (libusb::constants::LIBUSB_DT_HUB as u16) << 8
            };
            let data = Self::get_control_msg(handle, request_type, request, value, 0, 9)?;
            let mut hub = usb::HubDescriptor::try_from(data.as_slice())?;

            // get port statuses
            let port_request_type = libusb::request_type(
                libusb::Direction::In,
                libusb::RequestType::Class,
                libusb::Recipient::Other,
            );
            let mut port_statues: Vec<[u8; 8]> = Vec::with_capacity(hub.num_ports as usize);
            for p in 0..hub.num_ports {
                match Self::get_control_msg(
                    handle,
                    port_request_type,
                    libusb::constants::LIBUSB_REQUEST_GET_STATUS,
                    if is_ext_status { 2 } else { 0 },
                    p as u16 + 1,
                    if is_ext_status { 8 } else { 4 },
                ) {
                    Ok(mut data) => {
                        if data.len() < 8 {
                            let remaining = 8 - data.len();
                            data.extend(vec![0; remaining]);
                        }
                        port_statues.push(data.try_into().unwrap());
                    }
                    Err(e) => {
                        log::warn!("Failed to get port {} status: {}", p + 1, e);
                        return Ok(hub);
                    }
                }
            }

            hub.port_statuses = Some(port_statues);

            Ok(hub)
        }

        fn get_device_status<T: libusb::UsbContext>(
            handle: &mut Option<UsbDevice<T>>,
        ) -> Result<u16> {
            let request_type = libusb::request_type(
                libusb::Direction::In,
                libusb::RequestType::Standard,
                libusb::Recipient::Device,
            );
            let request = libusb::constants::LIBUSB_REQUEST_GET_STATUS;
            let value = 0;
            let data = Self::get_control_msg(handle, request_type, request, value, 0, 2)?;
            Ok(u16::from_le_bytes([data[0], data[1]]))
        }

        fn get_debug_descriptor<T: libusb::UsbContext>(
            handle: &mut Option<UsbDevice<T>>,
        ) -> Result<usb::DebugDescriptor> {
            let request_type = libusb::request_type(
                libusb::Direction::In,
                libusb::RequestType::Standard,
                libusb::Recipient::Device,
            );
            let request = libusb::constants::LIBUSB_REQUEST_GET_DESCRIPTOR;
            let value = 0x0a << 8;
            let data = Self::get_control_msg(handle, request_type, request, value, 0, 2)?;
            usb::DebugDescriptor::try_from(data.as_slice())
        }

        fn get_bos_descriptor<T: libusb::UsbContext>(
            handle: &mut Option<UsbDevice<T>>,
        ) -> Result<usb::descriptors::bos::BinaryObjectStoreDescriptor> {
            let request_type = libusb::request_type(
                libusb::Direction::In,
                libusb::RequestType::Standard,
                libusb::Recipient::Device,
            );
            let request = libusb::constants::LIBUSB_REQUEST_GET_DESCRIPTOR;
            let value = 0x0f << 8;
            let data = Self::get_control_msg(handle, request_type, request, value, 0, 5)?;
            let total_length = u16::from_le_bytes([data[2], data[3]]);
            log::trace!("Attempt read BOS descriptor total length: {}", total_length);
            // now get full descriptor
            let data = Self::get_control_msg(
                handle,
                request_type,
                request,
                value,
                0,
                total_length as usize,
            )?;
            log::trace!("BOS descriptor data: {:?}", data);
            let mut bos =
                usb::descriptors::bos::BinaryObjectStoreDescriptor::try_from(data.as_slice())?;

            // get any extra descriptor data now with handle
            for c in bos.capabilities.iter_mut() {
                match c {
                    usb::descriptors::bos::BosCapability::WebUsbPlatform(w) => {
                        w.url =
                            Self::get_webusb_url(handle, w.vendor_code, w.landing_page_index).ok();
                        log::trace!("WebUSB URL: {:?}", w.url);
                    }
                    usb::descriptors::bos::BosCapability::Billboard(ref mut b) => {
                        b.additional_info_url =
                            Self::get_descriptor_string(b.additional_info_url_index, handle);
                        for a in b.alternate_modes.iter_mut() {
                            a.alternate_mode_string =
                                Self::get_descriptor_string(a.alternate_mode_string_index, handle);
                        }
                    }
                    _ => (),
                }
            }

            Ok(bos)
        }

        fn get_device_qualifier<T: libusb::UsbContext>(
            handle: &mut Option<UsbDevice<T>>,
        ) -> Result<usb::DeviceQualifierDescriptor> {
            let request_type = libusb::request_type(
                libusb::Direction::In,
                libusb::RequestType::Standard,
                libusb::Recipient::Device,
            );
            let request = libusb::constants::LIBUSB_REQUEST_GET_DESCRIPTOR;
            let value = 0x06 << 8;
            let data = Self::get_control_msg(handle, request_type, request, value, 0, 10)?;
            log::trace!("Device Qualifier descriptor data: {:?}", data);
            usb::DeviceQualifierDescriptor::try_from(data.as_slice())
        }

        /// Gets the WebUSB URL from the device, parsed and formatted as a URL
        ///
        /// https://github.com/gregkh/usbutils/blob/master/lsusb.c#L3261
        fn get_webusb_url<T: libusb::UsbContext>(
            handle: &mut Option<UsbDevice<T>>,
            vendor_request: u8,
            index: u8,
        ) -> Result<String> {
            let request_type = libusb::request_type(
                libusb::Direction::In,
                libusb::RequestType::Vendor,
                libusb::Recipient::Device,
            );
            let value = (usb::WEBUSB_GET_URL as u16) << 8;
            let data = Self::get_control_msg(
                handle,
                request_type,
                vendor_request,
                value,
                index as u16,
                3,
            )?;
            log::trace!("WebUSB URL descriptor data: {:?}", data);
            let len = data[0] as usize;

            if data[1] != usb::USB_DT_WEBUSB_URL {
                return Err(Error {
                    kind: ErrorKind::Parsing,
                    message: "Failed to parse WebUSB URL: Bad URL descriptor type".to_string(),
                });
            }

            if data.len() < len {
                return Err(Error {
                    kind: ErrorKind::Parsing,
                    message: "Failed to parse WebUSB URL: Data length mismatch".to_string(),
                });
            }

            let url = String::from_utf8(data[3..len].to_vec()).map_err(|e| Error {
                kind: ErrorKind::Parsing,
                message: format!("Failed to parse WebUSB URL: {}", e),
            })?;

            match data[2] {
                0x00 => Ok(format!("http://{}", url)),
                0x01 => Ok(format!("https://{}", url)),
                0xFF => Ok(url),
                _ => Err(Error {
                    kind: ErrorKind::Parsing,
                    message: "Failed to parse WebUSB URL: Bad URL scheme".to_string(),
                }),
            }
        }

        /// Build fully described USB device descriptor with extra bytes
        fn build_descriptor_extra<T: libusb::UsbContext>(
            &self,
            handle: &mut Option<UsbDevice<T>>,
            interface_desc: Option<&libusb::InterfaceDescriptor>,
            extra_bytes: &[u8],
        ) -> Result<usb::Descriptor> {
            // Get any extra descriptors into a known type and add any handle data while we have it
            let mut dt = match usb::Descriptor::try_from(extra_bytes) {
                Ok(d) => d,
                Err(e) => {
                    log::debug!("Failed to convert extra descriptor bytes: {}", e);
                    return Err(e);
                }
            };

            // Assign class context to interface since descriptor did not know it
            if let Some(interface_desc) = interface_desc {
                if let Err(e) = dt.update_with_class_context((
                    interface_desc.class_code(),
                    interface_desc.sub_class_code(),
                    interface_desc.protocol_code(),
                )) {
                    log::debug!(
                        "Failed to update extra descriptor with class context: {}",
                        e
                    );
                }
            }

            // get any strings at string indexes while we have handle
            match dt {
                usb::Descriptor::InterfaceAssociation(ref mut iad) => {
                    iad.function_string =
                        Self::get_descriptor_string(iad.function_string_index, handle);
                }
                usb::Descriptor::Device(ref mut c)
                | usb::Descriptor::Interface(ref mut c)
                | usb::Descriptor::Endpoint(ref mut c) => match c {
                    usb::ClassDescriptor::Printer(ref mut p) => {
                        for pd in p.descriptors.iter_mut() {
                            pd.uuid_string =
                                Self::get_descriptor_string(pd.uuid_string_index, handle);
                        }
                    }
                    usb::ClassDescriptor::Communication(ref mut cdc) => match cdc.interface {
                        usb::descriptors::cdc::CdcInterfaceDescriptor::CountrySelection(
                            ref mut d,
                        ) => {
                            d.country_code_date =
                                Self::get_descriptor_string(d.country_code_date_index, handle);
                        }
                        usb::descriptors::cdc::CdcInterfaceDescriptor::NetworkChannel(
                            ref mut d,
                        ) => {
                            d.name = Self::get_descriptor_string(d.name_string_index, handle);
                        }
                        usb::descriptors::cdc::CdcInterfaceDescriptor::EthernetNetworking(
                            ref mut d,
                        ) => {
                            d.mac_address =
                                Self::get_descriptor_string(d.mac_address_index, handle);
                        }
                        usb::descriptors::cdc::CdcInterfaceDescriptor::CommandSet(ref mut d) => {
                            d.command_set_string =
                                Self::get_descriptor_string(d.command_set_string_index, handle);
                        }
                        _ => (),
                    },
                    // grab report descriptor data using usb_control_msg
                    usb::ClassDescriptor::Hid(ref mut hd) => {
                        for rd in hd.descriptors.iter_mut() {
                            if let Some(index) = interface_desc.map(|i| i.interface_number() as u16)
                            {
                                rd.data =
                                    Self::get_report_descriptor(handle, index, rd.length).ok();
                            }
                        }
                    }
                    usb::ClassDescriptor::Midi(ref mut md, _) => match md.interface {
                        usb::descriptors::audio::MidiInterfaceDescriptor::InputJack(ref mut mh) => {
                            mh.jack_string =
                                Self::get_descriptor_string(mh.jack_string_index, handle);
                        }
                        usb::descriptors::audio::MidiInterfaceDescriptor::OutputJack(
                            ref mut mh,
                        ) => {
                            mh.jack_string =
                                Self::get_descriptor_string(mh.jack_string_index, handle);
                        }
                        usb::descriptors::audio::MidiInterfaceDescriptor::Element(ref mut mh) => {
                            mh.element_string =
                                Self::get_descriptor_string(mh.element_string_index, handle);
                        }
                        _ => (),
                    },
                    usb::ClassDescriptor::Audio(ref mut ad, _) => match ad.interface {
                        usb::descriptors::audio::UacInterfaceDescriptor::InputTerminal1(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(ah.channel_names_index, handle);
                            ah.terminal = Self::get_descriptor_string(ah.terminal_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::InputTerminal2(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(ah.channel_names_index, handle);
                            ah.terminal = Self::get_descriptor_string(ah.terminal_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::OutputTerminal1(
                            ref mut ah,
                        ) => {
                            ah.terminal = Self::get_descriptor_string(ah.terminal_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::OutputTerminal2(
                            ref mut ah,
                        ) => {
                            ah.terminal = Self::get_descriptor_string(ah.terminal_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::StreamingInterface2(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(ah.channel_names_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::SelectorUnit1(
                            ref mut ah,
                        ) => {
                            ah.selector = Self::get_descriptor_string(ah.selector_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::SelectorUnit2(
                            ref mut ah,
                        ) => {
                            ah.selector = Self::get_descriptor_string(ah.selector_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ProcessingUnit1(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(ah.channel_names_index, handle);
                            ah.processing =
                                Self::get_descriptor_string(ah.processing_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ProcessingUnit2(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(ah.channel_names_index, handle);
                            ah.processing =
                                Self::get_descriptor_string(ah.processing_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::EffectUnit2(
                            ref mut ah,
                        ) => {
                            ah.effect = Self::get_descriptor_string(ah.effect_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::FeatureUnit1(
                            ref mut ah,
                        ) => {
                            ah.feature = Self::get_descriptor_string(ah.feature_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::FeatureUnit2(
                            ref mut ah,
                        ) => {
                            ah.feature = Self::get_descriptor_string(ah.feature_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ExtensionUnit1(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(ah.channel_names_index, handle);
                            ah.extension = Self::get_descriptor_string(ah.extension_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ExtensionUnit2(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(ah.channel_names_index, handle);
                            ah.extension = Self::get_descriptor_string(ah.extension_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ClockSource2(
                            ref mut ah,
                        ) => {
                            ah.clock_source =
                                Self::get_descriptor_string(ah.clock_source_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ClockSelector2(
                            ref mut ah,
                        ) => {
                            ah.clock_selector =
                                Self::get_descriptor_string(ah.clock_selector_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ClockMultiplier2(
                            ref mut ah,
                        ) => {
                            ah.clock_multiplier =
                                Self::get_descriptor_string(ah.clock_multiplier_index, handle);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::SampleRateConverter2(
                            ref mut ah,
                        ) => {
                            ah.src = Self::get_descriptor_string(ah.src_index, handle);
                        }
                        _ => (),
                    },
                    usb::ClassDescriptor::Video(ref mut vd, _) => match vd.interface {
                        usb::descriptors::video::UvcInterfaceDescriptor::InputTerminal(
                            ref mut vh,
                        ) => {
                            vh.terminal = Self::get_descriptor_string(vh.terminal_index, handle);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::OutputTerminal(
                            ref mut vh,
                        ) => {
                            vh.terminal = Self::get_descriptor_string(vh.terminal_index, handle);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::SelectorUnit(
                            ref mut vh,
                        ) => {
                            vh.selector = Self::get_descriptor_string(vh.selector_index, handle);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::ProcessingUnit(
                            ref mut vh,
                        ) => {
                            vh.processing =
                                Self::get_descriptor_string(vh.processing_index, handle);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::ExtensionUnit(
                            ref mut vh,
                        ) => {
                            vh.extension = Self::get_descriptor_string(vh.extension_index, handle);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::EncodingUnit(
                            ref mut vh,
                        ) => {
                            vh.encoding = Self::get_descriptor_string(vh.encoding_index, handle);
                        }
                        _ => (),
                    },
                    _ => (),
                },
                _ => (),
            }

            Ok(dt)
        }

        fn build_config_descriptor_extra<T: libusb::UsbContext>(
            &self,
            handle: &mut Option<UsbDevice<T>>,
            config_desc: &libusb::ConfigDescriptor,
        ) -> Result<Vec<usb::Descriptor>> {
            let mut extra_bytes = config_desc.extra().to_owned();
            let extra_len = extra_bytes.len();
            let mut taken = 0;
            let mut ret = Vec::new();

            // Iterate on chunks of the header length
            while taken < extra_len && extra_len >= 2 {
                let dt_len = extra_bytes[0] as usize;
                let dt = self.build_descriptor_extra(
                    handle,
                    None,
                    &extra_bytes.drain(..dt_len).collect::<Vec<u8>>(),
                )?;
                log::trace!("Config descriptor extra: {:?}", dt);
                ret.push(dt);
                taken += dt_len;
            }

            Ok(ret)
        }

        fn build_interface_descriptor_extra<T: libusb::UsbContext>(
            &self,
            handle: &mut Option<UsbDevice<T>>,
            interface_desc: &libusb::InterfaceDescriptor,
        ) -> Result<Vec<usb::Descriptor>> {
            let mut extra_bytes = interface_desc.extra().to_owned();
            let extra_len = extra_bytes.len();
            let mut taken = 0;
            let mut ret = Vec::new();

            // Iterate on chunks of the header length
            while taken < extra_len && extra_len >= 2 {
                let dt_len = extra_bytes[0] as usize;
                if let Some(b) = extra_bytes.get_mut(1) {
                    // Mask request type LIBUSB_REQUEST_TYPE_CLASS
                    *b &= !(0x01 << 5);
                    // if not Device or Interface, force it to Interface
                    if *b != 0x01 || *b != 0x04 {
                        *b = 0x04;
                    }
                }

                let dt = self.build_descriptor_extra(
                    handle,
                    Some(interface_desc),
                    &extra_bytes.drain(..dt_len).collect::<Vec<u8>>(),
                )?;

                log::trace!("Interface descriptor extra: {:?}", dt);
                ret.push(dt);
                taken += dt_len;
            }

            Ok(ret)
        }

        fn build_endpoint_descriptor_extra<T: libusb::UsbContext>(
            &self,
            handle: &mut Option<UsbDevice<T>>,
            interface_desc: &libusb::InterfaceDescriptor,
            endpoint_desc: &libusb::EndpointDescriptor,
        ) -> Result<Option<Vec<usb::Descriptor>>> {
            match endpoint_desc.extra() {
                Some(extra_bytes) => {
                    let mut extra_bytes = extra_bytes.to_owned();
                    let extra_len = extra_bytes.len();
                    let mut taken = 0;
                    let mut ret = Vec::new();

                    // Iterate on chunks of the header length
                    while taken < extra_len && extra_len >= 2 {
                        let dt_len = extra_bytes[0] as usize;
                        if let Some(b) = extra_bytes.get_mut(1) {
                            // Mask request type LIBUSB_REQUEST_TYPE_CLASS for Endpoint: 0x25
                            if *b == 0x25 {
                                *b &= !(0x01 << 5);
                            }
                        };

                        let dt = self.build_descriptor_extra(
                            handle,
                            Some(interface_desc),
                            &extra_bytes.drain(..dt_len).collect::<Vec<u8>>(),
                        )?;

                        log::trace!("Endpoint descriptor extra: {:?}", dt);
                        ret.push(dt);
                        taken += dt_len;
                    }

                    Ok(Some(ret))
                }
                None => Ok(None),
            }
        }

        fn build_endpoints<T: libusb::UsbContext>(
            &self,
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
                    extra: self
                        .build_endpoint_descriptor_extra(handle, interface_desc, &endpoint_desc)
                        .ok()
                        .flatten(),
                });
            }

            ret
        }

        fn build_interfaces<T: libusb::UsbContext>(
            &self,
            device: &libusb::Device<T>,
            handle: &mut Option<UsbDevice<T>>,
            config_desc: &libusb::ConfigDescriptor,
            with_udev: bool,
        ) -> Result<Vec<usb::USBInterface>> {
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
                        name: Self::get_sysfs_string(&path, "interface")
                            .or(Self::get_interface_string(&interface_desc, handle))
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
                            .build_interface_descriptor_extra(handle, &interface_desc)
                            .ok(),
                    };

                    // flag allows us to try again without udev if it raises an error
                    // but record the error for printing
                    if with_udev {
                        interface.driver = Self::get_udev_driver_name(&interface.path)?;
                        interface.syspath = Self::get_udev_syspath(&interface.path)?;
                    };

                    ret.push(interface);
                }
            }

            Ok(ret)
        }

        fn build_configurations<T: libusb::UsbContext>(
            &self,
            device: &libusb::Device<T>,
            handle: &mut Option<UsbDevice<T>>,
            device_desc: &libusb::DeviceDescriptor,
            sp_device: &system_profiler::USBDevice,
            with_udev: bool,
        ) -> Result<Vec<usb::USBConfiguration>> {
            // Retrieve the current configuration (if available)
            let cur_config = Self::get_sysfs_configuration_string(&sp_device.sysfs_name());
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
                    name: Self::get_configuration_string(&config_desc, handle)
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
                    interfaces: self.build_interfaces(device, handle, &config_desc, with_udev)?,
                    extra: self
                        .build_config_descriptor_extra(handle, &config_desc)
                        .ok(),
                });
            }

            Ok(ret)
        }

        #[allow(unused_variables)]
        fn build_spdevice_extra<T: libusb::UsbContext>(
            &self,
            device: &libusb::Device<T>,
            handle: &mut Option<UsbDevice<T>>,
            device_desc: &libusb::DeviceDescriptor,
            sp_device: &system_profiler::USBDevice,
            with_udev: bool,
        ) -> Result<usb::USBDeviceExtra> {
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
                vendor: names::vendor(device_desc.vendor_id())
                    .or(usb_ids::Vendor::from_id(device_desc.vendor_id())
                        .map(|v| v.name().to_owned())),
                product_name: names::product(device_desc.vendor_id(), device_desc.product_id()).or(
                    usb_ids::Device::from_vid_pid(
                        device_desc.vendor_id(),
                        device_desc.product_id(),
                    )
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
                extra.driver = Self::get_udev_driver_name(&sysfs_name)?;
                extra.syspath = Self::get_udev_syspath(&sysfs_name)?;
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
                    bos.capabilities.iter().any(|c| {
                        matches!(c, usb::descriptors::bos::BosCapability::SuperSpeedPlus(_))
                    })
                } else {
                    false
                };
                let bcd = sp_device.bcd_usb.map_or(0x0100, |v| v.into());
                extra.hub =
                    Self::get_hub_descriptor(handle, device_desc.protocol_code(), bcd, has_ssp)
                        .ok();
            }

            Ok(extra)
        }

        /// Builds a [`system_profiler::USBDevice`] from a [`libusb::Device`] by using `device_descriptor()` and intrograting for configuration strings. Optionally with `with_extra` will gather full device information, including from udev if feature is present.
        ///
        /// [`system_profiler::USBDevice.profiler_error`] `Option<String>` will contain any non-critical error during gather of `with_extra` data - normally due to permissions preventing open of device descriptors.
        fn build_spdevice<T: libusb::UsbContext>(
            &self,
            device: &libusb::Device<T>,
            with_extra: bool,
        ) -> Result<system_profiler::USBDevice> {
            let timeout = std::time::Duration::from_secs(1);
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
            sp_device.manufacturer = Self::get_manufacturer_string(&device_desc, &mut usb_device) // descriptor
                // sysfs cache
                .or(Self::get_sysfs_string(
                    &sp_device.sysfs_name(),
                    "manufacturer",
                ))
                // udev-hwdb
                .or(names::vendor(device_desc.vendor_id())) // udev, usb-ids if error
                // usb-ids
                .or(usb_ids::Vendor::from_id(device_desc.vendor_id())
                    .map(|vendor| vendor.name().to_owned()));

            sp_device.name =
                Self::get_product_string(&device_desc, &mut usb_device) // descriptor
                    // sysfs cache
                    .or(Self::get_sysfs_string(&sp_device.sysfs_name(), "product"))
                    // udev-hwdb
                    .or(names::product(
                        device_desc.vendor_id(),
                        device_desc.product_id(),
                    ))
                    // usb-ids
                    .or(usb_ids::Device::from_vid_pid(
                        device_desc.vendor_id(),
                        device_desc.product_id(),
                    )
                    .map(|device| device.name().to_owned()))
                    // empty
                    .unwrap_or_default();

            sp_device.serial_num = Self::get_serial_string(&device_desc, &mut usb_device)
                .or(Self::get_sysfs_string(&sp_device.sysfs_name(), "serial"));

            let extra_error_str = if with_extra {
                match self.build_spdevice_extra(
                    device,
                    &mut usb_device,
                    &device_desc,
                    &sp_device,
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
                                &mut usb_device,
                                &device_desc,
                                &sp_device,
                                false,
                            )?);
                            Some(format!(
                                    "Failed to get udev data for {}, probably requires elevated permissions",
                                    sp_device
                            ))
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
    }

    impl Profiler for LibUsbProfiler {
        fn profile_devices(
            &self,
            cache: &mut Vec<system_profiler::USBDevice>,
            root_hubs: &mut HashMap<u8, system_profiler::USBDevice>,
            with_extra: bool,
        ) -> Result<()> {
            // run through devices building USBDevice types
            for device in libusb::DeviceList::new()?.iter() {
                match self.build_spdevice(&device, with_extra) {
                    Ok(sp_device) => {
                        cache.push(sp_device.to_owned());
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

                        // save it if it's a root_hub for assigning to bus data
                        if !cfg!(target_os = "macos") && sp_device.is_root_hub() {
                            root_hubs.insert(sp_device.location_id.bus, sp_device);
                        }
                    }
                    Err(e) => eprintln!("Failed to get data for {:?}: {}", device, e),
                }
            }

            Ok(())
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

    struct UsbDevice<T: libusb::UsbContext> {
        handle: libusb::DeviceHandle<T>,
        language: libusb::Language,
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
}

#[cfg(feature = "nusb")]
pub mod nusb {
    //! Uses nusb (pure Rust) to get system USB information. Requires 'nusb' feature. Uses [`crate::system_profiler`] types to hold data so that it is cross-compatible with macOS system_profiler command.
    use super::*;
    use crate::error::{Error, ErrorKind};
    use crate::lsusb::names;
    use crate::usb::{self, NumericalUnit};
    use ::nusb;
    use usb_ids::{self, FromId};

    #[derive(Debug)]
    pub(crate) struct NusbProfiler;

    impl NusbProfiler {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        fn get_control_msg(
            device: &nusb::Device,
            control: nusb::transfer::Control,
            length: usize,
        ) -> Result<Vec<u8>> {
            let mut data = vec![0; length];
            let read = device
                .control_in_blocking(
                    control,
                    data.as_mut_slice(),
                    std::time::Duration::from_secs(1),
                )
                .map_err(|e| Error {
                    kind: ErrorKind::Nusb,
                    message: format!("Failed to get report descriptor: {}", e),
                })?;

            if read < length {
                log::debug!(
                    "Failed to get full report descriptor, only read {} of {}",
                    read,
                    length
                );
                return Err(Error {
                    kind: ErrorKind::Nusb,
                    message: format!(
                        "Failed to get full report descriptor, only read {} of {}",
                        read, length
                    ),
                });
            }

            Ok(data)
        }

        #[cfg(target_os = "windows")]
        fn get_control_msg(
            device: &nusb::Device,
            control: nusb::transfer::Control,
            length: usize,
        ) -> Result<Vec<u8>> {
            let mut data = vec![0; length];
            // TODO this should probably be dependant on the interface being called?
            let interface = device.claim_interface(0)?;
            let read = interface
                .control_in_blocking(
                    control,
                    data.as_mut_slice(),
                    std::time::Duration::from_secs(1),
                )
                .map_err(|e| Error {
                    kind: ErrorKind::Nusb,
                    message: format!("Failed to get report descriptor: {}", e),
                })?;

            if read < length {
                log::debug!(
                    "Failed to get full report descriptor, only read {} of {}",
                    read,
                    length
                );
                return Err(Error {
                    kind: ErrorKind::Nusb,
                    message: format!(
                        "Failed to get full report descriptor, only read {} of {}",
                        read, length
                    ),
                });
            }

            Ok(data)
        }

        fn get_report_descriptor(
            device: &nusb::Device,
            index: u16,
            length: u16,
        ) -> Result<Vec<u8>> {
            let control = nusb::transfer::Control {
                control_type: nusb::transfer::ControlType::Standard,
                request: 0x06,
                value: 0x22 << 8,
                index,
                recipient: nusb::transfer::Recipient::Interface,
            };

            Self::get_control_msg(device, control, length as usize)
        }

        fn get_hub_descriptor(
            device: &nusb::Device,
            protocol: u8,
            bcd: u16,
            has_ssp: bool,
        ) -> Result<usb::HubDescriptor> {
            let is_ext_status = protocol == 3 && bcd >= 0x0310 && has_ssp;
            let value = if bcd >= 0x0300 { 0x2a << 8 } else { 0x29 << 8 };
            let control = nusb::transfer::Control {
                control_type: nusb::transfer::ControlType::Class,
                request: 0x06,
                value,
                index: 0,
                recipient: nusb::transfer::Recipient::Device,
            };
            let data = Self::get_control_msg(device, control, 9)?;
            let mut hub = usb::HubDescriptor::try_from(data.as_slice())?;

            // get port statuses
            let mut port_statues: Vec<[u8; 8]> = Vec::with_capacity(hub.num_ports as usize);
            for p in 0..hub.num_ports {
                let control = nusb::transfer::Control {
                    control_type: nusb::transfer::ControlType::Class,
                    request: 0x00,
                    index: p as u16 + 1,
                    value: 0x23 << 8,
                    recipient: nusb::transfer::Recipient::Other,
                };
                match Self::get_control_msg(device, control, if is_ext_status { 8 } else { 4 }) {
                    Ok(mut data) => {
                        if data.len() < 8 {
                            let remaining = 8 - data.len();
                            data.extend(vec![0; remaining]);
                        }
                        port_statues.push(data.try_into().unwrap());
                    }
                    Err(e) => {
                        log::warn!("Failed to get port {} status: {}", p + 1, e);
                        return Ok(hub);
                    }
                }
            }

            hub.port_statuses = Some(port_statues);

            Ok(hub)
        }

        fn get_device_status(device: &nusb::Device) -> Result<u16> {
            let control = nusb::transfer::Control {
                control_type: nusb::transfer::ControlType::Standard,
                request: 0x00,
                value: 0,
                index: 0,
                recipient: nusb::transfer::Recipient::Device,
            };
            let data = Self::get_control_msg(device, control, 2)?;
            Ok(u16::from_le_bytes([data[0], data[1]]))
        }

        fn get_debug_descriptor(device: &nusb::Device) -> Result<usb::DebugDescriptor> {
            let control = nusb::transfer::Control {
                control_type: nusb::transfer::ControlType::Standard,
                request: 0x06,
                value: 0x0a << 8,
                index: 0,
                recipient: nusb::transfer::Recipient::Device,
            };
            let data = Self::get_control_msg(device, control, 2)?;
            usb::DebugDescriptor::try_from(data.as_slice())
        }

        fn get_bos_descriptor(
            device: &nusb::Device,
        ) -> Result<usb::descriptors::bos::BinaryObjectStoreDescriptor> {
            let control = nusb::transfer::Control {
                control_type: nusb::transfer::ControlType::Standard,
                request: 0x06,
                value: 0x0f << 8,
                index: 0,
                recipient: nusb::transfer::Recipient::Device,
            };
            let data = Self::get_control_msg(device, control, 5)?;
            let total_length = u16::from_le_bytes([data[2], data[3]]);
            log::trace!("Attempt read BOS descriptor total length: {}", total_length);
            // now get full descriptor
            let control = nusb::transfer::Control {
                control_type: nusb::transfer::ControlType::Standard,
                request: 0x06,
                value: 0x0f << 8,
                index: 0,
                recipient: nusb::transfer::Recipient::Device,
            };
            let data = Self::get_control_msg(device, control, total_length as usize)?;
            log::trace!("BOS descriptor data: {:?}", data);
            let mut bos =
                usb::descriptors::bos::BinaryObjectStoreDescriptor::try_from(data.as_slice())?;

            // get any extra descriptor data now with handle
            for c in bos.capabilities.iter_mut() {
                match c {
                    usb::descriptors::bos::BosCapability::WebUsbPlatform(w) => {
                        w.url =
                            Self::get_webusb_url(device, w.vendor_code, w.landing_page_index).ok();
                        log::trace!("WebUSB URL: {:?}", w.url);
                    }
                    usb::descriptors::bos::BosCapability::Billboard(ref mut b) => {
                        b.additional_info_url =
                            Self::get_descriptor_string(device, b.additional_info_url_index);
                        for a in b.alternate_modes.iter_mut() {
                            a.alternate_mode_string =
                                Self::get_descriptor_string(device, a.alternate_mode_string_index);
                        }
                    }
                    _ => (),
                }
            }

            Ok(bos)
        }

        fn get_device_qualifier(device: &nusb::Device) -> Result<usb::DeviceQualifierDescriptor> {
            let control = nusb::transfer::Control {
                control_type: nusb::transfer::ControlType::Standard,
                request: 0x06,
                value: 0x06 << 8,
                index: 0,
                recipient: nusb::transfer::Recipient::Device,
            };
            let data = Self::get_control_msg(device, control, 10)?;
            log::trace!("Device Qualifier descriptor data: {:?}", data);
            usb::DeviceQualifierDescriptor::try_from(data.as_slice())
        }

        /// Gets the WebUSB URL from the device, parsed and formatted as a URL
        ///
        /// https://github.com/gregkh/usbutils/blob/master/lsusb.c#L3261
        fn get_webusb_url(handle: &nusb::Device, vendor_request: u8, index: u8) -> Result<String> {
            let control = nusb::transfer::Control {
                control_type: nusb::transfer::ControlType::Vendor,
                request: vendor_request,
                value: (usb::WEBUSB_GET_URL as u16) << 8,
                index: index as u16,
                recipient: nusb::transfer::Recipient::Device,
            };
            let data = Self::get_control_msg(handle, control, 3)?;
            log::trace!("WebUSB URL descriptor data: {:?}", data);
            let len = data[0] as usize;

            if data[1] != usb::USB_DT_WEBUSB_URL {
                return Err(Error {
                    kind: ErrorKind::Parsing,
                    message: "Failed to parse WebUSB URL: Bad URL descriptor type".to_string(),
                });
            }

            if data.len() < len {
                return Err(Error {
                    kind: ErrorKind::Parsing,
                    message: "Failed to parse WebUSB URL: Data length mismatch".to_string(),
                });
            }

            let url = String::from_utf8(data[3..len].to_vec()).map_err(|e| Error {
                kind: ErrorKind::Parsing,
                message: format!("Failed to parse WebUSB URL: {}", e),
            })?;

            match data[2] {
                0x00 => Ok(format!("http://{}", url)),
                0x01 => Ok(format!("https://{}", url)),
                0xFF => Ok(url),
                _ => Err(Error {
                    kind: ErrorKind::Parsing,
                    message: "Failed to parse WebUSB URL: Bad URL scheme".to_string(),
                }),
            }
        }

        fn get_descriptor_string(device: &nusb::Device, string_index: u8) -> Option<String> {
            // 0 is reserved for language codes and we can assume we're not doing that for descriptor strings
            if string_index == 0 {
                return None;
            }

            device
                .get_string_descriptor(string_index, 0, std::time::Duration::from_secs(1))
                .map(|s| s.to_string())
                .ok()
        }

        /// Build fully described USB device descriptor with extra bytes
        fn build_descriptor_extra(
            &self,
            device: &nusb::Device,
            interface_alt: Option<&nusb::descriptors::InterfaceAltSetting>,
            extra_bytes: &[u8],
        ) -> Result<usb::Descriptor> {
            // Get any extra descriptors into a known type and add any handle data while we have it
            let mut dt = match usb::Descriptor::try_from(extra_bytes) {
                Ok(d) => d,
                Err(e) => {
                    log::debug!("Failed to convert extra descriptor bytes: {}", e);
                    return Err(e);
                }
            };

            // Assign class context to interface since descriptor did not know it
            if let Some(interface) = interface_alt {
                if let Err(e) = dt.update_with_class_context((
                    interface.class(),
                    interface.subclass(),
                    interface.protocol(),
                )) {
                    log::debug!(
                        "Failed to update extra descriptor with class context: {}",
                        e
                    );
                }
            }

            // get any strings at string indexes while we have handle
            match dt {
                usb::Descriptor::InterfaceAssociation(ref mut iad) => {
                    iad.function_string =
                        Self::get_descriptor_string(device, iad.function_string_index);
                }
                usb::Descriptor::Device(ref mut c)
                | usb::Descriptor::Interface(ref mut c)
                | usb::Descriptor::Endpoint(ref mut c) => match c {
                    usb::ClassDescriptor::Printer(ref mut p) => {
                        for pd in p.descriptors.iter_mut() {
                            pd.uuid_string =
                                Self::get_descriptor_string(device, pd.uuid_string_index);
                        }
                    }
                    usb::ClassDescriptor::Communication(ref mut cdc) => match cdc.interface {
                        usb::descriptors::cdc::CdcInterfaceDescriptor::CountrySelection(
                            ref mut d,
                        ) => {
                            d.country_code_date =
                                Self::get_descriptor_string(device, d.country_code_date_index);
                        }
                        usb::descriptors::cdc::CdcInterfaceDescriptor::NetworkChannel(
                            ref mut d,
                        ) => {
                            d.name = Self::get_descriptor_string(device, d.name_string_index);
                        }
                        usb::descriptors::cdc::CdcInterfaceDescriptor::EthernetNetworking(
                            ref mut d,
                        ) => {
                            d.mac_address =
                                Self::get_descriptor_string(device, d.mac_address_index);
                        }
                        usb::descriptors::cdc::CdcInterfaceDescriptor::CommandSet(ref mut d) => {
                            d.command_set_string =
                                Self::get_descriptor_string(device, d.command_set_string_index);
                        }
                        _ => (),
                    },
                    // grab report descriptor data using usb_control_msg
                    usb::ClassDescriptor::Hid(ref mut hd) => {
                        for rd in hd.descriptors.iter_mut() {
                            if let Some(index) = interface_alt.map(|i| i.interface_number() as u16)
                            {
                                rd.data =
                                    Self::get_report_descriptor(device, index, rd.length).ok();
                            }
                        }
                    }
                    usb::ClassDescriptor::Midi(ref mut md, _) => match md.interface {
                        usb::descriptors::audio::MidiInterfaceDescriptor::InputJack(ref mut mh) => {
                            mh.jack_string =
                                Self::get_descriptor_string(device, mh.jack_string_index);
                        }
                        usb::descriptors::audio::MidiInterfaceDescriptor::OutputJack(
                            ref mut mh,
                        ) => {
                            mh.jack_string =
                                Self::get_descriptor_string(device, mh.jack_string_index);
                        }
                        usb::descriptors::audio::MidiInterfaceDescriptor::Element(ref mut mh) => {
                            mh.element_string =
                                Self::get_descriptor_string(device, mh.element_string_index);
                        }
                        _ => (),
                    },
                    usb::ClassDescriptor::Audio(ref mut ad, _) => match ad.interface {
                        usb::descriptors::audio::UacInterfaceDescriptor::InputTerminal1(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(device, ah.channel_names_index);
                            ah.terminal = Self::get_descriptor_string(device, ah.terminal_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::InputTerminal2(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(device, ah.channel_names_index);
                            ah.terminal = Self::get_descriptor_string(device, ah.terminal_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::OutputTerminal1(
                            ref mut ah,
                        ) => {
                            ah.terminal = Self::get_descriptor_string(device, ah.terminal_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::OutputTerminal2(
                            ref mut ah,
                        ) => {
                            ah.terminal = Self::get_descriptor_string(device, ah.terminal_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::StreamingInterface2(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(device, ah.channel_names_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::SelectorUnit1(
                            ref mut ah,
                        ) => {
                            ah.selector = Self::get_descriptor_string(device, ah.selector_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::SelectorUnit2(
                            ref mut ah,
                        ) => {
                            ah.selector = Self::get_descriptor_string(device, ah.selector_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ProcessingUnit1(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(device, ah.channel_names_index);
                            ah.processing =
                                Self::get_descriptor_string(device, ah.processing_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ProcessingUnit2(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(device, ah.channel_names_index);
                            ah.processing =
                                Self::get_descriptor_string(device, ah.processing_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::EffectUnit2(
                            ref mut ah,
                        ) => {
                            ah.effect = Self::get_descriptor_string(device, ah.effect_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::FeatureUnit1(
                            ref mut ah,
                        ) => {
                            ah.feature = Self::get_descriptor_string(device, ah.feature_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::FeatureUnit2(
                            ref mut ah,
                        ) => {
                            ah.feature = Self::get_descriptor_string(device, ah.feature_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ExtensionUnit1(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(device, ah.channel_names_index);
                            ah.extension = Self::get_descriptor_string(device, ah.extension_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ExtensionUnit2(
                            ref mut ah,
                        ) => {
                            ah.channel_names =
                                Self::get_descriptor_string(device, ah.channel_names_index);
                            ah.extension = Self::get_descriptor_string(device, ah.extension_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ClockSource2(
                            ref mut ah,
                        ) => {
                            ah.clock_source =
                                Self::get_descriptor_string(device, ah.clock_source_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ClockSelector2(
                            ref mut ah,
                        ) => {
                            ah.clock_selector =
                                Self::get_descriptor_string(device, ah.clock_selector_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::ClockMultiplier2(
                            ref mut ah,
                        ) => {
                            ah.clock_multiplier =
                                Self::get_descriptor_string(device, ah.clock_multiplier_index);
                        }
                        usb::descriptors::audio::UacInterfaceDescriptor::SampleRateConverter2(
                            ref mut ah,
                        ) => {
                            ah.src = Self::get_descriptor_string(device, ah.src_index);
                        }
                        _ => (),
                    },
                    usb::ClassDescriptor::Video(ref mut vd, _) => match vd.interface {
                        usb::descriptors::video::UvcInterfaceDescriptor::InputTerminal(
                            ref mut vh,
                        ) => {
                            vh.terminal = Self::get_descriptor_string(device, vh.terminal_index);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::OutputTerminal(
                            ref mut vh,
                        ) => {
                            vh.terminal = Self::get_descriptor_string(device, vh.terminal_index);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::SelectorUnit(
                            ref mut vh,
                        ) => {
                            vh.selector = Self::get_descriptor_string(device, vh.selector_index);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::ProcessingUnit(
                            ref mut vh,
                        ) => {
                            vh.processing =
                                Self::get_descriptor_string(device, vh.processing_index);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::ExtensionUnit(
                            ref mut vh,
                        ) => {
                            vh.extension = Self::get_descriptor_string(device, vh.extension_index);
                        }
                        usb::descriptors::video::UvcInterfaceDescriptor::EncodingUnit(
                            ref mut vh,
                        ) => {
                            vh.encoding = Self::get_descriptor_string(device, vh.encoding_index);
                        }
                        _ => (),
                    },
                    _ => (),
                },
                _ => (),
            }

            Ok(dt)
        }

        fn build_config_descriptor_extra(
            &self,
            device: &nusb::Device,
            mut raw: Vec<u8>,
        ) -> Result<Vec<usb::Descriptor>> {
            let extra_len = raw.len();
            let mut taken = 0;
            let mut ret = Vec::new();

            // Iterate on chunks of the header length
            while taken < extra_len && extra_len >= 2 {
                let dt_len = raw[0] as usize;
                let dt = self.build_descriptor_extra(
                    device,
                    None,
                    &raw.drain(..dt_len).collect::<Vec<u8>>(),
                )?;
                log::trace!("Config descriptor extra: {:?}", dt);
                ret.push(dt);
                taken += dt_len;
            }

            Ok(ret)
        }

        fn build_interface_descriptor_extra(
            &self,
            device: &nusb::Device,
            interface_desc: &nusb::descriptors::InterfaceAltSetting,
            mut raw: Vec<u8>,
        ) -> Result<Vec<usb::Descriptor>> {
            let extra_len = raw.len();
            let mut taken = 0;
            let mut ret = Vec::new();

            // Iterate on chunks of the header length
            while taken < extra_len && extra_len >= 2 {
                let dt_len = raw[0] as usize;
                if let Some(b) = raw.get_mut(1) {
                    // Mask request type LIBUSB_REQUEST_TYPE_CLASS
                    *b &= !(0x01 << 5);
                    // if not Device or Interface, force it to Interface
                    if *b != 0x01 || *b != 0x04 {
                        *b = 0x04;
                    }
                }

                let dt = self.build_descriptor_extra(
                    device,
                    Some(interface_desc),
                    &raw.drain(..dt_len).collect::<Vec<u8>>(),
                )?;

                log::trace!("Interface descriptor extra: {:?}", dt);
                ret.push(dt);
                taken += dt_len;
            }

            Ok(ret)
        }

        fn build_endpoint_descriptor_extra(
            &self,
            device: &nusb::Device,
            interface_desc: &nusb::descriptors::InterfaceAltSetting,
            mut raw: Vec<u8>,
        ) -> Result<Option<Vec<usb::Descriptor>>> {
            let extra_len = raw.len();
            let mut taken = 0;
            let mut ret = Vec::new();

            // Iterate on chunks of the header length
            while taken < extra_len && extra_len >= 2 {
                let dt_len = raw[0] as usize;
                if let Some(b) = raw.get_mut(1) {
                    // Mask request type LIBUSB_REQUEST_TYPE_CLASS for Endpoint: 0x25
                    if *b == 0x25 {
                        *b &= !(0x01 << 5);
                    }
                };

                let dt = self.build_descriptor_extra(
                    device,
                    Some(interface_desc),
                    &raw.drain(..dt_len).collect::<Vec<u8>>(),
                )?;

                log::trace!("Endpoint descriptor extra: {:?}", dt);
                ret.push(dt);
                taken += dt_len;
            }

            Ok(Some(ret))
        }

        fn build_endpoints(
            &self,
            device: &nusb::Device,
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
                        .build_endpoint_descriptor_extra(device, interface_desc, endpoint_extra)
                        .ok()
                        .flatten(),
                });
            }

            ret
        }

        fn build_interfaces(
            &self,
            device: &nusb::Device,
            sp_device: &system_profiler::USBDevice,
            config: &nusb::descriptors::Configuration,
            with_udev: bool,
        ) -> Result<Vec<usb::USBInterface>> {
            let mut ret: Vec<usb::USBInterface> = Vec::new();

            for interface in config.interfaces() {
                for interface_alt in interface.alt_settings() {
                    let path = usb::get_interface_path(
                        sp_device.location_id.bus,
                        &sp_device.location_id.tree_positions,
                        config.configuration_value(),
                        interface_alt.interface_number(),
                    );
                    let name = if let Some(i) = interface_alt.string_index() {
                        device
                            .get_string_descriptor(i, 0, std::time::Duration::from_secs(1))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    let interface_desc = interface_alt.descriptors().next().unwrap();
                    let interface_extra = interface_alt
                        .descriptors()
                        .skip(1)
                        // only want device and interface descriptors - nusb everything trailing
                        .filter(|d| (d.descriptor_type() & 0x0F) == 0x04 || (d.descriptor_type() & 0x0F) == 0x01)
                        .flat_map(|d| d.to_vec())
                        .collect::<Vec<u8>>();

                    let mut interface = usb::USBInterface {
                        name,
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
                                &interface_alt,
                                interface_extra,
                            )
                            .ok(),
                    };

                    // flag allows us to try again without udev if it raises an error
                    // but record the error for printing
                    if with_udev {
                        interface.driver = Self::get_udev_driver_name(&interface.path)?;
                        interface.syspath = Self::get_udev_syspath(&interface.path)?;
                    };

                    ret.push(interface);
                }
            }

            Ok(ret)
        }

        fn build_configurations(
            &self,
            device: &nusb::Device,
            sp_device: &system_profiler::USBDevice,
            with_udev: bool,
        ) -> Result<Vec<usb::USBConfiguration>> {
            let mut ret: Vec<usb::USBConfiguration> = Vec::new();

            for c in device.configurations() {
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

                let name = if let Some(i) = c.string_index() {
                    device
                        .get_string_descriptor(i, 0, std::time::Duration::from_secs(1))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
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
                    name,
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
                    interfaces: self.build_interfaces(device, &sp_device, &c, with_udev)?,
                    extra: self
                        .build_config_descriptor_extra(device, config_extra)
                        .ok(),
                });
            }

            Ok(ret)
        }

        fn build_spdevice_extra(
            &self,
            device: &nusb::Device,
            sp_device: &mut system_profiler::USBDevice,
            with_udev: bool,
        ) -> Result<usb::USBDeviceExtra> {
            let device_desc_raw =
                device.get_descriptor(0x01, 0x00, 0x00, std::time::Duration::from_secs(1))?;
            let device_desc: usb::DeviceDescriptor =
                usb::DeviceDescriptor::try_from(device_desc_raw.as_slice())?;
            sp_device.bcd_usb = Some(device_desc.usb_version);

            let mut extra = usb::USBDeviceExtra {
                max_packet_size: device_desc.max_packet_size,
                string_indexes: (
                    device_desc.product_string_index,
                    device_desc.manufacturer_string_index,
                    device_desc.serial_number_string_index,
                ),
                driver: None,
                syspath: Self::get_syspath(&sp_device.sysfs_name()),
                // These are idProduct, idVendor in lsusb - from udev_hwdb/usb-ids
                vendor: names::vendor(device_desc.vendor_id)
                    .or(usb_ids::Vendor::from_id(device_desc.vendor_id)
                        .map(|v| v.name().to_owned())),
                product_name: names::product(device_desc.vendor_id, device_desc.product_id).or(
                    usb_ids::Device::from_vid_pid(device_desc.vendor_id, device_desc.product_id)
                        .map(|v| v.name().to_owned()),
                ),
                configurations: self.build_configurations(device, &sp_device, with_udev)?,
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
                extra.driver = Self::get_udev_driver_name(&sysfs_name)?;
                extra.syspath = Self::get_udev_syspath(&sysfs_name)?;
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
                    bos.capabilities.iter().any(|c| {
                        matches!(c, usb::descriptors::bos::BosCapability::SuperSpeedPlus(_))
                    })
                } else {
                    false
                };
                let bcd = sp_device.bcd_usb.map_or(0x0100, |v| v.into());
                extra.hub =
                    Self::get_hub_descriptor(device, device_desc.device_protocol, bcd, has_ssp)
                        .ok();
            }

            Ok(extra)
        }

        fn build_spdevice(
            &self,
            device_info: &nusb::DeviceInfo,
            with_extra: bool,
        ) -> Result<system_profiler::USBDevice> {
            let speed = device_info.speed().map(|s| {
                let s = usb::Speed::from(s);
                system_profiler::DeviceSpeed::SpeedValue(s)
            });

            let mut sp_device = system_profiler::USBDevice {
                vendor_id: Some(device_info.vendor_id()),
                product_id: Some(device_info.product_id()),
                device_speed: speed,
                location_id: system_profiler::DeviceLocation {
                    bus: device_info.bus_number(),
                    number: device_info.device_address(),
                    tree_positions: device_info.port_chain().unwrap_or(&[1, 0]).to_owned(),
                },
                bcd_device: Some(usb::Version::from_bcd(device_info.device_version())),
                // gets added on the extra read
                bcd_usb: None,
                class: Some(usb::ClassCode::from(device_info.class())),
                sub_class: Some(device_info.subclass()),
                protocol: Some(device_info.protocol()),
                ..Default::default()
            };

            // tree positions in relative to bus so remove bus number and if it's a bus (port 0), clear the vec
            // (legacy to libusb code)
            if sp_device.location_id.tree_positions.get(1) == Some(&0) {
                sp_device.location_id.tree_positions = vec![];
            } else {
                sp_device.location_id.tree_positions = sp_device
                    .location_id
                    .tree_positions
                    .into_iter()
                    .skip(1)
                    .collect();
            }

            sp_device.manufacturer =
                device_info
                    .manufacturer_string()
                    .map(|s| s.to_string())
                    .or(Self::get_sysfs_string(
                        &sp_device.sysfs_name(),
                        "manufacturer",
                    ))
                    .or(names::vendor(device_info.vendor_id()))
                    .or(usb_ids::Vendor::from_id(device_info.vendor_id())
                        .map(|v| v.name().to_string()));
            sp_device.name = device_info
                .product_string()
                .map(|s| s.to_string())
                .or(Self::get_sysfs_string(&sp_device.sysfs_name(), "product"))
                .or(names::product(
                    device_info.vendor_id(),
                    device_info.product_id(),
                ))
                .or(usb_ids::Device::from_vid_pid(
                    device_info.vendor_id(),
                    device_info.product_id(),
                )
                .map(|d| d.name().to_string()))
                .unwrap_or_default();
            sp_device.serial_num = device_info
                .serial_number()
                .map(|s| s.to_string())
                .or(Self::get_sysfs_string(&sp_device.sysfs_name(), "serial"));

            if let Ok(device) = device_info.open() {
                let mut error_str = None;

                let extra_error_str = if with_extra {
                    match self.build_spdevice_extra(&device, &mut sp_device, true) {
                        Ok(extra) => {
                            sp_device.extra = Some(extra);
                            None
                        }
                        Err(e) => {
                            // try again without udev if we have that feature but return message so device still added
                            if cfg!(feature = "udev") && e.kind() == ErrorKind::Udev {
                                sp_device.extra = Some(self.build_spdevice_extra(
                                    &device,
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
                } else {
                    None
                };

                if error_str.is_none() {
                    error_str = extra_error_str;
                }

                sp_device.profiler_error = error_str;
            } else {
                log::warn!("Failed to open device for extra data: {:04x}:{:04x}. Ensure user has USB access permissions: https://docs.rs/nusb/latest/nusb/#linux", device_info.vendor_id(), device_info.product_id());
                sp_device.profiler_error = Some("Failed to open device, extra data incomplete".to_string());
                sp_device.extra = Some(usb::USBDeviceExtra {
                    max_packet_size: device_info.max_packet_size(),
                    // nusb doesn't have these cached
                    string_indexes: (0, 0, 0),
                    driver: None,
                    syspath: Self::get_syspath(&sp_device.sysfs_name()),
                    // these should be read from the string descriptor we can't open device so just copy the cached ones in
                    vendor: sp_device.manufacturer.clone(),
                    product_name: Some(sp_device.name.clone()),
                    configurations: vec![],
                    status: None,
                    debug: None,
                    binary_object_store: None,
                    qualifier: None,
                    hub: None,
                });
            }

            Ok(sp_device)
        }
    }

    impl Profiler for NusbProfiler {
        fn profile_devices(
            &self,
            cache: &mut Vec<system_profiler::USBDevice>,
            root_hubs: &mut HashMap<u8, system_profiler::USBDevice>,
            with_extra: bool,
        ) -> Result<()> {
            for device in nusb::list_devices()? {
                match self.build_spdevice(&device, with_extra) {
                    Ok(sp_device) => {
                        cache.push(sp_device.to_owned());
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

                        // save it if it's a root_hub for assigning to bus data
                        if !cfg!(target_os = "macos") && sp_device.is_root_hub() {
                            root_hubs.insert(sp_device.location_id.bus, sp_device);
                        }
                    }
                    Err(e) => eprintln!("Failed to get data for {:?}: {}", device, e),
                }
            }

            Ok(())
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
}

/// Get [`system_profiler::SPUSBDataType`] using `libusb`. Does not source [`usb::USBDeviceExtra`] - use [`get_spusb_with_extra`] for that; the extra operation is mostly moving data around so the only hit is to stack.
///
/// Runs through `libusb::DeviceList` creating a cache of [`system_profiler::USBDevice`]. Then sorts into parent groups, accending in depth to build the [`system_profiler::USBBus`] tree.
///
/// Building the [`system_profiler::SPUSBDataType`] depends on system; on Linux, the root devices are at buses where as macOS the buses are not listed
#[cfg(all(feature = "libusb", not(feature = "nusb")))]
pub fn get_spusb() -> Result<system_profiler::SPUSBDataType> {
    let profiler = libusb::LibUsbProfiler;
    profiler.get_spusb(true)
}

/// Get [`system_profiler::SPUSBDataType`] using `nusb`. Does not source [`usb::USBDeviceExtra`] - use [`get_spusb_with_extra`] for that; the extra operation is mostly moving data around so the only hit is to stack.
///
/// Runs through `libusb::DeviceList` creating a cache of [`system_profiler::USBDevice`]. Then sorts into parent groups, accending in depth to build the [`system_profiler::USBBus`] tree.
///
/// Building the [`system_profiler::SPUSBDataType`] depends on system; on Linux, the root devices are at buses where as macOS the buses are not listed
#[cfg(feature = "nusb")]
pub fn get_spusb() -> Result<system_profiler::SPUSBDataType> {
    let profiler = nusb::NusbProfiler;
    profiler.get_spusb(true)
}

/// Abort with exit code before trying to call libusb feature if not present
#[cfg(all(not(feature = "libusb"), not(feature = "nusb")))]
pub fn get_spusb() -> Result<system_profiler::SPUSBDataType> {
    Err(crate::error::Error::new(
        crate::error::ErrorKind::Unsupported,
        "nusb or libusb feature is required to do this, install with `cargo install --features nusb/libusb`",
    ))
}

/// Get [`system_profiler::SPUSBDataType`] using `libusb` including [`usb::USBDeviceExtra`] - the main function to use for most use cases unless one does not want verbose data.
///
/// Like `get_spusb`, runs through `libusb::DeviceList` creating a cache of [`system_profiler::USBDevice`]. On Linux and with the 'udev' feature enabled, the syspath and driver will attempt to be obtained.
#[cfg(all(feature = "libusb", not(feature = "nusb")))]
pub fn get_spusb_with_extra() -> Result<system_profiler::SPUSBDataType> {
    let profiler = libusb::LibUsbProfiler;
    profiler.get_spusb(true)
}

/// Get [`system_profiler::SPUSBDataType`] using `nusb` including [`usb::USBDeviceExtra`] - the main function to use for most use cases unless one does not want verbose data.
///
/// Like `get_spusb`, runs through `libusb::DeviceList` creating a cache of [`system_profiler::USBDevice`]. On Linux and with the 'udev' feature enabled, the syspath and driver will attempt to be obtained.
#[cfg(feature = "nusb")]
pub fn get_spusb_with_extra() -> Result<system_profiler::SPUSBDataType> {
    let profiler = nusb::NusbProfiler;
    profiler.get_spusb(true)
}

/// Abort with exit code before trying to call libusb feature if not present
#[cfg(all(not(feature = "libusb"), not(feature = "nusb")))]
pub fn get_spusb_with_extra() -> Result<system_profiler::SPUSBDataType> {
    Err(crate::error::Error::new(
        crate::error::ErrorKind::Unsupported,
        "nusb or libusb feature is required to do this, install with `cargo install --features nusb/libusb`",
    ))
}
