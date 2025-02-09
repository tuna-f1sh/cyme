//! Watch for USB devices being connected and disconnected.
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
};
use std::io::stdout;
use std::io::Write;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use cyme::display::*;
use cyme::error::{Error, ErrorKind, Result};
use cyme::profiler::{watch::SystemProfileStreamBuilder, Filter, SystemProfile};
use futures_lite::stream::StreamExt;

enum WatchEvent {
    DrawDevices,
    ScrollUp(usize),
    ScrollDown(usize),
    Draw,
    Stop,
}

#[derive(Debug, Default)]
struct DisplayData {
    buffer: Vec<u8>,
    max_offset: usize,
    scroll_offset: usize,
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
        // alternative screen like other terminal apps so main screen buffer isn't cleared
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(terminal::ClearType::All)
    )
    .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    terminal::enable_raw_mode()?;
    let mut display_data = DisplayData::default();

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
    // first draw
    tx.send(WatchEvent::DrawDevices).unwrap();

    let print_settings_clone = Arc::clone(&print_settings);

    // Thread to listen for USB profiling events
    let tx_clone = tx.clone();
    thread::spawn(move || {
        futures_lite::future::block_on(async {
            while profile_stream.next().await.is_some() {
                // local spusb Arc<Mutex<SystemProfile>> is updated since we have a reference to it
                tx_clone.send(WatchEvent::DrawDevices).unwrap();
            }
        });
    });

    // Thread to listen for terminal events
    thread::spawn(move || loop {
        match event::read().unwrap() {
            Event::Resize(_, _) => {
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            Event::Mouse(MouseEvent { kind, .. }) => {
                if kind == MouseEventKind::ScrollUp {
                    tx.send(WatchEvent::ScrollUp(1)).unwrap();
                } else if kind == MouseEventKind::ScrollDown {
                    tx.send(WatchEvent::ScrollDown(1)).unwrap();
                }
            }
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                match (code, modifiers) {
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        tx.send(WatchEvent::Stop).unwrap();
                        break;
                    }
                    (KeyCode::Char('v'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.verbosity = (print_settings.verbosity + 1) % 4;
                        tx.send(WatchEvent::DrawDevices).unwrap();
                    }
                    (KeyCode::Char('t'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.tree = !print_settings.tree;
                        tx.send(WatchEvent::DrawDevices).unwrap();
                    }
                    (KeyCode::Char('h'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.headings = !print_settings.headings;
                        tx.send(WatchEvent::DrawDevices).unwrap();
                    }
                    (KeyCode::Char('m'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.more = !print_settings.more;
                        tx.send(WatchEvent::DrawDevices).unwrap();
                    }
                    (KeyCode::Char('d'), _) => {
                        let mut print_settings = print_settings.lock().unwrap();
                        print_settings.decimal = !print_settings.decimal;
                        tx.send(WatchEvent::DrawDevices).unwrap();
                    }
                    (KeyCode::Char('j'), _) | (KeyCode::PageDown, _) => {
                        tx.send(WatchEvent::ScrollDown(1)).unwrap();
                        tx.send(WatchEvent::Draw).unwrap();
                    }
                    (KeyCode::Char('k'), _) | (KeyCode::PageUp, _) => {
                        tx.send(WatchEvent::ScrollUp(1)).unwrap();
                        tx.send(WatchEvent::Draw).unwrap();
                    }
                    // TODO:
                    // filter pop-up enter filter - / like vim?
                    // sort pop-up enter sort
                    _ => (),
                };
            }
            _ => {}
        }
    });

    // Main event loop
    // manages the display and listens for events
    loop {
        match rx.recv() {
            Ok(WatchEvent::ScrollUp(n)) => {
                display_data.scroll_offset = display_data.scroll_offset.saturating_sub(n);
            }
            Ok(WatchEvent::ScrollDown(n)) => {
                display_data.scroll_offset = display_data
                    .max_offset
                    .min(display_data.scroll_offset.saturating_add(n));
            }
            Ok(WatchEvent::DrawDevices) => {
                let print_settings = print_settings_clone.lock().unwrap();
                let mut spusb = spusb.lock().unwrap();
                cyme::display::prepare(&mut spusb, &filter.lock().unwrap(), &print_settings);
                draw_devices_buffered(&spusb, &print_settings, &mut display_data)?;
                draw(&mut display_data, &print_settings)?;
            }
            Ok(WatchEvent::Draw) => {
                let print_settings = print_settings_clone.lock().unwrap();
                draw(&mut display_data, &print_settings)?;
            }
            Ok(WatchEvent::Stop) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        terminal::LeaveAlternateScreen,
        cursor::Show,
    )
    .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    terminal::disable_raw_mode()?;

    Ok(())
}

fn draw_devices_buffered(
    spusb: &SystemProfile,
    print_settings: &PrintSettings,
    display_data: &mut DisplayData,
) -> Result<()> {
    // use a Vec<u8> buffer instead of stdout
    // so we can scroll the output with offset
    display_data.buffer.clear();
    let mut dw = DisplayWriter::new(&mut display_data.buffer);
    dw.set_raw_mode(true);
    dw.print_sp_usb(spusb, print_settings);

    Ok(())
}

fn draw_footer<W: Write>(writer: &mut W, print_settings: &PrintSettings) -> Result<()> {
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    // construct footer message showing *toggle outcome* (what will happen on key press)
    let verbosity = if print_settings.verbosity == 3 {
        String::from("0")
    } else {
        (print_settings.verbosity + 1).to_string()
    };
    let footer = format!(
        " [q] Quit  [v] Verbosity (→ {})  [t] Tree (→ {})  [h] Headings (→ {})  [m] More (→ {})  [d] Decimal (→ {}) ",
        verbosity,
        if print_settings.tree { "Off" } else { "On" },
        if print_settings.headings { "Off" } else { "On" },
        if print_settings.more { "Off" } else { "On" },
        if print_settings.decimal { "Off" } else { "On" }
    );

    // move cursor to last row
    execute!(
        writer,
        cursor::MoveTo(0, term_height - 1),
        SetForegroundColor(Color::Black),
        SetBackgroundColor(Color::Green),
        Print(format!("{:<width$}", footer, width = term_width as usize)),
        ResetColor
    )?;

    Ok(())
}

fn draw(display_data: &mut DisplayData, print_settings: &PrintSettings) -> Result<()> {
    let mut stdout = stdout();
    let (_, term_height) = terminal::size().unwrap_or((80, 24));
    let footer_height = 1;
    let available_rows = term_height.saturating_sub(footer_height);

    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::All),
    )
    .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

    // convert buffer to string and split into lines
    let output = String::from_utf8_lossy(&display_data.buffer);
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();

    display_data.max_offset = lines.len().saturating_sub(available_rows as usize);
    // clamp ensures if output contracts fully scrolled, one doesn't have to *overscroll* back
    display_data.scroll_offset = display_data.scroll_offset.min(display_data.max_offset);

    // print the visible portion of the buffer
    for line in lines
        .iter()
        .skip(display_data.scroll_offset)
        .take(available_rows as usize)
    {
        write!(stdout, "{}\n\r", line)?;
    }

    // status bar with key bindings
    draw_footer(&mut stdout, print_settings)?;

    stdout.flush()?;

    Ok(())
}
