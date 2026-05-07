/// This example shows how to use the FilterGroup to filter out devices that match certain criteria
///
/// See [`FilterGroup`] docs for more information
use cyme::profiler::{self, Filter, FilterGroup};
use cyme::usb::BaseClass;

fn main() -> Result<(), String> {
    // get all system devices
    let mut sp_usb = profiler::get_spusb()
        .map_err(|e| format!("Failed to gather system USB data from libusb, Error({e})"))?;

    // create a filter group with a single filter that matches devices with the HID class
    let filter = FilterGroup::from(Filter::new_with_class(BaseClass::Hid));

    // will retain only the buses that have devices that match the filter - parent devices such as hubs with a HID device will be retained
    filter.retain_buses(&mut sp_usb.buses);
    sp_usb
        .buses
        .retain(|b| b.devices.as_ref().is_some_and(|d| d.is_empty()));

    // if one does not care about the tree, flatten the devices and do manually
    // let hid_devices = sp_usb.flatten_devices().iter().filter(|d| d.class == Some(BaseClass::HID));
    if sp_usb.buses.is_empty() {
        println!("No HID devices found");
    } else {
        println!("Found HID devices");
    }

    Ok(())
}
