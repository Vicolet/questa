//! Entry point: terminal setup, event loop, panic handler.

mod app;
mod data;
mod ui;

use crate::app::{App, Filter, Mode};
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::time::Duration;
use std::{io, panic};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data_path = parse_data_arg(&args)
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(data::find_data_file)?;
    let tracker = data::load(&data_path)?;
    let mut app = App::new(tracker, data_path);

    setup_panic_hook();
    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}

/// Tiny manual parser to avoid pulling in clap for one optional flag.
/// Accepts `--data <path>` or `-d <path>`.
fn parse_data_arg(args: &[String]) -> Option<String> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("questa {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--data" | "-d" => return it.next().cloned(),
            other if other.starts_with("--data=") => {
                return Some(other.trim_start_matches("--data=").to_string());
            }
            _ => {}
        }
    }
    None
}

fn print_usage() {
    println!(
        "questa {} — TUI for tracking your job quest\n\n\
         USAGE:\n  \
           questa [OPTIONS]\n\n\
         OPTIONS:\n  \
           -d, --data <PATH>   Path to applications.json (default: auto-detect)\n  \
           -h, --help          Print this help\n  \
           -V, --version       Print version\n\n\
         If --data is not provided, questa looks for `applications.json` in the\n\
         current directory or `examples/applications.json` for the bundled demo.",
        env!("CARGO_PKG_VERSION")
    );
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, app))?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Any key dismisses the error/flash before reaching modal handlers.
                    let had_msg = app.error.is_some() || app.flash.is_some();
                    if had_msg {
                        app.dismiss_flash();
                    }
                    handle_key(app, key.code, key.modifiers);
                }
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    // ── Modal handlers ─────────────────────────────────────────────────────
    match &app.mode {
        Mode::Help => {
            match code {
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => app.toggle_help(),
                _ => {}
            }
            return;
        }
        Mode::Search => {
            match code {
                KeyCode::Esc => app.exit_search(),
                KeyCode::Enter => app.confirm_search(),
                KeyCode::Backspace => app.search_pop(),
                KeyCode::Char(c) if !mods.contains(KeyModifiers::CONTROL) => app.search_push(c),
                _ => {}
            }
            return;
        }
        Mode::StatusPicker { .. } => {
            match code {
                KeyCode::Esc => app.status_picker_cancel(),
                KeyCode::Enter => app.status_picker_confirm(),
                KeyCode::Char('j') | KeyCode::Down => app.status_picker_move(1),
                KeyCode::Char('k') | KeyCode::Up => app.status_picker_move(-1),
                _ => {}
            }
            return;
        }
        Mode::NoteInput { .. } => {
            match code {
                KeyCode::Esc => app.note_cancel(),
                KeyCode::Enter => app.note_confirm(),
                KeyCode::Backspace => app.note_pop(),
                KeyCode::Char(c) if !mods.contains(KeyModifiers::CONTROL) => app.note_push(c),
                _ => {}
            }
            return;
        }
        Mode::Normal => {}
    }

    // ── Normal mode ────────────────────────────────────────────────────────
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit(),
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('g') => app.move_top(),
        KeyCode::Char('G') => app.move_bottom(),
        KeyCode::Char('1') => app.set_filter(Filter::All),
        KeyCode::Char('2') => app.set_filter(Filter::Active),
        KeyCode::Char('3') => app.set_filter(Filter::Interview),
        KeyCode::Char('4') => app.set_filter(Filter::Rejected),
        KeyCode::Char('5') => app.set_filter(Filter::Ghosted),
        KeyCode::Tab => app.cycle_filter(),
        KeyCode::Char('o') => app.cycle_sort(),
        KeyCode::Char('O') => app.open_selected_folder(),
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Char('s') => app.open_status_picker(),
        KeyCode::Char('n') => app.open_note_input(),
        KeyCode::Char('?') => app.toggle_help(),
        _ => {}
    }
}

// ── Terminal lifecycle ──────────────────────────────────────────────────────

type Tui = Terminal<CrosstermBackend<io::Stdout>>;

fn setup_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn setup_panic_hook() {
    let original = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original(info);
    }));
}
