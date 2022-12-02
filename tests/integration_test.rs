extern crate cyme;

mod common;

/// Tests lsusb with no args compatiability mode
#[test]
fn test_lsusb_list() {
    let te = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::LSUSB_OUTPUT);

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb"],
        comp.as_str(),
        false,
    );
}

/// Tests lsusb --tree compatiability mode
///
/// Requires feature udev because comparison contains drivers
#[cfg(feature = "udev")]
#[test]
fn test_lsusb_tree() {
    let te = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::LSUSB_TREE_OUTPUT);

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--tree"],
        comp.as_str(),
        false,
    );
}

/// Tests lsusb --tree fully verbose compatiability mode
///
/// Requires feature udev because comparison contains drivers
#[cfg(feature = "udev")]
#[test]
fn test_lsusb_tree_verbose() {
    let te = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::LSUSB_TREE_OUTPUT_VERBOSE);

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb", "--tree", "-vvv"],
        comp.as_str(),
        false,
    );
}
