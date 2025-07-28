use cyme::profiler::{self, Device};

fn recursive_map_devices(device: &Device) {
    // the alternate format will print with colour
    println!("Device: {device:#}");
    if let Some(v) = device.devices.as_ref() {
        for d in v {
            recursive_map_devices(d)
        }
    };
}

fn main() -> Result<(), String> {
    // get all system devices
    let sp_usb = profiler::get_spusb()
        .map_err(|e| format!("Failed to gather system USB data from libusb, Error({e})"))?;

    // SPUSBDataType contains buses...
    for bus in sp_usb.buses {
        // which may contain devices...
        if let Some(devices) = bus.devices {
            // to walk all the devices, since each device can have devices attached, call a recursive function
            for device in devices {
                recursive_map_devices(&device);
            }
        }
    }

    Ok(())
}
