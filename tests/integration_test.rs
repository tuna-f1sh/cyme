//! These test cyme CLI by reading from json but also output as json so that we can check without worrying about formatting
//!
//! It is slightly the dog wagging the tail but is as integration as it gets! Could improve by adding some tests for actual format like --block, --padding args etc
mod common;

#[test]
fn test_run() {
    let env = common::TestEnv::new();

    // just run and check it doesn't exit with error without --from-json arg
    env.assert_success_and_get_output(None, &[]);
}

#[test]
#[cfg(target_os = "macos")]
fn test_run_force_libusb() {
    let env = common::TestEnv::new();

    // just run and check it doesn't exit with error without --from-json arg
    env.assert_success_and_get_output(None, &["--force-libusb"]);
}

#[test]
fn test_list() {
    let env = common::TestEnv::new();

    let mut comp_sp = common::sp_data_from_libusb_linux();
    comp_sp.into_flattened();
    let devices = comp_sp.flattened_devices();
    let comp = serde_json::to_string_pretty(&devices).unwrap();

    // TODO not sure why assert_output_json doesn't work, might help to have module which shows diff
    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json"],
        &comp,
        false,
    );
}

#[test]
fn test_list_filtering() {
    let env = common::TestEnv::new();

    let mut comp_sp = common::sp_data_from_libusb_linux();
    let filter = cyme::profiler::Filter {
        name: Some("Black Magic".into()),
        no_exclude_root_hub: true,
        ..Default::default()
    };
    comp_sp.into_flattened();
    let mut devices = comp_sp.flattened_devices();
    filter.retain_flattened_devices_ref(&mut devices);
    let comp = serde_json::to_string_pretty(&devices).unwrap();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--filter-name", "Black Magic"],
        &comp,
        false,
    );

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--vidpid", "1d50"],
        &comp,
        false,
    );

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--vidpid", "1d50:6018"],
        &comp,
        false,
    );

    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--vidpid", "1d50:unhappy"],
    );

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--filter-serial", "97B6A11D"],
        &comp,
        false,
    );

    let mut comp_sp = common::sp_data_from_libusb_linux();
    let mut filter = cyme::profiler::Filter {
        bus: Some(2),
        no_exclude_root_hub: true,
        ..Default::default()
    };
    comp_sp.into_flattened();
    let mut devices = comp_sp.flattened_devices();
    filter.retain_flattened_devices_ref(&mut devices);
    let comp = serde_json::to_string_pretty(&devices).unwrap();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--show", "2:"],
        &comp,
        false,
    );

    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--show", "f"],
    );

    filter.number = Some(23);
    filter.retain_flattened_devices_ref(&mut devices);
    let comp = serde_json::to_string_pretty(&devices).unwrap();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--show", "2:23"],
        &comp,
        false,
    );

    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--show", "blah"],
    );
}

#[test]
// windows line ending messes this up
#[cfg(not(target_os = "windows"))]
fn test_tree() {
    let env = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::CYME_LIBUSB_LINUX_TREE_DUMP);

    env.assert_output_json(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--tree"],
        &comp,
    );
}

#[test]
fn test_tree_filtering() {
    let env = common::TestEnv::new();

    let mut comp_sp = common::sp_data_from_libusb_linux();
    let filter = cyme::profiler::Filter {
        name: Some("Black Magic".into()),
        ..Default::default()
    };
    filter.retain_buses(&mut comp_sp.buses);
    let comp = serde_json::to_string_pretty(&comp_sp).unwrap();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--tree", "--vidpid", "1d50"],
        &comp,
        false,
    );
}

#[test]
fn test_vidpid_multi_value() {
    let env = common::TestEnv::new();

    // repeated --vidpid and comma-separated should both accumulate into the
    // same Filter.vidpid Vec, so their outputs must be identical.
    let out_repeated = env.assert_success_and_get_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--vidpid", "1d50:6018", "--vidpid", "05ac:"],
    );
    let out_csv = env.assert_success_and_get_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--vidpid", "1d50:6018,05ac:"],
    );
    assert_eq!(out_repeated, out_csv);
}

#[test]
fn test_exclude_vidpid() {
    let env = common::TestEnv::new();

    let mut comp_sp = common::sp_data_from_libusb_linux();
    let filter = cyme::profiler::Filter {
        exclude_vidpid: vec![(Some(0x1d50), Some(0x6018))],
        no_exclude_root_hub: true,
        ..Default::default()
    };
    comp_sp.into_flattened();
    let mut devices = comp_sp.flattened_devices();
    filter.retain_flattened_devices_ref(&mut devices);
    let comp = serde_json::to_string_pretty(&devices).unwrap();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--exclude", "vidpid=1d50:6018"],
        &comp,
        false,
    );
}

#[test]
fn test_exclude_include_composition() {
    let env = common::TestEnv::new();

    // include all 1d50 devices but exclude the specific 1d50:6018 pair
    let mut comp_sp = common::sp_data_from_libusb_linux();
    let filter = cyme::profiler::Filter {
        vidpid: vec![(Some(0x1d50), None)],
        exclude_vidpid: vec![(Some(0x1d50), Some(0x6018))],
        no_exclude_root_hub: true,
        ..Default::default()
    };
    comp_sp.into_flattened();
    let mut devices = comp_sp.flattened_devices();
    filter.retain_flattened_devices_ref(&mut devices);
    let comp = serde_json::to_string_pretty(&devices).unwrap();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &[
            "--json",
            "--vidpid",
            "1d50",
            "--exclude",
            "vidpid=1d50:6018",
        ],
        &comp,
        false,
    );
}

#[test]
fn test_exclude_parse_errors() {
    let env = common::TestEnv::new();

    // missing '='
    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--exclude", "vidpid1d50:6018"],
    );
    // unknown key
    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--exclude", "foo=bar"],
    );
    // u16 overflow in exclude vidpid
    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--exclude", "vidpid=0x10000:0x1"],
    );
}

#[test]
fn test_device_filter() {
    let env = common::TestEnv::new();

    // EHCI Host Controller (root hub)
    env.assert_success_and_get_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--device", "/dev/bus/usb/001/001"],
    );

    // Virtual Mouse (not a root hub)
    env.assert_success_and_get_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--device", "/dev/bus/usb/001/002"],
    );

    // Non-existent
    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--device", "/dev/bus/usb/001/010"],
    );
}
