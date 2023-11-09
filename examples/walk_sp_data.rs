use cyme::lsusb::profiler;
use cyme::system_profiler::USBDevice;

fn recusive_map_devices(device: &USBDevice) {
    // the alternate format will print with colour
    println!("Device: {:#}", device);
    device.devices.as_ref().map(|v| {
        for d in v {
            recusive_map_devices(d)
        }
    });
}

fn main() -> Result<(), String> {
    // get all system devices
    let sp_usb = profiler::get_spusb(false)
        .map_err(|e| format!("Failed to gather system USB data from libusb, Error({})", e))?;

    // SPUSBDataType contains buses...
    for bus in sp_usb.buses {
        // which may contain devices...
        bus.devices.as_ref().map(|devices| {
            // to walk all the devices, since each device can have devices attached, call a recursive function
            for device in devices {
                recusive_map_devices(device);
            }
        });
    }

    Ok(())
}
