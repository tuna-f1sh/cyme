//! Originally based on [libusb list_devices.rs example](https://github.com/dcuddeback/libusb-rs/blob/master/examples/list_devices.rs), attempts to mimic lsusb output and provide cross-platform [`crate::system_profiler::SPUSBDataType`] getter

pub mod names {
    //! Port of names.c in usbutils that provides name lookups for USB data using udev, falling back to USB IDs repository.
    //!
    //! lsusb uses udev and the bundled hwdb (based on USB IDs) for name lookups. To attempt parity with lsusb, this module uses udev_hwdb if the feature is enabled, otherwise it will fall back to the USB IDs repository. Whilst they both get data from the same source, the bundled udev hwdb might be different due to release version/customisations.
    //!
    //! The function names match those found in the lsusb source code.
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

    /// Get name of [`usb_ids::VideoControl`] from id
    pub fn videoterminal(id: u16) -> Option<String> {
        usb_ids::VideoTerminal::from_id(id).map(|v| v.name().to_owned())
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
    //!
    //! The [lsusb source code](https://github.com/gregkh/usbutils/blob/master/lsusb.c) was used as a reference for a lot of the styling and content of the display module

    use crate::display::PrintSettings;
    use crate::error::{Error, ErrorKind};
    use crate::{system_profiler, usb};

    const TREE_LSUSB_BUS: &str = "/:  ";
    const TREE_LSUSB_DEVICE: &str = "|__ ";
    const TREE_LSUSB_SPACE: &str = "    ";

    const CAM_CTRL_NAMES: [&str; 22] = [
        "Scanning Mode",
        "Auto-Exposure Mode",
        "Auto-Exposure Priority",
        "Exposure Time (Absolute)",
        "Exposure Time (Relative)",
        "Focus (Absolute)",
        "Focus (Relative)",
        "Iris (Absolute)",
        "Iris (Relative)",
        "Zoom (Absolute)",
        "Zoom (Relative)",
        "PanTilt (Absolute)",
        "PanTilt (Relative)",
        "Roll (Absolute)",
        "Roll (Relative)",
        "Reserved",
        "Reserved",
        "Focus, Auto",
        "Privacy",
        "Focus, Simple",
        "Window",
        "Region of Interest",
    ];

    const CTRL_NAMES: [&str; 19] = [
        "Brightness",
        "Contrast",
        "Hue",
        "Saturation",
        "Sharpness",
        "Gamma",
        "White Balance Temperature",
        "White Balance Component",
        "Backlight Compensation",
        "Gain",
        "Power Line Frequency",
        "Hue, Auto",
        "White Balance Temperature, Auto",
        "White Balance Component, Auto",
        "Digital Multiplier",
        "Digital Multiplier Limit",
        "Analog Video Standard",
        "Analog Video Lock Status",
        "Contrast, Auto",
    ];

    const EN_CTRL_NAMES: [&str; 22] = [
        "Scanning Mode",
        "Auto-Exposure Mode",
        "Auto-Exposure Priority",
        "Exposure Time (Absolute)",
        "Exposure Time (Relative)",
        "Focus (Absolute)",
        "Focus (Relative)",
        "Iris (Absolute)",
        "Iris (Relative)",
        "Zoom (Absolute)",
        "Zoom (Relative)",
        "PanTilt (Absolute)",
        "PanTilt (Relative)",
        "Roll (Absolute)",
        "Roll (Relative)",
        "Reserved",
        "Reserved",
        "Focus, Auto",
        "Privacy",
        "Focus, Simple",
        "Window",
        "Region of Interest",
    ];

    const STD_NAMES: [&str; 6] = [
        "None",
        "NTSC - 525/60",
        "PAL - 625/50",
        "SECAM - 625/50",
        "NTSC - 625/50",
        "PAL - 525/60",
    ];

    const UAC2_INTERFACE_HEADER_BMCONTROLS: [&str; 1] = ["Legacy"];
    const UAC2_INPUT_TERMINAL_BMCONTROLS: [&str; 6] = [
        "Copy Protect",
        "Connector",
        "Overload",
        "Cluster",
        "Underflow",
        "Overflow",
    ];
    const UAC3_INPUT_TERMINAL_BMCONTROLS: [&str; 5] = [
        "Insertion",
        "Overload",
        "Underflow",
        "Overflow",
        "Underflow",
    ];
    const UAC2_OUTPUT_TERMINAL_BMCONTROLS: [&str; 5] = [
        "Copy Protect",
        "Connector",
        "Overload",
        "Underflow",
        "Overflow",
    ];
    const UAC3_OUTPUT_TERMINAL_BMCONTROLS: [&str; 4] =
        ["Insertion", "Overload", "Underflow", "Overflow"];
    const UAC2_AS_INTERFACE_BMCONTROLS: [&str; 2] =
        ["Active Alternate Setting", "Valid Alternate Setting"];
    const UAC3_AS_INTERFACE_BMCONTROLS: [&str; 3] = [
        "Active Alternate Setting",
        "Valid Alternate Setting",
        "Audio Data Format Control",
    ];
    const UAC2_AS_ISO_ENDPOINT_BMCONTROLS: [&str; 3] = ["Pitch", "Data Overrun", "Data Underrun"];
    const UAC2_MIXER_UNIT_BMCONTROLS: [&str; 4] = ["Cluster", "Underflow", "Overflow", "Overflow"];
    const UAC3_MIXER_UNIT_BMCONTROLS: [&str; 2] = ["Underflow", "Overflow"];
    const UAC2_SELECTOR_UNIT_BMCONTROLS: [&str; 1] = ["Selector"];
    const UAC1_FEATURE_UNIT_BMCONTROLS: [&str; 13] = [
        "Mute",
        "Volume",
        "Bass",
        "Mid",
        "Treble",
        "Graphic Equalizer",
        "Automatic Gain",
        "Delay",
        "Bass Boost",
        "Loudness",
        "Input gain",
        "Input gain pad",
        "Phase invert",
    ];
    const UAC2_EXTENSION_UNIT_BMCONTROLS: [&str; 4] =
        ["Enable", "Cluster", "Underflow", "Overflow"];
    const UAC3_EXTENSION_UNIT_BMCONTROLS: [&str; 2] = ["Underflow", "Overflow"];
    const UAC2_CLOCK_SOURCE_BMCONTROLS: [&str; 2] = ["Clock Frequency", "Clock Validity"];
    const UAC2_CLOCK_SELECTOR_BMCONTROLS: [&str; 1] = ["Clock Selector"];
    const UAC2_CLOCK_MULTIPLIER_BMCONTROLS: [&str; 2] = ["Clock Numerator", "Clock Denominator"];
    const UAC3_PROCESSING_UNIT_UP_DOWN_BMCONTROLS: [&str; 3] = ["Mode Select", "Underflow", "Overflow"];
    const UAC3_PROCESSING_UNIT_STEREO_EXTENDER_BMCONTROLS: [&str; 3] = ["Width", "Underflow", "Overflow"];
    const UAC3_PROCESSING_UNIT_MULTI_FUNC_BMCONTROLS: [&str; 2] = ["Underflow", "Overflow"];

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
                }

                print(&vec![device], true);
                return Ok(());
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
            class_name.unwrap_or(String::from("[unknown]"))
        );
        println!(
            "  bDeviceSubClass      {:3} {}",
            device.sub_class.unwrap_or(0),
            sub_class_name.unwrap_or(String::from("[unknown]"))
        );
        println!(
            "  bDeviceProtocol      {:3} {}",
            device.protocol.unwrap_or(0),
            protocol_name.unwrap_or_default()
        );
        println!("  bMaxPacketSize0      {:3}", device_extra.max_packet_size);
        println!(
            "  idVendor          0x{:04x} {}",
            device.vendor_id.unwrap_or(0),
            device_extra.vendor.as_ref().unwrap_or(&String::from("[unknown]"))
        );
        println!(
            "  idProduct         0x{:04x} {}",
            device.product_id.unwrap_or(0),
            device_extra.product_name.as_ref().unwrap_or(&String::from("[unknown]"))
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
            device.manufacturer.as_ref().unwrap_or(&String::from("[unknown]"))
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
        println!("    bLength              {:5}", config.length);
        println!("    bDescriptorType      {:5}", 2); // type 2 for configuration
        println!("    wTotalLength        0x{:04x}", config.total_length);
        println!("    bNumInterfaces       {:5}", config.interfaces.len());
        println!("    bConfigurationValue  {:5}", config.number);
        println!(
            "    iConfiguration       {:5} {}",
            config.string_index, config.name
        );
        println!(
            "    bmAttributes          0x{:02x}",
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
            "    MaxPower           {:5}{}",
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
                    usb::DescriptorType::Unknown(junk) | usb::DescriptorType::Junk(junk) => {
                        dump_unrecognised(junk, 4);
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
            interface_name.unwrap_or(String::from("[unknown]"))
        );
        println!(
            "      bInterfaceSubClass   {:3} {}",
            interface.sub_class,
            sub_class_name.unwrap_or(String::from("[unknown]"))
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
                    // Should only be Device or Interface as we mask out the rest
                    usb::DescriptorType::Device(cd) | usb::DescriptorType::Interface(cd) => {
                        match cd {
                            usb::ClassDescriptor::Hid(hidd) => dump_hid_device(hidd),
                            usb::ClassDescriptor::Ccid(ccid) => dump_ccid_desc(ccid),
                            usb::ClassDescriptor::Printer(pd) => dump_printer_desc(pd),
                            usb::ClassDescriptor::Communication(cd) => dump_comm_descriptor(cd, 6),
                            usb::ClassDescriptor::Midi(md, _) => dump_midistreaming_interface(md),
                            usb::ClassDescriptor::Video(vcd, p) => {
                                dump_videocontrol_interface(vcd, *p)
                            }
                            usb::ClassDescriptor::Generic(cc, gd) => match cc {
                                Some((usb::ClassCode::Audio, 1, p)) => {
                                    dump_audiocontrol_interface(gd, *p);
                                }
                                Some((usb::ClassCode::Audio, 2, p)) => {
                                    dump_audiostreaming_interface(gd, *p);
                                }
                                Some((usb::ClassCode::Audio, 3, _)) => {
                                    if let Ok(md) = usb::MidiDescriptor::try_from(gd.to_owned()) {
                                        dump_midistreaming_interface(&md);
                                    }
                                }
                                Some((usb::ClassCode::Video, 1, p)) => {
                                    if let Ok(vcd) = usb::UvcDescriptor::try_from(gd.to_owned()) {
                                        dump_videocontrol_interface(&vcd, *p);
                                    }
                                }
                                Some((usb::ClassCode::Video, 2, _)) => {
                                    dump_videostreaming_interface(gd);
                                }
                                Some((usb::ClassCode::ApplicationSpecificInterface, 1, _)) => {
                                    dump_dfu_interface(gd);
                                }
                                _ => {
                                    let junk = Vec::from(cd.to_owned());
                                    dump_unrecognised(&junk, 6);
                                }
                            },
                        }
                    }
                    usb::DescriptorType::Unknown(junk) | usb::DescriptorType::Junk(junk) => {
                        dump_unrecognised(junk, 6);
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
            "        bEndpointAddress    0x{:02x} EP {} {}",
            endpoint.address.address,
            endpoint.address.number,
            endpoint.address.direction.to_string().to_uppercase()
        );
        println!("        bmAttributes:        {:3}", endpoint.attributes());
        println!(
            "          Transfer Type          {:?}",
            endpoint.transfer_type
        );
        println!("          Sync Type              {:?}", endpoint.sync_type);
        println!("          Usage Type             {:?}", endpoint.usage_type);
        println!(
            "        wMaxPacketSize    0x{:04x} {} bytes",
            endpoint.max_packet_size,
            endpoint.max_packet_string()
        );
        println!("        bInterval            {:3}", endpoint.interval);

        // dump extra descriptors
        // kind of messy but it's out lsusb does it
        if let Some(dt_vec) = &endpoint.extra {
            for dt in dt_vec {
                match dt {
                    usb::DescriptorType::Endpoint(usb::ClassDescriptor::Generic(cc, gd)) => {
                        match cc {
                            Some((usb::ClassCode::Audio, 2, p)) => {
                                dump_audiostreaming_endpoint(gd, *p);
                            }
                            Some((usb::ClassCode::Audio, 3, _)) => {
                                dump_midistreaming_endpoint(gd);
                            }
                            _ => (),
                        }
                    }
                    // Misplaced descriptors
                    usb::DescriptorType::Device(cd) => match cd {
                        usb::ClassDescriptor::Ccid(ccid) => {
                            dump_ccid_desc(ccid);
                        }
                        _ => {
                            println!(
                                "        DEVICE CLASS: {}",
                                Vec::<u8>::from(cd.to_owned())
                                    .iter()
                                    .map(|b| format!("{:02x}", b))
                                    .collect::<Vec<String>>()
                                    .join(" ")
                            );
                        }
                    },
                    usb::DescriptorType::Interface(cd) => match cd {
                        usb::ClassDescriptor::Generic(cc, gd) => match cc {
                            Some((usb::ClassCode::CDCData, _, _))
                            | Some((usb::ClassCode::CDCCommunications, _, _)) => {
                                if let Ok(cd) = gd.to_owned().try_into() {
                                    dump_comm_descriptor(&cd, 6)
                                }
                            }
                            Some((usb::ClassCode::MassStorage, _, _)) => {
                                dump_pipe_desc(gd);
                            }
                            _ => {
                                println!(
                                    "        INTERFACE CLASS: {}",
                                    Vec::<u8>::from(cd.to_owned())
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join(" ")
                                );
                            }
                        },
                        usb::ClassDescriptor::Communication(cd) => dump_comm_descriptor(cd, 6),
                        _ => {
                            println!(
                                "        INTERFACE CLASS: {}",
                                Vec::<u8>::from(cd.to_owned())
                                    .iter()
                                    .map(|b| format!("{:02x}", b))
                                    .collect::<Vec<String>>()
                                    .join(" ")
                            );
                        }
                    },
                    usb::DescriptorType::InterfaceAssociation(iad) => {
                        dump_interface_association(iad);
                    }
                    usb::DescriptorType::SsEndpointCompanion(ss) => {
                        println!("        bMaxBurst {:>14}", ss.max_burst);
                        match endpoint.transfer_type {
                            usb::TransferType::Bulk => {
                                if ss.attributes & 0x1f != 0 {
                                    println!("        MaxStreams {:>13}", 1 << ss.attributes);
                                }
                            }
                            usb::TransferType::Isochronous => {
                                if ss.attributes & 0x03 != 0 {
                                    println!("        Mult {:>19}", ss.attributes & 0x3);
                                }
                            }
                            _ => (),
                        }
                    }
                    usb::DescriptorType::Unknown(junk) | usb::DescriptorType::Junk(junk) => {
                        dump_unrecognised(junk, 8);
                    }
                    _ => (),
                }
            }
        }
    }

    fn dump_junk(extra: &[u8], indent: usize, reported_len: usize, expected_len: usize) {
        if reported_len > expected_len && extra.len() >= reported_len {
            println!(
                "{:^indent$}junk at descriptor end: {}",
                "",
                extra[expected_len..reported_len]
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join(" ")
            )
        }
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

    fn dump_audiostreaming_endpoint(gd: &usb::GenericDescriptor, protocol: u8) {
        // audio streaming endpoint is only EP_GENERAL
        let subtype_string = match gd.descriptor_subtype {
            1 => "EP_GENERAL",
            // lowercase in lsusb
            _ => "invalid",
        };
        println!("        AudioStreaming Endpoint Descriptor:");
        println!("          bLength              {:3}", gd.length);
        println!("          bDescriptorType      {:3}", gd.descriptor_type);
        println!(
            "          bDescriptorSubType   {:3} ({})",
            gd.descriptor_subtype, subtype_string
        );

        if let Some(data) = gd.data.as_ref() {
            let uacp = usb::UacProtocol::from(protocol);
            if let Ok(uaci) =
                usb::UacInterfaceDescriptor::from_uac_as_iso_data_endpoint(&uacp, data)
            {
                dump_audio_subtype(&uaci, &uacp, 5);
            }
        }
    }

    fn dump_midistreaming_endpoint(gd: &usb::GenericDescriptor) {
        let subtype_string = match gd.descriptor_subtype {
            2 => "GENERAL",
            _ => "Invalid",
        };

        println!("        MIDIStreaming Endpoint Descriptor:");
        println!("          bLength              {:5}", gd.length);
        println!("          bDescriptorType      {:5}", gd.descriptor_type);
        println!(
            "          wDescriptorSubType   {:5} {}",
            gd.descriptor_subtype, subtype_string
        );

        if let Some(data) = gd.data.as_ref() {
            if data.len() >= 2 {
                let num_jacks: usize = data[0] as usize;
                println!("          bNumEmbMIDIJack      {:5}", num_jacks);
                for (i, jack_id) in data[1..num_jacks].iter().enumerate() {
                    println!("          baAssocJackID({:2})   {:3}", i, jack_id);
                }
            }
            dump_junk(data, 8, gd.expected_data_length(), 1 + data[0] as usize);
        }
    }

    fn dump_ccid_desc(ccid: &usb::CcidDescriptor) {
        println!("      ChipCard Interface Descriptor:");
        println!("        bLength              {:3}", ccid.length);
        println!("        bDescriptorType      {:3}", ccid.descriptor_type);
        println!("        bcdCCID              {}", ccid.version);
        if ccid.version.major() != 1 || ccid.version.minor() != 0 {
            println!("  (Warning: Only accurate for version 1.0)");
        }

        println!("        bMaxSlotIndex        {:3}", ccid.max_slot_index);
        print!("        bVoltageSupport      {:3} ", ccid.voltage_support);
        if ccid.voltage_support & 0x01 != 0 {
            print!("5.0V ");
        }
        if ccid.voltage_support & 0x02 != 0 {
            print!("3.0V ");
        }
        if ccid.voltage_support & 0x04 != 0 {
            print!("1.8V ");
        }
        println!();

        print!("        dwProtocols           {:3} ", ccid.protocols);
        if ccid.protocols & 0x01 != 0 {
            print!("T=0 ");
        }
        if ccid.protocols & 0x02 != 0 {
            print!("T=1 ");
        }
        if ccid.protocols & !0x03 != 0 {
            print!(" (Invalid values detected)");
        }
        println!();

        println!("        dwDefaultClock        {:3}", ccid.default_clock);
        println!("        dwMaxiumumClock       {:3}", ccid.max_clock);
        println!(
            "        bNumClockSupported    {:3}",
            ccid.num_clock_supported
        );
        println!("        dwDataRate        {:5} bps", ccid.data_rate);
        println!("        dwMaxDataRate     {:5} bps", ccid.max_data_rate);
        println!(
            "        bNumDataRatesSupp.    {:3}",
            ccid.num_data_rates_supp
        );
        println!("        dwMaxIFSD             {:3}", ccid.max_ifsd);
        print!("        dwSyncProtocols       {:08X} ", ccid.sync_protocols);
        if ccid.sync_protocols & 0x01 != 0 {
            print!(" 2-wire");
        }
        if ccid.sync_protocols & 0x02 != 0 {
            print!(" 3-wire");
        }
        if ccid.sync_protocols & 0x04 != 0 {
            print!(" I2C");
        }
        println!();

        print!("        dwMechanical          {:08X} ", ccid.mechanical);
        if ccid.mechanical & 0x01 != 0 {
            print!(" accept");
        }
        if ccid.mechanical & 0x02 != 0 {
            print!(" eject");
        }
        if ccid.mechanical & 0x04 != 0 {
            print!(" capture");
        }
        if ccid.mechanical & 0x08 != 0 {
            print!(" lock");
        }
        println!();

        println!("        dwFeatures            {:08X}", ccid.features);
        if ccid.features & 0x0002 != 0 {
            println!("          Auto configuration based on ATR ");
        }
        if ccid.features & 0x0004 != 0 {
            println!("          Auto activation on insert ");
        }
        if ccid.features & 0x0008 != 0 {
            println!("          Auto voltage selection ");
        }
        if ccid.features & 0x0010 != 0 {
            println!("          Auto clock change ");
        }
        if ccid.features & 0x0020 != 0 {
            println!("          Auto baud rate change ");
        }
        if ccid.features & 0x0040 != 0 {
            println!("          Auto parameter negotiation made by CCID ");
        } else if ccid.features & 0x0080 != 0 {
            println!("          Auto PPS made by CCID ");
        } else if (ccid.features & (0x0040 | 0x0080)) != 0 {
            println!("        WARNING: conflicting negotiation features");
        }
        if ccid.features & 0x0100 != 0 {
            println!("          CCID can set ICC in clock stop mode ");
        }
        if ccid.features & 0x0200 != 0 {
            println!("          NAD value other than 0x00 accepted ");
        }
        if ccid.features & 0x0400 != 0 {
            println!("          Auto IFSD exchange ");
        }
        if ccid.features & 0x00010000 != 0 {
            println!("          TPDU level exchange ");
        } else if ccid.features & 0x00020000 != 0 {
            println!("          Short APDU level exchange ");
        } else if ccid.features & 0x00040000 != 0 {
            println!("          Short and extended APDU level exchange ");
        } else if ccid.features & 0x00070000 != 0 {
            println!("        WARNING: conflicting exchange levels");
        }

        println!("        dwMaxCCIDMsgLen     {:3}", ccid.max_ccid_msg_len);
        print!("        bClassGetResponse    ");
        if ccid.class_get_response == 0xff {
            println!("echo");
        } else {
            println!("  {:02X}", ccid.class_get_response);
        }

        print!("        bClassEnvelope       ");
        if ccid.class_envelope == 0xff {
            println!("echo");
        } else {
            println!("  {:02X}", ccid.class_envelope);
        }

        print!("        wlcdLayout           ");
        if ccid.lcd_layout == (0, 0) {
            println!("none");
        } else {
            println!("{} cols {} lines", ccid.lcd_layout.0, ccid.lcd_layout.1);
        }

        print!("        bPINSupport         {:3} ", ccid.pin_support);
        if ccid.pin_support & 1 != 0 {
            print!(" verification");
        }
        if ccid.pin_support & 2 != 0 {
            print!(" modification");
        }
        println!();

        println!("        bMaxCCIDBusySlots   {:3}", ccid.max_ccid_busy_slots);
    }

    fn dump_printer_desc(pd: &usb::PrinterDescriptor) {
        println!("        IPP Printer Descriptor:");
        println!("          bLength              {:3}", pd.length);
        println!("          bDescriptorType      {:3}", pd.descriptor_type);
        println!("          bcdReleaseNumber     {:3}", pd.release_number);
        println!("          bcdNumDescriptors    {:3}", pd.descriptors.len());

        for desc in &pd.descriptors {
            // basic capabilities
            if desc.descriptor_type == 0x00 {
                println!(
                    "            iIPPVersionsSupported {:3}",
                    desc.versions_supported
                );
                if let Some(uuid) = &desc.uuid_string {
                    println!(
                        "            iIPPPrinterUUID       {:3} {}",
                        desc.uuid_string_index, uuid
                    );
                } else {
                    println!(
                        "            iIPPPrinterUUID       {:3}",
                        desc.uuid_string_index
                    );
                }
                print!(
                    "            wBasicCapabilities   0x{:04x} ",
                    desc.capabilities
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
                if (desc.capabilities & 0x0060) != 0 {
                    print!(" No-Auth");
                } else if (desc.capabilities & 0x0060) != 0x20 {
                    print!(" Username-Auth");
                } else if (desc.capabilities & 0x0060) != 0x40 {
                    print!(" Reserved-Auth");
                } else if (desc.capabilities & 0x0060) != 0x60 {
                    print!(" Negotiable-Auth");
                }
                println!();
            // vendor specific
            } else {
                println!(
                    "            UnknownCapabilities   {:3} {:3}\n",
                    desc.descriptor_type, desc.length
                );
            }
        }
    }

    fn dump_array<T: std::fmt::Display>(
        array: &[T],
        field_name: &str,
        indent: usize,
        width: usize,
    ) {
        for (i, b) in array.iter().enumerate() {
            dump_value(b, &format!("{}({:2})", field_name, i), indent, width);
        }
    }

    fn dump_bitmap_array<T: std::fmt::LowerHex + Into<u64> + Copy>(
        array: &[T],
        field_name: &str,
        indent: usize,
        width: usize,
    ) {
        for (i, b) in array.iter().enumerate() {
            dump_hex(*b, &format!("{}({:2})", field_name, i), indent, width);
        }
    }

    fn dump_value<T: std::fmt::Display>(value: T, field_name: &str, indent: usize, width: usize) {
        let value = value.to_string();
        let spaces = " ".repeat(
            (width - value.len())
                .saturating_sub(field_name.len())
                .max(1),
        );
        println!(
            "{:indent$}{}{}{}",
            "",
            field_name,
            spaces,
            value,
            indent = indent
        );
    }

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
        dump_value(&hex_value, field_name, indent, width);
    }

    fn dump_name<T: std::fmt::Display>(
        value: T,
        names_f: fn(T) -> Option<String>,
        field_name: &str,
        indent: usize,
        width: usize,
    ) {
        let value_string = value.to_string();
        let spaces = " ".repeat(
            (width - value_string.len())
                .saturating_sub(field_name.len())
                .max(1),
        );
        let dump = format!(
            "{:indent$}{}{}{}",
            "",
            field_name,
            spaces,
            value_string,
            indent = indent
        );
        if let Some(name) = names_f(value) {
            println!("{} {}", dump, name);
        }
    }

    /// Dumps the value and the string representation of the value to the right of width
    fn dump_number_string<T: std::fmt::Display, S: std::fmt::Display>(
        value: T,
        field_name: &str,
        value_string: S,
        indent: usize,
        width: usize,
    ) {
        let value = value.to_string();
        let spaces = " ".repeat(
            (width - value.len())
                .saturating_sub(field_name.len())
                .max(1),
        );
        println!(
            "{:indent$}{}{}{} {}",
            "",
            field_name,
            spaces,
            value,
            value_string,
            indent = indent
        );
    }

    /// Dumps strings matching the bits set in `bitmap` using `strings_f` function
    fn dump_bitmap_strings<T>(
        bitmap: T,
        strings_f: fn(usize) -> Option<&'static str>,
        indent: usize,
    ) where
        T: std::fmt::Display + std::fmt::LowerHex + Copy + Into<u64>,
    {
        let bitmap_u64: u64 = bitmap.into();
        let num_bits = std::mem::size_of::<T>() * 8;
        for index in 0..num_bits {
            if (bitmap_u64 >> index) & 0x1 != 0 {
                if let Some(string) = strings_f(index) {
                    println!("{:indent$}{}", "", string, indent = indent * 2);
                }
            }
        }
    }

    fn dump_bmcontrols<T: Into<u32>>(
        controls: T,
        control_descriptions: &[&'static str],
        desc_type: &usb::ControlType,
        indent: usize,
    ) {
        let controls: u32 = controls.into();
        for (index, control) in control_descriptions.iter().enumerate() {
            match desc_type {
                usb::ControlType::BmControl1 => {
                    if (controls >> index) & 0x1 != 0 {
                        println!("{:indent$}{} Control", "", control, indent = indent * 2);
                    }
                }
                usb::ControlType::BmControl2 => {
                    println!(
                        "{:indent$}{} Control ({})",
                        "",
                        control,
                        usb::ControlSetting::from(((controls >> (index * 2)) & 0x3) as u8),
                        indent = indent * 2
                    )
                }
            }
        }
    }

    fn dump_bmcontrols_array<T: Into<u32> + std::fmt::Display + Copy>(
        field_name: &str,
        controls: &[T],
        control_descriptions: &[&'static str],
        desc_type: &usb::ControlType,
        indent: usize,
        width: usize,
    ) {
        for (i, control) in controls.iter().enumerate() {
            let control = control.to_owned();
            let control: u32 = control.into();
            dump_value(control, &format!("{}({:2})", field_name, i), indent * 2, width);
            dump_bmcontrols(control, control_descriptions, &desc_type, indent + 1);
        }
    }

    fn dump_audio_mixer_unit1(mixer_unit: &usb::AudioMixerUnit1, indent: usize, width: usize) {
        dump_value(mixer_unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(mixer_unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&mixer_unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(mixer_unit.nr_channels, "bNrChannels", indent * 2, width);
        dump_hex(
            mixer_unit.channel_config,
            "wChannelConfig",
            indent * 2,
            width,
        );
        let channel_names = usb::UacInterfaceDescriptor::get_channel_names(
            &usb::UacProtocol::Uac1,
            mixer_unit.channel_config as u32,
        );
        for name in channel_names.iter() {
            println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
        }
        dump_value(mixer_unit.channel_names, "iChannelNames", indent * 2, width);
        dump_bitmap_array(&mixer_unit.controls, "bmControls", indent * 2, width);
        dump_value(mixer_unit.mixer, "iMixer", indent * 2, width);
    }

    fn dump_audio_mixer_unit2(mixer_unit: &usb::AudioMixerUnit2, indent: usize, width: usize) {
        dump_value(mixer_unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(mixer_unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&mixer_unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(mixer_unit.nr_channels, "bNrChannels", indent * 2, width);
        dump_hex(
            mixer_unit.channel_config,
            "bmChannelConfig",
            indent * 2,
            width,
        );
        let channel_names = usb::UacInterfaceDescriptor::get_channel_names(
            &usb::UacProtocol::Uac2,
            mixer_unit.channel_config,
        );
        for name in channel_names.iter() {
            println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
        }
        dump_value(mixer_unit.channel_names, "iChannelNames", indent * 2, width);
        dump_bitmap_array(
            &mixer_unit.mixer_controls,
            "bmMixerControls",
            indent * 2,
            width,
        );
        dump_hex(mixer_unit.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            mixer_unit.controls as u32,
            &UAC2_MIXER_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(mixer_unit.mixer, "iMixer", indent * 2, width);
    }

    fn dump_audio_mixer_unit3(mixer_unit: &usb::AudioMixerUnit3, indent: usize, width: usize) {
        dump_value(mixer_unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(mixer_unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&mixer_unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(
            mixer_unit.cluster_descr_id,
            "wClusterDescrID",
            indent * 2,
            width,
        );
        dump_bitmap_array(
            &mixer_unit.mixer_controls,
            "bmMixerControls",
            indent * 2,
            width,
        );
        dump_hex(mixer_unit.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            mixer_unit.controls,
            &UAC3_MIXER_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(
            mixer_unit.mixer_descr_str,
            "wMixerDescrStr",
            indent * 2,
            width,
        );
    }

    fn dump_audio_power_domain(power_domain: &usb::AudioPowerDomain, indent: usize, width: usize) {
        dump_value(
            power_domain.power_domain_id,
            "bPowerDomainID",
            indent * 2,
            width,
        );
        dump_value(
            power_domain.recovery_time_1,
            "waRecoveryTime(1)",
            indent * 2,
            width,
        );
        dump_value(
            power_domain.recovery_time_2,
            "waRecoveryTime(2)",
            indent * 2,
            width,
        );
        dump_value(power_domain.nr_entities, "bNrEntities", indent * 2, width);
        dump_array(&power_domain.entity_ids, "baEntityID", indent * 2, width);
        dump_value(
            power_domain.domain_descr_str,
            "wPDomainDescrStr",
            indent * 2,
            width,
        );
    }

    fn dump_audio_selector_unit1(
        selector_unit: &usb::AudioSelectorUnit1,
        indent: usize,
        width: usize,
    ) {
        dump_value(selector_unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(selector_unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&selector_unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(selector_unit.selector_index, "iSelector", indent * 2, width);
    }

    fn dump_audio_selector_unit2(
        selector_unit: &usb::AudioSelectorUnit2,
        indent: usize,
        width: usize,
    ) {
        dump_value(selector_unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(selector_unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&selector_unit.source_ids, "baSourceID", indent * 2, width);
        dump_hex(selector_unit.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            selector_unit.controls,
            &UAC2_SELECTOR_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(selector_unit.selector_index, "iSelector", indent * 2, width);
    }

    fn dump_audio_selector_unit3(
        selector_unit: &usb::AudioSelectorUnit3,
        indent: usize,
        width: usize,
    ) {
        dump_value(selector_unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(selector_unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&selector_unit.source_ids, "baSourceID", indent * 2, width);
        dump_hex(selector_unit.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            selector_unit.controls,
            &UAC2_SELECTOR_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(
            selector_unit.selector_descr_str,
            "wSelectorDescrStr",
            indent * 2,
            width,
        );
    }

    /// Dumps the contents of a UAC1 Processing Unit Descriptor
    fn dump_audio_processing_unit1(unit: &usb::AudioProcessingUnit1, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_number_string(
            unit.process_type,
            "wProcessType",
            &unit.processing_type(),
            indent * 2,
            width,
        );
        dump_value(unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(unit.nr_channels, "bNrChannels", indent * 2, width);
        dump_hex(unit.channel_config, "wChannelConfig", indent * 2, width);
        let channel_names = usb::UacInterfaceDescriptor::get_channel_names(&usb::UacProtocol::Uac1, unit.channel_config as u32);
        for name in channel_names.iter() {
            println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
        }
        dump_value(unit.channel_names_index, "iChannelNames", indent * 2, width);
        dump_value(unit.control_size, "bControlSize", indent * 2, width);
        dump_bitmap_array(&unit.controls, "bmControls", indent * 2, width);
        dump_value(unit.processing_index, "iProcessing", indent * 2, width);
        if let Some(ref specific) = unit.specific {
            dump_value(specific.nr_modes, "bNrModes", indent * 2, width);
            dump_bitmap_array(&specific.modes, "waModes", indent * 2, width);
        }
    }

    /// Dumps the contents of a UAC2 Processing Unit Descriptor
    fn dump_audio_processing_unit2(unit: &usb::AudioProcessingUnit2, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_number_string(
            unit.process_type,
            "wProcessType",
            &unit.processing_type(),
            indent * 2,
            width,
        );
        dump_value(unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(unit.nr_channels, "bNrChannels", indent * 2, width);
        dump_hex(unit.channel_config, "bmChannelConfig", indent * 2, width);
        let channel_names = usb::UacInterfaceDescriptor::get_channel_names(&usb::UacProtocol::Uac2, unit.channel_config as u32);
        for name in channel_names.iter() {
            println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
        }
        dump_value(unit.channel_names_index, "iChannelNames", indent * 2, width);
        dump_value(unit.controls, "bmControls", indent * 2, width);
        dump_value(unit.processing_index, "iProcessing", indent * 2, width);
        if let Some(ref specific) = unit.specific {
            match specific {
                usb::AudioProcessingUnit2Specific::UpDownMix(up_down_mix) => {
                    dump_value(up_down_mix.nr_modes, "bNrModes", indent * 2, width);
                    dump_bitmap_array(&up_down_mix.modes, "daModes", indent * 2, width);
                }
                usb::AudioProcessingUnit2Specific::DolbyPrologic(dolby_prologic) => {
                    dump_value(dolby_prologic.nr_modes, "bNrModes", indent * 2, width);
                    dump_bitmap_array(&dolby_prologic.modes, "daModes", indent * 2, width);
                }
            }
        }
    }

    /// Dumps the contents of a UAC3 Processing Unit Descriptor
    fn dump_audio_processing_unit3(unit: &usb::AudioProcessingUnit3, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_number_string(
            unit.process_type,
            "wProcessType",
            &unit.processing_type(),
            indent * 2,
            width,
        );
        dump_value(unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(unit.processing_descr_str, "wProcessingDescrStr", indent * 2, width);
        if let Some(ref specific) = unit.specific {
            match specific {
                usb::AudioProcessingUnit3Specific::UpDownMix(up_down_mix) => {
                    dump_hex(up_down_mix.controls, "bmControls", indent * 2, width);
                    dump_bmcontrols(up_down_mix.controls, &UAC3_PROCESSING_UNIT_UP_DOWN_BMCONTROLS, &usb::ControlType::BmControl2, indent + 1);
                    dump_value(up_down_mix.nr_modes, "bNrModes", indent * 2, width);
                    dump_array(&up_down_mix.cluster_descr_ids, "waClusterDescrID", indent * 2, width);
                }
                usb::AudioProcessingUnit3Specific::StereoExtender(stereo_extender) => {
                    dump_hex(stereo_extender.controls, "bmControls", indent * 2, width);
                    dump_bmcontrols(stereo_extender.controls, &UAC3_PROCESSING_UNIT_STEREO_EXTENDER_BMCONTROLS, &usb::ControlType::BmControl2, indent + 1);
                }
                usb::AudioProcessingUnit3Specific::MultiFunction(multi_function) => {
                    dump_hex(multi_function.controls, "bmControls", indent * 2, width);
                    dump_bmcontrols(multi_function.controls, &UAC3_PROCESSING_UNIT_MULTI_FUNC_BMCONTROLS, &usb::ControlType::BmControl2, indent + 1);
                    dump_value(multi_function.cluster_descr_id, "wClusterDescrID", indent * 2, width);
                    dump_value(multi_function.algorithms, "bmAlgorithms", indent * 2, width);
                    if let Some(ref algorithms) = unit.algorithms() {
                        for algorithm in algorithms.iter() {
                            println!("{:indent$}{}", "", algorithm, indent = (indent + 1) * 2);
                        }
                    }
                }
            }
        }
    }

    /// Dumps the contents of a UAC2 Effect Unit Descriptor
    fn dump_audio_effect_unit2(unit: &usb::AudioEffectUnit2, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(unit.effect_type, "wEffectType", indent * 2, width);
        dump_value(unit.source_id, "bSourceID", indent * 2, width);
        dump_bitmap_array(&unit.controls, "bmaControls", indent * 2, width);
        dump_value(unit.effect_index, "iEffects", indent * 2, width);
    }

    /// Dumps the contents of a UAC3 Effect Unit Descriptor
    fn dump_audio_effect_unit3(unit: &usb::AudioEffectUnit3, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(unit.effect_type, "wEffectType", indent * 2, width);
        dump_value(unit.source_id, "bSourceID", indent * 2, width);
        dump_bitmap_array(&unit.controls, "bmaControls", indent * 2, width);
        dump_value(unit.effect_descr_str, "wEffectsDescrStr", indent * 2, width);
    }

    /// Dumps the contents of a UAC1 Feature Unit Descriptor
    fn dump_audio_feature_unit1(unit: &usb::AudioFeatureUnit1, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(unit.source_id, "bSourceID", indent * 2, width);
        dump_value(unit.control_size, "bControlSize", indent * 2, width);
        dump_bmcontrols_array(
            "bmaControls",
            &unit.controls,
            &UAC1_FEATURE_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl1,
            indent,
            width,
        );
        dump_value(unit.feature_index, "iFeature", indent * 2, width);
    }

    /// Dumps the contents of a UAC2 Feature Unit Descriptor
    fn dump_audio_feature_unit2(unit: &usb::AudioFeatureUnit2, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(unit.source_id, "bSourceID", indent * 2, width);
        dump_bmcontrols_array(
            "bmaControls",
            &unit.controls,
            &UAC1_FEATURE_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl1,
            indent,
            width,
        );
        dump_value(unit.feature_index, "iFeature", indent * 2, width);
    }

    /// Dumps the contents of a UAC3 Feature Unit Descriptor
    fn dump_audio_feature_unit3(unit: &usb::AudioFeatureUnit3, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(unit.source_id, "bSourceID", indent * 2, width);
        dump_bmcontrols_array(
            "bmaControls",
            &unit.controls,
            &UAC1_FEATURE_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl1,
            indent,
            width,
        );
        dump_value(unit.feature_descr_str, "wFeatureDescrStr", indent * 2, width);
    }


    /// Dumps the contents of a UAC1 Extension Unit Descriptor
    fn dump_audio_extension_unit1(unit: &usb::AudioExtensionUnit1, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(unit.extension_code, "wExtensionCode", indent * 2, width);
        dump_value(unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(unit.nr_channels, "bNrChannels", indent * 2, width);
        dump_hex(unit.channel_config, "wChannelConfig", indent * 2, width);
        let channel_names = usb::UacInterfaceDescriptor::get_channel_names(
            &usb::UacProtocol::Uac1,
            unit.channel_config as u32,
        );
        for name in channel_names.iter() {
            println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
        }
        dump_value(unit.channel_names_index, "iChannelNames", indent * 2, width);
        dump_value(unit.control_size, "bControlSize", indent * 2, width);
        dump_bitmap_array(&unit.controls, "bmControls", indent * 2, width);
        dump_value(unit.extension_index, "iExtension", indent * 2, width);
    }

    /// Dumps the contents of a UAC2 Extension Unit Descriptor
    fn dump_audio_extension_unit2(unit: &usb::AudioExtensionUnit2, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(unit.extension_code, "wExtensionCode", indent * 2, width);
        dump_value(unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(unit.nr_channels, "bNrChannels", indent * 2, width);
        dump_hex(unit.channel_config, "bmChannelConfig", indent * 2, width);
        let channel_names = usb::UacInterfaceDescriptor::get_channel_names(
            &usb::UacProtocol::Uac2,
            unit.channel_config,
        );
        for name in channel_names.iter() {
            println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
        }
        dump_value(unit.channel_names_index, "iChannelNames", indent * 2, width);
        dump_hex(unit.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            unit.controls,
            &UAC2_EXTENSION_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(unit.extension_index, "iExtension", indent * 2, width);
    }

    /// Dumps the contents of a UAC3 Extension Unit Descriptor
    fn dump_audio_extension_unit3(unit: &usb::AudioExtensionUnit3, indent: usize, width: usize) {
        dump_value(unit.unit_id, "bUnitID", indent * 2, width);
        dump_value(unit.extension_code, "wExtensionCode", indent * 2, width);
        dump_value(unit.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&unit.source_ids, "baSourceID", indent * 2, width);
        dump_value(
            unit.extension_descr_str,
            "wExtensionDescrStr",
            indent * 2,
            width,
        );
        dump_hex(unit.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            unit.controls,
            &UAC3_EXTENSION_UNIT_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(unit.cluster_descr_id, "wClusterDescrID", indent * 2, width);
    }

    /// Dumps the contents of a UAC2 Clock Source Descriptor
    fn dump_audio_clock_source2(source: &usb::AudioClockSource2, indent: usize, width: usize) {
        let uac2_clk_src_bmattr = |index: usize| -> Option<&'static str> {
            match index {
                0 => Some("External"),
                1 => Some("Internal fixed"),
                2 => Some("Internal variable"),
                3 => Some("Internal programmable"),
                _ => None,
            }
        };

        dump_value(source.clock_id, "bClockID", indent * 2, width);
        dump_hex(source.attributes, "bmAttributes", indent * 2, width);
        dump_bitmap_strings(source.attributes, uac2_clk_src_bmattr, indent + 1);
        dump_hex(source.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            source.controls,
            &UAC2_CLOCK_SOURCE_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(source.assoc_terminal, "bAssocTerminal", indent * 2, width);
        dump_value(source.clock_source_index, "iClockSource", indent * 2, width);
    }

    /// Dumps the contents of a UAC3 Clock Source Descriptor
    fn dump_audio_clock_source3(source: &usb::AudioClockSource3, indent: usize, width: usize) {
        let uac3_clk_src_bmattr = |index: usize| -> Option<&'static str> {
            match index {
                0 => Some("External"),
                1 => Some("Internal"),
                2 => Some("(asynchronous)"),
                3 => Some("(synchronized to SOF)"),
                _ => None,
            }
        };

        dump_value(source.clock_id, "bClockID", indent * 2, width);
        dump_hex(source.attributes, "bmAttributes", indent * 2, width);
        dump_bitmap_strings(source.attributes, uac3_clk_src_bmattr, indent + 1);
        dump_hex(source.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            source.controls,
            &UAC2_CLOCK_SOURCE_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(
            source.reference_terminal,
            "bReferenceTerminal",
            indent * 2,
            width,
        );
        dump_value(
            source.clock_source_str,
            "wClockSourceStr",
            indent * 2,
            width,
        );
    }

    /// Dumps the contents of a UAC2 Clock Selector Descriptor
    fn dump_audio_clock_selector2(
        selector: &usb::AudioClockSelector2,
        indent: usize,
        width: usize,
    ) {
        dump_value(selector.clock_id, "bClockID", indent * 2, width);
        dump_value(selector.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&selector.csource_ids, "baCSourceID", indent * 2, width);
        dump_hex(selector.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            selector.controls,
            &UAC2_CLOCK_SELECTOR_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(
            selector.clock_selector_index,
            "iClockSelector",
            indent * 2,
            width,
        );
    }

    /// Dumps the contents of a UAC3 Clock Selector Descriptor
    fn dump_audio_clock_selector3(
        selector: &usb::AudioClockSelector3,
        indent: usize,
        width: usize,
    ) {
        dump_value(selector.clock_id, "bClockID", indent * 2, width);
        dump_value(selector.nr_in_pins, "bNrInPins", indent * 2, width);
        dump_array(&selector.csource_ids, "baCSourceID", indent * 2, width);
        dump_hex(selector.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            selector.controls,
            &UAC2_CLOCK_SELECTOR_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(
            selector.cselector_descr_str,
            "wCSelectorDescrStr",
            indent * 2,
            width,
        );
    }

    /// Dumps the contents of a UAC2 Clock Multiplier Descriptor
    fn dump_audio_clock_multiplier2(
        multiplier: &usb::AudioClockMultiplier2,
        indent: usize,
        width: usize,
    ) {
        dump_value(multiplier.clock_id, "bClockID", indent * 2, width);
        dump_value(multiplier.csource_id, "bCSourceID", indent * 2, width);
        dump_hex(multiplier.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            multiplier.controls,
            &UAC2_CLOCK_MULTIPLIER_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(
            multiplier.clock_multiplier_index,
            "iClockMultiplier",
            indent * 2,
            width,
        );
    }

    /// Dumps the contents of a UAC3 Clock Multiplier Descriptor
    fn dump_audio_clock_multiplier3(
        multiplier: &usb::AudioClockMultiplier3,
        indent: usize,
        width: usize,
    ) {
        dump_value(multiplier.clock_id, "bClockID", indent * 2, width);
        dump_value(multiplier.csource_id, "bCSourceID", indent * 2, width);
        dump_hex(multiplier.controls, "bmControls", indent * 2, width);
        dump_bmcontrols(
            multiplier.controls,
            &UAC2_CLOCK_MULTIPLIER_BMCONTROLS,
            &usb::ControlType::BmControl2,
            indent + 1,
        );
        dump_value(
            multiplier.cmultiplier_descr_str,
            "wCMultiplierDescrStr",
            indent * 2,
            width,
        );
    }

    fn dump_audio_sample_rate_converter2(
        converter: &usb::AudioSampleRateConverter2,
        indent: usize,
        width: usize,
    ) {
        dump_value(converter.unit_id, "bUnitID", indent * 2, width);
        dump_value(converter.source_id, "bSourceID", indent * 2, width);
        dump_value(converter.csource_in_id, "bCSourceInID", indent * 2, width);
        dump_value(converter.csource_out_id, "bCSourceOutID", indent * 2, width);
        dump_value(converter.src_index, "iSRC", indent * 2, width);
    }

    fn dump_audio_sample_rate_converter3(
        converter: &usb::AudioSampleRateConverter3,
        indent: usize,
        width: usize,
    ) {
        dump_value(converter.unit_id, "bUnitID", indent * 2, width);
        dump_value(converter.source_id, "bSourceID", indent * 2, width);
        dump_value(converter.csource_in_id, "bCSourceInID", indent * 2, width);
        dump_value(converter.csource_out_id, "bCSourceOutID", indent * 2, width);
        dump_value(converter.src_descr_str, "wSRCDescrStr", indent * 2, width);
    }

    fn dump_audio_subtype(
        uacid: &usb::UacInterfaceDescriptor,
        uacp: &usb::UacProtocol,
        indent: usize,
    ) {
        match uacid {
            usb::UacInterfaceDescriptor::AudioHeader1(ach) => {
                dump_value(&ach.version, "bcdADC", indent * 2, 24);
                dump_value(ach.total_length, "wTotalLength", indent * 2, 24);
                dump_value(ach.collection_bytes, "bInCollection", indent * 2, 24);
                dump_array(&ach.interfaces, "baInterfaceNr", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioHeader2(ach) => {
                dump_value(&ach.version, "bcdADC", indent * 2, 24);
                dump_value(ach.total_length, "wTotalLength", indent * 2, 24);
                dump_hex(ach.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    ach.controls as u32,
                    &UAC2_INTERFACE_HEADER_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
            }
            usb::UacInterfaceDescriptor::AudioHeader3(ach) => {
                dump_value(ach.category, "bCategory", indent * 2, 24);
                dump_value(ach.total_length, "wTotalLength", indent * 2, 24);
                dump_hex(ach.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    ach.controls,
                    &UAC2_INTERFACE_HEADER_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
            }
            usb::UacInterfaceDescriptor::AudioInputTerminal1(ait) => {
                dump_value(ait.terminal_id, "bTerminalID", indent * 2, 24);
                println!(
                    "{:indent$}wTerminalType      {:5} {}",
                    "",
                    ait.terminal_type,
                    super::names::videoterminal(ait.terminal_type).unwrap_or_default(),
                    indent = indent * 2
                );
                dump_value(ait.assoc_terminal, "bAssocTerminal", indent * 2, 24);
                dump_value(ait.nr_channels, "bNrChannels", indent * 2, 24);
                dump_hex(ait.channel_config, "wChannelConfig", indent * 2, 24);
                let channel_names =
                    usb::UacInterfaceDescriptor::get_channel_names(uacp, ait.channel_config as u32);
                for name in channel_names.iter() {
                    println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
                }
                dump_value(ait.channel_names_index, "iChannelNames", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioInputTerminal2(ait) => {
                dump_value(ait.terminal_id, "bTerminalID", indent * 2, 24);
                println!(
                    "{:indent$}wTerminalType      {:5} {}",
                    "",
                    ait.terminal_type,
                    super::names::videoterminal(ait.terminal_type).unwrap_or_default(),
                    indent = indent * 2
                );
                dump_value(ait.assoc_terminal, "bAssocTerminal", indent * 2, 24);
                dump_value(ait.nr_channels, "bNrChannels", indent * 2, 24);
                dump_hex(ait.channel_config, "wChannelConfig", indent * 2, 24);
                let channel_names =
                    usb::UacInterfaceDescriptor::get_channel_names(uacp, ait.channel_config);
                for name in channel_names.iter() {
                    println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
                }
                dump_value(ait.channel_names_index, "iChannelNames", indent * 2, 24);
                dump_hex(ait.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    ait.controls,
                    &UAC2_INPUT_TERMINAL_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
                dump_value(ait.terminal_index, "iTerminal", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioInputTerminal3(ait) => {
                dump_value(ait.terminal_id, "bTerminalID", indent * 2, 24);
                dump_name(
                    ait.terminal_type,
                    super::names::videoterminal,
                    "wTerminalType",
                    indent * 2,
                    24,
                );
                dump_value(ait.assoc_terminal, "bAssocTerminal", indent * 2, 24);
                dump_value(ait.csource_id, "bCSourceID", indent * 2, 24);
                dump_hex(ait.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    ait.controls,
                    &UAC3_INPUT_TERMINAL_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
                dump_value(ait.cluster_descr_id, "wClusterDescrID", indent * 2, 24);
                dump_value(
                    ait.ex_terminal_descr_id,
                    "wExTerminalDescrID",
                    indent * 2,
                    24,
                );
                dump_value(ait.connectors_descr_id, "wConnectorDescrId", indent * 2, 24);
                dump_value(ait.terminal_descr_str, "wTerminalDescrStr", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioOutputTerminal1(a) => {
                dump_value(a.terminal_id, "bTerminalID", indent * 2, 24);
                dump_name(
                    a.terminal_type,
                    super::names::videoterminal,
                    "wTerminalType",
                    indent * 2,
                    24,
                );
                dump_value(a.assoc_terminal, "bAssocTerminal", indent * 2, 24);
                dump_value(a.source_id, "bSourceID", indent * 2, 24);
                dump_value(a.terminal_index, "iTerminal", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioOutputTerminal2(a) => {
                dump_value(a.terminal_id, "bTerminalID", indent * 2, 24);
                dump_name(
                    a.terminal_type,
                    super::names::videoterminal,
                    "wTerminalType",
                    indent * 2,
                    24,
                );
                dump_value(a.assoc_terminal, "bAssocTerminal", indent * 2, 24);
                dump_value(a.source_id, "bSourceID", indent * 2, 24);
                dump_hex(a.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    a.controls,
                    &UAC2_OUTPUT_TERMINAL_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
                dump_value(a.terminal_index, "iTerminal", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioOutputTerminal3(a) => {
                dump_value(a.terminal_id, "bTerminalID", indent * 2, 24);
                dump_name(
                    a.terminal_type,
                    super::names::videoterminal,
                    "wTerminalType",
                    indent * 2,
                    24,
                );
                dump_value(a.assoc_terminal, "bAssocTerminal", indent * 2, 24);
                dump_value(a.c_source_id, "bCSourceID", indent * 2, 24);
                dump_hex(a.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    a.controls,
                    &UAC3_OUTPUT_TERMINAL_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
                dump_value(a.ex_terminal_descr_id, "wExTerminalDescrID", indent * 2, 24);
                dump_value(a.connectors_descr_id, "wConnectorDescrId", indent * 2, 24);
                dump_value(a.terminal_descr_str, "wTerminalDescrStr", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::ExtendedTerminalHeader(d) => {
                dump_value(d.descriptor_id, "wDescriptorID", indent * 2, 24);
                dump_value(d.nr_channels, "bNrChannels", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioPowerDomain(power_domain) => {
                dump_audio_power_domain(power_domain, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioMixerUnit1(mixer_unit) => {
                dump_audio_mixer_unit1(mixer_unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioMixerUnit2(mixer_unit) => {
                dump_audio_mixer_unit2(mixer_unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioMixerUnit3(mixer_unit) => {
                dump_audio_mixer_unit3(mixer_unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioSelectorUnit1(selector_unit) => {
                dump_audio_selector_unit1(selector_unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioSelectorUnit2(selector_unit) => {
                dump_audio_selector_unit2(selector_unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioSelectorUnit3(selector_unit) => {
                dump_audio_selector_unit3(selector_unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioProcessingUnit1(unit) => {
                dump_audio_processing_unit1(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioProcessingUnit2(unit) => {
                dump_audio_processing_unit2(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioProcessingUnit3(unit) => {
                dump_audio_processing_unit3(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioEffectUnit2(unit) => {
                dump_audio_effect_unit2(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioEffectUnit3(unit) => {
                dump_audio_effect_unit3(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioFeatureUnit1(unit) => {
                dump_audio_feature_unit1(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioFeatureUnit2(unit) => {
                dump_audio_feature_unit2(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioFeatureUnit3(unit) => {
                dump_audio_feature_unit3(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioExtensionUnit1(unit) => {
                dump_audio_extension_unit1(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioExtensionUnit2(unit) => {
                dump_audio_extension_unit2(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioExtensionUnit3(unit) => {
                dump_audio_extension_unit3(unit, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioClockSource2(source) => {
                dump_audio_clock_source2(source, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioClockSource3(source) => {
                dump_audio_clock_source3(source, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioClockSelector2(selector) => {
                dump_audio_clock_selector2(selector, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioClockSelector3(selector) => {
                dump_audio_clock_selector3(selector, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioClockMultiplier2(multiplier) => {
                dump_audio_clock_multiplier2(multiplier, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioClockMultiplier3(multiplier) => {
                dump_audio_clock_multiplier3(multiplier, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioSampleRateConverter2(converter) => {
                dump_audio_sample_rate_converter2(converter, indent, 24);
            }
            usb::UacInterfaceDescriptor::AudioSampleRateConverter3(converter) => {
                dump_audio_sample_rate_converter3(converter, indent, 24);
            }

            usb::UacInterfaceDescriptor::AudioStreamingInterface1(asi) => {
                dump_value(asi.terminal_link, "bTerminalLink", indent * 2, 24);
                dump_value(asi.delay, "bDelay", indent * 2, 24);
                dump_value(asi.format_tag, "wFormatTag", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioStreamingInterface2(asi) => {
                dump_value(asi.terminal_link, "bTerminalLink", indent * 2, 24);
                dump_hex(asi.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    asi.controls,
                    &UAC2_AS_INTERFACE_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
                dump_value(asi.format_type, "bFormatType", indent * 2, 24);
                dump_value(asi.nr_channels, "bNrChannels", indent * 2, 24);
                dump_hex(asi.channel_config, "bmChannelConfig", indent * 2, 24);
                let channel_names =
                    usb::UacInterfaceDescriptor::get_channel_names(uacp, asi.channel_config as u32);
                for name in channel_names.iter() {
                    println!("{:indent$}{}", "", name, indent = (indent + 1) * 2);
                }
                dump_value(asi.channel_names_index, "iChannelNames", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioStreamingInterface3(asi) => {
                dump_value(asi.terminal_link, "bTerminalLink", indent * 2, 24);
                dump_hex(asi.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    asi.controls,
                    &UAC3_AS_INTERFACE_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
                dump_value(asi.cluster_descr_id, "wClusterDescrID", indent * 2, 24);
                dump_hex(asi.formats, "bmFormats", indent * 2, 24);
                dump_value(asi.sub_slot_size, "bSubslotSize", indent * 2, 24);
                dump_value(asi.bit_resolution, "bBitResolution", indent * 2, 24);
                dump_hex(asi.aux_protocols, "bmAuxProtocols", indent * 2, 24);
                dump_value(asi.control_size, "bControlSize", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioDataStreamingEndpoint1(ads) => {
                let uac1_attrs = |a: usize| match a {
                    0x00 => Some("Sampling Frequency"),
                    0x01 => Some("Pitch"),
                    0x02 => Some("Audio Data Format Control"),
                    0x07 => Some("MaxPacketsOnly"),
                    _ => None,
                };
                dump_hex(ads.attributes, "bmAttributes", indent * 2, 24);
                dump_bitmap_strings(ads.attributes, uac1_attrs, indent + 1);
                dump_value(ads.lock_delay_units, "bLockDelayUnits", indent * 2, 24);
                dump_value(ads.lock_delay, "wLockDelay", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioDataStreamingEndpoint2(ads) => {
                let uac2_attrs = |attr: usize| match attr {
                    0x07 => Some("MaxPacketsOnly"),
                    _ => None,
                };
                dump_hex(ads.attributes, "bmAttributes", indent * 2, 24);
                dump_bitmap_strings(ads.attributes, uac2_attrs, indent + 1);
                dump_hex(ads.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    ads.controls,
                    &UAC2_AS_ISO_ENDPOINT_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
                dump_value(ads.lock_delay_units, "bLockDelayUnits", indent * 2, 24);
                dump_value(ads.lock_delay, "wLockDelay", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::AudioDataStreamingEndpoint3(ads) => {
                dump_hex(ads.controls, "bmControls", indent * 2, 24);
                dump_bmcontrols(
                    ads.controls,
                    &UAC2_AS_ISO_ENDPOINT_BMCONTROLS,
                    &usb::ControlType::BmControl2,
                    indent + 1,
                );
                dump_value(ads.lock_delay_units, "bLockDelayUnits", indent * 2, 24);
                dump_value(ads.lock_delay, "wLockDelay", indent * 2, 24);
            }
            usb::UacInterfaceDescriptor::Undefined(data) => {
                println!(
                    "{:indent$}Invalid desc subtype: {}",
                    "",
                    data.iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<String>>()
                        .join(" "),
                    indent = indent * 2
                );
            }
            // TODO remaining
            _ => (),
        }
    }

    fn dump_audiocontrol_interface(gd: &usb::GenericDescriptor, protocol: u8) {
        let subtype = usb::UacAcInterface::get_uac_subtype(gd.descriptor_subtype, protocol);
        println!("      AudioControl Interface Descriptor:");
        println!("        bLength              {:3}", gd.length);
        println!("        bDescriptorType      {:3}", gd.descriptor_type);
        println!(
            "        bDescriptorSubType   {:3} ({:#})",
            gd.descriptor_subtype, subtype
        );

        if let Some(data) = gd.data.as_ref() {
            let uacp = usb::UacProtocol::from(protocol);
            match subtype.get_descriptor(&uacp, data) {
                Ok(uacid) => {
                    dump_audio_subtype(&uacid, &uacp, 4);
                }
                Err(_) => {
                    println!(
                        "{:indent$}Warning: {:#} descriptors are illegal for {}",
                        "",
                        subtype,
                        uacp,
                        indent = 6
                    );
                }
            }
        }
    }

    fn dump_audiostreaming_interface(gd: &usb::GenericDescriptor, protocol: u8) {
        let subtype = usb::UacAsInterface::from(gd.descriptor_subtype);
        println!("      AudioStreaming Interface Descriptor:");
        println!("        bLength              {:3}", gd.length);
        println!("        bDescriptorType      {:3}", gd.descriptor_type);
        print!("        bDescriptorSubType   {:3} ", gd.descriptor_subtype);

        if let Some(data) = gd.data.as_ref() {
            let uacp = usb::UacProtocol::from(protocol);
            match subtype {
                usb::UacAsInterface::General | usb::UacAsInterface::Undefined => {
                    println!("({:#})", subtype);
                    match subtype.get_descriptor(&uacp, data) {
                        Ok(uacid) => {
                            dump_audio_subtype(&uacid, &uacp, 4);
                        }
                        Err(_) => {
                            println!(
                                "{:indent$}Warning: {:#} descriptors are illegal for {}",
                                "",
                                subtype,
                                uacp,
                                indent = 6
                            );
                        }
                    }
                }
                usb::UacAsInterface::FormatType => {
                    println!("(FORMAT_TYPE)");
                    match uacp {
                        usb::UacProtocol::Uac1 => {
                            if data.len() < 5 {
                                println!("      Warning: Descriptor too short");
                                return;
                            }
                            print!("        bFormatType        {:5} ", data[0]);
                            match data[0] {
                                0x01 => dump_format_type_i(data),
                                0x02 => dump_format_type_ii(data),
                                0x03 => dump_format_type_iii(data),
                                _ => println!(
                                    "(invalid)\n        Invalid desc format type: {}",
                                    data[1..]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join("")
                                ),
                            }
                        }
                        usb::UacProtocol::Uac2 => {
                            if data.len() < 1 {
                                println!("      Warning: Descriptor too short");
                                return;
                            }
                            print!("        bFormatType        {:5} ", data[0]);
                            match data[0] {
                                0x01 => dump_format_type_i_uac2(data),
                                0x02 => dump_format_type_ii_uac2(data),
                                0x03 => dump_format_type_iii_uac2(data),
                                0x04 => dump_format_type_iv_uac2(data),
                                _ => println!(
                                    "(invalid)\n        Invalid desc format type: {}",
                                    data[1..]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join("")
                                ),
                            }
                        }
                        _ => println!("Unknown protocol"),
                    }
                }
                usb::UacAsInterface::FormatSpecific => {
                    println!("(FORMAT_SPECIFIC)");
                    if data.len() < 2 {
                        println!("      Warning: Descriptor too short");
                        return;
                    }
                    let fmttag = u16::from_le_bytes([data[0], data[1]]);
                    let fmtptr = get_format_specific_string(fmttag);
                    println!("        wFormatTag          {:5} {}", fmttag, fmtptr);
                    match fmttag {
                        0x1001 => dump_format_specific_mpeg(data),
                        0x1002 => dump_format_specific_ac3(data),
                        _ => println!(
                            "        Invalid desc format type: {}",
                            data[2..]
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<String>>()
                                .join("")
                        ),
                    }
                }
            }
        }
    }

    fn get_format_specific_string(fmttag: u16) -> &'static str {
        const FMT_ITAG: [&str; 6] = [
            "TYPE_I_UNDEFINED",
            "PCM",
            "PCM8",
            "IEEE_FLOAT",
            "ALAW",
            "MULAW",
        ];
        const FMT_IITAG: [&str; 3] = ["TYPE_II_UNDEFINED", "MPEG", "AC-3"];
        const FMT_IIITAG: [&str; 7] = [
            "TYPE_III_UNDEFINED",
            "IEC1937_AC-3",
            "IEC1937_MPEG-1_Layer1",
            "IEC1937_MPEG-Layer2/3/NOEXT",
            "IEC1937_MPEG-2_EXT",
            "IEC1937_MPEG-2_Layer1_LS",
            "IEC1937_MPEG-2_Layer2/3_LS",
        ];

        match fmttag {
            0..=5 => FMT_ITAG[fmttag as usize],
            0x1000..=0x1002 => FMT_IITAG[(fmttag & 0xfff) as usize],
            0x2000..=0x2006 => FMT_IIITAG[(fmttag & 0xfff) as usize],
            _ => "undefined",
        }
    }

    fn dump_format_type_i(data: &[u8]) {
        println!("(FORMAT_TYPE_I)");
        let len = if data[4] != 0 {
            data[4] as usize * 3 + 5
        } else {
            11
        };
        if data.len() < len {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!("        bNrChannels        {:5}", data[1]);
        println!("        bSubframeSize      {:5}", data[2]);
        println!("        bBitResolution     {:5}", data[3]);
        println!(
            "        bSamFreqType       {:5} {}",
            data[4],
            if data[4] != 0 {
                "Discrete"
            } else {
                "Continuous"
            }
        );
        if data[4] == 0 {
            if data.len() < 11 {
                println!("      Warning: Descriptor too short for continuous sample frequency");
                return;
            }
            println!(
                "        tLowerSamFreq    {:7}",
                u32::from_le_bytes([data[5], data[6], data[7], 0])
            );
            println!(
                "        tUpperSamFreq    {:7}",
                u32::from_le_bytes([data[8], data[9], data[10], 0])
            );
        } else {
            for i in 0..data[4] {
                if data.len() < 5 + 3 * (i as usize + 1) {
                    println!("      Warning: Descriptor too short for discrete sample frequency");
                    return;
                }
                println!(
                    "        tSamFreq[{:2}]   {:7}",
                    i,
                    u32::from_le_bytes([
                        data[5 + 3 * i as usize],
                        data[6 + 3 * i as usize],
                        data[7 + 3 * i as usize],
                        0
                    ])
                );
            }
        }
    }

    fn dump_format_type_ii(data: &[u8]) {
        println!("(FORMAT_TYPE_II)");
        let len = if data[5] != 0 {
            data[4] as usize * 3 + 6
        } else {
            12
        };
        if data.len() < len {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!(
            "        wMaxBitRate        {:5}",
            u16::from_le_bytes([data[1], data[2]])
        );
        println!(
            "        wSamplesPerFrame   {:5}",
            u16::from_le_bytes([data[3], data[4]])
        );
        println!(
            "        bSamFreqType       {:5} {}",
            data[5],
            if data[5] != 0 {
                "Discrete"
            } else {
                "Continuous"
            }
        );
        if data[5] == 0 {
            if data.len() < 12 {
                println!("      Warning: Descriptor too short for continuous sample frequency");
                return;
            }
            println!(
                "        tLowerSamFreq    {:7}",
                u32::from_le_bytes([data[6], data[7], data[8], 0])
            );
            println!(
                "        tUpperSamFreq    {:7}",
                u32::from_le_bytes([data[9], data[10], data[11], 0])
            );
        } else {
            for i in 0..data[5] {
                if data.len() < 6 + 3 * (i as usize + 1) {
                    println!("      Warning: Descriptor too short for discrete sample frequency");
                    return;
                }
                println!(
                    "        tSamFreq[{:2}]     {:7}",
                    i,
                    u32::from_le_bytes([
                        data[6 + 3 * i as usize],
                        data[7 + 3 * i as usize],
                        data[8 + 3 * i as usize],
                        0
                    ])
                );
            }
        }
    }

    fn dump_format_type_iii(data: &[u8]) {
        println!("(FORMAT_TYPE_III)");
        let len = if data[4] != 0 {
            data[4] as usize * 3 + 5
        } else {
            11
        };
        if data.len() < len {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!("        bNrChannels        {:5}", data[1]);
        println!("        bSubframeSize      {:5}", data[2]);
        println!("        bBitResolution     {:5}", data[3]);
        println!(
            "        bSamFreqType       {:5} {}",
            data[4],
            if data[4] != 0 {
                "Discrete"
            } else {
                "Continuous"
            }
        );
        if data[4] == 0 {
            if data.len() < 11 {
                println!("      Warning: Descriptor too short for continuous sample frequency");
                return;
            }
            println!(
                "        tLowerSamFreq    {:7}",
                u32::from_le_bytes([data[5], data[6], data[7], 0])
            );
            println!(
                "        tUpperSamFreq    {:7}",
                u32::from_le_bytes([data[8], data[9], data[10], 0])
            );
        } else {
            for i in 0..data[4] {
                if data.len() < 5 + 3 * (i as usize + 1) {
                    println!("      Warning: Descriptor too short for discrete sample frequency");
                    return;
                }
                println!(
                    "        tSamFreq[{:2}]   {:7}",
                    i,
                    u32::from_le_bytes([
                        data[5 + 3 * i as usize],
                        data[6 + 3 * i as usize],
                        data[7 + 3 * i as usize],
                        0
                    ])
                );
            }
        }
    }

    fn dump_format_type_i_uac2(data: &[u8]) {
        println!("(FORMAT_TYPE_I)");
        if data.len() < 3 {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!("        bSubslotSize       {:5}", data[1]);
        println!("        bBitResolution     {:5}", data[2]);
    }

    fn dump_format_type_ii_uac2(data: &[u8]) {
        println!("(FORMAT_TYPE_II)");
        if data.len() < 5 {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!(
            "        wMaxBitRate        {:5}",
            u16::from_le_bytes([data[1], data[2]])
        );
        println!(
            "        wSlotsPerFrame     {:5}",
            u16::from_le_bytes([data[3], data[4]])
        );
    }

    fn dump_format_type_iii_uac2(data: &[u8]) {
        println!("(FORMAT_TYPE_III)");
        if data.len() < 3 {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!("        bSubslotSize       {:5}", data[1]);
        println!("        bBitResolution     {:5}", data[2]);
    }

    fn dump_format_type_iv_uac2(data: &[u8]) {
        println!("(FORMAT_TYPE_IV)");
        if data.len() < 1 {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!("        bFormatType        {:5}", data[0]);
    }

    fn dump_format_specific_mpeg(data: &[u8]) {
        if data.len() < 5 {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!(
            "        bmMPEGCapabilities 0x{:04x}",
            u16::from_le_bytes([data[2], data[3]])
        );
        if data[2] & 0x01 != 0 {
            println!("          Layer I");
        }
        if data[2] & 0x02 != 0 {
            println!("          Layer II");
        }
        if data[2] & 0x04 != 0 {
            println!("          Layer III");
        }
        if data[2] & 0x08 != 0 {
            println!("          MPEG-1 only");
        }
        if data[2] & 0x10 != 0 {
            println!("          MPEG-1 dual-channel");
        }
        if data[2] & 0x20 != 0 {
            println!("          MPEG-2 second stereo");
        }
        if data[2] & 0x40 != 0 {
            println!("          MPEG-2 7.1 channel augmentation");
        }
        if data[2] & 0x80 != 0 {
            println!("          Adaptive multi-channel prediction");
        }
        println!(
            "          MPEG-2 multilingual support: {}",
            match data[3] & 3 {
                0 => "Not supported",
                1 => "Supported at Fs",
                2 => "Reserved",
                _ => "Supported at Fs and 1/2Fs",
            }
        );
        println!("        bmMPEGFeatures       0x{:02x}", data[4]);
        println!(
            "          Internal Dynamic Range Control: {}",
            match (data[4] >> 4) & 3 {
                0 => "not supported",
                1 => "supported but not scalable",
                2 => "scalable, common boost and cut scaling value",
                _ => "scalable, separate boost and cut scaling value",
            }
        );
    }

    fn dump_format_specific_ac3(data: &[u8]) {
        if data.len() < 7 {
            println!("      Warning: Descriptor too short");
            return;
        }
        println!(
            "        bmBSID         0x{:08x}",
            u32::from_le_bytes([data[2], data[3], data[4], data[5]])
        );
        println!("        bmAC3Features        0x{:02x}", data[6]);
        if data[6] & 0x01 != 0 {
            println!("          RF mode");
        }
        if data[6] & 0x02 != 0 {
            println!("          Line mode");
        }
        if data[6] & 0x04 != 0 {
            println!("          Custom0 mode");
        }
        if data[6] & 0x08 != 0 {
            println!("          Custom1 mode");
        }
        println!(
            "          Internal Dynamic Range Control: {}",
            match (data[6] >> 4) & 3 {
                0 => "not supported",
                1 => "supported but not scalable",
                2 => "scalable, common boost and cut scaling value",
                _ => "scalable, separate boost and cut scaling value",
            }
        );
    }

    fn dump_midistreaming_interface(md: &usb::MidiDescriptor) {
        let jack_types = |t: u8| match t {
            0x00 => "Undefined",
            0x01 => "Embedded",
            0x02 => "External",
            _ => "Invalid",
        };

        println!("      MIDIStreaming Interface Descriptor:");
        println!("        bLength              {:5}", md.length);
        println!("        bDescriptorType      {:5}", md.descriptor_type);
        print!(
            "      bDescriptorSubType   {:5} ",
            md.midi_type.to_owned() as u8
        );

        match md.midi_type {
            usb::MidiInterface::Header => {
                println!("(HEADER)");
                if md.data.len() >= 4 {
                    let total_length = u16::from_le_bytes([md.data[2], md.data[3]]);
                    println!(
                        "        bcdADC              {:2x}.{:02x}",
                        md.data[1], md.data[0]
                    );
                    println!("        wTotalLength       0x{:04x}", total_length);
                }
                dump_junk(&md.data, 8, md.length as usize - 3, 4);
            }
            usb::MidiInterface::InputJack => {
                println!("(MIDI_IN_JACK)");
                if md.data.len() >= 3 {
                    println!(
                        "        bJackType           {:5} {}",
                        md.data[0],
                        jack_types(md.data[0])
                    );
                    println!("        bJackID             {:5}", md.data[1]);
                    println!(
                        "        iJack               {:5} {}",
                        md.data[2],
                        md.string.as_ref().unwrap_or(&String::new())
                    );
                }
                dump_junk(&md.data, 8, md.length as usize - 3, 4);
            }
            usb::MidiInterface::OutputJack => {
                println!("(MIDI_OUT_JACK)");
                if md.data.len() >= md.length as usize - 3 {
                    println!(
                        "        bJackType           {:5} {}",
                        md.data[0],
                        jack_types(md.data[0])
                    );
                    println!("        bJackID             {:5}", md.data[1]);
                    println!("        bNrInputPins        {:5}", md.data[2]);

                    for (i, b) in md.data[3..].chunks(2).enumerate() {
                        if i == md.data[2] as usize {
                            break;
                        }
                        println!("        baSourceID({:2})     {:5}", i, b[0]);
                        println!("        baSourcePin({:2})    {:5}", i, b[1]);
                    }

                    println!(
                        "        iJack               {:5} {}",
                        md.data[3 + md.data[2] as usize],
                        md.string.as_ref().unwrap_or(&String::new())
                    );
                    dump_junk(&md.data, 8, md.length as usize - 3, 4 + md.data[2] as usize);
                }
            }
            usb::MidiInterface::Element => {
                println!("(ELEMENT)");
                if md.data.len() >= md.length as usize - 3 {
                    let num_inputs = md.data[1] as usize;
                    println!("        bElementID          {:5}", md.data[0]);
                    println!("        bNrInputPins        {:5}", num_inputs);
                    for (i, b) in md.data[2..].chunks(2).enumerate() {
                        if i == num_inputs {
                            break;
                        }
                        println!("        baSourceID({:2})     {:5}", i, b[0]);
                        println!("        baSourcePin({:2})    {:5}", i, b[1]);
                    }
                    let j = 2 + num_inputs * 2;
                    println!("        bNrOutputPins       {:5}", md.data[j]);
                    println!("        bInTerminalLink     {:5}", md.data[j + 1]);
                    println!("        bOutTerminalLink    {:5}", md.data[j + 2]);
                    println!("        bElCapsSize         {:5}", md.data[j + 3]);
                    let capsize = md.data[j + 3] as usize;
                    let mut caps: u16 = 0;
                    for j in 0..capsize {
                        caps |= (md.data[j + 6 + num_inputs * 2] as u16) << (j * 8);
                    }
                    println!("        bmElementCaps  0x{:08x}", caps);
                    if caps & 0x01 != 0 {
                        println!("          Undefined");
                    }
                    if caps & 0x02 != 0 {
                        println!("          MIDI Clock");
                    }
                    if caps & 0x04 != 0 {
                        println!("          MTC (MIDI Time Code)");
                    }
                    if caps & 0x08 != 0 {
                        println!("          MMC (MIDI Machine Control)");
                    }
                    if caps & 0x10 != 0 {
                        println!("          GM1 (General MIDI v.1)");
                    }
                    if caps & 0x20 != 0 {
                        println!("          GM2 (General MIDI v.2)");
                    }
                    if caps & 0x40 != 0 {
                        println!("          GS MIDI Extension");
                    }
                    if caps & 0x80 != 0 {
                        println!("          XG MIDI Extension");
                    }
                    if caps & 0x0100 != 0 {
                        println!("          EFX");
                    }
                    if caps & 0x0200 != 0 {
                        println!("          MIDI Patch Bay");
                    }
                    if caps & 0x0400 != 0 {
                        println!("          DLS1 (Downloadable Sounds Level 1)");
                    }
                    if caps & 0x0800 != 0 {
                        println!("          DLS2 (Downloadable Sounds Level 2)");
                    }

                    println!(
                        "        iElement            {:5} {}",
                        md.data[2 + md.data[1] as usize],
                        md.string.as_ref().unwrap_or(&String::new())
                    );
                    dump_junk(&md.data, 8, md.length as usize - 3, j + 1_usize);
                }
            }
            _ => {
                println!(
                    "(invalid)\n        Invalid desc subtype: {}",
                    md.data
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<String>>()
                        .join(" ")
                );
            }
        }
    }

    fn dump_videocontrol_interface(vcd: &usb::UvcDescriptor, protocol: u8) {
        println!("      VideoControl Interface Descriptor:");
        println!("        bLength             {:5}", vcd.length);
        println!("        bDescriptorType     {:5}", vcd.descriptor_type);
        print!("        bDescriptorSubType  {:5} ", vcd.descriptor_subtype);

        match usb::UvcInterface::from(vcd.descriptor_subtype) {
            usb::UvcInterface::Header => {
                println!("(HEADER)");
                if vcd.data.len() >= 10 {
                    let n = vcd.data[8] as usize;
                    let freq =
                        u32::from_le_bytes([vcd.data[4], vcd.data[5], vcd.data[6], vcd.data[7]]);
                    println!(
                        "        bcdUVC              {:2x}.{:02x}",
                        vcd.data[1], vcd.data[0]
                    );
                    println!(
                        "        wTotalLength       0x{:04x}",
                        u16::from_le_bytes([vcd.data[2], vcd.data[3]])
                    );
                    println!(
                        "        dwClockFrequency    {:5}.{:06}MHz",
                        freq / 1000000,
                        freq % 1000000
                    );
                    println!("        bInCollection       {:5}", n);
                    for (i, b) in vcd.data[9..].iter().enumerate() {
                        if i == n {
                            break;
                        }
                        println!("        baInterfaceNr({:2})   {:5}", i, b);
                    }

                    dump_junk(&vcd.data, 8, vcd.length as usize - 3, 9 + n);
                }
            }
            usb::UvcInterface::InputTerminal => {
                println!("(INPUT_TERMINAL)");
                if vcd.data.len() >= 10 {
                    let term_type = u16::from_le_bytes([vcd.data[1], vcd.data[2]]);
                    let mut n = if term_type == 0x0201 { 7 } else { 0 };
                    println!("        bTerminalID         {:5}", vcd.data[0]);
                    println!(
                        "        wTerminalType      0x{:04x} {}",
                        term_type,
                        super::names::videoterminal(term_type).unwrap_or_default()
                    );
                    println!("        bAssocTerminal      {:5}", vcd.data[3]);
                    println!(
                        "        iTerminal           {:5} {}",
                        vcd.data[4],
                        vcd.string.as_ref().unwrap_or(&String::new())
                    );

                    if term_type == 0x0201 {
                        n += vcd.data[11] as usize;
                        println!(
                            "        wObjectiveFocalLengthMin  {:5}",
                            u16::from_le_bytes([vcd.data[5], vcd.data[6]])
                        );
                        println!(
                            "        wObjectiveFocalLengthMax  {:5}",
                            u16::from_le_bytes([vcd.data[7], vcd.data[8]])
                        );
                        println!(
                            "        wOcularFocalLength        {:5}",
                            u16::from_le_bytes([vcd.data[9], vcd.data[10]])
                        );
                        println!("        bControlSize              {:5}", vcd.data[11]);

                        let mut controls: u32 = 0;
                        for i in 0..3 {
                            if i < vcd.data[11] as usize {
                                controls = (controls << 8) | vcd.data[5 + n - i - 1] as u32;
                            }
                        }
                        println!("        bmControls           0x{:08x}", controls);

                        if protocol == 0x01 {
                            for (i, n) in CAM_CTRL_NAMES.iter().enumerate().take(22) {
                                if (controls >> i) & 1 != 0 {
                                    println!("         {}", n);
                                }
                            }
                        } else {
                            for (i, n) in CAM_CTRL_NAMES.iter().enumerate().take(19) {
                                if (controls >> i) & 1 != 0 {
                                    println!("         {}", n);
                                }
                            }
                        }
                    }

                    dump_junk(&vcd.data, 8, vcd.length as usize - 3, 5 + n);
                } else {
                    println!("      Warning: Descriptor too short");
                }
            }
            usb::UvcInterface::OutputTerminal => {
                println!("(OUTPUT_TERMINAL)");
                if vcd.data.len() >= 6 {
                    let term_type = u16::from_le_bytes([vcd.data[1], vcd.data[2]]);
                    println!("        bTerminalID         {:5}", vcd.data[0]);
                    println!(
                        "        wTerminalType      0x{:04x} {}",
                        term_type,
                        super::names::videoterminal(term_type).unwrap_or_default()
                    );
                    println!("        bAssocTerminal      {:5}", vcd.data[3]);
                    println!("        bSourceID           {:5}", vcd.data[4]);
                    println!(
                        "        iTerminal           {:5} {}",
                        vcd.data[5],
                        vcd.string.as_ref().unwrap_or(&String::new())
                    );
                } else {
                    println!("      Warning: Descriptor too short");
                }

                dump_junk(&vcd.data, 8, vcd.length as usize - 3, 6);
            }
            usb::UvcInterface::SelectorUnit => {
                println!("(SELECTOR_UNIT)");
                if vcd.data.len() >= 4 {
                    let pins = vcd.data[1] as usize;
                    println!("        bUnitID             {:5}", vcd.data[0]);
                    println!("        bNrInPins           {:5}", pins);
                    for (i, b) in vcd.data[2..].iter().enumerate() {
                        if i == pins {
                            break;
                        }
                        println!("        baSourceID({:2})        {:5}", i, b);
                    }
                    println!(
                        "        iSelector           {:5} {}",
                        vcd.data[2 + pins],
                        vcd.string.as_ref().unwrap_or(&String::new())
                    );

                    dump_junk(&vcd.data, 8, vcd.length as usize - 3, 3 + pins);
                } else {
                    println!("      Warning: Descriptor too short");
                }
            }
            usb::UvcInterface::ProcessingUnit => {
                println!("(PROCESSING_UNIT)");
                if vcd.data.len() >= 9 {
                    let n = vcd.data[4] as usize;
                    println!("        bUnitID             {:5}", vcd.data[0]);
                    println!("        bSourceID           {:5}", vcd.data[1]);
                    println!(
                        "        wMaxMultiplier      {:5}",
                        u16::from_le_bytes([vcd.data[2], vcd.data[3]])
                    );
                    println!("        bControlSize        {:5}", n);

                    let mut controls: u32 = 0;
                    for i in 0..3 {
                        if i < n {
                            controls = (controls << 8) | vcd.data[5 + n - i - 1] as u32;
                        }
                    }
                    println!("        bmControls     0x{:08x}", controls);
                    if protocol == 0x01 {
                        for (i, n) in CTRL_NAMES.iter().enumerate().take(19) {
                            if (controls >> i) & 1 != 0 {
                                println!("         {}", n);
                            }
                        }
                    } else {
                        for (i, n) in CTRL_NAMES.iter().enumerate().take(18) {
                            if (controls >> i) & 1 != 0 {
                                println!("         {}", n);
                            }
                        }
                    }
                    let stds = vcd.data[6 + n] as usize;
                    println!(
                        "        iProcessing         {:5} {}",
                        vcd.data[5 + n],
                        vcd.string.as_ref().unwrap_or(&String::new())
                    );
                    println!("        bmVideoStandards     0x{:02x}", stds);
                    for (i, n) in STD_NAMES.iter().enumerate().take(6) {
                        if (stds >> i) & 1 != 0 {
                            println!("         {}", n);
                        }
                    }
                } else {
                    println!("      Warning: Descriptor too short");
                }
            }
            usb::UvcInterface::ExtensionUnit => {
                println!("(EXTENSION_UNIT)");
                if vcd.data.len() >= 21 {
                    let p = vcd.data[18] as usize;
                    let n = vcd.data[19 + p] as usize;
                    println!("        bUnitID             {:5}", vcd.data[0]);
                    println!(
                        "        guidExtensionCode         {}",
                        get_guid(&vcd.data[1..17])
                    );
                    println!("        bNumControls        {:5}", vcd.data[17]);
                    println!("        bNrInPins           {:5}", vcd.data[18]);

                    if vcd.data.len() >= 21 + p + n {
                        for (i, b) in vcd.data[19..19 + p].iter().enumerate() {
                            println!("        baSourceID({:2})      {:5}", i, b);
                        }
                        println!("        bControlSize        {:5}", vcd.data[19 + p]);
                        for (i, b) in vcd.data[20 + p..20 + p + n].iter().enumerate() {
                            println!("        bmControls({:2})       0x{:02x}", i, b);
                        }
                        println!(
                            "        iExtension          {:5} {}",
                            vcd.data[20 + p + n],
                            vcd.string.as_ref().unwrap_or(&String::new())
                        );
                    }

                    dump_junk(&vcd.data, 8, vcd.length as usize - 3, 21 + p + n);
                } else {
                    println!("      Warning: Descriptor too short");
                }
            }
            usb::UvcInterface::EncodingUnit => {
                println!("(ENCODING_UNIT)");
                if vcd.data.len() >= 10 {
                    println!("        bUnitID             {:5}", vcd.data[0]);
                    println!("        bSourceID           {:5}", vcd.data[1]);
                    println!(
                        "        iEncoding           {:5} {}",
                        vcd.data[2],
                        vcd.string.as_ref().unwrap_or(&String::new())
                    );
                    println!("        bControlSize        {:5}", vcd.data[3]);

                    let mut controls: u32 = 0;
                    for i in 0..3 {
                        controls = (controls << 8) | vcd.data[6 - i] as u32;
                    }
                    println!("        bmControls              0x{:08x}", controls);
                    for (i, n) in EN_CTRL_NAMES.iter().enumerate().take(20) {
                        if (controls >> i) & 1 != 0 {
                            println!("         {}", n); // Replace with your Rust lookup approach
                        }
                    }
                    for i in 0..3 {
                        controls = (controls << 8) | vcd.data[9 - i] as u32;
                    }
                    println!("        bmControlsRuntime       0x{:08x}", controls);
                    for (i, n) in EN_CTRL_NAMES.iter().enumerate().take(20) {
                        if (controls >> i) & 1 != 0 {
                            println!("         {}", n);
                        }
                    }
                } else {
                    println!("      Warning: Descriptor too short");
                }
            }
            _ => {
                println!("(unknown)");
                println!(
                    "        Invalid desc subtype: {}",
                    vcd.data
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<String>>()
                        .join(" ")
                );
            }
        }
    }

    fn dump_videostreaming_interface(gd: &usb::GenericDescriptor) {
        println!("      VideoStreaming Interface Descriptor:");
        println!("        bLength              {:5}", gd.length);
        println!("        bDescriptorType      {:5}", gd.descriptor_type);
        print!("        bDescriptorSubType   {:5} ", gd.descriptor_subtype);

        let color_primatives = |c: u8| match c {
            1 => "BT.709,sRGB",
            2 => "BT.470-2 (M)",
            3 => "BT.470-2 (B,G)",
            4 => "SMPTE 170M",
            5 => "SMPTE 240M",
            _ => "Unspecified",
        };

        let transfer_characteristics = |c: u8| match c {
            1 => "BT.709",
            2 => "BT.470-2 (M)",
            3 => "BT.470-2 (B,G)",
            4 => "SMPTE 170M",
            5 => "SMPTE 240M",
            6 => "Linear",
            7 => "sRGB",
            _ => "Unspecified",
        };

        let matrix_coefficients = |c: u8| match c {
            1 => "BT.709",
            2 => "FCC",
            3 => "BT.470-2 (B,G)",
            4 => "SMPTE 170M (BT.601)",
            5 => "SMPTE 240M",
            _ => "Unspecified",
        };

        let field_pattern = |f: u8| match f {
            0 => "Field 1 only",
            1 => "Field 2 only",
            2 => "Regular pattern of fields 1 and 2",
            3 => "Random pattern of fields 1 and 2",
            _ => "Invalid",
        };

        if let Some(data) = &gd.data {
            match gd.descriptor_subtype {
                0x01 => {
                    println!("(INPUT_HEADER)");
                    if data.len() >= 11 {
                        let formats = data[0];
                        let control_size = data[9];
                        println!("        bNumFormats                     {:5}", formats);
                        println!(
                            "        wTotalLength                   0x{:04x}",
                            u16::from_le_bytes([data[1], data[2]])
                        );
                        println!(
                            "        bEndpointAddress                 0x{:02x}  EP {} {}",
                            data[3],
                            data[3] & 0x0f,
                            if data[3] & 0x80 != 0 { "IN" } else { "OUT" }
                        );
                        println!("        bmInfo                          {:5}", data[4]);
                        println!("        bTerminalLink                   {:5}", data[5]);
                        println!("        bStillCaptureMethod             {:5}", data[6]);
                        println!("        bTriggerSupport                 {:5}", data[7]);
                        println!("        bTriggerUsage                   {:5}", data[8]);
                        println!("        bControlSize                    {:5}", control_size);
                        for (i, b) in data[10..].chunks(control_size as usize).enumerate() {
                            if i == formats as usize {
                                break;
                            }
                            println!("        bmaControls({:2})                 {:5}", i, b[0]);
                        }

                        dump_junk(
                            data,
                            8,
                            gd.expected_data_length(),
                            10 + formats as usize * control_size as usize,
                        );
                    }
                }
                0x02 => {
                    println!("(OUTPUT_HEADER)");
                    if data.len() >= 7 {
                        let formats = data[0];
                        let control_size = data[8];
                        println!("        bNumFormats                     {:5}", formats);
                        println!(
                            "        wTotalLength                  0x{:04x}",
                            u16::from_le_bytes([data[1], data[2]])
                        );
                        println!(
                            "        bEndpointAddress                0x{:02x}  EP {} {}",
                            data[3],
                            data[3] & 0x0f,
                            if data[3] & 0x80 != 0 { "IN" } else { "OUT" }
                        );
                        println!("        bTerminalLink                   {:5}", data[4]);
                        println!("        bControlSize                    {:5}", control_size);
                        for (i, b) in data[6..].chunks(control_size as usize).enumerate() {
                            if i == formats as usize {
                                break;
                            }
                            println!("        bmaControls({:2})                 {:5}", i, b[0]);
                        }

                        dump_junk(
                            data,
                            8,
                            gd.expected_data_length(),
                            6 + formats as usize * control_size as usize,
                        );
                    }
                }
                0x03 => {
                    println!("(STILL_IMAGE_FRAME)");
                    if data.len() >= 3 {
                        let image_num = data[1] as usize;
                        let compression_num = data[2 + image_num * 4];
                        println!(
                            "        bEndpointAddress              0x{:02x}  EP {} {}",
                            data[0],
                            data[0] & 0x0f,
                            if data[0] & 0x80 != 0 { "IN" } else { "OUT" }
                        );
                        println!("        bNumImageSizePatterns          {:3}", image_num);
                        for (i, b) in data[2..].chunks(4).enumerate() {
                            if i == image_num {
                                break;
                            }
                            println!(
                                "        wWidth({:2})                   {:5}",
                                i,
                                u16::from_le_bytes([b[0], b[1]])
                            );
                            println!(
                                "        wHeight({:2})                  {:5}",
                                i,
                                u16::from_le_bytes([b[2], b[3]])
                            );
                        }
                        println!(
                            "        bNumCompressionPatterns        {:3}",
                            compression_num
                        );
                        if data.len() >= 3 + image_num * 4 + compression_num as usize {
                            for (i, b) in data[3 + image_num * 4..].iter().enumerate() {
                                if i == compression_num as usize {
                                    break;
                                }
                                println!("        bCompression({:2})             {:5}", i, b);
                            }
                        }

                        dump_junk(
                            data,
                            8,
                            gd.expected_data_length(),
                            3 + image_num * 4 + compression_num as usize,
                        );
                    }
                }
                0x04 | 0x010 => {
                    let len = if gd.descriptor_subtype == 0x04 {
                        println!("(FORMAT_UNCOMPRESSED)");
                        24
                    } else {
                        println!("(FORMAT_FRAME_BASED)");
                        25
                    };

                    if data.len() >= len {
                        let flags = data[22];
                        println!("        bFormatIndex                    {:5}", data[0]);
                        println!("        bNumFrameDescriptors            {:5}", data[1]);
                        println!(
                            "        guidFormat                            {}",
                            get_guid(&data[2..18])
                        );
                        println!("        bBitsPerPixel                   {:5}", data[18]);
                        println!("        bDefaultFrameIndex              {:5}", data[19]);
                        println!("        bAspectRatioX                   {:5}", data[20]);
                        println!("        bAspectRatioY                   {:5}", data[21]);
                        println!("        bmInterlaceFlags                 0x{:02x}", flags);
                        println!("        bCopyProtect                    {:5}", data[23]);
                        println!(
                            "          Interlaced stream or variable: {}",
                            if flags & 0x01 != 0 { "Yes" } else { "No" }
                        );
                        println!(
                            "          Fields per frame: {}",
                            if flags & 0x02 != 0 { "1" } else { "2" }
                        );
                        println!(
                            "          Field 1 first: {}",
                            if flags & 0x04 != 0 { "Yes" } else { "No" }
                        );
                        println!(
                            "          Field pattern: {}",
                            field_pattern((flags >> 4) & 0x03)
                        );
                        if gd.descriptor_subtype == 0x10 {
                            println!(
                                "        bVariableSize                  {:5}",
                                data.get(24).unwrap_or(&0)
                            );
                        }
                    }

                    dump_junk(data, 8, gd.expected_data_length(), len);
                }
                0x05 | 0x07 | 0x11 => {
                    let n = if gd.descriptor_subtype == 0x05 {
                        println!("(FRAME_UNCOMPRESSED)");
                        22
                    } else if gd.descriptor_subtype == 0x07 {
                        println!("(FRAME_MJPEG)");
                        22
                    } else {
                        println!("(FRAME_FRAME_BASED)");
                        18
                    };

                    if data.len() >= 23 {
                        let flags = data[1];
                        let len = if data[n] != 0 {
                            23 + data[n] as usize * 4
                        } else {
                            35
                        };
                        println!("        bFrameIndex                     {:5}", data[0]);
                        println!("        bmCapabilities                   0x{:02x}", flags);
                        if flags & 0x01 != 0 {
                            println!("          Still image supported");
                        } else {
                            println!("          Still image unsupported");
                        }
                        if flags & 0x02 != 0 {
                            println!("          Fixed frame-rate");
                        }
                        println!(
                            "        wWidth                          {:5}",
                            u16::from_le_bytes([data[2], data[3]])
                        );
                        println!(
                            "        wHeight                         {:5}",
                            u16::from_le_bytes([data[4], data[5]])
                        );
                        println!(
                            "        dwMinBitRate                {:9}",
                            u32::from_le_bytes([data[6], data[7], data[8], data[9]])
                        );
                        println!(
                            "        dwMaxBitRate                {:9}",
                            u32::from_le_bytes([data[10], data[11], data[12], data[13]])
                        );
                        if gd.descriptor_subtype == 0x11 {
                            println!(
                                "        dwDefaultFrameInterval      {:9}",
                                u32::from_le_bytes([data[14], data[15], data[16], data[17]])
                            );
                            println!("        bFrameIntervalType              {:5}", data[18]);
                            println!(
                                "        dwBytesPerLine              {:9}",
                                u32::from_le_bytes([data[19], data[20], data[21], data[22]])
                            );
                        } else {
                            println!(
                                "        dwMaxVideoFrameBufferSize   {:9}",
                                u32::from_le_bytes([data[14], data[15], data[16], data[17]])
                            );
                            println!(
                                "        dwDefaultFrameInterval      {:9}",
                                u32::from_le_bytes([data[18], data[19], data[20], data[21]])
                            );
                            println!("        bFrameIntervalType              {:5}", data[22]);
                        }
                        if data[n] == 0 && data.len() >= 35 {
                            println!(
                                "        dwMinFrameInterval          {:9}",
                                u32::from_le_bytes([data[23], data[24], data[25], data[26]])
                            );
                            println!(
                                "        dwMaxFrameInterval          {:9}",
                                u32::from_le_bytes([data[27], data[28], data[29], data[30]])
                            );
                            println!(
                                "        dwFrameIntervalStep         {:9}",
                                u32::from_le_bytes([data[31], data[32], data[33], data[34]])
                            );
                        } else {
                            for (i, b) in data[n..].chunks(4).enumerate() {
                                if i == data[n] as usize {
                                    break;
                                }
                                println!(
                                    "        dwFrameInterval({:2})       {:11}",
                                    i,
                                    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                                );
                            }
                        }

                        dump_junk(data, 8, gd.expected_data_length(), len);
                    }
                }
                0x06 => {
                    let mut flags = data[2];
                    println!("(FORMAT_MJPEG)");
                    if data.len() >= 8 {
                        println!("        bFormatIndex                    {:3}", data[0]);
                        println!("        bNumFrameDescriptors            {:3}", data[1]);
                        println!("        bFlags                          {:3}", flags);
                        println!(
                            "          Fixed-sized samples: {}",
                            if flags & 0x01 != 0 { "Yes" } else { "No" }
                        );
                        flags = data[6];
                        println!("        bDefaultFrameIndex              {:3}", data[3]);
                        println!("        bAspectRatioX                   {:3}", data[4]);
                        println!("        bAspectRatioY                   {:3}", data[5]);
                        println!("        bmInterlaceFlags               0x{:02x}", flags);
                        println!(
                            "          Interlaced stream or variable: {}",
                            if flags & 0x01 != 0 { "Yes" } else { "No" }
                        );
                        println!(
                            "          Fields per frame: {}",
                            if flags & 0x02 != 0 { "1" } else { "2" }
                        );
                        println!(
                            "          Field 1 first: {}",
                            if flags & 0x04 != 0 { "Yes" } else { "No" }
                        );
                        println!(
                            "          Field pattern: {}",
                            field_pattern((flags >> 4) & 0x03)
                        );
                        println!("        bCopyProtect                    {:3}", data[7]);
                    }

                    dump_junk(data, 8, gd.expected_data_length(), 8);
                }
                0x0a => {
                    println!("(FORMAT_MPEG2TS)");
                    if data.len() >= 4 {
                        println!("        bFormatIndex                    {:3}", data[0]);
                        println!("        bDataOffset                     {:3}", data[1]);
                        println!("        bPacketLength                   {:3}", data[2]);
                        println!("        bStrideLength                   {:3}", data[3]);
                        if data.len() >= 20 {
                            println!(
                                "        guidStrideFormat                      {}",
                                get_guid(&data[4..20])
                            );
                        }
                    }

                    if gd.len() < 23 {
                        dump_junk(data, 8, gd.expected_data_length(), 4);
                    } else {
                        dump_junk(data, 8, gd.expected_data_length(), 20);
                    }
                }
                0x0d => {
                    println!("(COLORFORMAT)");
                    if data.len() >= 3 {
                        println!(
                            "        bColorPrimaries                 {:3} ({})",
                            data[0],
                            color_primatives(data[0])
                        );
                        println!(
                            "        bTransferCharacteristics        {:3} ({})",
                            data[1],
                            transfer_characteristics(data[1])
                        );
                        println!(
                            "        bMatrixCoefficients             {:3} ({})",
                            data[2],
                            matrix_coefficients(data[2])
                        );
                    }

                    dump_junk(data, 8, gd.expected_data_length(), 3);
                }
                0x12 => {
                    println!("(FORMAT_STREAM_BASED)");
                    if data.len() >= 18 {
                        println!("        bFormatIndex                    {:3}", data[0]);
                        println!(
                            "        guidFormat                            {}",
                            get_guid(&data[1..17])
                        );
                        println!("        dwPacketLength                {:5}", data[17]);
                    }

                    dump_junk(data, 8, gd.expected_data_length(), 21);
                }
                _ => {
                    println!("(unknown)");
                    println!(
                        "        Invalid desc subtype: {}",
                        data.iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<String>>()
                            .join(" ")
                    );
                }
            }
        }
    }

    fn dump_bad_comm(cd: &usb::CommunicationDescriptor, indent: usize) {
        let data = Into::<Vec<u8>>::into(cd.to_owned());
        // convert to exact type str used by lsusb
        let type_str = match cd.communication_type {
            usb::CdcType::Header => "Header",
            usb::CdcType::CallManagement => "Call Management",
            usb::CdcType::AbstractControlManagement => "ACM",
            usb::CdcType::Union => "Union",
            usb::CdcType::CountrySelection => "Country Selection",
            usb::CdcType::TelephoneOperationalModes => "Telephone Operations",
            usb::CdcType::NetworkChannel => "Network Channel Terminal",
            usb::CdcType::EthernetNetworking => "Ethernet",
            usb::CdcType::WirelessHandsetControlModel => "WHCM version",
            usb::CdcType::MobileDirectLineModelFunctional => "MDLM",
            usb::CdcType::MobileDirectLineModelDetail => "MDLM detail",
            usb::CdcType::DeviceManagement => "Device Management",
            usb::CdcType::Obex => "OBEX",
            usb::CdcType::CommandSet => "Command Set",
            usb::CdcType::Ncm => "NCM",
            usb::CdcType::Mbim => "MBIM",
            usb::CdcType::MbimExtended => "MBIM Extended",
            _ => "",
        };
        println!(
            "{:^indent$}  INVALID CDC ({}): {}",
            "",
            type_str,
            data.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" ")
        );
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

    fn dump_comm_descriptor(cd: &usb::CommunicationDescriptor, indent: usize) {
        match cd.communication_type {
            usb::CdcType::Header => {
                if cd.data.len() >= 2 {
                    println!("{:^indent$}CDC Header:", "");
                    println!(
                        "{:^indent$}  bcdCDC              {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::CallManagement => {
                if cd.data.len() >= 2 {
                    println!("{:^indent$}CDC Call Management:", "");
                    println!("{:^indent$}  bmCapabilities      0x{:02x}", "", cd.data[0]);
                    if cd.data[0] & 0x01 != 0x00 {
                        println!("{:^indent$}    call management", "");
                    }
                    if cd.data[0] & 0x02 != 0x00 {
                        println!("{:^indent$}    use cd.dataInterface", "");
                    }
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::AbstractControlManagement => {
                if !cd.data.is_empty() {
                    println!("{:^indent$}CDC ACM:", "");
                    println!("{:^indent$}  bmCapabilities      0x{:02x}", "", cd.data[0]);
                    if cd.data[0] & 0x08 != 0x00 {
                        println!("{:^indent$}    connection notifications", "");
                    }
                    if cd.data[0] & 0x04 != 0x00 {
                        println!("{:^indent$}    sends break", "");
                    }
                    if cd.data[0] & 0x02 != 0x00 {
                        println!("{:^indent$}    line coding and serial state", "");
                    }
                    if cd.data[0] & 0x01 != 0x00 {
                        println!("{:^indent$}    get/set/clear comm features", "");
                    }
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::Union => {
                if cd.data.len() >= 2 {
                    println!("{:^indent$}CDC Union:", "");
                    println!("{:^indent$}  bMasterInterface     {:3}", "", cd.data[0]);
                    println!(
                        "{:^indent$}  bSlaveInterface      {}",
                        "",
                        cd.data[1..]
                            .iter()
                            .map(|b| format!("{:3}", b))
                            .collect::<Vec<String>>()
                            .join(" ")
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::CountrySelection => {
                if cd.data.len() >= 3 || (cd.length & 0x01) != 0 {
                    println!("{:^indent$}Country Selection:", "");
                    println!(
                        "{:^indent$}  iCountryCodeRelDate     {:3} {}",
                        "",
                        cd.string_index.unwrap_or_default(),
                        cd.string.as_ref().unwrap_or(&String::from("(?)"))
                    );
                    cd.data.chunks(2).for_each(|d| {
                        println!(
                            "{:^indent$}  wCountryCode          {:02x}{:02x}",
                            "", d[0], d[1]
                        );
                    });
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::TelephoneOperationalModes => {
                if !cd.data.is_empty() {
                    println!("{:^indent$}CDC Telephone operations:", "");
                    println!("{:^indent$}  bmCapabilities       0x{:02x}", "", cd.data[0]);
                    if cd.data[0] & 0x04 != 0x00 {
                        println!("{:^indent$}    computer centric mode", "");
                    }
                    if cd.data[0] & 0x02 != 0x00 {
                        println!("{:^indent$}    standalone mode", "");
                    }
                    if cd.data[0] & 0x01 != 0x00 {
                        println!("{:^indent$}    simple mode", "");
                    }
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::NetworkChannel => {
                if cd.data.len() >= 4 {
                    println!("{:^indent$}Network Channel Terminal:", "");
                    println!("{:^indent$}  bEntityId               {:3}", "", cd.data[0]);
                    println!(
                        "{:^indent$}  iName                   {:3} {}",
                        "",
                        cd.string_index.unwrap_or_default(),
                        cd.string.as_ref().unwrap_or(&String::from("(?)"))
                    );
                    println!("{:^indent$}  bChannelIndex           {:3}", "", cd.data[2]);
                    println!("{:^indent$}  bPhysicalInterface      {:3}", "", cd.data[3]);
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::EthernetNetworking => {
                if cd.data.len() >= 13 - 3 {
                    println!("{:^indent$}CDC Ethernet:", "");
                    println!(
                        "{:^indent$}  iMacAddress             {:10} {}",
                        "",
                        cd.string_index.unwrap_or_default(),
                        cd.string.as_ref().unwrap_or(&String::from("(?)"))
                    );
                    println!(
                        "{:^indent$}  bmEthernetStatistics    0x{:08x}",
                        "",
                        u32::from_le_bytes([cd.data[1], cd.data[2], cd.data[3], cd.data[4]])
                    );
                    println!(
                        "{:^indent$}  wMaxSegmentSize         {:10}",
                        "",
                        u16::from_le_bytes([cd.data[5], cd.data[6]])
                    );
                    println!(
                        "{:^indent$}  wNumberMCFilters            0x{:04x}",
                        "",
                        u16::from_le_bytes([cd.data[7], cd.data[8]])
                    );
                    println!("{:^indent$}  bNumberPowerFilters     {:10}", "", cd.data[9]);
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::WirelessHandsetControlModel => {
                if cd.data.len() >= 2 {
                    println!("{:^indent$}CDC WHCM:", "");
                    println!(
                        "{:^indent$}  bcdVersion           {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::MobileDirectLineModelFunctional => {
                if cd.data.len() >= 18 {
                    println!("{:^indent$}CDC MDLM:", "");
                    println!(
                        "{:^indent$}  bcdCDC               {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                    println!(
                        "{:^indent$}  bGUID               {}",
                        "",
                        get_guid(&cd.data[2..18])
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::MobileDirectLineModelDetail => {
                if cd.data.len() >= 2 {
                    println!("{:^indent$}CDC MDLM detail:", "");
                    println!("{:^indent$}  bGuidDescriptorType  {:02x}", "", cd.data[0]);
                    println!(
                        "{:^indent$}  bDetailData          {}",
                        "",
                        cd.data
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<String>>()
                            .join(" ")
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::DeviceManagement => {
                if cd.data.len() >= 4 {
                    println!("{:^indent$}CDC MDLM:", "");
                    println!(
                        "{:^indent$}  bcdVersion           {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                    println!(
                        "{:^indent$}  wMaxCommand          {:3}",
                        "",
                        u16::from_le_bytes([cd.data[2], cd.data[3]])
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::Obex => {
                if cd.data.len() >= 2 {
                    println!("{:^indent$}CDC OBEX:", "");
                    println!(
                        "{:^indent$}  bcdVersion           {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::CommandSet => {
                if cd.data.len() >= 19 {
                    println!("{:^indent$}CDC Command Set:", "");
                    println!(
                        "{:^indent$}  bcdVersion           {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                    println!(
                        "{:^indent$}  iCommandSet          {:4} {}",
                        "",
                        cd.string_index.unwrap_or_default(),
                        cd.string.as_ref().unwrap_or(&String::from("(?)"))
                    );
                    println!(
                        "{:^indent$}  bGUID               {}",
                        "",
                        get_guid(&cd.data[3..19])
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::Ncm => {
                if cd.data.len() >= 6 - 3 {
                    println!("{:^indent$}CDC NCM:", "");
                    println!(
                        "{:^indent$}  bcdNcmVersion        {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                    println!(
                        "{:^indent$}  bmNetworkCapabilities 0x{:02x}",
                        "", cd.data[2]
                    );
                    if cd.data[2] & (1 << 5) != 0 {
                        println!("{:^indent$}    8-byte ntb input size", "");
                    }
                    if cd.data[2] & (1 << 4) != 0 {
                        println!("{:^indent$}    crc mode", "");
                    }
                    if cd.data[2] & (1 << 2) != 0 {
                        println!("{:^indent$}    max cd.datagram size", "");
                    }
                    if cd.data[2] & (1 << 2) != 0 {
                        println!("{:^indent$}    encapsulated commands", "");
                    }
                    if cd.data[2] & (1 << 1) != 0 {
                        println!("{:^indent$}    net address", "");
                    }
                    if cd.data[2] & (1 << 0) != 0 {
                        println!("{:^indent$}    packet filter", "");
                    }
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::Mbim => {
                if cd.data.len() >= 9 {
                    println!("{:^indent$}CDC MBIM:", "");
                    println!(
                        "{:^indent$}  bcdMBIMVersion       {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                    println!(
                        "{:^indent$}  wMaxControlMessage   {}",
                        "",
                        u16::from_le_bytes([cd.data[2], cd.data[3]])
                    );
                    println!("{:^indent$}  bNumberFilters       {}", "", cd.data[4]);
                    println!("{:^indent$}  bMaxFilterSize       {}", "", cd.data[5]);
                    println!(
                        "{:^indent$}  wMaxSegmentSize      {}",
                        "",
                        u16::from_le_bytes([cd.data[6], cd.data[7]])
                    );
                    println!(
                        "{:^indent$}  bmNetworkCapabilities 0x{:02x}",
                        "", cd.data[8]
                    );
                    if cd.data[8] & 0x20 != 0x00 {
                        println!("{:^indent$}    8-byte ntb input size", "");
                    }
                    if cd.data[8] & 0x08 != 0x00 {
                        println!("{:^indent$}    max cd.datagram size", "");
                    }
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            usb::CdcType::MbimExtended => {
                if cd.data.len() >= 5 {
                    println!("{:^indent$}CDC MBIM Extended:", "");
                    println!(
                        "{:^indent$}  bcdMBIMExtendedVersion          {:x}.{:02x}",
                        "", cd.data[1], cd.data[0]
                    );
                    println!(
                        "{:^indent$}  bMaxOutstandingCommandMessages    {:3}",
                        "", cd.data[2]
                    );
                    println!(
                        "{:^indent$}  wMTU                            {:5}",
                        "",
                        u16::from_le_bytes([cd.data[3], cd.data[4]])
                    );
                } else {
                    dump_bad_comm(cd, indent);
                }
            }
            _ => {
                println!(
                    "{:^indent$}UNRECOGNIZED CDC: {}",
                    "",
                    Vec::<u8>::from(cd.to_owned())
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<String>>()
                        .join(" "),
                    indent = indent
                );
            }
        }
    }

    fn dump_dfu_interface(gd: &usb::GenericDescriptor) {
        println!("      Device Firmware Upgrade Interface Descriptor:");
        println!("        bLength                        {:5}", gd.length);
        println!(
            "        bDescriptorType                {:5}",
            gd.descriptor_type
        );
        println!(
            "        bmAttributes                   {:5}",
            gd.descriptor_subtype
        );

        if gd.descriptor_subtype & 0xf0 != 0 {
            println!("          (unknown attributes!)");
        }
        if gd.descriptor_subtype & 0x08 != 0 {
            println!("          Will Detach");
        } else {
            println!("          Will Not Detach");
        }
        if gd.descriptor_subtype & 0x04 != 0 {
            println!("          Manifestation Tolerant");
        } else {
            println!("          Manifestation Intolerant");
        }
        if gd.descriptor_subtype & 0x02 != 0 {
            println!("          Upload Supported");
        } else {
            println!("          Upload Unsupported");
        }
        if gd.descriptor_subtype & 0x01 != 0 {
            println!("          Download Supported");
        } else {
            println!("          Download Unsupported");
        }

        if let Some(data) = &gd.data {
            if data.len() >= 4 {
                let detach_timeout = u16::from_le_bytes([data[0], data[1]]);
                println!(
                    "        wDetachTimeout                 {:5} milliseconds",
                    detach_timeout
                );
                let transfer_size = u16::from_le_bytes([data[2], data[3]]);
                println!(
                    "        wTransferSize                  {:5} bytes",
                    transfer_size
                );
            }
            if data.len() >= 6 {
                println!(
                    "        bcdDFUVersion                  {:x}.{:02x}",
                    data[4], data[5]
                );
            }
        }
    }

    fn dump_pipe_desc(gd: &usb::GenericDescriptor) {
        if gd.length == 4 && gd.descriptor_type == 0x24 {
            let subtype_string = match gd.descriptor_subtype {
                1 => "Command pipe",
                2 => "Status pipe",
                3 => "Data-in pipe",
                4 => "Data-out pipe",
                0 | 5..=0xdf | 0xf0..=0xff => "Reserved",
                0xe0..=0xef => "Vendor-specific",
            };

            println!(
                "        {} (0x{:02x})",
                subtype_string, gd.descriptor_subtype
            );
        } else {
            println!(
                "        INTERFACE CLASS: {}",
                Vec::<u8>::from(gd.to_owned())
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join(" ")
            );
        }
    }

    fn dump_security(sec: &usb::SecurityDescriptor) {
        println!("    Security Descriptor:");
        println!("      bLength              {:5}", sec.length);
        println!("      bDescriptorType      {:5}", sec.descriptor_type);
        println!("      wTotalLength        0x{:04x}", sec.total_length);
        println!("      bNumEncryptionTypes  {:5}", sec.encryption_types);
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
        println!("          bcdHID               {}", hidd.bcd_hid);
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

    /// Verbatum port of lsusb's dump_unit - not very Rust, don't judge!
    fn dump_unit(mut data: u16, len: usize) {
        let systems = |t: u16| match t {
            0x01 => "SI Linear",
            0x02 => "SI Rotation",
            0x03 => "English Linear",
            0x04 => "English Rotation",
            _ => "None",
        };
        let units = |t: u16, i: usize| match (t, i) {
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
                0x06 => "Camdela",
                _ => "None",
            },
            (_, _) => "None",
        };

        let sys = data & 0xf;
        data >>= 4;

        if sys > 4 {
            if sys == 0xf {
                println!("System: Vendor defined, Unit: (unknown)");
            } else {
                println!("System: Reserved, Unit: (unknown)");
            }

            return;
        }

        print!("System: {}, Unit: ", systems(sys));

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
                    print!("^{}", val);
                }
            }
        }

        if earlier_unit == 0 {
            print!("(None)");
        }
        println!();
    }

    // ported directly from lsusb - it's not pretty but works...
    fn dump_report_desc(desc: &[u8], indent: usize) {
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
                0x64 => {
                    print!("{:^indent$}", "", indent = indent);
                    dump_unit(data, bsize)
                }
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
