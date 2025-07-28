//! Defines for the USB Communication Device Class (CDC) descriptors
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::Uuid;

use super::*;
use crate::error::{self, Error, ErrorKind};

/// USB Communication Device Class (CDC) types
///
/// Used to differentiate between different CDC descriptors
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum CdcType {
    Header = 0x00,
    CallManagement = 0x01,
    AbstractControlManagement = 0x02,
    DirectLineManagement = 0x03,
    TelephoneRinger = 0x04,
    TelephoneCall = 0x05,
    Union = 0x06,
    CountrySelection = 0x07,
    TelephoneOperationalModes = 0x08,
    UsbTerminal = 0x09,
    NetworkChannel = 0x0a,
    ProtocolUnit = 0x0b,
    ExtensionUnit = 0x0c,
    MultiChannel = 0x0d,
    CapiControl = 0x0e,
    EthernetNetworking = 0x0f,
    AtmNetworking = 0x10,
    WirelessHandsetControlModel = 0x11,
    MobileDirectLineModelFunctional = 0x12,
    MobileDirectLineModelDetail = 0x13,
    DeviceManagement = 0x14,
    Obex = 0x15,
    CommandSet = 0x16,
    CommandSetDetail = 0x17,
    TelephoneControlModel = 0x18,
    ObexCommandSet = 0x19,
    Ncm = 0x1a,
    Mbim = 0x1b,
    MbimExtended = 0x1c,
    Unknown = 0xff,
}

impl std::fmt::Display for CdcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // lsusb style
        if f.alternate() {
            match self {
                CdcType::Header => write!(f, "Header"),
                CdcType::CallManagement => write!(f, "Call Management"),
                CdcType::AbstractControlManagement => write!(f, "ACM"),
                CdcType::DirectLineManagement => write!(f, "DLM"),
                CdcType::TelephoneRinger => write!(f, "Telephone Ringer"),
                CdcType::TelephoneCall => write!(f, "Telephone Call"),
                CdcType::Union => write!(f, "Union"),
                CdcType::CountrySelection => write!(f, "Country Selection"),
                CdcType::TelephoneOperationalModes => write!(f, "Telephone Operations"),
                CdcType::UsbTerminal => write!(f, "USB Terminal"),
                CdcType::NetworkChannel => write!(f, "Network Channel Terminal"),
                CdcType::ProtocolUnit => write!(f, "Protocol Unit"),
                CdcType::ExtensionUnit => write!(f, "Extension Unit"),
                CdcType::MultiChannel => write!(f, "Multi Channel"),
                CdcType::CapiControl => write!(f, "CAPI Control"),
                CdcType::EthernetNetworking => write!(f, "Ethernet"),
                CdcType::AtmNetworking => write!(f, "ATM Networking"),
                CdcType::WirelessHandsetControlModel => write!(f, "WHCM version"),
                CdcType::MobileDirectLineModelFunctional => {
                    write!(f, "MDLM")
                }
                CdcType::MobileDirectLineModelDetail => write!(f, "MDLM detail"),
                CdcType::DeviceManagement => write!(f, "Device Management"),
                CdcType::Obex => write!(f, "OBEX"),
                CdcType::CommandSet => write!(f, "Command Set"),
                CdcType::CommandSetDetail => write!(f, "Command Set Detail"),
                CdcType::TelephoneControlModel => write!(f, "Telephone Control Model"),
                CdcType::ObexCommandSet => write!(f, "OBEX Command Set"),
                CdcType::Ncm => write!(f, "NCM"),
                CdcType::Mbim => write!(f, "MBIM"),
                CdcType::MbimExtended => write!(f, "MBIM Extended"),
                CdcType::Unknown => write!(f, ""),
            }
        } else {
            write!(f, "{self:?}")
        }
    }
}

impl From<u8> for CdcType {
    fn from(b: u8) -> Self {
        match b {
            0x00 => CdcType::Header,
            0x01 => CdcType::CallManagement,
            0x02 => CdcType::AbstractControlManagement,
            0x03 => CdcType::DirectLineManagement,
            0x04 => CdcType::TelephoneRinger,
            0x05 => CdcType::TelephoneCall,
            0x06 => CdcType::Union,
            0x07 => CdcType::CountrySelection,
            0x08 => CdcType::TelephoneOperationalModes,
            0x09 => CdcType::UsbTerminal,
            0x0a => CdcType::NetworkChannel,
            0x0b => CdcType::ProtocolUnit,
            0x0c => CdcType::ExtensionUnit,
            0x0d => CdcType::MultiChannel,
            0x0e => CdcType::CapiControl,
            0x0f => CdcType::EthernetNetworking,
            0x10 => CdcType::AtmNetworking,
            0x11 => CdcType::WirelessHandsetControlModel,
            0x12 => CdcType::MobileDirectLineModelFunctional,
            0x13 => CdcType::MobileDirectLineModelDetail,
            0x14 => CdcType::DeviceManagement,
            0x15 => CdcType::Obex,
            0x16 => CdcType::CommandSet,
            0x17 => CdcType::CommandSetDetail,
            0x18 => CdcType::TelephoneControlModel,
            0x19 => CdcType::ObexCommandSet,
            0x1a => CdcType::Ncm,
            0x1b => CdcType::Mbim,
            0x1c => CdcType::MbimExtended,
            _ => CdcType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Header {
    pub version: Version,
}

impl TryFrom<&[u8]> for Header {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 2 {
            return Err(Error::new_descriptor_len("Header", 2, value.len()));
        }

        let version = Version::from_bcd(u16::from_le_bytes([value[0], value[1]]));

        Ok(Header { version })
    }
}

impl From<Header> for Vec<u8> {
    fn from(h: Header) -> Self {
        u16::from(h.version).to_le_bytes().to_vec()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CallManagement {
    pub capabilities: u8,
    pub data_interface: u8,
}

impl TryFrom<&[u8]> for CallManagement {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 2 {
            return Err(Error::new_descriptor_len("CallManagement", 2, value.len()));
        }

        Ok(CallManagement {
            capabilities: value[0],
            data_interface: value[1],
        })
    }
}

impl From<CallManagement> for Vec<u8> {
    fn from(cm: CallManagement) -> Self {
        vec![cm.capabilities, cm.data_interface]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AbstractControlManagement {
    pub capabilities: u8,
}

impl TryFrom<&[u8]> for AbstractControlManagement {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.is_empty() {
            return Err(Error::new_descriptor_len(
                "AbstractControlManagement",
                1,
                value.len(),
            ));
        }

        Ok(AbstractControlManagement {
            capabilities: value[0],
        })
    }
}

impl From<AbstractControlManagement> for Vec<u8> {
    fn from(acm: AbstractControlManagement) -> Self {
        vec![acm.capabilities]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Union {
    pub master_interface: u8,
    pub slave_interface: Vec<u8>,
}

impl TryFrom<&[u8]> for Union {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 2 {
            return Err(Error::new_descriptor_len("Union", 2, value.len()));
        }

        Ok(Union {
            master_interface: value[0],
            slave_interface: value[1..].to_vec(),
        })
    }
}

impl From<Union> for Vec<u8> {
    fn from(u: Union) -> Self {
        let mut ret = vec![u.master_interface];
        ret.extend(u.slave_interface);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CountrySelection {
    pub country_code_date_index: u8,
    pub country_code_date: Option<String>,
    pub country_codes: Vec<u16>,
}

impl TryFrom<&[u8]> for CountrySelection {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new_descriptor_len(
                "CountrySelection",
                3,
                value.len(),
            ));
        }

        let country_code_date_index = value[0];
        let country_codes = value[1..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();

        Ok(CountrySelection {
            country_code_date_index,
            country_code_date: None,
            country_codes,
        })
    }
}

impl From<CountrySelection> for Vec<u8> {
    fn from(cs: CountrySelection) -> Self {
        let mut ret = vec![cs.country_code_date_index];
        for code in cs.country_codes {
            ret.extend(code.to_le_bytes().iter());
        }

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TelephoneOperations {
    pub capabilities: u8,
}

impl TryFrom<&[u8]> for TelephoneOperations {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.is_empty() {
            return Err(Error::new_descriptor_len(
                "TelephoneOperations",
                1,
                value.len(),
            ));
        }

        Ok(TelephoneOperations {
            capabilities: value[0],
        })
    }
}

impl From<TelephoneOperations> for Vec<u8> {
    fn from(to: TelephoneOperations) -> Self {
        vec![to.capabilities]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct NetworkChannel {
    pub entity_id: u8,
    pub name_string_index: u8,
    pub name: Option<String>,
    pub channel_index: u8,
    pub physical_interface: u8,
}

impl TryFrom<&[u8]> for NetworkChannel {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("NetworkChannel", 4, value.len()));
        }

        Ok(NetworkChannel {
            entity_id: value[0],
            name_string_index: value[1],
            name: None,
            channel_index: value[2],
            physical_interface: value[3],
        })
    }
}

impl From<NetworkChannel> for Vec<u8> {
    fn from(nc: NetworkChannel) -> Self {
        vec![
            nc.entity_id,
            nc.name_string_index,
            nc.channel_index,
            nc.physical_interface,
        ]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct EthernetNetworking {
    pub mac_address_index: u8,
    pub mac_address: Option<String>,
    pub ethernet_statistics: u32,
    pub max_segment_size: u16,
    pub num_multicast_filters: u16,
    pub num_power_filters: u8,
}

impl TryFrom<&[u8]> for EthernetNetworking {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new_descriptor_len("Ethernet", 10, value.len()));
        }

        Ok(EthernetNetworking {
            mac_address_index: value[0],
            mac_address: None,
            ethernet_statistics: u32::from_le_bytes([value[1], value[2], value[3], value[4]]),
            max_segment_size: u16::from_le_bytes([value[5], value[6]]),
            num_multicast_filters: u16::from_le_bytes([value[7], value[8]]),
            num_power_filters: value[9],
        })
    }
}

impl From<EthernetNetworking> for Vec<u8> {
    fn from(en: EthernetNetworking) -> Self {
        let mut ret = vec![en.mac_address_index];
        ret.extend(en.ethernet_statistics.to_le_bytes().iter());
        ret.extend(en.max_segment_size.to_le_bytes().iter());
        ret.extend(en.num_multicast_filters.to_le_bytes().iter());
        ret.push(en.num_power_filters);

        ret
    }
}

// just BCD version
#[allow(missing_docs)]
pub type WirelessHandsetControlModel = Header;
#[allow(missing_docs)]
pub type Obex = Header;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MobileDirectLineModelFunctional {
    pub version: Version,
    pub guid: Uuid,
}

impl TryFrom<&[u8]> for MobileDirectLineModelFunctional {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 18 {
            return Err(Error::new_descriptor_len(
                "MobileDirectLineModelFunctional",
                18,
                value.len(),
            ));
        }

        let version = Version::from_bcd(u16::from_le_bytes([value[0], value[1]]));
        let guid = Uuid::from_slice_le(&value[2..18]).map_err(|e| {
            Error::new(
                ErrorKind::InvalidDescriptor,
                &format!("Invalid GUID: {e}"),
            )
        })?;

        Ok(MobileDirectLineModelFunctional { version, guid })
    }
}

impl From<MobileDirectLineModelFunctional> for Vec<u8> {
    fn from(md: MobileDirectLineModelFunctional) -> Self {
        let mut ret = u16::from(md.version).to_le_bytes().to_vec();
        ret.extend(md.guid.to_bytes_le());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MobileDirectLineModelDetail {
    pub guid_descriptor_type: u8,
    pub detail_data: Vec<u8>,
}

impl TryFrom<&[u8]> for MobileDirectLineModelDetail {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 2 {
            return Err(Error::new_descriptor_len(
                "MobileDirectLineModelDetail",
                2,
                value.len(),
            ));
        }

        Ok(MobileDirectLineModelDetail {
            guid_descriptor_type: value[0],
            detail_data: value[1..].to_vec(),
        })
    }
}

impl From<MobileDirectLineModelDetail> for Vec<u8> {
    fn from(md: MobileDirectLineModelDetail) -> Self {
        let mut ret = vec![md.guid_descriptor_type];
        ret.extend(md.detail_data);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DeviceManagement {
    pub version: Version,
    pub max_command: u16,
}

impl TryFrom<&[u8]> for DeviceManagement {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len(
                "DeviceManagement",
                4,
                value.len(),
            ));
        }

        let version = Version::from_bcd(u16::from_le_bytes([value[0], value[1]]));
        let max_command = u16::from_le_bytes([value[2], value[3]]);

        Ok(DeviceManagement {
            version,
            max_command,
        })
    }
}

impl From<DeviceManagement> for Vec<u8> {
    fn from(dm: DeviceManagement) -> Self {
        let mut ret = u16::from(dm.version).to_le_bytes().to_vec();
        ret.extend(dm.max_command.to_le_bytes());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CommandSet {
    pub version: Version,
    pub command_set_string_index: u8,
    pub command_set_string: Option<String>,
    pub guid: Uuid,
}

impl TryFrom<&[u8]> for CommandSet {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 19 {
            return Err(Error::new_descriptor_len("CommandSet", 19, value.len()));
        }

        let version = Version::from_bcd(u16::from_le_bytes([value[0], value[1]]));
        let command_set_string_index = value[2];
        let guid = Uuid::from_slice_le(&value[3..19]).map_err(|e| {
            Error::new(
                ErrorKind::InvalidDescriptor,
                &format!("Invalid GUID: {e}"),
            )
        })?;

        Ok(CommandSet {
            version,
            command_set_string_index,
            command_set_string: None,
            guid,
        })
    }
}

impl From<CommandSet> for Vec<u8> {
    fn from(cs: CommandSet) -> Self {
        let mut ret = u16::from(cs.version).to_le_bytes().to_vec();
        ret.push(cs.command_set_string_index);
        ret.extend(cs.guid.to_bytes_le());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Ncm {
    pub version: Version,
    pub network_capabilities: u8,
}

impl TryFrom<&[u8]> for Ncm {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new_descriptor_len("NCM", 3, value.len()));
        }

        let version = Version::from_bcd(u16::from_le_bytes([value[0], value[1]]));
        let network_capabilities = value[2];

        Ok(Ncm {
            version,
            network_capabilities,
        })
    }
}

impl From<Ncm> for Vec<u8> {
    fn from(ncm: Ncm) -> Self {
        let mut ret = u16::from(ncm.version).to_le_bytes().to_vec();
        ret.push(ncm.network_capabilities);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Mbim {
    pub version: Version,
    pub max_control_message: u16,
    pub number_filters: u8,
    pub max_filter_size: u8,
    pub max_segment_size: u16,
    pub network_capabilities: u8,
}

impl TryFrom<&[u8]> for Mbim {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new_descriptor_len("MBIM", 9, value.len()));
        }

        let version = Version::from_bcd(u16::from_le_bytes([value[0], value[1]]));
        let max_control_message = u16::from_le_bytes([value[2], value[3]]);
        let number_filters = value[4];
        let max_filter_size = value[5];
        let max_segment_size = u16::from_le_bytes([value[6], value[7]]);
        let network_capabilities = value[8];

        Ok(Mbim {
            version,
            max_control_message,
            number_filters,
            max_filter_size,
            max_segment_size,
            network_capabilities,
        })
    }
}

impl From<Mbim> for Vec<u8> {
    fn from(mbim: Mbim) -> Self {
        let mut ret = u16::from(mbim.version).to_le_bytes().to_vec();
        ret.extend(mbim.max_control_message.to_le_bytes().iter());
        ret.push(mbim.number_filters);
        ret.push(mbim.max_filter_size);
        ret.extend(mbim.max_segment_size.to_le_bytes().iter());
        ret.push(mbim.network_capabilities);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MbimExtended {
    pub version: Version,
    pub max_outstanding_command_messages: u8,
    pub mtu: u16,
}

impl TryFrom<&[u8]> for MbimExtended {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len("MBIM Extended", 6, value.len()));
        }

        let version = Version::from_bcd(u16::from_le_bytes([value[0], value[1]]));
        let mtu = u16::from_le_bytes([value[3], value[4]]);

        Ok(MbimExtended {
            version,
            max_outstanding_command_messages: value[2],
            mtu,
        })
    }
}

impl From<MbimExtended> for Vec<u8> {
    fn from(mbim_ext: MbimExtended) -> Self {
        let mut ret = u16::from(mbim_ext.version).to_le_bytes().to_vec();
        ret.push(mbim_ext.max_outstanding_command_messages);
        ret.extend(mbim_ext.mtu.to_le_bytes().iter());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum CdcInterfaceDescriptor {
    Header(Header),
    CallManagement(CallManagement),
    AbstractControlManagement(AbstractControlManagement),
    Union(Union),
    CountrySelection(CountrySelection),
    TelephoneOperations(TelephoneOperations),
    NetworkChannel(NetworkChannel),
    EthernetNetworking(EthernetNetworking),
    WirelessHandsetControlModel(WirelessHandsetControlModel),
    Obex(Obex),
    MobileDirectLineModelFunctional(MobileDirectLineModelFunctional),
    MobileDirectLineModelDetail(MobileDirectLineModelDetail),
    DeviceManagement(DeviceManagement),
    CommandSet(CommandSet),
    Ncm(Ncm),
    Mbim(Mbim),
    MbimExtended(MbimExtended),
    Invalid(Vec<u8>),
    Undefined(Vec<u8>),
}

impl CdcInterfaceDescriptor {
    /// Create a [`CdcInterfaceDescriptor`] from CDC descriptor data
    pub fn from_cdc_descriptor(
        _descriptor_type: &DescriptorType,
        subtype: &CdcType,
        data: &[u8],
    ) -> error::Result<CdcInterfaceDescriptor> {
        match subtype {
            CdcType::Header => Header::try_from(data).map(CdcInterfaceDescriptor::Header),
            CdcType::CallManagement => {
                CallManagement::try_from(data).map(CdcInterfaceDescriptor::CallManagement)
            }
            CdcType::AbstractControlManagement => AbstractControlManagement::try_from(data)
                .map(CdcInterfaceDescriptor::AbstractControlManagement),
            CdcType::Union => Union::try_from(data).map(CdcInterfaceDescriptor::Union),
            CdcType::CountrySelection => {
                CountrySelection::try_from(data).map(CdcInterfaceDescriptor::CountrySelection)
            }
            CdcType::TelephoneOperationalModes => {
                TelephoneOperations::try_from(data).map(CdcInterfaceDescriptor::TelephoneOperations)
            }
            CdcType::NetworkChannel => {
                NetworkChannel::try_from(data).map(CdcInterfaceDescriptor::NetworkChannel)
            }
            CdcType::EthernetNetworking => {
                EthernetNetworking::try_from(data).map(CdcInterfaceDescriptor::EthernetNetworking)
            }
            CdcType::WirelessHandsetControlModel => {
                Header::try_from(data).map(CdcInterfaceDescriptor::WirelessHandsetControlModel)
            }
            CdcType::Obex => Header::try_from(data).map(CdcInterfaceDescriptor::Obex),
            CdcType::MobileDirectLineModelFunctional => {
                MobileDirectLineModelFunctional::try_from(data)
                    .map(CdcInterfaceDescriptor::MobileDirectLineModelFunctional)
            }
            CdcType::MobileDirectLineModelDetail => MobileDirectLineModelDetail::try_from(data)
                .map(CdcInterfaceDescriptor::MobileDirectLineModelDetail),
            CdcType::DeviceManagement => {
                DeviceManagement::try_from(data).map(CdcInterfaceDescriptor::DeviceManagement)
            }
            CdcType::CommandSet => {
                CommandSet::try_from(data).map(CdcInterfaceDescriptor::CommandSet)
            }
            CdcType::Ncm => Ncm::try_from(data).map(CdcInterfaceDescriptor::Ncm),
            CdcType::Mbim => Mbim::try_from(data).map(CdcInterfaceDescriptor::Mbim),
            CdcType::MbimExtended => {
                MbimExtended::try_from(data).map(CdcInterfaceDescriptor::MbimExtended)
            }
            _ => Ok(CdcInterfaceDescriptor::Undefined(data.to_vec())),
        }
    }
}

impl From<CdcInterfaceDescriptor> for Vec<u8> {
    fn from(cd: CdcInterfaceDescriptor) -> Self {
        match cd {
            CdcInterfaceDescriptor::Header(h) => h.into(),
            CdcInterfaceDescriptor::CallManagement(cm) => cm.into(),
            CdcInterfaceDescriptor::AbstractControlManagement(acm) => acm.into(),
            CdcInterfaceDescriptor::Union(u) => u.into(),
            CdcInterfaceDescriptor::CountrySelection(cs) => cs.into(),
            CdcInterfaceDescriptor::TelephoneOperations(to) => to.into(),
            CdcInterfaceDescriptor::NetworkChannel(nc) => nc.into(),
            CdcInterfaceDescriptor::EthernetNetworking(en) => en.into(),
            CdcInterfaceDescriptor::WirelessHandsetControlModel(whcm) => whcm.into(),
            CdcInterfaceDescriptor::Obex(obex) => obex.into(),
            CdcInterfaceDescriptor::MobileDirectLineModelFunctional(md) => md.into(),
            CdcInterfaceDescriptor::MobileDirectLineModelDetail(md) => md.into(),
            CdcInterfaceDescriptor::DeviceManagement(dm) => dm.into(),
            CdcInterfaceDescriptor::CommandSet(cs) => cs.into(),
            CdcInterfaceDescriptor::Ncm(ncm) => ncm.into(),
            CdcInterfaceDescriptor::Mbim(mbim) => mbim.into(),
            CdcInterfaceDescriptor::MbimExtended(mbim_ext) => mbim_ext.into(),
            CdcInterfaceDescriptor::Invalid(data) => data,
            CdcInterfaceDescriptor::Undefined(data) => data,
        }
    }
}

/// USB Communication Device Class (CDC) descriptor
///
/// Can be used by CDCData and CDCCommunications
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CommunicationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_subtype: CdcType,
    pub interface: CdcInterfaceDescriptor,
}

impl TryFrom<&[u8]> for CommunicationDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len(
                "CommunicationDescriptor",
                4,
                value.len(),
            ));
        }

        let communication_type = CdcType::from(value[2]);
        let interface = CdcInterfaceDescriptor::from_cdc_descriptor(
            &DescriptorType::Interface,
            &communication_type,
            &value[3..],
        )
        .unwrap_or_else(|e| {
            log::warn!(
                "Failed to parse CDC interface descriptor for {communication_type:?}: {e:?}"
            );
            CdcInterfaceDescriptor::Invalid(value[3..].to_vec())
        });

        Ok(CommunicationDescriptor {
            length: value[0],
            descriptor_type: value[1],
            descriptor_subtype: communication_type,
            interface,
        })
    }
}

impl From<CommunicationDescriptor> for Vec<u8> {
    fn from(cd: CommunicationDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(cd.length);
        ret.push(cd.descriptor_type);
        ret.push(cd.descriptor_subtype as u8);
        let data = Vec::<u8>::from(cd.interface);
        ret.extend(data);

        ret
    }
}

impl TryFrom<GenericDescriptor> for CommunicationDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        CommunicationDescriptor::try_from(&gd_vec[..])
    }
}
