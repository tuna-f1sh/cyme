//! Where the magic happens for `cyme` binary!
mod watch;

fn main() {
    match watch::watch_usb_devices() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
