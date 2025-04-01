use crate::usb::descriptors::bos;

use super::*;

fn dump_extension_capability(d: &bos::ExtensionCapability, lpm_requred: bool, indent: usize) {
    const BSEL_US: [u16; 16] = [
        125, 150, 200, 300, 400, 500, 1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000,
    ];
    dump_string("USB 2.0 Extension Device Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        u8::from(d.capability_type.to_owned()),
        "bDevCapabilityType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(d.attributes, "bmAttributes", indent + 2, LSUSB_DUMP_WIDTH);

    if (lpm_requred || (d.attributes & 0x04 == 0x04)) && d.attributes & 0x02 == 0 {
        dump_string("(Missing must-be-set LPM bit!)", indent + 4);
    } else if !lpm_requred && d.attributes & 0x02 == 0 {
        dump_string("Link Power Management (LPM) not supported", indent + 4);
    } else if d.attributes & 0x04 == 0 {
        dump_string("HIRD Link Power Management (LPM) Supported", indent + 4);
    } else {
        dump_string("BESL Link Power Management (LPM) Supported", indent + 4);
        if d.attributes & 0x08 != 0 {
            let val = ((d.attributes & 0xf00) >> 8) as usize;
            dump_value_string(
                BSEL_US[val],
                "BESL value",
                "us",
                indent + 4,
                LSUSB_DUMP_WIDTH,
            );
        }
        if d.attributes & 0x10 != 0 {
            let val = ((d.attributes & 0xf000) >> 12) as usize;
            dump_value_string(
                BSEL_US[val],
                "Deep BESL value",
                "us",
                indent + 4,
                LSUSB_DUMP_WIDTH,
            );
        }
    }
}

fn dump_ss_capability(d: &bos::SuperSpeedCapability, indent: usize) {
    dump_string("SuperSpeed USB Device Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        u8::from(d.capability_type.to_owned()),
        "bDevCapabilityType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(d.attributes, "bmAttributes", indent + 2, LSUSB_DUMP_WIDTH);
    dump_hex(
        d.speed_supported,
        "wSpeedsSupported",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_bitmap_strings(
        d.speed_supported,
        |b| match b {
            0 => Some("Device can operate at Low Speed (1Mbps)"),
            1 => Some("Device can operate at Full Speed (12Mbps)"),
            2 => Some("Device can operate at High Speed (480Mbps)"),
            3 => Some("Device can operate at SuperSpeed (5Gbps)"),
            _ => None,
        },
        indent + 4,
    );
    dump_value(
        d.functionality_supported,
        "bFunctionalitySupport",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_bitmap_strings(
        d.functionality_supported,
        |b| match b {
            0 => Some("Lowest fully-functional device speed is Low Speed (1Mbps)"),
            1 => Some("Lowest fully-functional device speed is Full Speed (12Mbps)"),
            2 => Some("Lowest fully-functional device speed is High Speed (480Mbps)"),
            3 => Some("Lowest fully-functional device speed is SuperSpeed (5Gbps)"),
            _ => Some("Lowest fully-functional device speed is at an unknown speed!"),
        },
        indent + 4,
    );
    dump_value_string(
        d.u1_device_exit_latency,
        "bU1DevExitLat",
        "micro seconds",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        d.u2_device_exit_latency,
        "bU2DevExitLat",
        "micro seconds",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
}

fn dump_ss_plus_capability(d: &bos::SuperSpeedPlusCapability, indent: usize) {
    dump_string("SuperSpeedPlus USB Device Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        u8::from(d.capability_type.to_owned()),
        "bDevCapabilityType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(d.attributes, "bmAttributes", indent + 2, LSUSB_DUMP_WIDTH);
    dump_string(
        &format!(
            "Sublink Speed Attribute count {}",
            d.sublink_speed_attribute_count()
        ),
        indent + 4,
    );
    dump_string(
        &format!("Sublink Speed ID count {}", d.sublink_speed_id_count()),
        indent + 4,
    );
    dump_hex(
        d.functionality_supported,
        "wFunctionalitySupport",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_string(
        &format!(
            "Min functional Speed Attribute ID: {}",
            d.functional_speed_attribute_id()
        ),
        indent + 4,
    );
    dump_string(
        &format!("Min functional RX lanes: {}", d.functional_rx_lanes()),
        indent + 4,
    );
    dump_string(
        &format!("Min functional TX lanes: {}", d.functional_tx_lanes()),
        indent + 4,
    );

    let bitrate_prefix = [' ', 'K', 'M', 'G'];

    for (i, &ss_attr) in d.sublink_attributes.iter().enumerate() {
        dump_hex(
            ss_attr,
            &format!("bmSublinkSpeedAttr[{}]", i),
            indent,
            LSUSB_DUMP_WIDTH,
        );
        dump_string(
            &format!(
                "Speed Attribute ID: {} {}{}b/s {} {} SuperSpeed{}",
                ss_attr & 0x0f,
                ss_attr >> 16,
                bitrate_prefix[((ss_attr >> 4) & 0x3) as usize],
                if (ss_attr & 0x40) != 0 {
                    "Asymmetric"
                } else {
                    "Symmetric"
                },
                if (ss_attr & 0x80) != 0 { "TX" } else { "RX" },
                if (ss_attr & 0x4000) != 0 { "Plus" } else { "" },
            ),
            indent + 4,
        );
    }
}

const VCONN_POWER_STRINGS: [&str; 8] = ["1W", "1.5W", "2W", "3W", "4W", "5W", "6W", "reserved"];

const ALT_MODE_STATE: [&str; 4] = [
    "Unspecified Error",
    "Alternate Mode configuration not attempted",
    "Alternate Mode configuration attempted but unsuccessful",
    "Alternate Mode configuration successful",
];

fn dump_billboard_capability(d: &bos::BillboardCapability, indent: usize) {
    let vconn = if d.vconn_power & (1 << 15) != 0 {
        "VCONN power not required"
    } else if (d.vconn_power & 0x7) < 7 {
        VCONN_POWER_STRINGS[(d.vconn_power & 0x7) as usize]
    } else {
        "reserved"
    };

    dump_string("Billboard Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        u8::from(d.capability_type.to_owned()),
        "bDevCapabilityType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        d.additional_info_url_index,
        "iAdditionalInfoURL",
        d.additional_info_url.as_ref().unwrap_or(&String::new()),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        d.number_of_alternate_modes,
        "bNumberOfAlternateModes",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        d.preferred_alternate_mode,
        "bPreferredAlternateMode",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        d.vconn_power,
        "VCONN Power",
        vconn,
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    let bytes_string = d
        .configured
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join(" ");
    dump_value(bytes_string, "bmConfigured", indent + 2, LSUSB_DUMP_WIDTH);

    dump_value(d.version, "bcdVersion", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.additional_failure_info,
        "bAdditionalFailureInfo",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(d.reserved, "bReserved", indent + 2, LSUSB_DUMP_WIDTH);

    dump_string("Alternate Modes supported by Device Container:", indent + 2);
    for (alt_mode, am) in d.alternate_modes.iter().enumerate() {
        let state = ((d.configured[alt_mode >> 2] >> ((alt_mode & 0x3) << 1)) & 0x3) as usize;
        dump_string(
            &format!("Alternate Mode {} : {}", alt_mode, ALT_MODE_STATE[state]),
            indent + 2,
        );
        dump_hex(
            am.svid,
            &format!("wSVID[{}]", alt_mode),
            indent + 4,
            LSUSB_DUMP_WIDTH,
        );
        dump_value(
            am.alternate_mode,
            &format!("bAlternateMode[{}]", alt_mode),
            indent + 4,
            LSUSB_DUMP_WIDTH,
        );
        dump_value_string(
            am.alternate_mode_string_index,
            &format!("iAlternateModeString[{}]", alt_mode),
            am.alternate_mode_string.as_ref().unwrap_or(&String::new()),
            indent + 4,
            LSUSB_DUMP_WIDTH,
        );
    }
}

fn dump_billboard_alt_mode_capability(d: &bos::BillboardAltModeCapability, indent: usize) {
    dump_string("Billboard Alternate Mode Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        u8::from(d.capability_type.to_owned()),
        "bDevCapabilityType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(d.index, "bIndex", indent + 2, LSUSB_DUMP_WIDTH);
    dump_hex(
        d.alternate_mode_vdo,
        "dwAlternateModeVdo",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
}

fn dump_platform_device_capability(
    d: &bos::PlatformDeviceCompatibility,
    data: bool,
    indent: usize,
) {
    dump_string("Platform Device Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        d.compatibility_type.to_owned(),
        "bDevCapabilityType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(d.reserved, "bReserved", indent + 2, LSUSB_DUMP_WIDTH);
    dump_guid(
        &d.guid,
        "PlatformCapabilityUUID",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    // Dump the data if requested
    if data {
        for (i, b) in d.data.iter().enumerate() {
            dump_hex(
                *b,
                &format!("CapabilityData[{}]", i),
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
        }
    }
}

fn dump_webusb_platform_capability(d: &bos::WebUsbPlatformCapability, indent: usize) {
    dump_platform_device_capability(&d.platform, false, indent);
    dump_string("WebUSB", indent + 4);
    dump_value(d.version, "bcdVersion", indent + 6, LSUSB_DUMP_WIDTH);
    dump_value(d.vendor_code, "bVendorCode", indent + 6, LSUSB_DUMP_WIDTH);
    dump_value_string(
        d.landing_page_index,
        "iLandingPage",
        d.url.as_ref().unwrap_or(&String::new()),
        indent + 6,
        LSUSB_DUMP_WIDTH,
    );

    for (i, b) in d.platform.data.iter().enumerate() {
        dump_hex(
            *b,
            &format!("CapabilityData[{}]", i),
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
    }
}

pub fn dump_container_id_capability(d: &bos::ContainerIdCapability, indent: usize) {
    dump_string("Container ID Device Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        u8::from(d.capability_type.to_owned()),
        "bDevCapabilityType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(d.reserved, "bReserved", indent + 2, LSUSB_DUMP_WIDTH);
    dump_guid(&d.container_id, "ContainerID", indent + 2, LSUSB_DUMP_WIDTH);
}

fn dump_usb3_dc_configuration_summary(d: &bos::ConfigurationSummaryCapability, indent: usize) {
    dump_string("Configuration Summary Device Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        u8::from(d.capability_type.to_owned()),
        "bDevCapabilityType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(d.version, "bcdVersion", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(d.class, "bClass", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(d.sub_class, "bSubClass", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(d.protocol, "bProtocol", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        d.configuration_count,
        "bConfigurationCount",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_array(
        &d.configured,
        "bConfigurationIndex",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
}

pub(crate) fn dump_bos_descriptor(
    bosd: &bos::BinaryObjectStoreDescriptor,
    lpm_requred: bool,
    indent: usize,
) {
    dump_string("Binary Object Store Descriptor:", indent);
    dump_value(bosd.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        bosd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_hex(
        bosd.total_length,
        "wTotalLength",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value(
        bosd.num_device_capabilities,
        "bNumDeviceCaps",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    for cap in &bosd.capabilities {
        match cap {
            bos::BosCapability::Usb2Extension(d) => {
                dump_extension_capability(d, lpm_requred, indent + 2);
            }
            bos::BosCapability::SuperSpeed(d) => {
                dump_ss_capability(d, indent + 2);
            }
            bos::BosCapability::SuperSpeedPlus(d) => {
                dump_ss_plus_capability(d, indent + 2);
            }
            bos::BosCapability::Billboard(d) => {
                dump_billboard_capability(d, indent + 2);
            }
            bos::BosCapability::BillboardAltMode(d) => {
                dump_billboard_alt_mode_capability(d, indent + 2);
            }
            bos::BosCapability::ContainerId(d) => {
                dump_container_id_capability(d, indent + 2);
            }
            bos::BosCapability::ConfigurationSummary(d) => {
                dump_usb3_dc_configuration_summary(d, indent + 2);
            }
            bos::BosCapability::Platform(d) => {
                dump_platform_device_capability(d, true, indent + 2);
            }
            bos::BosCapability::WebUsbPlatform(d) => {
                dump_webusb_platform_capability(d, indent + 2);
            }
            _ => {
                let data: Vec<u8> = cap.to_owned().into();
                dump_unrecognised(data.as_slice(), indent + 2);
            }
        }
    }
}
