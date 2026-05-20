# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
