use std::error::Error;
use std::path::Path;
use udev as udevlib;

pub fn get_driver(port_path: &String) -> Result<Option<String>, Box<dyn Error>> {
    let path: String = format!("/sys/bus/usb/devices/{}", port_path);
    let device = udevlib::Device::from_syspath(&Path::new(&path))?;
    log::debug!("Got device driver {:?}", device.driver());
    Ok(device.driver().map(|s| s.to_str().unwrap_or("").to_string()))
}
