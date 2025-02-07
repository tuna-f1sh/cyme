//! Watch for USB devices being connected and disconnected.
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cyme::display::*;
use cyme::error::{Error, ErrorKind, Result};
use cyme::profiler::{watch::SystemProfileStreamBuilder, Filter, SystemProfile};
use futures_lite::stream::StreamExt;

pub fn watch_usb_devices(
    spusb: SystemProfile,
    filter: Option<Filter>,
    mut print_settings: PrintSettings,
) -> Result<()> {
    print_settings.watch_mode = true;
    if print_settings.device_blocks.is_none() {
        print_settings.device_blocks =
            Some(DeviceBlocks::default_blocks(print_settings.verbosity > 0));
    }
    if let Some(b) = print_settings.device_blocks.as_mut() {
        b.insert(0, DeviceBlocks::EventIcon);
        b.push(DeviceBlocks::LastEvent);
    }

    let stop_flag = Arc::new(Mutex::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);

    let mut stdout = stdout();
    execute!(stdout, terminal::Clear(terminal::ClearType::All))
        .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;
    // TODO this requires a rethink of writer since raw needs \n\r
    //terminal::enable_raw_mode()?;

    // first draw
    draw_devices(&spusb, &print_settings)?;

    let profile_stream = SystemProfileStreamBuilder::new()
        .with_spusb(spusb)
        .is_verbose(print_settings.verbosity > 0)
        .build()
        .map_err(|e| Error::new(ErrorKind::Nusb, &e.to_string()))?;

    thread::spawn(move || {
        futures_lite::future::block_on(async {
            futures_lite::pin!(profile_stream);
            while let Some(spusb) = profile_stream.next().await {
                let mut spusb = spusb.lock().unwrap();
                // HACK this is prabably over kill, does sort and filter of whole tree every time - filter could be done once and on insert instead
                cyme::display::prepare(&mut spusb, &filter, &print_settings);
                draw_devices(&spusb, &print_settings).unwrap();
            }
        });
    });

    // Thread to listen for keyboard events
    thread::spawn(move || loop {
        if event::poll(Duration::from_millis(100)).unwrap() {
            if let Event::Key(KeyEvent { code, .. }) = event::read().unwrap() {
                if matches!(code, KeyCode::Char('q')) || matches!(code, KeyCode::Esc) {
                    *stop_flag_clone.lock().unwrap() = true;
                    break;
                }
            }
        }
    });

    // Main loop to check for stop flag
    loop {
        if *stop_flag.lock().unwrap() {
            log::info!("Exiting watch mode");
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    //terminal::disable_raw_mode()?;

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
