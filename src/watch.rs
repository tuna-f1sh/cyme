//! Watch for USB devices being connected and disconnected.
use cansi::v3::{categorise_text, construct_text_no_codes};
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
use std::env;
use std::io::stdout;
use std::io::Write;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use super::parse_vidpid;
use cyme::config::Config;
use cyme::display::*;
use cyme::error::{Error, ErrorKind, Result};
use cyme::profiler::{watch::SystemProfileStreamBuilder, Filter, SystemProfile};

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum WatchEvent {
    ScrollUp(usize),
    ScrollUpHalf,
    ScrollUpPage,
    ScrollDown(usize),
    ScrollDownHalf,
    ScrollDownPage,
    MoveUp(usize),
    MoveDown(usize),
    EditFilter(FilterField),
    ToggleBuses,
    ToggleHubs,
    ExpandSelected,
    PushFilter(char),
    PopFilter,
    EditBlock(BlockType),
    MoveBlockUp(usize),
    MoveBlockDown(usize),
    ToggleBlock,
    Error(String),
    Resize,
    DrawDevices,
    DrawEditBlocks,
    WriteEditBlocks,
    ConfirmSave(String),
    SaveConfig,
    Draw,
    ShowHelp,
    Enter,
    Esc,
    Quit,
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

    fn prev(&self) -> Self {
        match self {
            BlockType::Bus => BlockType::Endpoint,
            BlockType::Device => BlockType::Bus,
            BlockType::Config => BlockType::Device,
            BlockType::Interface => BlockType::Config,
            BlockType::Endpoint => BlockType::Interface,
        }
    }

    fn key_number(&self) -> char {
        match self {
            BlockType::Bus => '1',
            BlockType::Device => '2',
            BlockType::Config => '3',
            BlockType::Interface => '4',
            BlockType::Endpoint => '5',
        }
    }
}

#[derive(Debug, Clone)]
enum State {
    Normal,
    EditingFilter {
        field: FilterField,
        buffer: Option<String>,
        original: Option<String>,
    },
    BlockEditor {
        block_type: BlockType,
        selected_index: usize,
        // Box<dyn BlockEnum> would be nice with downcast back to print_settings concrete type
        // Block is currently not dyn-compatible however so serialize and deserialize works...
        blocks: Vec<(ColoredString, bool)>,
    },
    Help,
    Error(String),
    Confirm(String, WatchEvent),
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
    state: Arc<Mutex<State>>,
    /// Size of current window
    terminal_size: (u16, u16),
    /// Currently selected line
    ///
    /// Already accounts for header and footer
    selected_line: Option<usize>,
    /// Number of rows available for output
    available_rows: usize,
    /// Maximum offset for scrolling
    max_offset: usize,
    /// Number of lines in the buffer
    lines: usize,
    /// Current scroll offset
    scroll_offset: usize,
    /// Line context provides lookup for selected item
    line_context: Vec<LineItem>,
    /// Selected line context
    selected_item: Option<LineItem>,
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

fn strip_ansi_codes(input: &str) -> String {
    let decolored = categorise_text(input);
    construct_text_no_codes(&decolored)
}

fn print_json(
    spusb: &mut SystemProfile,
    filter: Option<&Filter>,
    print_settings: &PrintSettings,
) -> Result<()> {
    cyme::display::prepare(spusb, filter, print_settings);
    let json = serde_json::to_string(&spusb)?;
    println!("{json}");
    Ok(())
}

pub fn watch_usb_devices_json(
    spusb: SystemProfile,
    filter: Option<Filter>,
    print_settings: PrintSettings,
) -> Result<()> {
    // pass spusb to stream builder, will get Arc<Mutex<SystemProfile>> back below
    let mut profile_stream = SystemProfileStreamBuilder::new()
        .with_spusb(spusb)
        .is_verbose(print_settings.verbosity > 0)
        .build()
        .map_err(|e| Error::new(ErrorKind::Nusb, &e.to_string()))?;

    let (tx, rx) = mpsc::channel::<WatchEvent>();
    let print_settings = Arc::new(Mutex::new(print_settings));
    let spusb = profile_stream.get_profile();
    // first draw queue event
    tx.send(WatchEvent::DrawDevices).unwrap();

    // Thread to listen for USB profiling events
    let tx_clone = tx.clone();
    thread::spawn(move || {
        futures_lite::future::block_on(async {
            while profile_stream.next().await.is_some() {
                tx_clone.send(WatchEvent::DrawDevices).unwrap();
            }
        });
    });

    loop {
        match rx.recv() {
            Ok(WatchEvent::DrawDevices) => {
                let print_settings = print_settings.lock().unwrap();
                print_json(&mut spusb.lock().unwrap(), filter.as_ref(), &print_settings)?;
            }
            Ok(_) => {
                // ignore other events
            }
            Err(_) => {
                break;
            }
        }
    }

    Ok(())
}

pub fn watch_usb_devices(
    spusb: SystemProfile,
    filter: Option<Filter>,
    mut print_settings: PrintSettings,
    mut config: Config,
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
    print_settings.endpoint_blocks =
        print_settings
            .endpoint_blocks
            .or(Some(EndpointBlocks::default_blocks(
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
    let input_mode = Arc::new(Mutex::new(State::Normal));
    // first draw
    tx.send(WatchEvent::DrawDevices).unwrap();

    let mut display = Display {
        buffer: Vec::new(),
        // get a reference to the SystemProfile now since profile_stream can't be moved
        // main thread needs to draw with SystemProfile outside of the stream
        spusb: profile_stream.get_profile(),
        print_settings: print_settings.clone(),
        filter: filter.unwrap_or_default(),
        state: input_mode.clone(),
        terminal_size: terminal::size().unwrap_or((80, 24)),
        lines: 0,
        available_rows: 0,
        max_offset: 0,
        scroll_offset: 0,
        selected_line: None,
        selected_item: None,
        line_context: Vec::new(),
    };

    // Thread to listen for USB profiling events
    let tx_clone = tx.clone();
    // Flag to stop threads when exiting main loop and prevent sending on closed channel
    let stop_flag = Arc::new(Mutex::new(false));
    let stop_clone = stop_flag.clone();
    thread::spawn(move || {
        futures_lite::future::block_on(async {
            while profile_stream.next().await.is_some() {
                // local spusb Arc<Mutex<SystemProfile>> is updated since we have a reference to it
                if *stop_clone.lock().unwrap() {
                    break;
                }
                tx_clone.send(WatchEvent::DrawDevices).unwrap();
            }
        });
    });

    // Thread to listen for terminal events
    let stop_clone = stop_flag.clone();
    thread::spawn(move || loop {
        let send_wrapper = |e: WatchEvent| {
            if *stop_clone.lock().unwrap() {
                return true;
            }
            tx.send(e).unwrap();
            false
        };

        match event::read().unwrap() {
            Event::Resize(_, _) => {
                if send_wrapper(WatchEvent::Resize) {
                    break;
                }
            }
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => {
                    if send_wrapper(WatchEvent::ScrollUp(1)) {
                        break;
                    }
                }
                MouseEventKind::ScrollDown => {
                    if send_wrapper(WatchEvent::ScrollDown(1)) {
                        break;
                    }
                }
                _ => (),
            },
            Event::Key(key_event) => {
                if *stop_clone.lock().unwrap() {
                    break;
                }
                input_mode.lock().unwrap().process_key_event(
                    key_event,
                    tx.clone(),
                    print_settings.clone(),
                );
            }
            _ => (),
        }

        if *stop_clone.lock().unwrap() {
            break;
        }
    });

    // Main event loop
    // manages the display and listens for events
    loop {
        match rx.recv() {
            Ok(WatchEvent::ScrollUp(n)) => {
                display.scroll_offset = display.scroll_offset.saturating_sub(n);
                display.scroll_check_selected_visable();
            }
            Ok(WatchEvent::ScrollUpHalf) => {
                display.scroll_offset = display
                    .scroll_offset
                    .saturating_sub(display.available_rows / 2);
                display.scroll_check_selected_visable();
            }
            Ok(WatchEvent::ScrollUpPage) => {
                display.scroll_offset =
                    display.scroll_offset.saturating_sub(display.available_rows);
                display.scroll_check_selected_visable();
            }
            Ok(WatchEvent::ScrollDown(n)) => {
                display.scroll_offset = display
                    .max_offset
                    .min(display.scroll_offset.saturating_add(n));
                display.scroll_check_selected_visable();
            }
            Ok(WatchEvent::ScrollDownHalf) => {
                display.scroll_offset = display.max_offset.min(
                    display
                        .scroll_offset
                        .saturating_add(display.available_rows / 2),
                );
                display.scroll_check_selected_visable();
            }
            Ok(WatchEvent::ScrollDownPage) => {
                display.scroll_offset = display
                    .max_offset
                    .min(display.scroll_offset.saturating_add(display.max_offset));
                display.scroll_check_selected_visable();
            }
            Ok(WatchEvent::MoveUp(n)) => {
                display.move_up(n);
                if let Some(l) = display.selected_line {
                    if l < display.scroll_offset {
                        display.scroll_offset = display
                            .scroll_offset
                            .saturating_sub(display.scroll_offset - l);
                    }
                }
            }
            Ok(WatchEvent::MoveDown(n)) => {
                display.move_down(n);
                if let Some(l) = display.selected_line {
                    if l >= display.scroll_offset + display.available_rows {
                        display.scroll_offset = display.scroll_offset.saturating_add(
                            l - (display.scroll_offset + display.available_rows) + 1,
                        );
                    }
                }
            }
            Ok(WatchEvent::ExpandSelected) => {
                display.toggle_selected(false);
            }

            Ok(WatchEvent::EditFilter(field)) => {
                let original = match field {
                    FilterField::Name => display.filter.name.clone(),
                    FilterField::Serial => display.filter.serial.clone(),
                    FilterField::VidPid => match (display.filter.vid, display.filter.pid) {
                        (Some(vid), Some(pid)) => Some(format!("{vid:04x}:{pid:04x}")),
                        (Some(vid), None) => Some(format!("{vid:04x}")),
                        _ => None,
                    },
                    FilterField::Class => display.filter.class.map(|c| c.to_string()),
                };
                *display.state.lock().unwrap() = State::EditingFilter {
                    field,
                    buffer: original.clone(),
                    original,
                };
            }
            Ok(WatchEvent::PushFilter(c)) => {
                let mut input_mode = display.state.lock().unwrap();
                if let State::EditingFilter { field, buffer, .. } = &mut *input_mode {
                    buffer.get_or_insert_with(String::new).push(c);
                    match field {
                        FilterField::Serial | FilterField::Name => {
                            if let Err(e) = set_filter(field, buffer.clone(), &mut display.filter) {
                                *input_mode = State::Error(e.to_string());
                            }
                        }
                        _ => (),
                    }
                }
            }
            Ok(WatchEvent::PopFilter) => {
                let mut input_mode = display.state.lock().unwrap();
                if let State::EditingFilter { field, buffer, .. } = &mut *input_mode {
                    if let Some(b) = buffer {
                        let ret = b.pop();
                        if b.is_empty() | ret.is_none() {
                            buffer.take();
                        }
                    };
                    match field {
                        FilterField::Serial | FilterField::Name => {
                            if let Err(e) = set_filter(field, buffer.clone(), &mut display.filter) {
                                *input_mode = State::Error(e.to_string());
                            }
                        }
                        _ => (),
                    }
                }
            }
            Ok(WatchEvent::ToggleBuses) => {
                display.filter.exclude_empty_bus = !display.filter.exclude_empty_bus;
            }
            Ok(WatchEvent::ToggleHubs) => {
                display.filter.exclude_empty_hub = !display.filter.exclude_empty_hub;
            }

            Ok(WatchEvent::Enter) => {
                let new_mode = match &*display.state.lock().unwrap() {
                    State::EditingFilter { field, buffer, .. } => {
                        if let Err(e) = set_filter(field, buffer.to_owned(), &mut display.filter) {
                            State::Error(e.to_string())
                        } else {
                            State::Normal
                        }
                    }
                    State::Normal => {
                        display.toggle_selected(true);
                        State::Normal
                    }
                    _ => State::Normal,
                };

                *display.state.lock().unwrap() = new_mode;
                display.prepare_devices();
                display.draw_devices()?;
            }
            Ok(WatchEvent::Error(msg)) => {
                *display.state.lock().unwrap() = State::Error(msg);
                display.draw()?;
            }
            Ok(WatchEvent::ConfirmSave(msg)) => {
                *display.state.lock().unwrap() = State::Confirm(msg, WatchEvent::SaveConfig);
                display.draw()?;
            }
            Ok(WatchEvent::Resize) => {
                // resize needs to redraw devices as columns may have changed but not block editor
                if !matches!(&*display.state.lock().unwrap(), State::BlockEditor { .. }) {
                    display.print_settings.lock().unwrap().terminal_size = terminal::size().ok();
                    display.draw_devices()?;
                }
                display.draw()?;
            }

            Ok(WatchEvent::DrawDevices) => {
                display.prepare_devices();
                display.draw_devices()?;
            }

            Ok(WatchEvent::EditBlock(block_type)) => {
                display.prepare_edit_blocks(block_type);
            }
            Ok(WatchEvent::MoveBlockUp(n)) => {
                if let State::BlockEditor {
                    selected_index,
                    blocks,
                    ..
                } = &mut *display.state.lock().unwrap()
                {
                    if *selected_index != 0 {
                        blocks.swap(*selected_index, selected_index.saturating_sub(n));
                        *selected_index = selected_index.saturating_sub(n);
                        display.selected_line = Some(*selected_index);
                    }
                }
            }
            Ok(WatchEvent::MoveBlockDown(n)) => {
                if let State::BlockEditor {
                    selected_index,
                    blocks,
                    ..
                } = &mut *display.state.lock().unwrap()
                {
                    if *selected_index < blocks.len() - 1 {
                        blocks.swap(*selected_index, selected_index.saturating_add(n));
                        *selected_index = selected_index.saturating_add(n);
                        display.selected_line = Some(*selected_index);
                    }
                }
            }
            Ok(WatchEvent::ToggleBlock) => {
                if let State::BlockEditor {
                    selected_index,
                    blocks,
                    ..
                } = &mut *display.state.lock().unwrap()
                {
                    if *selected_index < blocks.len() {
                        blocks[*selected_index].1 = !blocks[*selected_index].1;
                    }
                }
            }
            Ok(WatchEvent::DrawEditBlocks) => {
                display.draw_edit_blocks()?;
            }
            Ok(WatchEvent::WriteEditBlocks) => {
                display.write_edit_blocks();
            }
            Ok(WatchEvent::SaveConfig) => {
                config.merge_print_settings(&display.print_settings.lock().unwrap());
                if let Err(e) = config.save() {
                    *display.state.lock().unwrap() = State::Error(e.to_string());
                    display.prepare_devices();
                    display.draw_devices()?;
                }
            }

            Ok(WatchEvent::ShowHelp) => {
                *display.state.lock().unwrap() = State::Help;
                display.draw_help()?;
            }
            Ok(WatchEvent::Draw) => {
                display.draw()?;
            }
            Ok(WatchEvent::Esc) => {
                let new_mode = match &*display.state.lock().unwrap() {
                    // clear selected item
                    State::Normal => {
                        display.selected_line = None;
                        display.selected_item = None;
                        State::Normal
                    }
                    State::EditingFilter {
                        field, original, ..
                    } => match set_filter(field, original.to_owned(), &mut display.filter) {
                        Ok(_) => State::Normal,
                        Err(e) => {
                            // original must be bad so clear it
                            set_filter(field, None, &mut display.filter)?;
                            State::Error(e.to_string())
                        }
                    },
                    _ => State::Normal,
                };

                *display.state.lock().unwrap() = new_mode;
                display.prepare_devices();
                display.draw_devices()?;
            }
            Ok(WatchEvent::Quit) => {
                *stop_flag.lock().unwrap() = true;
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

impl State {
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
            (KeyCode::Char(' '), _) => {
                tx.send(WatchEvent::ExpandSelected).unwrap();
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('?'), _) => {
                tx.send(WatchEvent::ShowHelp).unwrap();
            }
            (KeyCode::Char('q'), _) => {
                tx.send(WatchEvent::Quit).unwrap();
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
            (KeyCode::Char('o'), _) => {
                let mut print_settings = print_settings.lock().unwrap();
                print_settings.decimal = !print_settings.decimal;
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('p'), _) => {
                let mut print_settings = print_settings.lock().unwrap();
                match print_settings.sort_devices {
                    Sort::BranchPosition => print_settings.sort_devices = Sort::DeviceNumber,
                    _ => print_settings.sort_devices = Sort::BranchPosition,
                }
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('a'), _) => {
                let mut print_settings = print_settings.lock().unwrap();
                match print_settings.encoding {
                    Encoding::Glyphs => print_settings.encoding = Encoding::Ascii,
                    _ => print_settings.encoding = Encoding::Glyphs,
                }
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, _) => {
                tx.send(WatchEvent::MoveDown(1)).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, _) => {
                tx.send(WatchEvent::MoveUp(1)).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Char('/'), _) => {
                tx.send(WatchEvent::EditFilter(FilterField::Name)).unwrap();
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('\\'), _) => {
                tx.send(WatchEvent::ToggleBuses).unwrap();
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char(';'), _) => {
                tx.send(WatchEvent::ToggleHubs).unwrap();
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
            (KeyCode::Char('s'), KeyModifiers::NONE) => {
                tx.send(WatchEvent::EditFilter(FilterField::Serial))
                    .unwrap();
                tx.send(WatchEvent::DrawDevices).unwrap();
            }
            (KeyCode::Char('b'), _) => {
                tx.send(WatchEvent::EditBlock(BlockType::Device)).unwrap();
                tx.send(WatchEvent::DrawEditBlocks).unwrap();
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                tx.send(WatchEvent::ConfirmSave(
                    "Save current settings to global config?".into(),
                ))
                .unwrap();
            }
            (KeyCode::Char('d'), KeyModifiers::NONE) => {
                tx.send(WatchEvent::ScrollDownHalf).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Char('u'), KeyModifiers::NONE) => {
                tx.send(WatchEvent::ScrollUpHalf).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
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
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                // will stop current context or exit the program
                tx.send(WatchEvent::Quit).unwrap();
            }
            (KeyCode::Esc, _) => {
                tx.send(WatchEvent::Esc).unwrap();
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) | (KeyCode::PageDown, _) => {
                tx.send(WatchEvent::ScrollDownPage).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) | (KeyCode::PageUp, _) => {
                tx.send(WatchEvent::ScrollUpPage).unwrap();
                tx.send(WatchEvent::Draw).unwrap();
            }
            // others based on mode
            _ => match self {
                State::Normal | State::Help => {
                    Self::process_normal_mode(event, tx, print_settings);
                }
                State::EditingFilter { field, .. } => match (code, modifiers) {
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
                State::BlockEditor { block_type, .. } => match (code, modifiers) {
                    (KeyCode::Char('q'), _) => {
                        tx.send(WatchEvent::Esc).unwrap();
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                        tx.send(WatchEvent::MoveUp(1)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        tx.send(WatchEvent::MoveDown(1)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Char('b'), _) => {
                        tx.send(WatchEvent::EditBlock(block_type.next())).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Tab, KeyModifiers::SHIFT) => {
                        tx.send(WatchEvent::EditBlock(block_type.prev())).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char('1'), _) => {
                        tx.send(WatchEvent::EditBlock(BlockType::Bus)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char('2'), _) => {
                        tx.send(WatchEvent::EditBlock(BlockType::Device)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char('3'), _) => {
                        tx.send(WatchEvent::EditBlock(BlockType::Config)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char('4'), _) => {
                        tx.send(WatchEvent::EditBlock(BlockType::Interface))
                            .unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char('5'), _) => {
                        tx.send(WatchEvent::EditBlock(BlockType::Endpoint)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char(' '), _) => {
                        tx.send(WatchEvent::ToggleBlock).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Enter, _) => {
                        tx.send(WatchEvent::WriteEditBlocks).unwrap();
                        tx.send(WatchEvent::Enter).unwrap();
                    }
                    (KeyCode::Char('<'), _)
                    | (KeyCode::Char(','), _)
                    | (KeyCode::Char('h'), _)
                    | (KeyCode::Left, _)
                    | (KeyCode::Char('['), _) => {
                        tx.send(WatchEvent::MoveBlockUp(1)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    (KeyCode::Char('>'), _)
                    | (KeyCode::Char('.'), _)
                    | (KeyCode::Char('l'), _)
                    | (KeyCode::Right, _)
                    | (KeyCode::Char(']'), _) => {
                        tx.send(WatchEvent::MoveBlockDown(1)).unwrap();
                        tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    }
                    //(KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    //    tx.send(WatchEvent::WriteDefaultBlocks).unwrap();
                    //    tx.send(WatchEvent::DrawEditBlocks).unwrap();
                    //}
                    _ => (),
                },
                State::Confirm(_, n) => match (code, modifiers) {
                    // abort
                    (KeyCode::Char('n'), _) | (KeyCode::Esc, _) | (KeyCode::Char('N'), _) => {
                        tx.send(WatchEvent::Esc).unwrap();
                    }
                    _ => {
                        tx.send(n.clone()).unwrap();
                        tx.send(WatchEvent::Esc).unwrap();
                    }
                },
                // others (error etc.) exit current on any key
                e => {
                    log::debug!("Exiting state {:?} on other key event", e);
                    tx.send(WatchEvent::Quit).unwrap();
                }
            },
        }
    }
}

impl Display {
    fn toggle_selected(&self, all: bool) {
        match self.selected_item.as_ref() {
            Some(LineItem::Device(path)) => {
                if let Some(node) = self.spusb.lock().unwrap().get_node_mut(path) {
                    if all {
                        node.set_all_expanded(!node.is_expanded());
                    } else {
                        node.toggle_expanded();
                    }
                }
            }
            Some(LineItem::Config(path)) => {
                if let Some(config) = self.spusb.lock().unwrap().get_config_mut(&path.0, path.1) {
                    if all {
                        config.set_all_expanded(!config.is_expanded());
                    } else {
                        config.toggle_expanded();
                    }
                }
            }
            Some(LineItem::Interface(path)) => {
                if let Some(interface) = self.spusb.lock().unwrap().get_interface_mut(path) {
                    if all {
                        interface.set_all_expanded(!interface.is_expanded());
                    } else {
                        interface.toggle_expanded();
                    }
                }
            }
            Some(LineItem::Endpoint(path)) => {
                if let Some(endpoint) = self.spusb.lock().unwrap().get_endpoint_mut(path) {
                    endpoint.toggle_expanded();
                }
            }
            _ => (),
        }
    }

    fn prepare_devices(&mut self) {
        let print_settings = self.print_settings.lock().unwrap();
        let mut spusb = self.spusb.lock().unwrap();
        cyme::display::prepare(&mut spusb, Some(&self.filter), &print_settings);
    }

    fn move_up(&mut self, n: usize) {
        match &mut *self.state.lock().unwrap() {
            State::BlockEditor { selected_index, .. } => {
                *selected_index = selected_index.saturating_sub(n);
                self.selected_line = Some(*selected_index);
            }
            State::Normal => {
                let li = self
                    .line_context
                    .iter()
                    .enumerate()
                    .rev()
                    .skip(
                        self.line_context.len().saturating_sub(
                            self.selected_line
                                .unwrap_or(n)
                                .saturating_sub(n.saturating_sub(1)),
                        ),
                    )
                    .find(|(_, l)| !(matches!(l, LineItem::None) || matches!(l, LineItem::Bus(_))));
                if let Some((i, item)) = li {
                    self.selected_line = Some(i);
                    self.selected_item = Some(item.clone());
                }
            }
            _ => (),
        };

        log::debug!(
            "Selected line: {:?} ({:?})",
            self.selected_line,
            self.selected_item
        );
    }

    fn move_down(&mut self, n: usize) {
        match &mut *self.state.lock().unwrap() {
            State::BlockEditor {
                selected_index,
                blocks,
                ..
            } => {
                *selected_index = selected_index
                    .saturating_add(n)
                    .min(blocks.len().saturating_sub(1));
                self.selected_line = Some(*selected_index);
            }
            State::Normal => {
                let li = self
                    .line_context
                    .iter()
                    .enumerate()
                    .skip(self.selected_line.map_or(n, |i| i.saturating_add(n)))
                    .find(|(_, l)| !(matches!(l, LineItem::None) || matches!(l, LineItem::Bus(_))));
                if let Some((i, item)) = li {
                    self.selected_line = Some(i);
                    self.selected_item = Some(item.clone());
                }
            }
            _ => (),
        }

        log::debug!(
            "Selected line: {:?} ({:?})",
            self.selected_line,
            self.selected_item
        );
    }

    /// Shifts selected line into view when scrolling
    fn scroll_check_selected_visable(&mut self) {
        if let Some(l) = self.selected_line {
            if l < self.scroll_offset {
                self.move_down(self.scroll_offset - l);
            } else if l >= self.scroll_offset + self.available_rows {
                self.move_up(l - (self.scroll_offset + self.available_rows) + 1);
            }
        }
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
        self.line_context = dw.line_context().to_owned();

        // find selected device
        if let Some(selected_device) = self.selected_item.as_ref() {
            if let Some((i, _)) = self
                .line_context
                .iter()
                .enumerate()
                .find(|(_, l)| *l == selected_device)
            {
                self.selected_line = Some(i);
            }
        } else {
            self.selected_line = None;
        }

        self.draw()
    }

    fn draw_help(&mut self) -> Result<()> {
        self.buffer.clear();

        writeln!(
            self.buffer,
            r#" {} v{} - USB Device Watcher
 vim bindings where possible!

 [q]/[Esc]: Exit program/abort mode
 [v]: Cycle verbosity
 [t]: Toggle tree
 [h]: Toggle headings
 [o]: Toggle decimal/hex
 [p]: Cycle sort mode
 [b]: Enter block editor
 [/]: Edit name filter
 [s]: Edit serial filter
 [#]: Edit VID:PID filter
 [c]: Edit class filter
 [j]/[k], Up/Down: Move selection
 [u]/[d], PgUp/PgDn Ctrl+u/Ctrl+d: Scroll page
 [CR]: Expand selected/Accept or commit changes
 [space]: Toggle selected
 [C-s]: Save current display settings to cyme config.json
 [C-c]: Exit program
 [?]: Show this help
            "#,
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )?;

        // Then just call `draw()` to show the new buffer
        self.draw()
    }

    fn draw_header<W: Write>(&mut self, writer: &mut W) -> Result<()> {
        let (term_width, _) = self.terminal_size;

        let mut header = match &*self.state.lock().unwrap() {
            State::BlockEditor { block_type, .. } => {
                format!(
                    " EDITING BLOCKS: [{}] {}",
                    block_type.key_number(),
                    block_type
                )
            }
            _ => {
                format!(
                    " FILTERS Name={:?} Serial={:?} VID:PID={}:{} Class={:?}",
                    self.filter.name,
                    self.filter.serial,
                    self.filter
                        .vid
                        .map_or("".to_string(), |v| format!("{v:04x}")),
                    self.filter
                        .pid
                        .map_or("".to_string(), |p| format!("{p:04x}")),
                    self.filter.class
                )
            }
        };

        truncate_string(&mut header, term_width as usize);

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

        let footer = match &*self.state.lock().unwrap() {
            State::EditingFilter { field, buffer, .. } => {
                let mut footer = match field {
                    FilterField::Name => "Filter Name: ".to_string(),
                    FilterField::Serial => "Filter Serial: ".to_string(),
                    FilterField::VidPid => "Filter VID:PID: ".to_string(),
                    FilterField::Class => "Filter Class: ".to_string(),
                };
                if let Some(buffer) = buffer {
                    footer.push_str(buffer);
                }
                format!("{:<width$}", footer, width = term_width as usize)
                    .black()
                    .on_yellow()
            }
            State::Error(msg) => format!("{:<width$}", msg, width = term_width as usize)
                .bold()
                .red()
                .on_white(),
            State::Confirm(msg, _) => format!("{:<width$}", format!("{} Y/n", msg), width = term_width as usize)
                .bold()
                .black()
                .on_white(),
            State::BlockEditor { .. } => {
                let mut footer = String::from(" [Up/Down]-Navigate [Space]-Toggle [<]/[>]-Move [Tab]-Switch [Enter]-Accept [q]-Exit");
                truncate_string(&mut footer, term_width as usize);
                format!("{:<width$}", footer, width = term_width as usize)
                    .bold()
                    .black()
                    .on_green()
            }
            _ => {
                let print_settings = self.print_settings.lock().unwrap();
                let verbosity = if print_settings.verbosity == 3 {
                    String::from("0")
                } else {
                    (print_settings.verbosity + 1).to_string()
                };
                let mut footer = format!(
                    " [q]-Quit [b]-Edit Blocks [v]-Verbosity-(→{}) [t]-Tree-(→{}) [h]-Headings-(→{}) [o]-Decimal-(→{}) [p]-Sort-(→{})",
                    verbosity,
                    if print_settings.tree { "X" } else { "O" },
                    if print_settings.headings { "X" } else { "O" },
                    if print_settings.decimal { "X" } else { "O" },
                    if print_settings.sort_devices == Sort::BranchPosition {
                        "D#"
                    } else {
                        "P#"
                    }
                );
                truncate_string(&mut footer, term_width as usize);

                format!("{:<width$}", footer, width = term_width as usize)
                    .bold()
                    .black()
                    .on_green()
            }
        }.to_string();

        // move cursor to last row
        execute!(
            writer,
            cursor::MoveTo(0, term_height - 1),
            terminal::Clear(terminal::ClearType::CurrentLine),
            Print(footer),
        )?;

        Ok(())
    }

    fn prepare_edit_blocks(&mut self, block_type: BlockType) {
        let print_settings = self.print_settings.lock().unwrap();
        let ct = &print_settings.colours;

        let blocks: Vec<(ColoredString, bool)> = match block_type {
            BlockType::Device => {
                let enabled_blocks = print_settings.device_blocks.as_ref().unwrap();
                enabled_blocks
                    .iter()
                    .map(|b| (b, true))
                    .chain(
                        DeviceBlocks::all_blocks()
                            .iter()
                            .filter(|b| !enabled_blocks.contains(b))
                            .map(|b| (b, false)),
                    )
                    .map(|(b, enabled)| {
                        let cs = if let Some(ct) = ct.as_ref() {
                            let block = b.to_possible_value().unwrap().get_name().to_string();
                            b.colour(&block, ct)
                        } else {
                            b.to_possible_value().unwrap().get_name().into()
                        };
                        (cs, enabled)
                    })
                    .collect()
            }
            BlockType::Bus => {
                let enabled_blocks = print_settings.bus_blocks.as_ref().unwrap();
                enabled_blocks
                    .iter()
                    .map(|b| (b, true))
                    .chain(
                        BusBlocks::all_blocks()
                            .iter()
                            .filter(|b| !enabled_blocks.contains(b))
                            .map(|b| (b, false)),
                    )
                    .map(|(b, enabled)| {
                        let cs = if let Some(ct) = ct.as_ref() {
                            let block = b.to_possible_value().unwrap().get_name().to_string();
                            b.colour(&block, ct)
                        } else {
                            b.to_possible_value().unwrap().get_name().into()
                        };
                        (cs, enabled)
                    })
                    .collect()
            }
            BlockType::Config => {
                let enabled_blocks = print_settings.config_blocks.as_ref().unwrap();
                enabled_blocks
                    .iter()
                    .map(|b| (b, true))
                    .chain(
                        ConfigurationBlocks::all_blocks()
                            .iter()
                            .filter(|b| !enabled_blocks.contains(b))
                            .map(|b| (b, false)),
                    )
                    .map(|(b, enabled)| {
                        let cs = if let Some(ct) = ct.as_ref() {
                            let block = b.to_possible_value().unwrap().get_name().to_string();
                            b.colour(&block, ct)
                        } else {
                            b.to_possible_value().unwrap().get_name().into()
                        };
                        (cs, enabled)
                    })
                    .collect()
            }
            BlockType::Interface => {
                let enabled_blocks = print_settings.interface_blocks.as_ref().unwrap();
                enabled_blocks
                    .iter()
                    .map(|b| (b, true))
                    .chain(
                        InterfaceBlocks::all_blocks()
                            .iter()
                            .filter(|b| !enabled_blocks.contains(b))
                            .map(|b| (b, false)),
                    )
                    .map(|(b, enabled)| {
                        let cs = if let Some(ct) = ct.as_ref() {
                            let block = b.to_possible_value().unwrap().get_name().to_string();
                            b.colour(&block, ct)
                        } else {
                            b.to_possible_value().unwrap().get_name().into()
                        };
                        (cs, enabled)
                    })
                    .collect()
            }
            BlockType::Endpoint => {
                let enabled_blocks = print_settings.endpoint_blocks.as_ref().unwrap();
                enabled_blocks
                    .iter()
                    .map(|b| (b, true))
                    .chain(
                        EndpointBlocks::all_blocks()
                            .iter()
                            .filter(|b| !enabled_blocks.contains(b))
                            .map(|b| (b, false)),
                    )
                    .map(|(b, enabled)| {
                        let cs = if let Some(ct) = ct.as_ref() {
                            let block = b.to_possible_value().unwrap().get_name().to_string();
                            b.colour(&block, ct)
                        } else {
                            b.to_possible_value().unwrap().get_name().into()
                        };
                        (cs, enabled)
                    })
                    .collect()
            }
        };

        let index =
            if let State::BlockEditor { selected_index, .. } = &mut *self.state.lock().unwrap() {
                (*selected_index).min(blocks.len() - 1)
            } else {
                0
            };

        let new_mode = State::BlockEditor {
            block_type,
            selected_index: index,
            blocks,
        };
        // set selected line for block editor
        self.selected_line = Some(index);

        *self.state.lock().unwrap() = new_mode;
    }

    fn write_edit_blocks(&mut self) {
        if let State::BlockEditor {
            block_type, blocks, ..
        } = &*self.state.lock().unwrap()
        {
            let mut print_settings = self.print_settings.lock().unwrap();
            match block_type {
                BlockType::Device => {
                    print_settings.device_blocks = Some(
                        blocks
                            .iter()
                            .filter(|(_, enabled)| *enabled)
                            .map(|b| DeviceBlocks::from_str(&b.0, true).unwrap())
                            .collect(),
                    );
                }
                BlockType::Bus => {
                    print_settings.bus_blocks = Some(
                        blocks
                            .iter()
                            .filter(|(_, enabled)| *enabled)
                            .map(|b| BusBlocks::from_str(&b.0, true).unwrap())
                            .collect(),
                    );
                }
                BlockType::Config => {
                    print_settings.config_blocks = Some(
                        blocks
                            .iter()
                            .filter(|(_, enabled)| *enabled)
                            .map(|b| ConfigurationBlocks::from_str(&b.0, true).unwrap())
                            .collect(),
                    );
                }
                BlockType::Interface => {
                    print_settings.interface_blocks = Some(
                        blocks
                            .iter()
                            .filter(|(_, enabled)| *enabled)
                            .map(|b| InterfaceBlocks::from_str(&b.0, true).unwrap())
                            .collect(),
                    );
                }
                BlockType::Endpoint => {
                    print_settings.endpoint_blocks = Some(
                        blocks
                            .iter()
                            .filter(|(_, enabled)| *enabled)
                            .map(|b| EndpointBlocks::from_str(&b.0, true).unwrap())
                            .collect(),
                    );
                }
            }
        };
    }

    fn draw_edit_blocks(&mut self) -> Result<()> {
        self.buffer.clear();

        if let State::BlockEditor {
            selected_index,
            blocks,
            ..
        } = &mut *self.state.lock().unwrap()
        {
            for (i, (block, enabled)) in blocks.iter().enumerate() {
                let block_string = if i == *selected_index {
                    block.clone().normal().to_string()
                } else {
                    block.to_string()
                };
                if *enabled {
                    writeln!(self.buffer, " [x] {block_string}")?;
                } else {
                    writeln!(self.buffer, " [ ] {block_string}")?;
                }
            }
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
        self.scroll_offset = (self.scroll_offset).min(self.max_offset);

        // print the visible portion of the buffer
        for (i, line) in lines
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.available_rows)
        {
            if self.selected_line == Some(i) && !matches!(&*self.state.lock().unwrap(), State::Help)
            {
                let stripped_line = strip_ansi_codes(line);
                // use '>' to indicate selected line if no color
                // not that pretty but works...
                if self.print_settings.lock().unwrap().colours.is_none()
                    || env::var("NO_COLOR").is_ok_and(|v| v == "1")
                {
                    let mut indicated = stripped_line.chars().skip(1).collect::<String>();
                    indicated.insert(0, '>');
                    write!(stdout, "{}\n\r", indicated.bold().on_bright_purple())?;
                } else {
                    write!(stdout, "{}\n\r", stripped_line.bold().on_bright_purple())?;
                }
            } else {
                write!(stdout, "{line}\n\r")?;
            }
        }

        // status bar with key bindings
        self.draw_footer(&mut stdout)?;

        stdout.flush()?;

        Ok(())
    }
}
