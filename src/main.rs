//! Entry point: terminal setup, event loop, panic handler.

mod app;
mod data;
mod export;
mod logging;
mod text;
mod ui;

use crate::app::{App, Filter, Mode};
use crate::text::TextAction;
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
    // Logging is best-effort — if it can't write to the state dir, run anyway.
    if let Ok(path) = logging::init() {
        tracing::info!(version = env!("CARGO_PKG_VERSION"), log = %path.display(), "questa starting");
    }

    let args: Vec<String> = std::env::args().collect();
    let data_path = parse_data_arg(&args)
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(data::find_data_file)?;
    tracing::info!(path = %data_path.display(), "loading tracker");
    let tracker = data::load(&data_path)?;
    tracing::info!(applications = tracker.applications.len(), "tracker loaded");
    let mut app = App::new(tracker, data_path);

    setup_panic_hook();
    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    tracing::info!(error = result.is_err(), "questa exiting");
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
                _ => {
                    if let Some(action) = text_action_for(code, mods) {
                        app.apply_text_action(action);
                    }
                }
            }
            return;
        }
        Mode::ContactInput { .. } => {
            match code {
                KeyCode::Esc => app.contact_cancel(),
                KeyCode::Enter => app.contact_confirm(),
                _ => {
                    if let Some(action) = text_action_for(code, mods) {
                        app.apply_text_action(action);
                    }
                }
            }
            return;
        }
        Mode::Form(_) => {
            match code {
                KeyCode::Esc => app.form_cancel(),
                KeyCode::Enter => app.form_save(),
                KeyCode::Char('s') if mods.contains(KeyModifiers::CONTROL) => app.form_save(),
                KeyCode::Tab | KeyCode::Down => app.form_focus_next(),
                KeyCode::BackTab | KeyCode::Up => app.form_focus_prev(),
                _ => {
                    if let Some(action) = text_action_for(code, mods) {
                        app.apply_text_action(action);
                    }
                }
            }
            return;
        }
        Mode::ConfirmDelete { .. } => {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => app.delete_confirm(),
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => app.delete_cancel(),
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
        KeyCode::Char('U') => app.open_selected_url(),
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Char('s') => app.open_status_picker(),
        KeyCode::Char('n') => app.open_note_input(),
        KeyCode::Char('c') => app.open_contact_input(),
        KeyCode::Char('a') => app.open_add_form(),
        KeyCode::Char('e') => app.open_edit_form(),
        KeyCode::Char('d') => app.open_delete_confirm(),
        KeyCode::Char('u') => app.undo(),
        KeyCode::Char('x') => app.export_typst(),
        KeyCode::Char('?') => app.toggle_help(),
        _ => {}
    }
}

/// Map a key chord to a [`TextAction`] suitable for any single-line text
/// input (form field, note prompt, contact prompt). Returns `None` for
/// keys with no editing meaning so the per-mode dispatcher can decide
/// whether to ignore them.
///
/// Bindings follow the readline / Emacs conventions most TUI users
/// already know: Ctrl-A/E for home/end, Ctrl-W to delete the previous
/// word, Ctrl-U to clear the line, Ctrl-Left/Right (or Alt-B/F) to jump
/// by word.
fn text_action_for(code: KeyCode, mods: KeyModifiers) -> Option<TextAction> {
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let alt = mods.contains(KeyModifiers::ALT);
    match (code, ctrl, alt) {
        (KeyCode::Left, true, _) | (KeyCode::Char('b'), _, true) => Some(TextAction::WordLeft),
        (KeyCode::Right, true, _) | (KeyCode::Char('f'), _, true) => Some(TextAction::WordRight),
        (KeyCode::Left, false, false) => Some(TextAction::Left),
        (KeyCode::Right, false, false) => Some(TextAction::Right),
        (KeyCode::Home, ..) | (KeyCode::Char('a'), true, _) => Some(TextAction::Home),
        (KeyCode::End, ..) | (KeyCode::Char('e'), true, _) => Some(TextAction::End),
        (KeyCode::Backspace, ..) => Some(TextAction::Backspace),
        (KeyCode::Delete, ..) => Some(TextAction::Delete),
        (KeyCode::Char('w'), true, _) => Some(TextAction::DeleteWordBack),
        (KeyCode::Char('u'), true, _) => Some(TextAction::Clear),
        (KeyCode::Char(c), false, false) => Some(TextAction::Insert(c)),
        _ => None,
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
