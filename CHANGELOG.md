# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- PDF export (`x`) via Typst: writes paired `.typ` + `.json` (and `.pdf` when `typst` is on PATH) into `<data-dir>/exports/`. The template lives in `templates/export.typ` and is baked into the binary via `include_str!`.
- Atomic `data::save`: write to a sibling temp file, fsync, then rename into place. A crash mid-write no longer truncates `applications.json`.
- Add (`a`), edit (`e`), and delete (`d` with `y`/`n` confirm) applications from the TUI.
- Undo (`u`) for the last 10 mutations (status change, note, contact, add, edit, delete).
- Add a contact entry from the TUI (`c`), dated today.
- Open the selected application's URL in the default browser (`U`).
- `URL` field is now shown in the detail panel when set.
- Schema documentation in `docs/schema.md` listing every field and its UI/edit story.

### Changed
- Schema bumped to version `2`. The dead `salary` field is silently dropped on next save.
- `_meta.next_id` is now actually used (incremented on add).

### Removed
- `salary` field (was declared but never displayed, edited, or populated).

## [0.1.0] - 2026-05-20

### Added
- Initial public release.
- TUI dashboard with header counters (total / active / interview / overdue / this week).
- List view with status colors, ID, company, position, and overdue marker.
- Detail panel showing status, dates, next action, folder, contacts, and notes.
- Filters: all, active, interview, rejected, ghosted (number keys 1-5, tab to cycle).
- Sort cycle: date descending, status priority, alphabetical by company (`o` key).
- Fuzzy search on company and position (`/` key).
- Status editor: popup picker over all valid statuses (`s` key).
- Note input: popup text field, saves a note dated today (`n` key).
- `--data <PATH>` flag to point at a custom `applications.json` location.
- Help overlay (`?` key).
- Bundled demo dataset in `examples/applications.json`.

[Unreleased]: https://github.com/Vicolet/questa/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Vicolet/questa/releases/tag/v0.1.0
