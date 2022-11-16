/// USB defines ref: https://www.usb.org/defined-class-codes
#[derive(Debug)]
enum DescriptorUsage {
    Device,
    Interface,
    Both,
}

#[derive(Debug)]
enum ClassCode {
    UseInterfaceDescriptor,
    Audio,
    CDCCommunications,
    HID,
    Physical,
    Image,
    Printer,
    MassStorage,
    Hub,
    CDCData,
    SmartCart,
    ContentSecurity,
    Video,
    PersonalHealthcare,
    AudioVideo,
    Billboard,
    USBTypeCBridge,
    I3CDevice,
    Diagnostic,
    WirelessController,
    Miscellaneous,
    ApplicationSpecific,
    VendorSpecific,
}

impl From<u8> for ClassCode {
    fn from(b: u8) -> ClassCode {
        match b {
            0 => ClassCode::UseInterfaceDescriptor,
            1 => ClassCode::Audio,
            2 => ClassCode::CDCCommunications,
            3 => ClassCode::HID,
            5 => ClassCode::Physical,
            6 => ClassCode::Image,
            7 => ClassCode::Printer,
            8 => ClassCode::MassStorage,
            9 => ClassCode::Hub,
            0x0a => ClassCode::CDCData,
            0x0b => ClassCode::SmartCart,
            0x0d => ClassCode::ContentSecurity,
            0x0e => ClassCode::Video,
            0x0f => ClassCode::PersonalHealthcare,
            0x10 => ClassCode::AudioVideo,
            0x11 => ClassCode::Billboard,
            0x12 => ClassCode::USBTypeCBridge,
            0x3c => ClassCode::I3CDevice,
            0xdc => ClassCode::Diagnostic,
            0xe0 => ClassCode::WirelessController,
            0xef => ClassCode::Miscellaneous,
            0xfe => ClassCode::ApplicationSpecific,
            0xff => ClassCode::VendorSpecific,
            _ => ClassCode::UseInterfaceDescriptor
        }
    }
}

impl ClassCode {
    pub fn usage(&self) -> DescriptorUsage {
        match self {
            UseInterfaceDescriptor|Hub|Billboard => DescriptorUsage::Device,
            CDCCommunications|Diagnostic|Miscellaneous|VendorSpecific => DescriptorUsage::Both,
            _ => DescriptorUsage::Interface
        }
    }
}

impl from<ClassCode> for DescriptorUsage {
    fn from(c: ClassCode) -> DescriptorUsage {
        return c.usage();
    }
}
