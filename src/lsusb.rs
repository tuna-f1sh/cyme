///! Originally based on [libusb list_devices.rs example](https://github.com/dcuddeback/libusb-rs/blob/master/examples/list_devices.rs), attempts to mimic lsusb functions and provide cross-platform SPUSBDataType gather
///
///! lsusb uses udev for tree building, which libusb does not have access to and is Linux only. For this reason, this module cannot do everything lsusb does but tries to cover most things useful for listing USB device information.
///! TODO: add udev-rs as linux dependency and get device driver from path
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
use itertools::Itertools;
use rusb as libusb;
use crate::{usb, system_profiler};

struct UsbDevice<T: libusb::UsbContext> {
    handle: libusb::DeviceHandle<T>,
    language: libusb::Language,
    timeout: Duration,
}

/// Builds a `system_profiler::USBDevice` from a `libusb::Device` by using `device_descriptor()` and intrograting for configuration strings
fn build_spdevice<T: libusb::UsbContext>(
    device: &libusb::Device<T>,
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

    Ok(system_profiler::USBDevice {
        name: get_product_string(&device_desc, &mut usb_device),
        manufacturer: Some(get_manufacturer_string(&device_desc, &mut usb_device)),
        serial_num: Some(get_serial_string(&device_desc, &mut usb_device)),
        vendor_id: Some(device_desc.vendor_id()),
        product_id: Some(device_desc.product_id()),
        device_speed: speed,
        location_id: system_profiler::DeviceLocation {
            bus: device.bus_number(),
            number: Some(device.address()),
            tree_positions: device.port_numbers()?,
            ..Default::default()
        },
        bcd_device: version_to_float(&device_desc.device_version()),
        bcd_usb: version_to_float(&device_desc.usb_version()),
        class: Some(usb::ClassCode::from(device_desc.class_code())),
        ..Default::default()
    })
}

/// Get `SPUSBDataType` using `libusb` rather than `system_profiler`
///
/// Runs through `libusb::DeviceList` creating a cache of `USBDevice`. Then sorts into parent groups, accending in depth to build the `SPUSBDataType` tree.
pub fn get_spusb() -> libusb::Result<system_profiler::SPUSBDataType> {
    let mut sp_data = system_profiler::SPUSBDataType { buses: Vec::new() };
    // temporary store of devices created when iterating through DeviceList
    let mut cache: Vec<system_profiler::USBDevice> = Vec::new();

    // run through devices building USBDevice types
    for device in libusb::DeviceList::new()?.iter() {
        let sp_device = build_spdevice(&device)?;
        cache.push(sp_device);
    }

    // ensure sort of bus so that grouping is not broken up
    cache.sort_by_key(|d| d.location_id.bus);
    log::debug!("Sorted devices {:?}", cache);

    // group by bus number and then stick them into a bus in the returned SPUSBDataType
    for (key, group) in &cache.into_iter().group_by(|d| d.location_id.bus) {
        // create the bus, we'll add devices at next step
        let mut new_bus = system_profiler::USBBus {
            name: "Unknown".into(),
            host_controller: "Unknown".into(),
            usb_bus_number: Some(key),
            ..Default::default()
        };

        // group into parent groups with parent path as key - "-" for root devices so they end up in same place
        let parent_groups = group.group_by(|d| d.parent_path().unwrap_or("-".into()));

        // now go through parent paths inserting devices owned by that parent
        for (parent_path, children) in parent_groups.into_iter().sorted_by_key(|x| x.0.len()) {
            log::debug!("Adding devices to parent {}", parent_path);
            // if root devices, add them to push
            if parent_path == "-" {
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
    println!("  idVendor          {:#06x}", device_desc.vendor_id());
    println!("  idProduct         {:#06x}", device_desc.product_id());
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
