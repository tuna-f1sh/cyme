//! Watch for USB devices being connected and disconnected.
//!
//! TODO ideas:
//!
//! - Use cyme::display
//! - Make this into a full TUI with expanding device details
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use nusb::{hotplug::HotplugEvent, DeviceId, DeviceInfo};
use std::collections::HashMap;
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cyme::display::*;
use cyme::error::{Error, ErrorKind, Result};
use cyme::profiler::{Device, WatchEvent};
use futures_lite::stream::StreamExt;

pub fn watch_usb_devices() -> Result<()> {
    // last connected time none means connected
    // TODO struct so we can expand
    let mut devices: HashMap<DeviceId, (DeviceInfo, Option<WatchEvent>)> =
        nusb::list_devices()?.map(|d| (d.id(), (d, None))).collect();

    let stop_flag = Arc::new(Mutex::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);

    let mut stdout = stdout();
    execute!(stdout, terminal::Clear(terminal::ClearType::All))
        .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;
    let watch = nusb::watch_devices().map_err(|e| Error::new(ErrorKind::Nusb, &e.to_string()))?;

    // first draw
    draw_devices(&devices)?;

    thread::spawn(move || {
        futures_lite::future::block_on(async {
            let mut watch_stream = watch;

            while let Some(event) = watch_stream.next().await {
                match event {
                    HotplugEvent::Connected(device) => {
                        devices.insert(
                            device.id(),
                            (device, Some(WatchEvent::Connected(chrono::Local::now()))),
                        );
                    }
                    HotplugEvent::Disconnected(id) => {
                        if let Some((_, dt)) = devices.get_mut(&id) {
                            *dt = Some(WatchEvent::Disconnected(chrono::Local::now()));
                        }
                    }
                }
                draw_devices(&devices).unwrap();
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

fn draw_devices(devices: &HashMap<DeviceId, (DeviceInfo, Option<WatchEvent>)>) -> Result<()> {
    let mut stdout = stdout();
    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::All)
    )
    .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    let cyme_devices: Vec<cyme::profiler::Device> = devices
        .iter()
        .map(|(id, (device, last_seen))| {
            let mut device: cyme::profiler::Device = device.into();
            device.id = Some(*id);
            device.last_event = *last_seen;
            device
        })
        .collect();

    let mut print_settings = PrintSettings::default();
    print_settings.colours = Some(cyme::colour::ColourTheme::default());
    let mut device_blocks = DeviceBlocks::default_blocks(false);
    device_blocks.push(DeviceBlocks::LastEvent);
    print_settings.device_blocks = Some(device_blocks);

    // TODO use cyme::display
    cyme::display::print_flattened_devices(
        &cyme_devices.iter().collect::<Vec<&Device>>(),
        &print_settings,
    );

    Ok(())
}
