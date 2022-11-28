#![allow(dead_code)]
use std::fs::File;
use std::io::{BufReader, Read};

extern crate cyme;

pub fn system_profiler_dump() -> BufReader<File> {
    let f = File::open("./test/data/system_profiler_dump.json").expect("Unable to open json dump file");
    BufReader::new(f)
}

pub fn cyme_devices_dump() -> BufReader<File> {
    let f = File::open("./test/data/cyme_devices_dump.json").expect("Unable to open json dump file");
    BufReader::new(f)
}

pub fn cyme_tree_dump() -> BufReader<File> {
    let f = File::open("./test/data/cyme_tree_dump.json").expect("Unable to open json dump file");
    BufReader::new(f)
}

pub fn sp_data_from_system_profiler() -> cyme::system_profiler::SPUSBDataType {
    let mut br = system_profiler_dump();
    let mut data = String::new();
    br.read_to_string(&mut data).expect("Unable to read string");

    serde_json::from_str::<cyme::system_profiler::SPUSBDataType>(&data).unwrap()
}

pub fn bus_from_system_profiler() -> cyme::system_profiler::USBDevice {
    let mut sp_data = sp_data_from_system_profiler();
    sp_data.buses[0].get_node_mut("blah").expect("Test device missing from dump").to_owned()
}

pub fn device_from_system_profiler() -> cyme::system_profiler::USBDevice {
    let mut sp_data = sp_data_from_system_profiler();
    sp_data.buses[0].get_node_mut("blah").expect("Test device missing from dump").to_owned()
}
