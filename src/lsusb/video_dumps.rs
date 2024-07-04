use crate::usb::descriptors::video;

use super::*;

// TODO - convert these to Rust enum like [`Uac1ChannelNames`] etc.
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

fn dump_processing_unit(pu: &video::ProcessingUnit, protocol: u8, indent: usize, width: usize) {
    dump_value(pu.unit_id, "bUnitID", indent + 2, width);
    dump_value(pu.source_id, "bSourceID", indent + 2, width);
    dump_value(pu.max_multiplier, "wMaxMultiplier", indent + 2, width);
    dump_value(pu.control_size, "bControlSize", indent + 2, width);

    dump_hex(pu.controls, "bmControls", indent + 2, width);
    if protocol == 0x01 {
        for (i, n) in CTRL_NAMES.iter().enumerate().take(19) {
            if (pu.controls >> i) & 1 != 0 {
                dump_string(n, indent + 4);
            }
        }
    } else {
        for (i, n) in CTRL_NAMES.iter().enumerate().take(18) {
            if (pu.controls >> i) & 1 != 0 {
                dump_string(n, indent + 4);
            }
        }
    }

    dump_value_string(
        pu.processing_index,
        "iProcessing",
        pu.processing_index.to_string(),
        indent + 2,
        width,
    );
    dump_hex(pu.video_standards, "bmVideoStandards", indent + 2, width);
    for (i, n) in STD_NAMES.iter().enumerate().take(6) {
        if (pu.video_standards >> i) & 1 != 0 {
            dump_string(n, indent + 4);
        }
    }
}

fn dump_extension_unit(eu: &video::ExtensionUnit, indent: usize, width: usize) {
    dump_value(eu.unit_id, "bUnitID", indent + 2, width);
    dump_guid(&eu.guid_extension_code, "guidExtensionCode", indent + 2, width);
    dump_value(eu.num_controls, "bNumControls", indent + 2, width);
    dump_value(eu.num_input_pins, "bNrInPins", indent + 2, width);

    for (i, source_id) in eu.source_ids.iter().enumerate() {
        dump_value(*source_id, &format!("baSourceID({:2})", i), indent + 2, width);
    }

    dump_value(eu.control_size, "bControlSize", indent + 2, width);

    for (i, control) in eu.controls.iter().enumerate() {
        dump_hex(*control, &format!("bmControls({:2})", i), indent + 2, width);
    }

    dump_value_string(
        eu.extension_index,
        "iExtension",
        eu.extension.as_ref().unwrap_or(&String::new()),
        indent + 2,
        width,
    );
}

fn dump_encoding_unit(eu: &video::EncodingUnit, indent: usize, width: usize) {
    dump_value(eu.unit_id, "bUnitID", indent + 2, width);
    dump_value(eu.source_id, "bSourceID", indent + 2, width);
    dump_value_string(
        eu.encoding_index,
        "iEncoding",
        eu.encoding.as_ref().unwrap_or(&String::new()),
        indent + 2,
        width,
    );
    dump_value(eu.control_size, "bControlSize", indent + 2, width);

    dump_hex(eu.controls, "bmControls", indent + 2, width);
    for (i, n) in EN_CTRL_NAMES.iter().enumerate().take(20) {
        if (eu.controls >> i) & 1 != 0 {
            dump_string(n, indent + 4);
        }
    }

    dump_hex(eu.controls_runtime, "bmControlsRuntime", indent + 2, width);
    for (i, n) in EN_CTRL_NAMES.iter().enumerate().take(20) {
        if (eu.controls_runtime >> i) & 1 != 0 {
            dump_string(n, indent + 4);
        }
    }
}

pub(crate) fn dump_videocontrol_interface(vcd: &video::UvcDescriptor, vct: &video::ControlSubtype, protocol: u8, indent: usize) {
    const DUMP_WIDTH: usize = 36; // wider in lsusb for long numbers
    dump_string("VideoControl Interface Descriptor:", indent);
    dump_value(vcd.length, "bLength", indent + 2, DUMP_WIDTH);
    dump_value(
        vcd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        DUMP_WIDTH,
    );
    dump_value_string(
        vct.to_owned() as u8,
        "bDescriptorSubtype",
        format!("({:#})", vct),
        indent + 2,
        DUMP_WIDTH,
    );


    match &vcd.interface {
        video::UvcInterfaceDescriptor::Invalid(_) => {
            println!(
                "{:indent$}Warning: {:#} descriptors are illegal for {}",
                "",
                vct,
                u8::from(protocol.to_owned()),
                indent = indent
            );
        }
        uvcid => dump_video_subtype(uvcid, protocol, indent, DUMP_WIDTH),
    }
}

fn dump_video_input_header(ih: &video::InputHeader, indent: usize, width: usize) {
    dump_value(ih.num_formats, "bNumFormats", indent + 2, width);
    dump_hex(ih.total_length, "wTotalLength", indent + 2, width);
    dump_value_string(format!("0x{:02x}", ih.endpoint_address.address),
        "bEndpointAddress",
        ih.endpoint_address.to_string(),
        indent + 2,
        width,
    );
    dump_value(ih.info, "bmInfo", indent + 2, width);
    dump_value(ih.terminal_link, "bTerminalLink", indent + 2, width);
    dump_value(ih.still_capture_method, "bStillCaptureMethod", indent + 2, width);
    dump_value(ih.trigger_support, "bTriggerSupport", indent + 2, width);
    dump_value(ih.trigger_usage, "bTriggerUsage", indent + 2, width);
    dump_value(ih.control_size, "bControlSize", indent + 2, width);

    for (i, b) in ih.controls.chunks(ih.control_size as usize).enumerate() {
        dump_value(
            b[0],
            &format!("bmaControls({:2})", i),
            indent + 2,
            width,
        );
    }
}

fn dump_video_output_header(oh: &video::OutputHeader, indent: usize, width: usize) {
    dump_value(oh.num_formats, "bNumFormats", indent + 2, width);
    dump_hex(oh.total_length, "wTotalLength", indent + 2, width);
    dump_value_string(format!("0x{:02x}", oh.endpoint_address.address),
        "bEndpointAddress",
        oh.endpoint_address.to_string(),
        indent + 2,
        width,
    );
    dump_value(oh.terminal_link, "bTerminalLink", indent + 2, width);
    dump_value(oh.control_size, "bControlSize", indent + 2, width);

    for (i, b) in oh.controls.chunks(oh.control_size as usize).enumerate() {
        dump_value(
            b[0],
            &format!("bmaControls({:2})", i),
            indent + 2,
            width,
        );
    }
}

fn dump_video_color_format(cf: &video::ColorFormat, indent: usize, width: usize) {
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

    dump_value_string(
        cf.color_primaries,
        "bColorPrimaries",
        format!("({})", color_primatives(cf.color_primaries)),
        indent + 2,
        width,
    );
    dump_value_string(
        cf.transfer_characteristics,
        "bTransferCharacteristics",
        format!("({})", transfer_characteristics(cf.transfer_characteristics)),
        indent + 2,
        width,
    );
    dump_value_string(
        cf.matrix_coefficients,
        "bMatrixCoefficients",
        format!("({})", matrix_coefficients(cf.matrix_coefficients)),
        indent + 2,
        width,
    );
}

fn dump_format_stream_based(fs: &video::FormatStreamBased, indent: usize, width: usize) {
    dump_value(fs.format_index, "bFormatIndex", indent + 2, width);
    dump_guid(&fs.guid_format, "guidFormat", indent + 2, width);
    dump_value(fs.packet_length, "dwPacketLength", indent + 2, width);
}

fn dump_format_mpeg2ts(fmts: &video::FormatMPEG2TS, indent: usize, width: usize) {
    dump_value(fmts.format_index, "bFormatIndex", indent + 2, width);
    dump_value(fmts.data_offset, "bDataOffset", indent + 2, width);
    dump_value(fmts.packet_length, "bPacketLength", indent + 2, width);
    dump_value(fmts.stride_length, "bStrideLength", indent + 2, width);
    if let Some(guid) = &fmts.guid_stride_format {
        dump_guid(&guid, "guidStrideFormat", indent + 2, width);
    }
}

fn dump_interlace_flags(interlace_flags: u8, indent: usize, width: usize) {
    let field_pattern = |f: u8| -> &'static str {
        match f {
            0 => "Field 1 only",
            1 => "Field 2 only",
            2 => "Regular pattern of fields 1 and 2",
            3 => "Random pattern of fields 1 and 2",
            _ => "Invalid",
        }
    };

    dump_hex(interlace_flags, "bmInterlaceFlags", indent, width);
    dump_string(
        &format!(
            "Interlaced stream or variable: {}",
            if interlace_flags & 0x01 != 0 { "Yes" } else { "No" }
        ),
        indent + 2,
    );
    dump_string(
        &format!(
            "Fields per frame: {}",
            if interlace_flags & 0x02 != 0 { "1" } else { "2" }
        ),
        indent + 2,
    );
    dump_string(
        &format!(
            "Field 1 first: {}",
            if interlace_flags & 0x04 != 0 { "Yes" } else { "No" }
        ),
        indent + 2,
    );
    dump_string(
        &format!("Field pattern: {}", field_pattern((interlace_flags >> 4) & 0x03)),
        indent + 2,
    );
}

fn dump_format_mjpeg(fmjpeg: &video::FormatMJPEG, indent: usize, width: usize) {
    dump_value(fmjpeg.format_index, "bFormatIndex", indent + 2, width);
    dump_value(fmjpeg.num_frame_descriptors, "bNumFrameDescriptors", indent + 2, width);
    dump_value(fmjpeg.flags, "bmFlags", indent + 2, width);
    dump_string(
        &format!(
            "Fixed-size samples: {}",
            if fmjpeg.flags & 0x01 != 0 { "Yes" } else { "No" }
        ),
        indent + 2,
    );
    dump_value(fmjpeg.default_frame_index, "bDefaultFrameIndex", indent + 2, width);
    dump_value(fmjpeg.aspect_ratio_x, "bAspectRatioX", indent + 2, width);
    dump_value(fmjpeg.aspect_ratio_y, "bAspectRatioY", indent + 2, width);
    dump_interlace_flags(fmjpeg.interlace_flags, indent + 2, width);
    dump_value(fmjpeg.copy_protect, "bCopyProtect", indent + 2, width);
}

fn dump_still_image_frame(sif: &video::StillImageFrame, indent: usize, width: usize) {
    dump_value_string(
        sif.endpoint_address.address,
        "bEndpointAddress",
        sif.endpoint_address.to_string(),
        indent + 2,
        width,
    );
    dump_value(sif.num_image_size_patterns, "bNumImageSizePatterns", indent + 2, width);

    for (i, (w, h)) in sif.image_size_patterns.iter().enumerate() {
        dump_value(*w, &format!("wWidth({:2})", i), indent + 2, width);
        dump_value(*h, &format!("wHeight({:2})", i), indent + 2, width);
    }

    dump_value(sif.num_compression_patterns, "bNumCompressionPatterns", indent + 2, width);

    for (i, b) in sif.compression_patterns.iter().enumerate() {
        dump_value(
            *b,
            &format!("bCompression({:2})", i),
            indent + 2,
            width,
        );
    }
}

fn dump_format_frame(fufb: &video::FormatFrame, indent: usize, width: usize) {
    dump_value(fufb.format_index, "bFormatIndex", indent + 2, width);
    dump_value(fufb.num_frame_descriptors, "bNumFrameDescriptors", indent + 2, width);
    dump_guid(&fufb.guid_format, "guidFormat", indent + 2, width);
    dump_value(fufb.bits_per_pixel, "bBitsPerPixel", indent + 2, width);
    dump_value(fufb.default_frame_index, "bDefaultFrameIndex", indent + 2, width);
    dump_value(fufb.aspect_ratio_x, "bAspectRatioX", indent + 2, width);
    dump_value(fufb.aspect_ratio_y, "bAspectRatioY", indent + 2, width);
    dump_hex(fufb.interlace_flags, "bmInterlaceFlags", indent + 2, width);
    dump_value(fufb.copy_protect, "bCopyProtect", indent + 2, width);
    dump_interlace_flags(fufb.interlace_flags, indent + 2, width);
    if let Some(variable_size) = fufb.variable_size {
        dump_value(variable_size, "bVariableSize", indent + 2, width);
    }
}

fn dump_frame(frame: &video::FrameCommon, indent: usize, dump_width: usize) {
    dump_value(frame.frame_index, "bFrameIndex", indent + 2, dump_width);
    dump_hex(frame.capabilities, "bmCapabilities", indent + 2, dump_width);
    if frame.capabilities & 0x01 != 0 {
        dump_string("Still image supported", indent + 4);
    } else {
        dump_string("Still image unsupported", indent + 4);
    }
    if frame.capabilities & 0x02 != 0 {
        dump_string("Fixed frame-rate", indent + 4);
    }
    dump_value(frame.width, "wWidth", indent + 2, dump_width);
    dump_value(frame.height, "wHeight", indent + 2, dump_width);
    dump_value(frame.min_bit_rate, "dwMinBitRate", indent + 2, dump_width);
    dump_value(frame.max_bit_rate, "dwMaxBitRate", indent + 2, dump_width);
}

fn dump_frame_uncompressed(frame: &video::FrameUncompressed, indent: usize, dump_width: usize) {
    dump_frame(&frame.common, indent, dump_width);
    dump_value(frame.max_video_frame_buffer_size, "dwMaxVideoFrameBufferSize", indent + 2, dump_width);
    dump_value(frame.default_frame_interval, "dwDefaultFrameInterval", indent + 2, dump_width);
    dump_value(frame.frame_interval_type, "bFrameIntervalType", indent + 2, dump_width);
    if frame.frame_interval_type == 0 {
        dump_value(frame.frame_intervals[0], "dwMinFrameInterval", indent + 2, dump_width);
        dump_value(frame.frame_intervals[1], "dwMaxFrameInterval", indent + 2, dump_width);
        dump_value(frame.frame_intervals[2], "dwFrameIntervalStep", indent + 2, dump_width);
    } else {
        for (i, interval) in frame.frame_intervals.iter().enumerate() {
            dump_value(*interval, &format!("dwFrameInterval({:2})", i), indent + 2, dump_width);
        }
    }
}

fn dump_frame_framebased(frame: &video::FrameFrameBased, indent: usize, dump_width: usize) {
    dump_frame(&frame.common, indent, dump_width);
    dump_value(frame.default_frame_interval, "dwDefaultFrameInterval", indent + 2, dump_width);
    dump_value(frame.frame_interval_type, "bFrameIntervalType", indent + 2, dump_width);
    dump_value(frame.bytes_per_line, "dwBytesPerLine", indent + 2, dump_width);
    if frame.frame_interval_type == 0 {
        dump_value(frame.frame_intervals[0], "dwMinFrameInterval", indent + 2, dump_width);
        dump_value(frame.frame_intervals[1], "dwMaxFrameInterval", indent + 2, dump_width);
        dump_value(frame.frame_intervals[2], "dwFrameIntervalStep", indent + 2, dump_width);
    } else {
        for (i, interval) in frame.frame_intervals.iter().enumerate() {
            dump_value(*interval, &format!("dwFrameInterval({:2})", i), indent + 2, dump_width);
        }
    }
}

fn dump_video_subtype(
    uvcid: &video::UvcInterfaceDescriptor,
    protocol: u8,
    indent: usize,
    width: usize,
) {
    match uvcid {
        video::UvcInterfaceDescriptor::Header(h) => {
            dump_value(
                h.version,
                "bcdUVC",
                indent + 2,
                width,
            );
            dump_hex(
                h.total_length,
                "wTotalLength",
                indent + 2,
                width,
            );
            dump_value(
                format!("{:5}.{:06}MHz", h.clock_frequency / 1000000, h.clock_frequency % 1000000),
                "dwClockFrequency",
                indent + 2,
                width + 10,
            );
            dump_value(h.collection_bytes, "bInCollection", indent + 2, width);
            dump_array(
                &h.interfaces,
                "baInterfaceNr",
                indent + 2,
                width,
            );
        }
        video::UvcInterfaceDescriptor::InputTerminal(d) => {
            dump_value(d.terminal_id, "bTerminalID", indent + 2, width);
            dump_value_string(
                format!("0x{:04x}", d.terminal_type),
                "wTerminalType",
                names::videoterminal(d.terminal_type).unwrap_or_default(),
                indent + 2,
                width,
            );
            dump_value(d.associated_terminal, "bAssocTerminal", indent + 2, width);
            dump_value_string(
                d.terminal_index,
                "iTerminal",
                d.terminal.as_ref().unwrap_or(&String::new()),
                indent + 2,
                width,
            );

            if let Some(extra) = &d.extra {
                dump_value(
                    extra.objective_focal_length_min,
                    "wObjectiveFocalLengthMin",
                    indent + 2,
                    width,
                );
                dump_value(
                    extra.objective_focal_length_max,
                    "wObjectiveFocalLengthMax",
                    indent + 2,
                    width,
                );
                dump_value(
                    extra.ocular_focal_length,
                    "wOcularFocalLength",
                    indent + 2,
                    width,
                );
                dump_value(extra.control_size, "bControlSize", indent + 2, width);
                dump_hex(extra.controls, "bmControls", indent + 2, width);

                if protocol == 0x01 {
                    for (i, n) in CAM_CTRL_NAMES.iter().enumerate().take(22) {
                        if (extra.controls >> i) & 1 != 0 {
                            dump_string(n, indent + 4);
                        }
                    }
                } else {
                    for (i, n) in CAM_CTRL_NAMES.iter().enumerate().take(19) {
                        if (extra.controls >> i) & 1 != 0 {
                            dump_string(n, indent + 4);
                        }
                    }
                }
            }
        }
        video::UvcInterfaceDescriptor::OutputTerminal(ot) => {
            dump_audio_output_terminal1(&ot, indent, width);
        }
        video::UvcInterfaceDescriptor::SelectorUnit(su) => {
            dump_audio_selector_unit1(&su, indent, width);
        }
        video::UvcInterfaceDescriptor::ProcessingUnit(pu) => {
            dump_processing_unit(&pu, protocol, indent, width);
        }
        video::UvcInterfaceDescriptor::ExtensionUnit(eu) => {
            dump_extension_unit(&eu, indent, width);
        }
        video::UvcInterfaceDescriptor::EncodingUnit(eu) => {
            dump_encoding_unit(&eu, indent, width);
        }
        video::UvcInterfaceDescriptor::InputHeader(d) => {
            dump_video_input_header(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::OutputHeader(d) => {
            dump_video_output_header(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::StillImageFrame(d) => {
            dump_still_image_frame(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::FormatFrameBased(d) |
            video::UvcInterfaceDescriptor::FormatUncompressed(d) => {
            dump_format_frame(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::FrameUncompressed(d) |
            video::UvcInterfaceDescriptor::FrameMJPEG(d) => {
            dump_frame_uncompressed(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::FrameFrameBased(d) => {
            dump_frame_framebased(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::FormatMJPEG(d) => {
            dump_format_mjpeg(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::FormatMPEG2TS(d) => {
            dump_format_mpeg2ts(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::ColorFormat(d) => {
            dump_video_color_format(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::FormatStreamBased(d) => {
            dump_format_stream_based(&d, indent, width);
        }
        video::UvcInterfaceDescriptor::Undefined(data) | video::UvcInterfaceDescriptor::Invalid(data) => {
            println!(
                "{:indent$}Invalid desc subtype: {}",
                "",
                data.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join(" "),
                indent = indent + 2,
            );
        }
        _ => {
            log::warn!("Unhandled UVC interface descriptor: {:?}", uvcid);
        }
    }
}

pub(crate) fn dump_videostreaming_interface(vsd: &video::UvcDescriptor, vst: &video::StreamingSubtype, protocol: u8, indent: usize) {
    const DUMP_WIDTH: usize = 36; // wider in lsusb for long numbers
    dump_string("VideoStreaming Interface Descriptor:", indent);
    dump_value(vsd.length, "bLength", indent + 2, DUMP_WIDTH);
    dump_value(
        vsd.descriptor_type,
        "bDescriptorType",
        indent + 2,
        DUMP_WIDTH,
    );
    dump_value_string(
        vst.to_owned() as u8,
        "bDescriptorSubtype",
        format!("({:#})", vst),
        indent + 2,
        DUMP_WIDTH,
    );

    match &vsd.interface {
        video::UvcInterfaceDescriptor::Invalid(_) => {
            println!(
                "{:indent$}Warning: {:#} descriptors are illegal for {}",
                "",
                vst,
                u8::from(protocol.to_owned()),
                indent = indent
            );
        }
        uvcid => dump_video_subtype(uvcid, protocol, indent, DUMP_WIDTH),
    }
}

