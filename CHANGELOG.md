# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-05-28

### Added
- Add (`a`), edit (`e`), and delete (`d` with `y`/`n` confirm) applications from the TUI. No need to hand-edit `applications.json` for routine operations.
- Undo (`u`) restores the previous state for any of the last 10 mutations (status change, note, contact, add, edit, delete).
- Append a contact entry (`c`), dated today — symmetric to `n` for notes.
- Open the selected application's URL in the default browser (`U`). The `url` field is now shown in the detail panel when set.
- PDF export (`x`) via Typst: writes paired `.typ` + `.json` (and `.pdf` when `typst` is on `PATH`) into `<data-dir>/exports/`. The template lives in `templates/export.typ` and is baked into the binary via `include_str!`.
- Cursor-aware text editing in every input mode (form fields, note prompt, contact prompt): `←` / `→`, `Home` / `End` (or `^A` / `^E`), word jumps with `^←` / `^→` (or `alt-b` / `alt-f`), `^W` to delete the previous word, `^U` to clear the field, `Delete` for forward delete. A reversed-colour block caret marks the cursor position. Unicode-correct (characters, not bytes).
- Atomic `data::save`: write to a sibling temp file, fsync, then rename into place. A crash mid-write no longer truncates `applications.json`.
- Structured logging to `$XDG_STATE_HOME/questa/questa.log` via `tracing`. Save events, mutations, undo, and export results are recorded. Level is controlled by `RUST_LOG` (default `info`).
- Schema documentation in `docs/schema.md` listing every field and its UI/edit story.
- 63 tests (up from 6 at 0.2.0): TextBuf primitives, every CRUD entry point, every undo target, contact input, URL/folder pre-conditions, form validation, atomic-save invariants, export JSON/template, end-to-end PDF compile, plus 7 `insta` snapshots covering every UI overlay.
- CI: `cargo audit` job runs on every push and PR (non-blocking, surfaces advisories without gating merges).

### Changed
- Schema bumped to version `"2"`. The dead `salary` field is silently dropped on next save; older files are upgraded in memory on load.
- `_meta.next_id` is now read and incremented on add.

### Removed
- `salary` field (declared but never displayed, edited, or populated).

## [0.2.0] - 2026-05-21

### Added
- `O` opens the selected application's folder in the system file manager (`xdg-open` / `open` / `explorer`).
- Pre-built cross-platform binaries published on every release via GitHub Actions.
- Dependabot configuration for cargo and GitHub Actions updates.

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

[Unreleased]: https://github.com/Vicolet/questa/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Vicolet/questa/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Vicolet/questa/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Vicolet/questa/releases/tag/v0.1.0
