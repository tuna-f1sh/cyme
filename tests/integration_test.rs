extern crate cyme;

mod common;

#[ignore]
#[test]
fn test_lsusb_list() {
    let te = common::TestEnv::new();

    let comp = common::read_dump_to_string(common::LSUSB_OUTPUT);
    println!("Comparing {}", comp);

    te.assert_output(
        Some(common::CYME_LIBUSB_LINUX_TREE_DUMP),
        &["--lsusb"],
        comp.as_str(),
    );
}
