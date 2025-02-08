//! Watch for USB devices being connected and disconnected.
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use std::io::stdout;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use cyme::display::*;
use cyme::error::{Error, ErrorKind, Result};
use cyme::profiler::{watch::SystemProfileStreamBuilder, Filter, SystemProfile};
use futures_lite::stream::StreamExt;

enum WatchEvent {
    Draw,
    Stop,
}

pub fn watch_usb_devices(
    spusb: SystemProfile,
    filter: Option<Filter>,
    mut print_settings: PrintSettings,
) -> Result<()> {
    print_settings.watch_mode = true;
    if print_settings.device_blocks.is_none() {
        print_settings.device_blocks = Some(DeviceBlocks::default_watch_blocks(
            print_settings.verbosity > 0,
        ));
    }

    let mut stdout = stdout();
    execute!(
        stdout,
        cursor::Hide,
        terminal::Clear(terminal::ClearType::All)
    )
    .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    terminal::enable_raw_mode()?;

    // first draw
    draw_devices(&spusb, &print_settings)?;

    // pass spusb to stream builder, will get Arc<Mutex<SystemProfile>> back below
    let mut profile_stream = SystemProfileStreamBuilder::new()
        .with_spusb(spusb)
        .is_verbose(true) // because print_settings can change verbosity, always capture full device data
        .build()
        .map_err(|e| Error::new(ErrorKind::Nusb, &e.to_string()))?;

    let (tx, rx) = mpsc::channel::<WatchEvent>();
    let print_settings = Arc::new(Mutex::new(print_settings));
    let filter = Mutex::new(filter);
    // get a reference to the SystemProfile now since profile_stream can't be moved
    // main thread needs to draw with SystemProfile outside of the stream
    let spusb = profile_stream.get_profile();

    let print_settings_clone = Arc::clone(&print_settings);

    let tx_clone = tx.clone();
    thread::spawn(move || {
        futures_lite::future::block_on(async {
            while profile_stream.next().await.is_some() {
                // local spusb Arc<Mutex<SystemProfile>> is updated since we have a reference to it
                tx_clone.send(WatchEvent::Draw).unwrap();
            }
        });
    });

    // Thread to listen for keyboard events
    thread::spawn(move || loop {
        if event::poll(Duration::from_millis(100)).unwrap() {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read().unwrap()
            {
                if match (code, modifiers) {
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        tx.send(WatchEvent::Stop).unwrap();
                        break;
                    }
                    (KeyCode::Char('v'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.verbosity = (print_settings.verbosity + 1) % 4;
                        true
                    }
                    (KeyCode::Char('t'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.tree = !print_settings.tree;
                        true
                    }
                    (KeyCode::Char('h'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.headings = !print_settings.headings;
                        true
                    }
                    (KeyCode::Char('m'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.more = !print_settings.more;
                        true
                    }
                    (KeyCode::Char('d'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.decimal = !print_settings.decimal;
                        true
                    }
                    // TODO:
                    // filter pop-up enter filter - / like vim?
                    // sort pop-up enter sort
                    _ => false,
                } {
                    tx.send(WatchEvent::Draw).unwrap();
                }
            }
        }
    });

    // Main event loop
    loop {
        match rx.recv() {
            Ok(WatchEvent::Draw) => {
                let print_settings = print_settings_clone.lock().unwrap();
                let mut spusb = spusb.lock().unwrap();
                cyme::display::prepare(&mut spusb, &filter.lock().unwrap(), &print_settings);
                draw_devices(&spusb, &print_settings)?;
            }
            Ok(WatchEvent::Stop) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    execute!(stdout, cursor::Show)
        .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    terminal::disable_raw_mode()?;

    Ok(())
}

fn draw_devices(spusb: &SystemProfile, print_settings: &PrintSettings) -> Result<()> {
    let mut stdout = stdout();
    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::All),
    )
    .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    // TODO change color based on event in print? or post print?
    // status bar with key bindings
    // header bar
    let mut dw = DisplayWriter::new(&mut stdout);
    dw.set_raw_mode(true);
    dw.print_sp_usb(spusb, print_settings);

    Ok(())
}
