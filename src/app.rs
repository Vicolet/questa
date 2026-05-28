//! Application state: filtering, navigation, search, mutation.

use crate::data::{self, Application, Contact, Note, Tracker};
use anyhow::Result;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

const MAX_HISTORY: usize = 10;

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
    ContactInput { buffer: String },
    Form(AppForm),
    ConfirmDelete { id: u32, label: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKey {
    Company,
    Position,
    Location,
    Type,
    Ref,
    Url,
    AppliedDate,
    Deadline,
    Folder,
    Status,
    NextAction,
    NextActionDate,
}

#[derive(Debug, Clone)]
pub struct FormField {
    pub key: FieldKey,
    pub label: &'static str,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct AppForm {
    pub fields: Vec<FormField>,
    pub focus: usize,
    /// `None` for add, `Some(id)` for edit.
    pub edit_target_id: Option<u32>,
    pub error: Option<String>,
}

impl AppForm {
    fn new_empty(applied_default: String) -> Self {
        Self {
            fields: vec![
                FormField {
                    key: FieldKey::Company,
                    label: "Company *",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::Position,
                    label: "Position *",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::Location,
                    label: "Location",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::Type,
                    label: "Type",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::Ref,
                    label: "Ref",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::Url,
                    label: "URL",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::AppliedDate,
                    label: "Applied",
                    value: applied_default,
                },
                FormField {
                    key: FieldKey::Deadline,
                    label: "Deadline",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::Folder,
                    label: "Folder",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::Status,
                    label: "Status",
                    value: "applied".into(),
                },
                FormField {
                    key: FieldKey::NextAction,
                    label: "Next action",
                    value: String::new(),
                },
                FormField {
                    key: FieldKey::NextActionDate,
                    label: "Next date",
                    value: String::new(),
                },
            ],
            focus: 0,
            edit_target_id: None,
            error: None,
        }
    }

    fn from_application(app: &Application) -> Self {
        let mut f = Self::new_empty(app.applied_date.clone().unwrap_or_default());
        f.edit_target_id = Some(app.id);
        for field in f.fields.iter_mut() {
            field.value = match field.key {
                FieldKey::Company => app.company.clone(),
                FieldKey::Position => app.position.clone(),
                FieldKey::Location => app.location.clone().unwrap_or_default(),
                FieldKey::Type => app.app_type.clone().unwrap_or_default(),
                FieldKey::Ref => app.reference.clone().unwrap_or_default(),
                FieldKey::Url => app.url.clone().unwrap_or_default(),
                FieldKey::AppliedDate => app.applied_date.clone().unwrap_or_default(),
                FieldKey::Deadline => app.deadline.clone().unwrap_or_default(),
                FieldKey::Folder => app.folder.clone().unwrap_or_default(),
                FieldKey::Status => app.status.clone(),
                FieldKey::NextAction => app.next_action.clone().unwrap_or_default(),
                FieldKey::NextActionDate => app.next_action_date.clone().unwrap_or_default(),
            };
        }
        f
    }

    fn get(&self, key: FieldKey) -> &str {
        self.fields
            .iter()
            .find(|f| f.key == key)
            .map(|f| f.value.trim())
            .unwrap_or("")
    }

    fn opt(&self, key: FieldKey) -> Option<String> {
        let v = self.get(key);
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.get(FieldKey::Company).is_empty() {
            return Err("Company is required".into());
        }
        if self.get(FieldKey::Position).is_empty() {
            return Err("Position is required".into());
        }
        let status = self.get(FieldKey::Status);
        if !STATUSES.contains(&status) {
            return Err(format!("Status must be one of: {}", STATUSES.join(", ")));
        }
        for (key, label) in [
            (FieldKey::AppliedDate, "Applied"),
            (FieldKey::Deadline, "Deadline"),
            (FieldKey::NextActionDate, "Next date"),
        ] {
            let v = self.get(key);
            if !v.is_empty() && chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err() {
                return Err(format!("{label} must be YYYY-MM-DD"));
            }
        }
        Ok(())
    }
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
    history: VecDeque<Tracker>,
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
            history: VecDeque::with_capacity(MAX_HISTORY),
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
            let snapshot = self.tracker.clone();
            self.tracker.applications[tracker_idx].status = new_status.clone();
            match self.commit(snapshot.clone()) {
                Ok(()) => self.flash = Some(format!("status → {new_status}")),
                Err(e) => {
                    self.tracker = snapshot;
                    self.error = Some(format!("save failed: {e}"));
                }
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
            let snapshot = self.tracker.clone();
            let note = Note {
                date: data::today_str(),
                text: text.clone(),
            };
            self.tracker.applications[tracker_idx].notes.push(note);
            match self.commit(snapshot.clone()) {
                Ok(()) => self.flash = Some("note added".into()),
                Err(e) => {
                    self.tracker = snapshot;
                    self.error = Some(format!("save failed: {e}"));
                }
            }
        }
        self.mode = Mode::Normal;
    }

    pub fn note_cancel(&mut self) {
        self.mode = Mode::Normal;
    }

    // ── Contact input mode ─────────────────────────────────────────────────

    pub fn open_contact_input(&mut self) {
        if self.selected_app().is_none() {
            return;
        }
        self.mode = Mode::ContactInput {
            buffer: String::new(),
        };
    }

    pub fn contact_push(&mut self, c: char) {
        if let Mode::ContactInput { buffer } = &mut self.mode {
            buffer.push(c);
        }
    }

    pub fn contact_pop(&mut self) {
        if let Mode::ContactInput { buffer } = &mut self.mode {
            buffer.pop();
        }
    }

    pub fn contact_confirm(&mut self) {
        let Mode::ContactInput { buffer } = &self.mode else {
            return;
        };
        let info = buffer.trim().to_string();
        if info.is_empty() {
            self.mode = Mode::Normal;
            return;
        }
        if let Some(tracker_idx) = self.selected_index_in_tracker() {
            let snapshot = self.tracker.clone();
            let contact = Contact {
                date: data::today_str(),
                info: info.clone(),
            };
            self.tracker.applications[tracker_idx]
                .contacts
                .push(contact);
            match self.commit(snapshot.clone()) {
                Ok(()) => self.flash = Some("contact added".into()),
                Err(e) => {
                    self.tracker = snapshot;
                    self.error = Some(format!("save failed: {e}"));
                }
            }
        }
        self.mode = Mode::Normal;
    }

    pub fn contact_cancel(&mut self) {
        self.mode = Mode::Normal;
    }

    // ── Form mode (add / edit) ─────────────────────────────────────────────

    pub fn open_add_form(&mut self) {
        let form = AppForm::new_empty(data::today_str());
        self.mode = Mode::Form(form);
    }

    pub fn open_edit_form(&mut self) {
        let Some(app) = self.selected_app() else {
            return;
        };
        self.mode = Mode::Form(AppForm::from_application(&app));
    }

    pub fn form_focus_next(&mut self) {
        if let Mode::Form(f) = &mut self.mode {
            let n = f.fields.len();
            if n > 0 {
                f.focus = (f.focus + 1) % n;
            }
        }
    }

    pub fn form_focus_prev(&mut self) {
        if let Mode::Form(f) = &mut self.mode {
            let n = f.fields.len();
            if n > 0 {
                f.focus = (f.focus + n - 1) % n;
            }
        }
    }

    pub fn form_push(&mut self, c: char) {
        if let Mode::Form(f) = &mut self.mode {
            if let Some(field) = f.fields.get_mut(f.focus) {
                field.value.push(c);
            }
            f.error = None;
        }
    }

    pub fn form_pop(&mut self) {
        if let Mode::Form(f) = &mut self.mode {
            if let Some(field) = f.fields.get_mut(f.focus) {
                field.value.pop();
            }
            f.error = None;
        }
    }

    pub fn form_cancel(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn form_save(&mut self) {
        let form = match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Form(f) => f,
            other => {
                self.mode = other;
                return;
            }
        };
        if let Err(e) = form.validate() {
            let mut f = form;
            f.error = Some(e);
            self.mode = Mode::Form(f);
            return;
        }
        let snapshot = self.tracker.clone();
        let flash = match form.edit_target_id {
            None => self.apply_form_add(&form),
            Some(id) => self.apply_form_edit(id, &form),
        };
        match flash {
            Ok(msg) => match self.commit(snapshot.clone()) {
                Ok(()) => self.flash = Some(msg),
                Err(e) => {
                    self.tracker = snapshot;
                    self.error = Some(format!("save failed: {e}"));
                }
            },
            Err(e) => {
                self.tracker = snapshot;
                self.error = Some(e);
            }
        }
    }

    fn apply_form_add(&mut self, form: &AppForm) -> Result<String, String> {
        let id = self.tracker.meta.next_id;
        let new_app = Application {
            id,
            company: form.get(FieldKey::Company).to_string(),
            position: form.get(FieldKey::Position).to_string(),
            location: form.opt(FieldKey::Location),
            app_type: form.opt(FieldKey::Type),
            reference: form.opt(FieldKey::Ref),
            url: form.opt(FieldKey::Url),
            applied_date: form.opt(FieldKey::AppliedDate),
            deadline: form.opt(FieldKey::Deadline),
            folder: form.opt(FieldKey::Folder),
            status: form.get(FieldKey::Status).to_string(),
            contacts: Vec::new(),
            notes: Vec::new(),
            next_action: form.opt(FieldKey::NextAction),
            next_action_date: form.opt(FieldKey::NextActionDate),
        };
        self.tracker.applications.push(new_app);
        self.tracker.meta.next_id = id.saturating_add(1);
        Ok(format!("added #{id}"))
    }

    fn apply_form_edit(&mut self, id: u32, form: &AppForm) -> Result<String, String> {
        let Some(idx) = self.tracker.applications.iter().position(|a| a.id == id) else {
            return Err(format!("entry #{id} not found"));
        };
        let a = &mut self.tracker.applications[idx];
        a.company = form.get(FieldKey::Company).to_string();
        a.position = form.get(FieldKey::Position).to_string();
        a.location = form.opt(FieldKey::Location);
        a.app_type = form.opt(FieldKey::Type);
        a.reference = form.opt(FieldKey::Ref);
        a.url = form.opt(FieldKey::Url);
        a.applied_date = form.opt(FieldKey::AppliedDate);
        a.deadline = form.opt(FieldKey::Deadline);
        a.folder = form.opt(FieldKey::Folder);
        a.status = form.get(FieldKey::Status).to_string();
        a.next_action = form.opt(FieldKey::NextAction);
        a.next_action_date = form.opt(FieldKey::NextActionDate);
        Ok(format!("updated #{id}"))
    }

    // ── Delete with confirm ────────────────────────────────────────────────

    pub fn open_delete_confirm(&mut self) {
        let Some(app) = self.selected_app() else {
            return;
        };
        self.mode = Mode::ConfirmDelete {
            id: app.id,
            label: format!("#{} {} — {}", app.id, app.company, app.position),
        };
    }

    pub fn delete_cancel(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn delete_confirm(&mut self) {
        let id = match &self.mode {
            Mode::ConfirmDelete { id, .. } => *id,
            _ => return,
        };
        let Some(idx) = self.tracker.applications.iter().position(|a| a.id == id) else {
            self.mode = Mode::Normal;
            self.error = Some(format!("entry #{id} not found"));
            return;
        };
        let snapshot = self.tracker.clone();
        let removed = self.tracker.applications.remove(idx);
        let label = format!("deleted #{} {}", removed.id, removed.company);
        match self.commit(snapshot.clone()) {
            Ok(()) => {
                self.flash = Some(label);
                let len = self.filtered().len();
                if len == 0 {
                    self.selected = 0;
                } else if self.selected >= len {
                    self.selected = len - 1;
                }
            }
            Err(e) => {
                self.tracker = snapshot;
                self.error = Some(format!("save failed: {e}"));
            }
        }
        self.mode = Mode::Normal;
    }

    // ── Persistence + undo ─────────────────────────────────────────────────

    fn save(&self) -> Result<()> {
        data::save(&self.data_path, &self.tracker)
    }

    /// Persist the current tracker. On success, push `snapshot` (the
    /// pre-mutation tracker) onto the undo history, capped at `MAX_HISTORY`.
    /// On failure, the caller is responsible for restoring `self.tracker`
    /// from its own copy of the snapshot.
    fn commit(&mut self, snapshot: Tracker) -> Result<()> {
        self.save()?;
        self.history.push_back(snapshot);
        while self.history.len() > MAX_HISTORY {
            self.history.pop_front();
        }
        Ok(())
    }

    pub fn undo(&mut self) {
        let Some(snapshot) = self.history.pop_back() else {
            self.flash = Some("nothing to undo".into());
            return;
        };
        let backup = std::mem::replace(&mut self.tracker, snapshot);
        if let Err(e) = self.save() {
            self.tracker = backup;
            self.error = Some(format!("undo save failed: {e}"));
            return;
        }
        let len = self.filtered().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
        self.flash = Some("undone".into());
    }

    pub fn dismiss_flash(&mut self) {
        self.flash = None;
        self.error = None;
    }

    // ── Open folder / URL ──────────────────────────────────────────────────

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
        self.spawn_opener(target.as_os_str(), &format!("opened {}", target.display()));
    }

    /// Open the URL of the selected application in the system's default
    /// browser handler.
    pub fn open_selected_url(&mut self) {
        let Some(app) = self.selected_app() else {
            self.flash = Some("no selection".into());
            return;
        };
        let Some(url) = app.url.as_deref().filter(|s| !s.is_empty()) else {
            self.flash = Some("no url set on this entry".into());
            return;
        };
        let msg = format!("opened {url}");
        self.spawn_opener(std::ffi::OsStr::new(url), &msg);
    }

    fn spawn_opener(&mut self, arg: &std::ffi::OsStr, success: &str) {
        match std::process::Command::new(open_command()).arg(arg).spawn() {
            Ok(_) => self.flash = Some(success.to_string()),
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
    fn form_validates_required_fields() {
        let mut form = AppForm::new_empty("2026-05-28".into());
        assert!(form.validate().is_err(), "empty company must fail");
        form.fields[0].value = "Acme".into();
        assert!(form.validate().is_err(), "empty position must fail");
        form.fields[1].value = "Engineer".into();
        assert!(form.validate().is_ok(), "minimum valid form must pass");
    }

    #[test]
    fn form_rejects_unknown_status() {
        let mut form = AppForm::new_empty("2026-05-28".into());
        form.fields[0].value = "Acme".into();
        form.fields[1].value = "Engineer".into();
        // Status field is index 9.
        form.fields[9].value = "not-a-status".into();
        assert!(form.validate().is_err());
    }

    #[test]
    fn form_rejects_malformed_dates() {
        let mut form = AppForm::new_empty("not-a-date".into());
        form.fields[0].value = "Acme".into();
        form.fields[1].value = "Engineer".into();
        assert!(form.validate().is_err(), "malformed applied date must fail");
    }

    #[test]
    fn resolve_folder_joins_relative_to_data_dir() {
        let data = PathBuf::from("/home/me/jobs/applications.json");
        let resolved = resolve_folder_path(&data, "acme/intern");
        assert_eq!(resolved, PathBuf::from("/home/me/jobs/acme/intern"));
    }

    // ── Helpers for stateful App tests ─────────────────────────────────────

    use crate::data::Meta;
    use tempfile::NamedTempFile;

    fn sample_app(id: u32, company: &str, status: &str) -> Application {
        Application {
            id,
            company: company.into(),
            position: "Engineer".into(),
            location: None,
            app_type: None,
            reference: None,
            url: None,
            applied_date: Some("2026-05-01".into()),
            deadline: None,
            folder: None,
            status: status.into(),
            contacts: vec![],
            notes: vec![],
            next_action: None,
            next_action_date: None,
        }
    }

    /// Build an App backed by a tempfile, seeded with two entries.
    /// Returns the App and the tempfile (the tempfile must be kept alive
    /// for the duration of the test or it gets deleted).
    fn make_app() -> (App, NamedTempFile) {
        let tracker = Tracker {
            applications: vec![
                sample_app(1, "Acme", "applied"),
                sample_app(2, "Beta", "interview"),
            ],
            meta: Meta {
                next_id: 3,
                version: data::CURRENT_SCHEMA_VERSION.into(),
            },
        };
        let file = NamedTempFile::new().expect("tempfile");
        data::save(file.path(), &tracker).expect("seed save");
        let loaded = data::load(file.path()).expect("seed load");
        let mut app = App::new(loaded, file.path().to_path_buf());
        // Default filter is Active — make tests deterministic with everything visible.
        app.filter = Filter::All;
        (app, file)
    }

    fn fill_form_minimum(app: &mut App, company: &str, position: &str) {
        // Helper: type into a fresh form by setting fields directly.
        if let Mode::Form(f) = &mut app.mode {
            f.fields[0].value = company.into();
            f.fields[1].value = position.into();
        }
    }

    // ── Migration tests ────────────────────────────────────────────────────

    #[test]
    fn load_upgrades_old_version_in_memory() {
        let file = NamedTempFile::new().unwrap();
        // Pre-v2 file with literal "salary" key and version "1.0".
        let raw = r#"{
            "applications": [
                {"id": 1, "company": "Acme", "position": "Eng",
                 "salary": null, "status": "applied"}
            ],
            "_meta": {"next_id": 2, "version": "1.0"}
        }"#;
        std::fs::write(file.path(), raw).unwrap();
        let t = data::load(file.path()).unwrap();
        assert_eq!(t.meta.version, "2", "version must be bumped in memory");
        assert_eq!(t.applications.len(), 1);
    }

    #[test]
    fn salary_field_is_dropped_after_save() {
        let file = NamedTempFile::new().unwrap();
        let raw = r#"{
            "applications": [
                {"id": 1, "company": "Acme", "position": "Eng",
                 "salary": {"min": 50000}, "status": "applied"}
            ],
            "_meta": {"next_id": 2, "version": "1.0"}
        }"#;
        std::fs::write(file.path(), raw).unwrap();
        let t = data::load(file.path()).unwrap();
        data::save(file.path(), &t).unwrap();
        let saved = std::fs::read_to_string(file.path()).unwrap();
        assert!(!saved.contains("salary"), "salary key must be gone");
        assert!(saved.contains("\"version\": \"2\""));
    }

    // ── Add-form tests ─────────────────────────────────────────────────────

    #[test]
    fn add_form_opens_with_twelve_fields() {
        let (mut app, _f) = make_app();
        app.open_add_form();
        match &app.mode {
            Mode::Form(f) => {
                assert_eq!(f.fields.len(), 12);
                assert_eq!(f.focus, 0);
                assert_eq!(f.edit_target_id, None);
            }
            _ => panic!("expected Form mode"),
        }
    }

    #[test]
    fn add_form_defaults_applied_to_today() {
        let (mut app, _f) = make_app();
        app.open_add_form();
        if let Mode::Form(form) = &app.mode {
            let applied = form.get(FieldKey::AppliedDate);
            assert_eq!(applied, data::today_str());
            assert_eq!(form.get(FieldKey::Status), "applied");
        } else {
            panic!("expected Form mode");
        }
    }

    #[test]
    fn add_form_creates_application_and_increments_next_id() {
        let (mut app, _f) = make_app();
        let next_id_before = app.tracker.meta.next_id;
        let len_before = app.tracker.applications.len();
        app.open_add_form();
        fill_form_minimum(&mut app, "NewCo", "Backend Dev");
        app.form_save();
        assert!(matches!(app.mode, Mode::Normal), "form must close on save");
        assert!(app.error.is_none(), "no error expected: {:?}", app.error);
        assert_eq!(app.tracker.applications.len(), len_before + 1);
        assert_eq!(app.tracker.meta.next_id, next_id_before + 1);
        let added = app.tracker.applications.last().unwrap();
        assert_eq!(added.id, next_id_before);
        assert_eq!(added.company, "NewCo");
        assert_eq!(added.position, "Backend Dev");
        assert_eq!(added.status, "applied");
    }

    #[test]
    fn add_form_save_persists_to_disk() {
        let (mut app, file) = make_app();
        app.open_add_form();
        fill_form_minimum(&mut app, "Persisted", "QA");
        app.form_save();
        // Reload from disk and check it's there.
        let reloaded = data::load(file.path()).unwrap();
        assert!(
            reloaded
                .applications
                .iter()
                .any(|a| a.company == "Persisted")
        );
    }

    #[test]
    fn add_form_rejects_invalid_and_keeps_mode_open() {
        let (mut app, _f) = make_app();
        app.open_add_form();
        // Leave company empty.
        if let Mode::Form(f) = &mut app.mode {
            f.fields[1].value = "Engineer".into();
        }
        app.form_save();
        match &app.mode {
            Mode::Form(f) => assert!(f.error.is_some(), "validation error must be set"),
            _ => panic!("form should remain open on validation failure"),
        }
    }

    #[test]
    fn add_form_cancel_returns_to_normal_without_mutating() {
        let (mut app, _f) = make_app();
        let snapshot_len = app.tracker.applications.len();
        app.open_add_form();
        fill_form_minimum(&mut app, "Ghost", "Will be cancelled");
        app.form_cancel();
        assert!(matches!(app.mode, Mode::Normal));
        assert_eq!(app.tracker.applications.len(), snapshot_len);
    }

    // ── Edit-form tests ────────────────────────────────────────────────────

    #[test]
    fn edit_form_prefills_from_selection() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_edit_form();
        match &app.mode {
            Mode::Form(form) => {
                assert_eq!(form.edit_target_id, Some(1));
                assert_eq!(form.get(FieldKey::Company), "Acme");
                assert_eq!(form.get(FieldKey::Position), "Engineer");
                assert_eq!(form.get(FieldKey::Status), "applied");
            }
            _ => panic!("expected Form mode"),
        }
    }

    #[test]
    fn edit_form_updates_existing_entry_and_keeps_id_and_notes() {
        let (mut app, _f) = make_app();
        // Add a note so we can check it survives the edit.
        app.tracker.applications[0].notes.push(Note {
            date: "2026-01-01".into(),
            text: "keep me".into(),
        });
        app.selected = 0;
        app.open_edit_form();
        if let Mode::Form(f) = &mut app.mode {
            f.fields[0].value = "Acme Renamed".into();
        }
        app.form_save();
        let updated = &app.tracker.applications[0];
        assert_eq!(updated.id, 1, "id must not change");
        assert_eq!(updated.company, "Acme Renamed");
        assert_eq!(updated.notes.len(), 1, "notes must be preserved");
        assert_eq!(updated.notes[0].text, "keep me");
    }

    #[test]
    fn edit_form_does_not_increment_next_id() {
        let (mut app, _f) = make_app();
        let next_id_before = app.tracker.meta.next_id;
        app.selected = 0;
        app.open_edit_form();
        if let Mode::Form(f) = &mut app.mode {
            f.fields[0].value = "Whatever".into();
        }
        app.form_save();
        assert_eq!(app.tracker.meta.next_id, next_id_before);
    }

    // ── Delete tests ───────────────────────────────────────────────────────

    #[test]
    fn delete_removes_entry() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_delete_confirm();
        assert!(matches!(app.mode, Mode::ConfirmDelete { id: 1, .. }));
        app.delete_confirm();
        assert!(matches!(app.mode, Mode::Normal));
        assert_eq!(app.tracker.applications.len(), 1);
        assert_eq!(app.tracker.applications[0].id, 2);
    }

    #[test]
    fn delete_clamps_selected_when_last_entry_removed() {
        let (mut app, _f) = make_app();
        app.selected = 1; // points at "Beta"
        app.open_delete_confirm();
        app.delete_confirm();
        assert_eq!(app.tracker.applications.len(), 1);
        assert_eq!(app.selected, 0, "selected must clamp to new last index");
    }

    #[test]
    fn delete_cancel_keeps_entry() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_delete_confirm();
        app.delete_cancel();
        assert!(matches!(app.mode, Mode::Normal));
        assert_eq!(app.tracker.applications.len(), 2);
    }

    // ── Undo tests ─────────────────────────────────────────────────────────

    #[test]
    fn undo_on_empty_history_flashes() {
        let (mut app, _f) = make_app();
        app.undo();
        assert_eq!(app.flash.as_deref(), Some("nothing to undo"));
    }

    #[test]
    fn undo_restores_after_add() {
        let (mut app, _f) = make_app();
        let len_before = app.tracker.applications.len();
        let next_id_before = app.tracker.meta.next_id;
        app.open_add_form();
        fill_form_minimum(&mut app, "Will Disappear", "X");
        app.form_save();
        assert_eq!(app.tracker.applications.len(), len_before + 1);
        app.undo();
        assert_eq!(app.tracker.applications.len(), len_before);
        assert_eq!(app.tracker.meta.next_id, next_id_before);
    }

    #[test]
    fn undo_restores_after_delete() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_delete_confirm();
        app.delete_confirm();
        assert_eq!(app.tracker.applications.len(), 1);
        app.undo();
        assert_eq!(app.tracker.applications.len(), 2);
        assert_eq!(app.tracker.applications[0].id, 1);
    }

    #[test]
    fn undo_restores_after_status_change() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_status_picker();
        // Move to "interview" (index 2 in STATUSES).
        if let Mode::StatusPicker { idx } = &mut app.mode {
            *idx = 2;
        }
        app.status_picker_confirm();
        assert_eq!(app.tracker.applications[0].status, "interview");
        app.undo();
        assert_eq!(app.tracker.applications[0].status, "applied");
    }

    #[test]
    fn undo_restores_after_note_add() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_note_input();
        for c in "hello".chars() {
            app.note_push(c);
        }
        app.note_confirm();
        assert_eq!(app.tracker.applications[0].notes.len(), 1);
        app.undo();
        assert_eq!(app.tracker.applications[0].notes.len(), 0);
    }

    #[test]
    fn history_caps_at_max_history() {
        let (mut app, _f) = make_app();
        // Perform 15 status changes.
        for _ in 0..15 {
            app.selected = 0;
            app.open_status_picker();
            if let Mode::StatusPicker { idx } = &mut app.mode {
                *idx = 2;
            }
            app.status_picker_confirm();
            // Flip back so the next change is meaningful.
            app.open_status_picker();
            if let Mode::StatusPicker { idx } = &mut app.mode {
                *idx = 0;
            }
            app.status_picker_confirm();
        }
        // Undo until exhausted; the cap is MAX_HISTORY, so at most MAX_HISTORY undos
        // produce real work and the next one says "nothing to undo".
        let mut undos = 0;
        loop {
            app.flash = None;
            app.undo();
            if app.flash.as_deref() == Some("nothing to undo") {
                break;
            }
            undos += 1;
            assert!(undos <= MAX_HISTORY, "undo went past MAX_HISTORY");
        }
        assert_eq!(undos, MAX_HISTORY, "history should cap at MAX_HISTORY");
    }

    // ── Contact-input tests ────────────────────────────────────────────────

    #[test]
    fn contact_confirm_appends_with_today() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_contact_input();
        for c in "Alice".chars() {
            app.contact_push(c);
        }
        app.contact_confirm();
        assert!(matches!(app.mode, Mode::Normal));
        let contacts = &app.tracker.applications[0].contacts;
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].info, "Alice");
        assert_eq!(contacts[0].date, data::today_str());
    }

    #[test]
    fn contact_confirm_empty_is_noop() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_contact_input();
        app.contact_confirm();
        assert!(app.tracker.applications[0].contacts.is_empty());
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn contact_cancel_drops_buffer() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_contact_input();
        for c in "Bob".chars() {
            app.contact_push(c);
        }
        app.contact_cancel();
        assert!(app.tracker.applications[0].contacts.is_empty());
    }

    // ── URL/folder pre-conditions ──────────────────────────────────────────
    // We do not actually spawn xdg-open in tests. We only verify the
    // early-exit branches that set `flash` / `error` without spawning.

    #[test]
    fn open_url_without_selection_flashes() {
        let (mut app, _f) = make_app();
        app.tracker.applications.clear();
        app.open_selected_url();
        assert_eq!(app.flash.as_deref(), Some("no selection"));
    }

    #[test]
    fn open_url_without_url_field_flashes() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        // entry has no url
        app.open_selected_url();
        assert_eq!(app.flash.as_deref(), Some("no url set on this entry"));
    }

    #[test]
    fn open_folder_without_folder_field_flashes() {
        let (mut app, _f) = make_app();
        app.selected = 0;
        app.open_selected_folder();
        assert_eq!(app.flash.as_deref(), Some("no folder set on this entry"));
    }

    // ── Form focus navigation ──────────────────────────────────────────────

    #[test]
    fn form_focus_wraps_around() {
        let (mut app, _f) = make_app();
        app.open_add_form();
        for _ in 0..12 {
            app.form_focus_next();
        }
        if let Mode::Form(f) = &app.mode {
            assert_eq!(f.focus, 0, "12 nexts from 0 must wrap back to 0");
        }
        app.form_focus_prev();
        if let Mode::Form(f) = &app.mode {
            assert_eq!(f.focus, 11, "prev from 0 must wrap to last");
        }
    }

    #[test]
    fn form_push_pop_edits_focused_field() {
        let (mut app, _f) = make_app();
        app.open_add_form();
        // focus is on Company (index 0)
        app.form_push('A');
        app.form_push('B');
        app.form_push('C');
        if let Mode::Form(f) = &app.mode {
            assert_eq!(f.fields[0].value, "ABC");
        }
        app.form_pop();
        if let Mode::Form(f) = &app.mode {
            assert_eq!(f.fields[0].value, "AB");
        }
    }
}
