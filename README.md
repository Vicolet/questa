<div align="center">

# questa

**A keyboard-driven TUI for tracking your job quest.**

[![Build](https://img.shields.io/badge/build-cargo-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-brightgreen.svg)](CHANGELOG.md)

`questa` is a small, focused terminal app that turns a single `applications.json` file into a fast, navigable dashboard for your job search. Filter, sort, search, edit status, and add notes without ever leaving the keyboard.

</div>

---

## Why

Spreadsheets get cluttered. Notion is overkill. A plain JSON file plus a fast TUI is the right size for tracking job applications: the data stays portable and version-controllable, mutations are explicit, and you can grep it from any other tool.

`questa` is intentionally small. It does not parse job postings, send emails, or scrape boards. It shows you what is in your file and lets you update status and notes from a clean keyboard interface. That is the whole product.

## Features

- **Fast** — single static binary, sub-50 ms startup.
- **Keyboard-driven** — vim-style navigation, no mouse required.
- **Filter** — instantly switch between *all*, *active*, *interview*, *rejected*, *ghosted*.
- **Sort** — by date desc, by status priority (interviews first), or by company.
- **Fuzzy search** — match across company name and position title.
- **Edit in place** — change status from a picker, append notes dated today.
- **Visible signals** — overdue follow-ups marked with `⚠`, upcoming actions counted in the header.
- **Portable data** — your applications live in a plain JSON file. No database, no cloud.

## Demo

```text
┌─ Tracker │ Total 15 · Active 9 · Interview 1 · ⚠ Overdue 1 · 🔥 This week 3 │ sort: status │ 2026-05-20 ─┐
│ Active (9)                                                  │ Detail                                       │
│ ● #1   Acme Robotics       Junior Embedded Engineer  interv │ #1   Acme Robotics                           │
│   #7   Mistral Robotics    Junior CPS Engineer       offer  │ Junior Embedded Engineer                     │
│   #6   Quartz Insurance    GRC Analyst               scre…  │ Zurich, Switzerland                          │
│   #2   Northwind Bank      IT Security Analyst       scre…  │                                              │
│   #9   Cascade Cloud       Cloud Security Engineer   appli  │ Status     interview                         │
│   #11  Veridian Health     Information Security…     appli  │ Applied    2026-04-12  (38d ago)             │
│   #13  Cobalt Defence      AI Research Intern        appli  │ Next       Prepare technical interview ·     │
│   #3   Helios Labs         Machine Learning Engin…   appli  │            2026-05-21  (1d to go)            │
│                                                              │ Folder     acme-robotics/junior-embedded-…   │
│                                                              │                                              │
│                                                              │ Contacts                                     │
│                                                              │   2026-04-22 Anna Berger - Talent Acquisi…   │
│                                                              │   2026-05-02 Mark Steiner - Hiring Manager   │
│                                                              │                                              │
│                                                              │ Notes                                        │
│                                                              │   2026-05-02                                 │
│                                                              │     Passed recruiter screen. Technical       │
│                                                              │     interview booked May 22 at 14h, 60       │
│                                                              │     minutes, video.                          │
└─────────────────────────────────────────────────────────────┴──────────────────────────────────────────────┘
  j/k nav · 1-5 filter · o sort · / search · s status · n note · ? help · q quit
```

## Install

### From source

Requires Rust 1.85+ ([install via rustup](https://www.rust-lang.org/tools/install)).

```bash
git clone https://github.com/Vicolet/questa.git
cd questa
cargo install --path .
```

`questa` is then available on your `$PATH`.

### Try it without installing

```bash
git clone https://github.com/Vicolet/questa.git
cd questa
cargo run --release
```

The bundled demo dataset under `examples/applications.json` is loaded automatically.

## Usage

```bash
questa                       # auto-detect applications.json in CWD or examples/
questa --data path/to/file.json
questa -d ~/work/jobs.json
questa --help
questa --version
```

`questa` looks for `applications.json` in this order:

1. `--data <PATH>` argument
2. `applications.json` in the current directory
3. `examples/applications.json` (handy when running the repo directly)
4. `$XDG_DATA_HOME/questa/applications.json` (or `~/.local/share/questa/applications.json`)

## Key bindings

### Navigation

| Key       | Action          |
|-----------|-----------------|
| `j` / `↓` | move down       |
| `k` / `↑` | move up         |
| `g`       | jump to top     |
| `G`       | jump to bottom  |

### Filter

| Key   | Action                                              |
|-------|-----------------------------------------------------|
| `1`   | all                                                 |
| `2`   | active (applied / screening / interview / ...)      |
| `3`   | interview                                           |
| `4`   | rejected                                            |
| `5`   | ghosted                                             |
| `tab` | cycle through filters                               |

### Sort

| Key | Action                                          |
|-----|-------------------------------------------------|
| `o` | cycle: date desc → status priority → company    |

### Search

| Key     | Action                                  |
|---------|-----------------------------------------|
| `/`     | start fuzzy search (company + position) |
| `enter` | confirm                                 |
| `esc`   | cancel                                  |

### Edit

| Key | Action                                                       |
|-----|--------------------------------------------------------------|
| `s` | change status (picker overlay, `j`/`k` to select, `enter`)   |
| `n` | add a note (text field, dated today on save)                 |

Mutations write to `applications.json` immediately.

### General

| Key         | Action          |
|-------------|-----------------|
| `?`         | toggle help     |
| `q` / `esc` | quit            |

## Data format

`applications.json` is a single file with this shape:

```json
{
  "applications": [
    {
      "id": 1,
      "company": "Acme Robotics",
      "position": "Junior Embedded Engineer",
      "location": "Zurich, Switzerland",
      "type": "full-time",
      "ref": "ENG-2026-018",
      "url": "https://example.com/jobs/...",
      "applied_date": "2026-04-12",
      "deadline": null,
      "salary": null,
      "folder": "acme-robotics/junior-embedded-engineer",
      "status": "interview",
      "contacts": [
        { "date": "2026-04-22", "info": "Anna Berger - Talent Acquisition" }
      ],
      "notes": [
        { "date": "2026-04-12", "text": "Applied through the careers page." }
      ],
      "next_action": "Prepare technical interview",
      "next_action_date": "2026-05-21"
    }
  ],
  "_meta": { "next_id": 2, "version": "1.0" }
}
```

| Field              | Type            | Required | Description                                           |
|--------------------|-----------------|----------|-------------------------------------------------------|
| `id`               | integer         | yes      | Stable identifier, used for editing                   |
| `company`          | string          | yes      |                                                       |
| `position`         | string          | yes      |                                                       |
| `status`           | string          | yes      | See valid statuses below                              |
| `location`         | string          | no       |                                                       |
| `type`             | string          | no       | `internship`, `full-time`, `part-time`, `contract`    |
| `ref`              | string          | no       | Internal posting reference                            |
| `url`              | string          | no       |                                                       |
| `applied_date`     | string (ISO)    | no       | `YYYY-MM-DD`                                          |
| `deadline`         | string (ISO)    | no       |                                                       |
| `folder`           | string          | no       | Relative path to a folder of artefacts (CV, letter)   |
| `contacts`         | array of object | no       | `{ date, info }`                                      |
| `notes`            | array of object | no       | `{ date, text }`                                      |
| `next_action`      | string          | no       |                                                       |
| `next_action_date` | string (ISO)    | no       | Drives the overdue and this-week counters             |

### Valid statuses

`applied`, `screening`, `interview`, `technical`, `offer`, `accepted`, `rejected`, `withdrawn`, `ghosted`.

When sorting by status, the order is `interview > technical > offer > screening > applied > accepted > withdrawn > rejected > ghosted`.

## Development

```bash
cargo build              # debug build
cargo build --release    # optimised binary
cargo run --release      # build and run
cargo fmt
cargo clippy -- -D warnings
```

The codebase is split across four files:

```
src/
├── main.rs   # entry point, terminal lifecycle, key dispatch
├── data.rs   # serde structs + JSON load/save + date helpers
├── app.rs    # state machine: filter, sort, mode, mutations
└── ui.rs     # ratatui rendering: header, table, detail, overlays
```

## Roadmap

- `o` open folder of the selected application in `$FILE_MANAGER` / `xdg-open`
- `e` edit `next_action` and `next_action_date` from the TUI
- Sort by deadline and by next-action date
- Configurable colour theme via `~/.config/questa/theme.toml`
- Optional JSON schema validation on load

Contributions and feature suggestions welcome via GitHub issues.

## Acknowledgements

Built with [ratatui](https://github.com/ratatui/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm). Inspired by [gitui](https://github.com/extrawurst/gitui) and [yazi](https://github.com/sxyazi/yazi).

## License

[MIT](LICENSE)
