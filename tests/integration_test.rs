//! These test cyme CLI by reading from json but also output as json so that we can check without worrying about formatting
//!
//! It is slightly the dog wagging the tail but is as integration as it gets! Could improve by adding some tests for actual format like --block, --padding args etc
mod common;

#[test]
fn test_run() {
    let te = common::TestEnv::new();

    // just run and check it doesn't exit with error without --from-json arg
    te.assert_success_and_get_output(None, &[]);
}

#[test]
#[cfg(target_os = "macos")]
fn test_run_force_libusb() {
    let te = common::TestEnv::new();

    // just run and check it doesn't exit with error without --from-json arg
    te.assert_success_and_get_output(None, &["--force-libusb"]);
}

#[test]
fn test_list() {
    let te = common::TestEnv::new();

    let mut comp_sp = common::sp_data_from_libusb_linux();
    comp_sp.into_flattened();
    let devices = comp_sp.flattened_devices();
    let comp = serde_json::to_string_pretty(&devices).unwrap();

    // TODO not sure why assert_output_json doesn't work, might help to have module which shows diff
    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json"],
        &comp,
        false,
    );
}

#[test]
fn test_list_filtering() {
    let te = common::TestEnv::new();

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

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--filter-name", "Black Magic"],
        &comp,
        false,
    );

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--vidpid", "1d50"],
        &comp,
        false,
    );

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--vidpid", "1d50:6018"],
        &comp,
        false,
    );

    te.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--vidpid", "1d50:unhappy"],
    );

    te.assert_output(
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

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--show", "2:"],
        &comp,
        false,
    );

    te.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--show", "f"],
    );

    filter.number = Some(23);
    filter.retain_flattened_devices_ref(&mut devices);
    let comp = serde_json::to_string_pretty(&devices).unwrap();

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--show", "2:23"],
        &comp,
        false,
    );

    te.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--show", "blah"],
    );
}

#[test]
fn test_tree() {
    let te = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::CYME_LIBUSB_LINUX_TREE_DUMP);

    te.assert_output_json(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--tree"],
        &comp,
    );
}

#[test]
fn test_tree_filtering() {
    let te = common::TestEnv::new();

    let mut comp_sp = common::sp_data_from_libusb_linux();
    let filter = cyme::profiler::Filter {
        name: Some("Black Magic".into()),
        ..Default::default()
    };
    filter.retain_buses(&mut comp_sp.buses);
    let comp = serde_json::to_string_pretty(&comp_sp).unwrap();

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--tree", "--vidpid", "1d50"],
        &comp,
        false,
    );
}
