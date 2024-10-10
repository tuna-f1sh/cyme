//! Defines for the USB Audio Class (UAC) interface descriptors and MIDI
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
use strum::VariantArray;
use strum_macros::VariantArray;

use super::*;
use crate::error::{self, Error, ErrorKind};

/// bSubtype for MIDI interface descriptors
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum MidiSubtype {
    Undefined = 0x00,
    Header = 0x01,
    InputJack = 0x02,
    OutputJack = 0x03,
    Element = 0x04,
}

impl fmt::Display for MidiSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // lsusb style
        if f.alternate() {
            match self {
                MidiSubtype::Undefined => write!(f, "Invalid"),
                MidiSubtype::InputJack => write!(f, "MIDI_IN_JACK"),
                MidiSubtype::OutputJack => write!(f, "MIDI_OUT_JACK"),
                _ => write!(f, "{}", heck::AsShoutySnakeCase(format!("{:?}", self))),
            }
        } else {
            write!(f, "{:?}", self)
        }
    }
}

impl From<u8> for MidiSubtype {
    fn from(b: u8) -> Self {
        match b {
            0x00 => MidiSubtype::Undefined,
            0x01 => MidiSubtype::Header,
            0x02 => MidiSubtype::InputJack,
            0x03 => MidiSubtype::OutputJack,
            0x04 => MidiSubtype::Element,
            _ => MidiSubtype::Undefined,
        }
    }
}

impl From<MidiSubtype> for u8 {
    fn from(b: MidiSubtype) -> Self {
        b as u8
    }
}

impl MidiSubtype {
    /// Get the [`MidiInterfaceDescriptor`] from the descriptor data for this subtype
    pub fn get_interface_descriptor(&self, data: &[u8]) -> Result<MidiInterfaceDescriptor, Error> {
        MidiInterfaceDescriptor::from_midi_interface(self, data)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MidiDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_subtype: MidiSubtype,
    pub interface: MidiInterfaceDescriptor,
}

impl TryFrom<&[u8]> for MidiDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("MidiDescriptor", 4, value.len()));
        }

        let length = value[0];
        if length as usize > value.len() {
            return Err(Error::new_descriptor_len(
                "MidiDescriptor reported",
                length as usize,
                value.len(),
            ));
        }

        let descriptor_type = DescriptorType::from(value[1]);
        let descriptor_subtype = MidiSubtype::from(value[2]);
        let interface = MidiInterfaceDescriptor::from_midi_descriptor(
            &descriptor_type,
            &descriptor_subtype,
            &value[3..],
        )
        .unwrap_or_else(|_| {
            log::warn!(
                "Failed to parse MIDI interface descriptor for {:?} {:?}: {:?}",
                descriptor_type,
                descriptor_subtype,
                &value[3..]
            );
            MidiInterfaceDescriptor::Invalid(value[3..].to_vec())
        });

        Ok(MidiDescriptor {
            length,
            descriptor_type: value[1],
            descriptor_subtype,
            interface,
        })
    }
}

impl From<MidiDescriptor> for Vec<u8> {
    fn from(md: MidiDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(md.length);
        ret.push(md.descriptor_type);
        ret.push(md.descriptor_subtype as u8);
        let data: Vec<u8> = md.interface.into();
        ret.extend(&data);

        ret
    }
}

impl TryFrom<GenericDescriptor> for MidiDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        MidiDescriptor::try_from(&gd_vec[..])
    }
}

impl TryFrom<MidiDescriptor> for GenericDescriptor {
    type Error = Error;

    fn try_from(md: MidiDescriptor) -> error::Result<Self> {
        let data: Vec<u8> = md.into();
        GenericDescriptor::try_from(data.as_slice())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Header {
    pub version: Version,
    pub total_length: u16,
}

impl TryFrom<&[u8]> for Header {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("Header", 4, value.len()));
        }

        Ok(Header {
            version: Version::from_bcd(u16::from_le_bytes([value[0], value[1]])),
            total_length: u16::from_le_bytes([value[2], value[3]]),
        })
    }
}

impl From<Header> for Vec<u8> {
    fn from(h: Header) -> Self {
        let mut ret = Vec::new();
        ret.extend_from_slice(&(u16::from(h.version)).to_le_bytes());
        ret.extend_from_slice(&h.total_length.to_le_bytes());

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct InputJack {
    pub jack_type: u8,
    pub jack_id: u8,
    pub jack_string_index: u8,
    pub jack_string: Option<String>,
}

impl TryFrom<&[u8]> for InputJack {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new_descriptor_len("InputJack", 3, value.len()));
        }

        Ok(InputJack {
            jack_type: value[0],
            jack_id: value[1],
            jack_string_index: value[2],
            jack_string: None,
        })
    }
}

impl From<InputJack> for Vec<u8> {
    fn from(ij: InputJack) -> Self {
        vec![ij.jack_type, ij.jack_id, ij.jack_string_index]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct OutputJack {
    pub jack_type: u8,
    pub jack_id: u8,
    pub num_input_pins: u8,
    pub source_ids: Vec<(u8, u8)>,
    pub jack_string_index: u8,
    pub jack_string: Option<String>,
}

impl TryFrom<&[u8]> for OutputJack {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("OutputJack", 4, value.len()));
        }

        let num_input_pins = value[2] as usize;
        if value.len() < 3 + num_input_pins * 2 + 1 {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "OutputJack descriptor reported number of source pins too long for buffer",
            ));
        }

        let mut source_ids = Vec::with_capacity(num_input_pins);
        for chunk in value[3..3 + num_input_pins * 2].chunks_exact(2) {
            source_ids.push((chunk[0], chunk[1]));
        }

        Ok(OutputJack {
            jack_type: value[0],
            jack_id: value[1],
            num_input_pins: value[2],
            source_ids,
            jack_string_index: value[3 + num_input_pins * 2],
            jack_string: None,
        })
    }
}

impl From<OutputJack> for Vec<u8> {
    fn from(oj: OutputJack) -> Self {
        let mut ret = vec![oj.jack_type, oj.jack_id, oj.num_input_pins];
        for (a, b) in oj.source_ids {
            ret.push(a);
            ret.push(b);
        }
        ret.push(oj.jack_string_index);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Element {
    pub element_id: u8,
    pub num_input_pins: u8,
    pub source_ids: Vec<(u8, u8)>,
    pub num_output_pins: u8,
    pub in_terminal_link: u8,
    pub out_terminal_link: u8,
    pub el_caps_size: u8,
    pub element_caps: u16,
    pub element_string_index: u8,
    pub element_string: Option<String>,
}

impl TryFrom<&[u8]> for Element {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 8 {
            return Err(Error::new_descriptor_len("Element", 8, value.len()));
        }

        let element_id = value[0];
        let num_input_pins = value[1] as usize;

        let expected_len = 6 + 2 * num_input_pins;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Element descriptor reported number of input pins too long for buffer",
            ));
        }

        let mut source_ids = Vec::with_capacity(num_input_pins);
        for chunk in value[2..2 + num_input_pins * 2].chunks_exact(2) {
            source_ids.push((chunk[0], chunk[1]));
        }

        let j = 2 + num_input_pins * 2;
        let num_output_pins = value[j];
        let in_terminal_link = value[j + 1];
        let out_terminal_link = value[j + 2];
        let el_caps_size = value[j + 3];
        let capsize = el_caps_size as usize;

        if value.len() < j + 4 + capsize + 1 {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Element descriptor reported caps size too long for buffer",
            ));
        }

        let mut element_caps: u16 = 0;
        for i in 0..capsize {
            element_caps |= (value[j + 4 + i] as u16) << (i * 8);
        }

        let element_string_index = value[j + 4 + capsize];
        let element_string = None;

        Ok(Element {
            element_id,
            num_input_pins: value[1],
            source_ids,
            num_output_pins,
            in_terminal_link,
            out_terminal_link,
            el_caps_size,
            element_caps,
            element_string_index,
            element_string,
        })
    }
}

impl From<Element> for Vec<u8> {
    fn from(el: Element) -> Self {
        let mut ret = Vec::new();
        ret.push(el.element_id);
        ret.push(el.num_input_pins);

        for (source_id, source_pin) in el.source_ids {
            ret.push(source_id);
            ret.push(source_pin);
        }

        ret.push(el.num_output_pins);
        ret.push(el.in_terminal_link);
        ret.push(el.out_terminal_link);
        ret.push(el.el_caps_size);

        for i in 0..el.el_caps_size {
            ret.push((el.element_caps >> (i * 8)) as u8);
        }

        ret.push(el.element_string_index);
        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MidiEndpointDescriptor {
    pub num_jacks: u8,
    pub jacks: Vec<u8>,
}

impl TryFrom<&[u8]> for MidiEndpointDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.is_empty() {
            return Err(Error::new_descriptor_len("MidiEndpointDescriptor", 1, 0));
        }

        let num_jacks = value[0] as usize;
        if value.len() < 1 + num_jacks {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "MidiEndpointDescriptor descriptor reported number of jacks too long for buffer",
            ));
        }
        let jacks = value[1..1 + num_jacks].to_vec();

        Ok(MidiEndpointDescriptor {
            num_jacks: value[0],
            jacks,
        })
    }
}

impl From<MidiEndpointDescriptor> for Vec<u8> {
    fn from(md: MidiEndpointDescriptor) -> Self {
        let mut ret = vec![md.num_jacks];
        ret.extend(md.jacks);

        ret
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum MidiInterfaceDescriptor {
    Header(Header),
    InputJack(InputJack),
    OutputJack(OutputJack),
    Element(Element),
    Endpoint(MidiEndpointDescriptor),
    Invalid(Vec<u8>),
    Undefined(Vec<u8>),
}

impl MidiInterfaceDescriptor {
    /// Try to parse the MIDI interface descriptor from the main descriptor data
    pub fn from_midi_descriptor(
        descriptor_type: &DescriptorType,
        subtype: &MidiSubtype,
        data: &[u8],
    ) -> Result<Self, Error> {
        match descriptor_type {
            DescriptorType::Endpoint => Self::from_midi_endpoint(data),
            _ => subtype.get_interface_descriptor(data),
        }
    }

    /// Try to parse the MIDI interface descriptor for an Endpoint from the provided data
    pub fn from_midi_endpoint(data: &[u8]) -> Result<Self, Error> {
        MidiEndpointDescriptor::try_from(data).map(MidiInterfaceDescriptor::Endpoint)
    }

    /// Try to parse the MIDI interface descriptor for an Interface from the provided data
    pub fn from_midi_interface(subtype: &MidiSubtype, data: &[u8]) -> Result<Self, Error> {
        match subtype {
            MidiSubtype::Header => Header::try_from(data).map(MidiInterfaceDescriptor::Header),
            MidiSubtype::InputJack => {
                InputJack::try_from(data).map(MidiInterfaceDescriptor::InputJack)
            }
            MidiSubtype::OutputJack => {
                OutputJack::try_from(data).map(MidiInterfaceDescriptor::OutputJack)
            }
            MidiSubtype::Element => Element::try_from(data).map(MidiInterfaceDescriptor::Element),
            MidiSubtype::Undefined => Ok(MidiInterfaceDescriptor::Undefined(data.to_vec())),
        }
    }
}

impl From<MidiInterfaceDescriptor> for Vec<u8> {
    fn from(mid: MidiInterfaceDescriptor) -> Self {
        match mid {
            MidiInterfaceDescriptor::Header(a) => a.into(),
            MidiInterfaceDescriptor::InputJack(a) => a.into(),
            MidiInterfaceDescriptor::OutputJack(a) => a.into(),
            MidiInterfaceDescriptor::Element(a) => a.into(),
            MidiInterfaceDescriptor::Endpoint(a) => a.into(),
            MidiInterfaceDescriptor::Invalid(a) => a,
            MidiInterfaceDescriptor::Undefined(a) => a,
        }
    }
}

impl TryFrom<GenericDescriptor> for MidiEndpointDescriptor {
    type Error = Error;

    fn try_from(gd: GenericDescriptor) -> error::Result<Self> {
        let gd_vec: Vec<u8> = gd.into();
        MidiEndpointDescriptor::try_from(gd_vec.as_slice())
    }
}

/// Base USB Audio Class (UAC) interface descriptor that contains [`UacType`] and [`UacInterfaceDescriptor`]
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UacDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_subtype: UacType,
    pub interface: UacInterfaceDescriptor,
}

/// Try from ([`GenericDescriptor`], SubClass, Protocol)
impl TryFrom<(GenericDescriptor, u8, u8)> for UacDescriptor {
    type Error = Error;

    fn try_from((gd, subc, p): (GenericDescriptor, u8, u8)) -> error::Result<Self> {
        let length = gd.length;
        let descriptor_type = gd.descriptor_type;
        let descriptor_subtype: UacType = (subc, gd.descriptor_subtype, p).try_into()?;
        let interface = descriptor_subtype.uac_descriptor_from_generic(gd, p)?;

        Ok(UacDescriptor {
            length,
            descriptor_type,
            descriptor_subtype,
            interface,
        })
    }
}

impl From<UacDescriptor> for Vec<u8> {
    fn from(acd: UacDescriptor) -> Self {
        let mut ret: Vec<u8> = Vec::new();
        ret.push(acd.length);
        ret.push(acd.descriptor_type);
        ret.push(u8::from(acd.descriptor_subtype));
        let data: Vec<u8> = acd.interface.into();
        ret.extend(&data);

        ret
    }
}

impl TryFrom<UacDescriptor> for GenericDescriptor {
    type Error = Error;

    fn try_from(ud: UacDescriptor) -> error::Result<Self> {
        let data: Vec<u8> = ud.into();
        GenericDescriptor::try_from(data.as_slice())
    }
}

impl UacDescriptor {
    /// Get the [`UacProtocol`] for the attached UAC interface
    pub fn get_protocol(&self) -> UacProtocol {
        self.interface.get_protocol()
    }
}

/// USB Audio Class (UAC) interface descriptors
///
/// Ported from <https://github.com/gregkh/usbutils/blob/master/desc-defs.c>
///
/// Possibly much nicer way to define all these for more generic printing; enum types like desc-def.c wrapping the int values so they can be acted on in a more generic way
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum UacInterfaceDescriptor {
    // Audio Controls bSubClass
    Header1(Header1),
    Header2(Header2),
    Header3(Header3),
    InputTerminal1(InputTerminal1),
    InputTerminal2(InputTerminal2),
    InputTerminal3(InputTerminal3),
    OutputTerminal1(OutputTerminal1),
    OutputTerminal2(OutputTerminal2),
    OutputTerminal3(OutputTerminal3),
    ExtendedTerminalHeader(ExtendedTerminalHeader),
    PowerDomain(PowerDomain),
    MixerUnit1(MixerUnit1),
    MixerUnit2(MixerUnit2),
    MixerUnit3(MixerUnit3),
    SelectorUnit1(SelectorUnit1),
    SelectorUnit2(SelectorUnit2),
    SelectorUnit3(SelectorUnit3),
    ProcessingUnit1(ProcessingUnit1),
    ProcessingUnit2(ProcessingUnit2),
    ProcessingUnit3(ProcessingUnit3),
    EffectUnit2(EffectUnit2),
    EffectUnit3(EffectUnit3),
    FeatureUnit1(FeatureUnit1),
    FeatureUnit2(FeatureUnit2),
    FeatureUnit3(FeatureUnit3),
    ExtensionUnit1(ExtensionUnit1),
    ExtensionUnit2(ExtensionUnit2),
    ExtensionUnit3(ExtensionUnit3),
    ClockSource2(ClockSource2),
    ClockSource3(ClockSource3),
    ClockSelector2(ClockSelector2),
    ClockSelector3(ClockSelector3),
    ClockMultiplier2(ClockMultiplier2),
    ClockMultiplier3(ClockMultiplier3),
    SampleRateConverter2(SampleRateConverter2),
    SampleRateConverter3(SampleRateConverter3),
    // Audio Streaming bSubClass
    StreamingInterface1(StreamingInterface1),
    StreamingInterface2(StreamingInterface2),
    StreamingInterface3(StreamingInterface3),
    StreamingFormat(StreamingFormat),
    StreamingFormatSpecific(StreamingFormatSpecific),
    // Isochronous Audio Data Stream Endpoint
    DataStreamingEndpoint1(DataStreamingEndpoint1),
    DatastreamingEndpoint2(DataStreamingEndpoint2),
    DataStreamingEndpoint3(DataStreamingEndpoint3),
    /// Invalid descriptor for failing to parse matched
    Invalid(Vec<u8>),
    /// Generic descriptor for known but unsupported descriptors
    Generic(Vec<u8>),
    /// Undefined descriptor
    Undefined(Vec<u8>),
}

impl From<UacInterfaceDescriptor> for Vec<u8> {
    fn from(val: UacInterfaceDescriptor) -> Self {
        match val {
            UacInterfaceDescriptor::Header1(a) => a.into(),
            UacInterfaceDescriptor::Header2(a) => a.into(),
            UacInterfaceDescriptor::Header3(a) => a.into(),
            UacInterfaceDescriptor::InputTerminal1(a) => a.into(),
            UacInterfaceDescriptor::InputTerminal2(a) => a.into(),
            UacInterfaceDescriptor::InputTerminal3(a) => a.into(),
            UacInterfaceDescriptor::OutputTerminal1(a) => a.into(),
            UacInterfaceDescriptor::OutputTerminal2(a) => a.into(),
            UacInterfaceDescriptor::OutputTerminal3(a) => a.into(),
            UacInterfaceDescriptor::ExtendedTerminalHeader(a) => a.into(),
            UacInterfaceDescriptor::PowerDomain(a) => a.into(),
            UacInterfaceDescriptor::MixerUnit1(a) => a.into(),
            UacInterfaceDescriptor::MixerUnit2(a) => a.into(),
            UacInterfaceDescriptor::MixerUnit3(a) => a.into(),
            UacInterfaceDescriptor::SelectorUnit1(a) => a.into(),
            UacInterfaceDescriptor::SelectorUnit2(a) => a.into(),
            UacInterfaceDescriptor::SelectorUnit3(a) => a.into(),
            UacInterfaceDescriptor::ProcessingUnit1(a) => a.into(),
            UacInterfaceDescriptor::ProcessingUnit2(a) => a.into(),
            UacInterfaceDescriptor::ProcessingUnit3(a) => a.into(),
            UacInterfaceDescriptor::EffectUnit2(a) => a.into(),
            UacInterfaceDescriptor::EffectUnit3(a) => a.into(),
            UacInterfaceDescriptor::FeatureUnit1(a) => a.into(),
            UacInterfaceDescriptor::FeatureUnit2(a) => a.into(),
            UacInterfaceDescriptor::FeatureUnit3(a) => a.into(),
            UacInterfaceDescriptor::ExtensionUnit1(a) => a.into(),
            UacInterfaceDescriptor::ExtensionUnit2(a) => a.into(),
            UacInterfaceDescriptor::ExtensionUnit3(a) => a.into(),
            UacInterfaceDescriptor::ClockSource2(a) => a.into(),
            UacInterfaceDescriptor::ClockSource3(a) => a.into(),
            UacInterfaceDescriptor::ClockSelector2(a) => a.into(),
            UacInterfaceDescriptor::ClockSelector3(a) => a.into(),
            UacInterfaceDescriptor::ClockMultiplier2(a) => a.into(),
            UacInterfaceDescriptor::ClockMultiplier3(a) => a.into(),
            UacInterfaceDescriptor::SampleRateConverter2(a) => a.into(),
            UacInterfaceDescriptor::SampleRateConverter3(a) => a.into(),
            UacInterfaceDescriptor::StreamingInterface1(a) => a.into(),
            UacInterfaceDescriptor::StreamingInterface2(a) => a.into(),
            UacInterfaceDescriptor::StreamingInterface3(a) => a.into(),
            UacInterfaceDescriptor::StreamingFormat(a) => a.into(),
            UacInterfaceDescriptor::StreamingFormatSpecific(a) => a.into(),
            UacInterfaceDescriptor::DataStreamingEndpoint1(a) => a.into(),
            UacInterfaceDescriptor::DatastreamingEndpoint2(a) => a.into(),
            UacInterfaceDescriptor::DataStreamingEndpoint3(a) => a.into(),
            UacInterfaceDescriptor::Invalid(a) => a,
            UacInterfaceDescriptor::Generic(a) => a,
            UacInterfaceDescriptor::Undefined(a) => a,
        }
    }
}

/// USB Audio Class (UAC) protocol 1 channel names based on the "wChannelConfig" field
///
/// Decoded as bitstring; each bit corresponds to a channel name
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, VariantArray)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum Uac1ChannelNames {
    LeftFront,
    RightFront,
    CenterFront,
    LowFrequencyEnhancement,
    LeftSurround,
    RightSurround,
    LeftOfCenter,
    RightOfCenter,
    Surround,
    SideLeft,
    SideRight,
    Top,
}

impl fmt::Display for Uac1ChannelNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uac1ChannelNames::LeftFront => write!(f, "Left Front (L)"),
            Uac1ChannelNames::RightFront => write!(f, "Right Front (R)"),
            Uac1ChannelNames::CenterFront => write!(f, "Center Front (C)"),
            Uac1ChannelNames::LowFrequencyEnhancement => {
                write!(f, "Low Frequency Enhancement (LFE)")
            }
            Uac1ChannelNames::LeftSurround => write!(f, "Left Surround (LS)"),
            Uac1ChannelNames::RightSurround => write!(f, "Right Surround (RS)"),
            Uac1ChannelNames::LeftOfCenter => write!(f, "Left of Center (LC)"),
            Uac1ChannelNames::RightOfCenter => write!(f, "Right of Center (RC)"),
            Uac1ChannelNames::Surround => write!(f, "Surround (S)"),
            Uac1ChannelNames::SideLeft => write!(f, "Side Left (SL)"),
            Uac1ChannelNames::SideRight => write!(f, "Side Right (SR)"),
            Uac1ChannelNames::Top => write!(f, "Top (T)"),
        }
    }
}

impl Uac1ChannelNames {
    /// Get the supported [`Uac1ChannelNames`] from the bitmap value
    pub fn from_bitmap<T: Into<u32>>(bitmap: T) -> Vec<Uac1ChannelNames> {
        let mut ret = Vec::new();
        let bitmap = bitmap.into();
        for (i, s) in Uac1ChannelNames::VARIANTS.iter().enumerate() {
            if bitmap & (1 << i) != 0 {
                ret.push(*s);
            }
        }
        ret
    }
}

/// USB Audio Class (UAC) protocol 2 supported channel names based on the "wChannelConfig" bitmap
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, VariantArray)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum Uac2ChannelNames {
    FrontLeft,
    FrontRight,
    FrontCenter,
    LowFrequencyEffects,
    BackLeft,
    BackRight,
    FrontLeftOfCenter,
    FrontRightOfCenter,
    BackCenter,
    SideLeft,
    SideRight,
    TopCenter,
    TopFrontLeft,
    TopFrontCenter,
    TopFrontRight,
    TopBackLeft,
    TopBackCenter,
    TopBackRight,
    TopFrontLeftOfCenter,
    TopFrontRightOfCenter,
    LeftLowFrequencyEffects,
    RightLowFrequencyEffects,
    TopSideLeft,
    TopSideRight,
    BottomCenter,
    BackLeftOfCenter,
    BackRightOfCenter,
}

impl fmt::Display for Uac2ChannelNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uac2ChannelNames::FrontLeft => write!(f, "Front Left (FL)"),
            Uac2ChannelNames::FrontRight => write!(f, "Front Right (FR)"),
            Uac2ChannelNames::FrontCenter => write!(f, "Front Center (FC)"),
            Uac2ChannelNames::LowFrequencyEffects => write!(f, "Low Frequency Effects (LFE)"),
            Uac2ChannelNames::BackLeft => write!(f, "Back Left (BL)"),
            Uac2ChannelNames::BackRight => write!(f, "Back Right (BR)"),
            Uac2ChannelNames::FrontLeftOfCenter => write!(f, "Front Left of Center (FLC)"),
            Uac2ChannelNames::FrontRightOfCenter => write!(f, "Front Right of Center (FRC)"),
            Uac2ChannelNames::BackCenter => write!(f, "Back Center (BC)"),
            Uac2ChannelNames::SideLeft => write!(f, "Side Left (SL)"),
            Uac2ChannelNames::SideRight => write!(f, "Side Right (SR)"),
            Uac2ChannelNames::TopCenter => write!(f, "Top Center (TC)"),
            Uac2ChannelNames::TopFrontLeft => write!(f, "Top Front Left (TFL)"),
            Uac2ChannelNames::TopFrontCenter => write!(f, "Top Front Center (TFC)"),
            Uac2ChannelNames::TopFrontRight => write!(f, "Top Front Right (TFR)"),
            Uac2ChannelNames::TopBackLeft => write!(f, "Top Back Left (TBL)"),
            Uac2ChannelNames::TopBackCenter => write!(f, "Top Back Center (TBC)"),
            Uac2ChannelNames::TopBackRight => write!(f, "Top Back Right (TBR)"),
            Uac2ChannelNames::TopFrontLeftOfCenter => write!(f, "Top Front Left of Center (TFLC)"),
            Uac2ChannelNames::TopFrontRightOfCenter => {
                write!(f, "Top Front Right of Center (TFRC)")
            }
            Uac2ChannelNames::LeftLowFrequencyEffects => {
                write!(f, "Left Low Frequency Effects (LLFE)")
            }
            Uac2ChannelNames::RightLowFrequencyEffects => {
                write!(f, "Right Low Frequency Effects (RLFE)")
            }
            Uac2ChannelNames::TopSideLeft => write!(f, "Top Side Left (TSL)"),
            Uac2ChannelNames::TopSideRight => write!(f, "Top Side Right (TSR)"),
            Uac2ChannelNames::BottomCenter => write!(f, "Bottom Center (BC)"),
            Uac2ChannelNames::BackLeftOfCenter => write!(f, "Back Left of Center (BLC)"),
            Uac2ChannelNames::BackRightOfCenter => write!(f, "Back Right of Center (BRC)"),
        }
    }
}

impl Uac2ChannelNames {
    /// Get the supported [`Uac2ChannelNames`] from the bitmap value
    pub fn from_bitmap<T: Into<u32>>(bitmap: T) -> Vec<Uac2ChannelNames> {
        let mut ret = Vec::new();
        let bitmap = bitmap.into();
        for (i, s) in Uac2ChannelNames::VARIANTS.iter().enumerate() {
            if bitmap & (1 << i) != 0 {
                ret.push(*s);
            }
        }
        ret
    }
}

/// USB Audio Class (UAC) channel names based on the "wChannelConfig" field
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum ChannelNames {
    /// UAC1 channel names
    Uac1(Uac1ChannelNames),
    /// UAC2 channel names
    Uac2(Uac2ChannelNames),
}

impl fmt::Display for ChannelNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChannelNames::Uac1(c) => write!(f, "{}", c),
            ChannelNames::Uac2(c) => write!(f, "{}", c),
        }
    }
}

impl ChannelNames {
    /// Get the supported [`ChannelNames`] from the bitmap value
    pub fn from_bitmap<T: Into<u32>>(protocol: &UacProtocol, bitmap: T) -> Vec<ChannelNames> {
        match protocol {
            UacProtocol::Uac1 => Uac1ChannelNames::from_bitmap(bitmap)
                .iter()
                .map(|c| ChannelNames::Uac1(*c))
                .collect(),
            UacProtocol::Uac2 => Uac2ChannelNames::from_bitmap(bitmap)
                .iter()
                .map(|c| ChannelNames::Uac2(*c))
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl UacInterfaceDescriptor {
    /// Get the UAC AC interface descriptor from the UAC AC interface
    pub fn from_uac_ac_interface(
        uac_interface: &ControlSubtype,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match uac_interface {
            ControlSubtype::Header => match protocol {
                UacProtocol::Uac1 => Header1::try_from(data).map(UacInterfaceDescriptor::Header1),
                UacProtocol::Uac2 => Header2::try_from(data).map(UacInterfaceDescriptor::Header2),
                UacProtocol::Uac3 => Header3::try_from(data).map(UacInterfaceDescriptor::Header3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::InputTerminal => match protocol {
                UacProtocol::Uac1 => {
                    InputTerminal1::try_from(data).map(UacInterfaceDescriptor::InputTerminal1)
                }
                UacProtocol::Uac2 => {
                    InputTerminal2::try_from(data).map(UacInterfaceDescriptor::InputTerminal2)
                }
                UacProtocol::Uac3 => {
                    InputTerminal3::try_from(data).map(UacInterfaceDescriptor::InputTerminal3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::OutputTerminal => match protocol {
                UacProtocol::Uac1 => {
                    OutputTerminal1::try_from(data).map(UacInterfaceDescriptor::OutputTerminal1)
                }
                UacProtocol::Uac2 => {
                    OutputTerminal2::try_from(data).map(UacInterfaceDescriptor::OutputTerminal2)
                }
                UacProtocol::Uac3 => {
                    OutputTerminal3::try_from(data).map(UacInterfaceDescriptor::OutputTerminal3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::ExtendedTerminal => match protocol {
                UacProtocol::Uac3 => ExtendedTerminalHeader::try_from(data)
                    .map(UacInterfaceDescriptor::ExtendedTerminalHeader),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::PowerDomain => match protocol {
                UacProtocol::Uac3 => {
                    PowerDomain::try_from(data).map(UacInterfaceDescriptor::PowerDomain)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::MixerUnit => match protocol {
                UacProtocol::Uac1 => {
                    MixerUnit1::try_from(data).map(UacInterfaceDescriptor::MixerUnit1)
                }
                UacProtocol::Uac2 => {
                    MixerUnit2::try_from(data).map(UacInterfaceDescriptor::MixerUnit2)
                }
                UacProtocol::Uac3 => {
                    MixerUnit3::try_from(data).map(UacInterfaceDescriptor::MixerUnit3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::SelectorUnit => match protocol {
                UacProtocol::Uac1 => {
                    SelectorUnit1::try_from(data).map(UacInterfaceDescriptor::SelectorUnit1)
                }
                UacProtocol::Uac2 => {
                    SelectorUnit2::try_from(data).map(UacInterfaceDescriptor::SelectorUnit2)
                }
                UacProtocol::Uac3 => {
                    SelectorUnit3::try_from(data).map(UacInterfaceDescriptor::SelectorUnit3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::ProcessingUnit => match protocol {
                UacProtocol::Uac1 => {
                    ProcessingUnit1::try_from(data).map(UacInterfaceDescriptor::ProcessingUnit1)
                }
                UacProtocol::Uac2 => {
                    ProcessingUnit2::try_from(data).map(UacInterfaceDescriptor::ProcessingUnit2)
                }
                UacProtocol::Uac3 => {
                    ProcessingUnit3::try_from(data).map(UacInterfaceDescriptor::ProcessingUnit3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::EffectUnit => match protocol {
                UacProtocol::Uac2 => {
                    EffectUnit2::try_from(data).map(UacInterfaceDescriptor::EffectUnit2)
                }
                UacProtocol::Uac3 => {
                    EffectUnit3::try_from(data).map(UacInterfaceDescriptor::EffectUnit3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::FeatureUnit => match protocol {
                UacProtocol::Uac1 => {
                    FeatureUnit1::try_from(data).map(UacInterfaceDescriptor::FeatureUnit1)
                }
                UacProtocol::Uac2 => {
                    FeatureUnit2::try_from(data).map(UacInterfaceDescriptor::FeatureUnit2)
                }
                UacProtocol::Uac3 => {
                    FeatureUnit3::try_from(data).map(UacInterfaceDescriptor::FeatureUnit3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::ExtensionUnit => match protocol {
                UacProtocol::Uac1 => {
                    ExtensionUnit1::try_from(data).map(UacInterfaceDescriptor::ExtensionUnit1)
                }
                UacProtocol::Uac2 => {
                    ExtensionUnit2::try_from(data).map(UacInterfaceDescriptor::ExtensionUnit2)
                }
                UacProtocol::Uac3 => {
                    ExtensionUnit3::try_from(data).map(UacInterfaceDescriptor::ExtensionUnit3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::ClockSource => match protocol {
                UacProtocol::Uac2 => {
                    ClockSource2::try_from(data).map(UacInterfaceDescriptor::ClockSource2)
                }
                UacProtocol::Uac3 => {
                    ClockSource3::try_from(data).map(UacInterfaceDescriptor::ClockSource3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::ClockSelector => match protocol {
                UacProtocol::Uac2 => {
                    ClockSelector2::try_from(data).map(UacInterfaceDescriptor::ClockSelector2)
                }
                UacProtocol::Uac3 => {
                    ClockSelector3::try_from(data).map(UacInterfaceDescriptor::ClockSelector3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::ClockMultiplier => {
                match protocol {
                    UacProtocol::Uac2 => ClockMultiplier2::try_from(data)
                        .map(UacInterfaceDescriptor::ClockMultiplier2),
                    UacProtocol::Uac3 => ClockMultiplier3::try_from(data)
                        .map(UacInterfaceDescriptor::ClockMultiplier3),
                    _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
                }
            }
            ControlSubtype::SampleRateConverter => match protocol {
                UacProtocol::Uac2 => SampleRateConverter2::try_from(data)
                    .map(UacInterfaceDescriptor::SampleRateConverter2),
                UacProtocol::Uac3 => SampleRateConverter3::try_from(data)
                    .map(UacInterfaceDescriptor::SampleRateConverter3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            ControlSubtype::Undefined => Ok(UacInterfaceDescriptor::Undefined(data.to_vec())),
            _ => Ok(UacInterfaceDescriptor::Generic(data.to_vec())),
        }
    }

    /// Get the UAC AS interface descriptor from the UAC AS interface
    pub fn from_uac_as_interface(
        uac_interface: &StreamingSubtype,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match uac_interface {
            // Can be DataStreamingEndpoint or StreamingInterface so check the length
            // since we have no Class context here
            StreamingSubtype::General => match protocol {
                UacProtocol::Uac1 => {
                    if data.len() == DataStreamingEndpoint1::size() {
                        DataStreamingEndpoint1::try_from(data)
                            .map(UacInterfaceDescriptor::DataStreamingEndpoint1)
                    } else {
                        StreamingInterface1::try_from(data)
                            .map(UacInterfaceDescriptor::StreamingInterface1)
                    }
                }
                UacProtocol::Uac2 => {
                    if data.len() == DataStreamingEndpoint2::size() {
                        DataStreamingEndpoint2::try_from(data)
                            .map(UacInterfaceDescriptor::DatastreamingEndpoint2)
                    } else {
                        StreamingInterface2::try_from(data)
                            .map(UacInterfaceDescriptor::StreamingInterface2)
                    }
                }
                UacProtocol::Uac3 => {
                    if data.len() == DataStreamingEndpoint3::size() {
                        DataStreamingEndpoint3::try_from(data)
                            .map(UacInterfaceDescriptor::DataStreamingEndpoint3)
                    } else {
                        StreamingInterface3::try_from(data)
                            .map(UacInterfaceDescriptor::StreamingInterface3)
                    }
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            StreamingSubtype::FormatType => StreamingFormat::from_uac_as_interface(protocol, data)
                .map(UacInterfaceDescriptor::StreamingFormat),
            StreamingSubtype::FormatSpecific => {
                StreamingFormatSpecific::from_uac_as_interface(protocol, data)
                    .map(UacInterfaceDescriptor::StreamingFormatSpecific)
            }
            StreamingSubtype::Undefined => Ok(UacInterfaceDescriptor::Undefined(data.to_vec())),
            //_ => Ok(UacInterfaceDescriptor::Generic(data.to_vec())),
        }
    }

    /// Get the UAC Audio Data Endpoint descriptor from the UAC AS interface
    pub fn from_uac_as_iso_data_endpoint(
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match protocol {
            UacProtocol::Uac1 => DataStreamingEndpoint1::try_from(data)
                .map(UacInterfaceDescriptor::DataStreamingEndpoint1),
            UacProtocol::Uac2 => DataStreamingEndpoint2::try_from(data)
                .map(UacInterfaceDescriptor::DatastreamingEndpoint2),
            UacProtocol::Uac3 => DataStreamingEndpoint3::try_from(data)
                .map(UacInterfaceDescriptor::DataStreamingEndpoint3),
            // Only endpoint
            _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
        }
    }

    /// Get bitmap string from the descriptor bit field; each bit corresponds to a string in the array
    pub fn get_bitmap_string<T: Into<u32> + Copy>(bitmap: T, strings: &[&str]) -> Vec<String> {
        let mut ret = Vec::new();
        for (i, s) in strings.iter().enumerate() {
            if bitmap.into() & (1 << i) != 0 {
                ret.push(s.to_string());
            }
        }
        ret
    }

    /// Get the [`ChannelNames`] from the descriptor "wChannelConfig" field bitmap
    pub fn get_channel_names<T: Into<u32> + Copy>(&self, channel_config: T) -> Vec<ChannelNames> {
        match self.get_protocol() {
            UacProtocol::Uac1 => Uac1ChannelNames::from_bitmap(channel_config)
                .iter()
                .map(|c| ChannelNames::Uac1(*c))
                .collect(),
            UacProtocol::Uac2 => Uac2ChannelNames::from_bitmap(channel_config)
                .iter()
                .map(|c| ChannelNames::Uac2(*c))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Get USB Audio Device Class channel names from the descriptor "wChannelConfig" field bitmap string based on the protocol
    pub fn get_channel_name_strings<T: Into<u32> + Copy>(
        protocol: &UacProtocol,
        channel_config: T,
    ) -> Vec<String> {
        match protocol {
            UacProtocol::Uac1 => Uac1ChannelNames::from_bitmap(channel_config)
                .iter()
                .map(|c| c.to_string())
                .collect(),
            UacProtocol::Uac2 => Uac2ChannelNames::from_bitmap(channel_config)
                .iter()
                .map(|c| c.to_string())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Get the [`LockDelayUnits`] from the descriptor if it has the field
    pub fn get_lock_delay_units(&self) -> Option<LockDelayUnits> {
        match self {
            UacInterfaceDescriptor::DataStreamingEndpoint1(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            UacInterfaceDescriptor::DatastreamingEndpoint2(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            UacInterfaceDescriptor::DataStreamingEndpoint3(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            _ => None,
        }
    }

    /// Get the [`UacProtocol`] version for the interface descriptor
    pub fn get_protocol(&self) -> UacProtocol {
        match self {
            UacInterfaceDescriptor::Header1(_)
            | UacInterfaceDescriptor::InputTerminal1(_)
            | UacInterfaceDescriptor::OutputTerminal1(_)
            | UacInterfaceDescriptor::MixerUnit1(_)
            | UacInterfaceDescriptor::SelectorUnit1(_)
            | UacInterfaceDescriptor::ProcessingUnit1(_)
            | UacInterfaceDescriptor::FeatureUnit1(_)
            | UacInterfaceDescriptor::ExtensionUnit1(_) => UacProtocol::Uac1,
            UacInterfaceDescriptor::Header2(_)
            | UacInterfaceDescriptor::InputTerminal2(_)
            | UacInterfaceDescriptor::OutputTerminal2(_)
            | UacInterfaceDescriptor::MixerUnit2(_)
            | UacInterfaceDescriptor::SelectorUnit2(_)
            | UacInterfaceDescriptor::ProcessingUnit2(_)
            | UacInterfaceDescriptor::EffectUnit2(_)
            | UacInterfaceDescriptor::FeatureUnit2(_)
            | UacInterfaceDescriptor::ExtensionUnit2(_)
            | UacInterfaceDescriptor::ClockSource2(_)
            | UacInterfaceDescriptor::ClockSelector2(_)
            | UacInterfaceDescriptor::ClockMultiplier2(_)
            | UacInterfaceDescriptor::SampleRateConverter2(_)
            | UacInterfaceDescriptor::StreamingInterface2(_)
            | UacInterfaceDescriptor::DatastreamingEndpoint2(_) => UacProtocol::Uac2,
            UacInterfaceDescriptor::Header3(_)
            | UacInterfaceDescriptor::InputTerminal3(_)
            | UacInterfaceDescriptor::OutputTerminal3(_)
            | UacInterfaceDescriptor::MixerUnit3(_)
            | UacInterfaceDescriptor::SelectorUnit3(_)
            | UacInterfaceDescriptor::ProcessingUnit3(_)
            | UacInterfaceDescriptor::EffectUnit3(_)
            | UacInterfaceDescriptor::FeatureUnit3(_)
            | UacInterfaceDescriptor::ExtensionUnit3(_)
            | UacInterfaceDescriptor::ClockSource3(_)
            | UacInterfaceDescriptor::ClockSelector3(_)
            | UacInterfaceDescriptor::ClockMultiplier3(_)
            | UacInterfaceDescriptor::SampleRateConverter3(_)
            | UacInterfaceDescriptor::StreamingInterface3(_)
            | UacInterfaceDescriptor::DataStreamingEndpoint3(_)
            | UacInterfaceDescriptor::ExtendedTerminalHeader(_)
            | UacInterfaceDescriptor::PowerDomain(_) => UacProtocol::Uac3,
            _ => UacProtocol::Unknown(0xff),
        }
    }
}

/// USB Audio Class (UAC) protocol byte defines the version of the UAC
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum UacProtocol {
    Uac1 = 0x00,
    Uac2 = 0x20,
    Uac3 = 0x30,
    Unknown(u8),
}

impl From<u8> for UacProtocol {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UacProtocol::Uac1,
            0x20 => UacProtocol::Uac2,
            0x30 => UacProtocol::Uac3,
            b => UacProtocol::Unknown(b),
        }
    }
}

impl From<UacProtocol> for u8 {
    fn from(up: UacProtocol) -> u8 {
        match up {
            UacProtocol::Uac1 => 0x00,
            UacProtocol::Uac2 => 0x20,
            UacProtocol::Uac3 => 0x30,
            UacProtocol::Unknown(b) => b,
        }
    }
}

impl std::fmt::Display for UacProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UacProtocol::Uac1 => write!(f, "UAC1"),
            UacProtocol::Uac2 => write!(f, "UAC2"),
            UacProtocol::Uac3 => write!(f, "UAC3"),
            UacProtocol::Unknown(_) => write!(f, "Unknown"),
        }
    }
}

/// USB Audio Class (UAC) subtype based on the bDescriptorSubtype
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UacType {
    /// Audio Control (AC) subtype
    Control(ControlSubtype),
    /// Audio Streaming (AS) subtype
    Streaming(StreamingSubtype),
    /// MIDI subtype
    Midi(MidiSubtype),
}

/// From a tuple of (SubClass, DescriptorSub, Protocol) get the UAC subtype
impl TryFrom<(u8, u8, u8)> for UacType {
    type Error = Error;

    fn try_from((sub_class, descriptor_sub, protocol): (u8, u8, u8)) -> error::Result<Self> {
        match (sub_class, descriptor_sub, protocol) {
            (1, d, p) => Ok(UacType::Control(ControlSubtype::get_uac_subtype(d, p))),
            (2, d, _) => Ok(UacType::Streaming(StreamingSubtype::from(d))),
            (3, d, _) => Ok(UacType::Midi(MidiSubtype::from(d))),
            _ => Err(Error::new(ErrorKind::InvalidArg, "Invalid UAC subtype")),
        }
    }
}

impl From<UacType> for u8 {
    fn from(us: UacType) -> u8 {
        match us {
            UacType::Control(aci) => aci as u8,
            UacType::Streaming(asi) => asi as u8,
            UacType::Midi(mi) => mi as u8,
        }
    }
}

impl fmt::Display for UacType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                UacType::Control(aci) => write!(f, "{:#}", aci),
                UacType::Streaming(asi) => write!(f, "{:#}", asi),
                UacType::Midi(mi) => write!(f, "{:#}", mi),
            }
        } else {
            match self {
                UacType::Control(aci) => write!(f, "{}", aci),
                UacType::Streaming(asi) => write!(f, "{}", asi),
                UacType::Midi(mi) => write!(f, "{}", mi),
            }
        }
    }
}

impl UacType {
    /// Get the [`UacInterfaceDescriptor`] based on UAC subtype, [`UacProtocol`] and raw data
    pub fn get_uac_descriptor(
        &self,
        protocol: u8,
        data: &[u8],
    ) -> Result<UacInterfaceDescriptor, Error> {
        match self {
            UacType::Control(aci) => aci.get_descriptor(&UacProtocol::from(protocol), data),
            UacType::Streaming(asi) => asi.get_descriptor(&UacProtocol::from(protocol), data),
            UacType::Midi(_) => Err(Error::new(
                ErrorKind::InvalidArg,
                "Midi descriptor to UAC not supported, use MidiInterfaceDescriptor",
            )),
        }
    }

    /// Get the [`UacInterfaceDescriptor`] from a generic descriptor
    pub fn uac_descriptor_from_generic(
        &self,
        gd: GenericDescriptor,
        protocol: u8,
    ) -> Result<UacInterfaceDescriptor, Error> {
        match gd.data {
            Some(data) => match self.get_uac_descriptor(protocol, &data) {
                Ok(d) => Ok(d),
                Err(e) => {
                    log::warn!("Error parsing UVC descriptor: {}", e);
                    Ok(UacInterfaceDescriptor::Invalid(data))
                }
            },
            None => Err(Error::new(
                ErrorKind::InvalidArg,
                "GenericDescriptor data is None",
            )),
        }
    }
}

/// USB Audio Class (UAC) interface Audio Control (AC) types based on bDescriptorSubtype
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum ControlSubtype {
    Undefined = 0x00,
    Header = 0x01,
    InputTerminal = 0x02,
    OutputTerminal = 0x03,
    ExtendedTerminal = 0x04,
    MixerUnit = 0x05,
    SelectorUnit = 0x06,
    FeatureUnit = 0x07,
    EffectUnit = 0x08,
    ProcessingUnit = 0x09,
    ExtensionUnit = 0x0a,
    ClockSource = 0x0b,
    ClockSelector = 0x0c,
    ClockMultiplier = 0x0d,
    SampleRateConverter = 0x0e,
    Connectors = 0x0f,
    PowerDomain = 0x10,
}

impl std::fmt::Display for ControlSubtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            // uppercase with _ instead of space for lsusb dump
            match self {
                ControlSubtype::Undefined => write!(f, "unknown"),
                _ => write!(f, "{}", heck::AsShoutySnakeCase(format!("{:?}", self))),
            }
        } else {
            write!(f, "{:?}", self)
        }
    }
}

impl From<u8> for ControlSubtype {
    fn from(b: u8) -> Self {
        match b {
            0x00 => ControlSubtype::Undefined,
            0x01 => ControlSubtype::Header,
            0x02 => ControlSubtype::InputTerminal,
            0x03 => ControlSubtype::OutputTerminal,
            0x04 => ControlSubtype::ExtendedTerminal,
            0x05 => ControlSubtype::MixerUnit,
            0x06 => ControlSubtype::SelectorUnit,
            0x07 => ControlSubtype::FeatureUnit,
            0x08 => ControlSubtype::EffectUnit,
            0x09 => ControlSubtype::ProcessingUnit,
            0x0a => ControlSubtype::ExtensionUnit,
            0x0b => ControlSubtype::ClockSource,
            0x0c => ControlSubtype::ClockSelector,
            0x0d => ControlSubtype::ClockMultiplier,
            0x0e => ControlSubtype::SampleRateConverter,
            0x0f => ControlSubtype::Connectors,
            0x10 => ControlSubtype::PowerDomain,
            _ => ControlSubtype::Undefined,
        }
    }
}

impl ControlSubtype {
    /// UAC1, UAC2, and UAC3 define bDescriptorSubtype differently for the
    /// AudioControl interface, so we need to do some ugly remapping:
    pub fn get_uac_subtype(subtype: u8, protocol: u8) -> Self {
        match protocol {
            // UAC1
            0x00 => match subtype {
                0x04 => ControlSubtype::MixerUnit,
                0x05 => ControlSubtype::SelectorUnit,
                0x06 => ControlSubtype::FeatureUnit,
                0x07 => ControlSubtype::ProcessingUnit,
                0x08 => ControlSubtype::ExtensionUnit,
                _ => Self::from(subtype),
            },
            // UAC2
            0x20 => match subtype {
                0x04 => ControlSubtype::MixerUnit,
                0x05 => ControlSubtype::SelectorUnit,
                0x06 => ControlSubtype::FeatureUnit,
                0x07 => ControlSubtype::EffectUnit,
                0x08 => ControlSubtype::ProcessingUnit,
                0x09 => ControlSubtype::ExtensionUnit,
                0x0a => ControlSubtype::ClockSource,
                0x0b => ControlSubtype::ClockSelector,
                0x0c => ControlSubtype::ClockMultiplier,
                0x0d => ControlSubtype::SampleRateConverter,
                _ => Self::from(subtype),
            },
            // no re-map for UAC3..
            _ => Self::from(subtype),
        }
    }

    /// Get the UAC interface descriptor from the UAC interface
    pub fn get_descriptor(
        &self,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<UacInterfaceDescriptor, Error> {
        UacInterfaceDescriptor::from_uac_ac_interface(self, protocol, data)
    }
}

/// USB Audio Class (UAC) interface Audio Streaming (AS) types based on bDescriptorSubtype
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum StreamingSubtype {
    Undefined = 0x00,
    General = 0x01,
    FormatType = 0x02,
    FormatSpecific = 0x03,
}

impl From<u8> for StreamingSubtype {
    fn from(b: u8) -> Self {
        match b {
            0x00 => StreamingSubtype::Undefined,
            0x01 => StreamingSubtype::General,
            0x02 => StreamingSubtype::FormatType,
            0x03 => StreamingSubtype::FormatSpecific,
            _ => StreamingSubtype::Undefined,
        }
    }
}

impl fmt::Display for StreamingSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // uppercase with _ instead of space for lsusb dump
            write!(f, "{}", heck::AsShoutySnakeCase(format!("{:?}", self)))
        } else {
            write!(f, "{:?}", self)
        }
    }
}

impl StreamingSubtype {
    /// Get the UAC interface descriptor from the UAC interface
    pub fn get_descriptor(
        &self,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<UacInterfaceDescriptor, Error> {
        UacInterfaceDescriptor::from_uac_as_interface(self, protocol, data)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[serde(rename_all = "kebab-case")]
pub enum StreamingFormatType {
    TypeI = 0x01,
    TypeII = 0x02,
    TypeIII = 0x03,
    TypeIV = 0x04,
    Undefined(u8),
}

impl From<u8> for StreamingFormatType {
    fn from(b: u8) -> Self {
        match b {
            0x01 => StreamingFormatType::TypeI,
            0x02 => StreamingFormatType::TypeII,
            0x03 => StreamingFormatType::TypeIII,
            0x04 => StreamingFormatType::TypeIV,
            b => StreamingFormatType::Undefined(b),
        }
    }
}

impl From<StreamingFormatType> for u8 {
    fn from(sft: StreamingFormatType) -> u8 {
        match sft {
            StreamingFormatType::TypeI => 0x01,
            StreamingFormatType::TypeII => 0x02,
            StreamingFormatType::TypeIII => 0x03,
            StreamingFormatType::TypeIV => 0x04,
            StreamingFormatType::Undefined(b) => b,
        }
    }
}

impl fmt::Display for StreamingFormatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                StreamingFormatType::TypeI => write!(f, "FORMAT_TYPE_I"),
                StreamingFormatType::TypeII => write!(f, "FORMAT_TYPE_II"),
                StreamingFormatType::TypeIII => write!(f, "FORMAT_TYPE_III"),
                StreamingFormatType::TypeIV => write!(f, "FORMAT_TYPE_IV"),
                StreamingFormatType::Undefined(_) => write!(f, "invalid"),
            }
        } else {
            write!(f, "{:?}", self)
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum SampleFrequencyType {
    Continuous,
    Discrete(u8),
}

impl From<u8> for SampleFrequencyType {
    fn from(b: u8) -> Self {
        match b {
            0x00 => SampleFrequencyType::Continuous,
            b => SampleFrequencyType::Discrete(b),
        }
    }
}

impl From<SampleFrequencyType> for u8 {
    fn from(sft: SampleFrequencyType) -> u8 {
        match sft {
            SampleFrequencyType::Continuous => 0x00,
            SampleFrequencyType::Discrete(b) => b,
        }
    }
}

impl fmt::Display for SampleFrequencyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SampleFrequencyType::Continuous => write!(f, "Continuous"),
            SampleFrequencyType::Discrete(_) => write!(f, "Discrete"),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum StreamingFormatInterface {
    FormatTypeI1(FormatTypeI1),
    FormatTypeII1(FormatTypeII1),
    FormatTypeIII1(FormatTypeIII1),
    FormatTypeI2(FormatTypeI2),
    FormatTypeII2(FormatTypeII2),
    FormatTypeIII2(FormatTypeIII2),
    FormatSpecificMpeg(FormatSpecificMpeg),
    FormatSpecificAc3(FormatSpecificAc3),
    Invalid(Vec<u8>),
    Undefined(Vec<u8>),
}

impl From<StreamingFormatInterface> for Vec<u8> {
    fn from(sfi: StreamingFormatInterface) -> Vec<u8> {
        match sfi {
            StreamingFormatInterface::FormatTypeI1(ft) => ft.into(),
            StreamingFormatInterface::FormatTypeII1(ft) => ft.into(),
            StreamingFormatInterface::FormatTypeIII1(ft) => ft.into(),
            StreamingFormatInterface::FormatTypeI2(ft) => ft.into(),
            StreamingFormatInterface::FormatTypeII2(ft) => ft.into(),
            StreamingFormatInterface::FormatTypeIII2(ft) => ft.into(),
            StreamingFormatInterface::FormatSpecificMpeg(ft) => ft.into(),
            StreamingFormatInterface::FormatSpecificAc3(ft) => ft.into(),
            StreamingFormatInterface::Invalid(data) => data,
            StreamingFormatInterface::Undefined(data) => data,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct StreamingFormat {
    pub format_type: StreamingFormatType,
    pub interface: StreamingFormatInterface,
}

impl StreamingFormat {
    /// Get the StreamingFormat from the UAC AS interface
    pub fn from_uac_as_interface(protocol: &UacProtocol, data: &[u8]) -> Result<Self, Error> {
        if data.is_empty() {
            return Err(Error::new_descriptor_len("StreamingFormat", 1, data.len()));
        }

        let format_type = StreamingFormatType::from(data[0]);
        match protocol {
            UacProtocol::Uac1 => match format_type {
                StreamingFormatType::TypeI => {
                    FormatTypeI1::try_from(&data[1..]).map(|ft| StreamingFormat {
                        format_type,
                        interface: StreamingFormatInterface::FormatTypeI1(ft),
                    })
                }
                StreamingFormatType::TypeII => {
                    FormatTypeII1::try_from(&data[1..]).map(|ft| StreamingFormat {
                        format_type,
                        interface: StreamingFormatInterface::FormatTypeII1(ft),
                    })
                }
                StreamingFormatType::TypeIII => {
                    FormatTypeIII1::try_from(&data[1..]).map(|ft| StreamingFormat {
                        format_type,
                        interface: StreamingFormatInterface::FormatTypeIII1(ft),
                    })
                }
                _ => Ok(StreamingFormat {
                    format_type,
                    interface: StreamingFormatInterface::Undefined(data[1..].to_vec()),
                }),
            },
            UacProtocol::Uac2 => match format_type {
                StreamingFormatType::TypeI => {
                    FormatTypeI2::try_from(&data[1..]).map(|ft| StreamingFormat {
                        format_type,
                        interface: StreamingFormatInterface::FormatTypeI2(ft),
                    })
                }
                StreamingFormatType::TypeII => {
                    FormatTypeII2::try_from(&data[1..]).map(|ft| StreamingFormat {
                        format_type,
                        interface: StreamingFormatInterface::FormatTypeII2(ft),
                    })
                }
                StreamingFormatType::TypeIII => {
                    FormatTypeIII2::try_from(&data[1..]).map(|ft| StreamingFormat {
                        format_type,
                        interface: StreamingFormatInterface::FormatTypeIII2(ft),
                    })
                }
                _ => Ok(StreamingFormat {
                    format_type,
                    interface: StreamingFormatInterface::Undefined(data[1..].to_vec()),
                }),
            },
            _ => Ok(StreamingFormat {
                format_type,
                interface: StreamingFormatInterface::Invalid(data[1..].to_vec()),
            }),
        }
    }
}

impl From<StreamingFormat> for Vec<u8> {
    fn from(sf: StreamingFormat) -> Vec<u8> {
        let mut data = vec![sf.format_type.into()];
        data.extend_from_slice(&Vec::<u8>::from(sf.interface));
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct StreamingFormatSpecific {
    pub format_tag: u16,
    pub interface: StreamingFormatInterface,
}

impl StreamingFormatSpecific {
    /// Get the StreamingFormatSpecific from the UAC AS interface
    pub fn from_uac_as_interface(_protocol: &UacProtocol, data: &[u8]) -> Result<Self, Error> {
        StreamingFormatSpecific::try_from(data)
    }
}

impl TryFrom<&[u8]> for StreamingFormatSpecific {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 2 {
            return Err(Error::new_descriptor_len(
                "StreamingFormatSpecific",
                2,
                value.len(),
            ));
        }

        let format_tag = u16::from_le_bytes([value[0], value[1]]);
        let format = match format_tag {
            0x1001 => StreamingFormatInterface::FormatSpecificMpeg(FormatSpecificMpeg::try_from(
                &value[2..],
            )?),
            0x1002 => StreamingFormatInterface::FormatSpecificAc3(FormatSpecificAc3::try_from(
                &value[2..],
            )?),
            _ => StreamingFormatInterface::Undefined(value[2..].to_vec()),
        };

        Ok(StreamingFormatSpecific {
            format_tag,
            interface: format,
        })
    }
}

impl From<StreamingFormatSpecific> for Vec<u8> {
    fn from(sfs: StreamingFormatSpecific) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&sfs.format_tag.to_le_bytes());
        data.extend_from_slice(&Vec::<u8>::from(sfs.interface));
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatTypeI1 {
    pub num_channels: u8,
    pub subframe_size: u8,
    pub bit_resolution: u8,
    pub sample_frequency_type: SampleFrequencyType,
    pub sample_frequencies: Vec<u32>,
}

impl TryFrom<&[u8]> for FormatTypeI1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("FormatTypeI", 4, value.len()));
        }

        let nr_channels = value[0];
        let subframe_size = value[1];
        let bit_resolution = value[2];
        let sam_freq_type: SampleFrequencyType = value[3].into();

        let mut sam_freqs = Vec::new();
        match sam_freq_type {
            SampleFrequencyType::Continuous => {
                if value.len() < 10 {
                    return Err(Error::new(
                        ErrorKind::InvalidDescriptor,
                        "Descriptor too short for continuous sample frequency",
                    ));
                }
                sam_freqs.push(u32::from_le_bytes([value[4], value[5], value[6], 0]));
                sam_freqs.push(u32::from_le_bytes([value[7], value[8], value[9], 0]));
            }
            SampleFrequencyType::Discrete(b) => {
                for i in 0..b {
                    let index = 4 + (i as usize * 3);
                    if value.len() < index + 3 {
                        return Err(Error::new(
                            ErrorKind::InvalidDescriptor,
                            "Descriptor too short for discrete sample frequency",
                        ));
                    }
                    sam_freqs.push(u32::from_le_bytes([
                        value[index],
                        value[index + 1],
                        value[index + 2],
                        0,
                    ]));
                }
            }
        }

        Ok(FormatTypeI1 {
            num_channels: nr_channels,
            subframe_size,
            bit_resolution,
            sample_frequency_type: sam_freq_type,
            sample_frequencies: sam_freqs,
        })
    }
}

impl From<FormatTypeI1> for Vec<u8> {
    fn from(ft: FormatTypeI1) -> Vec<u8> {
        let mut data = vec![ft.num_channels, ft.subframe_size, ft.bit_resolution];
        data.push(ft.sample_frequency_type.into());
        for sf in ft.sample_frequencies {
            data.extend_from_slice(&sf.to_le_bytes());
        }
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatTypeII1 {
    pub max_bit_rate: u16,
    pub samples_per_frame: u16,
    pub sample_frequency_type: SampleFrequencyType,
    pub sample_frequencies: Vec<u32>,
}

impl TryFrom<&[u8]> for FormatTypeII1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 5 {
            return Err(Error::new_descriptor_len("FormatTypeII", 5, value.len()));
        }

        let max_bit_rate = u16::from_le_bytes([value[0], value[1]]);
        let samples_per_frame = u16::from_le_bytes([value[2], value[3]]);
        let sam_freq_type: SampleFrequencyType = value[4].into();

        let mut sam_freqs = Vec::new();
        match sam_freq_type {
            SampleFrequencyType::Continuous => {
                if value.len() < 11 {
                    return Err(Error::new(
                        ErrorKind::InvalidDescriptor,
                        "Descriptor too short for continuous sample frequency",
                    ));
                }
                sam_freqs.push(u32::from_le_bytes([value[5], value[6], value[7], 0]));
                sam_freqs.push(u32::from_le_bytes([value[8], value[9], value[10], 0]));
            }
            SampleFrequencyType::Discrete(b) => {
                for i in 0..b {
                    let index = 5 + (i as usize * 3);
                    if value.len() < index + 3 {
                        return Err(Error::new(
                            ErrorKind::InvalidDescriptor,
                            "Descriptor too short for discrete sample frequency",
                        ));
                    }
                    sam_freqs.push(u32::from_le_bytes([
                        value[index],
                        value[index + 1],
                        value[index + 2],
                        0,
                    ]));
                }
            }
        }

        Ok(FormatTypeII1 {
            max_bit_rate,
            samples_per_frame,
            sample_frequency_type: sam_freq_type,
            sample_frequencies: sam_freqs,
        })
    }
}

impl From<FormatTypeII1> for Vec<u8> {
    fn from(ft: FormatTypeII1) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&ft.max_bit_rate.to_le_bytes());
        data.extend_from_slice(&ft.samples_per_frame.to_le_bytes());
        data.push(ft.sample_frequency_type.into());
        for sf in ft.sample_frequencies {
            data.extend_from_slice(&sf.to_le_bytes());
        }
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatTypeIII1 {
    pub num_channels: u8,
    pub subframe_size: u8,
    pub bit_resolution: u8,
    pub sample_frequency_type: SampleFrequencyType,
    pub sample_frequencies: Vec<u32>,
}

impl TryFrom<&[u8]> for FormatTypeIII1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let sam_freq_type: SampleFrequencyType = match value.get(4) {
            Some(&sam_freq_type) => sam_freq_type.into(),
            None => return Err(Error::new_descriptor_len("FormatTypeIII", 4, value.len())),
        };
        let expected_len = match sam_freq_type {
            SampleFrequencyType::Continuous => 10,
            SampleFrequencyType::Discrete(b) => 4 + (b as usize * 3),
        };

        if value.len() < expected_len {
            return Err(Error::new_descriptor_len(
                "FormatTypeIII",
                expected_len,
                value.len(),
            ));
        }

        let nr_channels = value[0];
        let subframe_size = value[1];
        let bit_resolution = value[2];

        let mut sam_freqs = Vec::new();
        match sam_freq_type {
            SampleFrequencyType::Continuous => {
                sam_freqs.push(u32::from_le_bytes([value[4], value[5], value[6], 0]));
                sam_freqs.push(u32::from_le_bytes([value[7], value[8], value[9], 0]));
            }
            SampleFrequencyType::Discrete(b) => {
                for i in 0..b {
                    let index = 4 + (i as usize * 3);
                    sam_freqs.push(u32::from_le_bytes([
                        value[index],
                        value[index + 1],
                        value[index + 2],
                        0,
                    ]));
                }
            }
        }

        Ok(FormatTypeIII1 {
            num_channels: nr_channels,
            subframe_size,
            bit_resolution,
            sample_frequency_type: sam_freq_type,
            sample_frequencies: sam_freqs,
        })
    }
}

impl From<FormatTypeIII1> for Vec<u8> {
    fn from(ft: FormatTypeIII1) -> Vec<u8> {
        let mut data = vec![ft.num_channels, ft.subframe_size, ft.bit_resolution];
        data.push(ft.sample_frequency_type.into());
        for sf in ft.sample_frequencies {
            data.extend_from_slice(&sf.to_le_bytes());
        }
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatTypeI2 {
    pub sub_slot_size: u8,
    pub bit_resolution: u8,
}

impl TryFrom<&[u8]> for FormatTypeI2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 2 {
            return Err(Error::new_descriptor_len("FormatTypeI2", 2, value.len()));
        }

        Ok(FormatTypeI2 {
            sub_slot_size: value[0],
            bit_resolution: value[1],
        })
    }
}

impl From<FormatTypeI2> for Vec<u8> {
    fn from(ft: FormatTypeI2) -> Vec<u8> {
        vec![ft.sub_slot_size, ft.bit_resolution]
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatTypeII2 {
    pub max_bit_rate: u16,
    pub slots_per_frame: u16,
}

impl TryFrom<&[u8]> for FormatTypeII2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("FormatTypeII2", 4, value.len()));
        }

        Ok(FormatTypeII2 {
            max_bit_rate: u16::from_le_bytes([value[0], value[1]]),
            slots_per_frame: u16::from_le_bytes([value[2], value[3]]),
        })
    }
}

impl From<FormatTypeII2> for Vec<u8> {
    fn from(ft: FormatTypeII2) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&ft.max_bit_rate.to_le_bytes());
        data.extend_from_slice(&ft.slots_per_frame.to_le_bytes());
        data
    }
}

#[allow(missing_docs)]
pub type FormatTypeIII2 = FormatTypeI2;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatSpecificMpeg {
    pub mpeg_capabilities: u16,
    pub mpeg_features: u8,
}

impl TryFrom<&[u8]> for FormatSpecificMpeg {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 3 {
            return Err(Error::new_descriptor_len(
                "FormatSpecificMpeg",
                3,
                value.len(),
            ));
        }

        Ok(FormatSpecificMpeg {
            mpeg_capabilities: u16::from_le_bytes([value[0], value[1]]),
            mpeg_features: value[3],
        })
    }
}

impl From<FormatSpecificMpeg> for Vec<u8> {
    fn from(ft: FormatSpecificMpeg) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(ft.mpeg_features);
        data.extend_from_slice(&ft.mpeg_capabilities.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FormatSpecificAc3 {
    pub bsid: u32,
    pub ac3_features: u8,
}

impl TryFrom<&[u8]> for FormatSpecificAc3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 5 {
            return Err(Error::new_descriptor_len(
                "FormatSpecificAc3",
                5,
                value.len(),
            ));
        }

        Ok(FormatSpecificAc3 {
            bsid: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
            ac3_features: value[4],
        })
    }
}

impl From<FormatSpecificAc3> for Vec<u8> {
    fn from(ft: FormatSpecificAc3) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&ft.bsid.to_le_bytes());
        data.push(ft.ac3_features);
        data
    }
}

/// The control setting for a UAC bmControls byte
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum ControlSetting {
    ReadOnly = 0b01,
    IllegalValue = 0b10,
    ReadWrite = 0b11,
}

impl From<u8> for ControlSetting {
    fn from(b: u8) -> Self {
        match b {
            0b01 => ControlSetting::ReadOnly,
            0b10 => ControlSetting::IllegalValue,
            0b11 => ControlSetting::ReadWrite,
            _ => ControlSetting::IllegalValue,
        }
    }
}

impl fmt::Display for ControlSetting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ControlSetting::ReadOnly => write!(f, "read-only"),
            ControlSetting::IllegalValue => write!(f, "ILLEGAL VALUE (0b10)"),
            ControlSetting::ReadWrite => write!(f, "read/write"),
        }
    }
}

/// UAC bmControl can be 1 bit for just the control type or 2 bits for control type and whether it's read-only
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum ControlType {
    BmControl1,
    BmControl2,
}

/// UAC1: 4.3.2 Class-Specific AC Interface Descriptor; Table 4-2.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Header1 {
    pub version: Version,
    pub total_length: u16,
    pub collection_bytes: u8,
    pub interfaces: Vec<u8>,
}

impl TryFrom<&[u8]> for Header1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len("Header1", 6, value.len()));
        }

        let total_length = u16::from_le_bytes([value[2], value[3]]);
        let collection_bytes = value[4];
        let interfaces = value[5..].to_vec();

        Ok(Header1 {
            version: Version::from_bcd(u16::from_le_bytes([value[0], value[1]])),
            total_length,
            collection_bytes,
            interfaces,
        })
    }
}

impl From<Header1> for Vec<u8> {
    fn from(val: Header1) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(&(u16::from(val.version)).to_le_bytes());
        data.extend_from_slice(&val.total_length.to_le_bytes());
        data.push(val.collection_bytes);
        data.extend_from_slice(&val.interfaces);
        data
    }
}

/// UAC2: 4.7.2 Class-Specific AC Interface Descriptor; Table 4-5.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Header2 {
    pub version: Version,
    pub category: u8,
    pub total_length: u16,
    pub controls: u8,
}

impl TryFrom<&[u8]> for Header2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len("Header2", 6, value.len()));
        }

        let total_length = u16::from_le_bytes([value[3], value[4]]);
        let controls = value[5];

        Ok(Header2 {
            version: Version::from_bcd(u16::from_le_bytes([value[0], value[1]])),
            category: value[2],
            total_length,
            controls,
        })
    }
}

impl From<Header2> for Vec<u8> {
    fn from(val: Header2) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(&(u16::from(val.version)).to_le_bytes());
        data.push(val.category);
        data.extend_from_slice(&val.total_length.to_le_bytes());
        data.push(val.controls);
        data
    }
}

/// UAC3: 4.5.2 Class-Specific AC Interface Descriptor; Table 4-15.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Header3 {
    pub category: u8,
    pub total_length: u16,
    pub controls: u32,
}

impl TryFrom<&[u8]> for Header3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new_descriptor_len("Header3", 7, value.len()));
        }

        let total_length = u16::from_le_bytes([value[1], value[2]]);
        let controls = u32::from_le_bytes([value[3], value[4], value[5], value[6]]);

        Ok(Header3 {
            category: value[0],
            total_length,
            controls,
        })
    }
}

impl From<Header3> for Vec<u8> {
    fn from(val: Header3) -> Self {
        let mut data = Vec::new();
        data.push(val.category);
        data.extend_from_slice(&val.total_length.to_le_bytes());
        data.extend_from_slice(&val.controls.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.1 Input Terminal Descriptor; Table 4-3.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct InputTerminal1 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub nr_channels: u8,
    pub channel_config: u16,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for InputTerminal1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new_descriptor_len("InputTerminal1", 9, value.len()));
        }

        Ok(InputTerminal1 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            nr_channels: value[4],
            channel_config: u16::from_le_bytes([value[5], value[6]]),
            channel_names_index: value[7],
            channel_names: None,
            terminal_index: value[8],
            terminal: None,
        })
    }
}

impl From<InputTerminal1> for Vec<u8> {
    fn from(val: InputTerminal1) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_id);
        data.extend_from_slice(&val.terminal_type.to_le_bytes());
        data.push(val.assoc_terminal);
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names_index);
        data.push(val.terminal_index);
        data
    }
}

/// UAC2: 4.7.2.4 Input Terminal Descriptor; Table 4-9.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct InputTerminal2 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub csource_id: u8,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
    pub controls: u16,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for InputTerminal2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 14 {
            return Err(Error::new_descriptor_len("InputTerminal2", 14, value.len()));
        }

        Ok(InputTerminal2 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            csource_id: value[4],
            nr_channels: value[5],
            channel_config: u32::from_le_bytes([value[6], value[7], value[8], value[9]]),
            channel_names_index: value[10],
            channel_names: None,
            controls: u16::from_le_bytes([value[11], value[12]]),
            terminal_index: value[13],
            terminal: None,
        })
    }
}

impl From<InputTerminal2> for Vec<u8> {
    fn from(val: InputTerminal2) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_id);
        data.extend_from_slice(&val.terminal_type.to_le_bytes());
        data.push(val.assoc_terminal);
        data.push(val.csource_id);
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names_index);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.push(val.terminal_index);
        data
    }
}

/// UAC3: 4.5.2.1 Input Terminal Descriptor; Table 4-16.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct InputTerminal3 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub csource_id: u8,
    pub controls: u32,
    pub cluster_descr_id: u16,
    pub ex_terminal_descr_id: u16,
    pub connectors_descr_id: u16,
    pub terminal_descr_str: u16,
}

impl TryFrom<&[u8]> for InputTerminal3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 17 {
            return Err(Error::new_descriptor_len("InputTerminal3", 17, value.len()));
        }

        Ok(InputTerminal3 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            csource_id: value[4],
            controls: u32::from_le_bytes([value[5], value[6], value[7], value[8]]),
            cluster_descr_id: u16::from_le_bytes([value[9], value[10]]),
            ex_terminal_descr_id: u16::from_le_bytes([value[11], value[12]]),
            connectors_descr_id: u16::from_le_bytes([value[13], value[14]]),
            terminal_descr_str: u16::from_le_bytes([value[15], value[16]]),
        })
    }
}

impl From<InputTerminal3> for Vec<u8> {
    fn from(val: InputTerminal3) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_id);
        data.extend_from_slice(&val.terminal_type.to_le_bytes());
        data.push(val.assoc_terminal);
        data.push(val.csource_id);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.cluster_descr_id.to_le_bytes());
        data.extend_from_slice(&val.ex_terminal_descr_id.to_le_bytes());
        data.extend_from_slice(&val.connectors_descr_id.to_le_bytes());
        data.extend_from_slice(&val.terminal_descr_str.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.2 Output Terminal Descriptor; Table 4-4.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct OutputTerminal1 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub source_id: u8,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for OutputTerminal1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len("OutputTerminal1", 6, value.len()));
        }

        Ok(OutputTerminal1 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            source_id: value[4],
            terminal_index: value[5],
            terminal: None,
        })
    }
}

impl From<OutputTerminal1> for Vec<u8> {
    fn from(val: OutputTerminal1) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_id);
        data.extend_from_slice(&val.terminal_type.to_le_bytes());
        data.push(val.assoc_terminal);
        data.push(val.source_id);
        data.push(val.terminal_index);
        data
    }
}

/// UAC2: 4.7.2.5 Output Terminal Descriptor; Table 4-10.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct OutputTerminal2 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub source_id: u8,
    pub c_source_id: u8,
    pub controls: u16,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for OutputTerminal2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new_descriptor_len("OutputTerminal2", 9, value.len()));
        }

        Ok(OutputTerminal2 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            source_id: value[4],
            c_source_id: value[5],
            controls: u16::from_le_bytes([value[6], value[7]]),
            terminal_index: value[8],
            terminal: None,
        })
    }
}

impl From<OutputTerminal2> for Vec<u8> {
    fn from(val: OutputTerminal2) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_id);
        data.extend_from_slice(&val.terminal_type.to_le_bytes());
        data.push(val.assoc_terminal);
        data.push(val.source_id);
        data.push(val.c_source_id);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.push(val.terminal_index);
        data
    }
}

/// UAC3: 4.5.2.2 Output Terminal Descriptor; Table 4-17.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct OutputTerminal3 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub source_id: u8,
    pub c_source_id: u8,
    pub controls: u32,
    pub ex_terminal_descr_id: u16,
    pub connectors_descr_id: u16,
    pub terminal_descr_str: u16,
}

impl TryFrom<&[u8]> for OutputTerminal3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 16 {
            return Err(Error::new_descriptor_len(
                "OutputTerminal3",
                16,
                value.len(),
            ));
        }

        Ok(OutputTerminal3 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            source_id: value[4],
            c_source_id: value[5],
            controls: u32::from_le_bytes([value[6], value[7], value[8], value[9]]),
            ex_terminal_descr_id: u16::from_le_bytes([value[10], value[11]]),
            connectors_descr_id: u16::from_le_bytes([value[12], value[13]]),
            terminal_descr_str: u16::from_le_bytes([value[14], value[15]]),
        })
    }
}

impl From<OutputTerminal3> for Vec<u8> {
    fn from(val: OutputTerminal3) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_id);
        data.extend_from_slice(&val.terminal_type.to_le_bytes());
        data.push(val.assoc_terminal);
        data.push(val.source_id);
        data.push(val.c_source_id);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.ex_terminal_descr_id.to_le_bytes());
        data.extend_from_slice(&val.connectors_descr_id.to_le_bytes());
        data.extend_from_slice(&val.terminal_descr_str.to_le_bytes());
        data
    }
}

/// UAC3: 4.5.2.3.1 Extended Terminal Header Descriptor; Table 4-18.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ExtendedTerminalHeader {
    pub descriptor_id: u8,
    pub nr_channels: u8,
}

impl TryFrom<&[u8]> for ExtendedTerminalHeader {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 2 {
            return Err(Error::new_descriptor_len(
                "ExtendedTerminalHeader",
                2,
                value.len(),
            ));
        }

        Ok(ExtendedTerminalHeader {
            descriptor_id: value[0],
            nr_channels: value[1],
        })
    }
}

impl From<ExtendedTerminalHeader> for Vec<u8> {
    fn from(val: ExtendedTerminalHeader) -> Self {
        vec![val.descriptor_id, val.nr_channels]
    }
}

/// UAC3: 4.5.2.15 Power Domain Descriptor; Table 4-46. */
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct PowerDomain {
    pub power_domain_id: u8,
    pub recovery_time_1: u16,
    pub recovery_time_2: u16,
    pub nr_entities: u8,
    pub entity_ids: Vec<u8>,
    pub domain_descr_str: u16,
}

impl TryFrom<&[u8]> for PowerDomain {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new_descriptor_len("PowerDomain", 8, value.len()));
        }

        let nr_entities = value[5] as usize;
        let expected_len = 8 + nr_entities;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "PowerDomain3 descriptor too short for the number of entities",
            ));
        }

        Ok(PowerDomain {
            power_domain_id: value[0],
            recovery_time_1: u16::from_le_bytes([value[1], value[2]]),
            recovery_time_2: u16::from_le_bytes([value[3], value[4]]),
            nr_entities: value[5],
            entity_ids: value[6..6 + nr_entities].to_vec(),
            domain_descr_str: u16::from_le_bytes([value[6 + nr_entities], value[7 + nr_entities]]),
        })
    }
}

impl From<PowerDomain> for Vec<u8> {
    fn from(val: PowerDomain) -> Self {
        let mut data = Vec::new();
        data.push(val.power_domain_id);
        data.extend_from_slice(&val.recovery_time_1.to_le_bytes());
        data.extend_from_slice(&val.recovery_time_2.to_le_bytes());
        data.push(val.nr_entities);
        data.extend_from_slice(&val.entity_ids);
        data.extend_from_slice(&val.domain_descr_str.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.3 Mixer Unit Descriptor; Table 4-5.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MixerUnit1 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u16,
    pub channel_names: u8,
    pub controls: Vec<u8>,
    pub mixer: u8,
}

impl TryFrom<&[u8]> for MixerUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len("MixerUnit1", 6, value.len()));
        }

        let nr_in_pins = value[1] as usize;
        let nr_channels = value[3] as usize;
        let expected_len = 6 + nr_in_pins + nr_channels;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "MixerUnit1 descriptor too short for the number of pins and channels",
            ));
        }

        Ok(MixerUnit1 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids: value[2..2 + nr_in_pins].to_vec(),
            nr_channels: value[2 + nr_in_pins],
            channel_config: u16::from_le_bytes([value[3 + nr_in_pins], value[4 + nr_in_pins]]),
            channel_names: value[5 + nr_in_pins],
            controls: value[6 + nr_in_pins..6 + nr_in_pins + nr_channels].to_vec(),
            mixer: value[6 + nr_in_pins + nr_channels],
        })
    }
}

impl From<MixerUnit1> for Vec<u8> {
    fn from(val: MixerUnit1) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names);
        data.extend_from_slice(&val.controls);
        data.push(val.mixer);
        data
    }
}

/// UAC2: 4.7.2.6 Mixer Unit Descriptor; Table 4-11.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MixerUnit2 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names: u8,
    pub mixer_controls: Vec<u8>,
    pub controls: u8,
    pub mixer: u8,
}

impl TryFrom<&[u8]> for MixerUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new_descriptor_len("MixerUnit2", 10, value.len()));
        }

        let nr_in_pins = value[1] as usize;
        let nr_channels = value[3] as usize;
        let expected_len = 10 + nr_in_pins + nr_channels;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "MixerUnit2 descriptor too short for the number of pins and channels",
            ));
        }

        Ok(MixerUnit2 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids: value[2..2 + nr_in_pins].to_vec(),
            nr_channels: value[2 + nr_in_pins],
            channel_config: u32::from_le_bytes([
                value[3 + nr_in_pins],
                value[4 + nr_in_pins],
                value[5 + nr_in_pins],
                value[6 + nr_in_pins],
            ]),
            channel_names: value[7 + nr_in_pins],
            mixer_controls: value[8 + nr_in_pins..8 + nr_in_pins + nr_channels].to_vec(),
            controls: value[8 + nr_in_pins + nr_channels],
            mixer: value[9 + nr_in_pins + nr_channels],
        })
    }
}

impl From<MixerUnit2> for Vec<u8> {
    fn from(val: MixerUnit2) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names);
        data.extend_from_slice(&val.mixer_controls);
        data.push(val.controls);
        data.push(val.mixer);
        data
    }
}

/// UAC3: 4.5.2.5 Mixer Unit Descriptor; Table 4-29.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MixerUnit3 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub cluster_descr_id: u16,
    pub mixer_controls: Vec<u8>,
    pub controls: u32,
    pub mixer_descr_str: u16,
}

impl TryFrom<&[u8]> for MixerUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new_descriptor_len("MixerUnit3", 8, value.len()));
        }

        let nr_in_pins = value[1] as usize;
        let expected_len = 8 + nr_in_pins;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "MixerUnit3 descriptor too short for the number of pins",
            ));
        }

        Ok(MixerUnit3 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids: value[2..2 + nr_in_pins].to_vec(),
            cluster_descr_id: u16::from_le_bytes([value[2 + nr_in_pins], value[3 + nr_in_pins]]),
            mixer_controls: value[4 + nr_in_pins..4 + nr_in_pins + 1].to_vec(),
            controls: u32::from_le_bytes([
                value[5 + nr_in_pins],
                value[6 + nr_in_pins],
                value[7 + nr_in_pins],
                value[8 + nr_in_pins],
            ]),
            mixer_descr_str: u16::from_le_bytes([value[9 + nr_in_pins], value[10 + nr_in_pins]]),
        })
    }
}

impl From<MixerUnit3> for Vec<u8> {
    fn from(val: MixerUnit3) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.extend_from_slice(&val.cluster_descr_id.to_le_bytes());
        data.extend_from_slice(&val.mixer_controls);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.mixer_descr_str.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct StreamingInterface1 {
    pub terminal_link: u8,
    pub delay: u8,
    pub format_tag: u16,
}

impl TryFrom<&[u8]> for StreamingInterface1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len(
                "StreamingInterface1",
                4,
                value.len(),
            ));
        }

        Ok(StreamingInterface1 {
            terminal_link: value[0],
            delay: value[1],
            format_tag: u16::from_le_bytes([value[2], value[3]]),
        })
    }
}

impl From<StreamingInterface1> for Vec<u8> {
    fn from(val: StreamingInterface1) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_link);
        data.push(val.delay);
        data.extend_from_slice(&val.format_tag.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct StreamingInterface2 {
    pub terminal_link: u8,
    pub controls: u8, // BmControl2
    pub format_type: u8,
    pub formats: u32,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
}

impl TryFrom<&[u8]> for StreamingInterface2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 13 {
            return Err(Error::new_descriptor_len(
                "StreamingInterface2",
                13,
                value.len(),
            ));
        }

        Ok(StreamingInterface2 {
            terminal_link: value[0],
            controls: value[1],
            format_type: value[2],
            formats: u32::from_le_bytes([value[3], value[4], value[5], value[6]]),
            nr_channels: value[7],
            channel_config: u32::from_le_bytes([value[8], value[9], value[10], value[11]]),
            channel_names_index: value[12],
            channel_names: None,
        })
    }
}

impl From<StreamingInterface2> for Vec<u8> {
    fn from(val: StreamingInterface2) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_link);
        data.push(val.controls);
        data.push(val.format_type);
        data.extend_from_slice(&val.formats.to_le_bytes());
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names_index);
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct StreamingInterface3 {
    pub terminal_link: u8,
    pub controls: u32, // BmControl2
    pub cluster_descr_id: u16,
    pub formats: u64,
    pub sub_slot_size: u8,
    pub bit_resolution: u8,
    pub aux_protocols: u16,
    pub control_size: u8,
}

impl TryFrom<&[u8]> for StreamingInterface3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 20 {
            return Err(Error::new_descriptor_len(
                "StreamingInterface3",
                20,
                value.len(),
            ));
        }

        Ok(StreamingInterface3 {
            terminal_link: value[0],
            controls: u32::from_le_bytes([value[1], value[2], value[3], value[4]]),
            cluster_descr_id: u16::from_le_bytes([value[5], value[6]]),
            formats: u64::from_le_bytes([
                value[7], value[8], value[9], value[10], value[11], value[12], value[13], value[14],
            ]),
            sub_slot_size: value[15],
            bit_resolution: value[16],
            aux_protocols: u16::from_le_bytes([value[17], value[18]]),
            control_size: value[19],
        })
    }
}

impl From<StreamingInterface3> for Vec<u8> {
    fn from(val: StreamingInterface3) -> Self {
        let mut data = Vec::new();
        data.push(val.terminal_link);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.cluster_descr_id.to_le_bytes());
        data.extend_from_slice(&val.formats.to_le_bytes());
        data.push(val.sub_slot_size);
        data.push(val.bit_resolution);
        data.extend_from_slice(&val.aux_protocols.to_le_bytes());
        data.push(val.control_size);
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum LockDelayUnits {
    Undefined,
    Milliseconds,
    DecodedPcmSamples,
}

impl From<u8> for LockDelayUnits {
    fn from(b: u8) -> Self {
        match b {
            0 => LockDelayUnits::Undefined,
            1 => LockDelayUnits::Milliseconds,
            2 => LockDelayUnits::DecodedPcmSamples,
            _ => LockDelayUnits::Undefined,
        }
    }
}

impl fmt::Display for LockDelayUnits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LockDelayUnits::Undefined => write!(f, "Undefined"),
            LockDelayUnits::Milliseconds => write!(f, "Milliseconds"),
            LockDelayUnits::DecodedPcmSamples => write!(f, "Decoded PCM samples"),
        }
    }
}

/// Isochronous Audio Data Stream Endpoint for UAC1
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DataStreamingEndpoint1 {
    pub attributes: u8,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl DataStreamingEndpoint1 {
    const EXPECTED_LENGTH: usize = 4;

    /// Get the expected length of the descriptor
    pub fn size() -> usize {
        Self::EXPECTED_LENGTH
    }

    /// Get the lock delay units
    pub fn lock_delay_units(&self) -> LockDelayUnits {
        self.lock_delay_units.into()
    }
}

impl TryFrom<&[u8]> for DataStreamingEndpoint1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < Self::size() {
            return Err(Error::new_descriptor_len(
                "DataStreamingEndpoint1",
                Self::size(),
                value.len(),
            ));
        }

        Ok(DataStreamingEndpoint1 {
            attributes: value[0],
            lock_delay_units: value[1],
            lock_delay: u16::from_le_bytes([value[2], value[3]]),
        })
    }
}

impl From<DataStreamingEndpoint1> for Vec<u8> {
    fn from(val: DataStreamingEndpoint1) -> Self {
        vec![
            val.attributes,
            val.lock_delay_units,
            val.lock_delay.to_le_bytes()[0],
            val.lock_delay.to_le_bytes()[1],
        ]
    }
}

/// Isochronous Audio Data Stream Endpoint for UAC2
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DataStreamingEndpoint2 {
    pub attributes: u8,
    pub controls: u8,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl DataStreamingEndpoint2 {
    const EXPECTED_LENGTH: usize = 5;

    /// Get the expected length of the descriptor
    pub fn size() -> usize {
        Self::EXPECTED_LENGTH
    }

    /// Get the lock delay units
    pub fn lock_delay_units(&self) -> LockDelayUnits {
        self.lock_delay_units.into()
    }
}

impl TryFrom<&[u8]> for DataStreamingEndpoint2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < Self::size() {
            return Err(Error::new_descriptor_len(
                "DataStreamingEndpoint2",
                Self::size(),
                value.len(),
            ));
        }

        Ok(DataStreamingEndpoint2 {
            attributes: value[0],
            controls: value[1],
            lock_delay_units: value[2],
            lock_delay: u16::from_le_bytes([value[3], value[4]]),
        })
    }
}

impl From<DataStreamingEndpoint2> for Vec<u8> {
    fn from(val: DataStreamingEndpoint2) -> Self {
        vec![
            val.attributes,
            val.controls,
            val.lock_delay_units,
            val.lock_delay.to_le_bytes()[0],
            val.lock_delay.to_le_bytes()[1],
        ]
    }
}

/// Isochronous Audio Data Stream Endpoint for UAC3
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DataStreamingEndpoint3 {
    pub controls: u32,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl DataStreamingEndpoint3 {
    const EXPECTED_LENGTH: usize = 7;

    /// Get the expected length of the descriptor
    pub fn size() -> usize {
        Self::EXPECTED_LENGTH
    }

    /// Get the lock delay units
    pub fn lock_delay_units(&self) -> LockDelayUnits {
        self.lock_delay_units.into()
    }
}

impl TryFrom<&[u8]> for DataStreamingEndpoint3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < Self::size() {
            return Err(Error::new_descriptor_len(
                "DataStreamingEndpoint3",
                Self::size(),
                value.len(),
            ));
        }

        Ok(DataStreamingEndpoint3 {
            controls: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
            lock_delay_units: value[4],
            lock_delay: u16::from_le_bytes([value[5], value[6]]),
        })
    }
}

impl From<DataStreamingEndpoint3> for Vec<u8> {
    fn from(val: DataStreamingEndpoint3) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.push(val.lock_delay_units);
        data.extend_from_slice(&val.lock_delay.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SelectorUnit1 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub selector_index: u8,
    pub selector: Option<String>,
}

impl TryFrom<&[u8]> for SelectorUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("SelectorUnit1", 4, value.len()));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 3 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "SelectorUnit1 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(SelectorUnit1 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids,
            selector_index: value[expected_length - 1],
            selector: None,
        })
    }
}

impl From<SelectorUnit1> for Vec<u8> {
    fn from(val: SelectorUnit1) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.push(val.selector_index);
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SelectorUnit2 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub controls: u8,
    pub selector_index: u8,
    pub selector: Option<String>,
}

impl TryFrom<&[u8]> for SelectorUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new_descriptor_len("SelectorUnit2", 5, value.len()));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 4 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "SelectorUnit2 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(SelectorUnit2 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids,
            controls: value[2 + nr_in_pins],
            selector_index: value[expected_length - 1],
            selector: None,
        })
    }
}

impl From<SelectorUnit2> for Vec<u8> {
    fn from(val: SelectorUnit2) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.push(val.controls);
        data.push(val.selector_index);
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SelectorUnit3 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub controls: u32,
    pub selector_descr_str: u16,
}

impl TryFrom<&[u8]> for SelectorUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new_descriptor_len("SelectorUnit3", 8, value.len()));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 6 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "SelectorUnit3 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();
        let controls = u32::from_le_bytes([
            value[2 + nr_in_pins],
            value[3 + nr_in_pins],
            value[4 + nr_in_pins],
            value[5 + nr_in_pins],
        ]);

        Ok(SelectorUnit3 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids,
            controls,
            selector_descr_str: u16::from_le_bytes([
                value[expected_length - 2],
                value[expected_length - 1],
            ]),
        })
    }
}

impl From<SelectorUnit3> for Vec<u8> {
    fn from(val: SelectorUnit3) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.selector_descr_str.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum AudioProcessingUnitType {
    Undefined,
    UpDownMix,
    DolbyPrologic,
    StereoExtender3d,
    StereoExtender,
    Reverberation,
    Chorus,
    DynRangeComp,
    MultiFunction,
}

impl From<(UacProtocol, u16)> for AudioProcessingUnitType {
    fn from((protocol, b): (UacProtocol, u16)) -> Self {
        match protocol {
            UacProtocol::Uac1 => match b {
                0 => AudioProcessingUnitType::Undefined,
                1 => AudioProcessingUnitType::UpDownMix,
                2 => AudioProcessingUnitType::DolbyPrologic,
                3 => AudioProcessingUnitType::StereoExtender3d,
                4 => AudioProcessingUnitType::Reverberation,
                5 => AudioProcessingUnitType::Chorus,
                6 => AudioProcessingUnitType::DynRangeComp,
                _ => AudioProcessingUnitType::Undefined,
            },
            UacProtocol::Uac2 => match b {
                0 => AudioProcessingUnitType::Undefined,
                1 => AudioProcessingUnitType::UpDownMix,
                2 => AudioProcessingUnitType::DolbyPrologic,
                3 => AudioProcessingUnitType::StereoExtender,
                _ => AudioProcessingUnitType::Undefined,
            },
            UacProtocol::Uac3 => match b {
                0 => AudioProcessingUnitType::Undefined,
                1 => AudioProcessingUnitType::UpDownMix,
                2 => AudioProcessingUnitType::StereoExtender,
                3 => AudioProcessingUnitType::MultiFunction,
                _ => AudioProcessingUnitType::Undefined,
            },
            _ => AudioProcessingUnitType::Undefined,
        }
    }
}

impl fmt::Display for AudioProcessingUnitType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AudioProcessingUnitType::Undefined => write!(f, "Undefined"),
            AudioProcessingUnitType::UpDownMix => write!(f, "Up/Down-mix"),
            AudioProcessingUnitType::DolbyPrologic => write!(f, "Dolby Prologic"),
            AudioProcessingUnitType::StereoExtender3d => write!(f, "3D Stereo Extender"),
            AudioProcessingUnitType::StereoExtender => write!(f, "Stereo Extender"),
            AudioProcessingUnitType::Reverberation => write!(f, "Reverberation"),
            AudioProcessingUnitType::Chorus => write!(f, "Chorus"),
            AudioProcessingUnitType::DynRangeComp => write!(f, "Dyn Range Comp"),
            AudioProcessingUnitType::MultiFunction => write!(f, "Multi-Function"),
        }
    }
}

/// UAC1: Up/Down-mix and Dolby Prologic proc unit descriptor extensions Table 4-9, Table 4-10.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioProcessingUnitExtended1 {
    pub nr_modes: u8,
    pub modes: Vec<u16>,
}

impl TryFrom<&[u8]> for AudioProcessingUnitExtended1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 3 {
            return Err(Error::new_descriptor_len(
                "AudioProcessingUnitExtended1",
                3,
                value.len(),
            ));
        }

        let nr_modes = value[0];
        let modes = (1..value.len())
            .step_by(2)
            .map(|i| u16::from_le_bytes([value[i], value[i + 1]]))
            .collect();

        Ok(AudioProcessingUnitExtended1 { nr_modes, modes })
    }
}

impl From<AudioProcessingUnitExtended1> for Vec<u8> {
    fn from(val: AudioProcessingUnitExtended1) -> Self {
        let mut data = Vec::new();
        data.push(val.nr_modes);
        for mode in val.modes {
            data.extend_from_slice(&mode.to_le_bytes());
        }
        data
    }
}

/// UAC1: 4.3.2.6 Processing Unit Descriptor; Table 4-8.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ProcessingUnit1 {
    pub unit_id: u8,
    pub process_type: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u16,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
    pub control_size: u8,
    pub controls: Vec<u8>,
    pub processing_index: u8,
    pub processing: Option<String>,
    pub specific: Option<AudioProcessingUnitExtended1>,
}

impl TryFrom<&[u8]> for ProcessingUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new_descriptor_len(
                "ProcessingUnit1",
                10,
                value.len(),
            ));
        }

        let nr_in_pins = value[3];
        let control_size = value[9 + nr_in_pins as usize];
        let expected_length = 10 + nr_in_pins as usize + control_size as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "ProcessingUnit1 descriptor too short for the number of pins and controls",
            ));
        }

        let specific = match value[1] {
            1 | 2 => Some(AudioProcessingUnitExtended1::try_from(
                &value[expected_length..],
            )?),
            _ => None,
        };

        Ok(ProcessingUnit1 {
            unit_id: value[0],
            process_type: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins,
            source_ids: value[4..4 + nr_in_pins as usize].to_vec(),
            nr_channels: value[4 + nr_in_pins as usize],
            channel_config: u16::from_le_bytes([
                value[5 + nr_in_pins as usize],
                value[6 + nr_in_pins as usize],
            ]),
            channel_names_index: value[7 + nr_in_pins as usize],
            channel_names: None,
            control_size,
            controls: value
                [10 + nr_in_pins as usize..10 + nr_in_pins as usize + control_size as usize]
                .to_vec(),
            processing_index: value[expected_length - 1],
            processing: None,
            specific,
        })
    }
}

impl From<ProcessingUnit1> for Vec<u8> {
    fn from(val: ProcessingUnit1) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.extend_from_slice(&val.process_type.to_le_bytes());
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names_index);
        data.push(val.control_size);
        data.extend_from_slice(&val.controls);
        if let Some(specific) = val.specific {
            let specific_data: Vec<u8> = specific.into();
            data.extend_from_slice(&specific_data);
        }
        data.push(val.processing_index);
        data
    }
}

impl ProcessingUnit1 {
    /// Returns the [`AudioProcessingUnitType`] of the processing unit.
    pub fn processing_type(&self) -> AudioProcessingUnitType {
        (UacProtocol::Uac1, self.process_type).into()
    }
}

/// UAC2: 4.7.2.11.1 Up/Down-mix Processing Unit Descriptor; Table 4-21.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioProcessingUnit2UpDownMix {
    pub nr_modes: u8,
    pub modes: Vec<u32>,
}

impl TryFrom<&[u8]> for AudioProcessingUnit2UpDownMix {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new_descriptor_len(
                "AudioProcessingUnit2UpDownMix",
                5,
                value.len(),
            ));
        }

        let nr_modes = value[0];
        let modes = (1..value.len())
            .step_by(4)
            .map(|i| u32::from_le_bytes([value[i], value[i + 1], value[i + 2], value[i + 3]]))
            .collect();

        Ok(AudioProcessingUnit2UpDownMix { nr_modes, modes })
    }
}

impl From<AudioProcessingUnit2UpDownMix> for Vec<u8> {
    fn from(val: AudioProcessingUnit2UpDownMix) -> Self {
        let mut data = Vec::new();
        data.push(val.nr_modes);
        for mode in val.modes {
            data.extend_from_slice(&mode.to_le_bytes());
        }
        data
    }
}

/// UAC2: 4.7.2.11.2 Dolby prologic Processing Unit Descriptor; Table 4-22.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioProcessingUnit2DolbyPrologic {
    pub nr_modes: u8,
    pub modes: Vec<u32>,
}

impl TryFrom<&[u8]> for AudioProcessingUnit2DolbyPrologic {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new_descriptor_len(
                "AudioProcessingUnit2DolbyPrologic",
                5,
                value.len(),
            ));
        }

        let nr_modes = value[0];
        let modes = (1..value.len())
            .step_by(4)
            .map(|i| u32::from_le_bytes([value[i], value[i + 1], value[i + 2], value[i + 3]]))
            .collect();

        Ok(AudioProcessingUnit2DolbyPrologic { nr_modes, modes })
    }
}

impl From<AudioProcessingUnit2DolbyPrologic> for Vec<u8> {
    fn from(val: AudioProcessingUnit2DolbyPrologic) -> Self {
        let mut data = Vec::new();
        data.push(val.nr_modes);
        for mode in val.modes {
            data.extend_from_slice(&mode.to_le_bytes());
        }
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum AudioProcessingUnit2Specific {
    UpDownMix(AudioProcessingUnit2UpDownMix),
    DolbyPrologic(AudioProcessingUnit2DolbyPrologic),
}

impl From<AudioProcessingUnit2Specific> for Vec<u8> {
    fn from(val: AudioProcessingUnit2Specific) -> Self {
        match val {
            AudioProcessingUnit2Specific::UpDownMix(up_down_mix) => up_down_mix.into(),
            AudioProcessingUnit2Specific::DolbyPrologic(dolby_prologic) => dolby_prologic.into(),
        }
    }
}

/// UAC3: 4.5.2.10.1 Up/Down-mix Processing Unit Descriptor; Table 4-39.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioProcessingUnit3UpDownMix {
    pub controls: u32,
    pub nr_modes: u8,
    pub cluster_descr_ids: Vec<u16>,
}

impl TryFrom<&[u8]> for AudioProcessingUnit3UpDownMix {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len(
                "AudioProcessingUnit3UpDownMix",
                6,
                value.len(),
            ));
        }

        let nr_modes = value[4];
        let cluster_descr_ids = (5..value.len())
            .step_by(2)
            .map(|i| u16::from_le_bytes([value[i], value[i + 1]]))
            .collect();

        Ok(AudioProcessingUnit3UpDownMix {
            controls: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
            nr_modes,
            cluster_descr_ids,
        })
    }
}

impl From<AudioProcessingUnit3UpDownMix> for Vec<u8> {
    fn from(val: AudioProcessingUnit3UpDownMix) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.push(val.nr_modes);
        for cluster_descr_id in val.cluster_descr_ids {
            data.extend_from_slice(&cluster_descr_id.to_le_bytes());
        }
        data
    }
}

/// UAC3: 4.5.2.10.2 Stereo Extender Processing Unit Descriptor; Table 4-40.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioProcessingUnit3StereoExtender {
    pub controls: u32,
}

impl TryFrom<&[u8]> for AudioProcessingUnit3StereoExtender {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len(
                "AudioProcessingUnit3StereoExtender",
                4,
                value.len(),
            ));
        }

        Ok(AudioProcessingUnit3StereoExtender {
            controls: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
        })
    }
}

impl From<AudioProcessingUnit3StereoExtender> for Vec<u8> {
    fn from(val: AudioProcessingUnit3StereoExtender) -> Self {
        val.controls.to_le_bytes().to_vec()
    }
}

/// UAC3: 4.5.2.10.3 Multi Function Processing Unit Descriptor; Table 4-41.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioProcessingUnit3MultiFunction {
    pub controls: u32,
    pub cluster_descr_id: u16,
    pub algorithms: u32,
}

impl TryFrom<&[u8]> for AudioProcessingUnit3MultiFunction {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new_descriptor_len(
                "AudioProcessingUnit3MultiFunction",
                8,
                value.len(),
            ));
        }

        Ok(AudioProcessingUnit3MultiFunction {
            controls: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
            cluster_descr_id: u16::from_le_bytes([value[4], value[5]]),
            algorithms: u32::from_le_bytes([value[6], value[7], value[8], value[9]]),
        })
    }
}

impl From<AudioProcessingUnit3MultiFunction> for Vec<u8> {
    fn from(val: AudioProcessingUnit3MultiFunction) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.cluster_descr_id.to_le_bytes());
        data.extend_from_slice(&val.algorithms.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum AudioProcessingUnit3Specific {
    UpDownMix(AudioProcessingUnit3UpDownMix),
    StereoExtender(AudioProcessingUnit3StereoExtender),
    MultiFunction(AudioProcessingUnit3MultiFunction),
}

impl From<AudioProcessingUnit3Specific> for Vec<u8> {
    fn from(val: AudioProcessingUnit3Specific) -> Self {
        match val {
            AudioProcessingUnit3Specific::UpDownMix(up_down_mix) => up_down_mix.into(),
            AudioProcessingUnit3Specific::StereoExtender(stereo_extender) => stereo_extender.into(),
            AudioProcessingUnit3Specific::MultiFunction(multi_function) => multi_function.into(),
        }
    }
}

/// UAC2: 4.7.2.11 Processing Unit Descriptor; Table 4-20.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ProcessingUnit2 {
    pub unit_id: u8,
    pub process_type: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
    pub controls: u16,
    pub processing_index: u8,
    pub processing: Option<String>,
    pub specific: Option<AudioProcessingUnit2Specific>,
}

impl TryFrom<&[u8]> for ProcessingUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 12 {
            return Err(Error::new_descriptor_len(
                "ProcessingUnit2",
                12,
                value.len(),
            ));
        }

        let nr_in_pins = value[3];
        let expected_length = 12 + nr_in_pins as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "ProcessingUnit2 descriptor too short for the number of pins and controls",
            ));
        }

        let specific = match value[1] {
            1 => Some(AudioProcessingUnit2Specific::UpDownMix(
                AudioProcessingUnit2UpDownMix::try_from(&value[expected_length..])?,
            )),
            2 => Some(AudioProcessingUnit2Specific::DolbyPrologic(
                AudioProcessingUnit2DolbyPrologic::try_from(&value[expected_length..])?,
            )),
            _ => None,
        };

        Ok(ProcessingUnit2 {
            unit_id: value[0],
            process_type: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins,
            source_ids: value[4..4 + nr_in_pins as usize].to_vec(),
            nr_channels: value[4 + nr_in_pins as usize],
            channel_config: u32::from_le_bytes([
                value[5 + nr_in_pins as usize],
                value[6 + nr_in_pins as usize],
                value[7 + nr_in_pins as usize],
                value[8 + nr_in_pins as usize],
            ]),
            channel_names_index: value[9 + nr_in_pins as usize],
            channel_names: None,
            controls: u16::from_le_bytes([
                value[10 + nr_in_pins as usize],
                value[11 + nr_in_pins as usize],
            ]),
            processing_index: value[expected_length - 1],
            processing: None,
            specific,
        })
    }
}

impl From<ProcessingUnit2> for Vec<u8> {
    fn from(val: ProcessingUnit2) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.extend_from_slice(&val.process_type.to_le_bytes());
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names_index);
        data.extend_from_slice(&val.controls.to_le_bytes());
        if let Some(specific) = val.specific {
            let specific_data: Vec<u8> = specific.into();
            data.extend_from_slice(&specific_data);
        }
        data.push(val.processing_index);
        data
    }
}

impl ProcessingUnit2 {
    /// Returns the [`AudioProcessingUnitType`] of the processing unit.
    pub fn processing_type(&self) -> AudioProcessingUnitType {
        (UacProtocol::Uac2, self.process_type).into()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum AudioProcessingMultiFunction {
    AlgorithmUndefined,
    BeamForming,
    AcousticEchoCancellation,
    ActiveNoiseCancellation,
    BlindSourceSeparation,
    NoiseSuppression,
}

impl std::fmt::Display for AudioProcessingMultiFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            match self {
                AudioProcessingMultiFunction::AlgorithmUndefined => {
                    write!(f, "Algorithm Undefined.")
                }
                AudioProcessingMultiFunction::BeamForming => write!(f, "Beam Forming."),
                AudioProcessingMultiFunction::AcousticEchoCancellation => {
                    write!(f, "Acoustic Echo Cancellation.")
                }
                AudioProcessingMultiFunction::ActiveNoiseCancellation => {
                    write!(f, "Active Noise Cancellation.")
                }
                AudioProcessingMultiFunction::BlindSourceSeparation => {
                    write!(f, "Blind Source Separation.")
                }
                AudioProcessingMultiFunction::NoiseSuppression => {
                    write!(f, "Noise Suppression/Reduction.")
                }
            }
        } else {
            match self {
                AudioProcessingMultiFunction::AlgorithmUndefined => {
                    write!(f, "Algorithm Undefined")
                }
                AudioProcessingMultiFunction::BeamForming => write!(f, "Beam Forming"),
                AudioProcessingMultiFunction::AcousticEchoCancellation => {
                    write!(f, "Acoustic Echo Cancellation")
                }
                AudioProcessingMultiFunction::ActiveNoiseCancellation => {
                    write!(f, "Active Noise Cancellation")
                }
                AudioProcessingMultiFunction::BlindSourceSeparation => {
                    write!(f, "Blind Source Separation")
                }
                AudioProcessingMultiFunction::NoiseSuppression => {
                    write!(f, "Noise Suppression/Reduction")
                }
            }
        }
    }
}

impl AudioProcessingMultiFunction {
    /// Returns the [`AudioProcessingMultiFunction`]s supported from the bitmap value
    pub fn functions_from_bitmap(bitmap: u32) -> Vec<AudioProcessingMultiFunction> {
        let mut functions = Vec::new();
        if bitmap & 0x01 != 0 {
            functions.push(AudioProcessingMultiFunction::AlgorithmUndefined);
        }
        if bitmap & 0x02 != 0 {
            functions.push(AudioProcessingMultiFunction::BeamForming);
        }
        if bitmap & 0x04 != 0 {
            functions.push(AudioProcessingMultiFunction::AcousticEchoCancellation);
        }
        if bitmap & 0x08 != 0 {
            functions.push(AudioProcessingMultiFunction::ActiveNoiseCancellation);
        }
        if bitmap & 0x10 != 0 {
            functions.push(AudioProcessingMultiFunction::BlindSourceSeparation);
        }
        if bitmap & 0x20 != 0 {
            functions.push(AudioProcessingMultiFunction::NoiseSuppression);
        }
        functions
    }
}

/// UAC3: 4.5.2.10 Processing Unit Descriptor; Table 4-38.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ProcessingUnit3 {
    pub unit_id: u8,
    pub process_type: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub processing_descr_str: u16,
    pub specific: Option<AudioProcessingUnit3Specific>,
}

impl TryFrom<&[u8]> for ProcessingUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new_descriptor_len("ProcessingUnit3", 7, value.len()));
        }

        let nr_in_pins = value[3];
        let expected_length = 7 + nr_in_pins as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "ProcessingUnit3 descriptor too short for the number of pins and controls",
            ));
        }

        let specific = match value[1] {
            1 => Some(AudioProcessingUnit3Specific::UpDownMix(
                AudioProcessingUnit3UpDownMix::try_from(&value[expected_length..])?,
            )),
            2 => Some(AudioProcessingUnit3Specific::StereoExtender(
                AudioProcessingUnit3StereoExtender::try_from(&value[expected_length..])?,
            )),
            3 => Some(AudioProcessingUnit3Specific::MultiFunction(
                AudioProcessingUnit3MultiFunction::try_from(&value[expected_length..])?,
            )),
            _ => None,
        };

        Ok(ProcessingUnit3 {
            unit_id: value[0],
            process_type: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins,
            source_ids: value[4..4 + nr_in_pins as usize].to_vec(),
            processing_descr_str: u16::from_le_bytes([
                value[5 + nr_in_pins as usize],
                value[6 + nr_in_pins as usize],
            ]),
            specific,
        })
    }
}

impl From<ProcessingUnit3> for Vec<u8> {
    fn from(val: ProcessingUnit3) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.extend_from_slice(&val.process_type.to_le_bytes());
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.extend_from_slice(&val.processing_descr_str.to_le_bytes());
        if let Some(specific) = val.specific {
            let specific_data: Vec<u8> = specific.into();
            data.extend_from_slice(&specific_data);
        }
        data
    }
}

impl ProcessingUnit3 {
    /// Returns the [`AudioProcessingUnitType`] of the processing unit.
    pub fn processing_type(&self) -> AudioProcessingUnitType {
        (UacProtocol::Uac3, self.process_type).into()
    }

    /// Returns the [`AudioProcessingMultiFunction`] supported by the processing unit.
    pub fn algorithms(&self) -> Option<Vec<AudioProcessingMultiFunction>> {
        match &self.specific {
            Some(AudioProcessingUnit3Specific::MultiFunction(
                AudioProcessingUnit3MultiFunction { algorithms, .. },
            )) => Some(AudioProcessingMultiFunction::functions_from_bitmap(
                *algorithms,
            )),
            _ => None,
        }
    }
}

/// UAC2: 4.7.2.10 Effect Unit Descriptor; Table 4-15.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct EffectUnit2 {
    pub unit_id: u8,
    pub effect_type: u16,
    pub source_id: u8,
    pub controls: Vec<u32>,
    pub effect_index: u8,
    pub effect: Option<String>,
}

impl TryFrom<&[u8]> for EffectUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len("EffectUnit2", 6, value.len()));
        }

        let controls = (4..value.len() - 1)
            .step_by(4)
            .map(|i| u32::from_le_bytes([value[i], value[i + 1], value[i + 2], value[i + 3]]))
            .collect();

        Ok(EffectUnit2 {
            unit_id: value[0],
            effect_type: u16::from_le_bytes([value[1], value[2]]),
            source_id: value[3],
            controls,
            effect_index: value[value.len() - 1],
            effect: None,
        })
    }
}

impl From<EffectUnit2> for Vec<u8> {
    fn from(val: EffectUnit2) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.extend_from_slice(&val.effect_type.to_le_bytes());
        data.push(val.source_id);
        for control in val.controls {
            data.extend_from_slice(&control.to_le_bytes());
        }
        data.push(val.effect_index);
        data
    }
}

/// UAC3: 4.5.2.9 Effect Unit Descriptor; Table 4-33.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct EffectUnit3 {
    pub unit_id: u8,
    pub effect_type: u16,
    pub source_id: u8,
    pub controls: Vec<u32>,
    pub effect_descr_str: u16,
}

impl TryFrom<&[u8]> for EffectUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new_descriptor_len("EffectUnit3", 7, value.len()));
        }

        let controls = (4..value.len() - 2)
            .step_by(4)
            .map(|i| u32::from_le_bytes([value[i], value[i + 1], value[i + 2], value[i + 3]]))
            .collect();

        Ok(EffectUnit3 {
            unit_id: value[0],
            effect_type: u16::from_le_bytes([value[1], value[2]]),
            source_id: value[3],
            controls,
            effect_descr_str: u16::from_le_bytes([value[value.len() - 2], value[value.len() - 1]]),
        })
    }
}

impl From<EffectUnit3> for Vec<u8> {
    fn from(val: EffectUnit3) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.extend_from_slice(&val.effect_type.to_le_bytes());
        data.push(val.source_id);
        for control in val.controls {
            data.extend_from_slice(&control.to_le_bytes());
        }
        data.extend_from_slice(&val.effect_descr_str.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.5 Feature Unit Descriptor; Table 4-7.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FeatureUnit1 {
    pub unit_id: u8,
    pub source_id: u8,
    pub control_size: u8,
    pub controls: Vec<u8>,
    pub feature_index: u8,
    pub feature: Option<String>,
}

impl TryFrom<&[u8]> for FeatureUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("FeatureUnit1", 4, value.len()));
        }

        let control_size = value[2];
        let expected_length = 4 + control_size as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "Audio Feature Unit 1 descriptor too short for the number of controls",
            ));
        }

        let controls = value[3..(3 + control_size as usize)].to_vec();

        Ok(FeatureUnit1 {
            unit_id: value[0],
            source_id: value[1],
            control_size,
            controls,
            feature_index: value[expected_length - 1],
            feature: None,
        })
    }
}

impl From<FeatureUnit1> for Vec<u8> {
    fn from(val: FeatureUnit1) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.source_id);
        data.push(val.control_size);
        data.extend_from_slice(&val.controls);
        data.push(val.feature_index);
        data
    }
}

/// UAC2: 4.7.2.8 Feature Unit Descriptor; Table 4-13.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FeatureUnit2 {
    pub unit_id: u8,
    pub source_id: u8,
    pub controls: [u8; 4],
    pub feature_index: u8,
    pub feature: Option<String>,
}

impl TryFrom<&[u8]> for FeatureUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new_descriptor_len("FeatureUnit2", 7, value.len()));
        }

        Ok(FeatureUnit2 {
            unit_id: value[0],
            source_id: value[1],
            controls: value[2..6].try_into().unwrap(),
            feature_index: value[7],
            feature: None,
        })
    }
}

impl From<FeatureUnit2> for Vec<u8> {
    fn from(val: FeatureUnit2) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.source_id);
        data.extend_from_slice(&val.controls);
        data.push(val.feature_index);
        data
    }
}

/// UAC3: 4.5.2.7 Feature Unit Descriptor; Table 4-31.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FeatureUnit3 {
    pub unit_id: u8,
    pub source_id: u8,
    pub controls: [u8; 4],
    pub feature_descr_str: u16,
}

impl TryFrom<&[u8]> for FeatureUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new_descriptor_len("FeatureUnit3", 8, value.len()));
        }

        Ok(FeatureUnit3 {
            unit_id: value[0],
            source_id: value[1],
            controls: value[2..6].try_into().unwrap(),
            feature_descr_str: u16::from_le_bytes([value[6], value[7]]),
        })
    }
}

impl From<FeatureUnit3> for Vec<u8> {
    fn from(val: FeatureUnit3) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.push(val.source_id);
        data.extend_from_slice(&val.controls);
        data.extend_from_slice(&val.feature_descr_str.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.7 Extension Unit Descriptor; Table 4-15.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ExtensionUnit1 {
    pub unit_id: u8,
    pub extension_code: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u16,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
    pub control_size: u8,
    pub controls: Vec<u8>,
    pub extension_index: u8,
    pub extension: Option<String>,
}

impl TryFrom<&[u8]> for ExtensionUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new_descriptor_len("ExtensionUnit1", 10, value.len()));
        }

        let nr_in_pins = value[3] as usize;
        let control_size = value[8 + nr_in_pins];
        let expected_length = 10 + nr_in_pins + control_size as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "ExtensionUnit1 descriptor too short for the number of pins and controls",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();
        let controls = value[(9 + nr_in_pins)..(9 + nr_in_pins + control_size as usize)].to_vec();

        Ok(ExtensionUnit1 {
            unit_id: value[0],
            extension_code: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins: value[3],
            source_ids,
            nr_channels: value[4 + nr_in_pins],
            channel_config: u16::from_le_bytes([value[5 + nr_in_pins], value[6 + nr_in_pins]]),
            channel_names_index: value[7 + nr_in_pins],
            channel_names: None,
            control_size,
            controls,
            extension_index: value[expected_length - 1],
            extension: None,
        })
    }
}

impl From<ExtensionUnit1> for Vec<u8> {
    fn from(val: ExtensionUnit1) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.extend_from_slice(&val.extension_code.to_le_bytes());
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names_index);
        data.push(val.control_size);
        data.extend_from_slice(&val.controls);
        data.push(val.extension_index);
        data
    }
}

/// UAC2: 4.7.2.12 Extension Unit Descriptor; Table 4-24.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ExtensionUnit2 {
    pub unit_id: u8,
    pub extension_code: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
    pub controls: u8,
    pub extension_index: u8,
    pub extension: Option<String>,
}

impl TryFrom<&[u8]> for ExtensionUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 11 {
            return Err(Error::new_descriptor_len("ExtensionUnit2", 11, value.len()));
        }

        let nr_in_pins = value[3] as usize;
        let expected_length = 10 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "ExtensionUnit2 descriptor too short for the number of pins and controls",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();

        Ok(ExtensionUnit2 {
            unit_id: value[0],
            extension_code: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins: value[3],
            source_ids,
            nr_channels: value[4 + nr_in_pins],
            channel_config: u32::from_le_bytes([
                value[5 + nr_in_pins],
                value[6 + nr_in_pins],
                value[7 + nr_in_pins],
                value[8 + nr_in_pins],
            ]),
            channel_names_index: value[9 + nr_in_pins],
            channel_names: None,
            controls: value[10 + nr_in_pins],
            extension_index: value[11 + nr_in_pins],
            extension: None,
        })
    }
}

impl From<ExtensionUnit2> for Vec<u8> {
    fn from(val: ExtensionUnit2) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.extend_from_slice(&val.extension_code.to_le_bytes());
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.push(val.nr_channels);
        data.extend_from_slice(&val.channel_config.to_le_bytes());
        data.push(val.channel_names_index);
        data.push(val.controls);
        data.push(val.extension_index);
        data
    }
}

/// UAC3: 4.5.2.11 Extension Unit Descriptor; Table 4-42.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ExtensionUnit3 {
    pub unit_id: u8,
    pub extension_code: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub extension_descr_str: u16,
    pub controls: u32,
    pub cluster_descr_id: u16,
}

impl TryFrom<&[u8]> for ExtensionUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new_descriptor_len("ExtensionUnit3", 10, value.len()));
        }

        let nr_in_pins = value[3] as usize;
        let expected_length = 9 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "ExtensionUnit3 descriptor too short for the number of pins and controls",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();

        Ok(ExtensionUnit3 {
            unit_id: value[0],
            extension_code: u16::from_le_bytes([value[1], value[2]]),
            nr_in_pins: value[3],
            source_ids,
            extension_descr_str: u16::from_le_bytes([value[4 + nr_in_pins], value[5 + nr_in_pins]]),
            controls: u32::from_le_bytes([
                value[6 + nr_in_pins],
                value[7 + nr_in_pins],
                value[8 + nr_in_pins],
                value[9 + nr_in_pins],
            ]),
            cluster_descr_id: u16::from_le_bytes([value[10 + nr_in_pins], value[11 + nr_in_pins]]),
        })
    }
}

impl From<ExtensionUnit3> for Vec<u8> {
    fn from(val: ExtensionUnit3) -> Self {
        let mut data = Vec::new();
        data.push(val.unit_id);
        data.extend_from_slice(&val.extension_code.to_le_bytes());
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.source_ids);
        data.extend_from_slice(&val.extension_descr_str.to_le_bytes());
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.cluster_descr_id.to_le_bytes());
        data
    }
}

/// UAC2: 4.7.2.1 Clock Source Descriptor; Table 4-6.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ClockSource2 {
    pub clock_id: u8,
    pub attributes: u8,
    pub controls: u8,
    pub assoc_terminal: u8,
    pub clock_source_index: u8,
    pub clock_source: Option<String>,
}

impl TryFrom<&[u8]> for ClockSource2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new_descriptor_len("ClockSource2", 5, value.len()));
        }

        Ok(ClockSource2 {
            clock_id: value[0],
            attributes: value[1],
            controls: value[2],
            assoc_terminal: value[3],
            clock_source_index: value[4],
            clock_source: None,
        })
    }
}

impl From<ClockSource2> for Vec<u8> {
    fn from(val: ClockSource2) -> Self {
        vec![
            val.clock_id,
            val.attributes,
            val.controls,
            val.assoc_terminal,
            val.clock_source_index,
        ]
    }
}

/// UAC3: 4.5.2.12 Clock Source Descriptor; Table 4-43.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ClockSource3 {
    pub clock_id: u8,
    pub attributes: u8,
    pub controls: u32,
    pub reference_terminal: u8,
    pub clock_source_str: u16,
}

impl TryFrom<&[u8]> for ClockSource3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new_descriptor_len("ClockSource3", 9, value.len()));
        }

        Ok(ClockSource3 {
            clock_id: value[0],
            attributes: value[1],
            controls: u32::from_le_bytes([value[2], value[3], value[4], value[5]]),
            reference_terminal: value[6],
            clock_source_str: u16::from_le_bytes([value[7], value[8]]),
        })
    }
}

impl From<ClockSource3> for Vec<u8> {
    fn from(val: ClockSource3) -> Self {
        let mut data = Vec::new();
        data.push(val.clock_id);
        data.push(val.attributes);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.push(val.reference_terminal);
        data.extend_from_slice(&val.clock_source_str.to_le_bytes());
        data
    }
}

/// UAC2: 4.7.2.2 Clock Selector Descriptor; Table 4-7.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ClockSelector2 {
    pub clock_id: u8,
    pub nr_in_pins: u8,
    pub csource_ids: Vec<u8>,
    pub controls: u8,
    pub clock_selector_index: u8,
    pub clock_selector: Option<String>,
}

impl TryFrom<&[u8]> for ClockSelector2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len("ClockSelector2", 4, value.len()));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 3 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "ClockSelector2 descriptor too short for the number of pins and controls",
            ));
        }

        let csource_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(ClockSelector2 {
            clock_id: value[0],
            nr_in_pins: value[1],
            csource_ids,
            controls: value[2 + nr_in_pins],
            clock_selector_index: value[expected_length - 1],
            clock_selector: None,
        })
    }
}

impl From<ClockSelector2> for Vec<u8> {
    fn from(val: ClockSelector2) -> Self {
        let mut data = Vec::new();
        data.push(val.clock_id);
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.csource_ids);
        data.push(val.controls);
        data.push(val.clock_selector_index);
        data
    }
}

/// UAC3: 4.5.2.13 Clock Selector Descriptor; Table 4-44.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ClockSelector3 {
    pub clock_id: u8,
    pub nr_in_pins: u8,
    pub csource_ids: Vec<u8>,
    pub controls: u32,
    pub cselector_descr_str: u16,
}

impl TryFrom<&[u8]> for ClockSelector3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len("ClockSelector3", 6, value.len()));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 5 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidDescriptor,
                "ClockSelector3 descriptor too short for the number of pins and controls",
            ));
        }

        let csource_ids = value[2..(2 + nr_in_pins)].to_vec();
        let controls = u32::from_le_bytes([
            value[2 + nr_in_pins],
            value[3 + nr_in_pins],
            value[4 + nr_in_pins],
            value[5 + nr_in_pins],
        ]);

        Ok(ClockSelector3 {
            clock_id: value[0],
            nr_in_pins: value[1],
            csource_ids,
            controls,
            cselector_descr_str: u16::from_le_bytes([
                value[expected_length - 2],
                value[expected_length - 1],
            ]),
        })
    }
}

impl From<ClockSelector3> for Vec<u8> {
    fn from(val: ClockSelector3) -> Self {
        let mut data = Vec::new();
        data.push(val.clock_id);
        data.push(val.nr_in_pins);
        data.extend_from_slice(&val.csource_ids);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.cselector_descr_str.to_le_bytes());
        data
    }
}

/// UAC2: 4.7.2.3 Clock Multiplier Descriptor; Table 4-8.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ClockMultiplier2 {
    pub clock_id: u8,
    pub csource_id: u8,
    pub controls: u8,
    pub clock_multiplier_index: u8,
    pub clock_multiplier: Option<String>,
}

impl TryFrom<&[u8]> for ClockMultiplier2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new_descriptor_len(
                "ClockMultiplier2",
                4,
                value.len(),
            ));
        }

        Ok(ClockMultiplier2 {
            clock_id: value[0],
            csource_id: value[1],
            controls: value[2],
            clock_multiplier_index: value[3],
            clock_multiplier: None,
        })
    }
}

impl From<ClockMultiplier2> for Vec<u8> {
    fn from(val: ClockMultiplier2) -> Self {
        vec![
            val.clock_id,
            val.csource_id,
            val.controls,
            val.clock_multiplier_index,
        ]
    }
}

/// UAC3: 4.5.2.14 Clock Multiplier Descriptor; Table 4-45.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ClockMultiplier3 {
    pub clock_id: u8,
    pub csource_id: u8,
    pub controls: u32,
    pub cmultiplier_descr_str: u16,
}

impl TryFrom<&[u8]> for ClockMultiplier3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new_descriptor_len(
                "ClockMultiplier3",
                8,
                value.len(),
            ));
        }

        Ok(ClockMultiplier3 {
            clock_id: value[0],
            csource_id: value[1],
            controls: u32::from_le_bytes([value[2], value[3], value[4], value[5]]),
            cmultiplier_descr_str: u16::from_le_bytes([value[6], value[7]]),
        })
    }
}

impl From<ClockMultiplier3> for Vec<u8> {
    fn from(val: ClockMultiplier3) -> Self {
        let mut data = Vec::new();
        data.push(val.clock_id);
        data.push(val.csource_id);
        data.extend_from_slice(&val.controls.to_le_bytes());
        data.extend_from_slice(&val.cmultiplier_descr_str.to_le_bytes());
        data
    }
}

/// UAC2: 4.7.2.9 Sampling Rate Converter Descriptor; Table 4-14.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SampleRateConverter2 {
    pub unit_id: u8,
    pub source_id: u8,
    pub csource_in_id: u8,
    pub csource_out_id: u8,
    pub src_index: u8,
    pub src: Option<String>,
}

impl TryFrom<&[u8]> for SampleRateConverter2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new_descriptor_len(
                "SampleRateConverter2",
                5,
                value.len(),
            ));
        }

        Ok(SampleRateConverter2 {
            unit_id: value[0],
            source_id: value[1],
            csource_in_id: value[2],
            csource_out_id: value[3],
            src_index: value[4],
            src: None,
        })
    }
}

impl From<SampleRateConverter2> for Vec<u8> {
    fn from(val: SampleRateConverter2) -> Self {
        vec![
            val.unit_id,
            val.source_id,
            val.csource_in_id,
            val.csource_out_id,
            val.src_index,
        ]
    }
}

/// UAC3: 4.5.2.8 Sampling Rate Converter Descriptor; Table 4-32.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SampleRateConverter3 {
    pub unit_id: u8,
    pub source_id: u8,
    pub csource_in_id: u8,
    pub csource_out_id: u8,
    pub src_descr_str: u16,
}

impl TryFrom<&[u8]> for SampleRateConverter3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new_descriptor_len(
                "SampleRateConverter3",
                6,
                value.len(),
            ));
        }

        Ok(SampleRateConverter3 {
            unit_id: value[0],
            source_id: value[1],
            csource_in_id: value[2],
            csource_out_id: value[3],
            src_descr_str: u16::from_le_bytes([value[4], value[5]]),
        })
    }
}

impl From<SampleRateConverter3> for Vec<u8> {
    fn from(val: SampleRateConverter3) -> Self {
        let mut data = vec![
            val.unit_id,
            val.source_id,
            val.csource_in_id,
            val.csource_out_id,
        ];
        data.extend_from_slice(&val.src_descr_str.to_le_bytes());
        data
    }
}
