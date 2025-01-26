use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use nusb::{hotplug::HotplugEvent, DeviceId, DeviceInfo};
use std::collections::HashMap;
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossterm::{style::Color, style::SetForegroundColor};
use cyme::error::{Error, ErrorKind, Result};
use futures_lite::stream::StreamExt;
use std::time::SystemTime;

pub fn watch_usb_devices() -> Result<()> {
    // last connected time none means connected
    let mut devices: HashMap<DeviceId, (DeviceInfo, Option<SystemTime>)> =
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
                        devices.insert(device.id(), (device, None));
                    }
                    HotplugEvent::Disconnected(id) => {
                        if let Some((_, dt)) = devices.get_mut(&id) {
                            *dt = Some(SystemTime::now());
                        }
                    }
                }
                draw_devices(&devices).unwrap();
            }
        });
    });

    writeln!(stdout, "Press 'q' to quit").unwrap();

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

fn draw_devices(devices: &HashMap<DeviceId, (DeviceInfo, Option<SystemTime>)>) -> Result<()> {
    let mut stdout = stdout();
    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::All)
    )
    .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    for (_id, (device, last_seen)) in devices {
        if let Some(last_seen_time) = last_seen {
            execute!(stdout, SetForegroundColor(Color::Grey)).unwrap();
            writeln!(
                stdout,
                "{} - Disconnected (last seen: {:?})",
                device.product_string().unwrap_or("Unknown device"),
                last_seen_time
            )
            .unwrap();
        } else {
            execute!(stdout, SetForegroundColor(Color::White)).unwrap();
            writeln!(
                stdout,
                "{} - Connected",
                device.product_string().unwrap_or("Unknown device")
            )
            .unwrap();
        }
    }

    Ok(())
}
