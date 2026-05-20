//! Data structures matching `applications.json` and date helpers.

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tracker {
    pub applications: Vec<Application>,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    pub next_id: u32,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Application {
    pub id: u32,
    pub company: String,
    pub position: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(rename = "type", default)]
    pub app_type: Option<String>,
    #[serde(rename = "ref", default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub applied_date: Option<String>,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub salary: Option<serde_json::Value>,
    #[serde(default)]
    pub folder: Option<String>,
    pub status: String,
    #[serde(default)]
    pub contacts: Vec<Contact>,
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default)]
    pub next_action: Option<String>,
    #[serde(default)]
    pub next_action_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contact {
    pub date: String,
    pub info: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub date: String,
    pub text: String,
}

/// Try a handful of likely locations for `applications.json`.
///
/// Looks for, in order:
/// 1. `applications.json` in the current directory
/// 2. `examples/applications.json` (bundled demo dataset)
/// 3. `$XDG_DATA_HOME/questa/applications.json`
///    (or `$HOME/.local/share/questa/applications.json` as fallback)
pub fn find_data_file() -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = vec![
        PathBuf::from("applications.json"),
        PathBuf::from("examples/applications.json"),
    ];
    if let Some(dir) = data_dir() {
        candidates.push(dir.join("questa/applications.json"));
    }
    for c in &candidates {
        if c.exists() {
            return Ok(c.canonicalize()?);
        }
    }
    anyhow::bail!(
        "applications.json not found.\n\
         Tried: {}\n\
         Hint: pass --data <PATH> or place applications.json in the working directory.",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn data_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("XDG_DATA_HOME") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".local/share"))
}

pub fn load(path: &Path) -> Result<Tracker> {
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let t: Tracker = serde_json::from_str(&s)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(t)
}

pub fn save(path: &Path, t: &Tracker) -> Result<()> {
    let s = serde_json::to_string_pretty(t)
        .with_context(|| format!("serializing tracker"))?;
    std::fs::write(path, format!("{s}\n"))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn today_str() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

// ── Date helpers ────────────────────────────────────────────────────────────

fn parse(d: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
}

fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

pub fn days_since(d: &str) -> Option<i64> {
    parse(d).map(|date| (today() - date).num_days())
}

pub fn days_until(d: &str) -> Option<i64> {
    parse(d).map(|date| (date - today()).num_days())
}

pub fn rel_date(d: Option<&str>) -> String {
    let Some(d) = d else { return "—".into() };
    let Some(n) = days_since(d) else { return d.into() };
    match n {
        0 => "today".into(),
        1 => "yesterday".into(),
        n if n > 0 => format!("{n}d ago"),
        n => format!("in {}d", -n),
    }
}
