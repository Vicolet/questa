//! File-based structured logging via [`tracing`].
//!
//! A TUI app can't write to stderr without corrupting the alternate
//! screen, so all log output is sent to a file under the XDG state
//! directory:
//!
//! - `$XDG_STATE_HOME/questa/questa.log`
//! - or `$HOME/.local/state/questa/questa.log` as a fallback
//!
//! The active level is read from the standard `RUST_LOG` environment
//! variable (e.g. `RUST_LOG=questa=debug questa`). When the variable is
//! absent, the default level is `info`.
//!
//! Initialisation is best-effort: if the log directory cannot be
//! created or opened, questa starts anyway with logging disabled. Users
//! lose observability, not the ability to track their applications.
//!
//! Tests deliberately never call [`init`]; their tracing calls become
//! no-ops because no subscriber is installed.

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt};

const DEFAULT_LEVEL: &str = "info";

/// Install a global tracing subscriber that writes JSON-ish line records
/// to the questa log file. Returns the path that was opened so the
/// caller can log it.
pub fn init() -> Result<PathBuf> {
    let dir = state_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("questa.log");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LEVEL));

    // `try_init` rather than `init` so a second call in a test process
    // does not panic. We discard the error because there is only one
    // useful caller (main) and silent re-init is the right behaviour.
    let _ = fmt()
        .with_writer(file)
        .with_env_filter(env_filter)
        .with_ansi(false)
        .with_target(true)
        .try_init();

    Ok(path)
}

fn state_dir() -> PathBuf {
    if let Ok(p) = std::env::var("XDG_STATE_HOME") {
        if !p.is_empty() {
            return PathBuf::from(p).join("questa");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/state/questa");
    }
    // Last resort: log into the current directory. Better than panicking
    // in a TUI startup path.
    PathBuf::from(".")
}
