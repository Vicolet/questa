//! Rendering with ratatui.

use crate::app::{App, AppForm, Mode, STATUSES};
use crate::data;
use crate::text::TextBuf;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
        Wrap,
    },
};

// ── Status colours ────────────────────────────────────────────────────────

fn status_color(status: &str) -> Color {
    match status {
        "applied" => Color::Rgb(79, 142, 247),
        "screening" => Color::Rgb(38, 198, 218),
        "interview" => Color::Rgb(255, 167, 38),
        "technical" => Color::Rgb(171, 71, 188),
        "offer" => Color::Rgb(102, 187, 106),
        "accepted" => Color::Rgb(67, 160, 71),
        "rejected" => Color::Rgb(239, 83, 80),
        "withdrawn" => Color::Rgb(84, 110, 122),
        "ghosted" => Color::Rgb(61, 74, 82),
        _ => Color::Gray,
    }
}

const ACCENT: Color = Color::Rgb(79, 142, 247);

// ── Top-level layout ──────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(1),    // body
            Constraint::Length(1), // statusbar
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_body(f, app, chunks[1]);
    draw_statusbar(f, app, chunks[2]);

    match app.mode {
        Mode::Help => draw_help_overlay(f, f.area()),
        Mode::StatusPicker { idx } => draw_status_picker(f, f.area(), idx),
        Mode::NoteInput { ref buffer } => draw_text_input(f, f.area(), " Add note ", buffer),
        Mode::ContactInput { ref buffer } => draw_text_input(f, f.area(), " Add contact ", buffer),
        Mode::Form(ref form) => draw_form(f, f.area(), form),
        Mode::ConfirmDelete { ref label, .. } => draw_confirm_delete(f, f.area(), label),
        _ => {}
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let c = app.counts();
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut spans = vec![
        Span::styled(" Tracker ", Style::default().bold().fg(Color::White)),
        Span::raw("│ "),
        Span::raw(format!("Total {} ", c.total)),
        Span::raw("· "),
        Span::styled(format!("Active {} ", c.active), Style::default().fg(ACCENT)),
    ];
    if c.interview > 0 {
        spans.push(Span::raw("· "));
        spans.push(Span::styled(
            format!("Interview {} ", c.interview),
            Style::default().fg(status_color("interview")),
        ));
    }
    if c.overdue > 0 {
        spans.push(Span::raw("· "));
        spans.push(Span::styled(
            format!("⚠ Overdue {} ", c.overdue),
            Style::default().fg(status_color("rejected")).bold(),
        ));
    }
    if c.this_week > 0 {
        spans.push(Span::raw("· "));
        spans.push(Span::styled(
            format!("🔥 This week {} ", c.this_week),
            Style::default().fg(status_color("interview")),
        ));
    }
    spans.push(Span::raw("│ sort:"));
    spans.push(Span::styled(
        format!(" {} ", app.sort.label()),
        Style::default().fg(ACCENT),
    ));
    spans.push(Span::raw("│ "));
    spans.push(Span::styled(date, Style::default().fg(Color::DarkGray)));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

fn draw_body(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    draw_list(f, app, chunks[0]);
    draw_detail(f, app, chunks[1]);
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let apps = app.filtered();

    let title = format!(" {} ({}) ", app.filter.label(), apps.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);

    // Column widths: dot+id fixed, status fixed, warn fixed; company + position share the rest.
    let id_w: u16 = 7; // "● #999 "
    let status_w: u16 = 11; // "screening  "
    let warn_w: u16 = 2;
    let avail = inner.width.saturating_sub(id_w + status_w + warn_w);
    let company_w = (avail as u32 * 40 / 100) as u16;
    let position_w = avail.saturating_sub(company_w);

    let rows: Vec<Row> = apps
        .iter()
        .map(|a| {
            let dimmed = matches!(a.status.as_str(), "rejected" | "ghosted" | "withdrawn");
            let main_color = if dimmed {
                Color::DarkGray
            } else {
                Color::White
            };
            let sub_color = if dimmed {
                Color::Rgb(53, 58, 79)
            } else {
                Color::Gray
            };

            let id_cell = Cell::from(Line::from(vec![
                Span::styled("●", Style::default().fg(status_color(&a.status))),
                Span::raw(" "),
                Span::styled(
                    format!("#{:<4}", a.id),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));

            let company_cell = Cell::from(Span::styled(
                truncate(&a.company, company_w as usize),
                Style::default().fg(main_color),
            ));

            let position_cell = Cell::from(Span::styled(
                truncate(&a.position, position_w as usize),
                Style::default().fg(sub_color),
            ));

            let status_cell = Cell::from(Span::styled(
                a.status.clone(),
                Style::default().fg(status_color(&a.status)),
            ));

            let overdue = a
                .next_action_date
                .as_deref()
                .and_then(data::days_until)
                .map(|n| {
                    n < 0
                        && !matches!(
                            a.status.as_str(),
                            "rejected" | "ghosted" | "withdrawn" | "accepted"
                        )
                })
                .unwrap_or(false);
            let warn_cell = Cell::from(if overdue {
                Span::styled("⚠", Style::default().fg(status_color("rejected")))
            } else {
                Span::raw(" ")
            });

            Row::new(vec![
                id_cell,
                company_cell,
                position_cell,
                status_cell,
                warn_cell,
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(id_w),
            Constraint::Length(company_w),
            Constraint::Length(position_w),
            Constraint::Length(status_w),
            Constraint::Length(warn_w),
        ],
    )
    .block(block)
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(26, 32, 53))
            .add_modifier(Modifier::BOLD),
    );

    let mut state = TableState::default();
    if !apps.is_empty() {
        state.select(Some(app.selected.min(apps.len() - 1)));
    }
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let Some(a) = app.selected_app() else {
        let p = Paragraph::new("No applications match.")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(p, area);
        return;
    };

    let inner_width = block.inner(area).width.saturating_sub(2) as usize;

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(format!("#{}  ", a.id), Style::default().fg(Color::DarkGray)),
        Span::styled(a.company.clone(), Style::default().fg(Color::White).bold()),
    ]));
    lines.push(Line::from(Span::styled(
        a.position.clone(),
        Style::default().fg(Color::Gray),
    )));
    if let Some(loc) = &a.location {
        lines.push(Line::from(Span::styled(
            loc.clone(),
            Style::default().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::raw(""));

    lines.push(detail_row(
        "Status",
        &a.status,
        Some(status_color(&a.status)),
    ));
    if let Some(t) = &a.app_type {
        lines.push(detail_row("Type", t, None));
    }
    if let Some(r) = &a.reference {
        if !r.is_empty() {
            lines.push(detail_row("Ref", r, None));
        }
    }
    if let Some(d) = &a.applied_date {
        let s = format!("{}  ({})", d, data::rel_date(Some(d)));
        lines.push(detail_row("Applied", &s, None));
    }
    if let Some(d) = &a.deadline {
        lines.push(detail_row("Deadline", d, Some(Color::Yellow)));
    }

    if a.next_action.is_some() || a.next_action_date.is_some() {
        let na = a.next_action.as_deref().unwrap_or("—");
        let nad = a.next_action_date.as_deref();
        let s = match nad {
            Some(d) => {
                let du = data::days_until(d);
                let suffix = match du {
                    Some(n) if n < 0 => format!("  ({}d overdue)", -n),
                    Some(0) => "  (today)".into(),
                    Some(n) => format!("  ({}d to go)", n),
                    None => String::new(),
                };
                format!("{na}  ·  {d}{suffix}")
            }
            None => na.to_string(),
        };
        let color = match nad.and_then(data::days_until) {
            Some(n) if n < 0 => Some(status_color("rejected")),
            Some(n) if n <= 3 => Some(status_color("interview")),
            _ => None,
        };
        lines.push(detail_row("Next", &s, color));
    }

    if let Some(folder) = &a.folder {
        if !folder.is_empty() {
            lines.push(detail_row("Folder", folder, Some(ACCENT)));
        }
    }
    if let Some(url) = &a.url {
        if !url.is_empty() {
            lines.push(detail_row("URL", url, Some(ACCENT)));
        }
    }

    if !a.contacts.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Contacts",
            Style::default().fg(Color::DarkGray).bold(),
        )));
        for c in &a.contacts {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", c.date),
                    Style::default().fg(Color::Rgb(53, 58, 79)),
                ),
                Span::styled(c.info.clone(), Style::default().fg(Color::Gray)),
            ]));
        }
    }

    if !a.notes.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Notes",
            Style::default().fg(Color::DarkGray).bold(),
        )));
        for n in a.notes.iter().rev().take(8) {
            lines.push(Line::from(Span::styled(
                format!("  {}", n.date),
                Style::default().fg(Color::Rgb(53, 58, 79)),
            )));
            for line in textwrap_lines(&n.text, inner_width.saturating_sub(4)) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", line),
                    Style::default().fg(Color::Gray),
                )));
            }
        }
    }

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn detail_row(label: &str, value: &str, value_color: Option<Color>) -> Line<'static> {
    let value_style = match value_color {
        Some(c) => Style::default().fg(c),
        None => Style::default().fg(Color::White),
    };
    Line::from(vec![
        Span::styled(
            format!("  {:<10} ", label),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), value_style),
    ])
}

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    if let Some(err) = &app.error {
        let p = Paragraph::new(Line::from(Span::styled(
            format!("  ✗ {err}  (press any key to dismiss)"),
            Style::default().fg(status_color("rejected")).bold(),
        )));
        f.render_widget(p, area);
        return;
    }
    if let Some(flash) = &app.flash {
        let p = Paragraph::new(Line::from(Span::styled(
            format!("  ✓ {flash}"),
            Style::default().fg(status_color("offer")),
        )));
        f.render_widget(p, area);
        return;
    }
    let line = match &app.mode {
        Mode::Search => Line::from(vec![
            Span::styled("/", Style::default().fg(ACCENT).bold()),
            Span::raw(app.search.clone()),
            Span::styled("_", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::Help => Line::from(Span::styled(
            "  press ? or esc to close",
            Style::default().fg(Color::DarkGray),
        )),
        Mode::StatusPicker { .. } => Line::from(Span::styled(
            "  j/k select  ·  enter confirm  ·  esc cancel",
            Style::default().fg(Color::DarkGray),
        )),
        Mode::NoteInput { buffer } => {
            let mut spans = vec![Span::styled("note> ", Style::default().fg(ACCENT).bold())];
            spans.extend(textbuf_spans(buffer, true, Color::White));
            Line::from(spans)
        }
        Mode::ContactInput { buffer } => {
            let mut spans = vec![Span::styled(
                "contact> ",
                Style::default().fg(ACCENT).bold(),
            )];
            spans.extend(textbuf_spans(buffer, true, Color::White));
            Line::from(spans)
        }
        Mode::Form(_) => Line::from(Span::styled(
            "  tab/↓ next · shift-tab/↑ prev · enter save · esc cancel",
            Style::default().fg(Color::DarkGray),
        )),
        Mode::ConfirmDelete { .. } => Line::from(Span::styled(
            "  y confirm · n / esc cancel",
            Style::default().fg(Color::DarkGray),
        )),
        Mode::Normal => Line::from(vec![
            help_key("j/k"),
            help_sep(" nav"),
            help_key(" 1-5"),
            help_sep(" filter"),
            help_key(" o"),
            help_sep(" sort"),
            help_key(" /"),
            help_sep(" search"),
            help_key(" s"),
            help_sep(" status"),
            help_key(" n"),
            help_sep(" note"),
            help_key(" ?"),
            help_sep(" help"),
            help_key(" q"),
            help_sep(" quit"),
        ]),
    };
    f.render_widget(Paragraph::new(line).alignment(Alignment::Left), area);
}

fn help_key(s: &str) -> Span<'static> {
    Span::styled(s.to_string(), Style::default().fg(ACCENT).bold())
}

fn help_sep(s: &str) -> Span<'static> {
    Span::styled(s.to_string(), Style::default().fg(Color::DarkGray))
}

// ── Overlays ──────────────────────────────────────────────────────────────

fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let popup = centered_rect(60, 70, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));

    let lines = vec![
        section_title("Navigation"),
        help_line("j / ↓", "move down"),
        help_line("k / ↑", "move up"),
        help_line("g", "jump to top"),
        help_line("G", "jump to bottom"),
        Line::raw(""),
        section_title("Filters"),
        help_line("1", "all"),
        help_line("2", "active (applied/screening/interview/...)"),
        help_line("3", "interview"),
        help_line("4", "rejected"),
        help_line("5", "ghosted"),
        help_line("tab", "cycle filters"),
        Line::raw(""),
        section_title("Sort"),
        help_line("o", "cycle: date desc → status → company"),
        Line::raw(""),
        section_title("Search"),
        help_line("/", "fuzzy search (company + position)"),
        help_line("enter", "confirm"),
        help_line("esc", "cancel"),
        Line::raw(""),
        section_title("Open"),
        help_line("O", "open folder in file manager"),
        help_line("U", "open url in browser"),
        help_line("x", "export current view to PDF (typst)"),
        Line::raw(""),
        section_title("Edit"),
        help_line("a", "add a new application"),
        help_line("e", "edit selected application"),
        help_line("d", "delete selected (confirm)"),
        help_line("u", "undo last change (up to 10)"),
        help_line("s", "change status (picker)"),
        help_line("n", "add a note (today's date)"),
        help_line("c", "add a contact (today's date)"),
        Line::raw(""),
        section_title("Text input"),
        help_line("← / →", "move cursor"),
        help_line("home/end", "jump to start / end (also ^A / ^E)"),
        help_line("^← / ^→", "jump by word (also alt-b / alt-f)"),
        help_line("^W", "delete previous word"),
        help_line("^U", "clear field"),
        help_line("del", "delete forward"),
        Line::raw(""),
        section_title("General"),
        help_line("?", "toggle this help"),
        help_line("q / esc", "quit"),
    ];

    f.render_widget(Paragraph::new(lines).block(block), popup);
}

fn draw_status_picker(f: &mut Frame, area: Rect, selected_idx: usize) {
    let popup = centered_rect(40, 60, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Change status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));

    let items: Vec<ListItem> = STATUSES
        .iter()
        .map(|s| {
            ListItem::new(Line::from(vec![
                Span::styled("● ", Style::default().fg(status_color(s))),
                Span::styled(s.to_string(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::Rgb(26, 32, 53))
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    state.select(Some(selected_idx));
    f.render_stateful_widget(list, popup, &mut state);
}

fn draw_confirm_delete(f: &mut Frame, area: Rect, label: &str) {
    let popup = centered_rect(60, 25, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Confirm delete ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(status_color("rejected")));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            "Permanently delete this application?",
            Style::default().fg(Color::White).bold(),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            label.to_string(),
            Style::default().fg(Color::Gray),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            "  y confirm · n / esc cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        inner,
    );
}

/// Render a `TextBuf` as inline `Span`s, drawing a reversed-colour block
/// cursor at the current position when `focused` is true. The cursor is
/// painted over the character at the cursor — readline / Emacs style —
/// or over a trailing space if the cursor sits past the last character.
fn textbuf_spans(buf: &TextBuf, focused: bool, text_color: Color) -> Vec<Span<'static>> {
    let s = buf.as_string();
    if !focused {
        return vec![Span::styled(s, Style::default().fg(text_color))];
    }
    let chars: Vec<char> = s.chars().collect();
    let cursor = buf.cursor().min(chars.len());
    let before: String = chars[..cursor].iter().collect();
    let (at, after) = if cursor < chars.len() {
        let at = chars[cursor].to_string();
        let after: String = chars[cursor + 1..].iter().collect();
        (at, after)
    } else {
        (" ".to_string(), String::new())
    };
    let cursor_style = Style::default()
        .fg(text_color)
        .add_modifier(Modifier::REVERSED);
    vec![
        Span::styled(before, Style::default().fg(text_color)),
        Span::styled(at, cursor_style),
        Span::styled(after, Style::default().fg(text_color)),
    ]
}

fn draw_form(f: &mut Frame, area: Rect, form: &AppForm) {
    let popup = centered_rect(70, 90, area);
    f.render_widget(Clear, popup);

    let title = match form.edit_target_id {
        None => " Add application ".to_string(),
        Some(id) => format!(" Edit application #{id} "),
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // Reserve last 2 lines for hint + error.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::with_capacity(form.fields.len());
    let label_w: usize = form
        .fields
        .iter()
        .map(|f| f.label.chars().count())
        .max()
        .unwrap_or(0);
    for (i, field) in form.fields.iter().enumerate() {
        let is_focus = i == form.focus;
        let label_style = if is_focus {
            Style::default().fg(ACCENT).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let value_color = if is_focus { Color::White } else { Color::Gray };
        let arrow = if is_focus { "▶ " } else { "  " };
        let mut spans = vec![
            Span::styled(arrow.to_string(), Style::default().fg(ACCENT)),
            Span::styled(
                format!("{:<width$} ", field.label, width = label_w),
                label_style,
            ),
        ];
        spans.extend(textbuf_spans(&field.value, is_focus, value_color));
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[0]);

    let mut footer = Vec::new();
    if let Some(err) = &form.error {
        footer.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(status_color("rejected")).bold(),
        )));
    } else {
        footer.push(Line::raw(""));
    }
    footer.push(Line::from(Span::styled(
        "tab/↓ next · shift-tab/↑ prev · enter save · esc cancel",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(Paragraph::new(footer), chunks[1]);
}

fn draw_text_input(f: &mut Frame, area: Rect, title: &str, buffer: &TextBuf) {
    let popup = centered_rect(60, 20, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut prompt_spans = vec![Span::styled("> ", Style::default().fg(ACCENT).bold())];
    prompt_spans.extend(textbuf_spans(buffer, true, Color::White));

    let lines = vec![
        Line::from(Span::styled(
            format!("Date: {}", data::today_str()),
            Style::default().fg(Color::DarkGray),
        )),
        Line::raw(""),
        Line::from(prompt_spans),
        Line::raw(""),
        Line::from(Span::styled(
            "  enter save · esc cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn section_title(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", text),
        Style::default().fg(ACCENT).bold(),
    ))
}

fn help_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<8} ", key), Style::default().fg(ACCENT)),
        Span::styled(desc.to_string(), Style::default().fg(Color::Gray)),
    ])
}

// ── Utilities ──────────────────────────────────────────────────────────────

fn truncate(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if s.chars().count() <= width {
        return s.to_string();
    }
    let mut out: String = s.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn textwrap_lines(s: &str, width: usize) -> Vec<String> {
    if width < 8 {
        return vec![s.to_string()];
    }
    let mut out = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
        } else if line.chars().count() + 1 + word.chars().count() <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(std::mem::take(&mut line));
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r)[1];

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup)[1]
}
