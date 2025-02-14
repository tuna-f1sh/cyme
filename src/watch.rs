//! Watch for USB devices being connected and disconnected.
use clap::ValueEnum;
use colored::*;
use crossterm::{
    cursor,
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
    },
    execute,
    style::{Print, ResetColor},
    terminal,
};
use futures_lite::stream::StreamExt;
use std::io::stdout;
use std::io::Write;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use super::parse_vidpid;
use cyme::display::*;
use cyme::error::{Error, ErrorKind, Result};
use cyme::profiler::{watch::SystemProfileStreamBuilder, Filter, SystemProfile};

#[derive(Debug, Clone)]
enum WatchEvent {
    DrawDevices,
    ScrollUp(usize),
    ScrollUpPage,
    ScrollDown(usize),
    ScrollDownPage,
    MoveUp(usize),
    MoveDown(usize),
    EditFilter(FilterField),
    PushFilter(char),
    PopFilter,
    Draw,
    Enter,
    Stop,
}

#[derive(Debug, Clone)]
enum InputMode {
    Normal,
    EditingFilter {
        field: FilterField,
        buffer: String,
        original: Option<String>,
    },
}

#[derive(Debug, Clone)]
enum FilterField {
    Name,
    Serial,
    VidPid,
    Class,
}

#[derive(Debug)]
struct Display {
    buffer: Vec<u8>,
    spusb: Arc<Mutex<SystemProfile>>,
    print_settings: Arc<Mutex<PrintSettings>>,
    filter: Filter,
    input_mode: Arc<Mutex<InputMode>>,
    selected_line: Option<usize>,
    available_rows: usize,
    max_offset: usize,
    scroll_offset: usize,
}

fn set_filter(field: &FilterField, value: Option<String>, filter: &mut Filter) {
    match field {
        FilterField::Name => filter.name = value,
        FilterField::Serial => filter.serial = value,
        FilterField::VidPid => {
            if let Ok((vid, pid)) = parse_vidpid(&value.unwrap_or_default()) {
                filter.vid = vid;
                filter.pid = pid;
            } else {
                filter.vid = None;
                filter.pid = None;
            }
        }
        FilterField::Class => {
            filter.class = value.and_then(|s| cyme::usb::BaseClass::from_str(&s, true).ok())
        }
    }
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

    // pass spusb to stream builder, will get Arc<Mutex<SystemProfile>> back below
    let mut profile_stream = SystemProfileStreamBuilder::new()
        .with_spusb(spusb)
        .is_verbose(true) // because print_settings can change verbosity, always capture full device data
        .build()
        .map_err(|e| Error::new(ErrorKind::Nusb, &e.to_string()))?;

    let (tx, rx) = mpsc::channel::<WatchEvent>();
    let print_settings = Arc::new(Mutex::new(print_settings));
    let input_mode = Arc::new(Mutex::new(InputMode::Normal));
    // first draw
    tx.send(WatchEvent::DrawDevices).unwrap();

    let mut display = Display {
        buffer: Vec::new(),
        // get a reference to the SystemProfile now since profile_stream can't be moved
        // main thread needs to draw with SystemProfile outside of the stream
        spusb: profile_stream.get_profile(),
        print_settings: print_settings.clone(),
        filter: filter.unwrap_or_default(),
        input_mode: input_mode.clone(),
        available_rows: 0,
        max_offset: 0,
        scroll_offset: 0,
        selected_line: None,
    };

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
            Event::Key(key_event) => {
                input_mode.lock().unwrap().process_key_event(
                    key_event,
                    tx.clone(),
                    print_settings.clone(),
                );
            }
            _ => (),
        }
    });

    // Main event loop
    // manages the display and listens for events
    loop {
        match rx.recv() {
            Ok(WatchEvent::ScrollUp(n)) => {
                display.scroll_offset = display.scroll_offset.saturating_sub(n);
            }
            Ok(WatchEvent::ScrollUpPage) => {
                display.scroll_offset =
                    display.scroll_offset.saturating_sub(display.available_rows);
            }
            Ok(WatchEvent::ScrollDown(n)) => {
                display.scroll_offset = display
                    .max_offset
                    .min(display.scroll_offset.saturating_add(n));
            }
            Ok(WatchEvent::ScrollDownPage) => {
                display.scroll_offset = display
                    .max_offset
                    .min(display.scroll_offset.saturating_add(display.available_rows));
            }
            Ok(WatchEvent::MoveUp(n)) => {
                display.selected_line = display.selected_line.map(|l| l.saturating_sub(n));
            }
            Ok(WatchEvent::MoveDown(n)) => {
                display.selected_line = display.selected_line.map(|l| display.max_offset.max(l.saturating_add(n)));
            }
            Ok(WatchEvent::EditFilter(field)) => {
                let mut input_mode = display.input_mode.lock().unwrap();
                let original = match field {
                    FilterField::Name => display.filter.name.clone(),
                    FilterField::Serial => display.filter.serial.clone(),
                    FilterField::VidPid => match (display.filter.vid, display.filter.pid) {
                        (Some(vid), Some(pid)) => Some(format!("{:04x}:{:04x}", vid, pid)),
                        (Some(vid), None) => Some(format!("{:04x}", vid)),
                        _ => None,
                    },
                    FilterField::Class => display.filter.class.map(|c| c.to_string()),
                };
                *input_mode = InputMode::EditingFilter {
                    field,
                    buffer: original.clone().unwrap_or_default(),
                    original,
                };
            }
            Ok(WatchEvent::PushFilter(c)) => {
                let mut input_mode = display.input_mode.lock().unwrap();
                if let InputMode::EditingFilter { field, buffer, .. } = &mut *input_mode {
                    buffer.push(c);
                    set_filter(field, Some(buffer.to_string()), &mut display.filter);
                }
            }
            Ok(WatchEvent::PopFilter) => {
                let mut input_mode = display.input_mode.lock().unwrap();
                if let InputMode::EditingFilter { field, buffer, .. } = &mut *input_mode {
                    if buffer.pop().is_none() {
                        set_filter(field, None, &mut display.filter);
                    } else {
                        set_filter(field, Some(buffer.to_string()), &mut display.filter);
                    }
                }
            }
            Ok(WatchEvent::Enter) => {
                let input_mode = display.input_mode.lock().unwrap().clone();
                if let InputMode::EditingFilter { .. } = input_mode {
                    *display.input_mode.lock().unwrap() = InputMode::Normal;
                    display.prepare_devices();
                    display.draw_devices()?;
                }
            }
            Ok(WatchEvent::DrawDevices) => {
                display.prepare_devices();
                display.draw_devices()?;
            }
            Ok(WatchEvent::Draw) => {
                display.draw()?;
            }
            Ok(WatchEvent::Stop) => {
                let input_mode = display.input_mode.lock().unwrap().clone();
                match input_mode {
                    InputMode::Normal => {
                        break;
                    }
                    InputMode::EditingFilter {
                        field, original, ..
                    } => {
                        set_filter(&field, original, &mut display.filter);
                        *display.input_mode.lock().unwrap() = InputMode::Normal;
                        display.prepare_devices();
                    }
                }
                display.draw_devices()?;
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

impl InputMode {
    fn process_normal_mode(
        event: KeyEvent,
        tx: mpsc::Sender<WatchEvent>,
        print_settings: Arc<Mutex<PrintSettings>>,
    ) {
        let KeyEvent {
            code, modifiers, ..
        } = event;
        match (code, modifiers) {
            (KeyCode::Char('q'), _) => {
                tx.send(WatchEvent::Stop).unwrap();
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
            (KeyCode::Char('j'), KeyModifiers::NONE) => {
                tx.send(WatchEvent::ScrollDown(1)).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Char('k'), KeyModifiers::NONE) => {
                tx.send(WatchEvent::ScrollUp(1)).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Char('j'), KeyModifiers::CONTROL) | (KeyCode::PageDown, _) => {
                tx.send(WatchEvent::ScrollDownPage).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) | (KeyCode::PageUp, _) => {
                tx.send(WatchEvent::ScrollUpPage).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Char('/'), _) => {
                tx.send(WatchEvent::EditFilter(FilterField::Name)).unwrap();
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('#'), _) => {
                tx.send(WatchEvent::EditFilter(FilterField::VidPid))
                    .unwrap();
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('c'), _) => {
                tx.send(WatchEvent::EditFilter(FilterField::Class)).unwrap();
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('s'), _) => {
                tx.send(WatchEvent::EditFilter(FilterField::Serial))
                    .unwrap();
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            // TODO:
            // filter pop-up enter filter - / like vim?
            // sort pop-up enter sort
            _ => (),
        };
    }

    fn process_key_event(
        &self,
        event: KeyEvent,
        tx: mpsc::Sender<WatchEvent>,
        print_settings: Arc<Mutex<PrintSettings>>,
    ) {
        let KeyEvent {
            code,
            modifiers,
            kind,
            ..
        } = event;
        if !matches!(kind, KeyEventKind::Press) {
            return;
        }
        match (code, modifiers) {
            // global keys
            (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                // will stop current context or exit the program
                tx.send(WatchEvent::Stop).unwrap();
            }
            (KeyCode::Enter, _) => {
                tx.send(WatchEvent::Enter).unwrap();
            }
            // others based on mode
            _ => match self {
                InputMode::Normal => {
                    Self::process_normal_mode(event, tx, print_settings);
                }
                InputMode::EditingFilter { field, .. } => match (code, modifiers) {
                    // only re-draw devices for string filter changes - others only once fully entered
                    (KeyCode::Char(c), _) => {
                        tx.send(WatchEvent::PushFilter(c)).unwrap();
                        match field {
                            FilterField::Serial | FilterField::Name => {
                                tx.send(WatchEvent::DrawDevices).unwrap()
                            }
                            _ => tx.send(WatchEvent::Draw).unwrap(),
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        tx.send(WatchEvent::PopFilter).unwrap();
                        match field {
                            FilterField::Serial | FilterField::Name => {
                                tx.send(WatchEvent::DrawDevices).unwrap()
                            }
                            _ => tx.send(WatchEvent::Draw).unwrap(),
                        }
                    }
                    _ => {}
                },
            },
        }
    }
}

impl Display {
    fn prepare_devices(&mut self) {
        let print_settings = self.print_settings.lock().unwrap();
        let mut spusb = self.spusb.lock().unwrap();
        cyme::display::prepare(&mut spusb, Some(&self.filter), &print_settings);
    }

    fn draw_devices(&mut self) -> Result<()> {
        // use a Vec<u8> buffer instead of stdout
        // so we can scroll the output with offset
        self.buffer.clear();
        let mut dw = DisplayWriter::new(&mut self.buffer);
        dw.set_raw_mode(true);

        {
            let spusb = self.spusb.lock().unwrap();
            let print_settings = self.print_settings.lock().unwrap();
            dw.print_sp_usb(&spusb, &print_settings);
        }

        self.draw()?;

        Ok(())
    }

    fn draw_footer<W: Write>(&mut self, writer: &mut W) -> Result<()> {
        let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

        // construct footer message showing *toggle outcome* (what will happen on key press)
        match self.input_mode.lock().unwrap().clone() {
            InputMode::Normal => {
                let print_settings = self.print_settings.lock().unwrap();
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
                ).bold();

                // move cursor to last row
                execute!(
                    writer,
                    cursor::MoveTo(0, term_height - 1),
                    terminal::Clear(terminal::ClearType::CurrentLine),
                    Print(
                        format!("{:<width$}", footer, width = term_width as usize)
                            .black()
                            .on_green()
                    ),
                    ResetColor
                )?;
            }
            InputMode::EditingFilter { field, buffer, .. } => {
                let mut footer = match field {
                    FilterField::Name => "Filter by Name: ".to_string(),
                    FilterField::Serial => "Filter by Serial: ".to_string(),
                    FilterField::VidPid => "Filter by VID:PID: ".to_string(),
                    FilterField::Class => "Filter by Class: ".to_string(),
                };
                footer.push_str(&buffer);
                execute!(
                    writer,
                    cursor::MoveTo(0, term_height - 1),
                    terminal::Clear(terminal::ClearType::CurrentLine),
                    Print(
                        format!("{:<width$}", footer, width = term_width as usize)
                            .black()
                            .on_yellow()
                    ),
                    ResetColor
                )?;
            }
        }

        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let mut stdout = stdout();
        let (_, term_height) = terminal::size().unwrap_or((80, 24));
        let footer_height = 1;
        self.available_rows = term_height.saturating_sub(footer_height) as usize;

        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All),
        )
        .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

        // convert buffer to string and split into lines
        let output = String::from_utf8_lossy(&self.buffer);
        let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();

        self.max_offset = lines.len().saturating_sub(self.available_rows);
        // clamp ensures if output contracts fully scrolled, one doesn't have to *overscroll* back
        self.scroll_offset = self.scroll_offset.min(self.max_offset);

        // print the visible portion of the buffer
        for line in lines
            .iter()
            .skip(self.scroll_offset)
            .take(self.available_rows)
        {
            write!(stdout, "{}\n\r", line)?;
        }

        // TODO selected device
        if let Some(_selected_device) = self.selected_line {}

        // status bar with key bindings
        self.draw_footer(&mut stdout)?;

        stdout.flush()?;

        Ok(())
    }
}
