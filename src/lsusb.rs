///! Based on [libusb list_devices.rs example](https://github.com/dcuddeback/libusb-rs/blob/master/examples/list_devices.rs) provides some functionaility as lsusb verbose
use std::time::Duration;
use itertools::Itertools;
use crate::{system_profiler::{self, DeviceLocation}, usb};

struct UsbDevice<'a> {
    handle: libusb::DeviceHandle<'a>,
    language: libusb::Language,
    timeout: Duration,
}

pub fn get_spusb(filter: &Option<system_profiler::USBFilter>) -> libusb::Result<system_profiler::SPUSBDataType> {
    let timeout = Duration::from_secs(1);
    let context = libusb::Context::new()?;
    let mut sp_data = system_profiler::SPUSBDataType {
        buses: Vec::new()
    };
    let mut sp_devices: Vec<system_profiler::USBDevice> = Vec::new();

    for device in context.devices()?.iter() {
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
                    continue
                }
            }

            if let Some(address) = f.port {
                if address != device.address() {
                    continue
                }
            }
        }

        let speed = match usb::Speed::from(device.speed()) {
            usb::Speed::Unknown => None,
            v => Some(system_profiler::DeviceSpeed::SpeedValue(v))
        };

        sp_devices.push(system_profiler::USBDevice {
            name: get_product_string(&device_desc, &mut usb_device),
            manufacturer: Some(get_manufacturer_string(&device_desc, &mut usb_device)),
            serial_num: Some(get_serial_string(&device_desc, &mut usb_device)),
            vendor_id: Some(device_desc.vendor_id()),
            product_id: Some(device_desc.product_id()),
            device_speed: speed,
            location_id: DeviceLocation {
                bus: device.bus_number(),
                number: Some(device.address()),
                ..Default::default()
            },
            bcd_device: version_to_float(&device_desc.device_version()),
            bcd_usb: version_to_float(&device_desc.usb_version()),
            ..Default::default()
        });
    }

    // group by bus number and then stick them into a bus in the returned SPUSBDataType
    // TODO put within hubs to build tree?
    for (key, group) in &sp_devices.into_iter().group_by(|d| d.location_id.bus) {
        let new_bus = system_profiler::USBBus { name: "".into(), host_controller: "".into(), usb_bus_number: Some(key), devices: Some(group.collect()), ..Default::default() };
        sp_data.buses.push(new_bus);
    }

    Ok(sp_data)
}

pub fn lsusb_verbose(filter: &Option<system_profiler::USBFilter>) -> libusb::Result<()> {
    let timeout = Duration::from_secs(1);
    let context = libusb::Context::new()?;

    for device in context.devices()?.iter() {
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
                    continue
                }
            }

            if let Some(address) = f.port {
                if address != device.address() {
                    continue
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

fn get_product_string(
    device_desc: &libusb::DeviceDescriptor,
    handle: &mut Option<UsbDevice>,
) -> String {
    handle.as_mut().map_or(String::new(), |h| {
        h.handle
            .read_product_string(h.language, device_desc, h.timeout)
            .unwrap_or(String::new())
            .trim().trim_matches('\0').to_string()
    })
}

fn get_manufacturer_string(
    device_desc: &libusb::DeviceDescriptor,
    handle: &mut Option<UsbDevice>,
) -> String {
    handle.as_mut().map_or(String::new(), |h| {
        h.handle
            .read_manufacturer_string(h.language, device_desc, h.timeout)
            .unwrap_or(String::new())
            .trim().trim_matches('\0').to_string()
    })
}

fn get_serial_string(
    device_desc: &libusb::DeviceDescriptor,
    handle: &mut Option<UsbDevice>,
) -> String {
    handle.as_mut().map_or(String::new(), |h| {
        h.handle
            .read_serial_number_string(h.language, device_desc, h.timeout)
            .unwrap_or(String::new())
            .trim().trim_matches('\0').to_string()
    })
}

/// Convert libusb version to f32
///
/// Would be nicer to impl fmt::Display and From<f32> but cannot outside of crate
fn version_to_float(version: &libusb::Version) -> Option<f32> {
    if let Ok(v) = format!("{}.{}{}", version.major(), version.minor(), version.sub_minor()).parse::<f32>() {
        Some(v)
    } else {
        None
    }
}

fn print_device(device_desc: &libusb::DeviceDescriptor, handle: &mut Option<UsbDevice>) {
    println!("Device Descriptor:");
    println!(
        "  bcdUSB             {:2}.{}{}",
        device_desc.usb_version().major(),
        device_desc.usb_version().minor(),
        device_desc.usb_version().sub_minor()
    );
    println!("  bDeviceClass        {:#04x}", device_desc.class_code());
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
            .unwrap_or(String::new()))
    );
    println!(
        "  iProduct             {:3} {}",
        device_desc.product_string_index().unwrap_or(0),
        handle.as_mut().map_or(String::new(), |h| h
            .handle
            .read_product_string(h.language, device_desc, h.timeout)
            .unwrap_or(String::new()))
    );
    println!(
        "  iSerialNumber        {:3} {}",
        device_desc.serial_number_string_index().unwrap_or(0),
        handle.as_mut().map_or(String::new(), |h| h
            .handle
            .read_serial_number_string(h.language, device_desc, h.timeout)
            .unwrap_or(String::new()))
    );
    println!(
        "  bNumConfigurations   {:3}",
        device_desc.num_configurations()
    );
}

fn print_config(config_desc: &libusb::ConfigDescriptor, handle: &mut Option<UsbDevice>) {
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
            .unwrap_or(String::new()))
    );
    println!("    bmAttributes:");
    println!("      Self Powered     {:>5}", config_desc.self_powered());
    println!("      Remote Wakeup    {:>5}", config_desc.remote_wakeup());
    println!("    bMaxPower           {:4}mW", config_desc.max_power());
}

fn print_interface(interface_desc: &libusb::InterfaceDescriptor, handle: &mut Option<UsbDevice>) {
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
        "      bInterfaceClass     {:#04x}",
        interface_desc.class_code()
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
            .unwrap_or(String::new()))
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
            libusb::Speed::Super => usb::Speed::SuperSpeed,
            libusb::Speed::High => usb::Speed::HighSpeed,
            libusb::Speed::Full => usb::Speed::FullSpeed,
            libusb::Speed::Low => usb::Speed::LowSpeed,
            libusb::Speed::Unknown => usb::Speed::Unknown,
        }
    }
}
