/// This example shows how to use the USBFilter to filter out devices that match a certain criteria
///
/// See [`USBFilter`] docs for more information
use cyme::profiler;
use cyme::system_profiler::USBFilter;
use cyme::usb::ClassCode;

fn main() -> Result<(), String> {
    // get all system devices
    let mut sp_usb = profiler::get_spusb()
        .map_err(|e| format!("Failed to gather system USB data from libusb, Error({})", e))?;

    // if one does want the tree, use the utility
    let filter = USBFilter {
        class: Some(ClassCode::HID),
        ..Default::default()
    };

    // will retain only the buses that have devices that match the filter - parent devices such as hubs with a HID device will be retained
    filter.retain_buses(&mut sp_usb.buses);
    sp_usb
        .buses
        .retain(|b| b.devices.as_ref().map_or(false, |d| d.is_empty()));

    // if one does not care about the tree, flatten the devices and do manually
    // let hid_devices = sp_usb.flatten_devices().iter().filter(|d| d.class == Some(ClassCode::HID));

    if sp_usb.buses.is_empty() {
        println!("No HID devices found");
    } else {
        println!("Found HID devices");
    }

    Ok(())
}
