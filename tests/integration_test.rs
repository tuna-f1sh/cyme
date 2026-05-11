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
    let filter = cyme::profiler::DeviceFilter {
        include_root_hubs: true,
        ..cyme::profiler::Filter {
            name: Some("Black Magic".into()),
            ..Default::default()
        }
        .into()
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
    let mut filter = cyme::profiler::DeviceFilter {
        include_root_hubs: true,
        ..cyme::profiler::Filter {
            bus: Some(2),
            ..Default::default()
        }
        .into()
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

    if let Some(f) = filter.filters.first_mut() {
        f.number = Some(23);
    }
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

    // Multiple --vidpid args are OR'd: Black Magic (1d50:6018) OR J-Link (1366:1050)
    {
        let mut comp_sp = common::sp_data_from_libusb_linux();
        let filter = cyme::profiler::DeviceFilter {
            include_root_hubs: true,
            ..cyme::profiler::DeviceFilter::from(vec![
                cyme::profiler::Filter::new_with_vid_pid(0x1d50, 0x6018),
                cyme::profiler::Filter::new_with_vid_pid(0x1366, 0x1050),
            ])
        };
        comp_sp.into_flattened();
        let mut devices = comp_sp.flattened_devices();
        filter.retain_flattened_devices_ref(&mut devices);
        let comp = serde_json::to_string_pretty(&devices).unwrap();

        env.assert_output(
            Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
            &["--json", "--vidpid", "1d50:6018", "--vidpid", "1366:1050"],
            &comp,
            false,
        );
    }

    // Multiple --filter-name args are OR'd
    {
        let mut comp_sp = common::sp_data_from_libusb_linux();
        let filter = cyme::profiler::DeviceFilter {
            include_root_hubs: true,
            ..cyme::profiler::DeviceFilter::from(vec![
                cyme::profiler::Filter::new_with_name("Black Magic".into(), false),
                cyme::profiler::Filter::new_with_name("J-Link".into(), false),
            ])
        };
        comp_sp.into_flattened();
        let mut devices = comp_sp.flattened_devices();
        filter.retain_flattened_devices_ref(&mut devices);
        let comp = serde_json::to_string_pretty(&devices).unwrap();

        env.assert_output(
            Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
            &[
                "--json",
                "--filter-name",
                "Black Magic",
                "--filter-name",
                "J-Link",
            ],
            &comp,
            false,
        );
    }

    // --filter-exclude removes matching devices from the inclusion set
    // --vidpid 203a includes all three Virtual devices; --filter-exclude name=Mouse removes one
    {
        let mut comp_sp = common::sp_data_from_libusb_linux();
        let filter = cyme::profiler::DeviceFilter {
            filters: vec![cyme::profiler::Filter {
                vid: Some(0x203a),
                ..Default::default()
            }],
            exclude_filters: vec![cyme::profiler::Filter::new_with_name("Mouse".into(), false)],
            include_root_hubs: true,
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
                "203a",
                "--filter-exclude",
                "name=Mouse",
            ],
            &comp,
            false,
        );
    }

    // Cross-product: two vidpids × one name produces two Filters (AND within each).
    // J-Link (1366) does not contain "Virtual" so only the 203a devices pass.
    {
        let mut comp_sp = common::sp_data_from_libusb_linux();
        let filter = cyme::profiler::DeviceFilter {
            include_root_hubs: true,
            ..cyme::profiler::DeviceFilter::from(vec![
                cyme::profiler::Filter {
                    vid: Some(0x203a),
                    name: Some("Virtual".into()),
                    ..Default::default()
                },
                cyme::profiler::Filter {
                    vid: Some(0x1366),
                    name: Some("Virtual".into()),
                    ..Default::default()
                },
            ])
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
                "203a",
                "--vidpid",
                "1366",
                "--filter-name",
                "Virtual",
            ],
            &comp,
            false,
        );
    }
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
    let filter = cyme::profiler::DeviceFilter::from(cyme::profiler::Filter {
        name: Some("Black Magic".into()),
        ..Default::default()
    });
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
