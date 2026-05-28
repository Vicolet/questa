# Data schema

`applications.json` is the single source of truth. This document lists every
field, what it is for, where it shows up in the UI, and how the user can edit
it today. It also flags gaps that the application should close.

The top-level shape is:

```json
{
  "applications": [ Application, ... ],
  "_meta": { "next_id": 16, "version": "1.0" }
}
```

## Field audit

Legend for **Edit**:
- `auto` — set by the app, never typed by the user
- `key:X` — current keyboard shortcut in the TUI
- `JSON` — only editable by hand-editing the file
- `form` — planned: editable through the add/edit form (not yet implemented)

| Field | Type | Required | Shown in detail | Edit today | Edit target |
|---|---|---|---|---|---|
| `id` | `u32` | yes | yes (`#id`) | auto | auto (from `_meta.next_id`) |
| `company` | string | yes | yes | JSON | `form` |
| `position` | string | yes | yes | JSON | `form` |
| `location` | string? | no | yes | JSON | `form` |
| `type` | string? | no | yes | JSON | `form` |
| `ref` | string? | no | yes (if non-empty) | JSON | `form` |
| `url` | string? | no | **no** | JSON | `form` + `key:U` to open in browser |
| `applied_date` | `YYYY-MM-DD`? | no | yes (`+ relative`) | JSON | `form` (defaults to today on add) |
| `deadline` | `YYYY-MM-DD`? | no | yes | JSON | `form` |
| `folder` | string? | no | yes | JSON (`key:O` opens it) | `form` |
| `status` | string (enum) | yes | yes | `key:s` | `key:s` (kept) |
| `contacts` | `[Contact]` | no | yes (all) | JSON | `key:c` to append |
| `notes` | `[Note]` | no | yes (last 8, reverse) | `key:n` to append | `key:n` (kept) + scroll for older notes |
| `next_action` | string? | no | yes | JSON | `form` |
| `next_action_date` | `YYYY-MM-DD`? | no | yes (`+ overdue/3d/today`) | JSON | `form` |

### Status enum

`status` is a string but only nine values are recognised. Anything else falls
back to a default colour and a sort priority of `99` (i.e. last).

| Value | Group (filters) | Sort priority | Meaning |
|---|---|---|---|
| `applied` | active | 4 | Application submitted, no reply yet |
| `screening` | active | 3 | Recruiter screen scheduled or in progress |
| `interview` | active, interview | 0 | Behavioural / hiring-manager round |
| `technical` | active, interview | 1 | Technical round (live coding, take-home, ...) |
| `offer` | active | 2 | Offer received, deciding |
| `accepted` | — | 5 | Offer accepted |
| `rejected` | rejected | 7 | Rejected by the company |
| `withdrawn` | — | 6 | User withdrew |
| `ghosted` | ghosted | 8 | No reply after follow-ups |

### Nested types

```rust
Contact { date: YYYY-MM-DD, info: String }
Note    { date: YYYY-MM-DD, text: String }
```

Dates are stored as ISO `YYYY-MM-DD` strings. Anything that does not parse is
displayed verbatim but treated as "no date" by the relative-date and overdue
helpers.

## Gaps and decisions

This is the honest list of what is broken or inconsistent today. The friend's
feedback that "not all fields seem to be used" maps to the rows below.

### 1. `salary` is dead — drop it

- Declared as `Option<serde_json::Value>` (accepts anything).
- Never displayed in the UI.
- Never editable.
- Zero out of 15 entries in `examples/applications.json` set a non-null value.

**Decision: remove from the schema.** Bump `_meta.version` to `2` and add a
load-time migration that drops the field if present (silently, no error).

### 2. `url` is dead in the UI — surface it

The field is parsed and stored, but the detail panel never shows it and there
is no way to open it. **Decision: show it under "URL" and add `key:U` to open
in the default browser (`xdg-open` / `open` / `explorer`).**

### 3. Most fields cannot be edited from the TUI

This is the root cause of the friend's confusion. The TUI shows fields the
user never typed — because the only way to populate them is to edit the JSON
by hand. **Decision: implement an add/edit form (`key:a` / `key:e`) that
covers every editable field listed above.**

### 4. `contacts` is read-only in the UI

Notes can be appended (`key:n`) but contacts cannot. **Decision: add `key:c`
to append a contact (same pattern as `key:n`).**

### 5. Notes truncate silently after 8

The detail panel renders the last 8 notes in reverse chronological order with
no indicator that more exist. **Decision: when more than 8 notes exist, show
`... (N more)` and add a way to view the full history (scroll the detail
panel, or a dedicated overlay).**

### 6. `_meta.version` is parsed but never checked

There is no migration path today. If we change the schema, old files will
silently lose data or fail to parse. **Decision: read `version` at load,
apply migrations in order, write back with the latest version.**

### 7. `_meta.next_id` is parsed but never incremented

It exists in anticipation of "add application" but is unused. The future
`form` flow will read it, assign the id, increment it, and save.

## Compatibility rules

- **Adding an optional field** is non-breaking — keep `#[serde(default)]`.
- **Renaming or removing a field** requires a `_meta.version` bump and a
  migration that runs on load.
- **Changing the type** of an existing field requires a version bump and a
  migration.
- Hand-edited JSON should remain valid: prefer adding fields with sane
  defaults over making them required.
