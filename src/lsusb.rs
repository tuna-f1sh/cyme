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
use std::mem;
use std::cell::RefCell;
use std::time::Duration;
use itertools::Itertools;
use rusb as libusb;
use std::collections::{HashMap, HashSet};
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

/// This is a bit intensive and hacky but creates a device tree like system_profiler by recursively checking the `get_parent()` function of `Device` d
///
/// The idea is that the `DeviceList` can return the devices in any order and this function will check whether the device has a parent. If it does, it will check if we've already created it (exists in HashMap of port path). If it does it exist, appends the passed `d` as a `system_profiler::USBDevice`. If it does not, then recursively call the function using that parent device - the one returned by `get_parent()` for the passed `d`.
///
/// The recursion ensures that even when creating a parent, the parent (if it exists) for that will also be created so even if the first `Device` is 7 devices deep, it should still build correctly.
fn check_add_parent<T: libusb::UsbContext>(
    d: &libusb::Device<T>,
    mut root_devices: &mut HashMap<String, system_profiler::USBDevice>,
    mut parent_cache: &mut HashMap<String, RefCell<system_profiler::USBDevice>>,
    mut explored: &mut HashSet<String>,
    is_parent: bool,
) -> libusb::Result<Option<system_profiler::USBDevice>> {
    log::debug!("Check add device {:?}", d);

    if explored.contains(&usb::get_port_path(d.bus_number(), &d.port_numbers()?)) {
        log::debug!("Already exists, skipping");
        return Ok(None);
    }

    // if the device has a parent, try to find it in hashmap and put it in that device's devices
    if let Some(parent) = d.get_parent() {
        log::debug!("Has parent {:?}", parent);
        let new_device = match parent_cache.get(&usb::get_port_path(
            parent.bus_number(),
            &parent.port_numbers()?,
        )) {
            Some(sp_cell_parent) => {
                let sp_device = build_spdevice(d)?;
                // get the parent from cell
                let mut sp_parent = sp_cell_parent.borrow_mut();
                log::debug!("Parent on root exists {:?}", sp_parent);
                // take the current vec so we can make or extend
                let devices = mem::take(&mut sp_parent.devices);
                if let Some(mut devs) = devices {
                    devs.push(sp_device.to_owned());
                    sp_parent.devices = Some(devs);
                } else {
                    sp_parent.devices = Some(vec![sp_device.to_owned()]);
                }
                log::debug!("Updated parent {:}", sp_parent);
                // path_cache.insert(sp_device.location_id.port_path(), RefCell::new(sp_device));
                explored.insert(usb::get_port_path(sp_device.location_id.bus, &sp_device.location_id.tree_positions));
                if is_parent {
                    Some(sp_device)
                } else {
                    None
                }
                // Ok(Some(sp_device))
            }
            // no parent: make the parent now, needs to be recursively in case parent has parents...
            None => {
                log::debug!("Parent not in HashMap, will create");
                check_add_parent(&parent, &mut root_devices, &mut parent_cache, &mut explored, true)?;
                let cell = parent_cache.get_mut(&usb::get_port_path(parent.bus_number(), &parent.port_numbers()?)).expect("Parent was not created!");
                let mut sp_parent = cell.borrow_mut();
                let sp_device = build_spdevice(d)?;
                sp_parent.devices = Some(vec![sp_device.to_owned()]);
                log::debug!("New parent {:}", sp_parent);
                explored.insert(usb::get_port_path(sp_device.location_id.bus, &sp_device.location_id.tree_positions));
                if is_parent {
                    Some(sp_device)
                } else {
                    None
                }
            }
        };

        if let Some(add) = new_device {
            parent_cache.insert(add.location_id.port_path(), RefCell::new(add));
        }
        Ok(None)
    } else {
        let sp_device = build_spdevice(d)?;
        log::debug!("Created {:}", sp_device);

        // it must be depth 1
        if sp_device.location_id.tree_positions.len() != 1 {
            panic!("Found device without parent that has depth > 1 {:?}", d);
        }

        // insert to root devices map
        explored.insert(usb::get_port_path(sp_device.location_id.bus, &sp_device.location_id.tree_positions));
        parent_cache.insert(sp_device.location_id.port_path(), RefCell::new(sp_device));
        Ok(None)
    }
}

pub fn get_spusb() -> libusb::Result<system_profiler::SPUSBDataType> {
    let mut sp_data = system_profiler::SPUSBDataType { buses: Vec::new() };
    // Temporary store of devices used by `check_add_parent`
    // Uses port path as Hash key since that _should_ be unique for each device and easy to build
    let mut sp_devices: HashMap<String, system_profiler::USBDevice> = HashMap::new();
    let mut cache: HashMap<String, RefCell<system_profiler::USBDevice>> = HashMap::new();
    let mut explored: HashSet<String> = HashSet::new();

    // run through devices building tree of root devices
    for device in libusb::DeviceList::new()?.iter() {
        check_add_parent(&device, &mut sp_devices, &mut cache, &mut explored, false)?;
    }

    log::debug!("{:?}", sp_devices);
    log::debug!("{:?}", cache);
    log::debug!("{:?}", explored);

    // need to build by port path
    // let device_list1: Vec<system_profiler::USBDevice> = sp_devices.to_owned().into_values().collect();
    // for (key, group) in &device_list1.into_iter().group_by(|d| get_port_path(d.location_id.bus, &d.location_id.tree_positions[..d.location_id.tree_positions.len() - 1].to_vec())) {
    //     let vec: Vec<system_profiler::USBDevice> = group.collect();
    //     log::debug!("{:?}: {:?}", key, vec);
    // }

    // group by bus number and then stick them into a bus in the returned SPUSBDataType
    let device_list: Vec<system_profiler::USBDevice> = cache.into_values().map(|v| v.into_inner()).collect();
    for (key, group) in &device_list.into_iter().group_by(|d| d.location_id.bus) {
        // for (k, g) in &group
        //     .group_by(|d| d.location_id.tree_positions[..d.location_id.tree_positions.len()].to_vec()) {
        //     let vec: Vec<system_profiler::USBDevice> = g.collect();
        //     log::debug!("{:?}: {:?}", k, vec);
        //     // .filter(|d| d.location_id.tree_positions.len() > 1)
        //     // .into_iter()
        // };

        let new_bus = system_profiler::USBBus {
            name: "Unknown".into(),
            host_controller: "Unknown".into(),
            usb_bus_number: Some(key),
            // filter only devices at the bus depth
            devices: Some(group.collect()),
            // devices: Some(group.filter(|d| d.location_id.tree_positions.len() == 1).collect()),
            ..Default::default()
        };
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
