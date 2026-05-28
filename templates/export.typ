// Questa job-tracker export template.
//
// This file is loaded by `questa` at export time alongside a sibling
// `<stem>.json` data file. To re-render the PDF after edits, run:
//
//     typst compile <stem>.typ
//
// You may tweak colours, page size, fonts, etc. without touching the
// `.json`; the data and the presentation are deliberately split.

#let data = json("__DATA_JSON__")

// ── Page ───────────────────────────────────────────────────────────────────

#set page(
  paper: "a4",
  margin: (x: 2cm, y: 2.2cm),
  numbering: "1 / 1",
  header: align(right, text(size: 8pt, fill: gray)[
    Questa — exported #data.exported_at
  ]),
)

#set text(size: 10pt)
#set par(justify: true)
#set heading(numbering: none)

// ── Status colours (mirror the TUI palette) ────────────────────────────────

#let status-color = (
  applied: rgb("#4f8ef7"),
  screening: rgb("#26c6da"),
  interview: rgb("#ffa726"),
  technical: rgb("#ab47bc"),
  offer: rgb("#66bb6a"),
  accepted: rgb("#43a047"),
  rejected: rgb("#ef5350"),
  withdrawn: rgb("#546e7a"),
  ghosted: rgb("#3d4a52"),
)

#let status-badge(s) = box(
  fill: status-color.at(s, default: gray),
  inset: (x: 6pt, y: 1.5pt),
  radius: 3pt,
  text(fill: white, size: 8pt, weight: "bold", s),
)

#let kv-row(label, value) = (
  text(fill: gray, weight: "bold", label),
  value,
)

#let optional(s) = if s == none or s == "" { none } else { s }

// ── Cover ──────────────────────────────────────────────────────────────────

#align(center)[
  #text(size: 26pt, weight: "bold")[Job application tracker]

  #v(0.4em)

  #text(size: 11pt, fill: gray)[
    #data.exported_at · filter: *#data.filter* · #data.applications.len() entries
  ]
]

#v(0.6cm)

// ── Stats ──────────────────────────────────────────────────────────────────

#let stat(label, value, colour) = box(
  inset: 10pt,
  radius: 4pt,
  fill: colour.lighten(85%),
  stack(
    dir: ttb,
    spacing: 4pt,
    text(size: 16pt, weight: "bold", fill: colour, str(value)),
    text(size: 9pt, fill: gray, label),
  ),
)

#align(center)[
  #stack(
    dir: ltr,
    spacing: 14pt,
    stat("total", data.stats.total, rgb("#4f8ef7")),
    stat("active", data.stats.active, rgb("#4f8ef7")),
    stat("interview", data.stats.interview, rgb("#ffa726")),
    stat("rejected", data.stats.rejected, rgb("#ef5350")),
    stat("ghosted", data.stats.ghosted, rgb("#3d4a52")),
  )
]

#v(1cm)

// ── Summary table ──────────────────────────────────────────────────────────

== Summary

#table(
  columns: (auto, 2fr, 3fr, auto, auto, 2fr),
  inset: (x: 6pt, y: 5pt),
  align: (right, left, left, center, left, left),
  stroke: 0.4pt + rgb("#dddddd"),
  table.header(
    text(weight: "bold", "ID"),
    text(weight: "bold", "Company"),
    text(weight: "bold", "Position"),
    text(weight: "bold", "Status"),
    text(weight: "bold", "Applied"),
    text(weight: "bold", "Next action"),
  ),
  ..data.applications
    .map(a => (
      text(fill: gray, "#" + str(a.id)),
      a.company,
      a.position,
      status-badge(a.status),
      a.applied_date,
      a.next_action,
    ))
    .flatten()
)

#pagebreak()

// ── Detail cards ───────────────────────────────────────────────────────────

== Details

#for (i, app) in data.applications.enumerate() [
  #block(breakable: false)[
    === #app.company — #app.position

    #grid(
      columns: (auto, 1fr),
      column-gutter: 1em,
      row-gutter: 0.4em,
      ..kv-row([Status], status-badge(app.status)),
      ..kv-row([ID], [\##app.id]),
      ..if app.location != "" { kv-row([Location], app.location) } else { () },
      ..if app.app_type != "" { kv-row([Type], app.app_type) } else { () },
      ..if app.reference != "" { kv-row([Ref], app.reference) } else { () },
      ..if app.applied_date != "" { kv-row([Applied], app.applied_date) } else { () },
      ..if app.deadline != "" { kv-row([Deadline], app.deadline) } else { () },
      ..if app.next_action != "" {
        kv-row([Next action], [
          #app.next_action
          #if app.next_action_date != "" [
            #h(0.5em) #text(fill: gray, "(" + app.next_action_date + ")")
          ]
        ])
      } else { () },
      ..if app.url != "" { kv-row([URL], link(app.url, app.url)) } else { () },
      ..if app.folder != "" { kv-row([Folder], raw(app.folder)) } else { () },
    )

    #if app.contacts.len() > 0 [
      #v(0.3em)
      #text(weight: "bold", fill: gray, "Contacts")
      #list(
        ..app.contacts.map(c => [
          #text(fill: gray, c.date) — #c.info
        ])
      )
    ]

    #if app.notes.len() > 0 [
      #v(0.3em)
      #text(weight: "bold", fill: gray, "Notes")
      #for n in app.notes [
        #text(size: 9pt, fill: gray, n.date) \
        #n.text \
      ]
    ]
  ]

  #v(0.7cm)
]
