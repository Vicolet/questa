# Data schema

`applications.json` is the single source of truth. This document lists every
field, what it is for, where it shows up in the UI, and how the user can edit
it. Compatibility rules and migration history live at the bottom.

The top-level shape is:

```json
{
  "applications": [ Application, ... ],
  "_meta": { "next_id": 16, "version": "2" }
}
```

## Field reference

Legend for **Edit**:

- `auto` — set by the app, never typed by the user
- `key:X` — direct keyboard shortcut in the TUI
- `form` — editable through the add (`a`) / edit (`e`) form

| Field              | Type            | Required | Shown in detail            | Edit             |
|--------------------|-----------------|----------|----------------------------|------------------|
| `id`               | `u32`           | yes      | yes (`#id`)                | `auto` from `_meta.next_id` |
| `company`          | string          | yes      | yes                        | `form`           |
| `position`         | string          | yes      | yes                        | `form`           |
| `location`         | string?         | no       | yes                        | `form`           |
| `type`             | string?         | no       | yes                        | `form`           |
| `ref`              | string?         | no       | yes (if non-empty)         | `form`           |
| `url`              | string?         | no       | yes (if non-empty)         | `form` · `key:U` to open in browser |
| `applied_date`     | `YYYY-MM-DD`?   | no       | yes (`+ relative`)         | `form` (defaults to today on add) |
| `deadline`         | `YYYY-MM-DD`?   | no       | yes                        | `form`           |
| `folder`           | string?         | no       | yes                        | `form` · `key:O` opens in file manager |
| `status`           | string (enum)   | yes      | yes                        | `key:s` (picker) or `form` |
| `contacts`         | `[Contact]`     | no       | yes (all)                  | `key:c` appends a contact dated today |
| `notes`            | `[Note]`        | no       | yes (last 8, reverse)      | `key:n` appends a note dated today |
| `next_action`      | string?         | no       | yes                        | `form`           |
| `next_action_date` | `YYYY-MM-DD`?   | no       | yes (`+ overdue/3d/today`) | `form`           |

Every field is editable from the TUI — there is no need to hand-edit
`applications.json`. The form validates required fields (`company`,
`position`), the `status` enum, and `YYYY-MM-DD` shape on all date fields.

## Status enum

`status` is a string but only nine values are recognised. Anything else falls
back to a default colour and a sort priority of `99` (i.e. last).

| Value | Group (filters) | Sort priority | Meaning |
|---|---|---|---|
| `applied` | active | 4 | Application submitted, no reply yet |
| `screening` | active | 3 | Recruiter screen scheduled or in progress |
| `interview` | active, interview | 0 | Behavioural / hiring-manager round |
| `technical` | active, interview | 1 | Technical round (live coding, take-home, …) |
| `offer` | active | 2 | Offer received, deciding |
| `accepted` | — | 5 | Offer accepted |
| `rejected` | rejected | 7 | Rejected by the company |
| `withdrawn` | — | 6 | User withdrew |
| `ghosted` | ghosted | 8 | No reply after follow-ups |

## Nested types

```rust
Contact { date: YYYY-MM-DD, info: String }
Note    { date: YYYY-MM-DD, text: String }
```

Dates are stored as ISO `YYYY-MM-DD` strings. A value that does not parse is
displayed verbatim but is treated as "no date" by the relative-date and
overdue helpers.

## Persistence guarantees

- **Atomic writes.** Saves go to a sibling `.tmp` file, are fsynced, then
  renamed into place. A crash mid-write never leaves `applications.json`
  truncated.
- **Undo history.** Every mutation pushes the pre-mutation tracker onto an
  in-memory stack capped at 10. `u` pops the most recent snapshot and writes
  the previous state back.

## Migrations

`_meta.version` records the on-disk schema version. The current version is
`"2"`. On load, questa reads the version, applies any migrations in order,
and writes back the latest version on the next save.

| From → To | Change |
|---|---|
| `"1.0"` → `"2"` | Drop the unused `salary` field. Serde silently ignores the key on load; the next save omits it. |

When old files are encountered, the upgrade is silent and lossless within
the constraints of the schema change.

## Compatibility rules

- **Adding an optional field** is non-breaking — keep `#[serde(default)]`.
- **Renaming or removing a field** requires a `_meta.version` bump and a
  migration that runs on load.
- **Changing the type** of an existing field requires a version bump and a
  migration.
- Hand-edited JSON should remain valid: prefer adding fields with sane
  defaults over making them required.

## Known limitations

- The detail panel renders only the last 8 notes (most recent first); older
  notes are not visible from the TUI. They are preserved in the JSON.
- There is no in-app way to delete an individual note or contact. Hand-edit
  the JSON for now.
