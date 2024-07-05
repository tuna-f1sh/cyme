//! Binary Object Store (BOS) descriptor types and capabilities parsing
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use super::*;
use crate::error::{self, Error, ErrorKind};

const WEBUSB_GUID: &str = "{3408b638-09a9-47a0-8bfd-a0768815b665}";

/// The Binary Object Store descriptor type codes as defined in the USB 3.0 spec.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum BosType {
    WirelessUsb = 0x01,
    Usb2Extension = 0x02,
    SuperSpeed = 0x03,
    ContainerId = 0x04,
    PlatformCapability = 0x05,
    SuperSpeedPlus = 0x0a,
    Billboard = 0x0d,
    BillboardAltMode = 0x0f,
    ConfigurationSummary = 0x10,
    Unknown(u8),
}

impl From<u8> for BosType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => BosType::WirelessUsb,
            0x02 => BosType::Usb2Extension,
            0x03 => BosType::SuperSpeed,
            0x04 => BosType::ContainerId,
            0x05 => BosType::PlatformCapability,
            0x0a => BosType::SuperSpeedPlus,
            0x0d => BosType::Billboard,
            0x0f => BosType::BillboardAltMode,
            0x10 => BosType::ConfigurationSummary,
            _ => BosType::Unknown(value),
        }
    }
}

impl From<BosType> for u8 {
    fn from(value: BosType) -> Self {
        match value {
            BosType::WirelessUsb => 0x01,
            BosType::Usb2Extension => 0x02,
            BosType::SuperSpeed => 0x03,
            BosType::ContainerId => 0x04,
            BosType::PlatformCapability => 0x05,
            BosType::SuperSpeedPlus => 0x0a,
            BosType::Billboard => 0x0d,
            BosType::BillboardAltMode => 0x0f,
            BosType::ConfigurationSummary => 0x10,
            BosType::Unknown(v) => v,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum BosCapability {
    Generic(GenericCapability),
    Platform(PlatformDeviceCompatibility),
    WebUsbPlatform(WebUsbPlatformCapability),
}

impl TryFrom<&[u8]> for BosCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "BOS capability descriptor too short",
            ));
        }

        match value[2].into() {
            BosType::Unknown(_) => Err(Error::new(
                ErrorKind::InvalidArg,
                "BOS capability descriptor has unknown capability type",
            )),
            BosType::PlatformCapability => {
                let pdc = PlatformDeviceCompatibility::try_from(value)?;
                // WebUSB is a special case of PlatformCapability with a specific GUID: https://developer.chrome.com/docs/capabilities/build-for-webusb
                if pdc.guid == WEBUSB_GUID {
                    Ok(BosCapability::WebUsbPlatform(
                        WebUsbPlatformCapability::try_from(value)?,
                    ))
                } else {
                    Ok(BosCapability::Platform(pdc))
                }
            }
            // TODO implement rest of types
            _ => Ok(BosCapability::Generic(GenericCapability::try_from(value)?)),
        }
    }
}

impl From<BosCapability> for Vec<u8> {
    fn from(bcd: BosCapability) -> Self {
        match bcd {
            BosCapability::Generic(gcd) => Vec::<u8>::from(gcd),
            BosCapability::Platform(pdc) => Vec::<u8>::from(pdc),
            BosCapability::WebUsbPlatform(wpc) => Vec::<u8>::from(wpc),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BinaryObjectStoreDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_device_capabilities: u8,
    pub capabilities: Vec<BosCapability>,
}

impl TryFrom<&[u8]> for BinaryObjectStoreDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Binary Object Store descriptor too short",
            ));
        }

        let length = value[0];
        let descriptor_type = value[1];
        let total_length = u16::from_le_bytes([value[2], value[3]]);
        let num_device_capabilities = value[4];

        if value.len() < total_length as usize {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Binary Object Store descriptor reported length too long for data returned",
            ));
        }

        if total_length <= 5 && value[4] > 0 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Binary Object Store descriptor reported num_device_capabilities but no data",
            ));
        }

        let mut capabilities = Vec::new();
        let mut offset = 5;
        let mut cd_len = value[offset] as usize;
        while offset < total_length as usize && value.len() >= offset + cd_len {
            match BosCapability::try_from(&value[offset..offset + cd_len]) {
                Ok(c) => capabilities.push(c),
                Err(e) => log::warn!("Failed to parse BOS capability: {:?}", e),
            }
            offset += cd_len;
            cd_len = value[offset] as usize;
        }

        Ok(BinaryObjectStoreDescriptor {
            length,
            descriptor_type,
            total_length,
            num_device_capabilities,
            capabilities,
        })
    }
}

impl From<BinaryObjectStoreDescriptor> for Vec<u8> {
    fn from(bosd: BinaryObjectStoreDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(bosd.length);
        ret.push(bosd.descriptor_type);
        ret.extend(bosd.total_length.to_le_bytes());
        ret.push(bosd.num_device_capabilities);
        for cap in bosd.capabilities {
            ret.extend(Vec::<u8>::from(cap));
        }

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct GenericCapability {
    pub length: u8,
    pub descriptor_type: u8,
    pub capability_type: BosType,
    pub data: Vec<u8>,
}

impl TryFrom<&[u8]> for GenericCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Generic BOS descriptor too short",
            ));
        }

        Ok(GenericCapability {
            length: value[0],
            descriptor_type: value[1],
            capability_type: value[2].into(),
            data: value[3..].to_vec(),
        })
    }
}

impl From<GenericCapability> for Vec<u8> {
    fn from(gbd: GenericCapability) -> Self {
        let mut ret = Vec::new();
        ret.push(gbd.length);
        ret.push(gbd.descriptor_type);
        ret.push(u8::from(gbd.capability_type));
        ret.extend(gbd.data);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct PlatformDeviceCompatibility {
    pub length: u8,
    pub descriptor_type: u8,
    pub compatibility_descriptor: u8,
    pub reserved: u8,
    pub guid: String,
    pub data: Vec<u8>,
}

impl TryFrom<&[u8]> for PlatformDeviceCompatibility {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 20 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Platform Device Compatibility descriptor too short",
            ));
        }

        Ok(PlatformDeviceCompatibility {
            length: value[0],
            descriptor_type: value[1],
            compatibility_descriptor: value[2],
            reserved: value[3],
            guid: get_guid(&value[4..20])?,
            data: value[20..].to_vec(),
        })
    }
}

impl From<PlatformDeviceCompatibility> for Vec<u8> {
    fn from(pdc: PlatformDeviceCompatibility) -> Self {
        let mut ret = vec![
            pdc.length,
            pdc.descriptor_type,
            pdc.compatibility_descriptor,
            pdc.reserved,
        ];
        ret.extend(&guid_to_bytes(&pdc.guid).unwrap());
        ret.extend(pdc.data);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct WebUsbPlatformCapability {
    pub platform: PlatformDeviceCompatibility,
    pub version: Version,
    pub vendor_code: u8,
    pub landing_page_index: u8,
    pub url: Option<String>,
}

impl TryFrom<&[u8]> for WebUsbPlatformCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 24 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "WebUSB Platform Capability descriptor too short",
            ));
        }

        let platform = PlatformDeviceCompatibility::try_from(value)?;

        Ok(WebUsbPlatformCapability {
            platform,
            version: Version::from_bcd(u16::from_le_bytes([value[20], value[21]])),
            vendor_code: value[22],
            landing_page_index: value[23],
            url: None,
        })
    }
}

impl From<WebUsbPlatformCapability> for Vec<u8> {
    fn from(wpc: WebUsbPlatformCapability) -> Self {
        let mut ret = Vec::new();
        ret.extend(Vec::<u8>::from(wpc.platform));
        ret.push(u16::from(wpc.version).to_le_bytes()[0]);
        ret.push(wpc.vendor_code);
        ret.push(wpc.landing_page_index);

        ret
    }
}
