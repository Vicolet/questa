//! Application state: filtering, navigation, search, mutation.

use crate::data::{self, Application, Note, Tracker};
use anyhow::Result;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Active,
    Interview,
    Rejected,
    Ghosted,
}

impl Filter {
    pub fn matches(&self, status: &str) -> bool {
        match self {
            Filter::All => true,
            Filter::Active => matches!(
                status,
                "applied" | "screening" | "interview" | "technical" | "offer"
            ),
            Filter::Interview => matches!(status, "interview" | "technical"),
            Filter::Rejected => status == "rejected",
            Filter::Ghosted => status == "ghosted",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Interview => "Interview",
            Filter::Rejected => "Rejected",
            Filter::Ghosted => "Ghosted",
        }
    }

    pub const ALL: [Filter; 5] = [
        Filter::All,
        Filter::Active,
        Filter::Interview,
        Filter::Rejected,
        Filter::Ghosted,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    DateDesc,
    Status,
    Company,
}

impl Sort {
    pub fn label(&self) -> &'static str {
        match self {
            Sort::DateDesc => "date↓",
            Sort::Status => "status",
            Sort::Company => "company",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Sort::DateDesc => Sort::Status,
            Sort::Status => Sort::Company,
            Sort::Company => Sort::DateDesc,
        }
    }
}

/// Priority of a status when sorting by status (lower = first).
fn status_priority(status: &str) -> u8 {
    match status {
        "interview" => 0,
        "technical" => 1,
        "offer" => 2,
        "screening" => 3,
        "applied" => 4,
        "accepted" => 5,
        "withdrawn" => 6,
        "rejected" => 7,
        "ghosted" => 8,
        _ => 99,
    }
}

pub const STATUSES: [&str; 9] = [
    "applied",
    "screening",
    "interview",
    "technical",
    "offer",
    "accepted",
    "rejected",
    "withdrawn",
    "ghosted",
];

#[derive(Debug, Clone)]
pub enum Mode {
    Normal,
    Search,
    Help,
    StatusPicker { idx: usize },
    NoteInput { buffer: String },
}

pub struct App {
    pub tracker: Tracker,
    pub filter: Filter,
    pub sort: Sort,
    pub mode: Mode,
    pub search: String,
    pub selected: usize,
    pub error: Option<String>,
    pub flash: Option<String>,
    pub should_quit: bool,
    data_path: PathBuf,
    matcher: SkimMatcherV2,
}

impl App {
    pub fn new(tracker: Tracker, data_path: PathBuf) -> Self {
        Self {
            tracker,
            filter: Filter::Active,
            sort: Sort::DateDesc,
            mode: Mode::Normal,
            search: String::new(),
            selected: 0,
            error: None,
            flash: None,
            should_quit: false,
            data_path,
            matcher: SkimMatcherV2::default().ignore_case(),
        }
    }

    /// Apps after filter + search, sorted according to `self.sort`.
    pub fn filtered(&self) -> Vec<&Application> {
        let mut apps: Vec<&Application> = self
            .tracker
            .applications
            .iter()
            .filter(|a| self.filter.matches(&a.status))
            .filter(|a| {
                if self.search.is_empty() {
                    true
                } else {
                    let hay = format!("{} {}", a.company, a.position);
                    self.matcher.fuzzy_match(&hay, &self.search).is_some()
                }
            })
            .collect();

        match self.sort {
            Sort::DateDesc => {
                apps.sort_by(|a, b| b.applied_date.cmp(&a.applied_date));
            }
            Sort::Status => {
                apps.sort_by(|a, b| {
                    status_priority(&a.status)
                        .cmp(&status_priority(&b.status))
                        .then_with(|| b.applied_date.cmp(&a.applied_date))
                });
            }
            Sort::Company => {
                apps.sort_by(|a, b| {
                    a.company
                        .to_lowercase()
                        .cmp(&b.company.to_lowercase())
                        .then_with(|| b.applied_date.cmp(&a.applied_date))
                });
            }
        }
        apps
    }

    pub fn selected_app(&self) -> Option<Application> {
        self.filtered().get(self.selected).map(|a| (*a).clone())
    }

    fn selected_index_in_tracker(&self) -> Option<usize> {
        let id = self.selected_app()?.id;
        self.tracker.applications.iter().position(|a| a.id == id)
    }

    // ── Navigation ─────────────────────────────────────────────────────────

    pub fn move_down(&mut self) {
        let len = self.filtered().len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1).min(len - 1);
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_bottom(&mut self) {
        let len = self.filtered().len();
        if len == 0 {
            return;
        }
        self.selected = len - 1;
    }

    pub fn set_filter(&mut self, filter: Filter) {
        if self.filter != filter {
            self.filter = filter;
            self.selected = 0;
        }
    }

    pub fn cycle_filter(&mut self) {
        let i = Filter::ALL
            .iter()
            .position(|f| *f == self.filter)
            .unwrap_or(0);
        self.set_filter(Filter::ALL[(i + 1) % Filter::ALL.len()]);
    }

    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.selected = 0;
        self.flash = Some(format!("sort: {}", self.sort.label()));
    }

    // ── Search mode ────────────────────────────────────────────────────────

    pub fn enter_search(&mut self) {
        self.mode = Mode::Search;
        self.search.clear();
        self.selected = 0;
    }

    pub fn exit_search(&mut self) {
        self.mode = Mode::Normal;
        self.search.clear();
        self.selected = 0;
    }

    pub fn confirm_search(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn search_push(&mut self, c: char) {
        self.search.push(c);
        self.selected = 0;
    }

    pub fn search_pop(&mut self) {
        self.search.pop();
        self.selected = 0;
    }

    // ── Help ───────────────────────────────────────────────────────────────

    pub fn toggle_help(&mut self) {
        self.mode = match self.mode {
            Mode::Help => Mode::Normal,
            _ => Mode::Help,
        };
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    // ── Status picker mode ─────────────────────────────────────────────────

    pub fn open_status_picker(&mut self) {
        let Some(app) = self.selected_app() else {
            return;
        };
        let idx = STATUSES.iter().position(|s| *s == app.status).unwrap_or(0);
        self.mode = Mode::StatusPicker { idx };
    }

    pub fn status_picker_move(&mut self, delta: i32) {
        if let Mode::StatusPicker { idx } = &mut self.mode {
            let new = (*idx as i32 + delta).rem_euclid(STATUSES.len() as i32);
            *idx = new as usize;
        }
    }

    pub fn status_picker_confirm(&mut self) {
        let Mode::StatusPicker { idx } = &self.mode else {
            return;
        };
        let new_status = STATUSES[*idx].to_string();
        if let Some(tracker_idx) = self.selected_index_in_tracker() {
            let prev = self.tracker.applications[tracker_idx].status.clone();
            self.tracker.applications[tracker_idx].status = new_status.clone();
            if let Err(e) = self.save() {
                self.tracker.applications[tracker_idx].status = prev;
                self.error = Some(format!("save failed: {e}"));
            } else {
                self.flash = Some(format!("status → {new_status}"));
            }
        }
        self.mode = Mode::Normal;
    }

    pub fn status_picker_cancel(&mut self) {
        self.mode = Mode::Normal;
    }

    // ── Note input mode ────────────────────────────────────────────────────

    pub fn open_note_input(&mut self) {
        if self.selected_app().is_none() {
            return;
        }
        self.mode = Mode::NoteInput {
            buffer: String::new(),
        };
    }

    pub fn note_push(&mut self, c: char) {
        if let Mode::NoteInput { buffer } = &mut self.mode {
            buffer.push(c);
        }
    }

    pub fn note_pop(&mut self) {
        if let Mode::NoteInput { buffer } = &mut self.mode {
            buffer.pop();
        }
    }

    pub fn note_confirm(&mut self) {
        let Mode::NoteInput { buffer } = &self.mode else {
            return;
        };
        let text = buffer.trim().to_string();
        if text.is_empty() {
            self.mode = Mode::Normal;
            return;
        }
        if let Some(tracker_idx) = self.selected_index_in_tracker() {
            let note = Note {
                date: data::today_str(),
                text: text.clone(),
            };
            self.tracker.applications[tracker_idx].notes.push(note);
            if let Err(e) = self.save() {
                self.tracker.applications[tracker_idx].notes.pop();
                self.error = Some(format!("save failed: {e}"));
            } else {
                self.flash = Some("note added".into());
            }
        }
        self.mode = Mode::Normal;
    }

    pub fn note_cancel(&mut self) {
        self.mode = Mode::Normal;
    }

    // ── Persistence ────────────────────────────────────────────────────────

    fn save(&self) -> Result<()> {
        data::save(&self.data_path, &self.tracker)
    }

    pub fn dismiss_flash(&mut self) {
        self.flash = None;
        self.error = None;
    }

    // ── Open folder ────────────────────────────────────────────────────────

    /// Open the folder of the selected application in the system's default
    /// file manager. The folder field is interpreted as a path relative to
    /// the directory containing applications.json, unless it is absolute.
    pub fn open_selected_folder(&mut self) {
        let Some(app) = self.selected_app() else {
            self.flash = Some("no selection".into());
            return;
        };
        let Some(folder) = app.folder.as_deref().filter(|s| !s.is_empty()) else {
            self.flash = Some("no folder set on this entry".into());
            return;
        };
        let target = resolve_folder_path(&self.data_path, folder);
        if !target.exists() {
            self.error = Some(format!("folder not found: {}", target.display()));
            return;
        }
        match std::process::Command::new(open_command())
            .arg(&target)
            .spawn()
        {
            Ok(_) => self.flash = Some(format!("opened {}", target.display())),
            Err(e) => self.error = Some(format!("failed to open: {e}")),
        }
    }

    // ── Dashboard stats ────────────────────────────────────────────────────

    pub fn counts(&self) -> Counts {
        let mut c = Counts {
            total: self.tracker.applications.len(),
            ..Default::default()
        };
        for a in &self.tracker.applications {
            match a.status.as_str() {
                "applied" | "screening" | "interview" | "technical" | "offer" => c.active += 1,
                "rejected" => c.rejected += 1,
                "ghosted" => c.ghosted += 1,
                _ => {}
            }
            if matches!(a.status.as_str(), "interview" | "technical") {
                c.interview += 1;
            }
            if let Some(d) = a.next_action_date.as_deref() {
                if let Some(du) = data::days_until(d) {
                    let dead = matches!(
                        a.status.as_str(),
                        "rejected" | "ghosted" | "withdrawn" | "accepted"
                    );
                    if !dead {
                        if du < 0 {
                            c.overdue += 1;
                        } else if (0..=7).contains(&du) {
                            c.this_week += 1;
                        }
                    }
                }
            }
        }
        c
    }
}

/// Resolve a folder path declared in an application entry against the
/// directory that holds applications.json. Absolute paths are returned
/// unchanged so users can opt into a fixed location if they want.
pub fn resolve_folder_path(data_path: &Path, folder: &str) -> PathBuf {
    let p = PathBuf::from(folder);
    if p.is_absolute() {
        return p;
    }
    let base = data_path.parent().unwrap_or_else(|| Path::new("."));
    base.join(p)
}

fn open_command() -> &'static str {
    if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    }
}

#[derive(Debug, Default)]
pub struct Counts {
    pub total: usize,
    pub active: usize,
    pub interview: usize,
    pub rejected: usize,
    pub ghosted: usize,
    pub overdue: usize,
    pub this_week: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matches_groups_active_statuses() {
        assert!(Filter::Active.matches("applied"));
        assert!(Filter::Active.matches("screening"));
        assert!(Filter::Active.matches("interview"));
        assert!(Filter::Active.matches("offer"));
        assert!(!Filter::Active.matches("rejected"));
        assert!(!Filter::Active.matches("ghosted"));
    }

    #[test]
    fn filter_all_matches_everything() {
        for s in &[
            "applied",
            "screening",
            "interview",
            "rejected",
            "ghosted",
            "withdrawn",
        ] {
            assert!(Filter::All.matches(s), "All should match {s}");
        }
    }

    #[test]
    fn sort_next_cycles_through_three_modes() {
        assert_eq!(Sort::DateDesc.next(), Sort::Status);
        assert_eq!(Sort::Status.next(), Sort::Company);
        assert_eq!(Sort::Company.next(), Sort::DateDesc);
    }

    #[test]
    fn status_priority_orders_actionable_first() {
        // interview, technical, offer come before passive states.
        assert!(status_priority("interview") < status_priority("applied"));
        assert!(status_priority("interview") < status_priority("rejected"));
        assert!(status_priority("offer") < status_priority("screening"));
        // dead-ends are last.
        assert!(status_priority("ghosted") > status_priority("rejected"));
    }

    #[test]
    fn resolve_folder_keeps_absolute_paths_untouched() {
        let data = PathBuf::from("/etc/questa/applications.json");
        let abs = if cfg!(windows) {
            r"C:\Users\me\jobs\acme"
        } else {
            "/home/me/jobs/acme"
        };
        assert_eq!(resolve_folder_path(&data, abs), PathBuf::from(abs));
    }

    #[test]
    fn resolve_folder_joins_relative_to_data_dir() {
        let data = PathBuf::from("/home/me/jobs/applications.json");
        let resolved = resolve_folder_path(&data, "acme/intern");
        assert_eq!(resolved, PathBuf::from("/home/me/jobs/acme/intern"));
    }
}
