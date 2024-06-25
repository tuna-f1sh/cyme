//! Defines for the USB Audio Class (UAC) interface descriptors and MIDI
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

use crate::error::{self, Error, ErrorKind};
use super::*;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
#[non_exhaustive]
pub enum MidiInterface {
    Undefined = 0x00,
    Header = 0x01,
    InputJack = 0x02,
    OutputJack = 0x03,
    Element = 0x04,
}

impl From<u8> for MidiInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => MidiInterface::Undefined,
            0x01 => MidiInterface::Header,
            0x02 => MidiInterface::InputJack,
            0x03 => MidiInterface::OutputJack,
            0x04 => MidiInterface::Element,
            _ => MidiInterface::Undefined,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MidiDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub midi_type: MidiInterface,
    pub string_index: Option<u8>,
    pub string: Option<String>,
    pub data: Vec<u8>,
}

impl TryFrom<&[u8]> for MidiDescriptor {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "MidiDescriptor descriptor too short",
            ));
        }

        let length = value[0];
        if length as usize > value.len() {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "MidiDescriptor descriptor reported length too long for buffer",
            ));
        }

        let midi_type = MidiInterface::from(value[2]);

        let string_index = match midi_type {
            MidiInterface::InputJack => value.get(5).copied(),
            MidiInterface::OutputJack => value.get(5).map(|v| 6 + *v * 2),
            MidiInterface::Element => {
                // don't ask...
                if let Some(j) = value.get(4) {
                    if let Some(capsize) = value.get((5 + *j as usize * 2) + 3) {
                        value.get(9 + 2 * *j as usize + *capsize as usize).copied()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        Ok(MidiDescriptor {
            length,
            descriptor_type: value[1],
            midi_type,
            string_index,
            string: None,
            data: value[3..].to_vec(),
        })
    }
}

impl From<MidiDescriptor> for Vec<u8> {
    fn from(md: MidiDescriptor) -> Self {
        let mut ret = Vec::new();
        ret.push(md.length);
        ret.push(md.descriptor_type);
        ret.push(md.midi_type as u8);
        ret.extend(md.data);

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

/// Base USB Audio Class (UAC) interface descriptor that contains [`UacSubtype`] and [`UacInterfaceDescriptor`]
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UacDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub subtype: UacSubtype,
    pub interface: UacInterfaceDescriptor,
}

impl TryFrom<(GenericDescriptor, u8, u8)> for UacDescriptor {
    type Error = Error;

    fn try_from((gd, subc, p): (GenericDescriptor, u8, u8)) -> error::Result<Self> {
        let length = gd.length;
        let descriptor_type = gd.descriptor_type;
        let subtype: UacSubtype = (subc, gd.descriptor_subtype, p).try_into()?;
        let interface = subtype.uac_descriptor_from_generic(gd, p)?;
        Ok(UacDescriptor {
            length,
            descriptor_type,
            subtype,
            interface,
        })
    }
}

impl Into<Vec<u8>> for UacDescriptor {
    fn into(self) -> Vec<u8> {
        let mut ret: Vec<u8> = Vec::new();
        ret.push(self.length);
        ret.push(self.descriptor_type);
        let subtype: u8 = match self.subtype {
            UacSubtype::Control(aci) => aci as u8,
            UacSubtype::Streaming(asi) => asi as u8,
            UacSubtype::Midi(mi) => mi as u8,
        };
        ret.push(subtype);
        let data: Vec<u8> = self.interface.into();
        ret.extend(&data);

        ret
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
/// Ported from https://github.com/gregkh/usbutils/blob/master/desc-defs.c
///
/// I think there is a much nicer way to define all these for more generic printing; enum types like desc-def.c wrapping the int values so they can be acted on in a more generic way
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum UacInterfaceDescriptor {
    // Audio Controls bSubClass
    AudioHeader1(AudioHeader1),
    AudioHeader2(AudioHeader2),
    AudioHeader3(AudioHeader3),
    AudioInputTerminal1(AudioInputTerminal1),
    AudioInputTerminal2(AudioInputTerminal2),
    AudioInputTerminal3(AudioInputTerminal3),
    AudioOutputTerminal1(AudioOutputTerminal1),
    AudioOutputTerminal2(AudioOutputTerminal2),
    AudioOutputTerminal3(AudioOutputTerminal3),
    ExtendedTerminalHeader(ExtendedTerminalHeader),
    AudioPowerDomain(AudioPowerDomain),
    AudioMixerUnit1(AudioMixerUnit1),
    AudioMixerUnit2(AudioMixerUnit2),
    AudioMixerUnit3(AudioMixerUnit3),
    AudioSelectorUnit1(AudioSelectorUnit1),
    AudioSelectorUnit2(AudioSelectorUnit2),
    AudioSelectorUnit3(AudioSelectorUnit3),
    AudioProcessingUnit1(AudioProcessingUnit1),
    AudioProcessingUnit2(AudioProcessingUnit2),
    AudioProcessingUnit3(AudioProcessingUnit3),
    AudioEffectUnit2(AudioEffectUnit2),
    AudioEffectUnit3(AudioEffectUnit3),
    AudioFeatureUnit1(AudioFeatureUnit1),
    AudioFeatureUnit2(AudioFeatureUnit2),
    AudioFeatureUnit3(AudioFeatureUnit3),
    AudioExtensionUnit1(AudioExtensionUnit1),
    AudioExtensionUnit2(AudioExtensionUnit2),
    AudioExtensionUnit3(AudioExtensionUnit3),
    AudioClockSource2(AudioClockSource2),
    AudioClockSource3(AudioClockSource3),
    AudioClockSelector2(AudioClockSelector2),
    AudioClockSelector3(AudioClockSelector3),
    AudioClockMultiplier2(AudioClockMultiplier2),
    AudioClockMultiplier3(AudioClockMultiplier3),
    AudioSampleRateConverter2(AudioSampleRateConverter2),
    AudioSampleRateConverter3(AudioSampleRateConverter3),
    // Audio Streaming bSubClass
    AudioStreamingInterface1(AudioStreamingInterface1),
    AudioStreamingInterface2(AudioStreamingInterface2),
    AudioStreamingInterface3(AudioStreamingInterface3),
    // Isochronous Audio Data Stream Endpoint
    AudioDataStreamingEndpoint1(AudioDataStreamingEndpoint1),
    AudioDataStreamingEndpoint2(AudioDataStreamingEndpoint2),
    AudioDataStreamingEndpoint3(AudioDataStreamingEndpoint3),
    /// Invalid descriptor for failing to parse matched
    Invalid(Vec<u8>),
    /// Generic descriptor for unsupported descriptors
    Generic(Vec<u8>),
    /// Undefined descriptor
    Undefined(Vec<u8>),
}

impl Into<Vec<u8>> for UacInterfaceDescriptor {
    fn into(self) -> Vec<u8> {
        match self {
            UacInterfaceDescriptor::AudioHeader1(a) => a.into(),
            UacInterfaceDescriptor::AudioHeader2(a) => a.into(),
            UacInterfaceDescriptor::AudioHeader3(a) => a.into(),
            UacInterfaceDescriptor::AudioInputTerminal1(a) => a.into(),
            UacInterfaceDescriptor::AudioInputTerminal2(a) => a.into(),
            UacInterfaceDescriptor::AudioInputTerminal3(a) => a.into(),
            UacInterfaceDescriptor::AudioOutputTerminal1(a) => a.into(),
            UacInterfaceDescriptor::AudioOutputTerminal2(a) => a.into(),
            UacInterfaceDescriptor::AudioOutputTerminal3(a) => a.into(),
            UacInterfaceDescriptor::ExtendedTerminalHeader(a) => a.into(),
            UacInterfaceDescriptor::AudioPowerDomain(a) => a.into(),
            UacInterfaceDescriptor::AudioMixerUnit1(a) => a.into(),
            UacInterfaceDescriptor::AudioMixerUnit2(a) => a.into(),
            UacInterfaceDescriptor::AudioMixerUnit3(a) => a.into(),
            UacInterfaceDescriptor::AudioSelectorUnit1(a) => a.into(),
            UacInterfaceDescriptor::AudioSelectorUnit2(a) => a.into(),
            UacInterfaceDescriptor::AudioSelectorUnit3(a) => a.into(),
            UacInterfaceDescriptor::AudioProcessingUnit1(a) => a.into(),
            UacInterfaceDescriptor::AudioProcessingUnit2(a) => a.into(),
            UacInterfaceDescriptor::AudioProcessingUnit3(a) => a.into(),
            UacInterfaceDescriptor::AudioEffectUnit2(a) => a.into(),
            UacInterfaceDescriptor::AudioEffectUnit3(a) => a.into(),
            UacInterfaceDescriptor::AudioFeatureUnit1(a) => a.into(),
            UacInterfaceDescriptor::AudioFeatureUnit2(a) => a.into(),
            UacInterfaceDescriptor::AudioFeatureUnit3(a) => a.into(),
            UacInterfaceDescriptor::AudioExtensionUnit1(a) => a.into(),
            UacInterfaceDescriptor::AudioExtensionUnit2(a) => a.into(),
            UacInterfaceDescriptor::AudioExtensionUnit3(a) => a.into(),
            UacInterfaceDescriptor::AudioClockSource2(a) => a.into(),
            UacInterfaceDescriptor::AudioClockSource3(a) => a.into(),
            UacInterfaceDescriptor::AudioClockSelector2(a) => a.into(),
            UacInterfaceDescriptor::AudioClockSelector3(a) => a.into(),
            UacInterfaceDescriptor::AudioClockMultiplier2(a) => a.into(),
            UacInterfaceDescriptor::AudioClockMultiplier3(a) => a.into(),
            UacInterfaceDescriptor::AudioSampleRateConverter2(a) => a.into(),
            UacInterfaceDescriptor::AudioSampleRateConverter3(a) => a.into(),
            UacInterfaceDescriptor::AudioStreamingInterface1(a) => a.into(),
            UacInterfaceDescriptor::AudioStreamingInterface2(a) => a.into(),
            UacInterfaceDescriptor::AudioStreamingInterface3(a) => a.into(),
            UacInterfaceDescriptor::AudioDataStreamingEndpoint1(a) => a.into(),
            UacInterfaceDescriptor::AudioDataStreamingEndpoint2(a) => a.into(),
            UacInterfaceDescriptor::AudioDataStreamingEndpoint3(a) => a.into(),
            UacInterfaceDescriptor::Invalid(a) => a,
            UacInterfaceDescriptor::Generic(a) => a,
            UacInterfaceDescriptor::Undefined(a) => a,
        }
    }
}

impl UacInterfaceDescriptor {
    const UAC1_CHANNEL_NAMES: [&'static str; 12] = [
        "Left Front (L)",
        "Right Front (R)",
        "Center Front (C)",
        "Low Frequency Enhancement (LFE)",
        "Left Surround (LS)",
        "Right Surround (RS)",
        "Left of Center (LC)",
        "Right of Center (RC)",
        "Surround (S)",
        "Side Left (SL)",
        "Side Right (SR)",
        "Top (T)",
    ];

    const UAC2_CHANNEL_NAMES: [&'static str; 27] = [
        "Front Left (FL)",
        "Front Right (FR)",
        "Front Center (FC)",
        "Low Frequency Effects (LFE)",
        "Back Left (BL)",
        "Back Right (BR)",
        "Front Left of Center (FLC)",
        "Front Right of Center (FRC)",
        "Back Center (BC)",
        "Side Left (SL)",
        "Side Right (SR)",
        "Top Center (TC)",
        "Top Front Left (TFL)",
        "Top Front Center (TFC)",
        "Top Front Right (TFR)",
        "Top Back Left (TBL)",
        "Top Back Center (TBC)",
        "Top Back Right (TBR)",
        "Top Front Left of Center (TFLC)",
        "Top Front Right of Center (TFRC)",
        "Left Low Frequency Effects (LLFE)",
        "Right Low Frequency Effects (RLFE)",
        "Top Side Left (TSL)",
        "Top Side Right (TSR)",
        "Bottom Center (BC)",
        "Back Left of Center (BLC)",
        "Back Right of Center (BRC)",
    ];

    /// Get the UAC AC interface descriptor from the UAC AC interface
    pub fn from_uac_ac_interface(
        uac_interface: &UacAcInterface,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match uac_interface {
            UacAcInterface::Header => match protocol {
                UacProtocol::Uac1 => {
                    AudioHeader1::try_from(data).map(UacInterfaceDescriptor::AudioHeader1)
                }
                UacProtocol::Uac2 => {
                    AudioHeader2::try_from(data).map(UacInterfaceDescriptor::AudioHeader2)
                }
                UacProtocol::Uac3 => {
                    AudioHeader3::try_from(data).map(UacInterfaceDescriptor::AudioHeader3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::InputTerminal => match protocol {
                UacProtocol::Uac1 => AudioInputTerminal1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioInputTerminal1),
                UacProtocol::Uac2 => AudioInputTerminal2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioInputTerminal2),
                UacProtocol::Uac3 => AudioInputTerminal3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioInputTerminal3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::OutputTerminal => match protocol {
                UacProtocol::Uac1 => AudioOutputTerminal1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioOutputTerminal1),
                UacProtocol::Uac2 => AudioOutputTerminal2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioOutputTerminal2),
                UacProtocol::Uac3 => AudioOutputTerminal3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioOutputTerminal3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::ExtendedTerminal => match protocol {
                UacProtocol::Uac3 => ExtendedTerminalHeader::try_from(data)
                    .map(UacInterfaceDescriptor::ExtendedTerminalHeader),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::PowerDomain => match protocol {
                UacProtocol::Uac3 => {
                    AudioPowerDomain::try_from(data).map(UacInterfaceDescriptor::AudioPowerDomain)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::MixerUnit => match protocol {
                UacProtocol::Uac1 => {
                    AudioMixerUnit1::try_from(data).map(UacInterfaceDescriptor::AudioMixerUnit1)
                }
                UacProtocol::Uac2 => {
                    AudioMixerUnit2::try_from(data).map(UacInterfaceDescriptor::AudioMixerUnit2)
                }
                UacProtocol::Uac3 => {
                    AudioMixerUnit3::try_from(data).map(UacInterfaceDescriptor::AudioMixerUnit3)
                }
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::SelectorUnit => match protocol {
                UacProtocol::Uac1 => AudioSelectorUnit1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSelectorUnit1),
                UacProtocol::Uac2 => AudioSelectorUnit2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSelectorUnit2),
                UacProtocol::Uac3 => AudioSelectorUnit3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSelectorUnit3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::ProcessingUnit => match protocol {
                UacProtocol::Uac1 => AudioProcessingUnit1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioProcessingUnit1),
                UacProtocol::Uac2 => AudioProcessingUnit2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioProcessingUnit2),
                UacProtocol::Uac3 => AudioProcessingUnit3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioProcessingUnit3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::EffectUnit => {
                match protocol {
                    UacProtocol::Uac2 => AudioEffectUnit2::try_from(data)
                        .map(UacInterfaceDescriptor::AudioEffectUnit2),
                    UacProtocol::Uac3 => AudioEffectUnit3::try_from(data)
                        .map(UacInterfaceDescriptor::AudioEffectUnit3),
                    _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
                }
            }
            UacAcInterface::FeatureUnit => {
                match protocol {
                    UacProtocol::Uac1 => AudioFeatureUnit1::try_from(data)
                        .map(UacInterfaceDescriptor::AudioFeatureUnit1),
                    UacProtocol::Uac2 => AudioFeatureUnit2::try_from(data)
                        .map(UacInterfaceDescriptor::AudioFeatureUnit2),
                    UacProtocol::Uac3 => AudioFeatureUnit3::try_from(data)
                        .map(UacInterfaceDescriptor::AudioFeatureUnit3),
                    _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
                }
            }
            UacAcInterface::ExtensionUnit => match protocol {
                UacProtocol::Uac1 => AudioExtensionUnit1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioExtensionUnit1),
                UacProtocol::Uac2 => AudioExtensionUnit2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioExtensionUnit2),
                UacProtocol::Uac3 => AudioExtensionUnit3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioExtensionUnit3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::ClockSource => {
                match protocol {
                    UacProtocol::Uac2 => AudioClockSource2::try_from(data)
                        .map(UacInterfaceDescriptor::AudioClockSource2),
                    UacProtocol::Uac3 => AudioClockSource3::try_from(data)
                        .map(UacInterfaceDescriptor::AudioClockSource3),
                    _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
                }
            }
            UacAcInterface::ClockSelector => match protocol {
                UacProtocol::Uac2 => AudioClockSelector2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioClockSelector2),
                UacProtocol::Uac3 => AudioClockSelector3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioClockSelector3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::ClockMultiplier => match protocol {
                UacProtocol::Uac2 => AudioClockMultiplier2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioClockMultiplier2),
                UacProtocol::Uac3 => AudioClockMultiplier3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioClockMultiplier3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::SampleRateConverter => match protocol {
                UacProtocol::Uac2 => AudioSampleRateConverter2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSampleRateConverter2),
                UacProtocol::Uac3 => AudioSampleRateConverter3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioSampleRateConverter3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAcInterface::Undefined => Ok(UacInterfaceDescriptor::Undefined(data.to_vec())),
            _ => Ok(UacInterfaceDescriptor::Generic(data.to_vec())),
        }
    }

    /// Get the UAC AS interface descriptor from the UAC AS interface
    pub fn from_uac_as_interface(
        uac_interface: &UacAsInterface,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match uac_interface {
            UacAsInterface::General => match protocol {
                UacProtocol::Uac1 => AudioStreamingInterface1::try_from(data)
                    .map(UacInterfaceDescriptor::AudioStreamingInterface1),
                UacProtocol::Uac2 => AudioStreamingInterface2::try_from(data)
                    .map(UacInterfaceDescriptor::AudioStreamingInterface2),
                UacProtocol::Uac3 => AudioStreamingInterface3::try_from(data)
                    .map(UacInterfaceDescriptor::AudioStreamingInterface3),
                _ => Ok(UacInterfaceDescriptor::Invalid(data.to_vec())),
            },
            UacAsInterface::Undefined => Ok(UacInterfaceDescriptor::Undefined(data.to_vec())),
            _ => Ok(UacInterfaceDescriptor::Generic(data.to_vec())),
        }
    }

    /// Get the UAC Audio Data Endpoint descriptor from the UAC AS interface
    pub fn from_uac_as_iso_data_endpoint(
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<Self, Error> {
        match protocol {
            UacProtocol::Uac1 => AudioDataStreamingEndpoint1::try_from(data)
                .map(UacInterfaceDescriptor::AudioDataStreamingEndpoint1),
            UacProtocol::Uac2 => AudioDataStreamingEndpoint2::try_from(data)
                .map(UacInterfaceDescriptor::AudioDataStreamingEndpoint2),
            UacProtocol::Uac3 => AudioDataStreamingEndpoint3::try_from(data)
                .map(UacInterfaceDescriptor::AudioDataStreamingEndpoint3),
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

    /// Get USB Audio Device Class channel names from the descriptor "wChannelConfig" field bitmap string based on the protocol
    pub fn get_channel_names<T: Into<u32> + Copy>(
        protocol: &UacProtocol,
        channel_config: T,
    ) -> Vec<String> {
        match protocol {
            UacProtocol::Uac1 => Self::get_bitmap_string(channel_config, &Self::UAC1_CHANNEL_NAMES),
            UacProtocol::Uac2 => Self::get_bitmap_string(channel_config, &Self::UAC2_CHANNEL_NAMES),
            _ => Vec::new(),
        }
    }

    /// Get the [`LockDelayUnits`] from the descriptor if it has the field
    pub fn get_lock_delay_units(&self) -> Option<LockDelayUnits> {
        match self {
            UacInterfaceDescriptor::AudioDataStreamingEndpoint1(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            UacInterfaceDescriptor::AudioDataStreamingEndpoint2(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            UacInterfaceDescriptor::AudioDataStreamingEndpoint3(ep) => {
                Some(LockDelayUnits::from(ep.lock_delay_units))
            }
            _ => None,
        }
    }

    /// Get the [`UacProtocol`] version for the interface descriptor
    pub fn get_protocol(&self) -> UacProtocol {
        match self {
            UacInterfaceDescriptor::AudioHeader1(_)
            | UacInterfaceDescriptor::AudioInputTerminal1(_)
            | UacInterfaceDescriptor::AudioOutputTerminal1(_)
            | UacInterfaceDescriptor::AudioMixerUnit1(_)
            | UacInterfaceDescriptor::AudioSelectorUnit1(_)
            | UacInterfaceDescriptor::AudioFeatureUnit1(_)
            | UacInterfaceDescriptor::AudioExtensionUnit1(_) => UacProtocol::Uac1,
            UacInterfaceDescriptor::AudioHeader2(_)
            | UacInterfaceDescriptor::AudioInputTerminal2(_)
            | UacInterfaceDescriptor::AudioOutputTerminal2(_)
            | UacInterfaceDescriptor::AudioMixerUnit2(_)
            | UacInterfaceDescriptor::AudioSelectorUnit2(_)
            | UacInterfaceDescriptor::AudioEffectUnit2(_)
            | UacInterfaceDescriptor::AudioFeatureUnit2(_)
            | UacInterfaceDescriptor::AudioExtensionUnit2(_)
            | UacInterfaceDescriptor::AudioClockSource2(_)
            | UacInterfaceDescriptor::AudioClockSelector2(_)
            | UacInterfaceDescriptor::AudioClockMultiplier2(_)
            | UacInterfaceDescriptor::AudioSampleRateConverter2(_)
            | UacInterfaceDescriptor::AudioStreamingInterface2(_)
            | UacInterfaceDescriptor::AudioDataStreamingEndpoint2(_) => UacProtocol::Uac2,
            UacInterfaceDescriptor::AudioHeader3(_)
            | UacInterfaceDescriptor::AudioInputTerminal3(_)
            | UacInterfaceDescriptor::AudioOutputTerminal3(_)
            | UacInterfaceDescriptor::AudioMixerUnit3(_)
            | UacInterfaceDescriptor::AudioSelectorUnit3(_)
            | UacInterfaceDescriptor::AudioEffectUnit3(_)
            | UacInterfaceDescriptor::AudioFeatureUnit3(_)
            | UacInterfaceDescriptor::AudioExtensionUnit3(_)
            | UacInterfaceDescriptor::AudioClockSource3(_)
            | UacInterfaceDescriptor::AudioClockSelector3(_)
            | UacInterfaceDescriptor::AudioClockMultiplier3(_)
            | UacInterfaceDescriptor::AudioSampleRateConverter3(_)
            | UacInterfaceDescriptor::AudioStreamingInterface3(_)
            | UacInterfaceDescriptor::AudioDataStreamingEndpoint3(_)
            | UacInterfaceDescriptor::ExtendedTerminalHeader(_)
            | UacInterfaceDescriptor::AudioPowerDomain(_) => UacProtocol::Uac3,
            _ => UacProtocol::Unknown(0xff),
        }
    }
}

/// USB Audio Class (UAC) protocol byte defines the version of the UAC
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum UacSubtype {
    Control(UacAcInterface),
    Streaming(UacAsInterface),
    Midi(MidiInterface),
}

/// From a [`GenericDescriptor`] and a protocol, get the UAC subtype
impl TryFrom<(&GenericDescriptor, u8)> for UacSubtype {
    type Error = Error;

    fn try_from((gd, p): (&GenericDescriptor, u8)) -> error::Result<Self> {
        (gd.descriptor_type, gd.descriptor_subtype, p).try_into()
    }
}

impl TryFrom<(u8, u8, u8)> for UacSubtype {
    type Error = Error;

    fn try_from((sub_class, descriptor_sub, protocol): (u8, u8, u8)) -> error::Result<Self> {
        match (sub_class, descriptor_sub, protocol) {
            (1, d, p) => Ok(UacSubtype::Control(UacAcInterface::get_uac_subtype(d, p))),
            (2, d, _) => Ok(UacSubtype::Streaming(UacAsInterface::from(d))),
            (3, d, _) => Ok(UacSubtype::Midi(MidiInterface::from(d))),
            _ => Err(Error::new(
                ErrorKind::InvalidArg,
                "Invalid UAC subtype",
            )), 
        }
    }
}

impl From<UacSubtype> for u8 {
    fn from(us: UacSubtype) -> u8 {
        match us {
            UacSubtype::Control(aci) => aci as u8,
            UacSubtype::Streaming(asi) => asi as u8,
            UacSubtype::Midi(mi) => mi as u8,
        }
    }
}

impl fmt::Display for UacSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UacSubtype::Control(aci) => write!(f, "{}", aci),
            UacSubtype::Streaming(asi) => write!(f, "{}", asi),
            UacSubtype::Midi(mi) => write!(f, "{:?}", mi),
        }
    }
}

impl UacSubtype {
    /// Get the [`UacInterfaceDescriptor`] based on UAC subtype, [`UacProtocol`] and raw data
    pub fn get_uac_descriptor(
        &self,
        protocol: u8,
        data: &[u8],
    ) -> Result<UacInterfaceDescriptor, Error> {
        match self {
            UacSubtype::Control(aci) => aci.get_descriptor(&UacProtocol::from(protocol), data),
            UacSubtype::Streaming(asi) => asi.get_descriptor(&UacProtocol::from(protocol), data),
            // TODO decode all MidiInterface types like Control and Streaming
            UacSubtype::Midi(_) => Err(Error::new(
                ErrorKind::InvalidArg,
                "Midi descriptor to UAC not yet supported, use MidiDescriptor.data",
            )),
        }
    }

    /// Get the [`UacAcInterface`] from a generic descriptor
    pub fn uac_descriptor_from_generic(
        &self,
        gd: GenericDescriptor,
        protocol: u8,
    ) -> Result<UacInterfaceDescriptor, Error> {
        match gd.data {
            Some(data) => self.get_uac_descriptor(protocol, &data),
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
pub enum UacAcInterface {
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

impl std::fmt::Display for UacAcInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            // uppercase with _ instead of space for lsusb dump
            match self {
                UacAcInterface::Undefined => write!(f, "unknown"),
                UacAcInterface::Header => write!(f, "HEADER"),
                UacAcInterface::InputTerminal => write!(f, "INPUT_TERMINAL"),
                UacAcInterface::OutputTerminal => write!(f, "OUTPUT_TERMINAL"),
                UacAcInterface::ExtendedTerminal => write!(f, "EXTENDED_TERMINAL"),
                UacAcInterface::MixerUnit => write!(f, "MIXER_UNIT"),
                UacAcInterface::SelectorUnit => write!(f, "SELECTOR_UNIT"),
                UacAcInterface::FeatureUnit => write!(f, "FEATURE_UNIT"),
                UacAcInterface::EffectUnit => write!(f, "EFFECT_UNIT"),
                UacAcInterface::ProcessingUnit => write!(f, "PROCESSING_UNIT"),
                UacAcInterface::ExtensionUnit => write!(f, "EXTENSION_UNIT"),
                UacAcInterface::ClockSource => write!(f, "CLOCK_SOURCE"),
                UacAcInterface::ClockSelector => write!(f, "CLOCK_SELECTOR"),
                UacAcInterface::ClockMultiplier => write!(f, "CLOCK_MULTIPLIER"),
                UacAcInterface::SampleRateConverter => write!(f, "SAMPLE_RATE_CONVERTER"),
                UacAcInterface::Connectors => write!(f, "CONNECTORS"),
                UacAcInterface::PowerDomain => write!(f, "POWER_DOMAIN"),
            }
        } else {
            match self {
                UacAcInterface::Undefined => write!(f, "Undefined"),
                UacAcInterface::Header => write!(f, "Header"),
                UacAcInterface::InputTerminal => write!(f, "Input Terminal"),
                UacAcInterface::OutputTerminal => write!(f, "Output Terminal"),
                UacAcInterface::ExtendedTerminal => write!(f, "Extended Terminal"),
                UacAcInterface::MixerUnit => write!(f, "Mixer Unit"),
                UacAcInterface::SelectorUnit => write!(f, "Selector Unit"),
                UacAcInterface::FeatureUnit => write!(f, "Feature Unit"),
                UacAcInterface::EffectUnit => write!(f, "Effect Unit"),
                UacAcInterface::ProcessingUnit => write!(f, "Processing Unit"),
                UacAcInterface::ExtensionUnit => write!(f, "Extension Unit"),
                UacAcInterface::ClockSource => write!(f, "Clock Source"),
                UacAcInterface::ClockSelector => write!(f, "Clock Selector"),
                UacAcInterface::ClockMultiplier => write!(f, "Clock Multiplier"),
                UacAcInterface::SampleRateConverter => write!(f, "Sample Rate Converter"),
                UacAcInterface::Connectors => write!(f, "Connectors"),
                UacAcInterface::PowerDomain => write!(f, "Power Domain"),
            }
        }
    }
}

impl From<u8> for UacAcInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UacAcInterface::Undefined,
            0x01 => UacAcInterface::Header,
            0x02 => UacAcInterface::InputTerminal,
            0x03 => UacAcInterface::OutputTerminal,
            0x04 => UacAcInterface::ExtendedTerminal,
            0x05 => UacAcInterface::MixerUnit,
            0x06 => UacAcInterface::SelectorUnit,
            0x07 => UacAcInterface::FeatureUnit,
            0x08 => UacAcInterface::EffectUnit,
            0x09 => UacAcInterface::ProcessingUnit,
            0x0a => UacAcInterface::ExtensionUnit,
            0x0b => UacAcInterface::ClockSource,
            0x0c => UacAcInterface::ClockSelector,
            0x0d => UacAcInterface::ClockMultiplier,
            0x0e => UacAcInterface::SampleRateConverter,
            0x0f => UacAcInterface::Connectors,
            0x10 => UacAcInterface::PowerDomain,
            _ => UacAcInterface::Undefined,
        }
    }
}

impl UacAcInterface {
    /// UAC1, UAC2, and UAC3 define bDescriptorSubtype differently for the
    /// AudioControl interface, so we need to do some ugly remapping:
    pub fn get_uac_subtype(subtype: u8, protocol: u8) -> Self {
        match protocol {
            // UAC1
            0x00 => match subtype {
                0x04 => UacAcInterface::MixerUnit,
                0x05 => UacAcInterface::SelectorUnit,
                0x06 => UacAcInterface::FeatureUnit,
                0x07 => UacAcInterface::ProcessingUnit,
                0x08 => UacAcInterface::ExtensionUnit,
                _ => Self::from(subtype),
            },
            // UAC2
            0x20 => match subtype {
                0x04 => UacAcInterface::MixerUnit,
                0x05 => UacAcInterface::SelectorUnit,
                0x06 => UacAcInterface::FeatureUnit,
                0x07 => UacAcInterface::EffectUnit,
                0x08 => UacAcInterface::ProcessingUnit,
                0x09 => UacAcInterface::ExtensionUnit,
                0x0a => UacAcInterface::ClockSource,
                0x0b => UacAcInterface::ClockSelector,
                0x0c => UacAcInterface::ClockMultiplier,
                0x0d => UacAcInterface::SampleRateConverter,
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
pub enum UacAsInterface {
    Undefined = 0x00,
    General = 0x01,
    FormatType = 0x02,
    FormatSpecific = 0x03,
}

impl From<u8> for UacAsInterface {
    fn from(b: u8) -> Self {
        match b {
            0x00 => UacAsInterface::Undefined,
            0x01 => UacAsInterface::General,
            0x02 => UacAsInterface::FormatType,
            0x03 => UacAsInterface::FormatSpecific,
            _ => UacAsInterface::Undefined,
        }
    }
}

impl fmt::Display for UacAsInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // uppercase with _ instead of space for lsusb dump
            match self {
                UacAsInterface::Undefined => write!(f, "UNDEFINED"),
                UacAsInterface::General => write!(f, "GENERAL"),
                UacAsInterface::FormatType => write!(f, "FORMAT_TYPE"),
                UacAsInterface::FormatSpecific => write!(f, "FORMAT_SPECIFIC"),
            }
        } else {
            match self {
                UacAsInterface::Undefined => write!(f, "Undefined"),
                UacAsInterface::General => write!(f, "General"),
                UacAsInterface::FormatType => write!(f, "Format Type"),
                UacAsInterface::FormatSpecific => write!(f, "Format Specific"),
            }
        }
    }
}

impl UacAsInterface {
    /// Get the UAC interface descriptor from the UAC interface
    pub fn get_descriptor(
        &self,
        protocol: &UacProtocol,
        data: &[u8],
    ) -> Result<UacInterfaceDescriptor, Error> {
        UacInterfaceDescriptor::from_uac_as_interface(self, protocol, data)
    }
}

/// The control setting for a UAC bmControls byte
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
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
pub enum ControlType {
    BmControl1,
    BmControl2,
}

/// UAC1: 4.3.2 Class-Specific AC Interface Descriptor; Table 4-2.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioHeader1 {
    pub version: Version,
    pub total_length: u16,
    pub collection_bytes: u8,
    pub interfaces: Vec<u8>,
}

impl TryFrom<&[u8]> for AudioHeader1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Header 1 descriptor too short",
            ));
        }

        let total_length = u16::from_le_bytes([value[2], value[3]]);
        let collection_bytes = value[4];
        let interfaces = value[5..].to_vec();

        Ok(AudioHeader1 {
            version: Version::from_bcd(u16::from_le_bytes([value[0], value[1]])),
            total_length,
            collection_bytes,
            interfaces,
        })
    }
}

impl Into<Vec<u8>> for AudioHeader1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&(u16::from(self.version)).to_le_bytes());
        data.extend_from_slice(&self.total_length.to_le_bytes());
        data.push(self.collection_bytes);
        data.extend_from_slice(&self.interfaces);
        data
    }
}

/// UAC2: 4.7.2 Class-Specific AC Interface Descriptor; Table 4-5.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioHeader2 {
    pub version: Version,
    pub category: u8,
    pub total_length: u16,
    pub controls: u8,
}

impl TryFrom<&[u8]> for AudioHeader2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Header 2 descriptor too short",
            ));
        }

        let total_length = u16::from_le_bytes([value[3], value[4]]);
        let controls = value[5];

        Ok(AudioHeader2 {
            version: Version::from_bcd(u16::from_le_bytes([value[0], value[1]])),
            category: value[2],
            total_length,
            controls,
        })
    }
}

impl Into<Vec<u8>> for AudioHeader2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&(u16::from(self.version)).to_le_bytes());
        data.push(self.category);
        data.extend_from_slice(&self.total_length.to_le_bytes());
        data.push(self.controls);
        data
    }
}

/// UAC3: 4.5.2 Class-Specific AC Interface Descriptor; Table 4-15.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioHeader3 {
    pub category: u8,
    pub total_length: u16,
    pub controls: u32,
}

impl TryFrom<&[u8]> for AudioHeader3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Header 3 descriptor too short",
            ));
        }

        let total_length = u16::from_le_bytes([value[1], value[2]]);
        let controls = u32::from_le_bytes([value[3], value[4], value[5], value[6]]);

        Ok(AudioHeader3 {
            category: value[0],
            total_length,
            controls,
        })
    }
}

impl Into<Vec<u8>> for AudioHeader3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.category);
        data.extend_from_slice(&self.total_length.to_le_bytes());
        data.extend_from_slice(&self.controls.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.1 Input Terminal Descriptor; Table 4-3.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioInputTerminal1 {
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

impl TryFrom<&[u8]> for AudioInputTerminal1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Input Terminal 1 descriptor too short",
            ));
        }

        Ok(AudioInputTerminal1 {
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

impl Into<Vec<u8>> for AudioInputTerminal1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_id);
        data.extend_from_slice(&self.terminal_type.to_le_bytes());
        data.push(self.assoc_terminal);
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names_index);
        data.push(self.terminal_index);
        data
    }
}

/// UAC2: 4.7.2.4 Input Terminal Descriptor; Table 4-9.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioInputTerminal2 {
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

impl TryFrom<&[u8]> for AudioInputTerminal2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 14 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Input Terminal 2 descriptor too short",
            ));
        }

        Ok(AudioInputTerminal2 {
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

impl Into<Vec<u8>> for AudioInputTerminal2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_id);
        data.extend_from_slice(&self.terminal_type.to_le_bytes());
        data.push(self.assoc_terminal);
        data.push(self.csource_id);
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names_index);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.push(self.terminal_index);
        data
    }
}

/// UAC3: 4.5.2.1 Input Terminal Descriptor; Table 4-16.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioInputTerminal3 {
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

impl TryFrom<&[u8]> for AudioInputTerminal3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 17 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Input Terminal 3 descriptor too short",
            ));
        }

        Ok(AudioInputTerminal3 {
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

impl Into<Vec<u8>> for AudioInputTerminal3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_id);
        data.extend_from_slice(&self.terminal_type.to_le_bytes());
        data.push(self.assoc_terminal);
        data.push(self.csource_id);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.cluster_descr_id.to_le_bytes());
        data.extend_from_slice(&self.ex_terminal_descr_id.to_le_bytes());
        data.extend_from_slice(&self.connectors_descr_id.to_le_bytes());
        data.extend_from_slice(&self.terminal_descr_str.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.2 Output Terminal Descriptor; Table 4-4.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioOutputTerminal1 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub source_id: u8,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for AudioOutputTerminal1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Output Terminal 1 descriptor too short",
            ));
        }

        Ok(AudioOutputTerminal1 {
            terminal_id: value[0],
            terminal_type: u16::from_le_bytes([value[1], value[2]]),
            assoc_terminal: value[3],
            source_id: value[4],
            terminal_index: value[5],
            terminal: None,
        })
    }
}

impl Into<Vec<u8>> for AudioOutputTerminal1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_id);
        data.extend_from_slice(&self.terminal_type.to_le_bytes());
        data.push(self.assoc_terminal);
        data.push(self.source_id);
        data.push(self.terminal_index);
        data
    }
}

/// UAC2: 4.7.2.5 Output Terminal Descriptor; Table 4-10.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioOutputTerminal2 {
    pub terminal_id: u8,
    pub terminal_type: u16,
    pub assoc_terminal: u8,
    pub source_id: u8,
    pub c_source_id: u8,
    pub controls: u16,
    pub terminal_index: u8,
    pub terminal: Option<String>,
}

impl TryFrom<&[u8]> for AudioOutputTerminal2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 9 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Output Terminal 2 descriptor too short",
            ));
        }

        Ok(AudioOutputTerminal2 {
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

impl Into<Vec<u8>> for AudioOutputTerminal2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_id);
        data.extend_from_slice(&self.terminal_type.to_le_bytes());
        data.push(self.assoc_terminal);
        data.push(self.source_id);
        data.push(self.c_source_id);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.push(self.terminal_index);
        data
    }
}

/// UAC3: 4.5.2.2 Output Terminal Descriptor; Table 4-17.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioOutputTerminal3 {
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

impl TryFrom<&[u8]> for AudioOutputTerminal3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 17 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Output Terminal 3 descriptor too short",
            ));
        }

        Ok(AudioOutputTerminal3 {
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

impl Into<Vec<u8>> for AudioOutputTerminal3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_id);
        data.extend_from_slice(&self.terminal_type.to_le_bytes());
        data.push(self.assoc_terminal);
        data.push(self.source_id);
        data.push(self.c_source_id);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.ex_terminal_descr_id.to_le_bytes());
        data.extend_from_slice(&self.connectors_descr_id.to_le_bytes());
        data.extend_from_slice(&self.terminal_descr_str.to_le_bytes());
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Extended Terminal Header descriptor too short",
            ));
        }

        Ok(ExtendedTerminalHeader {
            descriptor_id: value[0],
            nr_channels: value[1],
        })
    }
}

impl Into<Vec<u8>> for ExtendedTerminalHeader {
    fn into(self) -> Vec<u8> {
        vec![self.descriptor_id, self.nr_channels]
    }
}

/// UAC3: 4.5.2.15 Power Domain Descriptor; Table 4-46. */
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioPowerDomain {
    pub power_domain_id: u8,
    pub recovery_time_1: u16,
    pub recovery_time_2: u16,
    pub nr_entities: u8,
    pub entity_ids: Vec<u8>,
    pub domain_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioPowerDomain {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Power Domain 3 descriptor too short",
            ));
        }

        let nr_entities = value[5] as usize;
        let expected_len = 8 + nr_entities;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Power Domain 3 descriptor too short for the number of entities",
            ));
        }

        Ok(AudioPowerDomain {
            power_domain_id: value[0],
            recovery_time_1: u16::from_le_bytes([value[1], value[2]]),
            recovery_time_2: u16::from_le_bytes([value[3], value[4]]),
            nr_entities: value[5],
            entity_ids: value[6..6 + nr_entities].to_vec(),
            domain_descr_str: u16::from_le_bytes([value[6 + nr_entities], value[7 + nr_entities]]),
        })
    }
}

impl Into<Vec<u8>> for AudioPowerDomain {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.power_domain_id);
        data.extend_from_slice(&self.recovery_time_1.to_le_bytes());
        data.extend_from_slice(&self.recovery_time_2.to_le_bytes());
        data.push(self.nr_entities);
        data.extend_from_slice(&self.entity_ids);
        data.extend_from_slice(&self.domain_descr_str.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.3 Mixer Unit Descriptor; Table 4-5.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioMixerUnit1 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub nr_channels: u8,
    pub channel_config: u16,
    pub channel_names: u8,
    pub controls: Vec<u8>,
    pub mixer: u8,
}

impl TryFrom<&[u8]> for AudioMixerUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 1 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let nr_channels = value[3] as usize;
        let expected_len = 6 + nr_in_pins + nr_channels;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 1 descriptor too short for the number of pins and channels",
            ));
        }

        Ok(AudioMixerUnit1 {
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

impl Into<Vec<u8>> for AudioMixerUnit1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names);
        data.extend_from_slice(&self.controls);
        data.push(self.mixer);
        data
    }
}

/// UAC2: 4.7.2.6 Mixer Unit Descriptor; Table 4-11.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioMixerUnit2 {
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

impl TryFrom<&[u8]> for AudioMixerUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let nr_channels = value[3] as usize;
        let expected_len = 10 + nr_in_pins + nr_channels;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 2 descriptor too short for the number of pins and channels",
            ));
        }

        Ok(AudioMixerUnit2 {
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

impl Into<Vec<u8>> for AudioMixerUnit2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names);
        data.extend_from_slice(&self.mixer_controls);
        data.push(self.controls);
        data.push(self.mixer);
        data
    }
}

/// UAC3: 4.5.2.5 Mixer Unit Descriptor; Table 4-29.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioMixerUnit3 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub cluster_descr_id: u16,
    pub mixer_controls: Vec<u8>,
    pub controls: u32,
    pub mixer_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioMixerUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_len = 8 + nr_in_pins;
        if value.len() < expected_len {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Mixer Unit 3 descriptor too short for the number of pins",
            ));
        }

        Ok(AudioMixerUnit3 {
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

impl Into<Vec<u8>> for AudioMixerUnit3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.extend_from_slice(&self.cluster_descr_id.to_le_bytes());
        data.extend_from_slice(&self.mixer_controls);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.mixer_descr_str.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioStreamingInterface1 {
    pub terminal_link: u8,
    pub delay: u8,
    pub format_tag: u16,
}

impl TryFrom<&[u8]> for AudioStreamingInterface1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Streaming Interface 1 descriptor too short",
            ));
        }

        Ok(AudioStreamingInterface1 {
            terminal_link: value[0],
            delay: value[1],
            format_tag: u16::from_le_bytes([value[2], value[3]]),
        })
    }
}

impl Into<Vec<u8>> for AudioStreamingInterface1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_link);
        data.push(self.delay);
        data.extend_from_slice(&self.format_tag.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioStreamingInterface2 {
    pub terminal_link: u8,
    pub controls: u8, // BmControl2
    pub format_type: u8,
    pub formats: u32,
    pub nr_channels: u8,
    pub channel_config: u32,
    pub channel_names_index: u8,
    pub channel_names: Option<String>,
}

impl TryFrom<&[u8]> for AudioStreamingInterface2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 13 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Streaming Interface 2 descriptor too short",
            ));
        }

        Ok(AudioStreamingInterface2 {
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

impl Into<Vec<u8>> for AudioStreamingInterface2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_link);
        data.push(self.controls);
        data.push(self.format_type);
        data.extend_from_slice(&self.formats.to_le_bytes());
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names_index);
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioStreamingInterface3 {
    pub terminal_link: u8,
    pub controls: u32, // BmControl2
    pub cluster_descr_id: u16,
    pub formats: u64,
    pub sub_slot_size: u8,
    pub bit_resolution: u8,
    pub aux_protocols: u16,
    pub control_size: u8,
}

impl TryFrom<&[u8]> for AudioStreamingInterface3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 20 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Streaming Interface 3 descriptor too short",
            ));
        }

        Ok(AudioStreamingInterface3 {
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

impl Into<Vec<u8>> for AudioStreamingInterface3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.terminal_link);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.cluster_descr_id.to_le_bytes());
        data.extend_from_slice(&self.formats.to_le_bytes());
        data.push(self.sub_slot_size);
        data.push(self.bit_resolution);
        data.extend_from_slice(&self.aux_protocols.to_le_bytes());
        data.push(self.control_size);
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(missing_docs)]
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
pub struct AudioDataStreamingEndpoint1 {
    pub attributes: u8,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl TryFrom<&[u8]> for AudioDataStreamingEndpoint1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Data Streaming Endpoint 1 descriptor too short",
            ));
        }

        Ok(AudioDataStreamingEndpoint1 {
            attributes: value[0],
            lock_delay_units: value[1],
            lock_delay: u16::from_le_bytes([value[2], value[3]]),
        })
    }
}

impl Into<Vec<u8>> for AudioDataStreamingEndpoint1 {
    fn into(self) -> Vec<u8> {
        vec![self.attributes, self.lock_delay_units, self.lock_delay.to_le_bytes()[0], self.lock_delay.to_le_bytes()[1]]
    }
}

/// Isochronous Audio Data Stream Endpoint for UAC2
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioDataStreamingEndpoint2 {
    pub attributes: u8,
    pub controls: u8,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl TryFrom<&[u8]> for AudioDataStreamingEndpoint2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Data Streaming Endpoint 2 descriptor too short",
            ));
        }

        Ok(AudioDataStreamingEndpoint2 {
            attributes: value[0],
            controls: value[1],
            lock_delay_units: value[2],
            lock_delay: u16::from_le_bytes([value[3], value[4]]),
        })
    }
}

impl Into<Vec<u8>> for AudioDataStreamingEndpoint2 {
    fn into(self) -> Vec<u8> {
        vec![self.attributes, self.controls, self.lock_delay_units, self.lock_delay.to_le_bytes()[0], self.lock_delay.to_le_bytes()[1]]
    }
}

/// Isochronous Audio Data Stream Endpoint for UAC3
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioDataStreamingEndpoint3 {
    pub controls: u32,
    pub lock_delay_units: u8,
    pub lock_delay: u16,
}

impl TryFrom<&[u8]> for AudioDataStreamingEndpoint3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Data Streaming Endpoint 3 descriptor too short",
            ));
        }

        Ok(AudioDataStreamingEndpoint3 {
            controls: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
            lock_delay_units: value[4],
            lock_delay: u16::from_le_bytes([value[5], value[6]]),
        })
    }
}

impl Into<Vec<u8>> for AudioDataStreamingEndpoint3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.push(self.lock_delay_units);
        data.extend_from_slice(&self.lock_delay.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSelectorUnit1 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub selector_index: u8,
    pub selector: Option<String>,
}

impl TryFrom<&[u8]> for AudioSelectorUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 1 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 3 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 1 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(AudioSelectorUnit1 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids,
            selector_index: value[expected_length - 1],
            selector: None,
        })
    }
}

impl Into<Vec<u8>> for AudioSelectorUnit1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.push(self.selector_index);
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSelectorUnit2 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub controls: u8,
    pub selector_index: u8,
    pub selector: Option<String>,
}

impl TryFrom<&[u8]> for AudioSelectorUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 4 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 2 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(AudioSelectorUnit2 {
            unit_id: value[0],
            nr_in_pins: value[1],
            source_ids,
            controls: value[2 + nr_in_pins],
            selector_index: value[expected_length - 1],
            selector: None,
        })
    }
}

impl Into<Vec<u8>> for AudioSelectorUnit2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.push(self.controls);
        data.push(self.selector_index);
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSelectorUnit3 {
    pub unit_id: u8,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub controls: u32,
    pub selector_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioSelectorUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 6 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Selector Unit 3 descriptor too short",
            ));
        }

        let source_ids = value[2..(2 + nr_in_pins)].to_vec();
        let controls = u32::from_le_bytes([
            value[2 + nr_in_pins],
            value[3 + nr_in_pins],
            value[4 + nr_in_pins],
            value[5 + nr_in_pins],
        ]);

        Ok(AudioSelectorUnit3 {
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

impl Into<Vec<u8>> for AudioSelectorUnit3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.selector_descr_str.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit Extended 1 descriptor too short",
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

impl Into<Vec<u8>> for AudioProcessingUnitExtended1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.nr_modes);
        for mode in self.modes {
            data.extend_from_slice(&mode.to_le_bytes());
        }
        data
    }
}

/// UAC1: 4.3.2.6 Processing Unit Descriptor; Table 4-8.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioProcessingUnit1 {
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

impl TryFrom<&[u8]> for AudioProcessingUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 1 descriptor too short",
            ));
        }

        let nr_in_pins = value[3];
        let control_size = value[9 + nr_in_pins as usize];
        let expected_length = 10 + nr_in_pins as usize + control_size as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 1 descriptor too short",
            ));
        }

        let specific = match value[1] {
            1 | 2 => Some(AudioProcessingUnitExtended1::try_from(
                &value[expected_length..],
            )?),
            _ => None,
        };

        Ok(AudioProcessingUnit1 {
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

impl Into<Vec<u8>> for AudioProcessingUnit1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.extend_from_slice(&self.process_type.to_le_bytes());
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names_index);
        data.push(self.control_size);
        data.extend_from_slice(&self.controls);
        if let Some(specific) = self.specific {
            let specific_data: Vec<u8> = specific.into();
            data.extend_from_slice(&specific_data);
        }
        data.push(self.processing_index);
        data
    }
}

impl AudioProcessingUnit1 {
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 2 Up/Down-mix descriptor too short",
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

impl Into<Vec<u8>> for AudioProcessingUnit2UpDownMix {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.nr_modes);
        for mode in self.modes {
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 2 Dolby Prologic descriptor too short",
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

impl Into<Vec<u8>> for AudioProcessingUnit2DolbyPrologic {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.nr_modes);
        for mode in self.modes {
            data.extend_from_slice(&mode.to_le_bytes());
        }
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum AudioProcessingUnit2Specific {
    UpDownMix(AudioProcessingUnit2UpDownMix),
    DolbyPrologic(AudioProcessingUnit2DolbyPrologic),
}

impl Into<Vec<u8>> for AudioProcessingUnit2Specific {
    fn into(self) -> Vec<u8> {
        match self {
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 3 Up/Down-mix descriptor too short",
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

impl Into<Vec<u8>> for AudioProcessingUnit3UpDownMix {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.push(self.nr_modes);
        for cluster_descr_id in self.cluster_descr_ids {
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 3 Stereo Extender descriptor too short",
            ));
        }

        Ok(AudioProcessingUnit3StereoExtender {
            controls: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
        })
    }
}

impl Into<Vec<u8>> for AudioProcessingUnit3StereoExtender {
    fn into(self) -> Vec<u8> {
        self.controls.to_le_bytes().to_vec()
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
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 3 Multi Function descriptor too short",
            ));
        }

        Ok(AudioProcessingUnit3MultiFunction {
            controls: u32::from_le_bytes([value[0], value[1], value[2], value[3]]),
            cluster_descr_id: u16::from_le_bytes([value[4], value[5]]),
            algorithms: u32::from_le_bytes([value[6], value[7], value[8], value[9]]),
        })
    }
}

impl Into<Vec<u8>> for AudioProcessingUnit3MultiFunction {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.cluster_descr_id.to_le_bytes());
        data.extend_from_slice(&self.algorithms.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum AudioProcessingUnit3Specific {
    UpDownMix(AudioProcessingUnit3UpDownMix),
    StereoExtender(AudioProcessingUnit3StereoExtender),
    MultiFunction(AudioProcessingUnit3MultiFunction),
}

impl Into<Vec<u8>> for AudioProcessingUnit3Specific {
    fn into(self) -> Vec<u8> {
        match self {
            AudioProcessingUnit3Specific::UpDownMix(up_down_mix) => up_down_mix.into(),
            AudioProcessingUnit3Specific::StereoExtender(stereo_extender) => stereo_extender.into(),
            AudioProcessingUnit3Specific::MultiFunction(multi_function) => multi_function.into(),
        }
    }
}

/// UAC2: 4.7.2.11 Processing Unit Descriptor; Table 4-20.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioProcessingUnit2 {
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

impl TryFrom<&[u8]> for AudioProcessingUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 12 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[3];
        let expected_length = 12 + nr_in_pins as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 2 descriptor too short",
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

        Ok(AudioProcessingUnit2 {
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

impl Into<Vec<u8>> for AudioProcessingUnit2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.extend_from_slice(&self.process_type.to_le_bytes());
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names_index);
        data.extend_from_slice(&self.controls.to_le_bytes());
        if let Some(specific) = self.specific {
            let specific_data: Vec<u8> = specific.into();
            data.extend_from_slice(&specific_data);
        }
        data.push(self.processing_index);
        data
    }
}

impl AudioProcessingUnit2 {
    /// Returns the [`AudioProcessingUnitType`] of the processing unit.
    pub fn processing_type(&self) -> AudioProcessingUnitType {
        (UacProtocol::Uac2, self.process_type).into()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
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
pub struct AudioProcessingUnit3 {
    pub unit_id: u8,
    pub process_type: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub processing_descr_str: u16,
    pub specific: Option<AudioProcessingUnit3Specific>,
}

impl TryFrom<&[u8]> for AudioProcessingUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[3];
        let expected_length = 7 + nr_in_pins as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Processing Unit 3 descriptor too short",
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

        Ok(AudioProcessingUnit3 {
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

impl Into<Vec<u8>> for AudioProcessingUnit3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.extend_from_slice(&self.process_type.to_le_bytes());
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.extend_from_slice(&self.processing_descr_str.to_le_bytes());
        if let Some(specific) = self.specific {
            let specific_data: Vec<u8> = specific.into();
            data.extend_from_slice(&specific_data);
        }
        data
    }
}

impl AudioProcessingUnit3 {
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
pub struct AudioEffectUnit2 {
    pub unit_id: u8,
    pub effect_type: u16,
    pub source_id: u8,
    pub controls: Vec<u32>,
    pub effect_index: u8,
    pub effect: Option<String>,
}

impl TryFrom<&[u8]> for AudioEffectUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Effect Unit 2 descriptor too short",
            ));
        }

        let controls = (4..value.len() - 1)
            .step_by(4)
            .map(|i| u32::from_le_bytes([value[i], value[i + 1], value[i + 2], value[i + 3]]))
            .collect();

        Ok(AudioEffectUnit2 {
            unit_id: value[0],
            effect_type: u16::from_le_bytes([value[1], value[2]]),
            source_id: value[3],
            controls,
            effect_index: value[value.len() - 1],
            effect: None,
        })
    }
}

impl Into<Vec<u8>> for AudioEffectUnit2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.extend_from_slice(&self.effect_type.to_le_bytes());
        data.push(self.source_id);
        for control in self.controls {
            data.extend_from_slice(&control.to_le_bytes());
        }
        data.push(self.effect_index);
        data
    }
}

/// UAC3: 4.5.2.9 Effect Unit Descriptor; Table 4-33.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioEffectUnit3 {
    pub unit_id: u8,
    pub effect_type: u16,
    pub source_id: u8,
    pub controls: Vec<u32>,
    pub effect_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioEffectUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Effect Unit 3 descriptor too short",
            ));
        }

        let controls = (4..value.len() - 2)
            .step_by(4)
            .map(|i| u32::from_le_bytes([value[i], value[i + 1], value[i + 2], value[i + 3]]))
            .collect();

        Ok(AudioEffectUnit3 {
            unit_id: value[0],
            effect_type: u16::from_le_bytes([value[1], value[2]]),
            source_id: value[3],
            controls,
            effect_descr_str: u16::from_le_bytes([value[value.len() - 2], value[value.len() - 1]]),
        })
    }
}

impl Into<Vec<u8>> for AudioEffectUnit3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.extend_from_slice(&self.effect_type.to_le_bytes());
        data.push(self.source_id);
        for control in self.controls {
            data.extend_from_slice(&control.to_le_bytes());
        }
        data.extend_from_slice(&self.effect_descr_str.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.5 Feature Unit Descriptor; Table 4-7.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioFeatureUnit1 {
    pub unit_id: u8,
    pub source_id: u8,
    pub control_size: u8,
    pub controls: Vec<u8>,
    pub feature_index: u8,
    pub feature: Option<String>,
}

impl TryFrom<&[u8]> for AudioFeatureUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Feature Unit 1 descriptor too short",
            ));
        }

        let control_size = value[2];
        let expected_length = 4 + control_size as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Feature Unit 1 descriptor too short",
            ));
        }

        let controls = value[3..(3 + control_size as usize)].to_vec();

        Ok(AudioFeatureUnit1 {
            unit_id: value[0],
            source_id: value[1],
            control_size,
            controls,
            feature_index: value[expected_length - 1],
            feature: None,
        })
    }
}

impl Into<Vec<u8>> for AudioFeatureUnit1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.source_id);
        data.push(self.control_size);
        data.extend_from_slice(&self.controls);
        data.push(self.feature_index);
        data
    }
}

/// UAC2: 4.7.2.8 Feature Unit Descriptor; Table 4-13.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioFeatureUnit2 {
    pub unit_id: u8,
    pub source_id: u8,
    pub controls: [u8; 4],
    pub feature_index: u8,
    pub feature: Option<String>,
}

impl TryFrom<&[u8]> for AudioFeatureUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 7 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Feature Unit 2 descriptor too short",
            ));
        }

        Ok(AudioFeatureUnit2 {
            unit_id: value[0],
            source_id: value[1],
            controls: value[2..6].try_into().unwrap(),
            feature_index: value[7],
            feature: None,
        })
    }
}

impl Into<Vec<u8>> for AudioFeatureUnit2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.source_id);
        data.extend_from_slice(&self.controls);
        data.push(self.feature_index);
        data
    }
}

/// UAC3: 4.5.2.7 Feature Unit Descriptor; Table 4-31.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioFeatureUnit3 {
    pub unit_id: u8,
    pub source_id: u8,
    pub controls: [u8; 4],
    pub feature_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioFeatureUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Feature Unit 3 descriptor too short",
            ));
        }

        Ok(AudioFeatureUnit3 {
            unit_id: value[0],
            source_id: value[1],
            controls: value[2..6].try_into().unwrap(),
            feature_descr_str: u16::from_le_bytes([value[6], value[7]]),
        })
    }
}

impl Into<Vec<u8>> for AudioFeatureUnit3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.source_id);
        data.extend_from_slice(&self.controls);
        data.extend_from_slice(&self.feature_descr_str.to_le_bytes());
        data
    }
}

/// UAC1: 4.3.2.7 Extension Unit Descriptor; Table 4-15.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioExtensionUnit1 {
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

impl TryFrom<&[u8]> for AudioExtensionUnit1 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 1 descriptor too short",
            ));
        }

        let nr_in_pins = value[3] as usize;
        let control_size = value[8 + nr_in_pins];
        let expected_length = 10 + nr_in_pins + control_size as usize;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 1 descriptor too short",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();
        let controls = value[(9 + nr_in_pins)..(9 + nr_in_pins + control_size as usize)].to_vec();

        Ok(AudioExtensionUnit1 {
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

impl Into<Vec<u8>> for AudioExtensionUnit1 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.extend_from_slice(&self.extension_code.to_le_bytes());
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names_index);
        data.push(self.control_size);
        data.extend_from_slice(&self.controls);
        data.push(self.extension_index);
        data
    }
}

/// UAC2: 4.7.2.12 Extension Unit Descriptor; Table 4-24.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioExtensionUnit2 {
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

impl TryFrom<&[u8]> for AudioExtensionUnit2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 11 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[3] as usize;
        let expected_length = 10 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 2 descriptor too short",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();

        Ok(AudioExtensionUnit2 {
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

impl Into<Vec<u8>> for AudioExtensionUnit2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.extend_from_slice(&self.extension_code.to_le_bytes());
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.push(self.nr_channels);
        data.extend_from_slice(&self.channel_config.to_le_bytes());
        data.push(self.channel_names_index);
        data.push(self.controls);
        data.push(self.extension_index);
        data
    }
}

/// UAC3: 4.5.2.11 Extension Unit Descriptor; Table 4-42.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioExtensionUnit3 {
    pub unit_id: u8,
    pub extension_code: u16,
    pub nr_in_pins: u8,
    pub source_ids: Vec<u8>,
    pub extension_descr_str: u16,
    pub controls: u32,
    pub cluster_descr_id: u16,
}

impl TryFrom<&[u8]> for AudioExtensionUnit3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[3] as usize;
        let expected_length = 9 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Extension Unit 3 descriptor too short",
            ));
        }

        let source_ids = value[4..(4 + nr_in_pins)].to_vec();

        Ok(AudioExtensionUnit3 {
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

impl Into<Vec<u8>> for AudioExtensionUnit3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.extend_from_slice(&self.extension_code.to_le_bytes());
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.source_ids);
        data.extend_from_slice(&self.extension_descr_str.to_le_bytes());
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.cluster_descr_id.to_le_bytes());
        data
    }
}

/// UAC2: 4.7.2.1 Clock Source Descriptor; Table 4-6.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockSource2 {
    pub clock_id: u8,
    pub attributes: u8,
    pub controls: u8,
    pub assoc_terminal: u8,
    pub clock_source_index: u8,
    pub clock_source: Option<String>,
}

impl TryFrom<&[u8]> for AudioClockSource2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Source 2 descriptor too short",
            ));
        }

        Ok(AudioClockSource2 {
            clock_id: value[0],
            attributes: value[1],
            controls: value[2],
            assoc_terminal: value[3],
            clock_source_index: value[4],
            clock_source: None,
        })
    }
}

impl Into<Vec<u8>> for AudioClockSource2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.clock_id);
        data.push(self.attributes);
        data.push(self.controls);
        data.push(self.assoc_terminal);
        data.push(self.clock_source_index);
        data
    }
}

/// UAC3: 4.5.2.12 Clock Source Descriptor; Table 4-43.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockSource3 {
    pub clock_id: u8,
    pub attributes: u8,
    pub controls: u32,
    pub reference_terminal: u8,
    pub clock_source_str: u16,
}

impl TryFrom<&[u8]> for AudioClockSource3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Source 3 descriptor too short",
            ));
        }

        Ok(AudioClockSource3 {
            clock_id: value[0],
            attributes: value[1],
            controls: u32::from_le_bytes([value[2], value[3], value[4], value[5]]),
            reference_terminal: value[6],
            clock_source_str: u16::from_le_bytes([value[7], value[8]]),
        })
    }
}

impl Into<Vec<u8>> for AudioClockSource3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.clock_id);
        data.push(self.attributes);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.push(self.reference_terminal);
        data.extend_from_slice(&self.clock_source_str.to_le_bytes());
        data
    }
}

/// UAC2: 4.7.2.2 Clock Selector Descriptor; Table 4-7.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockSelector2 {
    pub clock_id: u8,
    pub nr_in_pins: u8,
    pub csource_ids: Vec<u8>,
    pub controls: u8,
    pub clock_selector_index: u8,
    pub clock_selector: Option<String>,
}

impl TryFrom<&[u8]> for AudioClockSelector2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Selector 2 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 3 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Selector 2 descriptor too short",
            ));
        }

        let csource_ids = value[2..(2 + nr_in_pins)].to_vec();

        Ok(AudioClockSelector2 {
            clock_id: value[0],
            nr_in_pins: value[1],
            csource_ids,
            controls: value[2 + nr_in_pins],
            clock_selector_index: value[expected_length - 1],
            clock_selector: None,
        })
    }
}

impl Into<Vec<u8>> for AudioClockSelector2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.clock_id);
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.csource_ids);
        data.push(self.controls);
        data.push(self.clock_selector_index);
        data
    }
}

/// UAC3: 4.5.2.13 Clock Selector Descriptor; Table 4-44.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockSelector3 {
    pub clock_id: u8,
    pub nr_in_pins: u8,
    pub csource_ids: Vec<u8>,
    pub controls: u32,
    pub cselector_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioClockSelector3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Selector 3 descriptor too short",
            ));
        }

        let nr_in_pins = value[1] as usize;
        let expected_length = 5 + nr_in_pins;
        if value.len() < expected_length {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Selector 3 descriptor too short",
            ));
        }

        let csource_ids = value[2..(2 + nr_in_pins)].to_vec();
        let controls = u32::from_le_bytes([
            value[2 + nr_in_pins],
            value[3 + nr_in_pins],
            value[4 + nr_in_pins],
            value[5 + nr_in_pins],
        ]);

        Ok(AudioClockSelector3 {
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

impl Into<Vec<u8>> for AudioClockSelector3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.clock_id);
        data.push(self.nr_in_pins);
        data.extend_from_slice(&self.csource_ids);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.cselector_descr_str.to_le_bytes());
        data
    }
}

/// UAC2: 4.7.2.3 Clock Multiplier Descriptor; Table 4-8.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockMultiplier2 {
    pub clock_id: u8,
    pub csource_id: u8,
    pub controls: u8,
    pub clock_multiplier_index: u8,
    pub clock_multiplier: Option<String>,
}

impl TryFrom<&[u8]> for AudioClockMultiplier2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Multiplier 2 descriptor too short",
            ));
        }

        Ok(AudioClockMultiplier2 {
            clock_id: value[0],
            csource_id: value[1],
            controls: value[2],
            clock_multiplier_index: value[3],
            clock_multiplier: None,
        })
    }
}

impl Into<Vec<u8>> for AudioClockMultiplier2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.clock_id);
        data.push(self.csource_id);
        data.push(self.controls);
        data.push(self.clock_multiplier_index);
        data
    }
}

/// UAC3: 4.5.2.14 Clock Multiplier Descriptor; Table 4-45.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioClockMultiplier3 {
    pub clock_id: u8,
    pub csource_id: u8,
    pub controls: u32,
    pub cmultiplier_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioClockMultiplier3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Clock Multiplier 3 descriptor too short",
            ));
        }

        Ok(AudioClockMultiplier3 {
            clock_id: value[0],
            csource_id: value[1],
            controls: u32::from_le_bytes([value[2], value[3], value[4], value[5]]),
            cmultiplier_descr_str: u16::from_le_bytes([value[6], value[7]]),
        })
    }
}

impl Into<Vec<u8>> for AudioClockMultiplier3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.clock_id);
        data.push(self.csource_id);
        data.extend_from_slice(&self.controls.to_le_bytes());
        data.extend_from_slice(&self.cmultiplier_descr_str.to_le_bytes());
        data
    }
}

/// UAC2: 4.7.2.9 Sampling Rate Converter Descriptor; Table 4-14.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSampleRateConverter2 {
    pub unit_id: u8,
    pub source_id: u8,
    pub csource_in_id: u8,
    pub csource_out_id: u8,
    pub src_index: u8,
    pub src: Option<String>,
}

impl TryFrom<&[u8]> for AudioSampleRateConverter2 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 5 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Sample Rate Converter 2 descriptor too short",
            ));
        }

        Ok(AudioSampleRateConverter2 {
            unit_id: value[0],
            source_id: value[1],
            csource_in_id: value[2],
            csource_out_id: value[3],
            src_index: value[4],
            src: None,
        })
    }
}

impl Into<Vec<u8>> for AudioSampleRateConverter2 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.source_id);
        data.push(self.csource_in_id);
        data.push(self.csource_out_id);
        data.push(self.src_index);
        data
    }
}

/// UAC3: 4.5.2.8 Sampling Rate Converter Descriptor; Table 4-32.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSampleRateConverter3 {
    pub unit_id: u8,
    pub source_id: u8,
    pub csource_in_id: u8,
    pub csource_out_id: u8,
    pub src_descr_str: u16,
}

impl TryFrom<&[u8]> for AudioSampleRateConverter3 {
    type Error = Error;

    fn try_from(value: &[u8]) -> error::Result<Self> {
        if value.len() < 6 {
            return Err(Error::new(
                ErrorKind::InvalidArg,
                "Audio Sample Rate Converter 3 descriptor too short",
            ));
        }

        Ok(AudioSampleRateConverter3 {
            unit_id: value[0],
            source_id: value[1],
            csource_in_id: value[2],
            csource_out_id: value[3],
            src_descr_str: u16::from_le_bytes([value[4], value[5]]),
        })
    }
}

impl Into<Vec<u8>> for AudioSampleRateConverter3 {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.unit_id);
        data.push(self.source_id);
        data.push(self.csource_in_id);
        data.push(self.csource_out_id);
        data.extend_from_slice(&self.src_descr_str.to_le_bytes());
        data
    }
}
