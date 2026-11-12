//! Regenerates `tests/data/cyme_descriptor_fixture.json`.
//!
//! This is a tool rather than an example of using the library. The fixture is a
//! synthetic dump carrying the class descriptors that `--lsusb --verbose`
//! renders but that no single host is likely to have attached at once: CCID,
//! DFU, IPP printer, a readable HID report, OTG, UVC and UAC. Together with
//! `tests/data/lsusb_fixture_verbose.txt` it gives
//! `test_lsusb_descriptor_fixture_verbose` something to check the dump against.
//!
//! Every descriptor below starts life as the raw bytes a device would send and
//! is parsed by cyme's own `TryFrom<&[u8]>` impls, so the fixture cannot drift
//! away from the parsers it is meant to exercise. The device and interface tree
//! around them is plain JSON: it is scaffolding to hang the descriptors off,
//! not anything under test.
//!
//! To regenerate both files after a deliberate change, from the repository root:
//!
//! ```sh
//! cargo run --example gen_descriptor_fixture > tests/data/cyme_descriptor_fixture.json
//! cargo run -- --from-json tests/data/cyme_descriptor_fixture.json --lsusb -v \
//!     > tests/data/lsusb_fixture_verbose.txt
//! ```
//!
//! Then read the diff of the second file before committing - that is the point
//! of it.
use cyme::usb::descriptors::audio;
use cyme::usb::descriptors::video;
use cyme::usb::descriptors::*;
use serde_json::{json, Value};

/// A 61 byte CCID descriptor: 54 bytes of fields plus 7 bytes of trailing junk.
///
/// dwFeatures is 0x000204BA, which sets a flag in every group lsusb splits the
/// field into: plain bits (0x02 auto config, 0x08 auto voltage, 0x10 auto clock,
/// 0x20 auto baud, 0x0400 auto IFSD), the exclusive negotiation choice (0x0080
/// auto PPS) and the exclusive exchange level (0x00020000 short APDU).
fn ccid_descriptor() -> Vec<u8> {
    let mut b = vec![0u8; 54];
    b[0] = 61; // bLength, 7 bytes longer than the descriptor proper
    b[1] = 0x21; // bDescriptorType
    b[2..4].copy_from_slice(&0x0110u16.to_le_bytes()); // bcdCCID 1.10
    b[4] = 0; // bMaxSlotIndex
    b[5] = 0x07; // bVoltageSupport: 5.0V 3.0V 1.8V
    b[6..10].copy_from_slice(&3u32.to_le_bytes()); // dwProtocols T=0 T=1
    b[10..14].copy_from_slice(&4_000u32.to_le_bytes()); // dwDefaultClock
    b[14..18].copy_from_slice(&5_000u32.to_le_bytes()); // dwMaximumClock
    b[18] = 0; // bNumClockSupported
    b[19..23].copy_from_slice(&10_752u32.to_le_bytes()); // dwDataRate
    b[23..27].copy_from_slice(&307_200u32.to_le_bytes()); // dwMaxDataRate
    b[27] = 0; // bNumDataRatesSupp.
    b[28..32].copy_from_slice(&254u32.to_le_bytes()); // dwMaxIFSD
    b[32..36].copy_from_slice(&7u32.to_le_bytes()); // dwSyncProtocols
    b[36..40].copy_from_slice(&0x0fu32.to_le_bytes()); // dwMechanical
    b[40..44].copy_from_slice(&0x0002_04bau32.to_le_bytes()); // dwFeatures
    b[44..48].copy_from_slice(&271u32.to_le_bytes()); // dwMaxCCIDMsgLen
    b[48] = 0xff; // bClassGetResponse: echo
    b[49] = 0xff; // bClassEnvelope: echo
    b[50] = 16; // wLcdLayout cols
    b[51] = 2; // wLcdLayout lines
    b[52] = 0x03; // bPINSupport: verification modification
    b[53] = 1; // bMaxCCIDBusySlots
    b.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef, 0x01, 0x02, 0x03]);
    b
}

/// A 9 byte DFU 1.1 functional descriptor.
///
/// bmAttributes 0x0f sets all four defined bits, so every attribute line takes
/// its "set" branch - including bitManifestationTolerant, which lsusb renders
/// as "Manifestation Tolerant".
fn dfu_descriptor() -> Vec<u8> {
    vec![
        0x09, // bLength
        0x21, // bDescriptorType
        0x0f, // bmAttributes: will detach, manifestation tolerant, upload, download
        0xe8, 0x03, // wDetachTimeout 1000
        0x00, 0x04, // wTransferSize 1024
        0x1a, 0x01, // bcdDFUVersion 1.1a
    ]
}

/// A 14 byte IPP printer descriptor with one basic capabilities block.
///
/// wBasicCapabilities 0x0031 is Print + HTTP-over-USB with the authentication
/// field at 0x20, i.e. Username-Auth.
fn printer_descriptor() -> Vec<u8> {
    vec![
        0x0e, // bLength
        0x21, // bDescriptorType
        0x00, // bcdReleaseNumber
        0x01, // bcdNumDescriptors
        0x00, // capability type 0 (basic)
        0x06, // capability length
        0x31, 0x00, // wBasicCapabilities 0x0031
        0x02, // iIPPVersionsSupported
        0x05, // iIPPPrinterUUID
        0x00, 0x00, 0x00, 0x00,
    ]
}

/// A HID report descriptor exercising the tags lsusb decodes specially: Usage
/// Page (so the Usages that follow resolve against it), Usage, Collection, Unit
/// Exponent, Unit with an exponent nibble, and Input with its two attribute
/// lines.
///
/// Note lsusb prints the unit exponent as a signed *char*, not as the signed
/// nibble the HID spec defines, so 0x0e renders as 14 rather than -2. cyme
/// copies that; the two only diverge above 0x7f.
fn hid_report() -> Vec<u8> {
    vec![
        0x05, 0x01, // Usage Page (Generic Desktop)
        0x09, 0x02, // Usage (Mouse)
        0xa1, 0x01, // Collection (Application)
        0x09, 0x01, //   Usage (Pointer)
        0x55, 0x0e, //   Unit Exponent
        0x66, 0x21, 0x00, //   Unit (SI Linear: Centimeter^2)
        0x81, 0x02, //   Input (Data, Variable, Absolute)
        0xc0, // End Collection
    ]
}

/// A HID descriptor whose report descriptor is present rather than UNAVAILABLE.
fn hid_descriptor() -> HidDescriptor {
    let report = hid_report();
    let mut hid = vec![
        0x09, // bLength
        0x21, // bDescriptorType
        0x11, 0x01, // bcdHID 1.11
        0x00, // bCountryCode
        0x01, // bNumDescriptors
        0x22, // bDescriptorType (Report)
    ];
    hid.extend_from_slice(&(report.len() as u16).to_le_bytes());
    let mut hidd = HidDescriptor::try_from(hid.as_slice()).expect("HID descriptor");
    hidd.descriptors[0].data = Some(report);
    hidd
}

/// VideoControl interface descriptors: a header, a camera input terminal and a
/// processing unit.
///
/// Between them these cover the three different column widths lsusb uses
/// inside a VideoControl descriptor - the default, the wider dwClockFrequency,
/// and the wider bmControls of a camera terminal.
fn videocontrol_descriptors() -> Vec<Vec<u8>> {
    vec![
        // HEADER: bcdUVC 1.10, wTotalLength, dwClockFrequency 30 MHz, one
        // streaming interface in the collection
        vec![
            0x0d, 0x24, 0x01, // bLength, CS_INTERFACE, HEADER
            0x10, 0x01, // bcdUVC 1.10
            0x4d, 0x00, // wTotalLength
            0xc0, 0xc6, 0x2d, 0x00, // dwClockFrequency 3000000
            0x01, // bInCollection
            0x01, // baInterfaceNr[0]
        ],
        // INPUT_TERMINAL, wTerminalType 0x0201 (camera), so it carries the
        // optical and bmControls fields
        vec![
            0x12, 0x24, 0x02, // bLength, CS_INTERFACE, INPUT_TERMINAL
            0x01, // bTerminalID
            0x01, 0x02, // wTerminalType 0x0201 ITT_CAMERA
            0x00, // bAssocTerminal
            0x00, // iTerminal
            0x00, 0x00, // wObjectiveFocalLengthMin
            0x00, 0x00, // wObjectiveFocalLengthMax
            0x00, 0x00, // wOcularFocalLength
            0x03, // bControlSize
            0x0e, 0x00, 0x00, // bmControls: exposure time, focus, iris
        ],
        // PROCESSING_UNIT
        vec![
            0x0c, 0x24, 0x05, // bLength, CS_INTERFACE, PROCESSING_UNIT
            0x03, // bUnitID
            0x01, // bSourceID
            0x00, 0x00, // wMaxMultiplier
            0x02, // bControlSize
            0x05, 0x00, // bmControls: brightness, hue
            0x00, // iProcessing
            0x00, // bmVideoStandards
        ],
    ]
}

/// VideoStreaming interface descriptors: an input header, an uncompressed
/// format and one uncompressed frame.
fn videostreaming_descriptors() -> Vec<Vec<u8>> {
    vec![
        // INPUT_HEADER, one format, one byte of controls per format
        vec![
            0x0e, 0x24, 0x01, // bLength, CS_INTERFACE, INPUT_HEADER
            0x01, // bNumFormats
            0x3f, 0x00, // wTotalLength
            0x81, // bEndpointAddress
            0x00, // bmInfo
            0x02, // bTerminalLink
            0x00, // bStillCaptureMethod
            0x00, // bTriggerSupport
            0x00, // bTriggerUsage
            0x01, // bControlSize
            0x00, // bmaControls[0]
        ],
        // FORMAT_UNCOMPRESSED, YUY2
        vec![
            0x1b, 0x24, 0x04, // bLength, CS_INTERFACE, FORMAT_UNCOMPRESSED
            0x01, // bFormatIndex
            0x01, // bNumFrameDescriptors
            0x59, 0x55, 0x59, 0x32, 0x00, 0x00, 0x10, 0x00, // guidFormat YUY2
            0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71, 0x10, // bBitsPerPixel
            0x01, // bDefaultFrameIndex
            0x00, // bAspectRatioX
            0x00, // bAspectRatioY
            0x00, // bmInterlaceFlags
            0x00, // bCopyProtect
        ],
        // FRAME_UNCOMPRESSED, 640x480, one discrete frame interval
        vec![
            0x1e, 0x24, 0x05, // bLength, CS_INTERFACE, FRAME_UNCOMPRESSED
            0x01, // bFrameIndex
            0x03, // bmCapabilities: still image, fixed frame-rate
            0x80, 0x02, // wWidth 640
            0xe0, 0x01, // wHeight 480
            0x00, 0x00, 0x15, 0x16, // dwMinBitRate
            0x00, 0x00, 0x15, 0x16, // dwMaxBitRate
            0x00, 0x60, 0x09, 0x00, // dwMaxVideoFrameBufferSize
            0x15, 0x16, 0x05, 0x00, // dwDefaultFrameInterval
            0x01, // bFrameIntervalType
            0x15, 0x16, 0x05, 0x00, // dwFrameInterval[0]
        ],
    ]
}

/// UAC1 AudioControl descriptors: a header, an input and output terminal pair,
/// and a feature unit.
///
/// These are the descriptors lsusb renders from its field tables, padding every
/// field in a descriptor out to the longest name in it. None of these names
/// reaches the 18 character floor, so they all share one column - the point of
/// including them is to catch it if that stops being true.
fn audiocontrol_descriptors() -> Vec<Vec<u8>> {
    vec![
        // HEADER: bcdADC 1.00, one streaming interface in the collection
        vec![
            0x09, 0x24, 0x01, // bLength, CS_INTERFACE, HEADER
            0x00, 0x01, // bcdADC 1.00
            0x28, 0x00, // wTotalLength
            0x01, // bInCollection
            0x01, // baInterfaceNr(0)
        ],
        // INPUT_TERMINAL, wTerminalType 0x0201 (microphone)
        vec![
            0x0c, 0x24, 0x02, // bLength, CS_INTERFACE, INPUT_TERMINAL
            0x01, // bTerminalID
            0x01, 0x02, // wTerminalType 0x0201
            0x00, // bAssocTerminal
            0x02, // bNrChannels
            0x03, 0x00, // wChannelConfig left front, right front
            0x00, // iChannelNames
            0x00, // iTerminal
        ],
        // OUTPUT_TERMINAL, wTerminalType 0x0101 (USB streaming)
        vec![
            0x09, 0x24, 0x03, // bLength, CS_INTERFACE, OUTPUT_TERMINAL
            0x02, // bTerminalID
            0x01, 0x01, // wTerminalType 0x0101
            0x00, // bAssocTerminal
            0x01, // bSourceID
            0x00, // iTerminal
        ],
        // FEATURE_UNIT with one control byte per channel
        vec![
            0x0a, 0x24, 0x06, // bLength, CS_INTERFACE, FEATURE_UNIT
            0x03, // bUnitID
            0x01, // bSourceID
            0x01, // bControlSize
            0x03, // bmaControls(0): mute, volume
            0x00, // bmaControls(1)
            0x00, // bmaControls(2)
            0x00, // iFeature
        ],
    ]
}

/// UAC1 AudioStreaming descriptors: the general interface descriptor and a
/// type I format with one discrete sampling frequency.
fn audiostreaming_descriptors() -> Vec<Vec<u8>> {
    vec![
        vec![
            0x07, 0x24, 0x01, // bLength, CS_INTERFACE, AS_GENERAL
            0x02, // bTerminalLink
            0x01, // bDelay
            0x01, 0x00, // wFormatTag PCM
        ],
        vec![
            0x0b, 0x24, 0x02, // bLength, CS_INTERFACE, FORMAT_TYPE
            0x01, // bFormatType FORMAT_TYPE_I
            0x02, // bNrChannels
            0x02, // bSubframeSize
            0x10, // bBitResolution
            0x01, // bSamFreqType: one discrete frequency
            0x44, 0xac, 0x00, // tSamFreq[0] 44100
        ],
    ]
}

/// Parse a class specific audio descriptor for the given interface subclass
fn audio_descriptor(bytes: &[u8], subclass: u8) -> Descriptor {
    let gd = GenericDescriptor::try_from(bytes).expect("generic descriptor");
    let protocol = 0x00; // UAC1
    Descriptor::Interface(ClassDescriptor::Audio(
        audio::UacDescriptor::try_from((gd, subclass, protocol)).expect("UAC descriptor"),
        audio::UacProtocol::from(protocol),
    ))
}

/// Parse a class specific video descriptor for the given interface subclass
fn video_descriptor(bytes: &[u8], subclass: u8) -> Descriptor {
    let gd = GenericDescriptor::try_from(bytes).expect("generic descriptor");
    Descriptor::Interface(ClassDescriptor::Video(
        video::UvcDescriptor::try_from((gd, subclass, 0x00)).expect("UVC descriptor"),
        0x00,
    ))
}

/// A UAC1 EP_GENERAL isochronous audio data endpoint descriptor
///
/// Carries CS_ENDPOINT rather than CS_INTERFACE as its descriptor type, which
/// is the only thing that distinguishes it from AS_GENERAL: both are
/// bDescriptorSubtype 1 and both carry four bytes of data under UAC1.
fn audio_endpoint_descriptor() -> Descriptor {
    let bytes = [
        0x07, 0x25, 0x01, // bLength, CS_ENDPOINT, EP_GENERAL
        0x01, // bmAttributes: sampling frequency
        0x02, // bLockDelayUnits: decoded PCM samples
        0x00, 0x00, // wLockDelay
    ];
    let gd = GenericDescriptor::try_from(bytes.as_slice()).expect("generic descriptor");
    let protocol = 0x00; // UAC1
    Descriptor::Endpoint(ClassDescriptor::Audio(
        audio::UacDescriptor::try_from((gd, 2, protocol)).expect("UAC endpoint descriptor"),
        audio::UacProtocol::from(protocol),
    ))
}

/// An endpoint, with `length` 9 rather than 7 to give it bRefresh and
/// bSynchAddress as an audio endpoint has
fn endpoint(address: u8, transfer_type: &str, length: u8, extra: Option<Vec<Descriptor>>) -> Value {
    let mut ep = json!({
        "length": length,
        "address": {
            "address": address,
            "number": address & 0x0f,
            "direction": if address & 0x80 != 0 { "In" } else { "Out" },
        },
        "transfer_type": transfer_type,
        "sync_type": "None",
        "usage_type": "Data",
        "max_packet_size": 64,
        "interval": 10,
        "extra": extra,
        "endpoint_path": Value::Null,
    });

    // only a 9 byte descriptor carries these, and cyme omits them rather than
    // writing nulls, so leave the keys out entirely otherwise
    if length == 9 {
        let map = ep.as_object_mut().expect("endpoint object");
        map.insert(String::from("refresh"), json!(4));
        map.insert(String::from("sync_address"), json!(0x81));
    }

    ep
}

/// `class` is the (name, subclass, protocol) triplet for the interface
fn interface(
    name: &str,
    number: u8,
    device: u8,
    class: (&str, u8, u8),
    extra: Option<Vec<Descriptor>>,
    endpoints: Vec<Value>,
) -> Value {
    let (class, sub_class, protocol) = class;
    json!({
        "name": name,
        "string_index": 0,
        "number": number,
        "path": format!("9-{device}:1.0"),
        "class": class,
        "sub_class": sub_class,
        "protocol": protocol,
        "alt_setting": 0,
        "active": true,
        "driver": Value::Null,
        "syspath": Value::Null,
        "endpoints": endpoints,
        "length": 9,
        "extra": extra,
        "device_path": Value::Null,
        "devpaths": Value::Null,
        "mount_paths": Value::Null,
    })
}

fn device(
    name: &str,
    product_id: u8,
    class: &str,
    speed: &str,
    interfaces: Vec<Value>,
    config_extra: Option<Vec<Descriptor>>,
) -> Value {
    json!({
        "name": name,
        "vendor_id": 0xffff,
        "product_id": product_id,
        "location_id": { "bus": 9, "tree_positions": [product_id], "number": product_id },
        "serial_num": "FIXTURE",
        "manufacturer": "cyme fixture",
        "bcd_device": "1.00",
        "bcd_usb": "2.00",
        "device_speed": speed,
        "class": class,
        "sub_class": 0,
        "protocol": 0,
        "extra": {
            "max_packet_size": 64,
            "driver": Value::Null,
            "syspath": Value::Null,
            "vendor": "cyme fixture",
            "product_name": name,
            "string_indexes": [2, 1, 3],
            "configurations": [{
                "name": "Fixture Configuration",
                "string_index": 4,
                "number": 1,
                "active": true,
                "num_interfaces": interfaces.len(),
                "interfaces": interfaces,
                "attributes": ["SelfPowered"],
                "max_power": { "value": 100, "unit": "mA" },
                "length": 9,
                "total_length": 32,
                "extra": config_extra,
            }],
            "status": 1,
            "debug": Value::Null,
            "binary_object_store": Value::Null,
            "qualifier": Value::Null,
            "hub": Value::Null,
            "negotiated_speed": Value::Null,
        },
    })
}

fn main() {
    let ccid = Descriptor::Device(ClassDescriptor::Ccid(
        CcidDescriptor::try_from(ccid_descriptor().as_slice()).expect("CCID descriptor"),
    ));
    let dfu = Descriptor::Interface(ClassDescriptor::Dfu(
        DfuDescriptor::try_from(dfu_descriptor().as_slice()).expect("DFU descriptor"),
    ));
    let printer = Descriptor::Interface(ClassDescriptor::Printer(
        PrinterDescriptor::try_from(printer_descriptor().as_slice()).expect("printer descriptor"),
    ));
    let hid = Descriptor::Device(ClassDescriptor::Hid(hid_descriptor()));
    let otg = Descriptor::Otg(
        OnTheGoDescriptor::try_from([0x03u8, 0x09, 0x03].as_slice()).expect("OTG descriptor"),
    );

    let devices = vec![
        device(
            "Fixture ChipCard Reader",
            1,
            "smart-card",
            "12.0 Mb/s",
            vec![interface(
                "CCID Interface",
                0,
                1,
                ("smart-card", 0, 0),
                Some(vec![ccid]),
                vec![endpoint(0x81, "Bulk", 7, None)],
            )],
            None,
        ),
        device(
            "Fixture DFU Device",
            2,
            "application-specific-interface",
            "12.0 Mb/s",
            vec![interface(
                "DFU Interface",
                0,
                2,
                ("application-specific-interface", 1, 2),
                Some(vec![dfu]),
                vec![],
            )],
            None,
        ),
        device(
            "Fixture IPP Printer",
            3,
            "printer",
            "12.0 Mb/s",
            vec![interface(
                "IPP Interface",
                0,
                3,
                ("printer", 1, 4),
                Some(vec![printer]),
                vec![endpoint(0x02, "Bulk", 7, None)],
            )],
            None,
        ),
        device(
            "Fixture HID Device",
            4,
            "hid",
            "12.0 Mb/s",
            // a 9 byte endpoint descriptor, so bRefresh/bSynchAddress print
            vec![interface(
                "HID Interface",
                0,
                4,
                ("hid", 1, 2),
                Some(vec![hid]),
                vec![endpoint(0x81, "Interrupt", 9, None)],
            )],
            None,
        ),
        device(
            "Fixture OTG Device",
            5,
            "vendor-specific-class",
            "12.0 Mb/s",
            vec![interface(
                "OTG Interface",
                0,
                5,
                ("vendor-specific-class", 0, 0),
                None,
                vec![endpoint(0x81, "Bulk", 7, None)],
            )],
            Some(vec![otg]),
        ),
        device(
            "Fixture UVC Camera",
            6,
            "video",
            "480.0 Mb/s",
            vec![
                interface(
                    "VideoControl",
                    0,
                    6,
                    ("video", 1, 0),
                    Some(
                        videocontrol_descriptors()
                            .iter()
                            .map(|b| video_descriptor(b, 1))
                            .collect(),
                    ),
                    vec![endpoint(0x83, "Interrupt", 7, None)],
                ),
                interface(
                    "VideoStreaming",
                    1,
                    6,
                    ("video", 2, 0),
                    Some(
                        videostreaming_descriptors()
                            .iter()
                            .map(|b| video_descriptor(b, 2))
                            .collect(),
                    ),
                    vec![endpoint(0x81, "Isochronous", 7, None)],
                ),
            ],
            None,
        ),
        device(
            "Fixture UAC Microphone",
            7,
            "audio",
            "12.0 Mb/s",
            vec![
                interface(
                    "AudioControl",
                    0,
                    7,
                    ("audio", 1, 0),
                    Some(
                        audiocontrol_descriptors()
                            .iter()
                            .map(|b| audio_descriptor(b, 1))
                            .collect(),
                    ),
                    vec![],
                ),
                interface(
                    "AudioStreaming",
                    1,
                    7,
                    ("audio", 2, 0),
                    Some(
                        audiostreaming_descriptors()
                            .iter()
                            .map(|b| audio_descriptor(b, 2))
                            .collect(),
                    ),
                    vec![endpoint(
                        0x81,
                        "Isochronous",
                        9,
                        Some(vec![audio_endpoint_descriptor()]),
                    )],
                ),
            ],
            None,
        ),
    ];

    let profile = json!({
        "buses": [{
            "name": "Fixture Host Controller",
            "host_controller": "cyme descriptor fixture",
            "pci_vendor": Value::Null,
            "pci_device": Value::Null,
            "usb_bus_number": 9,
            "devices": devices,
        }],
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&profile).expect("serialise fixture")
    );
}
