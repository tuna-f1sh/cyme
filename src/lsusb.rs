//! Methods to print system USB information in lsusb style
//!
//! Originally based on [libusb list_devices.rs example](https://github.com/dcuddeback/libusb-rs/blob/master/examples/list_devices.rs), attempts to mimic lsusb output. The [lsusb source code](https://github.com/gregkh/usbutils/blob/master/lsusb.c) was used as a reference for the styling and content; even odities/inconsistencies were kept!
use crate::display::PrintSettings;
use crate::error::{Error, ErrorKind};
use crate::profiler::{Device, SystemProfile};
use std::path::Path;
use uuid::Uuid;

use crate::usb::descriptors::audio;
use crate::usb::descriptors::cdc;
use crate::usb::descriptors::video;
use crate::usb::descriptors::*;
use crate::usb::*;

mod audio_dumps;
mod bos_dumps;
pub mod names;
mod video_dumps;

use audio_dumps::*;
use bos_dumps::*;
use video_dumps::*;

const TREE_LSUSB_BUS: &str = "/:  ";
const TREE_LSUSB_DEVICE: &str = "|__ ";
const TREE_LSUSB_SPACE: &str = "    ";

const LSUSB_DUMP_WIDTH: usize = 25;
const LSUSB_DUMP_INDENT_BASE: usize = 2;

fn get_spaces(value_len: usize, field_len: usize, width: usize) -> String {
    if value_len >= width || value_len == usize::MAX {
        String::from(" ")
    } else {
        " ".repeat((width - value_len).saturating_sub(field_len).max(1))
    }
}

/// Dump an array of value like lsusb
fn dump_array<T: std::fmt::Display>(array: &[T], field_name: &str, indent: usize, width: usize) {
    for (i, b) in array.iter().enumerate() {
        dump_value(b, &format!("{field_name}({i:2})"), indent, width);
    }
}

/// Dump a bitmap value mapping as hex like lsusb
fn dump_bitmap_array<T: std::fmt::LowerHex + Into<u64> + Copy>(
    array: &[T],
    field_name: &str,
    indent: usize,
    width: usize,
) {
    for (i, b) in array.iter().enumerate() {
        dump_hex(*b, &format!("{field_name}({i:2})"), indent, width);
    }
}

/// Dump just indented string
fn dump_string(field_name: &str, indent: usize) {
    println!("{:indent$}{}", "", field_name);
}

/// Dump a single value like lsusb
fn dump_value<T: std::fmt::Display>(value: T, field_name: &str, indent: usize, width: usize) {
    let value = value.to_string();
    let spaces = get_spaces(value.len(), field_name.len(), width);
    println!("{:indent$}{}{}{}", "", field_name, spaces, value);
}

/// Dump a single hex value like lsusb
fn dump_hex<T: std::fmt::LowerHex + Into<u64>>(
    value: T,
    field_name: &str,
    indent: usize,
    width: usize,
) {
    let value_as_u64: u64 = value.into();
    let hex_value = format!(
        "0x{:0width$x}",
        value_as_u64,
        width = (std::mem::size_of::<T>() * 2)
    );
    dump_value(hex_value, field_name, indent, width);
}

/// Dump a single hex value in upper case, as lsusb does for a handful of fields
fn dump_hex_upper<T: std::fmt::UpperHex + Into<u64>>(
    value: T,
    field_name: &str,
    indent: usize,
    width: usize,
) {
    let value_as_u64: u64 = value.into();
    let hex_value = format!(
        "0x{:0width$X}",
        value_as_u64,
        width = (std::mem::size_of::<T>() * 2)
    );
    dump_value(hex_value, field_name, indent, width);
}

/// Lookup the name of the value from passed function and dump it
fn dump_name<T: std::fmt::Display>(
    value: T,
    names_f: fn(T) -> Option<String>,
    field_name: &str,
    indent: usize,
    width: usize,
) {
    let value_string = value.to_string();
    let spaces = get_spaces(value_string.len(), field_name.len(), width);
    let dump = format!("{:indent$}{}{}{}", "", field_name, spaces, value_string,);
    if let Some(name) = names_f(value) {
        println!("{dump} {name}");
    }
}

/// Lookup the name of the value from passed function and dump it as hex
fn dump_name_hex<T: std::fmt::LowerHex>(
    value: T,
    names_f: fn(T) -> Option<String>,
    field_name: &str,
    indent: usize,
    width: usize,
) {
    let value_string = format!("{:#04x}", value);
    let spaces = get_spaces(value_string.len(), field_name.len(), width);
    let dump = format!("{:indent$}{}{}{}", "", field_name, spaces, value_string,);
    if let Some(name) = names_f(value) {
        println!("{dump} {name}");
    }
}

/// Dumps the value and the string representation of the value to the right of width
fn dump_value_string<T: std::fmt::Display, S: std::fmt::Display>(
    value: T,
    field_name: &str,
    value_string: S,
    indent: usize,
    width: usize,
) {
    dump_value_string_gap(value, field_name, value_string, indent, width, 1);
}

/// As [`dump_value_string`] but with `gap` spaces between the value and its
/// string, since lsusb uses two for the hex formatted fields
fn dump_value_string_gap<T: std::fmt::Display, S: std::fmt::Display>(
    value: T,
    field_name: &str,
    value_string: S,
    indent: usize,
    width: usize,
    gap: usize,
) {
    let value = value.to_string();
    let spaces = get_spaces(value.len(), field_name.len(), width);
    println!(
        "{:indent$}{}{}{}{:gap$}{}",
        "", field_name, spaces, value, "", value_string,
    );
}

/// Dumps a value left aligned at the column the value field would start at
///
/// Used where lsusb pads the field name out with literal spaces rather than
/// right aligning what follows it
fn dump_string_right<T: std::fmt::Display>(
    value: T,
    field_name: &str,
    indent: usize,
    width: usize,
) {
    // 1 to account for space
    let spaces = get_spaces(1, field_name.len(), width);
    println!("{:indent$}{}{}{}", "", field_name, spaces, value);
}

/// Dumps GUID enclosed in braces like lsusb
fn dump_guid(guid: &Uuid, field_name: &str, indent: usize, width: usize) {
    dump_string_right(guid.braced().to_string(), field_name, indent, width);
}

/// Dumps junk descriptor bytes as hex like lsusb
fn dump_junk(extra: &[u8], indent: usize, reported_len: usize, expected_len: usize) {
    if reported_len > expected_len && extra.len() >= reported_len {
        println!(
            "{:^indent$}junk at descriptor end: {}",
            "",
            extra[expected_len..reported_len]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}

/// Dumps unknown descriptor bytes as hex like lsusb
fn dump_unrecognised(extra: &[u8], indent: usize) {
    println!(
        "{:indent$}** UNRECOGNIZED: {}",
        "",
        extra
            .iter()
            .map(|b| format!(" {b:02x}"))
            .collect::<String>()
    )
}

/// Dumps strings matching the bits set in `bitmap` using `strings_f` function from LSB to MSB
fn dump_bitmap_strings<T>(bitmap: T, strings_f: fn(usize) -> Option<&'static str>, indent: usize)
where
    T: std::fmt::Display + std::fmt::LowerHex + Copy + Into<u64>,
{
    let bitmap_u64: u64 = bitmap.into();
    let num_bits = std::mem::size_of::<T>() * 8;
    for index in 0..num_bits {
        if (bitmap_u64 >> index) & 0x1 != 0 {
            if let Some(string) = strings_f(index) {
                println!("{:indent$}{}", "", string);
            }
        }
    }
}

/// Dumps strings matching the bits set in `bitmap` using `strings_f` function from MSB to LSB
fn dump_bitmap_strings_invert<T>(
    bitmap: T,
    strings_f: fn(usize) -> Option<&'static str>,
    indent: usize,
) where
    T: std::fmt::Display + std::fmt::LowerHex + Copy + Into<u64>,
{
    let bitmap_u64: u64 = bitmap.into();
    let num_bits = std::mem::size_of::<T>() * 8;
    for index in (0..num_bits).rev() {
        if (bitmap_u64 >> index) & 0x1 != 0 {
            if let Some(string) = strings_f(index) {
                println!("{:indent$}{}", "", string);
            }
        }
    }
}

/// Dump a value followed, on the same line, by the strings for the bits set in it
///
/// lsusb separates the value from the first string by two spaces. `trailing`
/// adds a space after the last one, which lsusb does for the fields whose
/// strings carry their own separator rather than being prefixed with one.
fn dump_bitmap_strings_inline<T, V>(
    value: V,
    bitmap: T,
    field_name: &str,
    strings_f: fn(usize) -> Option<&'static str>,
    indent: usize,
    width: usize,
    trailing: bool,
) where
    T: std::fmt::Display + std::fmt::LowerHex + Copy + Into<u64>,
    V: std::fmt::Display,
{
    let value = value.to_string();
    let spaces = get_spaces(value.len(), field_name.len(), width);
    print!("{:indent$}{}{}{} ", "", field_name, spaces, value,);
    let bitmap_u64: u64 = bitmap.into();
    let num_bits = std::mem::size_of::<T>() * 8;
    for index in 0..num_bits {
        if (bitmap_u64 >> index) & 0x1 != 0 {
            if let Some(string) = strings_f(index) {
                print!(" {string}");
            }
        }
    }
    if trailing {
        print!(" ");
    }
    println!();
}

fn get_guid(buf: &[u8]) -> String {
    if buf.len() < 16 {
        return String::from("INVALID GUID");
    }

    format!("{{{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}", 
        buf[3], buf[2], buf[1], buf[0],
        buf[5], buf[4],
        buf[7], buf[6],
        buf[8], buf[9],
        buf[10], buf[11], buf[12], buf[13], buf[14], buf[15])
}

/// Print [`SystemProfile`] as a lsusb style tree with the two optional `verbosity` levels
pub fn print_tree(spusb: &SystemProfile, settings: &PrintSettings) {
    fn print_tree_devices(devices: &Vec<Device>, settings: &PrintSettings) {
        for device in devices {
            if device.is_root_hub() {
                log::debug!("lsusb tree skipping root_hub {device}");
                continue;
            }
            // the const len should get compiled to const...
            let indent = (device.get_depth() * TREE_LSUSB_DEVICE.len()) + TREE_LSUSB_SPACE.len();
            let device_tree_strings = device.to_lsusb_tree_strings(settings.verbosity);

            for strings in device_tree_strings {
                println!("{:>indent$}{}", TREE_LSUSB_DEVICE, strings[0]);
                for s in &strings[1..] {
                    println!("{TREE_LSUSB_SPACE:>indent$}{s}");
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
        let bus_tree_strings = bus.to_lsusb_tree_string(settings.verbosity);
        println!("{}{}", TREE_LSUSB_BUS, bus_tree_strings[0]);
        for strings in &bus_tree_strings[1..] {
            println!("{TREE_LSUSB_SPACE}{strings}");
        }

        // followed by devices if there are some
        bus.devices
            .as_ref()
            .map_or((), |d| print_tree_devices(d, settings))
    }
}

/// Dump a single [`Device`] matching `dev_path` verbosely
pub fn dump_one_device<P: AsRef<Path>>(devices: &Vec<&Device>, dev_path: P) -> Result<(), Error> {
    for device in devices {
        if device.dev_path() == dev_path.as_ref() {
            // error if extra is none because we need it for vebose
            if device.extra.is_none() {
                return Err(Error::new(
                    ErrorKind::Opening,
                    &format!("Unable to open {}", dev_path.as_ref().display()),
                ));
            }

            print(&vec![device], true);
            return Ok(());
        }
    }

    Err(Error::new(
        ErrorKind::NotFound,
        &format!("Unable to find {}", dev_path.as_ref().display()),
    ))
}

/// Find the OTG descriptor belonging to a [`Configuration`]
///
/// Mirrors lsusb's `do_otg`, which looks in the configuration's own extra
/// descriptors first, then those of each interface, then each endpoint. Purely
/// a search - the caller dumps it, once, so that a device with an OTG
/// descriptor does not print it per interface and endpoint.
fn find_otg(config: &Configuration) -> Option<&OnTheGoDescriptor> {
    fn in_extra(extra: Option<&Vec<Descriptor>>) -> Option<&OnTheGoDescriptor> {
        extra?.iter().find_map(|d| match d {
            Descriptor::Otg(otg) => {
                log::debug!("Found OTG descriptor: {otg:?}");
                Some(otg)
            }
            _ => None,
        })
    }

    in_extra(config.extra.as_ref()).or_else(|| {
        config.interfaces.iter().find_map(|interface| {
            in_extra(interface.extra.as_ref()).or_else(|| {
                interface
                    .endpoints
                    .iter()
                    .find_map(|endpoint| in_extra(endpoint.extra.as_ref()))
            })
        })
    })
}

/// Print USB devices in lsusb style flat dump
///
/// `verbose` flag enables verbose printing like lsusb (configs, interfaces and endpoints) - a huge dump!
pub fn print(devices: &Vec<&Device>, verbose: bool) {
    if !verbose {
        for device in devices {
            println!("{}", device.to_lsusb_string());
        }
    } else {
        for device in devices {
            println!(); // new lines separate in verbose lsusb
            println!("{}", device.to_lsusb_string());
            // print error regarding open if non-critical during probe like lsusb --verbose
            if device.profiler_error.is_some() {
                eprintln!("Couldn't open device, some information will be missing");
            }
            match device.extra.as_ref() {
                None => log::warn!(
                    "Device {device} does not contain extra data required for verbose print"
                ),
                Some(device_extra) => {
                    dump_device(device);

                    // lsusb do_otg: configuration 0 only, dumped once at the top
                    // level before any configuration descriptor
                    let otg = device_extra.configurations.first().and_then(find_otg);
                    if let Some(otg) = otg {
                        dump_otg(otg, 0);
                    }

                    for config in &device_extra.configurations {
                        dump_config(config, LSUSB_DUMP_INDENT_BASE);

                        for interface in &config.interfaces {
                            dump_interface(interface, LSUSB_DUMP_INDENT_BASE * 2);

                            for endpoint in &interface.endpoints {
                                dump_endpoint(endpoint, LSUSB_DUMP_INDENT_BASE * 3);
                            }
                        }
                    }

                    let has_ssp = if let Some(bos) = &device_extra.binary_object_store {
                        let lpm_required: bool =
                            device.bcd_usb.is_some_and(|v| u16::from(v) >= 0x0210);
                        dump_bos_descriptor(bos, lpm_required, 0);
                        bos.capabilities
                            .iter()
                            .any(|c| matches!(c, bos::BosCapability::SuperSpeedPlus(_)))
                    } else {
                        false
                    };
                    if let Some(hub) = &device_extra.hub {
                        let bcd = device.bcd_usb.map_or(0x0100, |v| v.into());
                        dump_hub(hub, device.protocol.unwrap_or(1), bcd, has_ssp, 0);
                    }
                    // lsusb do_dualspeed: only called for devices reporting
                    // exactly USB 2.00, since only those have a qualifier
                    if device.bcd_usb.is_some_and(|v| u16::from(v) == 0x0200) {
                        if let Some(qualifier) = &device_extra.qualifier {
                            dump_device_qualifier(qualifier, 0);
                        }
                    }
                    if let Some(debug) = &device_extra.debug {
                        dump_debug(debug, 0);
                    }

                    if let Some(status) = device_extra.status {
                        dump_device_status(
                            status,
                            otg.is_some(),
                            device.bcd_usb.is_some_and(|v| v.major() >= 3),
                            0,
                        );
                    }
                }
            }
        }
    }
}

/// Dump a [`Device`] in style of lsusb --verbose
fn dump_device(device: &Device) {
    let device_extra = device
        .extra
        .as_ref()
        .expect("Cannot print verbose without extra data");

    let (class_name, sub_class_name, protocol_name) =
        match (device.base_class_code(), device.sub_class, device.protocol) {
            (Some(bc), Some(scid), Some(pid)) => (
                names::class(bc),
                names::subclass(bc, scid),
                names::protocol(bc, scid, pid),
            ),
            (Some(bc), Some(scid), None) => (names::class(bc), names::subclass(bc, scid), None),
            (Some(bc), None, None) => (names::class(bc), None, None),
            (None, None, None) => (None, None, None),
            _ => unreachable!(),
        };

    // print negotiated speed
    let speed_str = match device_extra.negotiated_speed {
        Some(Speed::LowSpeed) => "Low Speed (1Mbps)",
        Some(Speed::FullSpeed) => "Full Speed (12Mbps)",
        Some(Speed::HighSpeed) | Some(Speed::HighBandwidth) => "High Speed (480Mbps)",
        Some(Speed::SuperSpeed) => "SuperSpeed (5Gbps)",
        Some(Speed::SuperSpeedPlus) => "SuperSpeed+ (10Gbps)",
        Some(Speed::SuperSpeedPlusX2) => "SuperSpeed++ (20Gbps)",
        _ => "Unknown",
    };
    println!("Negotiated speed: {speed_str}");

    println!("Device Descriptor:");
    // These are constants - length is 18 bytes for descriptor, type is 1
    dump_value(18, "bLength", 2, LSUSB_DUMP_WIDTH);
    dump_value(1, "bDescriptorType", 2, LSUSB_DUMP_WIDTH);
    dump_value(
        device
            .bcd_usb
            .as_ref()
            .map_or(String::new(), |v| v.to_string()),
        "bcdUSB",
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value_string(
        device.base_class_code().unwrap_or(0),
        "bDeviceClass",
        class_name.unwrap_or(String::from("[unknown]")),
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value_string(
        device.sub_class.unwrap_or(0),
        "bDeviceSubClass",
        sub_class_name.unwrap_or(String::from("[unknown]")),
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value_string(
        device.protocol.unwrap_or(0),
        "bDeviceProtocol",
        protocol_name.unwrap_or_default(),
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value(
        device_extra.max_packet_size,
        "bMaxPacketSize0",
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value_string(
        format!("0x{:04x}", device.vendor_id.unwrap_or(0)),
        "idVendor",
        device_extra.vendor.as_ref().unwrap_or(
            device
                .manufacturer
                .as_ref()
                .unwrap_or(&String::from("[unknown]")),
        ),
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value_string(
        format!("0x{:04x}", device.product_id.unwrap_or(0)),
        "idProduct",
        device_extra.product_name.as_ref().unwrap_or(&device.name),
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value(
        device
            .bcd_device
            .as_ref()
            .map_or(String::new(), |v| v.to_string()),
        "bcdDevice",
        2,
        LSUSB_DUMP_WIDTH,
    );

    // lsusb prints the sysfs manufacturer/product/serial properties, which are
    // populated from the string descriptors, so a zero index means no string
    // rather than a name looked up from the vendor database
    let string_or_empty = |index: u8, s: Option<&String>| -> String {
        if index == 0 {
            String::new()
        } else {
            s.cloned().unwrap_or_default()
        }
    };

    dump_value_string(
        device_extra.string_indexes.1,
        "iManufacturer",
        string_or_empty(device_extra.string_indexes.1, device.manufacturer.as_ref()),
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value_string(
        device_extra.string_indexes.0,
        "iProduct",
        string_or_empty(device_extra.string_indexes.0, Some(&device.name)),
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value_string(
        device_extra.string_indexes.2,
        "iSerial",
        string_or_empty(device_extra.string_indexes.2, device.serial_num.as_ref()),
        2,
        LSUSB_DUMP_WIDTH,
    );

    dump_value(
        device_extra.configurations.len(),
        "bNumConfigurations",
        2,
        LSUSB_DUMP_WIDTH,
    );
}

/// Dump a [`Configuration`] in style of lsusb --verbose
fn dump_config(config: &Configuration, indent: usize) {
    dump_string("Configuration Descriptor:", indent);
    dump_value(config.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(2, "bDescriptorType", indent + 2, LSUSB_DUMP_WIDTH); // type 2 for configuration
    dump_hex(
        config.total_length,
        "wTotalLength",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        config.number_of_interfaces(),
        "bNumInterfaces",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        config.number,
        "bConfigurationValue",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        config.string_index,
        "iConfiguration",
        &config.name,
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(
        config.attributes_value(),
        "bmAttributes",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    // bit 7 is reserved and must be set
    if config.attributes_value() & 0x80 == 0 {
        dump_string("(Missing must-be-set bit!)", indent + 4);
    }
    // no attributes is bus powered
    if config.attributes.contains(&ConfigAttributes::SelfPowered) {
        dump_string("Self Powered", indent + 4);
    }
    if config.attributes.contains(&ConfigAttributes::BusPowered) {
        dump_string("(Bus Powered)", indent + 4);
    }
    if config.attributes.contains(&ConfigAttributes::RemoteWakeup) {
        dump_string("Remote Wakeup", indent + 4);
    }
    if config
        .attributes
        .contains(&ConfigAttributes::BatteryPowered)
    {
        dump_string("Battery Powered", indent + 4);
    }
    dump_value(
        format!("{}{}", config.max_power.value, config.max_power.unit),
        "MaxPower",
        indent + 2,
        LSUSB_DUMP_WIDTH + 2,
    );

    // dump extra descriptors
    if let Some(dt_vec) = &config.extra {
        for dt in dt_vec {
            match dt {
                Descriptor::InterfaceAssociation(iad) => {
                    dump_interface_association(iad, indent + 2);
                }
                Descriptor::Security(sec) => {
                    dump_security(sec, indent + 2);
                }
                Descriptor::Encrypted(enc) => {
                    dump_encryption_type(enc, indent + 2);
                }
                Descriptor::Unknown(junk) | Descriptor::Junk(junk) => {
                    dump_unrecognised(junk, indent + 2);
                }
                _ => (),
            }
        }
    }
}

/// Dump a [`InterfaceAssociation`] in style of lsusb --verbose
fn dump_interface(interface: &Interface, indent: usize) {
    let interface_name = names::class(interface.class.into());
    let sub_class_name = names::subclass(interface.class.into(), interface.sub_class);
    let protocol_name = names::protocol(
        interface.class.into(),
        interface.sub_class,
        interface.protocol,
    );

    dump_string("Interface Descriptor:", indent);
    dump_value(interface.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(4, "bDescriptorType", indent + 2, LSUSB_DUMP_WIDTH); // type 4 for interface
    dump_value(
        interface.number,
        "bInterfaceNumber",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        interface.alt_setting,
        "bAlternateSetting",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        interface.endpoints.len(),
        "bNumEndpoints",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        u8::from(interface.class.to_owned()),
        "bInterfaceClass",
        interface_name.unwrap_or(String::from("[unknown]")),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        interface.sub_class,
        "bInterfaceSubClass",
        sub_class_name.unwrap_or(String::from("[unknown]")),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        interface.protocol,
        "bInterfaceProtocol",
        protocol_name.unwrap_or_default(),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        interface.string_index,
        "iInterface",
        interface.name.as_ref().unwrap_or(&String::new()),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    // dump extra descriptors
    if let Some(dt_vec) = &interface.extra {
        for dt in dt_vec {
            match dt {
                // Should only be Device or Interface as we mask out the rest
                Descriptor::Device(cd) | Descriptor::Interface(cd) => match cd {
                    ClassDescriptor::Hid(hidd) => dump_hid_device(hidd, indent + 4),
                    ClassDescriptor::Ccid(ccid) => dump_ccid_desc(ccid, indent + 2),
                    ClassDescriptor::Printer(pd) => dump_printer_desc(pd, indent + 4),
                    ClassDescriptor::Communication(cd) => dump_comm_descriptor(cd, indent + 2),
                    ClassDescriptor::Dfu(dfud) => dump_dfu_interface(dfud, indent + 2),
                    ClassDescriptor::Midi(md, _) => dump_midistreaming_interface(md, indent + 4),
                    ClassDescriptor::Audio(uacd, uacp) => match &uacd.descriptor_subtype {
                        audio::UacType::Control(cs) => {
                            dump_audiocontrol_interface(uacd, cs, uacp, indent + 2)
                        }
                        audio::UacType::Streaming(ss) => {
                            dump_audiostreaming_interface(uacd, ss, uacp, indent + 2)
                        }
                        _ => (),
                    },
                    ClassDescriptor::Video(vcd, p) => match &vcd.descriptor_subtype {
                        video::UvcType::Control(cs) => {
                            dump_videocontrol_interface(vcd, cs, *p, indent + 2)
                        }
                        video::UvcType::Streaming(ss) => {
                            dump_videostreaming_interface(vcd, ss, *p, indent + 2);
                        }
                    },
                    ClassDescriptor::Generic(cc, gd) => match cc {
                        Some((BaseClass::Audio, 3, _)) => {
                            if let Ok(md) = audio::MidiDescriptor::try_from(gd.to_owned()) {
                                dump_midistreaming_interface(&md, indent + 4);
                            }
                        }
                        Some((BaseClass::Audio, s, p)) => {
                            if let Ok(uacd) =
                                audio::UacDescriptor::try_from((gd.to_owned(), *s, *p))
                            {
                                let uacp = audio::UacProtocol::from(*p);
                                match &uacd.descriptor_subtype {
                                    audio::UacType::Control(cs) => {
                                        dump_audiocontrol_interface(&uacd, cs, &uacp, indent + 2)
                                    }
                                    audio::UacType::Streaming(ss) => {
                                        dump_audiostreaming_interface(&uacd, ss, &uacp, indent + 2)
                                    }
                                    _ => (),
                                }
                            }
                        }
                        Some((BaseClass::Video, s, p)) => {
                            if let Ok(uvcd) =
                                video::UvcDescriptor::try_from((gd.to_owned(), *s, *p))
                            {
                                match &uvcd.descriptor_subtype {
                                    video::UvcType::Control(cs) => {
                                        dump_videocontrol_interface(&uvcd, cs, *p, indent + 2);
                                    }
                                    video::UvcType::Streaming(ss) => {
                                        dump_videostreaming_interface(&uvcd, ss, *p, indent + 2);
                                    }
                                }
                            }
                        }
                        Some((BaseClass::ApplicationSpecificInterface, 1, _)) => {
                            if let Ok(dfud) = DfuDescriptor::try_from(gd.to_owned()) {
                                dump_dfu_interface(&dfud, indent + 2);
                            }
                        }
                        _ => {
                            let junk = Vec::from(cd.to_owned());
                            dump_unrecognised(&junk, indent + 2);
                        }
                    },
                },
                Descriptor::Unknown(junk) | Descriptor::Junk(junk) => {
                    dump_unrecognised(junk, indent + 2);
                }
                _ => dump_unrecognised(&Vec::from(dt.to_owned()), indent + 2),
            }
        }
    }
}

/// Dump a [`Endpoint`] in style of lsusb --verbose
fn dump_endpoint(endpoint: &Endpoint, indent: usize) {
    dump_string("Endpoint Descriptor:", indent);
    dump_value(endpoint.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(5, "bDescriptorType", indent + 2, LSUSB_DUMP_WIDTH); // type 5 for endpoint
    dump_value_string_gap(
        format!("0x{:02x}", endpoint.address.address),
        "bEndpointAddress",
        format!(
            "EP {} {}",
            endpoint.address.number,
            endpoint.address.direction.to_string().to_uppercase()
        ),
        indent + 2,
        LSUSB_DUMP_WIDTH,
        2,
    );
    // this is printed as int even though it's a bitmap
    dump_value(
        endpoint.attributes(),
        "bmAttributes",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    // the three attribute breakdowns are left aligned one column past the
    // usual value field, so they get the wider width
    const ATTR_WIDTH: usize = LSUSB_DUMP_WIDTH + 1;
    dump_string_right(
        format!("{:?}", endpoint.transfer_type),
        "Transfer Type",
        indent + 4,
        ATTR_WIDTH,
    );
    dump_string_right(
        format!("{:?}", endpoint.sync_type),
        "Synch Type",
        indent + 4,
        ATTR_WIDTH,
    );
    dump_string_right(
        format!("{:?}", endpoint.usage_type),
        "Usage Type",
        indent + 4,
        ATTR_WIDTH,
    );
    dump_value_string_gap(
        format!("0x{:04x}", endpoint.max_packet_size),
        "wMaxPacketSize",
        format!("{} bytes", endpoint.max_packet_string()),
        indent + 2,
        LSUSB_DUMP_WIDTH,
        2,
    );
    dump_value(endpoint.interval, "bInterval", indent + 2, LSUSB_DUMP_WIDTH);
    // only for audio endpoints, which have a 9 byte rather than 7 byte descriptor
    if endpoint.length == 9 {
        dump_value(
            endpoint.refresh.unwrap_or(0),
            "bRefresh",
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
        dump_value(
            endpoint.sync_address.unwrap_or(0),
            "bSynchAddress",
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
    }

    // dump extra descriptors
    // kind of messy but it's out lsusb does it
    if let Some(dt_vec) = &endpoint.extra {
        for dt in dt_vec {
            match dt {
                Descriptor::Endpoint(cd) => match cd {
                    ClassDescriptor::Audio(ad, _) => {
                        dump_audiostreaming_endpoint(ad, indent + 2);
                    }
                    ClassDescriptor::Midi(md, _) => {
                        dump_midistreaming_endpoint(md, indent + 2);
                    }
                    ClassDescriptor::Video(vd, _) => {
                        dump_videocontrol_interrupt_endpoint(vd, indent + 2, LSUSB_DUMP_WIDTH);
                    }
                    // legacy as context should have been added to the descriptor
                    ClassDescriptor::Generic(cc, gd) => match cc {
                        Some((BaseClass::Audio, 2, p)) => {
                            if let Ok(uacd) = audio::UacDescriptor::try_from((gd.to_owned(), 2, *p))
                            {
                                dump_audiostreaming_endpoint(&uacd, indent + 2);
                            }
                        }
                        Some((BaseClass::Audio, 3, _)) => {
                            if let Ok(md) = audio::MidiDescriptor::try_from(gd.to_owned()) {
                                dump_midistreaming_endpoint(&md, indent + 2);
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                },
                // Misplaced descriptors
                Descriptor::Device(cd) => match cd {
                    ClassDescriptor::Ccid(ccid) => {
                        dump_ccid_desc(ccid, indent);
                    }
                    _ => {
                        println!(
                            "{:indent$}DEVICE CLASS: {}",
                            "",
                            Vec::<u8>::from(cd.to_owned())
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<Vec<String>>()
                                .join(" "),
                            indent = indent + 2
                        );
                    }
                },
                Descriptor::Interface(cd) => match cd {
                    ClassDescriptor::Generic(cc, gd) => match cc {
                        Some((BaseClass::CdcData, _, _))
                        | Some((BaseClass::CdcCommunications, _, _)) => {
                            if let Ok(cd) = gd.to_owned().try_into() {
                                dump_comm_descriptor(&cd, indent)
                            }
                        }
                        Some((BaseClass::MassStorage, _, _)) => {
                            dump_pipe_desc(gd, indent + 2);
                        }
                        _ => {
                            println!(
                                "{:indent$}INTERFACE CLASS: {}",
                                "",
                                Vec::<u8>::from(cd.to_owned())
                                    .iter()
                                    .map(|b| format!("{b:02x}"))
                                    .collect::<Vec<String>>()
                                    .join(" "),
                                indent = indent + 2
                            );
                        }
                    },
                    ClassDescriptor::Communication(cd) => dump_comm_descriptor(cd, 6),
                    _ => {
                        println!(
                            "{:indent$}INTERFACE CLASS: {}",
                            "",
                            Vec::<u8>::from(cd.to_owned())
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<Vec<String>>()
                                .join(" "),
                            indent = indent + 2
                        );
                    }
                },
                Descriptor::InterfaceAssociation(iad) => {
                    dump_interface_association(iad, indent + 2);
                }
                Descriptor::SsEndpointCompanion(ss) => {
                    dump_value(ss.max_burst, "bMaxBurst", indent + 2, LSUSB_DUMP_WIDTH);
                    // bulk streams, isochronous Mult and the byte count are
                    // each gated on their own transfer type in lsusb
                    if matches!(endpoint.transfer_type, TransferType::Bulk)
                        && ss.attributes & 0x1f != 0
                    {
                        dump_value(
                            1u32 << (ss.attributes & 0x1f),
                            "MaxStreams",
                            indent + 2,
                            LSUSB_DUMP_WIDTH,
                        );
                    }
                    if matches!(endpoint.transfer_type, TransferType::Isochronous)
                        && ss.attributes & 0x03 != 0
                    {
                        dump_value(ss.attributes & 0x3, "Mult", indent + 2, LSUSB_DUMP_WIDTH);
                    }
                    if matches!(
                        endpoint.transfer_type,
                        TransferType::Isochronous | TransferType::Interrupt
                    ) {
                        if let Some(bi) = ss.bytes_per_interval {
                            dump_value(bi, "wBytesPerInterval", indent + 2, LSUSB_DUMP_WIDTH);
                        }
                    }
                }
                Descriptor::Unknown(junk) | Descriptor::Junk(junk) => {
                    dump_unrecognised(junk, indent + 2);
                }
                _ => (),
            }
        }
    }
}

fn dump_ccid_desc(ccid: &CcidDescriptor, indent: usize) {
    dump_string("ChipCard Interface Descriptor:", indent);
    dump_value(ccid.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        ccid.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    if ccid.version.major() != 1 || (ccid.version.minor() != 0 && ccid.version.minor() != 1) {
        dump_value_string(
            ccid.version,
            "bcdCCID",
            "(Warning: Only accurate for version 1.0/1.1)",
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
    } else {
        dump_value(ccid.version, "bcdCCID", indent + 2, LSUSB_DUMP_WIDTH);
    }

    // lsusb names this nMaxSlotIndex, though the CCID spec calls it bMaxSlotIndex
    dump_value(
        ccid.max_slot_index,
        "nMaxSlotIndex",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_bitmap_strings_inline(
        ccid.voltage_support,
        ccid.voltage_support,
        "bVoltageSupport",
        |index| match index {
            0 => Some("5.0V"),
            1 => Some("3.0V"),
            2 => Some("1.8V"),
            _ => None,
        },
        indent + 2,
        LSUSB_DUMP_WIDTH,
        true,
    );

    dump_bitmap_strings_inline(
        ccid.protocols,
        ccid.protocols,
        "dwProtocols",
        |index| match index {
            0 => Some("T=0"),
            1 => Some("T=1"),
            _ => Some("(Invalid values detected)"),
        },
        indent + 2,
        LSUSB_DUMP_WIDTH,
        false,
    );

    dump_value(
        ccid.default_clock,
        "dwDefaultClock",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        ccid.max_clock,
        "dwMaximumClock",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        ccid.num_clock_supported,
        "bNumClockSupported",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        ccid.data_rate,
        "dwDataRate",
        "bps",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        ccid.max_data_rate,
        "dwMaxDataRate",
        "bps",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        ccid.num_data_rates_supp,
        "bNumDataRatesSupp.",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(ccid.max_ifsd, "dwMaxIFSD", indent + 2, LSUSB_DUMP_WIDTH);
    dump_bitmap_strings_inline(
        format!("{:08X}", ccid.sync_protocols),
        ccid.sync_protocols,
        "dwSyncProtocols",
        |index| match index {
            0 => Some("2-wire"),
            1 => Some("3-wire"),
            2 => Some("I2C"),
            _ => None,
        },
        indent + 2,
        LSUSB_DUMP_WIDTH,
        false,
    );

    dump_bitmap_strings_inline(
        format!("{:08X}", ccid.mechanical),
        ccid.mechanical,
        "dwMechanical",
        |index| match index {
            0 => Some("accept"),
            1 => Some("eject"),
            2 => Some("capture"),
            3 => Some("lock"),
            _ => None,
        },
        indent + 2,
        LSUSB_DUMP_WIDTH,
        false,
    );

    dump_value(
        format!("{:08X}", ccid.features),
        "dwFeatures",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    // written out longhand rather than as a bitmap walk because lsusb
    // interleaves two mutually exclusive choices - the negotiation mode and
    // the exchange level - between the plain flag bits, and warns rather than
    // listing when more than one bit of a choice is set
    for (mask, s) in [
        (0x0002, "Auto configuration based on ATR"),
        (0x0004, "Auto activation on insert"),
        (0x0008, "Auto voltage selection"),
        (0x0010, "Auto clock change"),
        (0x0020, "Auto baud rate change"),
    ] {
        if ccid.features & mask != 0 {
            dump_string(s, indent + 4);
        }
    }
    match ccid.features & (0x0040 | 0x0080) {
        0x0040 => dump_string("Auto parameter negotiation made by CCID", indent + 4),
        0x0080 => dump_string("Auto PPS made by CCID", indent + 4),
        0x0000 => (),
        _ => println!(
            "{:indent$}WARNING: conflicting negotiation features",
            "",
            indent = indent + 2
        ),
    }
    for (mask, s) in [
        (0x0100, "CCID can set ICC in clock stop mode"),
        (0x0200, "NAD value other than 0x00 accepted (T=1)"),
        (0x0400, "Auto IFSD exchange (T=1)"),
    ] {
        if ccid.features & mask != 0 {
            dump_string(s, indent + 4);
        }
    }
    match ccid.features & 0x00070000 {
        0x00000000 => dump_string("Character level exchange", indent + 4),
        0x00010000 => dump_string("TPDU level exchange", indent + 4),
        0x00020000 => dump_string("Short APDU level exchange", indent + 4),
        0x00040000 => dump_string("Short and extended APDU level exchange", indent + 4),
        _ => println!(
            "{:indent$}WARNING: conflicting exchange levels",
            "",
            indent = indent + 2
        ),
    }
    if ccid.features & 0x00100000 != 0 {
        dump_string("USB wakeup on ICC insertion and removal", indent + 4);
    }

    dump_value(
        ccid.max_ccid_msg_len,
        "dwMaxCCIDMsgLen",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    if ccid.class_get_response == 0xff {
        dump_value("echo", "bClassGetResponse", indent + 2, LSUSB_DUMP_WIDTH);
    } else {
        dump_value(
            format!("{:02X}", ccid.class_get_response),
            "bClassGetResponse",
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
    }

    if ccid.class_envelope == 0xff {
        dump_value("echo", "bClassEnvelope", indent + 2, LSUSB_DUMP_WIDTH);
    } else {
        dump_value(
            format!("{:02X}", ccid.class_envelope),
            "bClassEnvelope",
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
    }

    // left aligned at the value column rather than right aligned in it
    const LCD_WIDTH: usize = LSUSB_DUMP_WIDTH - 3;
    if ccid.lcd_layout == (0, 0) {
        dump_string_right("none", "wlcdLayout", indent + 2, LCD_WIDTH);
    } else {
        dump_string_right(
            format!("{} cols {} lines", ccid.lcd_layout.0, ccid.lcd_layout.1),
            "wlcdLayout",
            indent + 2,
            LCD_WIDTH,
        );
    }

    dump_bitmap_strings_inline(
        ccid.pin_support,
        ccid.pin_support,
        "bPINSupport",
        |index| match index {
            0 => Some("verification"),
            1 => Some("modification"),
            _ => None,
        },
        indent + 2,
        LSUSB_DUMP_WIDTH,
        false,
    );

    dump_value(
        ccid.max_ccid_busy_slots,
        "bMaxCCIDBusySlots",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    if !ccid.junk.is_empty() {
        println!(
            "{:indent$}junk             {}",
            "",
            ccid.junk
                .iter()
                .map(|b| format!(" {b:02x}"))
                .collect::<String>(),
            indent = indent + 2
        );
    }
}

fn dump_printer_desc(pd: &PrinterDescriptor, indent: usize) {
    dump_string("IPP Printer Descriptor:", indent);
    dump_value(pd.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        pd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        pd.release_number,
        "bcdReleaseNumber",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        pd.descriptors.len(),
        "bcdNumDescriptors",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    // the capability blocks are indented a level in from the descriptor's
    // own fields, and are two columns wider
    const CAP_WIDTH: usize = LSUSB_DUMP_WIDTH + 2;

    for desc in &pd.descriptors {
        // basic capabilities
        if desc.descriptor_type == 0x00 {
            dump_value(
                desc.versions_supported,
                "iIPPVersionsSupported",
                indent + 4,
                CAP_WIDTH,
            );
            dump_value_string(
                desc.uuid_string_index,
                "iIPPPrinterUUID",
                desc.uuid_string.as_ref().unwrap_or(&String::new()),
                indent + 4,
                CAP_WIDTH,
            );
            print!(
                "{:indent$}wBasicCapabilities   0x{:04x} ",
                "",
                desc.capabilities,
                indent = indent + 4
            );

            // capabilities
            if desc.capabilities & 0x0001 != 0 {
                print!(" Print");
            }
            if desc.capabilities & 0x0002 != 0 {
                print!(" Scan");
            }
            if desc.capabilities & 0x0004 != 0 {
                print!(" Fax");
            }
            if desc.capabilities & 0x0008 != 0 {
                print!(" Other");
            }
            if desc.capabilities & 0x0010 != 0 {
                print!(" HTTP-over-USB");
            }
            // the authentication field is a two bit enum, not a bitmap
            match desc.capabilities & 0x0060 {
                0x00 => print!(" No-Auth"),
                0x20 => print!(" Username-Auth"),
                0x40 => print!(" Reserved-Auth"),
                _ => print!(" Negotiable-Auth"),
            }
            println!();
        // vendor specific
        } else {
            dump_value_string(
                desc.descriptor_type,
                "UnknownCapabilities",
                format!("{:5}", desc.length),
                indent + 4,
                CAP_WIDTH,
            );
        }
    }
}

fn dump_bad_comm(cd: &cdc::CommunicationDescriptor, indent: usize) {
    let data = Into::<Vec<u8>>::into(cd.to_owned());
    println!(
        "{:^indent$}INVALID CDC ({:#}): {}",
        "",
        cd.descriptor_subtype,
        data.iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<String>>()
            .join(" ")
    );
}

fn dump_comm_descriptor(cd: &cdc::CommunicationDescriptor, indent: usize) {
    match &cd.interface {
        cdc::CdcInterfaceDescriptor::Header(d) => {
            dump_string("CDC Header:", indent);
            dump_value(d.version, "bcdCDC", indent + 2, LSUSB_DUMP_WIDTH);
        }
        cdc::CdcInterfaceDescriptor::CallManagement(cd) => {
            dump_string("CDC Call Management:", indent);
            dump_hex(
                cd.capabilities,
                "bmCapabilities",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_bitmap_strings(
                cd.capabilities,
                |b| match b {
                    0 => Some("call management"),
                    1 => Some("use DataInterface"),
                    _ => None,
                },
                indent + 4,
            );
            dump_string_right(
                cd.data_interface,
                "bDataInterface",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
        }
        cdc::CdcInterfaceDescriptor::AbstractControlManagement(cd) => {
            dump_string("CDC ACM:", indent);
            dump_hex(
                cd.capabilities,
                "bmCapabilities",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_bitmap_strings_invert(
                cd.capabilities,
                |b| match b {
                    0 => Some("get/set/clear comm features"),
                    1 => Some("line coding and serial state"),
                    2 => Some("sends break"),
                    3 => Some("connection notifications"),
                    _ => None,
                },
                indent + 4,
            );
        }
        cdc::CdcInterfaceDescriptor::Union(cd) => {
            dump_string("CDC Union:", indent);
            dump_string_right(
                cd.master_interface,
                "bMasterInterface",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            // each slave is followed by a space rather than separated by one
            println!(
                "{:indent$}bSlaveInterface         {}",
                "",
                cd.slave_interface
                    .iter()
                    .map(|b| format!("{b} "))
                    .collect::<String>(),
                indent = indent + 2
            );
        }
        cdc::CdcInterfaceDescriptor::CountrySelection(cd) => {
            dump_string("Country Selection:", indent);
            dump_value_string(
                cd.country_code_date_index,
                "iCountryCodeRelDate",
                cd.country_code_date
                    .as_ref()
                    .unwrap_or(&String::from("(?)")),
                indent + 2,
                LSUSB_DUMP_WIDTH + 3,
            );
            for d in &cd.country_codes {
                // printed as the two bytes in descriptor order, which puts
                // the low byte first
                dump_value(
                    format!("0x{:02x}{:02x}", d & 0xff, d >> 8),
                    "wCountryCode",
                    indent + 2,
                    LSUSB_DUMP_WIDTH + 3,
                );
            }
        }
        cdc::CdcInterfaceDescriptor::TelephoneOperations(d) => {
            dump_string("CDC Telephone operations:", indent);
            dump_hex(
                d.capabilities,
                "bmCapabilities",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_bitmap_strings_invert(
                d.capabilities,
                |b| match b {
                    0 => Some("simple mode"),
                    1 => Some("standalone mode"),
                    2 => Some("computer centric mode"),
                    _ => None,
                },
                indent + 4,
            );
        }
        cdc::CdcInterfaceDescriptor::NetworkChannel(d) => {
            dump_string("Network Channel Terminal:", indent);
            // this descriptor is two columns wider than most CDC ones
            const NCT_WIDTH: usize = LSUSB_DUMP_WIDTH + 2;
            dump_value(d.entity_id, "bEntityId", indent + 2, NCT_WIDTH);
            dump_value_string(
                d.name_string_index,
                "iName",
                d.name.as_ref().unwrap_or(&String::from("(?)")),
                indent + 2,
                NCT_WIDTH,
            );
            dump_value(d.channel_index, "bChannelIndex", indent + 2, NCT_WIDTH);
            dump_value(
                d.physical_interface,
                "bPhysicalInterface",
                indent + 2,
                NCT_WIDTH,
            );
        }
        cdc::CdcInterfaceDescriptor::EthernetNetworking(d) => {
            // the widest CDC descriptor by some margin
            const ETH_WIDTH: usize = LSUSB_DUMP_WIDTH + 9;
            dump_string("CDC Ethernet:", indent);
            dump_value_string(
                d.mac_address_index,
                "iMacAddress",
                d.mac_address.as_ref().unwrap_or(&String::from("(?)")),
                indent + 2,
                ETH_WIDTH,
            );
            dump_hex(
                d.ethernet_statistics,
                "bmEthernetStatistics",
                indent + 2,
                ETH_WIDTH,
            );
            dump_value(d.max_segment_size, "wMaxSegmentSize", indent + 2, ETH_WIDTH);
            dump_hex(
                d.num_multicast_filters,
                "wNumberMCFilters",
                indent + 2,
                ETH_WIDTH,
            );
            dump_value(
                d.num_power_filters,
                "bNumberPowerFilters",
                indent + 2,
                ETH_WIDTH,
            );
        }
        cdc::CdcInterfaceDescriptor::WirelessHandsetControlModel(d) => {
            dump_string("CDC WHCM:", indent);
            dump_value(d.version, "bcdVersion", indent + 2, LSUSB_DUMP_WIDTH);
        }
        cdc::CdcInterfaceDescriptor::MobileDirectLineModelFunctional(d) => {
            dump_string("CDC MDLM:", indent);
            dump_value(d.version, "bcdCDC", indent + 2, LSUSB_DUMP_WIDTH);
            dump_guid(&d.guid, "bGUID", indent + 2, LSUSB_DUMP_WIDTH - 4);
        }
        cdc::CdcInterfaceDescriptor::MobileDirectLineModelDetail(d) => {
            dump_string("CDC MDLM detail:", indent);
            dump_value(
                format!("{:02x}", d.guid_descriptor_type),
                "bGuidDescriptorType",
                indent + 2,
                LSUSB_DUMP_WIDTH - 2,
            );
            println!(
                "{:indent$}bDetailData         {}",
                "",
                d.detail_data
                    .iter()
                    .map(|b| format!(" {b:02x}"))
                    .collect::<String>(),
                indent = indent + 2
            );
        }
        cdc::CdcInterfaceDescriptor::DeviceManagement(d) => {
            dump_string("CDC Device Management:", indent);
            dump_value(d.version, "bcdVersion", indent + 2, LSUSB_DUMP_WIDTH);
            dump_string_right(
                d.max_command,
                "wMaxCommand",
                indent + 2,
                LSUSB_DUMP_WIDTH - 3,
            );
        }
        cdc::CdcInterfaceDescriptor::Obex(d) => {
            dump_string("CDC OBEX:", indent);
            dump_value(d.version, "bcdVersion", indent + 2, LSUSB_DUMP_WIDTH);
        }
        cdc::CdcInterfaceDescriptor::CommandSet(d) => {
            dump_string("CDC Command Set:", indent);
            dump_value(d.version, "bcdVersion", indent + 2, LSUSB_DUMP_WIDTH);
            dump_value_string(
                d.command_set_string_index,
                "iCommandSet",
                d.command_set_string
                    .as_ref()
                    .unwrap_or(&String::from("(?)")),
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_guid(&d.guid, "bGUID", indent + 2, LSUSB_DUMP_WIDTH - 3);
        }
        cdc::CdcInterfaceDescriptor::Ncm(d) => {
            dump_string("CDC NCM:", indent);
            dump_value(d.version, "bcdNcmVersion", indent + 2, LSUSB_DUMP_WIDTH);
            dump_hex(
                d.network_capabilities,
                "bmNetworkCapabilities",
                indent + 2,
                LSUSB_DUMP_WIDTH + 1,
            );
            dump_bitmap_strings_invert(
                d.network_capabilities,
                |b| match b {
                    0 => Some("packet filter"),
                    1 => Some("net address"),
                    2 => Some("encapsulated commands"),
                    3 => Some("max datagram size"),
                    4 => Some("crc mode"),
                    5 => Some("8-byte ntb input size"),
                    _ => None,
                },
                indent + 4,
            );
        }
        cdc::CdcInterfaceDescriptor::Mbim(d) => {
            dump_string("CDC MBIM:", indent);
            dump_value(d.version, "bcdMBIMVersion", indent + 2, LSUSB_DUMP_WIDTH);
            // these four are left aligned at the value column
            const MBIM_WIDTH: usize = LSUSB_DUMP_WIDTH - 3;
            dump_string_right(
                d.max_control_message,
                "wMaxControlMessage",
                indent + 2,
                MBIM_WIDTH,
            );
            dump_string_right(d.number_filters, "bNumberFilters", indent + 2, MBIM_WIDTH);
            dump_string_right(d.max_filter_size, "bMaxFilterSize", indent + 2, MBIM_WIDTH);
            dump_string_right(
                d.max_segment_size,
                "wMaxSegmentSize",
                indent + 2,
                MBIM_WIDTH,
            );
            dump_hex(
                d.network_capabilities,
                "bmNetworkCapabilities",
                indent + 2,
                LSUSB_DUMP_WIDTH + 1,
            );
            dump_bitmap_strings_invert(
                d.network_capabilities,
                |b| match b {
                    3 => Some("max datagram size"),
                    5 => Some("8-byte ntb input size"),
                    _ => None,
                },
                indent + 4,
            );
        }
        cdc::CdcInterfaceDescriptor::MbimExtended(d) => {
            dump_string("CDC MBIM Extended:", indent);
            // the widest CDC descriptor of all
            const MBIMX_WIDTH: usize = LSUSB_DUMP_WIDTH + 12;
            dump_value(d.version, "bcdMBIMExtendedVersion", indent + 2, MBIMX_WIDTH);
            dump_value(
                d.max_outstanding_command_messages,
                "bMaxOutstandingCommandMessages",
                indent + 2,
                MBIMX_WIDTH,
            );
            dump_value(d.mtu, "wMTU", indent + 2, MBIMX_WIDTH);
        }
        cdc::CdcInterfaceDescriptor::Invalid(_) => {
            dump_bad_comm(cd, indent);
        }
        _ => {
            println!(
                "{:^indent$}UNRECOGNIZED CDC: {}",
                "",
                Vec::<u8>::from(cd.to_owned())
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<String>>()
                    .join(" "),
                indent = indent
            );
        }
    }
}

fn dump_dfu_interface(dfud: &DfuDescriptor, indent: usize) {
    // lsusb lays this descriptor out wider than the rest
    const DFU_WIDTH: usize = LSUSB_DUMP_WIDTH + 12;

    dump_string("Device Firmware Upgrade Interface Descriptor:", indent);
    dump_value(dfud.length, "bLength", indent + 2, DFU_WIDTH);
    dump_value(
        dfud.descriptor_type,
        "bDescriptorType",
        indent + 2,
        DFU_WIDTH,
    );
    dump_value(dfud.attributes, "bmAttributes", indent + 2, DFU_WIDTH);

    if dfud.attributes & 0xf0 != 0 {
        println!("{:indent$}(unknown attributes!)", "", indent = indent + 4);
    }
    if dfud.attributes & 0x08 != 0 {
        println!("{:indent$}Will Detach", "", indent = indent + 4);
    } else {
        println!("{:indent$}Will Not Detach", "", indent = indent + 4);
    }
    // bitManifestationTolerant: set means the device can still be talked to
    // after the manifestation phase (DFU 1.1 section 4.1.3)
    if dfud.attributes & 0x04 != 0 {
        println!("{:indent$}Manifestation Tolerant", "", indent = indent + 4);
    } else {
        println!(
            "{:indent$}Manifestation Intolerant",
            "",
            indent = indent + 4
        );
    }
    if dfud.attributes & 0x02 != 0 {
        println!("{:indent$}Upload Supported", "", indent = indent + 4);
    } else {
        println!("{:indent$}Upload Unsupported", "", indent = indent + 4);
    }
    if dfud.attributes & 0x01 != 0 {
        println!("{:indent$}Download Supported", "", indent = indent + 4);
    } else {
        println!("{:indent$}Download Unsupported", "", indent = indent + 4);
    }

    dump_value_string(
        dfud.detach_timeout,
        "wDetachTimeout",
        "milliseconds",
        indent + 2,
        DFU_WIDTH,
    );
    dump_value_string(
        dfud.transfer_size,
        "wTransferSize",
        "bytes",
        indent + 2,
        DFU_WIDTH,
    );
    if let Some(bcd) = dfud.dfu_version.as_ref() {
        // left aligned at the value column, unlike the fields above it
        dump_string_right(bcd, "bcdDFUVersion", indent + 2, DFU_WIDTH - 4);
    }
}

fn dump_pipe_desc(gd: &GenericDescriptor, indent: usize) {
    if gd.length == 4 && gd.descriptor_type == 0x24 {
        let subtype_string = match gd.descriptor_subtype {
            1 => "Command pipe",
            2 => "Status pipe",
            3 => "Data-in pipe",
            4 => "Data-out pipe",
            0 | 5..=0xdf | 0xf0..=0xff => "Reserved",
            0xe0..=0xef => "Vendor specific",
        };

        println!(
            "{:indent$}{} (0x{:02x})",
            "",
            subtype_string,
            gd.descriptor_subtype,
            indent = indent
        );
    } else {
        println!(
            "{:indent$}INTERFACE CLASS: {}",
            "",
            Vec::<u8>::from(gd.to_owned())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<String>>()
                .join(" "),
            indent = indent
        );
    }
}

fn dump_security(sec: &SecurityDescriptor, indent: usize) {
    dump_string("Security Descriptor:", indent);
    dump_value(sec.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        sec.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(
        sec.total_length,
        "wTotalLength",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        sec.encryption_types,
        "bNumEncryptionTypes",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
}

fn dump_encryption_type(enc: &EncryptionDescriptor, indent: usize) {
    let enct_string = match enc.encryption_type as u8 {
        0 => "INSECURE",
        1 => "WIRED",
        2 => "CCM_1",
        3 => "RSA_1",
        _ => "RESERVED",
    };

    dump_string("Encryption Type Descriptor:", indent);
    dump_value(enc.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        enc.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        enc.encryption_type as u8,
        "bEncryptionType",
        enct_string,
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        enc.encryption_value,
        "bEncryptionValue",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        enc.auth_key_index,
        "bAuthKeyIndex",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
}

fn dump_interface_association(iad: &InterfaceAssociationDescriptor, indent: usize) {
    dump_string("Interface Association:", indent);
    dump_value(iad.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        iad.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        iad.first_interface,
        "bFirstInterface",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        iad.interface_count,
        "bInterfaceCount",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        iad.function_class,
        "bFunctionClass",
        names::class(iad.function_class).unwrap_or_default(),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        iad.function_sub_class,
        "bFunctionSubClass",
        names::subclass(iad.function_class, iad.function_sub_class).unwrap_or_default(),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        iad.function_protocol,
        "bFunctionProtocol",
        names::protocol(
            iad.function_class,
            iad.function_sub_class,
            iad.function_protocol,
        )
        .unwrap_or_default(),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        iad.function_string_index,
        "iFunction",
        iad.function_string.as_ref().unwrap_or(&String::new()),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
}

fn dump_hid_device(hidd: &HidDescriptor, indent: usize) {
    dump_string("HID Device Descriptor:", indent);
    dump_value(hidd.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        hidd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(hidd.bcd_hid, "bcdHID", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value_string(
        hidd.country_code,
        "bCountryCode",
        names::countrycode(hidd.country_code).unwrap_or_else(|| String::from("Unknown")),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        hidd.descriptors.len(),
        "bNumDescriptors",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    for desc in &hidd.descriptors {
        dump_value_string(
            desc.descriptor_type,
            "bDescriptorType",
            names::hid(desc.descriptor_type).unwrap_or_else(|| String::from("Unknown")),
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
        dump_value(
            desc.length,
            "wDescriptorLength",
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
    }

    for desc in &hidd.descriptors {
        // only print report descriptor
        if desc.descriptor_type != 0x22 {
            continue;
        }

        match desc.data.as_ref() {
            Some(d) => {
                dump_report_desc(d, indent + 2);
            }
            None => {
                // lsusb leaves a trailing space after the colon here
                dump_string("Report Descriptors: ", indent + 2);
                dump_string("** UNAVAILABLE **", indent + 4);
            }
        }
    }
}

fn dump_device_qualifier(dqd: &DeviceQualifierDescriptor, indent: usize) {
    dump_string("Device Qualifier (for other device speed):", indent);
    dump_value(dqd.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        dqd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(dqd.version, "bcdUSB", indent + 2, LSUSB_DUMP_WIDTH);
    let class: u8 = dqd.device_class as u8;
    dump_value_string(
        class,
        "bDeviceClass",
        names::class(class).unwrap_or(String::from("[unknown]")),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        dqd.device_subclass,
        "bDeviceSubClass",
        names::subclass(class, dqd.device_subclass).unwrap_or(String::from("[unknown]")),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        dqd.device_protocol,
        "bDeviceProtocol",
        names::protocol(class, dqd.device_subclass, dqd.device_protocol).unwrap_or_default(),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        dqd.max_packet_size,
        "bMaxPacketSize0",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        dqd.num_configurations,
        "bNumConfigurations",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
}

fn dump_debug(dd: &DebugDescriptor, indent: usize) {
    dump_string("Debug descriptor:", indent);
    dump_value(dd.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        dd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(
        dd.debug_in_endpoint,
        "bDebugInEndpoint",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(
        dd.debug_out_endpoint,
        "bDebugOutEndpoint",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
}

fn dump_otg(otg: &OnTheGoDescriptor, indent: usize) {
    dump_string("OTG Descriptor:", indent);
    dump_value(otg.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        otg.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(otg.attributes, "bmAttributes", indent + 2, LSUSB_DUMP_WIDTH);
    if otg.attributes & 0x01 != 0 {
        dump_string("SRP (Session Request Protocol)", indent + 4);
    }
    if otg.attributes & 0x02 != 0 {
        dump_string("HNP (Host Negotiation Protocol)", indent + 4);
    }
}

const LINK_STATE_DESCRIPTIONS: [&str; 12] = [
    "U0",
    "U1",
    "U2",
    "suspend",
    "SS.disabled",
    "Rx.Detect",
    "SS.Inactive",
    "Polling",
    "Recovery",
    "Hot Reset",
    "Compliance",
    "Loopback",
];

/// Collect the strings for the bits set in a port status byte, MSB to LSB
///
/// Each is prefixed with a space, as lsusb emits them, so that concatenating
/// them onto the port line needs no separator of its own.
fn bitmap_strings_port<T>(bitmap: T, strings_f: fn(usize) -> Option<&'static str>) -> String
where
    T: std::fmt::Display + std::fmt::LowerHex + Copy + Into<u64>,
{
    let bitmap_u64: u64 = bitmap.into();
    let num_bits = std::mem::size_of::<T>() * 8;
    let mut ret = String::new();
    for index in (0..num_bits).rev() {
        if (bitmap_u64 >> index) & 0x1 != 0 {
            if let Some(string) = strings_f(index) {
                ret.push(' ');
                ret.push_str(string);
            }
        }
    }

    ret
}

/// The hub descriptor is a column narrower than the rest of the dump
const LSUSB_HUB_WIDTH: usize = LSUSB_DUMP_WIDTH - 2;

fn dump_hub(hd: &HubDescriptor, protocol: u8, bcd: u16, has_ssp: bool, indent: usize) {
    let is_ext_status = protocol == 3 && bcd >= 0x0310 && has_ssp;
    dump_string("Hub Descriptor:", indent);
    dump_value(hd.length, "bLength", indent + 2, LSUSB_HUB_WIDTH);
    dump_value(
        hd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_HUB_WIDTH,
    );
    dump_value(hd.num_ports, "nNbrPorts", indent + 2, LSUSB_HUB_WIDTH);
    dump_hex(
        hd.characteristics,
        "wHubCharacteristic",
        indent + 2,
        LSUSB_HUB_WIDTH,
    );
    match hd.characteristics & 0x03 {
        0 => println!("{:indent$}Ganged power switching", "", indent = indent + 4),
        1 => println!(
            "{:indent$}Per-port power switching",
            "",
            indent = indent + 4
        ),
        _ => println!(
            "{:indent$}No power switching (usb 1.0)",
            "",
            indent = indent + 4
        ),
    }
    if hd.characteristics & 0x04 != 0 {
        println!("{:indent$}Compound device", "", indent = indent + 4);
    }
    match (hd.characteristics >> 3) & 0x03 {
        0 => println!(
            "{:indent$}Ganged overcurrent protection",
            "",
            indent = indent + 4
        ),
        1 => println!(
            "{:indent$}Per-port overcurrent protection",
            "",
            indent = indent + 4
        ),
        _ => println!(
            "{:indent$}No overcurrent protection",
            "",
            indent = indent + 4
        ),
    }

    if (1..3).contains(&protocol) {
        let l = (hd.characteristics >> 5) & 0x03;
        dump_string(
            &format!("TT think time {} FS bits", (l + 1) * 8),
            indent + 4,
        );
    }
    if protocol != 3 && hd.characteristics & (1 << 7) != 0 {
        dump_string("Port indicators", indent + 4);
    }
    dump_value_string(
        hd.power_on_to_power_good,
        "bPwrOn2PwrGood",
        "* 2 milli seconds",
        indent + 2,
        LSUSB_HUB_WIDTH,
    );

    if protocol == 3 {
        dump_value_string(
            (hd.control_current as u32) * 4,
            "bHubContrCurrent",
            "milli Ampere",
            indent + 2,
            LSUSB_HUB_WIDTH,
        );
    } else {
        dump_value_string(
            hd.control_current,
            "bHubContrCurrent",
            "milli Ampere",
            indent + 2,
            LSUSB_HUB_WIDTH,
        );
    }

    let offset = if protocol == 3 {
        dump_value_string(
            format!("0.{:1}", hd.latency().unwrap_or(0)),
            "bHubDecLat",
            "micro seconds",
            indent + 2,
            LSUSB_HUB_WIDTH,
        );
        dump_value_string(
            hd.delay().unwrap_or(0),
            "wHubDelay",
            "nano seconds",
            indent + 2,
            LSUSB_HUB_WIDTH,
        );
        2
    } else {
        0
    };

    // determine the number of bytes needed to represent the number of ports
    let mut l = ((hd.num_ports >> 3) + 1) as usize;
    if l > 3 {
        l = 3;
    }
    // these two are the only hub fields lsusb left aligns: a fixed prefix
    // followed by one space-prefixed byte per port group, so the line grows
    // rightwards rather than the value being right aligned in a column
    let bitmask_bytes = |skip: usize| -> String {
        hd.data
            .iter()
            .skip(skip)
            .take(l)
            .map(|b| format!(" 0x{b:02x}"))
            .collect::<String>()
    };
    println!(
        "{:indent$}DeviceRemovable   {}",
        "",
        bitmask_bytes(offset),
        indent = indent + 2
    );
    if protocol != 3 {
        println!(
            "{:indent$}PortPwrCtrlMask   {}",
            "",
            bitmask_bytes(offset + l),
            indent = indent + 2
        );
    }

    if let Some(ps) = hd.port_statuses.as_ref() {
        // + 1 in lsusb for some reason...
        dump_string("Hub Port Status:", indent + 1);
        for (i, p) in ps.iter().enumerate() {
            let port_status_string = format!(
                "Port {}: {:02x}{:02x}.{:02x}{:02x}",
                i + 1,
                p[3],
                p[2],
                p[1],
                p[0]
            );
            if bcd < 0x0300 {
                let s2_string = bitmap_strings_port(p[2], |b| match b {
                    0 => Some("C_CONNECT"),
                    1 => Some("C_ENABLE"),
                    2 => Some("C_SUSPEND"),
                    3 => Some("C_OC"),
                    4 => Some("C_RESET"),
                    _ => None,
                });
                let s1_string = bitmap_strings_port(p[1], |b| match b {
                    0 => Some("power"),
                    1 => Some("lowspeed"),
                    2 => Some("highspeed"),
                    3 => Some("test"),
                    4 => Some("indicator"),
                    _ => None,
                });
                let s0_string = bitmap_strings_port(p[0], |b| match b {
                    0 => Some("connect"),
                    1 => Some("enable"),
                    2 => Some("suspend"),
                    3 => Some("oc"),
                    4 => Some("RESET"),
                    5 => Some("L1"),
                    _ => None,
                });
                dump_string(
                    &format!("{port_status_string}{s2_string}{s1_string}{s0_string}"),
                    indent + 3,
                );
            } else {
                let link_state = (((p[0] & 0xe0) >> 5) + ((p[1] & 0x01) << 3)) as usize;
                let s2_string = bitmap_strings_port(p[2], |b| match b {
                    0 => Some("C_CONNECT"),
                    3 => Some("C_OC"),
                    4 => Some("C_RESET"),
                    5 => Some("C_BH_RESET"),
                    6 => Some("C_LINK_STATE"),
                    7 => Some("C_CONFIG_ERROR"),
                    _ => None,
                });
                let s1_string = format!(
                    "{}{}",
                    if p[1] & 0x1c == 0 {
                        " 5Gbps"
                    } else {
                        " Unknown Speed"
                    },
                    if p[1] & 0x02 != 0 { " power" } else { "" },
                );
                let s0_string = bitmap_strings_port(p[0], |b| match b {
                    0 => Some("connect"),
                    1 => Some("enable"),
                    3 => Some("oc"),
                    4 => Some("RESET"),
                    _ => None,
                });
                let link_state_string = LINK_STATE_DESCRIPTIONS
                    .get(link_state)
                    .map(|s| format!(" {s}"))
                    .unwrap_or_default();
                dump_string(
                    &format!(
                        "{port_status_string}{s2_string}{s1_string}{link_state_string}{s0_string}"
                    ),
                    indent + 3,
                );
            }

            if is_ext_status && (p[0] & 0x01 == 0x01) {
                dump_string(
                    &format!(
                        "Ext Status: {:02x}{:02x}.{:02x}{:02x}",
                        p[7], p[6], p[5], p[4]
                    ),
                    indent + 5,
                );
                dump_string(
                    &format!(
                        "RX Speed Attribute ID: {} Lanes: {}",
                        p[4] & 0x0f,
                        (p[5] & 0x0f) + 1
                    ),
                    indent + 7,
                );
                dump_string(
                    &format!(
                        "TX Speed Attribute ID: {} Lanes: {}",
                        (p[4] >> 4) & 0x0f,
                        ((p[5] >> 4) & 0x0f) + 1
                    ),
                    indent + 7,
                );
            }
        }
    }
}

fn dump_device_status(status: u16, otg: bool, super_speed: bool, indent: usize) {
    dump_hex(status, "Device Status:", indent, LSUSB_DUMP_WIDTH);
    if status & 0x01 != 0 {
        println!("{:indent$}Self Powered", "", indent = indent + 2);
    } else {
        println!("{:indent$}(Bus Powered)", "", indent = indent + 2);
    }
    if status & 0x02 != 0 {
        println!("{:indent$}Remote Wakeup Enabled", "", indent = indent + 2);
    }
    if super_speed {
        if status & (1 << 2) != 0 {
            println!("{:indent$}U1 Enabled", "", indent = indent + 2);
        }
        if status & (1 << 3) != 0 {
            println!("{:indent$}U2 Enabled", "", indent = indent + 2);
        }
        if status & (1 << 4) != 0 {
            println!(
                "{:indent$}Latency Tolerance Messaging (LTM) Enabled",
                "",
                indent = indent + 2
            );
        }
    }
    if otg {
        if status & (1 << 3) != 0 {
            println!("{:indent$}HNP Enabled", "", indent = indent + 2);
        }
        if status & (1 << 4) != 0 {
            println!("{:indent$}HNP Capable", "", indent = indent + 2);
        }
        if status & (1 << 5) != 0 {
            println!("{:indent$}ALT port is HNP Capable", "", indent = indent + 2);
        }
    }
    if status & (1 << 6) != 0 {
        println!("{:indent$}Debug Mode", "", indent = indent + 2);
    }
}

/// Verbatim port of lsusb's dump_unit - not very Rust, don't judge!
///
/// `data` is the full item value; a Unit item may carry four data bytes, so it
/// must not be narrowed to u16 before the nibble walk below.
fn dump_unit(mut data: u32, len: usize, indent: usize) {
    let systems = |t: u32| match t {
        0x01 => "SI Linear",
        0x02 => "SI Rotation",
        0x03 => "English Linear",
        0x04 => "English Rotation",
        _ => "None",
    };
    let units = |t: u32, i: usize| match (t, i) {
        (1, 1) => "Centimeter",
        (2, 1) => "Radians",
        (1, 2) | (2, 2) => "Gram",
        (1, 4) | (2, 4) => "Kelvin",
        (3, 1) => "Inch",
        (4, 1) => "Degrees",
        (1, i) | (2, i) | (3, i) | (4, i) => match i {
            0x02 => "Slug",
            0x03 => "Seconds",
            0x04 => "Fahrenheit",
            0x05 => "Ampere",
            0x06 => "Candela",
            _ => "None",
        },
        (_, _) => "None",
    };

    let sys = data & 0xf;
    data >>= 4;

    if sys > 4 {
        if sys == 0xf {
            println!("{:indent$}System: Vendor defined, Unit: (unknown)", "");
        } else {
            println!("{:indent$}System: Reserved, Unit: (unknown)", "");
        }

        return;
    }

    print!("{:indent$}System: {}, Unit: ", "", systems(sys));

    let mut earlier_unit = 0;

    for i in 1..len * 2 {
        let nibble = data & 0xf;
        data >>= 4;
        if nibble != 0 {
            if earlier_unit > 0 {
                print!("*");
            }
            print!("{}", units(sys, i));
            earlier_unit += 1;
            /* This is a _signed_ nibble(!) */
            if nibble != 1 {
                let mut val: i8 = (nibble as i8) & 0x7;
                if nibble & 0x08 != 0x00 {
                    val = -((0x7 & !val) + 1);
                }
                print!("^{val}");
            }
        }
    }

    if earlier_unit == 0 {
        print!("(None)");
    }
    println!();
}

/// Dumps HID report data ported directly from lsusb - it's not pretty but works...
fn dump_report_desc(desc: &[u8], indent: usize) {
    // lsusb hard codes a 28 space indent for the lines decoding an item,
    // rather than deriving it from the item's own
    const REPORT_INDENT: usize = 28;
    let types = |t: u8| match t {
        0x00 => "Main",
        0x01 => "Global",
        0x02 => "Local",
        _ => "reserved",
    };

    dump_string(
        &format!("Report Descriptor: (length is {})", desc.len()),
        indent,
    );

    // both carry across items: the usage page set by a Global item qualifies
    // every Local usage that follows it, and lsusb lets an item with no data
    // bytes see the previous item's value
    let mut data: u32 = 0xffff;
    let mut hut: u8 = 0xff;

    let mut i = 0;
    while i < desc.len() {
        let b = desc[i];
        let mut bsize = (b & 0x03) as usize;
        if bsize == 3 {
            bsize = 4;
        }
        let btype = b & (0x03 << 2);
        let btag = b & !0x03;

        // Check for descriptor bounds before the header, so a truncated item
        // does not leave a half written line behind
        if i + 1 + bsize > desc.len() {
            println!(
                "{:indent$}** TRUNCATED at offset {} **",
                "",
                i,
                indent = indent + 2
            );
            break;
        }

        // Item Header
        print!(
            "{:indent$}Item({:<6}): {}, data=",
            "",
            types(btype >> 2),
            names::report_tag(btag).unwrap_or_else(|| String::from("Unknown")),
            indent = indent + 2
        );

        if bsize > 0 {
            print!(" [ ");
            data = 0;
            for j in 0..bsize {
                data |= (desc[i + 1 + j] as u32) << (j * 8);
                print!("0x{:02x} ", desc[i + 1 + j]);
            }
            println!("] {data}");
        } else {
            println!("none");
        }

        match btag {
            // usage page
            0x04 => {
                hut = data as u8;
                println!(
                    "{:indent$}{}",
                    "",
                    names::huts(hut).unwrap_or_else(|| String::from("Unknown")),
                    indent = REPORT_INDENT
                );
            }
            // usage, usage minimum, usage maximum
            0x08 | 0x18 | 0x28 => {
                println!(
                    "{:indent$}{}",
                    "",
                    names::hutus(hut, data as u16).unwrap_or_else(|| String::from("Unknown")),
                    indent = REPORT_INDENT
                );
            }
            // unit exponent - signed, so 0x0e reads as -2 not 14
            0x54 => {
                println!(
                    "{:indent$}Unit Exponent: {}",
                    "",
                    data as u8 as i8,
                    indent = REPORT_INDENT
                );
            }
            // unit
            0x64 => dump_unit(data, bsize, REPORT_INDENT),
            // collection
            0xa0 => match data {
                0x00 => println!("{:indent$}Physical", "", indent = REPORT_INDENT),
                0x01 => println!("{:indent$}Application", "", indent = REPORT_INDENT),
                0x02 => println!("{:indent$}Logical", "", indent = REPORT_INDENT),
                0x03 => println!("{:indent$}Report", "", indent = REPORT_INDENT),
                0x04 => println!("{:indent$}Named Array", "", indent = REPORT_INDENT),
                0x05 => println!("{:indent$}Usage Switch", "", indent = REPORT_INDENT),
                0x06 => println!("{:indent$}Usage Modifier", "", indent = REPORT_INDENT),
                _ => {
                    if (data & 0x80) == 0x80 {
                        println!("{:indent$}Vendor defined", "", indent = REPORT_INDENT)
                    } else {
                        println!(
                            "{:indent$}Reserved for future use.",
                            "",
                            indent = REPORT_INDENT
                        )
                    }
                }
            },
            // input, output, feature
            0x80 | 0x90 | 0xb0 => {
                let attributes_1 = format!(
                    "{:indent$}{} {} {} {} {}",
                    "",
                    if data & 0x01 != 0 { "Constant" } else { "Data" },
                    if data & 0x02 != 0 {
                        "Variable"
                    } else {
                        "Array"
                    },
                    if data & 0x04 != 0 {
                        "Relative"
                    } else {
                        "Absolute"
                    },
                    if data & 0x08 != 0 { "Wrap" } else { "No_Wrap" },
                    if data & 0x10 != 0 {
                        "Non_Linear"
                    } else {
                        "Linear"
                    },
                    indent = REPORT_INDENT
                );

                let attributes_2 = format!(
                    "{:indent$}{} {} {} {}",
                    "",
                    if data & 0x20 != 0 {
                        "No_Preferred_State"
                    } else {
                        "Preferred_State"
                    },
                    if data & 0x40 != 0 {
                        "Null_State"
                    } else {
                        "No_Null_Position"
                    },
                    if data & 0x80 != 0 {
                        "Volatile"
                    } else {
                        "Non_Volatile"
                    },
                    if data & 0x100 != 0 {
                        "Buffered Bytes"
                    } else {
                        "Bitfield"
                    },
                    indent = REPORT_INDENT
                );
                println!("{attributes_1}");
                println!("{attributes_2}");
            }
            _ => (),
        }
        i += 1 + bsize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_spaces() {
        assert_eq!(get_spaces(4, 10, LSUSB_DUMP_WIDTH), "           ");
        assert_eq!(get_spaces(24, 10, 20), " ");
        assert_eq!(get_spaces(2, 17, 20), " ");
        assert_eq!(get_spaces(17, 2, 20), " ");
        assert_eq!(get_spaces(16, 2, 20), "  ");
    }

    #[test]
    fn test_dump_value() {
        let bytes = [0x01; 32];
        let bytes_string = bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<String>>()
            .join(" ");
        // test no panic since is to stdout
        dump_value(bytes_string, "bmConfigured", 4, LSUSB_DUMP_WIDTH);
    }
}
