use crate::usb::descriptors::bos;

use super::*;

pub fn dump_extension_capability(d: &bos::ExtensionCapability, indent: usize) {
    dump_string("USB 2.0 Extension Device Capability:", indent);
    dump_value(d.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(d.descriptor_type, "bDescriptorType", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(u8::from(d.capability_type.to_owned()), "bDevCapabilityType", indent + 2, LSUSB_DUMP_WIDTH);
    dump_hex(d.attributes, "bmAttributes", indent + 2, LSUSB_DUMP_WIDTH);

    if d.attributes & 0x02 == 0 {
        dump_string("(Missing must-be-set LPM bit!)", indent + 4);
    } else if d.attributes & 0x04 == 0 {
        dump_string("HIRD Link Power Management (LPM) Supported", indent + 4);
    } else {
        dump_string("BESL Link Power Management (LPM) Supported", indent + 4);
    }
    if d.attributes & 0x08 != 0 {
        let val = d.attributes & 0xf00;
        dump_value_string(val, "BESL value", "us", indent + 4, LSUSB_DUMP_WIDTH);
    }
    if d.attributes & 0x10 != 0 {
        let val = d.attributes & 0xf000;
        dump_value_string(val, "Deep BESL value", "us", indent + 4, LSUSB_DUMP_WIDTH);
    }
}

pub(crate) fn dump_bos_descriptor(bosd: &bos::BinaryObjectStoreDescriptor, indent: usize) {
    for cap in &bosd.capabilities {
        match cap {
            bos::BosCapability::Usb2Extension(d) => {
                dump_extension_capability(d, indent);
            }
            _ => (),
        }
    }
}
