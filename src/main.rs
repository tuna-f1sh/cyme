mod system_profiler;

// fn print_recursive(devices: Vec<USBDevice>) {
//     for device in devices {
//         // print the device details
//         println!("{:#}", device);
//         // print all devices with this device - if hub for example
//         device.devices.map(print_recursive);
//     }
// }

fn main() {
    let sp_usb = system_profiler::get_spusb().unwrap();

    println!("{:#}", sp_usb);
}
