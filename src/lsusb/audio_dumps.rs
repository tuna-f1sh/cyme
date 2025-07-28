use crate::usb::descriptors::audio;

use super::*;

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
const UAC2_EXTENSION_UNIT_BMCONTROLS: [&str; 4] = ["Enable", "Cluster", "Underflow", "Overflow"];
const UAC3_EXTENSION_UNIT_BMCONTROLS: [&str; 2] = ["Underflow", "Overflow"];
const UAC2_CLOCK_SOURCE_BMCONTROLS: [&str; 2] = ["Clock Frequency", "Clock Validity"];
const UAC2_CLOCK_SELECTOR_BMCONTROLS: [&str; 1] = ["Clock Selector"];
const UAC2_CLOCK_MULTIPLIER_BMCONTROLS: [&str; 2] = ["Clock Numerator", "Clock Denominator"];
const UAC3_PROCESSING_UNIT_UP_DOWN_BMCONTROLS: [&str; 3] = ["Mode Select", "Underflow", "Overflow"];
const UAC3_PROCESSING_UNIT_STEREO_EXTENDER_BMCONTROLS: [&str; 3] =
    ["Width", "Underflow", "Overflow"];
const UAC3_PROCESSING_UNIT_MULTI_FUNC_BMCONTROLS: [&str; 2] = ["Underflow", "Overflow"];

fn dump_bitmap_controls<T: Into<u32>>(
    controls: T,
    control_descriptions: &[&'static str],
    desc_type: &audio::ControlType,
    indent: usize,
) {
    let controls: u32 = controls.into();
    for (index, control) in control_descriptions.iter().enumerate() {
        match desc_type {
            audio::ControlType::BmControl1 => {
                if (controls >> index) & 0x1 != 0 {
                    println!("{:indent$}{} Control", "", control, indent = indent);
                }
            }
            audio::ControlType::BmControl2 => {
                let c = ((controls >> (index * 2)) & 0x3) as u8;
                if c != 0 {
                    println!(
                        "{:indent$}{} Control ({})",
                        "",
                        control,
                        audio::ControlSetting::from(c),
                        indent = indent
                    )
                }
            }
        }
    }
}

fn dump_bitmap_controls_array<T: Into<u32> + std::fmt::Display + Copy>(
    field_name: &str,
    controls: &[T],
    control_descriptions: &[&'static str],
    desc_type: &audio::ControlType,
    indent: usize,
    width: usize,
) {
    for (i, control) in controls.iter().enumerate() {
        let control = control.to_owned();
        let control: u32 = control.into();
        dump_value(control, &format!("{field_name}({i:2})"), indent, width);
        dump_bitmap_controls(control, control_descriptions, desc_type, indent + 2);
    }
}

fn dump_audio_mixer_unit1(mixer_unit: &audio::MixerUnit1, indent: usize, width: usize) {
    dump_value(mixer_unit.unit_id, "bUnitID", indent, width);
    dump_value(mixer_unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&mixer_unit.source_ids, "baSourceID", indent, width);
    dump_value(mixer_unit.nr_channels, "bNrChannels", indent, width);
    dump_hex(mixer_unit.channel_config, "wChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac1,
        mixer_unit.channel_config as u32,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value(mixer_unit.channel_names, "iChannelNames", indent, width);
    dump_bitmap_array(&mixer_unit.controls, "bmControls", indent, width);
    dump_value(mixer_unit.mixer, "iMixer", indent, width);
}

fn dump_audio_mixer_unit2(mixer_unit: &audio::MixerUnit2, indent: usize, width: usize) {
    dump_value(mixer_unit.unit_id, "bUnitID", indent, width);
    dump_value(mixer_unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&mixer_unit.source_ids, "baSourceID", indent, width);
    dump_value(mixer_unit.nr_channels, "bNrChannels", indent, width);
    dump_hex(mixer_unit.channel_config, "bmChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac2,
        mixer_unit.channel_config,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value(mixer_unit.channel_names, "iChannelNames", indent, width);
    dump_bitmap_array(&mixer_unit.mixer_controls, "bmMixerControls", indent, width);
    dump_hex(mixer_unit.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        mixer_unit.controls as u32,
        &UAC2_MIXER_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(mixer_unit.mixer, "iMixer", indent, width);
}

fn dump_audio_mixer_unit3(mixer_unit: &audio::MixerUnit3, indent: usize, width: usize) {
    dump_value(mixer_unit.unit_id, "bUnitID", indent, width);
    dump_value(mixer_unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&mixer_unit.source_ids, "baSourceID", indent, width);
    dump_value(
        mixer_unit.cluster_descr_id,
        "wClusterDescrID",
        indent,
        width,
    );
    dump_bitmap_array(&mixer_unit.mixer_controls, "bmMixerControls", indent, width);
    dump_hex(mixer_unit.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        mixer_unit.controls,
        &UAC3_MIXER_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(mixer_unit.mixer_descr_str, "wMixerDescrStr", indent, width);
}

fn dump_audio_power_domain(power_domain: &audio::PowerDomain, indent: usize, width: usize) {
    dump_value(
        power_domain.power_domain_id,
        "bPowerDomainID",
        indent,
        width,
    );
    dump_value(
        power_domain.recovery_time_1,
        "waRecoveryTime(1)",
        indent,
        width,
    );
    dump_value(
        power_domain.recovery_time_2,
        "waRecoveryTime(2)",
        indent,
        width,
    );
    dump_value(power_domain.nr_entities, "bNrEntities", indent, width);
    dump_array(&power_domain.entity_ids, "baEntityID", indent, width);
    dump_value(
        power_domain.domain_descr_str,
        "wPDomainDescrStr",
        indent,
        width,
    );
}

pub(crate) fn dump_audio_selector_unit1(
    selector_unit: &audio::SelectorUnit1,
    indent: usize,
    width: usize,
) {
    dump_value(selector_unit.unit_id, "bUnitID", indent, width);
    dump_value(selector_unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&selector_unit.source_ids, "baSourceID", indent, width);
    dump_value_string(
        selector_unit.selector_index,
        "iSelector",
        selector_unit.selector.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

fn dump_audio_selector_unit2(selector_unit: &audio::SelectorUnit2, indent: usize, width: usize) {
    dump_value(selector_unit.unit_id, "bUnitID", indent, width);
    dump_value(selector_unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&selector_unit.source_ids, "baSourceID", indent, width);
    dump_hex(selector_unit.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        selector_unit.controls,
        &UAC2_SELECTOR_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value_string(
        selector_unit.selector_index,
        "iSelector",
        selector_unit.selector.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

fn dump_audio_selector_unit3(selector_unit: &audio::SelectorUnit3, indent: usize, width: usize) {
    dump_value(selector_unit.unit_id, "bUnitID", indent, width);
    dump_value(selector_unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&selector_unit.source_ids, "baSourceID", indent, width);
    dump_hex(selector_unit.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        selector_unit.controls,
        &UAC2_SELECTOR_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(
        selector_unit.selector_descr_str,
        "wSelectorDescrStr",
        indent,
        width,
    );
}

/// Dumps the contents of a UAC1 Processing Unit Descriptor
fn dump_audio_processing_unit1(unit: &audio::ProcessingUnit1, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value_string(
        unit.process_type,
        "wProcessType",
        unit.processing_type(),
        indent,
        width,
    );
    dump_value(unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&unit.source_ids, "baSourceID", indent, width);
    dump_value(unit.nr_channels, "bNrChannels", indent, width);
    dump_hex(unit.channel_config, "wChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac1,
        unit.channel_config as u32,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value_string(
        unit.channel_names_index,
        "iChannelNames",
        unit.channel_names.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
    dump_value(unit.control_size, "bControlSize", indent, width);
    dump_bitmap_array(&unit.controls, "bmControls", indent, width);
    dump_value_string(
        unit.processing_index,
        "iProcessing",
        unit.processing.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
    if let Some(ref specific) = unit.specific {
        dump_value(specific.nr_modes, "bNrModes", indent, width);
        dump_bitmap_array(&specific.modes, "waModes", indent, width);
    }
}

/// Dumps the contents of a UAC2 Processing Unit Descriptor
fn dump_audio_processing_unit2(unit: &audio::ProcessingUnit2, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value_string(
        unit.process_type,
        "wProcessType",
        unit.processing_type(),
        indent,
        width,
    );
    dump_value(unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&unit.source_ids, "baSourceID", indent, width);
    dump_value(unit.nr_channels, "bNrChannels", indent, width);
    dump_hex(unit.channel_config, "bmChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac2,
        unit.channel_config,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value_string(
        unit.channel_names_index,
        "iChannelNames",
        unit.channel_names.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
    dump_value(unit.controls, "bmControls", indent, width);
    dump_value_string(
        unit.processing_index,
        "iProcessing",
        unit.processing.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
    if let Some(ref specific) = unit.specific {
        match specific {
            audio::AudioProcessingUnit2Specific::UpDownMix(up_down_mix) => {
                dump_value(up_down_mix.nr_modes, "bNrModes", indent, width);
                dump_bitmap_array(&up_down_mix.modes, "daModes", indent, width);
            }
            audio::AudioProcessingUnit2Specific::DolbyPrologic(dolby_prologic) => {
                dump_value(dolby_prologic.nr_modes, "bNrModes", indent, width);
                dump_bitmap_array(&dolby_prologic.modes, "daModes", indent, width);
            }
        }
    }
}

/// Dumps the contents of a UAC3 Processing Unit Descriptor
fn dump_audio_processing_unit3(unit: &audio::ProcessingUnit3, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value_string(
        unit.process_type,
        "wProcessType",
        unit.processing_type(),
        indent,
        width,
    );
    dump_value(unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&unit.source_ids, "baSourceID", indent, width);
    dump_value(
        unit.processing_descr_str,
        "wProcessingDescrStr",
        indent,
        width,
    );
    if let Some(ref specific) = unit.specific {
        match specific {
            audio::AudioProcessingUnit3Specific::UpDownMix(up_down_mix) => {
                dump_hex(up_down_mix.controls, "bmControls", indent, width);
                dump_bitmap_controls(
                    up_down_mix.controls,
                    &UAC3_PROCESSING_UNIT_UP_DOWN_BMCONTROLS,
                    &audio::ControlType::BmControl2,
                    indent + 2,
                );
                dump_value(up_down_mix.nr_modes, "bNrModes", indent, width);
                dump_array(
                    &up_down_mix.cluster_descr_ids,
                    "waClusterDescrID",
                    indent,
                    width,
                );
            }
            audio::AudioProcessingUnit3Specific::StereoExtender(stereo_extender) => {
                dump_hex(stereo_extender.controls, "bmControls", indent, width);
                dump_bitmap_controls(
                    stereo_extender.controls,
                    &UAC3_PROCESSING_UNIT_STEREO_EXTENDER_BMCONTROLS,
                    &audio::ControlType::BmControl2,
                    indent + 2,
                );
            }
            audio::AudioProcessingUnit3Specific::MultiFunction(multi_function) => {
                dump_hex(multi_function.controls, "bmControls", indent, width);
                dump_bitmap_controls(
                    multi_function.controls,
                    &UAC3_PROCESSING_UNIT_MULTI_FUNC_BMCONTROLS,
                    &audio::ControlType::BmControl2,
                    indent + 2,
                );
                dump_value(
                    multi_function.cluster_descr_id,
                    "wClusterDescrID",
                    indent,
                    width,
                );
                dump_value(multi_function.algorithms, "bmAlgorithms", indent, width);
                if let Some(ref algorithms) = unit.algorithms() {
                    for algorithm in algorithms.iter() {
                        println!("{:indent$}{}", "", algorithm, indent = indent + 2);
                    }
                }
            }
        }
    }
}

/// Dumps the contents of a UAC2 Effect Unit Descriptor
fn dump_audio_effect_unit2(unit: &audio::EffectUnit2, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value(unit.effect_type, "wEffectType", indent, width);
    dump_value(unit.source_id, "bSourceID", indent, width);
    dump_bitmap_array(&unit.controls, "bmaControls", indent, width);
    dump_value(unit.effect_index, "iEffects", indent, width);
    dump_value_string(
        unit.effect_index,
        "iEffects",
        unit.effect.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

/// Dumps the contents of a UAC3 Effect Unit Descriptor
fn dump_audio_effect_unit3(unit: &audio::EffectUnit3, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value(unit.effect_type, "wEffectType", indent, width);
    dump_value(unit.source_id, "bSourceID", indent, width);
    dump_bitmap_array(&unit.controls, "bmaControls", indent, width);
    dump_value(unit.effect_descr_str, "wEffectsDescrStr", indent, width);
}

/// Dumps the contents of a UAC1 Feature Unit Descriptor
fn dump_audio_feature_unit1(unit: &audio::FeatureUnit1, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value(unit.source_id, "bSourceID", indent, width);
    dump_value(unit.control_size, "bControlSize", indent, width);
    dump_bitmap_controls_array(
        "bmaControls",
        &unit.controls,
        &UAC1_FEATURE_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl1,
        indent,
        width,
    );
    dump_value_string(
        unit.feature_index,
        "iFeature",
        unit.feature.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

/// Dumps the contents of a UAC2 Feature Unit Descriptor
fn dump_audio_feature_unit2(unit: &audio::FeatureUnit2, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value(unit.source_id, "bSourceID", indent, width);
    dump_bitmap_controls_array(
        "bmaControls",
        &unit.controls,
        &UAC1_FEATURE_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl1,
        indent,
        width,
    );
    dump_value_string(
        unit.feature_index,
        "iFeature",
        unit.feature.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

/// Dumps the contents of a UAC3 Feature Unit Descriptor
fn dump_audio_feature_unit3(unit: &audio::FeatureUnit3, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value(unit.source_id, "bSourceID", indent, width);
    dump_bitmap_controls_array(
        "bmaControls",
        &unit.controls,
        &UAC1_FEATURE_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl1,
        indent,
        width,
    );
    dump_value(unit.feature_descr_str, "wFeatureDescrStr", indent, width);
}

/// Dumps the contents of a UAC1 Extension Unit Descriptor
fn dump_audio_extension_unit1(unit: &audio::ExtensionUnit1, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value(unit.extension_code, "wExtensionCode", indent, width);
    dump_value(unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&unit.source_ids, "baSourceID", indent, width);
    dump_value(unit.nr_channels, "bNrChannels", indent, width);
    dump_hex(unit.channel_config, "wChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac1,
        unit.channel_config as u32,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value(unit.channel_names_index, "iChannelNames", indent, width);
    dump_value_string(
        unit.channel_names_index,
        "iChannelNames",
        unit.channel_names.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
    dump_value(unit.control_size, "bControlSize", indent, width);
    dump_bitmap_array(&unit.controls, "bmControls", indent, width);
    dump_value_string(
        unit.extension_index,
        "iExtension",
        unit.extension.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

/// Dumps the contents of a UAC2 Extension Unit Descriptor
fn dump_audio_extension_unit2(unit: &audio::ExtensionUnit2, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value(unit.extension_code, "wExtensionCode", indent, width);
    dump_value(unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&unit.source_ids, "baSourceID", indent, width);
    dump_value(unit.nr_channels, "bNrChannels", indent, width);
    dump_hex(unit.channel_config, "bmChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac2,
        unit.channel_config,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value_string(
        unit.channel_names_index,
        "iChannelNames",
        unit.channel_names.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
    dump_hex(unit.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        unit.controls,
        &UAC2_EXTENSION_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value_string(
        unit.extension_index,
        "iExtension",
        unit.extension.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

/// Dumps the contents of a UAC3 Extension Unit Descriptor
fn dump_audio_extension_unit3(unit: &audio::ExtensionUnit3, indent: usize, width: usize) {
    dump_value(unit.unit_id, "bUnitID", indent, width);
    dump_value(unit.extension_code, "wExtensionCode", indent, width);
    dump_value(unit.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&unit.source_ids, "baSourceID", indent, width);
    dump_value(
        unit.extension_descr_str,
        "wExtensionDescrStr",
        indent,
        width,
    );
    dump_hex(unit.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        unit.controls,
        &UAC3_EXTENSION_UNIT_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(unit.cluster_descr_id, "wClusterDescrID", indent, width);
}

/// Dumps the contents of a UAC2 Clock Source Descriptor
fn dump_audio_clock_source2(source: &audio::ClockSource2, indent: usize, width: usize) {
    let uac2_clk_src_bmattr = |index: usize| -> Option<&'static str> {
        match index {
            0 => Some("External"),
            1 => Some("Internal fixed"),
            2 => Some("Internal variable"),
            3 => Some("Internal programmable"),
            _ => None,
        }
    };

    dump_value(source.clock_id, "bClockID", indent, width);
    dump_hex(source.attributes, "bmAttributes", indent, width);
    dump_bitmap_strings(source.attributes, uac2_clk_src_bmattr, indent + 2);
    dump_hex(source.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        source.controls,
        &UAC2_CLOCK_SOURCE_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(source.assoc_terminal, "bAssocTerminal", indent, width);
    dump_value_string(
        source.clock_source_index,
        "iClockSource",
        source.clock_source.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

/// Dumps the contents of a UAC3 Clock Source Descriptor
fn dump_audio_clock_source3(source: &audio::ClockSource3, indent: usize, width: usize) {
    let uac3_clk_src_bmattr = |index: usize| -> Option<&'static str> {
        match index {
            0 => Some("External"),
            1 => Some("Internal"),
            2 => Some("(asynchronous)"),
            3 => Some("(synchronized to SOF)"),
            _ => None,
        }
    };

    dump_value(source.clock_id, "bClockID", indent, width);
    dump_hex(source.attributes, "bmAttributes", indent, width);
    dump_bitmap_strings(source.attributes, uac3_clk_src_bmattr, indent + 2);
    dump_hex(source.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        source.controls,
        &UAC2_CLOCK_SOURCE_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(
        source.reference_terminal,
        "bReferenceTerminal",
        indent,
        width,
    );
    dump_value(source.clock_source_str, "wClockSourceStr", indent, width);
}

/// Dumps the contents of a UAC2 Clock Selector Descriptor
fn dump_audio_clock_selector2(selector: &audio::ClockSelector2, indent: usize, width: usize) {
    dump_value(selector.clock_id, "bClockID", indent, width);
    dump_value(selector.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&selector.csource_ids, "baCSourceID", indent, width);
    dump_hex(selector.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        selector.controls,
        &UAC2_CLOCK_SELECTOR_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value_string(
        selector.clock_selector_index,
        "iClockSelector",
        selector.clock_selector.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

/// Dumps the contents of a UAC3 Clock Selector Descriptor
fn dump_audio_clock_selector3(selector: &audio::ClockSelector3, indent: usize, width: usize) {
    dump_value(selector.clock_id, "bClockID", indent, width);
    dump_value(selector.nr_in_pins, "bNrInPins", indent, width);
    dump_array(&selector.csource_ids, "baCSourceID", indent, width);
    dump_hex(selector.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        selector.controls,
        &UAC2_CLOCK_SELECTOR_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(
        selector.cselector_descr_str,
        "wCSelectorDescrStr",
        indent,
        width,
    );
}

/// Dumps the contents of a UAC2 Clock Multiplier Descriptor
fn dump_audio_clock_multiplier2(multiplier: &audio::ClockMultiplier2, indent: usize, width: usize) {
    dump_value(multiplier.clock_id, "bClockID", indent, width);
    dump_value(multiplier.csource_id, "bCSourceID", indent, width);
    dump_hex(multiplier.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        multiplier.controls,
        &UAC2_CLOCK_MULTIPLIER_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value_string(
        multiplier.clock_multiplier_index,
        "iClockMultiplier",
        multiplier.clock_multiplier.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

/// Dumps the contents of a UAC3 Clock Multiplier Descriptor
fn dump_audio_clock_multiplier3(multiplier: &audio::ClockMultiplier3, indent: usize, width: usize) {
    dump_value(multiplier.clock_id, "bClockID", indent, width);
    dump_value(multiplier.csource_id, "bCSourceID", indent, width);
    dump_hex(multiplier.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        multiplier.controls,
        &UAC2_CLOCK_MULTIPLIER_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(
        multiplier.cmultiplier_descr_str,
        "wCMultiplierDescrStr",
        indent,
        width,
    );
}

fn dump_audio_sample_rate_converter2(
    converter: &audio::SampleRateConverter2,
    indent: usize,
    width: usize,
) {
    dump_value(converter.unit_id, "bUnitID", indent, width);
    dump_value(converter.source_id, "bSourceID", indent, width);
    dump_value(converter.csource_in_id, "bCSourceInID", indent, width);
    dump_value(converter.csource_out_id, "bCSourceOutID", indent, width);
    dump_value_string(
        converter.src_index,
        "iSRC",
        converter.src.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

fn dump_audio_sample_rate_converter3(
    converter: &audio::SampleRateConverter3,
    indent: usize,
    width: usize,
) {
    dump_value(converter.unit_id, "bUnitID", indent, width);
    dump_value(converter.source_id, "bSourceID", indent, width);
    dump_value(converter.csource_in_id, "bCSourceInID", indent, width);
    dump_value(converter.csource_out_id, "bCSourceOutID", indent, width);
    dump_value(converter.src_descr_str, "wSRCDescrStr", indent, width);
}

fn dump_audio_header1(header: &audio::Header1, indent: usize, width: usize) {
    dump_value(header.version, "bcdADC", indent, width);
    dump_value(header.total_length, "wTotalLength", indent, width);
    dump_value(header.collection_bytes, "bInCollection", indent, width);
    dump_array(&header.interfaces, "baInterfaceNr", indent, width);
}

fn dump_audio_header2(header: &audio::Header2, indent: usize, width: usize) {
    dump_value(header.version, "bcdADC", indent, width);
    dump_value(header.total_length, "wTotalLength", indent, width);
    dump_hex(header.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        header.controls as u32,
        &UAC2_INTERFACE_HEADER_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
}

fn dump_audio_header3(header: &audio::Header3, indent: usize, width: usize) {
    dump_value(header.category, "bCategory", indent, width);
    dump_value(header.total_length, "wTotalLength", indent, width);
    dump_hex(header.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        header.controls,
        &UAC2_INTERFACE_HEADER_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
}

fn dump_audio_input_terminal1(ait: &audio::InputTerminal1, indent: usize, width: usize) {
    dump_value(ait.terminal_id, "bTerminalID", indent, width);
    println!(
        "{:indent$}wTerminalType      {:5} {}",
        "",
        ait.terminal_type,
        names::videoterminal(ait.terminal_type).unwrap_or_default(),
        indent = indent
    );
    dump_value(ait.assoc_terminal, "bAssocTerminal", indent, width);
    dump_value(ait.nr_channels, "bNrChannels", indent, width);
    dump_hex(ait.channel_config, "wChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac1,
        ait.channel_config as u32,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value_string(
        ait.channel_names_index,
        "iChannelNames",
        ait.channel_names.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
    dump_value_string(
        ait.terminal_index,
        "iTerminal",
        ait.terminal.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

fn dump_audio_input_terminal2(ait: &audio::InputTerminal2, indent: usize, width: usize) {
    dump_value(ait.terminal_id, "bTerminalID", indent, width);
    dump_name(
        ait.terminal_type,
        names::videoterminal,
        "wTerminalType",
        indent,
        width,
    );
    dump_value(ait.assoc_terminal, "bAssocTerminal", indent, width);
    dump_value(ait.nr_channels, "bNrChannels", indent, width);
    dump_hex(ait.channel_config, "wChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac2,
        ait.channel_config,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value_string(
        ait.channel_names_index,
        "iChannelNames",
        ait.channel_names.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
    dump_hex(ait.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        ait.controls,
        &UAC2_INPUT_TERMINAL_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(ait.terminal_index, "iTerminal", indent, width);
    dump_value_string(
        ait.terminal_index,
        "iTerminal",
        ait.terminal.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

fn dump_audio_input_terminal3(ait: &audio::InputTerminal3, indent: usize, width: usize) {
    dump_value(ait.terminal_id, "bTerminalID", indent, width);
    dump_name(
        ait.terminal_type,
        names::videoterminal,
        "wTerminalType",
        indent,
        width,
    );
    dump_value(ait.assoc_terminal, "bAssocTerminal", indent, width);
    dump_value(ait.csource_id, "bCSourceID", indent, width);
    dump_hex(ait.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        ait.controls,
        &UAC3_INPUT_TERMINAL_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(ait.cluster_descr_id, "wClusterDescrID", indent, width);
    dump_value(
        ait.ex_terminal_descr_id,
        "wExTerminalDescrID",
        indent,
        width,
    );
    dump_value(ait.connectors_descr_id, "wConnectorDescrId", indent, width);
    dump_value(ait.terminal_descr_str, "wTerminalDescrStr", indent, width);
}

pub(crate) fn dump_audio_output_terminal1(a: &audio::OutputTerminal1, indent: usize, width: usize) {
    dump_value(a.terminal_id, "bTerminalID", indent, width);
    dump_name(
        a.terminal_type,
        names::videoterminal,
        "wTerminalType",
        indent,
        width,
    );
    dump_value(a.assoc_terminal, "bAssocTerminal", indent, width);
    dump_value(a.source_id, "bSourceID", indent, width);
    dump_value_string(
        a.terminal_index,
        "iTerminal",
        a.terminal.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

fn dump_audio_output_terminal2(a: &audio::OutputTerminal2, indent: usize, width: usize) {
    dump_value(a.terminal_id, "bTerminalID", indent, width);
    dump_name(
        a.terminal_type,
        names::videoterminal,
        "wTerminalType",
        indent,
        width,
    );
    dump_value(a.assoc_terminal, "bAssocTerminal", indent, width);
    dump_value(a.source_id, "bSourceID", indent, width);
    dump_hex(a.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        a.controls,
        &UAC2_OUTPUT_TERMINAL_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value_string(
        a.terminal_index,
        "iTerminal",
        a.terminal.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

fn dump_audio_output_terminal3(a: &audio::OutputTerminal3, indent: usize, width: usize) {
    dump_value(a.terminal_id, "bTerminalID", indent, width);
    dump_name(
        a.terminal_type,
        names::videoterminal,
        "wTerminalType",
        indent,
        width,
    );
    dump_value(a.assoc_terminal, "bAssocTerminal", indent, width);
    dump_value(a.c_source_id, "bCSourceID", indent, width);
    dump_hex(a.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        a.controls,
        &UAC3_OUTPUT_TERMINAL_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(a.ex_terminal_descr_id, "wExTerminalDescrID", indent, width);
    dump_value(a.connectors_descr_id, "wConnectorDescrId", indent, width);
    dump_value(a.terminal_descr_str, "wTerminalDescrStr", indent, width);
}

fn dump_extended_terminal_header(d: &audio::ExtendedTerminalHeader, indent: usize, width: usize) {
    dump_value(d.descriptor_id, "wDescriptorID", indent, width);
    dump_value(d.nr_channels, "bNrChannels", indent, width);
}

fn dump_audio_streaming_interface1(asi: &audio::StreamingInterface1, indent: usize, width: usize) {
    dump_value(asi.terminal_link, "bTerminalLink", indent, width);
    dump_value(asi.delay, "bDelay", indent, width);
    dump_value(asi.format_tag, "wFormatTag", indent, width);
}

fn dump_audio_streaming_interface2(asi: &audio::StreamingInterface2, indent: usize, width: usize) {
    dump_value(asi.terminal_link, "bTerminalLink", indent, width);
    dump_hex(asi.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        asi.controls,
        &UAC2_AS_INTERFACE_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(asi.format_type, "bFormatType", indent, width);
    dump_value(asi.nr_channels, "bNrChannels", indent, width);
    dump_hex(asi.channel_config, "bmChannelConfig", indent, width);
    let channel_names = audio::UacInterfaceDescriptor::get_channel_name_strings(
        &audio::UacProtocol::Uac2,
        asi.channel_config,
    );
    for name in channel_names.iter() {
        println!("{:indent$}{}", "", name, indent = indent + 2);
    }
    dump_value_string(
        asi.channel_names_index,
        "iChannelNames",
        asi.channel_names.as_ref().unwrap_or(&"".into()),
        indent,
        width,
    );
}

fn dump_audio_streaming_interface3(asi: &audio::StreamingInterface3, indent: usize, width: usize) {
    dump_value(asi.terminal_link, "bTerminalLink", indent, width);
    dump_hex(asi.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        asi.controls,
        &UAC3_AS_INTERFACE_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(asi.cluster_descr_id, "wClusterDescrID", indent, width);
    dump_hex(asi.formats, "bmFormats", indent, width);
    dump_value(asi.sub_slot_size, "bSubslotSize", indent, width);
    dump_value(asi.bit_resolution, "bBitResolution", indent, width);
    dump_hex(asi.aux_protocols, "bmAuxProtocols", indent, width);
    dump_value(asi.control_size, "bControlSize", indent, width);
}

fn dump_audio_data_streaming_endpoint1(
    ads: &audio::DataStreamingEndpoint1,
    indent: usize,
    width: usize,
) {
    let uac1_attrs = |a: usize| match a {
        0 => Some("Sampling Frequency"),
        1 => Some("Pitch"),
        2 => Some("Audio Data Format Control"),
        7 => Some("MaxPacketsOnly"),
        _ => None,
    };
    dump_hex(ads.attributes, "bmAttributes", indent, width);
    dump_bitmap_strings(ads.attributes, uac1_attrs, indent + 2);
    dump_value(ads.lock_delay_units, "bLockDelayUnits", indent, width);
    dump_value(ads.lock_delay, "wLockDelay", indent, width);
}

fn dump_audio_data_streaming_endpoint2(
    ads: &audio::DataStreamingEndpoint2,
    indent: usize,
    width: usize,
) {
    let uac2_attrs = |attr: usize| match attr {
        0x07 => Some("MaxPacketsOnly"),
        _ => None,
    };
    dump_hex(ads.attributes, "bmAttributes", indent, width);
    dump_bitmap_strings(ads.attributes, uac2_attrs, indent + 2);
    dump_hex(ads.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        ads.controls,
        &UAC2_AS_ISO_ENDPOINT_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(ads.lock_delay_units, "bLockDelayUnits", indent, width);
    dump_value(ads.lock_delay, "wLockDelay", indent, width);
}

fn dump_audio_data_streaming_endpoint3(
    ads: &audio::DataStreamingEndpoint3,
    indent: usize,
    width: usize,
) {
    dump_hex(ads.controls, "bmControls", indent, width);
    dump_bitmap_controls(
        ads.controls,
        &UAC2_AS_ISO_ENDPOINT_BMCONTROLS,
        &audio::ControlType::BmControl2,
        indent + 2,
    );
    dump_value(ads.lock_delay_units, "bLockDelayUnits", indent, width);
    dump_value(ads.lock_delay, "wLockDelay", indent, width);
}

fn dump_audio_streaming_format(af: &audio::StreamingFormat, indent: usize, width: usize) {
    dump_value_string(
        u8::from(af.format_type.to_owned()),
        "bFormatType",
        format!("({:#})", af.format_type),
        indent,
        width,
    );

    match &af.interface {
        audio::StreamingFormatInterface::FormatTypeI1(asi) => {
            dump_format_type_i(asi, indent, width);
        }
        audio::StreamingFormatInterface::FormatTypeII1(asi) => {
            dump_format_type_ii(asi, indent, width);
        }
        audio::StreamingFormatInterface::FormatTypeIII1(asi) => {
            dump_format_type_iii(asi, indent, width);
        }
        audio::StreamingFormatInterface::FormatTypeI2(asi) => {
            dump_format_type_i_uac2(asi, indent, width);
        }
        audio::StreamingFormatInterface::FormatTypeII2(asi) => {
            dump_format_type_ii_uac2(asi, indent, width);
        }
        audio::StreamingFormatInterface::FormatTypeIII2(asi) => {
            dump_format_type_iii_uac2(asi, indent, width);
        }
        i => {
            let data: Vec<u8> = i.to_owned().into();
            println!(
                "{:indent$}invalid desc format type: {}",
                "",
                data.iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<String>>()
                    .join(""),
                indent = indent
            )
        }
    }
}

fn dump_audio_streaming_format_specific(
    af: &audio::StreamingFormatSpecific,
    indent: usize,
    width: usize,
) {
    let fmtptr = get_format_specific_string(af.format_tag);
    dump_value_string(af.format_tag, "wFormatTag", fmtptr, indent, width);

    match &af.interface {
        audio::StreamingFormatInterface::FormatSpecificAc3(fs) => {
            dump_format_specific_ac3(fs, indent, width);
        }
        audio::StreamingFormatInterface::FormatSpecificMpeg(fs) => {
            dump_format_specific_mpeg(fs, indent, width);
        }
        i => {
            let data: Vec<u8> = i.to_owned().into();
            println!(
                "{:indent$}Invalid desc format type: {}",
                "",
                data.iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<String>>()
                    .join(""),
                indent = indent
            )
        }
    }
}

fn dump_audio_subtype(uacid: &audio::UacInterfaceDescriptor, indent: usize) {
    match uacid {
        audio::UacInterfaceDescriptor::Header1(a) => {
            dump_audio_header1(a, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::Header2(ach) => {
            dump_audio_header2(ach, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::Header3(ach) => {
            dump_audio_header3(ach, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::InputTerminal1(ait) => {
            dump_audio_input_terminal1(ait, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::InputTerminal2(ait) => {
            dump_audio_input_terminal2(ait, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::InputTerminal3(ait) => {
            dump_audio_input_terminal3(ait, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::OutputTerminal1(a) => {
            dump_audio_output_terminal1(a, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::OutputTerminal2(a) => {
            dump_audio_output_terminal2(a, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::OutputTerminal3(a) => {
            dump_audio_output_terminal3(a, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ExtendedTerminalHeader(d) => {
            dump_extended_terminal_header(d, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::PowerDomain(power_domain) => {
            dump_audio_power_domain(power_domain, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::MixerUnit1(mixer_unit) => {
            dump_audio_mixer_unit1(mixer_unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::MixerUnit2(mixer_unit) => {
            dump_audio_mixer_unit2(mixer_unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::MixerUnit3(mixer_unit) => {
            dump_audio_mixer_unit3(mixer_unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::SelectorUnit1(selector_unit) => {
            dump_audio_selector_unit1(selector_unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::SelectorUnit2(selector_unit) => {
            dump_audio_selector_unit2(selector_unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::SelectorUnit3(selector_unit) => {
            dump_audio_selector_unit3(selector_unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ProcessingUnit1(unit) => {
            dump_audio_processing_unit1(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ProcessingUnit2(unit) => {
            dump_audio_processing_unit2(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ProcessingUnit3(unit) => {
            dump_audio_processing_unit3(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::EffectUnit2(unit) => {
            dump_audio_effect_unit2(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::EffectUnit3(unit) => {
            dump_audio_effect_unit3(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::FeatureUnit1(unit) => {
            dump_audio_feature_unit1(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::FeatureUnit2(unit) => {
            dump_audio_feature_unit2(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::FeatureUnit3(unit) => {
            dump_audio_feature_unit3(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ExtensionUnit1(unit) => {
            dump_audio_extension_unit1(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ExtensionUnit2(unit) => {
            dump_audio_extension_unit2(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ExtensionUnit3(unit) => {
            dump_audio_extension_unit3(unit, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ClockSource2(source) => {
            dump_audio_clock_source2(source, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ClockSource3(source) => {
            dump_audio_clock_source3(source, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ClockSelector2(selector) => {
            dump_audio_clock_selector2(selector, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ClockSelector3(selector) => {
            dump_audio_clock_selector3(selector, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ClockMultiplier2(multiplier) => {
            dump_audio_clock_multiplier2(multiplier, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::ClockMultiplier3(multiplier) => {
            dump_audio_clock_multiplier3(multiplier, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::SampleRateConverter2(converter) => {
            dump_audio_sample_rate_converter2(converter, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::SampleRateConverter3(converter) => {
            dump_audio_sample_rate_converter3(converter, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::StreamingInterface1(asi) => {
            dump_audio_streaming_interface1(asi, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::StreamingInterface2(asi) => {
            dump_audio_streaming_interface2(asi, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::StreamingInterface3(asi) => {
            dump_audio_streaming_interface3(asi, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::StreamingFormat(fmt) => {
            dump_audio_streaming_format(fmt, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::StreamingFormatSpecific(fmt) => {
            dump_audio_streaming_format_specific(fmt, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::DataStreamingEndpoint1(ads) => {
            dump_audio_data_streaming_endpoint1(ads, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::DatastreamingEndpoint2(ads) => {
            dump_audio_data_streaming_endpoint2(ads, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::DataStreamingEndpoint3(ads) => {
            dump_audio_data_streaming_endpoint3(ads, indent, LSUSB_DUMP_WIDTH);
        }
        audio::UacInterfaceDescriptor::Undefined(data)
        | audio::UacInterfaceDescriptor::Invalid(data) => {
            println!(
                "{:indent$}Invalid desc subtype: {}",
                "",
                data.iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<String>>()
                    .join(" "),
            );
        }
        _ => {
            log::warn!("Unsupported UAC interface descriptor: {uacid:?}");
        }
    }
}

pub(crate) fn dump_audiocontrol_interface(
    uacd: &audio::UacDescriptor,
    uaci: &audio::ControlSubtype,
    protocol: &audio::UacProtocol,
    indent: usize,
) {
    dump_string("AudioControl Interface Descriptor", indent);
    dump_value(uacd.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        uacd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        uaci.to_owned() as u8,
        "bDescriptorSubtype",
        format!("({uaci:#})"),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    match &uacd.interface {
        audio::UacInterfaceDescriptor::Invalid(_) => {
            println!(
                "{:indent$}Warning: {:#} descriptors are illegal for {}",
                "",
                uacd.descriptor_subtype,
                u8::from(protocol.to_owned()),
                indent = indent
            );
        }
        uacid => dump_audio_subtype(uacid, indent + 2),
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

fn dump_format_type_i(ft: &audio::FormatTypeI1, indent: usize, width: usize) {
    dump_value(ft.num_channels, "bNrChannels", indent, width);
    dump_value(ft.subframe_size, "bSubframeSize", indent, width);
    dump_value(ft.bit_resolution, "bBitResolution", indent, width);
    dump_value_string(
        u8::from(ft.sample_frequency_type.to_owned()),
        "bSamFreqType",
        ft.sample_frequency_type.to_string(),
        indent,
        width,
    );
    match ft.sample_frequency_type {
        audio::SampleFrequencyType::Continuous => {
            dump_value(
                ft.sample_frequencies.first().unwrap_or(&0),
                "tLowerSamFreq",
                indent,
                width,
            );
            dump_value(
                ft.sample_frequencies.get(1).unwrap_or(&0),
                "tUpperSamFreq",
                indent,
                width,
            );
        }
        audio::SampleFrequencyType::Discrete(n) => {
            for i in 0..n {
                dump_value(
                    ft.sample_frequencies[i as usize],
                    &format!("tSamFreq[{i}]"),
                    indent,
                    width,
                );
            }
        }
    }
}

fn dump_format_type_ii(ft: &audio::FormatTypeII1, indent: usize, width: usize) {
    dump_value(ft.max_bit_rate, "wMaxBitRate", indent, width);
    dump_value(ft.samples_per_frame, "wSamplesPerFrame", indent, width);
    dump_value_string(
        u8::from(ft.sample_frequency_type.to_owned()),
        "bSamFreqType",
        ft.sample_frequency_type.to_string(),
        indent,
        width,
    );
    match ft.sample_frequency_type {
        audio::SampleFrequencyType::Continuous => {
            dump_value(
                ft.sample_frequencies.first().unwrap_or(&0),
                "tLowerSamFreq",
                indent,
                width,
            );
            dump_value(
                ft.sample_frequencies.get(1).unwrap_or(&0),
                "tUpperSamFreq",
                indent,
                width,
            );
        }
        audio::SampleFrequencyType::Discrete(n) => {
            for i in 0..n {
                dump_value(
                    ft.sample_frequencies[i as usize],
                    &format!("tSamFreq[{i}]"),
                    indent,
                    width,
                );
            }
        }
    }
}

fn dump_format_type_iii(ft: &audio::FormatTypeIII1, indent: usize, width: usize) {
    dump_value(ft.num_channels, "bNrChannels", indent, width);
    dump_value(ft.subframe_size, "bSubframeSize", indent, width);
    dump_value(ft.bit_resolution, "bBitResolution", indent, width);
    dump_value_string(
        u8::from(ft.sample_frequency_type.to_owned()),
        "bSamFreqType",
        ft.sample_frequency_type.to_string(),
        indent,
        width,
    );
    match ft.sample_frequency_type {
        audio::SampleFrequencyType::Continuous => {
            dump_value(
                ft.sample_frequencies.first().unwrap_or(&0),
                "tLowerSamFreq",
                indent,
                width,
            );
            dump_value(
                ft.sample_frequencies.get(1).unwrap_or(&0),
                "tUpperSamFreq",
                indent,
                width,
            );
        }
        audio::SampleFrequencyType::Discrete(n) => {
            for i in 0..n {
                dump_value(
                    ft.sample_frequencies[i as usize],
                    &format!("tSamFreq[{i}]"),
                    indent,
                    width,
                );
            }
        }
    }
}

fn dump_format_type_i_uac2(ft: &audio::FormatTypeI2, indent: usize, width: usize) {
    dump_value(ft.sub_slot_size, "bSubslotSize", indent, width);
    dump_value(ft.bit_resolution, "bBitResolution", indent, width);
}

fn dump_format_type_ii_uac2(ft: &audio::FormatTypeII2, indent: usize, width: usize) {
    dump_value(ft.max_bit_rate, "wMaxBitRate", indent, width);
    dump_value(ft.slots_per_frame, "wSlotsPerFrame", indent, width);
}

fn dump_format_type_iii_uac2(ft: &audio::FormatTypeIII2, indent: usize, width: usize) {
    dump_value(ft.sub_slot_size, "bSubslotSize", indent, width);
    dump_value(ft.bit_resolution, "bBitResolution", indent, width);
}

fn dump_format_specific_mpeg(fs: &audio::FormatSpecificMpeg, indent: usize, width: usize) {
    dump_hex(fs.mpeg_capabilities, "bmMPEGCapabilities", indent, width);
    dump_bitmap_strings(
        fs.mpeg_capabilities as u8,
        |b| match b {
            0 => Some("Layer I"),
            1 => Some("Layer II"),
            2 => Some("Layer III"),
            3 => Some("MPEG-1 only"),
            4 => Some("MPEG-1 dual-channel"),
            5 => Some("MPEG-2 second stereo"),
            6 => Some("MPEG-2 7.1 channel augmentation"),
            7 => Some("Adaptive multi-channel prediction"),
            _ => None,
        },
        indent + 2,
    );
    println!(
        "{:indent$}MPEG-2 multilingual support: {}",
        "",
        match (fs.mpeg_capabilities >> 8) & 3 {
            0 => "Not supported",
            1 => "Supported at Fs",
            2 => "Reserved",
            _ => "Supported at Fs and 1/2Fs",
        },
        indent = indent + 2
    );
    dump_hex(fs.mpeg_features, "bmMPEGFeatures", indent, width);
    println!(
        "{:indent$}Internal Dynamic Range Control: {}",
        "",
        match (fs.mpeg_features >> 4) & 3 {
            0 => "not supported",
            1 => "supported but not scalable",
            2 => "scalable, common boost and cut scaling value",
            _ => "scalable, separate boost and cut scaling value",
        },
        indent = indent + 2
    );
}

fn dump_format_specific_ac3(fs: &audio::FormatSpecificAc3, indent: usize, width: usize) {
    dump_hex(fs.bsid, "bmBSID", indent, width);
    dump_hex(fs.ac3_features, "bmAC3Features", indent, width);
    dump_bitmap_strings(
        fs.ac3_features,
        |b| match b {
            0 => Some("RF mode"),
            1 => Some("Line mode"),
            2 => Some("Custom0 mode"),
            3 => Some("Custom1 mode"),
            _ => None,
        },
        indent + 2,
    );
    println!(
        "{:indent$}Internal Dynamic Range Control: {}",
        "",
        match (fs.ac3_features >> 4) & 3 {
            0 => "not supported",
            1 => "supported but not scalable",
            2 => "scalable, common boost and cut scaling value",
            _ => "scalable, separate boost and cut scaling value",
        },
        indent = indent + 2
    );
}

pub(crate) fn dump_audiostreaming_interface(
    uacd: &audio::UacDescriptor,
    uasi: &audio::StreamingSubtype,
    protocol: &audio::UacProtocol,
    indent: usize,
) {
    dump_string("AudioStreaming Interface Descriptor:", indent);
    dump_value(uacd.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        uacd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        uasi.to_owned() as u8,
        "bDescriptorSubtype",
        format!("({uasi:#})"),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    match &uacd.interface {
        audio::UacInterfaceDescriptor::Invalid(_) => {
            println!(
                "{:indent$}Warning: {:#} descriptors are illegal for {}",
                "",
                uacd.descriptor_subtype,
                u8::from(protocol.to_owned()),
                indent = indent + 2
            );
        }
        uacid => dump_audio_subtype(uacid, indent + 2),
    }
}

pub(crate) fn dump_audiostreaming_endpoint(ad: &audio::UacDescriptor, indent: usize) {
    // audio streaming endpoint is only EP_GENERAL
    let subtype_string = match ad.descriptor_subtype {
        audio::UacType::Streaming(audio::StreamingSubtype::General) => "EP_GENERAL",
        // lowercase in lsusb
        _ => "invalid",
    };
    dump_string("AudioStreaming Endpoint Descriptor:", indent);
    dump_value(ad.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        ad.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        u8::from(ad.descriptor_subtype.to_owned()),
        "bDescriptorSubtype",
        format!("({subtype_string:#})"),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    if matches!(
        ad.descriptor_subtype,
        audio::UacType::Streaming(audio::StreamingSubtype::General)
    ) {
        dump_audio_subtype(&ad.interface, indent + 2);
    }
}

pub(crate) fn dump_midistreaming_interface(md: &audio::MidiDescriptor, indent: usize) {
    let jack_types = |t: u8| match t {
        0x00 => "Undefined",
        0x01 => "Embedded",
        0x02 => "External",
        _ => "Invalid",
    };

    dump_string("MIDIStreaming Interface Descriptor:", indent);
    dump_value(md.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        md.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        md.descriptor_subtype.to_owned() as u8,
        "bDescriptorSubtype",
        format!("({:#})", md.descriptor_subtype),
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    match &md.interface {
        audio::MidiInterfaceDescriptor::Header(d) => {
            dump_value(d.version, "bcdADC", indent + 2, LSUSB_DUMP_WIDTH);
            dump_hex(d.total_length, "wTotalLength", indent + 2, LSUSB_DUMP_WIDTH);
        }
        audio::MidiInterfaceDescriptor::InputJack(d) => {
            dump_value_string(
                d.jack_type,
                "bJackType",
                jack_types(d.jack_type),
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_value(d.jack_id, "bJackID", indent + 2, LSUSB_DUMP_WIDTH);
            dump_value_string(
                d.jack_string_index,
                "iJack",
                d.jack_string.as_ref().unwrap_or(&"".into()),
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
        }
        audio::MidiInterfaceDescriptor::OutputJack(d) => {
            dump_value_string(
                d.jack_type,
                "bJackType",
                jack_types(d.jack_type),
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_value(d.jack_id, "bJackID", indent + 2, LSUSB_DUMP_WIDTH);
            dump_value(
                d.num_input_pins,
                "bNrInputPins",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );

            for (i, p) in d.source_ids.iter() {
                dump_value(
                    i,
                    &format!("baSourceID({i:2})"),
                    indent + 2,
                    LSUSB_DUMP_WIDTH,
                );
                dump_value(
                    p,
                    &format!("baSourcePin({i:2})"),
                    indent + 2,
                    LSUSB_DUMP_WIDTH,
                );
            }
            dump_value_string(
                d.jack_string_index,
                "iJack",
                d.jack_string.as_ref().unwrap_or(&"".into()),
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
        }
        audio::MidiInterfaceDescriptor::Element(d) => {
            dump_value(d.element_id, "bElementID", indent + 2, LSUSB_DUMP_WIDTH);
            dump_value(
                d.num_input_pins,
                "bNrInputPins",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );

            for (i, p) in d.source_ids.iter() {
                dump_value(
                    i,
                    &format!("baSourceID({i:2})"),
                    indent + 2,
                    LSUSB_DUMP_WIDTH,
                );
                dump_value(
                    p,
                    &format!("baSourcePin({i:2})"),
                    indent + 2,
                    LSUSB_DUMP_WIDTH,
                );
            }

            dump_value(
                d.num_output_pins,
                "bNrOutputPins",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_value(
                d.in_terminal_link,
                "bInTerminalLink",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_value(
                d.out_terminal_link,
                "bOutTerminalLink",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_value(d.el_caps_size, "bElCapsSize", indent + 2, LSUSB_DUMP_WIDTH);
            dump_hex(
                d.element_caps,
                "bmElementCaps",
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
            dump_bitmap_strings(
                d.element_caps,
                |b| match b {
                    0 => Some("Undefined"),
                    1 => Some("MIDI Clock"),
                    2 => Some("MTC (MIDI Time Code)"),
                    3 => Some("MMC (MIDI Machine Control)"),
                    4 => Some("GM1 (General MIDI v.1)"),
                    5 => Some("GM2 (General MIDI v.2)"),
                    6 => Some("GS MIDI Extension"),
                    7 => Some("XG MIDI Extension"),
                    8 => Some("EFX"),
                    9 => Some("MIDI Patch Bay"),
                    10 => Some("DLS1 (Downloadable Sounds Level 1)"),
                    11 => Some("DLS2 (Downloadable Sounds Level 2)"),
                    _ => None,
                },
                indent + 2,
            );

            dump_value_string(
                d.element_string_index,
                "iElement",
                d.element_string.as_ref().unwrap_or(&String::new()),
                indent + 2,
                LSUSB_DUMP_WIDTH,
            );
        }
        audio::MidiInterfaceDescriptor::Undefined(_) => (),
        invalid => {
            let data: Vec<u8> = invalid.to_owned().into();
            println!(
                "{:indent$}Invalid desc subtype: {}",
                "",
                data.iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<String>>()
                    .join(" "),
                indent = indent + 2,
            );
        }
    }
}

pub(crate) fn dump_midistreaming_endpoint(md: &audio::MidiDescriptor, indent: usize) {
    let subtype_string = match u8::from(md.descriptor_subtype.to_owned()) {
        1 | 2 => "GENERAL",
        _ => "Invalid",
    };

    dump_string("MIDIStreaming Endpoint Descriptor:", indent);
    dump_value(md.length, "bLength", indent + 2, LSUSB_DUMP_WIDTH);
    dump_value(
        md.descriptor_type,
        "bDescriptorType",
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );
    dump_value_string(
        u8::from(md.descriptor_subtype.to_owned()),
        "bDescriptorSubtype",
        subtype_string,
        indent + 2,
        LSUSB_DUMP_WIDTH,
    );

    if let audio::MidiInterfaceDescriptor::Endpoint(ep) = &md.interface {
        dump_value(
            ep.num_jacks,
            "bNumEmbMIDIJack",
            indent + 2,
            LSUSB_DUMP_WIDTH,
        );
        dump_array(&ep.jacks, "baAssocJackID", indent + 2, LSUSB_DUMP_WIDTH);
    }
}
