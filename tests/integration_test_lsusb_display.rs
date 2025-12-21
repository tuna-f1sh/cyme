mod common;

/// Tests lsusb with no args compatibility mode
#[test]
fn test_lsusb_list() {
    let env = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::LSUSB_OUTPUT);

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb"],
        comp.as_str(),
        false,
    );
}

/// Tests lsusb --tree compatibility mode
#[test]
fn test_lsusb_tree() {
    let env = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::LSUSB_TREE_OUTPUT);

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--tree"],
        comp.as_str(),
        false,
    );
}

/// Tests lsusb --tree fully verbose compatibility mode
#[test]
fn test_lsusb_tree_verbose() {
    let env = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::LSUSB_TREE_OUTPUT_VERBOSE);

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--tree", "-vvv"],
        comp.as_str(),
        false,
    );
}

/// Tests lsusb -d vidpid filter
#[test]
fn test_lsusb_vidpid() {
    let env = common::TestEnv::new();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--vidpid", "1d50"],
        "Bus 002 Device 024: ID 1d50:6018 OpenMoko, Inc. Black Magic Debug Probe (Application)",
        false,
    );
    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--vidpid", "1d50:"],
        "Bus 002 Device 024: ID 1d50:6018 OpenMoko, Inc. Black Magic Debug Probe (Application)",
        false,
    );
    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--vidpid", "1d50:6018"],
        "Bus 002 Device 024: ID 1d50:6018 OpenMoko, Inc. Black Magic Debug Probe (Application)",
        false,
    );
    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--vidpid", "1d6b:"],
        r#"Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 002 Device 001: ID 1d6b:0001 Linux Foundation 1.1 root hub
Bus 003 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 004 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub"#,
        true,
    );
    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--vidpid", "dfgdfg"],
    );
}

/// Tests lsusb -s bus:devno filter
#[test]
fn test_lsusb_show() {
    let env = common::TestEnv::new();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--show", "24"],
        "Bus 002 Device 024: ID 1d50:6018 OpenMoko, Inc. Black Magic Debug Probe (Application)",
        false,
    );
    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--show", "2:24"],
        "Bus 002 Device 024: ID 1d50:6018 OpenMoko, Inc. Black Magic Debug Probe (Application)",
        false,
    );
    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--show", "2:"],
        r#"Bus 002 Device 022: ID 203a:fffe PARALLELS Virtual USB1.1 HUB
Bus 002 Device 024: ID 1d50:6018 OpenMoko, Inc. Black Magic Debug Probe (Application)
Bus 002 Device 023: ID 1366:1050 SEGGER J-Link
Bus 002 Device 001: ID 1d6b:0001 Linux Foundation 1.1 root hub"#,
        false,
    );
    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--show", "d"],
    );
}

/// Only tests contains first line...full verbose is not exactly the same but too difficult to match!
#[test]
fn test_lsusb_device() {
    let env = common::TestEnv::new();

    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--device", "/dev/bus/usb/002/024"],
        "Bus 002 Device 024: ID 1d50:6018 OpenMoko, Inc. Black Magic Debug Probe (Application)",
        true,
    );
    env.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--device", "/dev/bus/usb/002/001"],
        "Bus 002 Device 001: ID 1d6b:0001 Linux Foundation 1.1 root hub",
        true,
    );
    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--device", "/dev/blah/002/001"],
    );
    env.assert_failure(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--device", "/dev/bus/usb/002"],
    );
}
