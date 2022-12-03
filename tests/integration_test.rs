mod common;

#[ignore]
#[test]
fn test_json_round_trip() {
    let te = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::CYME_LIBUSB_LINUX_TREE_DUMP);

    te.assert_output_json(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--json", "--tree"],
        comp.as_str(),
    );
}
