fn get_driver(port_path: String) -> String {
    let device = udev::Device::from_syspath(format!("/sys/bus/usb/devices/{}", port_path));
    device.driver().to_string();
}
