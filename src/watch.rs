//! Watch for USB devices being connected and disconnected.
//!
//! TODO ideas:
//!
//! - Use cyme::display
//! - Make this into a full TUI with expanding device details
//! - Adjustable PrintSettings with keybindings
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use nusb::hotplug::HotplugEvent;
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cyme::display::*;
use cyme::error::{Error, ErrorKind, Result};
use cyme::profiler::{Device, SystemProfile, WatchEvent};
use futures_lite::stream::StreamExt;

pub fn watch_usb_devices(
    mut spusb: SystemProfile,
    mut print_settings: PrintSettings,
) -> Result<()> {
    print_settings.device_blocks = Some(DeviceBlocks::default_blocks(print_settings.verbosity > 0));
    print_settings
        .device_blocks
        .as_mut()
        .map(|b| b.push(DeviceBlocks::LastEvent));

    let stop_flag = Arc::new(Mutex::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);

    let mut stdout = stdout();
    execute!(stdout, terminal::Clear(terminal::ClearType::All))
        .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;
    let watch = nusb::watch_devices().map_err(|e| Error::new(ErrorKind::Nusb, &e.to_string()))?;

    // first draw
    draw_devices(&spusb, &print_settings)?;

    thread::spawn(move || {
        futures_lite::future::block_on(async {
            let mut watch_stream = watch;

            while let Some(event) = watch_stream.next().await {
                match event {
                    HotplugEvent::Connected(device) => {
                        // TODO profile device with extra using nusb profiler
                        // requires to be part of crate
                        let mut cyme_device: Device = Device::from(&device);
                        cyme_device.last_event = Some(WatchEvent::Connected(chrono::Local::now()));
                        // is it existing? TODO this is a mess, need to take existing, put devices into new and replace since might have new descriptors
                        if let Some(existing) = spusb.get_node_mut(&cyme_device.port_path()) {
                            let devices = std::mem::take(&mut existing.devices);
                            cyme_device.devices = devices;
                            *existing = cyme_device;
                        // else we have to stick into tree at correct place
                        // TODO re-sort?
                        } else if cyme_device.is_trunk_device() {
                            let bus = spusb.get_bus_mut(cyme_device.location_id.bus).unwrap();
                            if let Some(bd) = bus.devices.as_mut() {
                                bd.push(cyme_device);
                            } else {
                                bus.devices = Some(vec![cyme_device]);
                            }
                        } else if let Ok(parent_path) = cyme_device.parent_path() {
                            if let Some(parent) = spusb.get_node_mut(&parent_path) {
                                if let Some(bd) = parent.devices.as_mut() {
                                    bd.push(cyme_device);
                                } else {
                                    parent.devices = Some(vec![cyme_device]);
                                }
                            }
                        }
                    }
                    HotplugEvent::Disconnected(id) => {
                        if let Some(device) = spusb.get_id_mut(&id) {
                            device.last_event =
                                Some(WatchEvent::Disconnected(chrono::Local::now()));
                        }
                    }
                }
                draw_devices(&spusb, &print_settings).unwrap();
            }
        });
    });

    // Thread to listen for keyboard events
    thread::spawn(move || loop {
        if event::poll(Duration::from_millis(100)).unwrap() {
            if let Event::Key(KeyEvent { code, .. }) = event::read().unwrap() {
                if matches!(code, KeyCode::Char('q')) || matches!(code, KeyCode::Esc) {
                    let mut stop = stop_flag_clone.lock().unwrap();
                    *stop = true;
                    break;
                }
            }
        }
    });

    // Main loop to check for stop flag
    loop {
        let stop = stop_flag.lock().unwrap();
        if *stop {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn draw_devices(spusb: &SystemProfile, print_settings: &PrintSettings) -> Result<()> {
    let mut stdout = stdout();
    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::All)
    )
    .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    // TODO change color based on event in print? or post print?
    cyme::display::print_sp_usb(spusb, print_settings);

    Ok(())
}
