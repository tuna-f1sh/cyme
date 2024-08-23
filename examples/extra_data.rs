use cyme::profiler;

fn main() -> Result<(), String> {
    // get all system devices - this time with extra data which contain the Configuration, driver data (with udev)
    let sp_usb = profiler::get_spusb_with_extra()
        .map_err(|e| format!("Failed to gather system USB data from libusb, Error({})", e))?;

    let devices = sp_usb.flattened_devices();

    // print all configurations
    for device in devices {
        if let Some(extra) = device.extra.as_ref() {
            println!("Device {} has configurations:", device.name);
            for c in extra.configurations.iter() {
                println!("{:?}", c);
            }
        };
    }

    Ok(())
}
