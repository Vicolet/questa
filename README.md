<div align="center">

# questa

**A keyboard-driven TUI for tracking your job quest.**

[![Crates.io](https://img.shields.io/crates/v/questa.svg?logo=rust)](https://crates.io/crates/questa)
[![CI](https://github.com/Vicolet/questa/actions/workflows/ci.yml/badge.svg)](https://github.com/Vicolet/questa/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/questa.svg)](https://crates.io/crates/questa)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/crates/l/questa.svg)](LICENSE)

`questa` is a small, focused terminal app that turns a single `applications.json` file into a fast, navigable dashboard for your job search. Filter, sort, search, edit status, and add notes without ever leaving the keyboard.

</div>

---

## Why

Spreadsheets get cluttered. Notion is overkill. A plain JSON file plus a fast TUI is the right size for tracking job applications: the data stays portable and version-controllable, mutations are explicit, and you can grep it from any other tool.

`questa` is intentionally small. It does not parse job postings, send emails, or scrape boards. It shows you what is in your file and lets you update status and notes from a clean keyboard interface. That is the whole product.

## Features

- **Fast** — single static binary, sub-50 ms startup.
- **Keyboard-driven** — vim-style navigation, no mouse required.
- **Full CRUD in-app** — add (`a`), edit (`e`), delete (`d` with confirm), undo (`u`, last 10 mutations). No need to hand-edit JSON.
- **Filter** — instantly switch between *all*, *active*, *interview*, *rejected*, *ghosted*.
- **Sort** — by date desc, by status priority (interviews first), or by company.
- **Fuzzy search** — match across company name and position title.
- **Cursor-aware text editing** — readline-style keys (`←`/`→`, `Home`/`End`, `^A`/`^E`, `^W`, `^U`, word jumps), Unicode-correct.
- **Visible signals** — overdue follow-ups marked with `⚠`, upcoming actions counted in the header.
- **PDF export** — press `x` to render the current view through Typst (`.typ` + `.json` + `.pdf`).
- **Atomic saves** — writes go through `tmp + fsync + rename`; a crash never truncates your data.
- **Portable data** — your applications live in a plain JSON file. No database, no cloud.
- **Observable** — structured logs at `$XDG_STATE_HOME/questa/questa.log` (`RUST_LOG`-tunable).

## Demo

![questa in action](docs/screenshots/demo.gif)

The recording walks through filtering, sorting, fuzzy search, the status picker, adding a note, and the help overlay. The bundled dataset under [`examples/applications.json`](examples/applications.json) is what you see.

> The session was recorded with [VHS](https://github.com/charmbracelet/vhs). The script lives in [`docs/demo.tape`](docs/demo.tape) and can be regenerated with `vhs docs/demo.tape`.

## Install

```bash
cargo install questa
```

That is it. The `questa` binary is now on your `$PATH`. Run it from any directory containing an `applications.json`, or point it at one explicitly:

```bash
questa --data ~/jobs/applications.json
```

Requires Rust 1.85+ ([install via rustup](https://www.rust-lang.org/tools/install)).

### Pre-built binaries

Each release attaches pre-built binaries for Linux, macOS (Intel and Apple Silicon), and Windows on the [releases page](https://github.com/Vicolet/questa/releases/latest). Download, extract, and drop the binary somewhere on your `$PATH`. No Rust toolchain required.

### Try the demo without installing

```bash
git clone https://github.com/Vicolet/questa.git
cd questa
cargo run --release
```

The bundled demo dataset under `examples/applications.json` is loaded automatically.

### Build from source

```bash
git clone https://github.com/Vicolet/questa.git
cd questa
cargo install --path .
```

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
| `a` | add a new application (multi-field form)                     |
| `e` | edit the selected application (same form, prefilled)         |
| `d` | delete the selected application (confirm with `y` / `n`)     |
| `u` | undo the last mutation (history of 10)                       |
| `s` | change status (picker overlay, `j`/`k` to select, `enter`)   |
| `n` | add a note (text field, dated today on save)                 |
| `c` | add a contact (text field, dated today on save)              |
| `O` | open the selected application's `folder` in the system file manager |
| `U` | open the selected application's `url` in the default browser |
| `x` | export the current view to PDF (writes `.typ` + `.json` + `.pdf`)  |

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
  "_meta": { "next_id": 2, "version": "2" }
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

## Logs

Questa writes a structured log file to
`$XDG_STATE_HOME/questa/questa.log` (or
`~/.local/state/questa/questa.log` as a fallback). Save events,
mutations, undo, and export results all land here. This is the place
to look when something behaved unexpectedly.

Level defaults to `info`; raise it via the standard `RUST_LOG` knob:

```bash
RUST_LOG=questa=debug questa
```

## PDF export

Press `x` to export the currently filtered applications. Questa writes
three files into a sibling `exports/` directory next to your data file:

```
<data-dir>/exports/
├── questa-YYYYMMDD-HHMMSS.json   # the data this export was built from
├── questa-YYYYMMDD-HHMMSS.typ    # the Typst template, references the json
└── questa-YYYYMMDD-HHMMSS.pdf    # produced if `typst` is on PATH
```

The PDF holds a cover page with stats, a summary table, and one detail
card per application. If `typst` is not installed the `.typ` and `.json`
are still written and you get a flash message telling you to run
`typst compile <file>.typ` yourself. Install Typst from
<https://typst.app> or `cargo install typst-cli`.

The template can be hand-edited — colours, fonts, page size, layout —
and re-compiled without touching questa. Data and presentation are kept
in separate files on purpose.

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
├── ui.rs      # ratatui rendering: header, table, detail, overlays
├── export.rs  # data + template generation for the PDF export
├── text.rs    # TextBuf + TextAction (cursor-aware single-line input)
└── logging.rs # tracing subscriber wired to the XDG state directory

templates/
└── export.typ  # Typst document, baked into the binary via include_str!
```

## Roadmap

- Sort by deadline and by next-action date
- Per-note delete / edit from the TUI (currently only append + view)
- Scroll the detail panel to see notes beyond the last 8
- Configurable colour theme via `$XDG_CONFIG_HOME/questa/theme.toml`
- Optional JSON-schema validation on load
- Reusable Typst templates under `$XDG_CONFIG_HOME/questa/templates/` so
  users can override the bundled export without forking

Contributions and feature suggestions welcome via GitHub issues.

## Acknowledgements

Built with [ratatui](https://github.com/ratatui/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm). Inspired by [gitui](https://github.com/extrawurst/gitui) and [yazi](https://github.com/sxyazi/yazi).

## License

[MIT](LICENSE)
