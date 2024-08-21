//! System USB profiler for getting system USB information, devices and descriptors
//!
//! Get [`SPUSBDataType`] struct of system USB buses and devices with extra data like configs, interfaces and endpoints. The mod function will be based on the feature enabled, either `libusb` or `nusb`. To use a specific profiler, see the submodules [`libusb`], [`nusb`] and [`macos`].
//!
//! ```no_run
//! use cyme::profiler;
//!
//! let spusb = profiler::get_spusb_with_extra().unwrap();
//! // print with alternative styling (#) is using utf-8 icons
//! println!("{:#}", spusb);
//! ```
//!
//! See [`types`] docs for what can be done with returned data, such as [`USBFilter`]
use crate::error::Result;
use itertools::Itertools;
use std::collections::HashMap;

use crate::error::{Error, ErrorKind};
use crate::types::NumericalUnit;
#[cfg(all(target_os = "linux", any(feature = "udev", feature = "udevlib")))]
use crate::udev;
use crate::usb;

const REQUEST_GET_DESCRIPTOR: u8 = 0x06;
const REQUEST_GET_STATUS: u8 = 0x00;
const REQUEST_WEBUSB_URL: u8 = 0x02;

// separate module but import all
pub mod types;
pub use types::*;

#[cfg(feature = "libusb")]
pub mod libusb;
pub mod macos;
#[cfg(feature = "nusb")]
pub mod nusb;

/// Transfer direction
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum Direction {
    /// Host to device
    Out = 0,
    /// Device to host
    In = 1,
}

/// Specification defining the request.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum ControlType {
    /// Request defined by the USB standard.
    Standard = 0,
    /// Request defined by the standard USB class specification.
    Class = 1,
    /// Non-standard request.
    Vendor = 2,
}

/// Entity targeted by the request.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum Recipient {
    /// Request made to device as a whole.
    Device = 0,
    /// Request made to specific interface.
    Interface = 1,
    /// Request made to specific endpoint.
    Endpoint = 2,
    /// Other request.
    Other = 3,
}

/// Control request to USB device.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ControlRequest {
    pub control_type: ControlType,
    pub recipient: Recipient,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: usize,
}

/// Device USB operations required by the [`Profiler`]
pub(crate) trait UsbOperations {
    fn get_descriptor_string(&self, string_index: u8) -> Option<String>;
    fn get_control_msg(&self, control_request: &ControlRequest) -> Result<Vec<u8>>;
}

/// OS level USB Profiler trait for profiling USB devices
pub(crate) trait Profiler<T>
where
    T: UsbOperations,
    Self: std::fmt::Debug,
{
    /// Get the USB HID Report Descriptor with a Control request
    fn get_report_descriptor(device: &T, index: u16, length: u16) -> Result<Vec<u8>> {
        let control_request = ControlRequest {
            control_type: ControlType::Standard,
            recipient: Recipient::Interface,
            request: REQUEST_GET_DESCRIPTOR,
            value: (u8::from(usb::DescriptorType::Report) as u16) << 8,
            index,
            length: length as usize,
        };
        device.get_control_msg(&control_request)
    }

    /// Get the USB Hub Descriptor with a Control request, include hub port statuses
    fn get_hub_descriptor(
        device: &T,
        protocol: u8,
        bcd: u16,
        has_ssp: bool,
    ) -> Result<usb::HubDescriptor> {
        let is_ext_status = protocol == 3 && bcd >= 0x0310 && has_ssp;
        let value = if bcd >= 0x0300 {
            (u8::from(usb::DescriptorType::SuperSpeedHub) as u16) << 8
        } else {
            (u8::from(usb::DescriptorType::Hub) as u16) << 8
        };
        let control = ControlRequest {
            control_type: ControlType::Class,
            request: REQUEST_GET_DESCRIPTOR,
            value,
            index: 0,
            recipient: Recipient::Device,
            length: 9,
        };
        let data = device.get_control_msg(&control)?;
        let mut hub = usb::HubDescriptor::try_from(data.as_slice())?;

        // get port statuses
        let mut port_statues: Vec<[u8; 8]> = Vec::with_capacity(hub.num_ports as usize);
        for p in 0..hub.num_ports {
            let control = ControlRequest {
                control_type: ControlType::Class,
                request: REQUEST_GET_STATUS,
                index: p as u16 + 1,
                value: 0x23 << 8,
                recipient: Recipient::Other,
                length: if is_ext_status { 8 } else { 4 },
            };
            match device.get_control_msg(&control) {
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

    /// Get the USB Device status with a Control request
    fn get_device_status(device: &T) -> Result<u16> {
        let control = ControlRequest {
            control_type: ControlType::Standard,
            request: REQUEST_GET_STATUS,
            value: 0,
            index: 0,
            recipient: Recipient::Device,
            length: 2,
        };
        let data = device.get_control_msg(&control)?;
        Ok(u16::from_le_bytes([data[0], data[1]]))
    }

    /// Get the USB Debug Descriptor with a Control request
    fn get_debug_descriptor(device: &T) -> Result<usb::DebugDescriptor> {
        let control = ControlRequest {
            control_type: ControlType::Standard,
            request: REQUEST_GET_DESCRIPTOR,
            value: (u8::from(usb::DescriptorType::Debug) as u16) << 8,
            index: 0,
            recipient: Recipient::Device,
            length: 2,
        };
        let data = device.get_control_msg(&control)?;
        usb::DebugDescriptor::try_from(data.as_slice())
    }

    /// Get the USB Device Binary Object Store (BOS) Descriptor with a Control request
    fn get_bos_descriptor(
        device: &T,
    ) -> Result<usb::descriptors::bos::BinaryObjectStoreDescriptor> {
        let mut control = ControlRequest {
            control_type: ControlType::Standard,
            request: REQUEST_GET_DESCRIPTOR,
            value: (u8::from(usb::DescriptorType::Bos) as u16) << 8,
            index: 0,
            recipient: Recipient::Device,
            length: 5,
        };
        let data = device.get_control_msg(&control)?;
        let total_length = u16::from_le_bytes([data[2], data[3]]);
        log::trace!("Attempt read BOS descriptor total length: {}", total_length);
        // now get full descriptor
        control.length = total_length as usize;
        let data = device.get_control_msg(&control)?;
        log::trace!("BOS descriptor data: {:?}", data);
        let mut bos =
            usb::descriptors::bos::BinaryObjectStoreDescriptor::try_from(data.as_slice())?;

        // get any extra descriptor data now with handle
        for c in bos.capabilities.iter_mut() {
            match c {
                usb::descriptors::bos::BosCapability::WebUsbPlatform(w) => {
                    w.url = Self::get_webusb_url(device, w.vendor_code, w.landing_page_index).ok();
                    log::trace!("WebUSB URL: {:?}", w.url);
                }
                usb::descriptors::bos::BosCapability::Billboard(ref mut b) => {
                    b.additional_info_url =
                        device.get_descriptor_string(b.additional_info_url_index);
                    for a in b.alternate_modes.iter_mut() {
                        a.alternate_mode_string =
                            device.get_descriptor_string(a.alternate_mode_string_index);
                    }
                }
                _ => (),
            }
        }

        Ok(bos)
    }

    /// Get the USB Device Qualifier Descriptor with a Control request
    fn get_device_qualifier(device: &T) -> Result<usb::DeviceQualifierDescriptor> {
        let control = ControlRequest {
            control_type: ControlType::Standard,
            request: REQUEST_GET_DESCRIPTOR,
            value: 0x06 << 8,
            index: 0,
            recipient: Recipient::Device,
            length: 10,
        };
        let data = device.get_control_msg(&control)?;
        log::trace!("Device Qualifier descriptor data: {:?}", data);
        usb::DeviceQualifierDescriptor::try_from(data.as_slice())
    }

    /// Gets the WebUSB URL from the device, parsed and formatted as a URL
    ///
    /// https://github.com/gregkh/usbutils/blob/master/lsusb.c#L3261
    fn get_webusb_url(device: &T, vendor_request: u8, index: u8) -> Result<String> {
        let control = ControlRequest {
            control_type: ControlType::Vendor,
            request: vendor_request,
            value: index as u16,
            index: (REQUEST_WEBUSB_URL as u16) << 8,
            recipient: Recipient::Device,
            length: 3,
        };
        let data = device.get_control_msg(&control)?;
        log::trace!("WebUSB URL descriptor data: {:?}", data);
        let len = data[0] as usize;

        if data[1] != u8::from(usb::DescriptorType::String) {
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
    ///
    /// Fully described is based on the [`usb::ClassCodeTriplet`] and [`usb::Descriptor`] types. Any string indexes (or data which requires a control message) will be fetched and added to the descriptor while the device is still available.
    fn build_descriptor_extra<C: Into<usb::ClassCode> + Copy>(
        &self,
        device: &T,
        class_code: Option<usb::ClassCodeTriplet<C>>,
        interface_number: Option<u8>,
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
        if let Some(interface_desc) = class_code {
            if let Err(e) = dt.update_with_class_context(interface_desc) {
                log::debug!(
                    "Failed to update extra descriptor with class context: {}",
                    e
                );
            }
        }

        // get any strings at string indexes while we have handle
        match dt {
            usb::Descriptor::InterfaceAssociation(ref mut iad) => {
                iad.function_string = device.get_descriptor_string(iad.function_string_index);
            }
            usb::Descriptor::Device(ref mut c)
            | usb::Descriptor::Interface(ref mut c)
            | usb::Descriptor::Endpoint(ref mut c) => match c {
                usb::ClassDescriptor::Printer(ref mut p) => {
                    for pd in p.descriptors.iter_mut() {
                        pd.uuid_string = device.get_descriptor_string(pd.uuid_string_index);
                    }
                }
                usb::ClassDescriptor::Communication(ref mut cdc) => match cdc.interface {
                    usb::descriptors::cdc::CdcInterfaceDescriptor::CountrySelection(ref mut d) => {
                        d.country_code_date =
                            device.get_descriptor_string(d.country_code_date_index);
                    }
                    usb::descriptors::cdc::CdcInterfaceDescriptor::NetworkChannel(ref mut d) => {
                        d.name = device.get_descriptor_string(d.name_string_index);
                    }
                    usb::descriptors::cdc::CdcInterfaceDescriptor::EthernetNetworking(
                        ref mut d,
                    ) => {
                        d.mac_address = device.get_descriptor_string(d.mac_address_index);
                    }
                    usb::descriptors::cdc::CdcInterfaceDescriptor::CommandSet(ref mut d) => {
                        d.command_set_string =
                            device.get_descriptor_string(d.command_set_string_index);
                    }
                    _ => (),
                },
                // grab report descriptor data using usb_control_msg
                usb::ClassDescriptor::Hid(ref mut hd) => {
                    for rd in hd.descriptors.iter_mut() {
                        if let Some(index) = interface_number {
                            rd.data =
                                Self::get_report_descriptor(device, index as u16, rd.length).ok();
                        }
                    }
                }
                usb::ClassDescriptor::Midi(ref mut md, _) => match md.interface {
                    usb::descriptors::audio::MidiInterfaceDescriptor::InputJack(ref mut mh) => {
                        mh.jack_string = device.get_descriptor_string(mh.jack_string_index);
                    }
                    usb::descriptors::audio::MidiInterfaceDescriptor::OutputJack(ref mut mh) => {
                        mh.jack_string = device.get_descriptor_string(mh.jack_string_index);
                    }
                    usb::descriptors::audio::MidiInterfaceDescriptor::Element(ref mut mh) => {
                        mh.element_string = device.get_descriptor_string(mh.element_string_index);
                    }
                    _ => (),
                },
                usb::ClassDescriptor::Audio(ref mut ad, _) => match ad.interface {
                    usb::descriptors::audio::UacInterfaceDescriptor::InputTerminal1(ref mut ah) => {
                        ah.channel_names = device.get_descriptor_string(ah.channel_names_index);
                        ah.terminal = device.get_descriptor_string(ah.terminal_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::InputTerminal2(ref mut ah) => {
                        ah.channel_names = device.get_descriptor_string(ah.channel_names_index);
                        ah.terminal = device.get_descriptor_string(ah.terminal_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::OutputTerminal1(
                        ref mut ah,
                    ) => {
                        ah.terminal = device.get_descriptor_string(ah.terminal_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::OutputTerminal2(
                        ref mut ah,
                    ) => {
                        ah.terminal = device.get_descriptor_string(ah.terminal_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::StreamingInterface2(
                        ref mut ah,
                    ) => {
                        ah.channel_names = device.get_descriptor_string(ah.channel_names_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::SelectorUnit1(ref mut ah) => {
                        ah.selector = device.get_descriptor_string(ah.selector_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::SelectorUnit2(ref mut ah) => {
                        ah.selector = device.get_descriptor_string(ah.selector_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::ProcessingUnit1(
                        ref mut ah,
                    ) => {
                        ah.channel_names = device.get_descriptor_string(ah.channel_names_index);
                        ah.processing = device.get_descriptor_string(ah.processing_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::ProcessingUnit2(
                        ref mut ah,
                    ) => {
                        ah.channel_names = device.get_descriptor_string(ah.channel_names_index);
                        ah.processing = device.get_descriptor_string(ah.processing_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::EffectUnit2(ref mut ah) => {
                        ah.effect = device.get_descriptor_string(ah.effect_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::FeatureUnit1(ref mut ah) => {
                        ah.feature = device.get_descriptor_string(ah.feature_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::FeatureUnit2(ref mut ah) => {
                        ah.feature = device.get_descriptor_string(ah.feature_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::ExtensionUnit1(ref mut ah) => {
                        ah.channel_names = device.get_descriptor_string(ah.channel_names_index);
                        ah.extension = device.get_descriptor_string(ah.extension_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::ExtensionUnit2(ref mut ah) => {
                        ah.channel_names = device.get_descriptor_string(ah.channel_names_index);
                        ah.extension = device.get_descriptor_string(ah.extension_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::ClockSource2(ref mut ah) => {
                        ah.clock_source = device.get_descriptor_string(ah.clock_source_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::ClockSelector2(ref mut ah) => {
                        ah.clock_selector = device.get_descriptor_string(ah.clock_selector_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::ClockMultiplier2(
                        ref mut ah,
                    ) => {
                        ah.clock_multiplier =
                            device.get_descriptor_string(ah.clock_multiplier_index);
                    }
                    usb::descriptors::audio::UacInterfaceDescriptor::SampleRateConverter2(
                        ref mut ah,
                    ) => {
                        ah.src = device.get_descriptor_string(ah.src_index);
                    }
                    _ => (),
                },
                usb::ClassDescriptor::Video(ref mut vd, _) => match vd.interface {
                    usb::descriptors::video::UvcInterfaceDescriptor::InputTerminal(ref mut vh) => {
                        vh.terminal = device.get_descriptor_string(vh.terminal_index);
                    }
                    usb::descriptors::video::UvcInterfaceDescriptor::OutputTerminal(ref mut vh) => {
                        vh.terminal = device.get_descriptor_string(vh.terminal_index);
                    }
                    usb::descriptors::video::UvcInterfaceDescriptor::SelectorUnit(ref mut vh) => {
                        vh.selector = device.get_descriptor_string(vh.selector_index);
                    }
                    usb::descriptors::video::UvcInterfaceDescriptor::ProcessingUnit(ref mut vh) => {
                        vh.processing = device.get_descriptor_string(vh.processing_index);
                    }
                    usb::descriptors::video::UvcInterfaceDescriptor::ExtensionUnit(ref mut vh) => {
                        vh.extension = device.get_descriptor_string(vh.extension_index);
                    }
                    usb::descriptors::video::UvcInterfaceDescriptor::EncodingUnit(ref mut vh) => {
                        vh.encoding = device.get_descriptor_string(vh.encoding_index);
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }

        Ok(dt)
    }

    /// Build [`usb::Descriptor`]s from extra bytes of a Configuration Descriptor
    fn build_config_descriptor_extra(
        &self,
        device: &T,
        mut raw: Vec<u8>,
    ) -> Result<Vec<usb::Descriptor>> {
        let extra_len = raw.len();
        let mut taken = 0;
        let mut ret = Vec::new();

        // Iterate on chunks of the header length
        while taken < extra_len && extra_len >= 2 {
            let dt_len = raw[0] as usize;
            let dt = self.build_descriptor_extra::<u8>(
                device,
                None,
                None,
                &raw.drain(..dt_len).collect::<Vec<u8>>(),
            )?;
            log::trace!("Config descriptor extra: {:?}", dt);
            ret.push(dt);
            taken += dt_len;
        }

        Ok(ret)
    }

    /// Build [`usb::Descriptor`]s from extra bytes of an Interface Descriptor
    fn build_interface_descriptor_extra<C: Into<usb::ClassCode> + Copy>(
        &self,
        device: &T,
        class_code: usb::ClassCodeTriplet<C>,
        interface_number: u8,
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
                // if not Device or Interface, force it to Interface (like lsusb) but warn
                if !(*b == 0x01 || *b == 0x04) {
                    log::warn!("Misplaced descriptor type in interfaces: {:02x}", *b);
                    *b = 0x04;
                }
            }

            let dt = self.build_descriptor_extra(
                device,
                Some(class_code),
                Some(interface_number),
                &raw.drain(..dt_len).collect::<Vec<u8>>(),
            )?;

            log::trace!("Interface descriptor extra: {:?}", dt);
            ret.push(dt);
            taken += dt_len;
        }

        Ok(ret)
    }

    /// Build [`usb::Descriptor`]s from extra bytes of an Endpoint Descriptor
    fn build_endpoint_descriptor_extra<C: Into<usb::ClassCode> + Copy>(
        &self,
        device: &T,
        class_code: usb::ClassCodeTriplet<C>,
        interface_number: u8,
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
                Some(class_code),
                Some(interface_number),
                &raw.drain(..dt_len).collect::<Vec<u8>>(),
            )?;

            log::trace!("Endpoint descriptor extra: {:?}", dt);
            ret.push(dt);
            taken += dt_len;
        }

        Ok(Some(ret))
    }

    /// Get [`USBDevice`]s connected to the host, excluding root hubs
    fn get_devices(&self, with_extra: bool) -> Result<Vec<USBDevice>>;

    /// Get root hubs connected to the host as [`USBDevice`]s
    ///
    /// Root Hub devices are not always listed in the device list, so this is a separate function to get them. They only exist on Linux and are used to assign info to [`USBBus`]s.
    fn get_root_hubs(&self) -> Result<HashMap<u8, USBDevice>>;

    /// Build the [`SPUSBDataType`] from the Profiler get_devices and get_root_hubs (for buses) functions
    fn get_spusb(&self, with_extra: bool) -> Result<SPUSBDataType> {
        let mut spusb = SPUSBDataType { buses: Vec::new() };

        log::info!("Building SPUSBDataType with {:?}", self);

        // temporary store of devices created when iterating through DeviceList
        let mut cache = self.get_devices(with_extra)?;
        cache.sort_by_key(|d| d.location_id.bus);
        log::trace!("Sorted devices {:#?}", cache);
        // lookup for root hubs to assign info to bus on linux
        let mut root_hubs = self.get_root_hubs()?;
        log::trace!("Root hubs: {:#?}", root_hubs);

        // group by bus number and then stick them into a bus in the returned SPUSBDataType
        for (key, group) in &cache.into_iter().group_by(|d| d.location_id.bus) {
            // create the bus, we'll add devices at next step
            // if root hub exists, add it to the bus and remove so we can add empty buses if missing after
            let mut new_bus = if let Some(root_hub) = root_hubs.remove(&key) {
                let mut bus = USBBus {
                    // TODO lookup from pci.ids crate
                    name: root_hub.name.clone(),
                    host_controller: root_hub.manufacturer.clone().unwrap_or_default(),
                    usb_bus_number: Some(key),
                    // TODO root hub VID and PID is not PCI VID and PID
                    pci_vendor: root_hub.vendor_id,
                    pci_device: root_hub.product_id,
                    ..Default::default()
                };
                // add root hub to devices like lsusb on Linux since they are like devices
                if cfg!(target_os = "linux") {
                    bus.devices = Some(vec![root_hub])
                }

                bus
            } else {
                USBBus {
                    name: "Unknown".into(),
                    host_controller: "Unknown".into(),
                    usb_bus_number: Some(key),
                    ..Default::default()
                }
            };

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

        // add empty root_hubs if missing
        if !root_hubs.is_empty() {
            for (key, root_hub) in root_hubs {
                let mut bus = USBBus {
                    name: root_hub.name.clone(),
                    host_controller: root_hub.manufacturer.clone().unwrap_or_default(),
                    usb_bus_number: Some(key),
                    pci_vendor: root_hub.vendor_id,
                    pci_device: root_hub.product_id,
                    ..Default::default()
                };
                if cfg!(target_os = "linux") {
                    bus.devices = Some(vec![root_hub])
                }
                spusb.buses.push(bus);
            }
            spusb.buses.sort_by_key(|b| b.usb_bus_number);
        }

        Ok(spusb)
    }

    /// Fills a passed mutable `spusb` reference to fill using `get_spusb`. Will replace existing [`USBDevice`]s found in the Profiler tree but leave others and the buses.
    ///
    /// The main use case for this is to merge with macOS `system_profiler` data, so that [`usb::USBDeviceExtra`] can be obtained but internal buses kept. One could also use it to update a static .json dump.
    fn fill_spusb(&self, spusb: &mut SPUSBDataType) -> Result<()> {
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

/// Get device information from sysfs path on Linux
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

/// Get the driver name from udev on Linux if the feature is enabled
#[allow(unused_variables)]
fn get_udev_driver_name(port_path: &str) -> Result<Option<String>> {
    #[cfg(all(target_os = "linux", any(feature = "udev", feature = "udevlib")))]
    return udev::get_udev_driver_name(port_path);
    #[cfg(not(all(target_os = "linux", any(feature = "udev", feature = "udevlib"))))]
    return Ok(None);
}

/// Get the syspath from udev on Linux if the feature is enabled
#[allow(unused_variables)]
fn get_udev_syspath(port_path: &str) -> Result<Option<String>> {
    #[cfg(all(target_os = "linux", any(feature = "udev", feature = "udevlib")))]
    return udev::get_udev_syspath(port_path);
    #[cfg(not(all(target_os = "linux", any(feature = "udev", feature = "udevlib"))))]
    return Ok(None);
}

/// Get the syspath based on the default location "/sys/bus/usb/devices" on Linux
#[allow(unused_variables)]
fn get_syspath(port_path: &str) -> Option<String> {
    #[cfg(target_os = "linux")]
    return Some(format!("/sys/bus/usb/devices/{}", port_path));
    #[cfg(not(target_os = "linux"))]
    return None;
}

/// Build [`SPUSBDataType`] by profiling system. Does not source [`usb::USBDeviceExtra`] - use [`get_spusb_with_extra`] for that; the extra operation is mostly moving data around so the only hit is to stack.
///
/// Runs through `libusb::DeviceList` creating a cache of [`USBDevice`]. Then sorts into parent groups, accending in depth to build the [`USBBus`] tree.
///
/// Building the [`SPUSBDataType`] depends on system; on Linux, the root devices are at buses where as macOS the buses are not listed
pub fn get_spusb() -> Result<SPUSBDataType> {
    #[cfg(all(feature = "libusb", not(feature = "nusb")))]
    {
        let profiler = libusb::LibUsbProfiler;
        <libusb::LibUsbProfiler as Profiler<libusb::UsbDevice<rusb::Context>>>::get_spusb(
            &profiler, false,
        )
    }
    #[cfg(feature = "nusb")]
    {
        let profiler = nusb::NusbProfiler;
        profiler.get_spusb(true)
    }

    #[cfg(all(not(feature = "libusb"), not(feature = "nusb")))]
    {
        Err(crate::error::Error::new(
            crate::error::ErrorKind::Unsupported,
            "nusb or libusb feature is required to do this, install with `cargo install --features nusb/libusb`",
        ))
    }
}

/// Build [`SPUSBDataType`] including [`usb::USBDeviceExtra`] - the main function to use for most use cases unless one does not want verbose data. The extra data requires opening the device to read device descriptors.
///
/// Like `get_spusb`, runs through `libusb::DeviceList` creating a cache of [`USBDevice`]. On Linux and with the 'udev' feature enabled, the syspath and driver will attempt to be obtained.
pub fn get_spusb_with_extra() -> Result<SPUSBDataType> {
    #[cfg(all(feature = "libusb", not(feature = "nusb")))]
    {
        let profiler = libusb::LibUsbProfiler;
        <libusb::LibUsbProfiler as Profiler<libusb::UsbDevice<rusb::Context>>>::get_spusb(
            &profiler, true,
        )
    }

    #[cfg(feature = "nusb")]
    {
        let profiler = nusb::NusbProfiler;
        profiler.get_spusb(true)
    }

    #[cfg(all(not(feature = "libusb"), not(feature = "nusb")))]
    {
        Err(crate::error::Error::new(
            crate::error::ErrorKind::Unsupported,
            "nusb or libusb feature is required to do this, install with `cargo install --features nusb/libusb`",
        ))
    }
}
