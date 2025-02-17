//! Watch for USB devices being connected and disconnected.
use clap::ValueEnum;
use colored::*;
use crossterm::{
    cursor,
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
    },
    execute,
    style::Print,
    terminal,
};
use futures_lite::stream::StreamExt;
use std::io::stdout;
use std::io::Write;
use std::string::ToString;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use super::parse_vidpid;
use cyme::display::*;
use cyme::error::{Error, ErrorKind, Result};
use cyme::profiler::{watch::SystemProfileStreamBuilder, Filter, SystemProfile};

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum WatchEvent {
    ScrollUp(usize),
    ScrollUpPage,
    ScrollDown(usize),
    ScrollDownPage,
    MoveUp(usize),
    MoveDown(usize),
    EditFilter(FilterField),
    PushFilter(char),
    PopFilter,
    EditBlock(BlockType, BlockSelection),
    Error(String),
    Resize,
    DrawDevices,
    DrawEditBlocks,
    Draw,
    Enter,
    Stop,
}

/// Distinguish which set of blocks we’re editing
#[derive(Debug, Clone, Copy, strum_macros::Display)]
enum BlockType {
    Device,
    Bus,
    Config,
    Interface,
    Endpoint,
}

impl BlockType {
    fn next(&self) -> Self {
        match self {
            BlockType::Bus => BlockType::Device,
            BlockType::Device => BlockType::Config,
            BlockType::Config => BlockType::Interface,
            BlockType::Interface => BlockType::Endpoint,
            BlockType::Endpoint => BlockType::Bus,
        }
    }
}

/// Block selection state (index, max)
#[derive(Debug, Clone, Copy)]
enum BlockSelection {
    Enabled(usize, usize),
    Available(usize, usize),
}

impl BlockSelection {
    fn add(&self, n: usize) -> Self {
        match self {
            BlockSelection::Enabled(i, max) => {
                BlockSelection::Enabled(i.saturating_add(n).min(*max), *max)
            }
            BlockSelection::Available(i, max) => {
                BlockSelection::Available(i.saturating_add(n).min(*max), *max)
            }
        }
    }

    fn sub(&self, n: usize) -> Self {
        match self {
            BlockSelection::Enabled(i, max) => BlockSelection::Enabled(i.saturating_sub(n), *max),
            BlockSelection::Available(i, max) => {
                BlockSelection::Available(i.saturating_sub(n), *max)
            }
        }
    }

    fn set_max(&mut self, max: usize) {
        match self {
            BlockSelection::Enabled(_, m) => *m = max,
            BlockSelection::Available(_, m) => *m = max,
        }
    }

    fn next(&self) -> Self {
        match self {
            BlockSelection::Enabled(..) => BlockSelection::Available(0, 0),
            BlockSelection::Available(..) => BlockSelection::Enabled(0, 0),
        }
    }
}

#[derive(Debug, Clone)]
enum InputMode {
    Normal,
    EditingFilter {
        field: FilterField,
        buffer: Option<String>,
        original: Option<String>,
    },
    BlockEditor {
        block_type: BlockType,
        selected: BlockSelection,
    },
    Error(String),
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
    /// Size of current window
    terminal_size: (u16, u16),
    /// Currently selected line
    selected_line: Option<usize>,
    /// Number of rows available for output
    available_rows: usize,
    /// Maximum offset for scrolling
    max_offset: usize,
    /// Number of lines in the buffer
    lines: usize,
    /// Current scroll offset
    scroll_offset: usize,
}

fn set_filter(field: &FilterField, value: Option<String>, filter: &mut Filter) -> Result<()> {
    match field {
        FilterField::Name => filter.name = value,
        FilterField::Serial => filter.serial = value,
        FilterField::VidPid => match value {
            Some(s) => {
                let (vid, pid) = parse_vidpid(&s)?;
                filter.vid = vid;
                filter.pid = pid;
            }
            None => {
                filter.vid = None;
                filter.pid = None;
            }
        },
        FilterField::Class => match value {
            Some(s) => {
                filter.class = Some(
                    cyme::usb::BaseClass::from_str(&s, true)
                        .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))?,
                );
            }
            None => filter.class = None,
        },
    };

    Ok(())
}

pub fn watch_usb_devices(
    spusb: SystemProfile,
    filter: Option<Filter>,
    mut print_settings: PrintSettings,
) -> Result<()> {
    // set print mode to dynamic so we can update the display without re-running the profiler
    // non-destructively hides devices that don't match the filter etc.
    print_settings.print_mode = PrintMode::Dynamic;
    // make sure we have blocks
    if print_settings.device_blocks.is_none() {
        print_settings.device_blocks = Some(DeviceBlocks::default_watch_blocks(
            print_settings.verbosity > 0,
            print_settings.tree,
        ));
    }
    print_settings.bus_blocks = print_settings.bus_blocks.or(Some(BusBlocks::default_blocks(
        print_settings.verbosity > 0,
    )));
    print_settings.config_blocks =
        print_settings
            .config_blocks
            .or(Some(ConfigurationBlocks::default_blocks(
                print_settings.verbosity > 0,
            )));
    print_settings.interface_blocks =
        print_settings
            .interface_blocks
            .or(Some(InterfaceBlocks::default_blocks(
                print_settings.verbosity > 0,
            )));

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
        terminal_size: terminal::size().unwrap_or((80, 24)),
        lines: 0,
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
                tx.send(WatchEvent::Resize).unwrap();
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
                    .min(display.scroll_offset.saturating_add(display.max_offset));
            }
            Ok(WatchEvent::MoveUp(n)) => {
                match &mut *display.input_mode.lock().unwrap() {
                    InputMode::BlockEditor { selected, .. } => {
                        *selected = selected.sub(n);
                        // TODO update selected line so auto-scrolls
                    }
                    _ => {
                        display.selected_line = display
                            .selected_line
                            .map_or(Some(display.available_rows), |l| Some(l.saturating_sub(n)));
                    }
                };

                if let Some(l) = display.selected_line {
                    if l < display.scroll_offset {
                        display.scroll_offset = display.scroll_offset.saturating_sub(1);
                    }
                }
            }
            Ok(WatchEvent::MoveDown(n)) => {
                match &mut *display.input_mode.lock().unwrap() {
                    InputMode::BlockEditor { selected, .. } => {
                        *selected = selected.add(n);
                        log::info!("{:?}", selected);
                    }
                    _ => {
                        display.selected_line = display.selected_line.map_or(Some(1), |l| {
                            Some((display.lines - 1).min(l.saturating_add(n)))
                        })
                    }
                }

                if let Some(l) = display.selected_line {
                    if l >= display.available_rows {
                        display.scroll_offset = display
                            .max_offset
                            .min(display.scroll_offset.saturating_add(1));
                    }
                }
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
                    buffer: original.clone(),
                    original,
                };
            }
            Ok(WatchEvent::PushFilter(c)) => {
                let mut input_mode = display.input_mode.lock().unwrap();
                if let InputMode::EditingFilter { field, buffer, .. } = &mut *input_mode {
                    buffer.get_or_insert_with(String::new).push(c);
                    match field {
                        FilterField::Serial | FilterField::Name => {
                            if let Err(e) = set_filter(field, buffer.clone(), &mut display.filter) {
                                *input_mode = InputMode::Error(e.to_string());
                            }
                        }
                        _ => (),
                    }
                }
            }
            Ok(WatchEvent::PopFilter) => {
                let mut input_mode = display.input_mode.lock().unwrap();
                if let InputMode::EditingFilter { field, buffer, .. } = &mut *input_mode {
                    if let Some(b) = buffer {
                        let ret = b.pop();
                        if b.is_empty() | ret.is_none() {
                            buffer.take();
                        }
                    };
                    match field {
                        FilterField::Serial | FilterField::Name => {
                            if let Err(e) = set_filter(field, buffer.clone(), &mut display.filter) {
                                *input_mode = InputMode::Error(e.to_string());
                            }
                        }
                        _ => (),
                    }
                }
            }
            Ok(WatchEvent::Enter) => {
                let input_mode = display.input_mode.lock().unwrap().clone();
                match input_mode {
                    InputMode::EditingFilter { field, buffer, .. } => {
                        if let Err(e) = set_filter(&field, buffer, &mut display.filter) {
                            *display.input_mode.lock().unwrap() = InputMode::Error(e.to_string());
                        } else {
                            *display.input_mode.lock().unwrap() = InputMode::Normal;
                            display.prepare_devices();
                            display.draw_devices()?;
                        }
                    }
                    InputMode::BlockEditor { selected, .. } => match selected {
                        BlockSelection::Enabled(i, _) => {
                            let mut print_settings = display.print_settings.lock().unwrap();
                            let enabled_blocks = print_settings.device_blocks.as_mut().unwrap();
                            // keep at least one block enabled
                            if enabled_blocks.len() > 1 {
                                enabled_blocks.remove(i);
                            }
                        }
                        BlockSelection::Available(i, _) => {
                            let mut print_settings = display.print_settings.lock().unwrap();
                            let enabled_blocks = print_settings.device_blocks.as_mut().unwrap();
                            let all_blocks = DeviceBlocks::all_blocks().to_vec();
                            let available_blocks: Vec<DeviceBlocks> = all_blocks
                                .into_iter()
                                .filter(|b| !enabled_blocks.contains(b))
                                .collect();
                            if let Some(block) = available_blocks.get(i) {
                                enabled_blocks.push(*block);
                            }
                        }
                    },
                    _ => {
                        *display.input_mode.lock().unwrap() = InputMode::Normal;
                    }
                }
            }
            Ok(WatchEvent::Error(msg)) => {
                *display.input_mode.lock().unwrap() = InputMode::Error(msg);
                display.draw()?;
            }
            Ok(WatchEvent::Resize) => {
                // resize needs to redraw devices as columns may have changed but not block editor
                if !matches!(
                    &*display.input_mode.lock().unwrap(),
                    InputMode::BlockEditor { .. }
                ) {
                    display.print_settings.lock().unwrap().terminal_size = terminal::size().ok();
                    display.draw_devices()?;
                }
                display.draw()?;
            }
            Ok(WatchEvent::DrawDevices) => {
                display.prepare_devices();
                display.draw_devices()?;
            }
            Ok(WatchEvent::EditBlock(block_type, selected)) => {
                let mut input_mode = display.input_mode.lock().unwrap();
                *input_mode = InputMode::BlockEditor {
                    block_type,
                    selected,
                };
            }
            Ok(WatchEvent::DrawEditBlocks) => {
                log::info!("Draw edit blocks");
                display.draw_edit_blocks()?;
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
                    } => match set_filter(&field, original, &mut display.filter) {
                        Ok(_) => {
                            *display.input_mode.lock().unwrap() = InputMode::Normal;
                            display.prepare_devices();
                        }
                        Err(e) => {
                            // original must be bad so clear it
                            set_filter(&field, None, &mut display.filter)?;
                            *display.input_mode.lock().unwrap() = InputMode::Error(e.to_string());
                        }
                    },
                    _ => {
                        *display.input_mode.lock().unwrap() = InputMode::Normal;
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
            (KeyCode::Enter, _) => {
                tx.send(WatchEvent::Enter).unwrap();
            }
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
            (KeyCode::Up, _) => {
                tx.send(WatchEvent::MoveUp(1)).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Down, _) => {
                tx.send(WatchEvent::MoveDown(1)).unwrap();
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
            (KeyCode::Char('b'), _) => {
                tx.send(WatchEvent::EditBlock(
                    BlockType::Device,
                    BlockSelection::Enabled(0, 0),
                ))
                .unwrap();
                tx.send(WatchEvent::DrawEditBlocks).unwrap();
            }
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
                    (KeyCode::Enter, _) => {
                        tx.send(WatchEvent::Enter).unwrap();
                    }
                    _ => {}
                },
                InputMode::BlockEditor {
                    block_type,
                    selected,
                } => match (code, modifiers) {
                    (KeyCode::Char('q'), _) => {
                        tx.send(WatchEvent::Stop).unwrap();
                    }
                    (KeyCode::Up, _) => {
                        tx.send(WatchEvent::MoveUp(1)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Down, _) => {
                        tx.send(WatchEvent::MoveDown(1)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Char('b'), _) => {
                        log::info!("Tab: switch block type");
                        tx.send(WatchEvent::EditBlock(block_type.next(), *selected))
                            .unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char('s'), _) | (KeyCode::Tab, KeyModifiers::SHIFT) => {
                        log::info!("Tab: switch selected type");
                        tx.send(WatchEvent::EditBlock(*block_type, selected.next()))
                            .unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Enter, _) | (KeyCode::Char(' '), _) => {
                        log::info!("Enter: add/remove");
                        tx.send(WatchEvent::Enter).unwrap();
                        tx.send(WatchEvent::MoveUp(1)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char('<'), _) => {
                        // reorder in "enabled" if focus is on it
                    }
                    (KeyCode::Char('>'), _) => {
                        // reorder in "enabled"
                    }
                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        // default
                    }
                    _ => {}
                },
                // others (error etc.) exit current on any key
                _ => {
                    tx.send(WatchEvent::Stop).unwrap();
                }
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

        self.draw()
    }

    fn draw_header<W: Write>(&mut self, writer: &mut W) -> Result<()> {
        let (term_width, _) = self.terminal_size;

        let header = match &*self.input_mode.lock().unwrap() {
            InputMode::BlockEditor { block_type, .. } => {
                format!(" EDITING DISPLAY BLOCKS: {}", block_type)
            }
            _ => {
                format!(
                    " FILTERS Name: {:?} Serial: {:?} VID:PID: {}:{} Class: {:?}",
                    self.filter.name,
                    self.filter.serial,
                    self.filter
                        .vid
                        .map_or("".to_string(), |v| format!("{:04x}", v)),
                    self.filter
                        .pid
                        .map_or("".to_string(), |p| format!("{:04x}", p)),
                    self.filter.class
                )
            }
        };

        execute!(
            writer,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::CurrentLine),
            Print(
                format!("{:<width$}", header, width = term_width as usize)
                    .bold()
                    .black()
                    .on_blue()
            ),
        )?;

        Ok(())
    }

    fn draw_footer<W: Write>(&mut self, writer: &mut W) -> Result<()> {
        let (term_width, term_height) = self.terminal_size;

        // TODO elipses for long footers
        let footer = match self.input_mode.lock().unwrap().clone() {
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
                );

                format!("{:<width$}", footer, width = term_width as usize)
                    .bold()
                    .black()
                    .on_green()
            }
            InputMode::EditingFilter { field, buffer, .. } => {
                let mut footer = match field {
                    FilterField::Name => "Filter by Name: ".to_string(),
                    FilterField::Serial => "Filter by Serial: ".to_string(),
                    FilterField::VidPid => "Filter by VID:PID: ".to_string(),
                    FilterField::Class => "Filter by Class: ".to_string(),
                };
                footer.push_str(&buffer.unwrap_or_default());
                format!("{:<width$}", footer, width = term_width as usize)
                    .black()
                    .on_yellow()
            }
            InputMode::Error(msg) => format!("{:<width$}", msg, width = term_width as usize)
                .bold()
                .red()
                .on_white(),
            InputMode::BlockEditor { .. } => {
                let footer = " [Up/Down] Move in one list  [Tab] switch lists  [Enter] Add/Remove  [<]/[>] reorder  [q] Exit".bold();
                format!("{:<width$}", footer, width = term_width as usize)
                    .black()
                    .on_green()
            }
        };

        // move cursor to last row
        execute!(
            writer,
            cursor::MoveTo(0, term_height - 1),
            terminal::Clear(terminal::ClearType::CurrentLine),
            Print(footer),
        )?;

        Ok(())
    }

    fn draw_block_editor<W: Write, B: BlockEnum + ValueEnum, T>(
        writer: &mut W,
        print_settings: &PrintSettings,
        selected: &BlockSelection,
        enabled_blocks: &[impl Block<B, T> + ToString],
        available_blocks: &[impl Block<B, T> + ToString],
    ) -> Result<()> {
        let selected_enabled = match selected {
            BlockSelection::Enabled(i, _) => Some(*i),
            _ => None,
        };

        let selected_available = match selected {
            BlockSelection::Available(i, _) => Some(*i),
            _ => None,
        };

        for (i, block) in enabled_blocks.iter().enumerate() {
            let sel = Some(i) == selected_enabled;
            let block_string = if sel {
                log::info!("Selected: {}", block.to_string());
                block.to_string().bold().on_bright_purple().to_string()
            } else {
                print_settings.colours.as_ref().map_or_else(
                    || block.to_string(),
                    |c| block.colour(&block.to_string(), c).to_string(),
                )
            };
            writeln!(writer, " [x] {}", block_string)?;
        }

        writeln!(writer, " ---")?;
        for (i, block) in available_blocks.iter().enumerate() {
            let sel = Some(i) == selected_available;
            let block_string = if sel {
                log::info!("Selected: {}", block.to_string());
                block.to_string().bold().on_bright_purple().to_string()
            } else {
                print_settings.colours.as_ref().map_or_else(
                    || block.to_string(),
                    |c| block.colour(&block.to_string(), c).to_string(),
                )
            };
            writeln!(writer, " [ ] {}", block_string)?;
        }

        Ok(())
    }

    fn draw_edit_blocks(&mut self) -> Result<()> {
        self.buffer.clear();

        if let InputMode::BlockEditor {
            block_type,
            selected,
        } = &mut *self.input_mode.lock().unwrap()
        {
            match block_type {
                // TODO - implement for other block types and ensure print_settings has blocks so unwrap safe
                BlockType::Device => {
                    let print_settings = self.print_settings.lock().unwrap();
                    let enabled_blocks = print_settings.device_blocks.as_ref();
                    let all_blocks = DeviceBlocks::all_blocks().to_vec();
                    let available_blocks: Vec<DeviceBlocks> = all_blocks
                        .into_iter()
                        .filter(|b| !enabled_blocks.as_ref().is_some_and(|e| e.contains(b)))
                        .collect();
                    match selected {
                        BlockSelection::Enabled(_, _) => {
                            selected
                                .set_max(enabled_blocks.as_ref().unwrap().len().saturating_sub(1));
                        }
                        BlockSelection::Available(_, _) => {
                            selected.set_max(available_blocks.len().saturating_sub(1));
                        }
                    }
                    Display::draw_block_editor(
                        &mut self.buffer,
                        &print_settings,
                        selected,
                        enabled_blocks.as_ref().unwrap(),
                        &available_blocks,
                    )?;
                }
                _ => {
                    log::warn!("Not implemented");
                }
            }
        } else {
            panic!("draw_edit_blocks called when not in block editor mode");
        }

        self.draw()
    }

    fn draw(&mut self) -> Result<()> {
        let mut stdout = stdout();
        self.terminal_size = terminal::size().unwrap_or((80, 24));
        let (_, term_height) = self.terminal_size;
        // two for header and footer
        self.available_rows = term_height.saturating_sub(2) as usize;

        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All),
        )
        .map_err(|e| Error::new(ErrorKind::Other("crossterm"), &e.to_string()))?;

        self.draw_header(&mut stdout)?;

        // convert buffer to string and split into lines
        let output = String::from_utf8_lossy(&self.buffer);
        let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();

        self.lines = lines.len();
        self.max_offset = self.lines.saturating_sub(self.available_rows);
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

        // status bar with key bindings
        self.draw_footer(&mut stdout)?;

        stdout.flush()?;

        Ok(())
    }
}
