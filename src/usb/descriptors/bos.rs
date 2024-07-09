//! Binary Object Store (BOS) descriptor types and capabilities parsing
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::{uuid, Uuid};

use super::*;
use crate::error::{self, Error, ErrorKind};

const WEBUSB_GUID: Uuid = uuid!("{3408b638-09a9-47a0-8bfd-a0768815b665}");

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
    Usb2Extension(ExtensionCapability),
    SuperSpeed(SuperSpeedCapability),
    SuperSpeedPlus(SuperSpeedPlusCapability),
    Billboard(BillboardCapability),
    BillboardAltMode(BillboardAltModeCapability),
    ConfigurationSummary(ConfigurationSummaryCapability),
    ContainerId(ContainerIdCapability),
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
            BosType::Usb2Extension => Ok(BosCapability::Usb2Extension(
                ExtensionCapability::try_from(value)?,
            )),
            BosType::SuperSpeed => Ok(BosCapability::SuperSpeed(SuperSpeedCapability::try_from(
                value,
            )?)),
            BosType::SuperSpeedPlus => Ok(BosCapability::SuperSpeedPlus(
                SuperSpeedPlusCapability::try_from(value)?,
            )),
            BosType::Billboard => Ok(BosCapability::Billboard(BillboardCapability::try_from(
                value,
            )?)),
            BosType::BillboardAltMode => Ok(BosCapability::BillboardAltMode(
                BillboardAltModeCapability::try_from(value)?,
            )),
            BosType::ConfigurationSummary => Ok(BosCapability::ConfigurationSummary(
                ConfigurationSummaryCapability::try_from(value)?,
            )),
            BosType::ContainerId => Ok(BosCapability::ContainerId(
                ContainerIdCapability::try_from(value)?,
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
            BosCapability::Usb2Extension(ebd) => Vec::<u8>::from(ebd),
            BosCapability::SuperSpeed(ssc) => Vec::<u8>::from(ssc),
            BosCapability::SuperSpeedPlus(sspc) => Vec::<u8>::from(sspc),
            BosCapability::Billboard(bc) => Vec::<u8>::from(bc),
            BosCapability::BillboardAltMode(bac) => Vec::<u8>::from(bac),
            BosCapability::ConfigurationSummary(ucs) => Vec::<u8>::from(ucs),
            BosCapability::ContainerId(cic) => Vec::<u8>::from(cic),
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
        // probably a Rustier way to do this with drain but this works..
        // already checked that the total length is correct
        while offset < total_length as usize {
            let cd_len = value[offset] as usize;
            if value.len() < offset + cd_len {
                // break if we're going to read past the end of the buffer rather than Err so all is not lost...
                log::warn!("BOS capability has invalid length, breaking");
                break;
            }
            match BosCapability::try_from(&value[offset..offset + cd_len]) {
                Ok(c) => capabilities.push(c),
                // allow to continue parsing even if one fails
                Err(e) => log::warn!("Failed to parse BOS capability: {:?}", e),
            }
            offset += cd_len;
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
    pub compatibility_type: u8,
    pub reserved: u8,
    pub guid: Uuid,
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
            compatibility_type: value[2],
            reserved: value[3],
            //guid: get_guid(&value[4..20])?,
            guid: Uuid::from_slice_le(&value[4..20]).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidArg,
                    "Platform Device Compatibility descriptor has invalid GUID",
                )
            })?,
            data: value[20..].to_vec(),
        })
    }
}

impl From<PlatformDeviceCompatibility> for Vec<u8> {
    fn from(pdc: PlatformDeviceCompatibility) -> Self {
        let mut ret = vec![
            pdc.length,
            pdc.descriptor_type,
            pdc.compatibility_type,
            pdc.reserved,
        ];
        ret.extend(pdc.guid.to_bytes_le());
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
        // platform has all the data in data field
        wpc.platform.into()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ExtensionCapability {
    pub length: u8,
    pub descriptor_type: u8,
    pub capability_type: BosType,
    pub attributes: u32,
}

impl TryFrom<&[u8]> for ExtensionCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Extension BOS descriptor too short",
            ));
        }

        Ok(ExtensionCapability {
            length: value[0],
            descriptor_type: value[1],
            capability_type: value[2].into(),
            attributes: u32::from_le_bytes([value[3], value[4], value[5], value[6]]),
        })
    }
}

impl From<ExtensionCapability> for Vec<u8> {
    fn from(ebd: ExtensionCapability) -> Self {
        let mut ret = Vec::new();
        ret.push(ebd.length);
        ret.push(ebd.descriptor_type);
        ret.push(u8::from(ebd.capability_type));
        ret.extend(ebd.attributes.to_le_bytes());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SuperSpeedCapability {
    pub length: u8,
    pub descriptor_type: u8,
    pub capability_type: BosType,
    pub attributes: u8,
    pub speed_supported: u16,
    pub functionality_supported: u8,
    pub u1_device_exit_latency: u8,
    pub u2_device_exit_latency: u16,
}

impl TryFrom<&[u8]> for SuperSpeedCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "SuperSpeed BOS descriptor too short",
            ));
        }

        Ok(SuperSpeedCapability {
            length: value[0],
            descriptor_type: value[1],
            capability_type: value[2].into(),
            attributes: value[3],
            speed_supported: u16::from_le_bytes([value[4], value[5]]),
            functionality_supported: value[6],
            u1_device_exit_latency: value[7],
            u2_device_exit_latency: u16::from_le_bytes([value[8], value[9]]),
        })
    }
}

impl From<SuperSpeedCapability> for Vec<u8> {
    fn from(ssc: SuperSpeedCapability) -> Self {
        let mut ret = vec![
            ssc.length,
            ssc.descriptor_type,
            u8::from(ssc.capability_type),
            ssc.attributes,
        ];
        ret.extend(ssc.speed_supported.to_le_bytes());
        ret.push(ssc.functionality_supported);
        ret.push(ssc.u1_device_exit_latency);
        ret.extend(ssc.u2_device_exit_latency.to_le_bytes());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SuperSpeedPlusCapability {
    pub length: u8,
    pub descriptor_type: u8,
    pub capability_type: BosType,
    pub attributes: u32,
    pub functionality_supported: u16,
    pub sublink_attributes: Vec<u32>,
}

impl TryFrom<&[u8]> for SuperSpeedPlusCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 12 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "SuperSpeedPlus BOS descriptor too short",
            ));
        }

        let sublink_speed_attr_count = (value[4] & 0x1f) as usize + 1;
        let mut sublink_attributes = Vec::with_capacity(sublink_speed_attr_count);

        if value.len() < 12 + sublink_speed_attr_count * 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "SuperSpeedPlus BOS descriptor too short for sublink speed attributes",
            ));
        }

        for chunk in value[12..12 + sublink_speed_attr_count].chunks_exact(4) {
            sublink_attributes.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }

        Ok(SuperSpeedPlusCapability {
            length: value[0],
            descriptor_type: value[1],
            capability_type: value[2].into(),
            attributes: u32::from_le_bytes([value[4], value[5], value[6], value[7]]),
            functionality_supported: u16::from_le_bytes([value[8], value[9]]),
            sublink_attributes,
        })
    }
}

impl From<SuperSpeedPlusCapability> for Vec<u8> {
    fn from(sspc: SuperSpeedPlusCapability) -> Self {
        let mut ret = Vec::new();
        ret.push(sspc.length);
        ret.push(sspc.descriptor_type);
        ret.push(u8::from(sspc.capability_type));
        ret.extend(sspc.attributes.to_le_bytes());
        ret.extend(sspc.functionality_supported.to_le_bytes());
        for attr in sspc.sublink_attributes {
            ret.extend(attr.to_le_bytes());
        }

        ret
    }
}

impl SuperSpeedPlusCapability {
    /// Returns the number of sublink speed attributes supported by this device.
    pub fn sublink_speed_attribute_count(&self) -> usize {
        self.attributes as usize & (0x1f + 1)
    }

    /// Returns the number of sublink speed IDs supported by this device.
    pub fn sublink_speed_id_count(&self) -> usize {
        (self.attributes as usize) >> 5 & (0xf + 1)
    }

    /// Returns the minimum functional speed attribute ID supported by this device.
    pub fn functional_speed_attribute_id(&self) -> usize {
        (self.functionality_supported as usize) & 0x0f
    }

    /// Returns the minimum number of functional RX lanes supported by this device.
    pub fn functional_rx_lanes(&self) -> usize {
        (self.functionality_supported as usize) >> 8 & 0x0f
    }

    /// Returns the minimum number of functional TX lanes supported by this device.
    pub fn functional_tx_lanes(&self) -> usize {
        (self.functionality_supported as usize) >> 12 & 0x0f
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BillboardCapability {
    pub length: u8,
    pub descriptor_type: u8,
    pub capability_type: BosType,
    pub additional_info_url_index: u8,
    pub additional_info_url: Option<String>,
    pub number_of_alternate_modes: u8,
    pub preferred_alternate_mode: u8,
    pub vconn_power: u16,
    pub configured: [u8; 32],
    pub version: Version,
    pub additional_failure_info: u8,
    pub reserved: u8,
    pub alternate_modes: Vec<AlternateMode>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AlternateMode {
    pub svid: u16,
    pub alternate_mode: u8,
    pub alternate_mode_string_index: u8,
    pub alternate_mode_string: Option<String>,
}

impl TryFrom<&[u8]> for BillboardCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 48 {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Billboard Capability descriptor too short",
            ));
        }

        let number_of_alternate_modes = value[4];
        if number_of_alternate_modes > 0x34 {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Invalid number of alternate modes in Billboard Capability descriptor",
            ));
        }

        if value.len() < (44 + number_of_alternate_modes as usize * 4) {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Billboard Capability descriptor too short for alternate modes",
            ));
        }

        let mut alternate_modes = Vec::with_capacity(number_of_alternate_modes as usize);
        let mut i = 44;
        for _ in 0..number_of_alternate_modes {
            alternate_modes.push(AlternateMode {
                svid: u16::from_le_bytes([value[i], value[i + 1]]),
                alternate_mode: value[i + 2],
                alternate_mode_string_index: value[i + 3],
                alternate_mode_string: None,
            });
            i += 4;
        }

        let version = if value[41] == 0 {
            Version::from_bcd(u16::from_le_bytes([value[40], 1]))
        } else {
            Version::from_bcd(u16::from_le_bytes([value[40], value[41]]))
        };

        Ok(BillboardCapability {
            length: value[0],
            descriptor_type: value[1],
            capability_type: value[2].into(),
            additional_info_url_index: value[3],
            additional_info_url: None,
            number_of_alternate_modes,
            preferred_alternate_mode: value[5],
            vconn_power: u16::from_le_bytes([value[6], value[7]]),
            configured: value[8..40].try_into().expect("bmConfigured slice error"),
            version,
            additional_failure_info: value[42],
            reserved: value[43],
            alternate_modes,
        })
    }
}

impl From<BillboardCapability> for Vec<u8> {
    fn from(bc: BillboardCapability) -> Self {
        let mut ret = vec![
            bc.length,
            bc.descriptor_type,
            u8::from(bc.capability_type),
            bc.additional_info_url_index,
            bc.number_of_alternate_modes,
            bc.preferred_alternate_mode,
        ];
        ret.extend_from_slice(&bc.vconn_power.to_le_bytes());
        ret.extend_from_slice(&bc.configured);
        ret.extend_from_slice(&(u16::from(bc.version)).to_le_bytes());
        ret.push(bc.additional_failure_info);
        ret.push(bc.reserved);

        for alt_mode in bc.alternate_modes {
            ret.extend_from_slice(&alt_mode.svid.to_le_bytes());
            ret.push(alt_mode.alternate_mode);
            ret.push(alt_mode.alternate_mode_string_index);
        }

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BillboardAltModeCapability {
    pub length: u8,
    pub descriptor_type: u8,
    pub capability_type: BosType,
    pub index: u8,
    pub alternate_mode_vdo: u32,
}

impl TryFrom<&[u8]> for BillboardAltModeCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Billboard Alt Mode Capability descriptor has invalid length",
            ));
        }

        Ok(BillboardAltModeCapability {
            length: value[0],
            descriptor_type: value[1],
            capability_type: value[2].into(),
            index: value[3],
            alternate_mode_vdo: u32::from_le_bytes([value[4], value[5], value[6], value[7]]),
        })
    }
}

impl From<BillboardAltModeCapability> for Vec<u8> {
    fn from(bac: BillboardAltModeCapability) -> Self {
        let mut ret = vec![
            bac.length,
            bac.descriptor_type,
            u8::from(bac.capability_type),
            bac.index,
        ];
        ret.extend_from_slice(&bac.alternate_mode_vdo.to_le_bytes());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ContainerIdCapability {
    pub length: u8,
    pub descriptor_type: u8,
    pub capability_type: BosType,
    pub reserved: u8,
    pub container_id: Uuid,
}

impl TryFrom<&[u8]> for ContainerIdCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 20 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Container ID BOS descriptor too short",
            ));
        }

        Ok(ContainerIdCapability {
            length: value[0],
            descriptor_type: value[1],
            capability_type: value[2].into(),
            reserved: value[3],
            container_id: Uuid::from_slice_le(&value[4..20]).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidArg,
                    "Container ID BOS descriptor has invalid GUID",
                )
            })?,
        })
    }
}

impl From<ContainerIdCapability> for Vec<u8> {
    fn from(cic: ContainerIdCapability) -> Self {
        let mut ret = vec![
            cic.length,
            cic.descriptor_type,
            u8::from(cic.capability_type),
            cic.reserved,
        ];
        ret.extend(cic.container_id.to_bytes_le());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ConfigurationSummaryCapability {
    pub length: u8,
    pub descriptor_type: u8,
    pub capability_type: BosType,
    pub version: Version,
    pub class: u8,
    pub sub_class: u8,
    pub protocol: u8,
    pub configuration_count: u8,
    pub configurations: u8,
    pub configured: Vec<u8>,
}

impl TryFrom<&[u8]> for ConfigurationSummaryCapability {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "USB 3.0 Configuration Summary BOS descriptor too short",
            ));
        }

        let configured_count = value[7];
        if value.len() < 10 + configured_count as usize {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "USB 3.0 Configuration Summary BOS descriptor too short for configured",
            ));
        }

        Ok(ConfigurationSummaryCapability {
            length: value[0],
            descriptor_type: value[1],
            capability_type: value[2].into(),
            version: Version::from_bcd(u16::from_le_bytes([value[3], value[4]])),
            class: value[5],
            sub_class: value[6],
            protocol: value[7],
            configuration_count: configured_count,
            configurations: value[8],
            configured: value[9..].to_vec(),
        })
    }
}

impl From<ConfigurationSummaryCapability> for Vec<u8> {
    fn from(ucs: ConfigurationSummaryCapability) -> Self {
        let mut ret = Vec::new();
        ret.push(ucs.length);
        ret.push(ucs.descriptor_type);
        ret.push(u8::from(ucs.capability_type));
        ret.extend_from_slice(&u16::from(ucs.version).to_le_bytes());
        ret.push(ucs.class);
        ret.push(ucs.sub_class);
        ret.push(ucs.protocol);
        ret.push(ucs.configuration_count);
        ret.push(ucs.configurations);
        ret.extend(ucs.configured);

        ret
    }
}
