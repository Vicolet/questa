//! Typst-based PDF export.
//!
//! The export writes two files to `<data-dir>/exports/`:
//!
//! - `questa-<timestamp>.json` — the data, schema-stable
//! - `questa-<timestamp>.typ`  — the template (a copy of
//!   `templates/export.typ` with the data filename substituted)
//!
//! If the `typst` binary is on `PATH`, a sibling `.pdf` is also produced.
//! Otherwise the user can compile by hand:
//!
//! ```sh
//! typst compile questa-<timestamp>.typ
//! ```
//!
//! Separating data and template means the template can be edited and
//! re-compiled without re-running questa, and the JSON has no Typst
//! escaping concerns.

use crate::app::{Counts, Filter};
use crate::data::{Application, Tracker};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

const TEMPLATE: &str = include_str!("../templates/export.typ");
const DATA_PLACEHOLDER: &str = "__DATA_JSON__";

/// Outcome of an export attempt. The `.typ` and `.json` are always
/// written on success. `pdf_path` is only set if a `typst` invocation
/// completed successfully.
#[derive(Debug)]
pub struct ExportResult {
    pub typ_path: PathBuf,
    pub json_path: PathBuf,
    pub pdf_path: Option<PathBuf>,
    pub status: TypstStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TypstStatus {
    /// `typst compile` succeeded; `pdf_path` is set.
    Compiled,
    /// The `typst` binary was not found on PATH.
    NotInstalled,
    /// `typst` ran but returned a non-zero exit code. The first line of
    /// stderr is captured here for the user-facing message.
    CompileFailed(String),
}

#[derive(Serialize)]
struct ExportData<'a> {
    exported_at: String,
    filter: &'static str,
    stats: ExportStats,
    applications: Vec<ExportApp<'a>>,
}

#[derive(Serialize)]
struct ExportStats {
    total: usize,
    active: usize,
    interview: usize,
    rejected: usize,
    ghosted: usize,
}

#[derive(Serialize)]
struct ExportApp<'a> {
    id: u32,
    company: &'a str,
    position: &'a str,
    status: &'a str,
    location: &'a str,
    app_type: &'a str,
    reference: &'a str,
    url: &'a str,
    applied_date: &'a str,
    deadline: &'a str,
    folder: &'a str,
    next_action: &'a str,
    next_action_date: &'a str,
    contacts: Vec<ExportContact<'a>>,
    notes: Vec<ExportNote<'a>>,
}

#[derive(Serialize)]
struct ExportContact<'a> {
    date: &'a str,
    info: &'a str,
}

#[derive(Serialize)]
struct ExportNote<'a> {
    date: &'a str,
    text: &'a str,
}

impl<'a> From<&'a Application> for ExportApp<'a> {
    fn from(a: &'a Application) -> Self {
        Self {
            id: a.id,
            company: &a.company,
            position: &a.position,
            status: &a.status,
            location: a.location.as_deref().unwrap_or(""),
            app_type: a.app_type.as_deref().unwrap_or(""),
            reference: a.reference.as_deref().unwrap_or(""),
            url: a.url.as_deref().unwrap_or(""),
            applied_date: a.applied_date.as_deref().unwrap_or(""),
            deadline: a.deadline.as_deref().unwrap_or(""),
            folder: a.folder.as_deref().unwrap_or(""),
            next_action: a.next_action.as_deref().unwrap_or(""),
            next_action_date: a.next_action_date.as_deref().unwrap_or(""),
            contacts: a
                .contacts
                .iter()
                .map(|c| ExportContact {
                    date: &c.date,
                    info: &c.info,
                })
                .collect(),
            notes: a
                .notes
                .iter()
                .map(|n| ExportNote {
                    date: &n.date,
                    text: &n.text,
                })
                .collect(),
        }
    }
}

/// Render the tracker (filtered by `filter`) to `<data_dir>/exports/`.
/// Returns the paths written and the outcome of the typst invocation.
pub fn export(tracker: &Tracker, filter: Filter, data_dir: &Path) -> Result<ExportResult> {
    let exports_dir = data_dir.join("exports");
    std::fs::create_dir_all(&exports_dir)
        .with_context(|| format!("creating {}", exports_dir.display()))?;

    let stem = format!("questa-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let json_path = exports_dir.join(format!("{stem}.json"));
    let typ_path = exports_dir.join(format!("{stem}.typ"));
    let pdf_path = exports_dir.join(format!("{stem}.pdf"));

    write_data_file(tracker, filter, &json_path)?;
    write_template_file(&json_path, &typ_path)?;

    let status = invoke_typst(&typ_path, &pdf_path);
    let pdf_path = if status == TypstStatus::Compiled {
        Some(pdf_path)
    } else {
        None
    };

    Ok(ExportResult {
        typ_path,
        json_path,
        pdf_path,
        status,
    })
}

fn write_data_file(tracker: &Tracker, filter: Filter, path: &Path) -> Result<()> {
    let apps: Vec<&Application> = tracker
        .applications
        .iter()
        .filter(|a| filter.matches(&a.status))
        .collect();
    let counts = Counts::from_tracker(tracker);
    let data = ExportData {
        exported_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        filter: filter.label(),
        stats: ExportStats {
            total: counts.total,
            active: counts.active,
            interview: counts.interview,
            rejected: counts.rejected,
            ghosted: counts.ghosted,
        },
        applications: apps.iter().map(|a| (*a).into()).collect(),
    };
    let json = serde_json::to_string_pretty(&data).context("serializing export data")?;
    std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn write_template_file(json_path: &Path, typ_path: &Path) -> Result<()> {
    // The template references the data file by its filename only — the
    // .typ and .json are siblings, and typst resolves relative paths
    // against the .typ's directory.
    let data_name = json_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("export json path has no UTF-8 filename")?;
    let rendered = TEMPLATE.replace(DATA_PLACEHOLDER, data_name);
    std::fs::write(typ_path, rendered)
        .with_context(|| format!("writing {}", typ_path.display()))?;
    Ok(())
}

fn invoke_typst(typ_path: &Path, pdf_path: &Path) -> TypstStatus {
    let outcome = Command::new("typst")
        .arg("compile")
        .arg(typ_path)
        .arg(pdf_path)
        .output();
    match outcome {
        Ok(out) if out.status.success() => TypstStatus::Compiled,
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let first_line = stderr.lines().next().unwrap_or("typst exited non-zero");
            TypstStatus::CompileFailed(first_line.to_string())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => TypstStatus::NotInstalled,
        Err(e) => TypstStatus::CompileFailed(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Contact, Meta, Note};

    fn sample_tracker() -> Tracker {
        Tracker {
            applications: vec![
                Application {
                    id: 1,
                    company: "Acme".into(),
                    position: "Engineer".into(),
                    location: Some("Zurich".into()),
                    app_type: Some("full-time".into()),
                    reference: None,
                    url: Some("https://example.com".into()),
                    applied_date: Some("2026-04-12".into()),
                    deadline: None,
                    folder: None,
                    status: "interview".into(),
                    contacts: vec![Contact {
                        date: "2026-04-20".into(),
                        info: "Anna".into(),
                    }],
                    notes: vec![Note {
                        date: "2026-04-12".into(),
                        text: "applied via careers page".into(),
                    }],
                    next_action: Some("prepare interview".into()),
                    next_action_date: Some("2026-05-01".into()),
                },
                Application {
                    id: 2,
                    company: "Beta".into(),
                    position: "Analyst".into(),
                    location: None,
                    app_type: None,
                    reference: None,
                    url: None,
                    applied_date: Some("2026-04-01".into()),
                    deadline: None,
                    folder: None,
                    status: "rejected".into(),
                    contacts: vec![],
                    notes: vec![],
                    next_action: None,
                    next_action_date: None,
                },
            ],
            meta: Meta {
                next_id: 3,
                version: "2".into(),
            },
        }
    }

    #[test]
    fn export_creates_exports_dir_and_three_filenames() {
        let dir = tempfile::tempdir().unwrap();
        let tracker = sample_tracker();
        let result = export(&tracker, Filter::All, dir.path()).unwrap();
        assert!(result.typ_path.exists());
        assert!(result.json_path.exists());
        assert_eq!(
            result.typ_path.parent(),
            Some(dir.path().join("exports").as_path())
        );
        let stem_typ = result
            .typ_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let stem_json = result
            .json_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(stem_typ, stem_json, "paired filenames must share a stem");
    }

    #[test]
    fn export_data_respects_filter() {
        let dir = tempfile::tempdir().unwrap();
        let tracker = sample_tracker();
        // Active filter excludes the "rejected" entry.
        let result = export(&tracker, Filter::Active, dir.path()).unwrap();
        let json = std::fs::read_to_string(&result.json_path).unwrap();
        assert!(
            json.contains("\"Acme\""),
            "Acme (interview) must be present"
        );
        assert!(
            !json.contains("\"Beta\""),
            "Beta (rejected) must be filtered out"
        );
        assert!(json.contains("\"filter\": \"Active\""));
    }

    #[test]
    fn export_data_includes_global_stats_not_filtered_counts() {
        // Stats reflect the whole tracker, not just the filtered slice —
        // the cover page should always show the true totals.
        let dir = tempfile::tempdir().unwrap();
        let tracker = sample_tracker();
        let result = export(&tracker, Filter::Active, dir.path()).unwrap();
        let json = std::fs::read_to_string(&result.json_path).unwrap();
        assert!(json.contains("\"total\": 2"));
        assert!(json.contains("\"rejected\": 1"));
    }

    #[test]
    fn template_references_sibling_json_by_filename() {
        let dir = tempfile::tempdir().unwrap();
        let tracker = sample_tracker();
        let result = export(&tracker, Filter::All, dir.path()).unwrap();
        let typ = std::fs::read_to_string(&result.typ_path).unwrap();
        let json_name = result
            .json_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            typ.contains(&format!("json(\"{json_name}\")")),
            "template must reference {json_name}"
        );
        assert!(
            !typ.contains(DATA_PLACEHOLDER),
            "placeholder must be substituted"
        );
    }

    /// End-to-end check: if `typst` is on PATH, the export must actually
    /// produce a valid-looking PDF (one starting with `%PDF-`). On systems
    /// without typst we accept the `NotInstalled` outcome.
    #[test]
    fn export_compiles_to_pdf_when_typst_available() {
        let dir = tempfile::tempdir().unwrap();
        let tracker = sample_tracker();
        let result = export(&tracker, Filter::All, dir.path()).unwrap();
        match &result.status {
            TypstStatus::Compiled => {
                let pdf = result.pdf_path.as_ref().expect("pdf path");
                assert!(pdf.exists(), "pdf must exist on disk");
                let head = std::fs::read(pdf).unwrap();
                assert!(
                    head.starts_with(b"%PDF-"),
                    "output does not look like a PDF"
                );
            }
            TypstStatus::NotInstalled => {
                eprintln!("skipping pdf assertion: typst not on PATH");
            }
            TypstStatus::CompileFailed(msg) => {
                panic!(
                    "typst compile failed: {msg}\ntyp at: {}",
                    result.typ_path.display()
                );
            }
        }
    }

    /// Manual sanity check: render the bundled example dataset and leave
    /// the output in `/tmp/questa-real/exports/` for inspection. Run with
    /// `cargo test -- --ignored --nocapture export_real_example`.
    #[test]
    #[ignore = "writes PDF to /tmp for manual inspection"]
    fn export_real_example() {
        let _ = std::fs::remove_dir_all("/tmp/questa-real");
        std::fs::create_dir_all("/tmp/questa-real").unwrap();
        std::fs::copy(
            "examples/applications.json",
            "/tmp/questa-real/applications.json",
        )
        .unwrap();
        let tracker =
            crate::data::load(std::path::Path::new("/tmp/questa-real/applications.json")).unwrap();
        let result = export(
            &tracker,
            Filter::All,
            std::path::Path::new("/tmp/questa-real"),
        )
        .unwrap();
        eprintln!("typ:  {}", result.typ_path.display());
        eprintln!("json: {}", result.json_path.display());
        if let Some(pdf) = &result.pdf_path {
            eprintln!("pdf:  {}", pdf.display());
        }
        eprintln!("status: {:?}", result.status);
        assert!(result.typ_path.exists());
    }

    #[test]
    fn export_emits_unicode_safely() {
        let dir = tempfile::tempdir().unwrap();
        let mut tracker = sample_tracker();
        tracker.applications[0].company = "Café Crème 株式会社".into();
        tracker.applications[0].notes.push(Note {
            date: "2026-05-01".into(),
            text: "Notes with \"quotes\" and \\backslashes\\ and #hash".into(),
        });
        let result = export(&tracker, Filter::All, dir.path()).unwrap();
        let json = std::fs::read_to_string(&result.json_path).unwrap();
        // serde_json escapes properly — round-trip back to verify.
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let apps = parsed["applications"].as_array().unwrap();
        assert_eq!(apps[0]["company"], "Café Crème 株式会社");
    }
}
