//! Originally based on [libusb list_devices.rs example](https://github.com/dcuddeback/libusb-rs/blob/master/examples/list_devices.rs), attempts to mimic lsusb functions and provide cross-platform SPUSBDataType gather
//! lsusb uses udev for tree building, which libusb does not have access to and is Linux only. udev-rs is used on Linux to attempt to mirror the output of lsusb on Linux. On other platforms, certain information like driver used cannot be obtained.
/* Ref for list:
Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 004 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
Bus 003 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 002 Device 003: ID 1d50:6018 OpenMoko, Inc. Black Magic Debug Probe (Application)
Bus 002 Device 002: ID 203a:fffe PARALLELS Virtual USB1.1 HUB
Bus 002 Device 001: ID 1d6b:0001 Linux Foundation 1.1 root hub
* Ref for Tree:
/:  Bus 04.Port 1: Dev 1, Class=root_hub, Driver=xhci_hcd/12p, 10000M
/:  Bus 03.Port 1: Dev 1, Class=root_hub, Driver=xhci_hcd/2p, 480M
/:  Bus 02.Port 1: Dev 1, Class=root_hub, Driver=uhci_hcd/2p, 12M
   |__ Port 2: Dev 2, If 0, Class=Hub, Driver=hub/15p, 12M
       |__ Port 4: Dev 3, If 0, Class=Communications, Driver=cdc_acm, 12M
       |__ Port 4: Dev 3, If 1, Class=CDC Data, Driver=cdc_acm, 12M
       |__ Port 4: Dev 3, If 2, Class=Communications, Driver=cdc_acm, 12M
       |__ Port 4: Dev 3, If 3, Class=CDC Data, Driver=cdc_acm, 12M
       |__ Port 4: Dev 3, If 4, Class=Application Specific Interface, Driver=, 12M
       |__ Port 4: Dev 3, If 5, Class=Vendor Specific Class, Driver=, 12M
/:  Bus 01.Port 1: Dev 1, Class=root_hub, Driver=ehci-pci/15p, 480M
   |__ Port 2: Dev 2, If 0, Class=Human Interface Device, Driver=usbhid, 480M
   |__ Port 2: Dev 2, If 1, Class=Human Interface Device, Driver=usbhid, 480M
   |__ Port 6: Dev 3, If 0, Class=Printer, Driver=usblp, 480M
*/
use std::time::Duration;
use std::collections::HashSet;
use std::collections::HashMap;
use itertools::Itertools;
use rusb as libusb;
use usb_ids::{self, FromId};
use crate::{usb, system_profiler, types::NumericalUnit};
#[cfg(target_os = "linux")]
#[cfg(feature = "udev")]
use crate::udev;

struct UsbDevice<T: libusb::UsbContext> {
    handle: libusb::DeviceHandle<T>,
    language: libusb::Language,
    timeout: Duration,
}

pub fn set_log_level(debug: u8) -> () {
    let log_level = match debug {
        0 => rusb::LogLevel::None,
        1 => rusb::LogLevel::Info,
        2 | _ => rusb::LogLevel::Debug,
    };

    rusb::set_log_level(log_level);
}

fn build_endpoints(
    interface_desc: &libusb::InterfaceDescriptor,
) -> libusb::Result<Vec<usb::USBEndpoint>> {
    let mut ret: Vec<usb::USBEndpoint> = Vec::new();

    for endpoint_desc in interface_desc.endpoint_descriptors() {
        ret.push(usb::USBEndpoint {
            address: usb::EndpointAddress {
                address: endpoint_desc.address(),
                number: endpoint_desc.number(),
                direction: usb::Direction::from(endpoint_desc.direction())
            },
            transfer_type: usb::TransferType::from(endpoint_desc.transfer_type()),
            sync_type: usb::SyncType::from(endpoint_desc.sync_type()),
            usage_type: usb::UsageType::from(endpoint_desc.usage_type()),
            max_packet_size: endpoint_desc.max_packet_size(),
            interval: endpoint_desc.interval(),
        });
    }

    Ok(ret)
}

fn build_interfaces<T: libusb::UsbContext>(
    device: &libusb::Device<T>,
    handle: &mut Option<UsbDevice<T>>,
    config_desc: &libusb::ConfigDescriptor,
) -> libusb::Result<Vec<usb::USBInterface>> {
    let mut ret: Vec<usb::USBInterface> = Vec::new();

    for interface in config_desc.interfaces() {
        for interface_desc in interface.descriptors() {
            let mut _interface = usb::USBInterface {
                name: get_interface_string(&interface_desc, handle),
                number: interface_desc.interface_number(),
                path: usb::get_interface_path(device.bus_number(), &device.port_numbers()?, config_desc.number(), interface_desc.interface_number()),
                class: usb::ClassCode::from(interface_desc.class_code()),
                sub_class: interface_desc.sub_class_code(),
                protocol: interface_desc.protocol_code(),
                alt_setting: interface_desc.setting_number(),
                driver: None,
                endpoints: build_endpoints(&interface_desc)?
            };

            #[cfg(target_os = "linux")]
            #[cfg(feature = "udev")]
            udev::get_driver(&mut _interface.driver, &_interface.path).or(Err(libusb::Error::Other))?;

            ret.push(_interface);
        }
    }

    Ok(ret)
}

fn build_configurations<T: libusb::UsbContext>(
    device: &libusb::Device<T>,
    handle: &mut Option<UsbDevice<T>>,
    device_desc: &libusb::DeviceDescriptor,
) -> libusb::Result<Vec<usb::USBConfiguration>> {
    let mut ret: Vec<usb::USBConfiguration> = Vec::new();

    for n in 0..device_desc.num_configurations() {
        let config_desc = match device.config_descriptor(n) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut attributes = HashSet::new();
        if config_desc.remote_wakeup() {
            attributes.insert(usb::ConfigAttributes::RemoteWakeup);
        }

        if config_desc.self_powered() {
            attributes.insert(usb::ConfigAttributes::SelfPowered);
        }

        ret.push(usb::USBConfiguration {
            name: get_configuration_string(&config_desc, handle),
            number: config_desc.number(),
            attributes,
            max_power: NumericalUnit{value: config_desc.max_power() as u32, unit: String::from("mW"), description: None},
            interfaces: build_interfaces(device, handle, &config_desc)?,
        });
    }

    Ok(ret)
}


fn build_spdevice_extra<T: libusb::UsbContext>(
    device: &libusb::Device<T>,
    handle: &mut Option<UsbDevice<T>>,
    device_desc: &libusb::DeviceDescriptor,
    _sp_device: &system_profiler::USBDevice,
) -> libusb::Result<usb::USBDeviceExtra> {
    let mut _extra = usb::USBDeviceExtra {
        max_packet_size: device_desc.max_packet_size(),
        driver: None,
        configurations: build_configurations(device, handle, device_desc)?
    };

    #[cfg(target_os = "linux")]
    #[cfg(feature = "udev")]
    udev::get_driver(&mut _extra.driver, &_sp_device.port_path()).or(Err(libusb::Error::Other))?;

    Ok(_extra)
}

/// Builds a `system_profiler::USBDevice` from a `libusb::Device` by using `device_descriptor()` and intrograting for configuration strings
fn build_spdevice<T: libusb::UsbContext>(
    device: &libusb::Device<T>,
    with_extra: bool,
) -> libusb::Result<system_profiler::USBDevice> {
    let timeout = Duration::from_secs(1);
    let speed = match usb::Speed::from(device.speed()) {
        usb::Speed::Unknown => None,
        v => Some(system_profiler::DeviceSpeed::SpeedValue(v)),
    };

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
                Err(_) => None,
            },
            Err(_) => None,
        }
    };

    // lookup manufacturer and device name from Linux list if empty
    let mut manufacturer = get_manufacturer_string(&device_desc, &mut usb_device);
    let mut name = get_product_string(&device_desc, &mut usb_device);
    if manufacturer.is_empty() {
        match usb_ids::Vendor::from_id(device_desc.vendor_id()) {
            Some(vendor) => manufacturer = vendor.name().to_owned(),
            None => (),
        };
    }

    if name.is_empty() {
        match usb_ids::Device::from_vid_pid(device_desc.vendor_id(), device_desc.product_id()) {
            Some(product) => name = product.name().to_owned(),
            None => (),
        };
    }

    let mut sp_device = system_profiler::USBDevice {
        name,
        manufacturer: Some(manufacturer),
        serial_num: Some(get_serial_string(&device_desc, &mut usb_device)),
        vendor_id: Some(device_desc.vendor_id()),
        product_id: Some(device_desc.product_id()),
        device_speed: speed,
        location_id: system_profiler::DeviceLocation {
            bus: device.bus_number(),
            number: device.address(),
            tree_positions: device.port_numbers()?,
            ..Default::default()
        },
        bcd_device: version_to_float(&device_desc.device_version()),
        bcd_usb: version_to_float(&device_desc.usb_version()),
        class: Some(usb::ClassCode::from(device_desc.class_code())),
        sub_class: Some(device_desc.sub_class_code()),
        protocol: Some(device_desc.protocol_code()),
        ..Default::default()
    };

    if with_extra { 
        sp_device.extra = Some(build_spdevice_extra(device, &mut usb_device, &device_desc, &sp_device)?);
    }

    Ok(sp_device)
}

/// Get `SPUSBDataType` using `libusb` rather than `system_profiler`
///
/// Runs through `libusb::DeviceList` creating a cache of `USBDevice`. Then sorts into parent groups, accending in depth to build the `SPUSBDataType` tree.
///
/// Building the `SPUSBDataType` depends on system; on Linux, the root devices are at buses where as macOS the buses are not listed
pub fn get_spusb(with_extra: bool) -> libusb::Result<system_profiler::SPUSBDataType> {
    let mut sp_data = system_profiler::SPUSBDataType { buses: Vec::new() };
    // temporary store of devices created when iterating through DeviceList
    let mut cache: Vec<system_profiler::USBDevice> = Vec::new();
    // lookup for root hubs to assign info to bus on linux
    let mut root_hubs: HashMap<u8, system_profiler::USBDevice> = HashMap::new();

    // run through devices building USBDevice types
    for device in libusb::DeviceList::new()?.iter() {
        match build_spdevice(&device, with_extra) {
            Ok(sp_device) => {
                cache.push(sp_device.to_owned());

                if !cfg!(target_os = "macos") {
                    if sp_device.is_root_hub() {
                        root_hubs.insert(sp_device.location_id.bus, sp_device);
                    }
                }
            },
            Err(e) => { eprintln!("Failed to get data for {:?}: {}", device, e.to_string()) }
        }
    }

    // ensure sort of bus so that grouping is not broken up
    cache.sort_by_key(|d| d.location_id.bus);
    log::debug!("Sorted devices {:?}", cache);

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
            new_bus.host_controller = root_hub.manufacturer.as_ref().unwrap_or(&String::new()).to_owned();
            new_bus.pci_vendor = root_hub.vendor_id;
            new_bus.pci_device = root_hub.product_id;
        }

        // group into parent groups with parent path as key or trunk devices so they end up in same place
        let parent_groups = group.group_by(|d| d.parent_path().unwrap_or(d.trunk_path()));

        // now go through parent paths inserting devices owned by that parent
        // this is not perfect...if the sort of devices does not result in order of depth, it will panic because the parent of a device will not exist. But that won't happen, right...
        // sort key - ends_with to ensure root_hubs, which will have same str length as trunk devices will still be ahead
        for (parent_path, children) in parent_groups.into_iter().sorted_by_key(|x| x.0.len() - x.0.ends_with("-0") as usize) {
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
                log::debug!("Updated bus devices {:?}", new_bus.devices);
            // else find and add parent - this should work because we are sorted to accend the tree so parents should be created before their children
            } else {
                let parent_node = new_bus.get_node_mut(&parent_path).expect("Parent node does not exist in new bus!");
                let devices = std::mem::take(&mut parent_node.devices);
                if let Some(mut d) = devices {
                    for new_device in children {
                        d.push(new_device);
                    }
                    parent_node.devices = Some(d);
                } else {
                    parent_node.devices = Some(children.collect());
                }
                log::debug!("Updated parent devices {:?}", parent_node.devices);
            }
        }

        sp_data.buses.push(new_bus);
    }

    Ok(sp_data)
}

pub fn lsusb_verbose(filter: &Option<system_profiler::USBFilter>) -> libusb::Result<()> {
    let timeout = Duration::from_secs(1);

    for device in libusb::DeviceList::new()?.iter() {
        let device_desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };

        if let Some(f) = filter {
            if let Some(fvid) = f.vid {
                if device_desc.vendor_id() != fvid {
                    continue;
                }
            }

            if let Some(fpid) = f.pid {
                if device_desc.product_id() != fpid {
                    continue;
                }
            }
        }

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
                    Err(_) => None,
                },
                Err(_) => None,
            }
        };

        // now we have device, filter on bus number and address
        if let Some(f) = filter {
            if let Some(bus_number) = f.bus {
                if bus_number != device.bus_number() {
                    continue;
                }
            }

            if let Some(address) = f.number {
                if address != device.address() {
                    continue;
                }
            }

            if let Some(name) = &f.name {
                if !get_product_string(&device_desc, &mut usb_device).contains(name) {
                    continue;
                }
            }

            if let Some(serial) = &f.serial {
                if !get_serial_string(&device_desc, &mut usb_device).contains(serial) {
                    continue;
                }
            }
        }

        println!(""); // new lines separate in verbose lsusb
        println!(
            "Bus {:03} Device {:03}: ID {:04x}:{:04x} {} {}",
            device.bus_number(),
            device.address(),
            device_desc.vendor_id(),
            device_desc.product_id(),
            get_manufacturer_string(&device_desc, &mut usb_device),
            get_product_string(&device_desc, &mut usb_device)
        );
        print_device(&device_desc, &mut usb_device);

        for n in 0..device_desc.num_configurations() {
            let config_desc = match device.config_descriptor(n) {
                Ok(c) => c,
                Err(_) => continue,
            };

            print_config(&config_desc, &mut usb_device);

            for interface in config_desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    print_interface(&interface_desc, &mut usb_device);

                    for endpoint_desc in interface_desc.endpoint_descriptors() {
                        print_endpoint(&endpoint_desc);
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_product_string<T: libusb::UsbContext>(
    device_desc: &libusb::DeviceDescriptor,
    handle: &mut Option<UsbDevice<T>>,
) -> String {
    handle.as_mut().map_or(String::new(), |h| {
        h.handle
            .read_product_string(h.language, device_desc, h.timeout)
            .unwrap_or(String::new())
            .trim()
            .trim_end_matches('\0')
            .to_string()
    })
}

fn get_manufacturer_string<T: libusb::UsbContext>(
    device_desc: &libusb::DeviceDescriptor,
    handle: &mut Option<UsbDevice<T>>,
) -> String {
    handle.as_mut().map_or(String::new(), |h| {
        h.handle
            .read_manufacturer_string(h.language, device_desc, h.timeout)
            .unwrap_or(String::new())
            .trim()
            .trim_end_matches('\0')
            .to_string()
    })
}

fn get_serial_string<T: libusb::UsbContext>(
    device_desc: &libusb::DeviceDescriptor,
    handle: &mut Option<UsbDevice<T>>,
) -> String {
    handle.as_mut().map_or(String::new(), |h| {
        h.handle
            .read_serial_number_string(h.language, device_desc, h.timeout)
            .unwrap_or(String::new())
            .trim()
            .trim_end_matches('\0')
            .to_string()
    })
}

fn get_configuration_string<T: libusb::UsbContext>(
    config_desc: &libusb::ConfigDescriptor,
    handle: &mut Option<UsbDevice<T>>,
) -> String {
    handle.as_mut().map_or(String::new(), |h| {
        h.handle
            .read_configuration_string(h.language, config_desc, h.timeout)
            .unwrap_or(String::new())
            .trim()
            .trim_end_matches('\0')
            .to_string()
    })
}

fn get_interface_string<T: libusb::UsbContext>(
    interface_desc: &libusb::InterfaceDescriptor,
    handle: &mut Option<UsbDevice<T>>,
) -> String {
    handle.as_mut().map_or(String::new(), |h| {
        h.handle
            .read_interface_string(h.language, interface_desc, h.timeout)
            .unwrap_or(String::new())
            .trim()
            .trim_end_matches('\0')
            .to_string()
    })
}

/// Convert libusb version to f32
///
/// Would be nicer to impl fmt::Display and From<f32> but cannot outside of crate
fn version_to_float(version: &libusb::Version) -> Option<f32> {
    if let Ok(v) = format!(
        "{}.{}{}",
        version.major(),
        version.minor(),
        version.sub_minor()
    )
    .parse::<f32>()
    {
        Some(v)
    } else {
        None
    }
}

fn print_device<T: libusb::UsbContext>(
    device_desc: &libusb::DeviceDescriptor,
    handle: &mut Option<UsbDevice<T>>,
) {
    let vendor_name = match usb_ids::Vendor::from_id(device_desc.vendor_id()) {
        Some(vendor) => vendor.name(),
        None => "Unknown vendor",
    };

    let product_name =
        match usb_ids::Device::from_vid_pid(device_desc.vendor_id(), device_desc.product_id()) {
            Some(product) => product.name(),
            None => "Unknown product",
    };

    println!("Device Descriptor:");
    println!(
        "  bcdUSB             {:2}.{}{}",
        device_desc.usb_version().major(),
        device_desc.usb_version().minor(),
        device_desc.usb_version().sub_minor()
    );
    println!(
        "  bDeviceClass        {:#04x} {:?}",
        device_desc.class_code(),
        usb::ClassCode::from(device_desc.class_code())
    );
    println!(
        "  bDeviceSubClass     {:#04x}",
        device_desc.sub_class_code()
    );
    println!("  bDeviceProtocol     {:#04x}", device_desc.protocol_code());
    println!("  bMaxPacketSize0      {:3}", device_desc.max_packet_size());
    println!("  idVendor          {:#06x} {}", device_desc.vendor_id(), vendor_name);
    println!("  idProduct         {:#06x} {}", device_desc.product_id(), product_name);
    println!(
        "  bcdDevice          {:2}.{}{}",
        device_desc.device_version().major(),
        device_desc.device_version().minor(),
        device_desc.device_version().sub_minor()
    );
    println!(
        "  iManufacturer        {:3} {}",
        device_desc.manufacturer_string_index().unwrap_or(0),
        handle.as_mut().map_or(String::new(), |h| h
            .handle
            .read_manufacturer_string(h.language, device_desc, h.timeout)
            .unwrap_or_default())
    );
    println!(
        "  iProduct             {:3} {}",
        device_desc.product_string_index().unwrap_or(0),
        handle.as_mut().map_or(String::new(), |h| h
            .handle
            .read_product_string(h.language, device_desc, h.timeout)
            .unwrap_or_default())
    );
    println!(
        "  iSerialNumber        {:3} {}",
        device_desc.serial_number_string_index().unwrap_or(0),
        handle.as_mut().map_or(String::new(), |h| h
            .handle
            .read_serial_number_string(h.language, device_desc, h.timeout)
            .unwrap_or_default())
    );
    println!(
        "  bNumConfigurations   {:3}",
        device_desc.num_configurations()
    );
}

fn print_config<T: libusb::UsbContext>(
    config_desc: &libusb::ConfigDescriptor,
    handle: &mut Option<UsbDevice<T>>,
) {
    println!("  Config Descriptor:");
    println!(
        "    bNumInterfaces       {:3}",
        config_desc.num_interfaces()
    );
    println!("    bConfigurationValue  {:3}", config_desc.number());
    println!(
        "    iConfiguration       {:3} {}",
        config_desc.description_string_index().unwrap_or(0),
        handle.as_mut().map_or(String::new(), |h| h
            .handle
            .read_configuration_string(h.language, config_desc, h.timeout)
            .unwrap_or_default())
    );
    println!("    bmAttributes:");
    println!("      Self Powered     {:>5}", config_desc.self_powered());
    println!("      Remote Wakeup    {:>5}", config_desc.remote_wakeup());
    println!("    bMaxPower           {:4}mW", config_desc.max_power());
}

fn print_interface<T: libusb::UsbContext>(
    interface_desc: &libusb::InterfaceDescriptor,
    handle: &mut Option<UsbDevice<T>>,
) {
    println!("    Interface Descriptor:");
    println!(
        "      bInterfaceNumber     {:3}",
        interface_desc.interface_number()
    );
    println!(
        "      bAlternateSetting    {:3}",
        interface_desc.setting_number()
    );
    println!(
        "      bNumEndpoints        {:3}",
        interface_desc.num_endpoints()
    );
    println!(
        "      bInterfaceClass     {:#04x} {:?}",
        interface_desc.class_code(),
        usb::ClassCode::from(interface_desc.class_code())
    );
    println!(
        "      bInterfaceSubClass  {:#04x}",
        interface_desc.sub_class_code()
    );
    println!(
        "      bInterfaceProtocol  {:#04x}",
        interface_desc.protocol_code()
    );
    println!(
        "      iInterface           {:3} {}",
        interface_desc.description_string_index().unwrap_or(0),
        handle.as_mut().map_or(String::new(), |h| h
            .handle
            .read_interface_string(h.language, interface_desc, h.timeout)
            .unwrap_or_default())
    );
}

fn print_endpoint(endpoint_desc: &libusb::EndpointDescriptor) {
    println!("      Endpoint Descriptor:");
    println!(
        "        bEndpointAddress    {:#04x} EP {} {:?}",
        endpoint_desc.address(),
        endpoint_desc.number(),
        endpoint_desc.direction()
    );
    println!("        bmAttributes:");
    println!(
        "          Transfer Type          {:?}",
        endpoint_desc.transfer_type()
    );
    println!(
        "          Synch Type             {:?}",
        endpoint_desc.sync_type()
    );
    println!(
        "          Usage Type             {:?}",
        endpoint_desc.usage_type()
    );
    println!(
        "        wMaxPacketSize    {:#06x}",
        endpoint_desc.max_packet_size()
    );
    println!(
        "        bInterval            {:3}",
        endpoint_desc.interval()
    );
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
            libusb::SyncType::NoSync => usb::SyncType::NoSync,
            libusb::SyncType::Asynchronous => usb::SyncType::Asynchronous,
            libusb::SyncType::Adaptive => usb::SyncType::Adaptive,
            libusb::SyncType::Synchronous => usb::SyncType::Synchronous,
        }
    }
}
