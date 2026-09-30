//! Terminal rendering: rail, content with aligned values, search palette,
//! chooser/confirm/form overlays and mouse hit registration.

use crate::app::{
    App, Hit, HitAction, KEYBIND_FIELDS, MONITOR_FIELDS, Row, SearchState, ZONE_FIELD_LABELS,
};
use crate::catalog::{Group, Kind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

const ACCENT: Color = Color::Rgb(203, 166, 247);
const DIM: Color = Color::DarkGray;
const OK: Color = Color::Rgb(166, 227, 161);
const WARN: Color = Color::Rgb(249, 226, 175);

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let mut hits: Vec<Hit> = Vec::new();
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(area);

    render_header(f, app, chunks[0]);
    let body = Layout::horizontal([Constraint::Length(26), Constraint::Min(10)]).split(chunks[1]);
    render_rail(f, app, body[0], &mut hits);
    render_content(f, app, body[1], &mut hits);
    render_footer(f, app, chunks[2]);

    match &app.mode {
        crate::app::Mode::Chooser { target, idx } => {
            render_chooser(f, app, target, *idx, area, &mut hits)
        }
        crate::app::Mode::Confirm { msg, .. } => render_confirm(f, msg, area),
        crate::app::Mode::Form { kb, field } => render_form(f, app, *kb, *field, area),
        crate::app::Mode::Record { kb } => render_record(f, app, *kb, area),
        crate::app::Mode::Help => render_help(f, area),
        crate::app::Mode::Search(state) => render_search(f, state, area, &mut hits),
        _ => {}
    }

    app.hits = hits;
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let page = app.current_page();
    let left = Line::from(vec![
        Span::styled(
            " xturing ",
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} › {}", page.group.label(), page.label),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(format!("[{}]", app.engine), Style::default().fg(ACCENT)),
    ]);
    let left_width = line_width(&left);
    let right = " / search · click/double-tap · wheel ";
    let mut spans = left.spans;
    let pad = (area.width as usize).saturating_sub(left_width + right.chars().count());
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(right.to_string(), Style::default().fg(DIM)));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_rail(f: &mut Frame, app: &App, area: Rect, hits: &mut Vec<Hit>) {
    let block = Block::bordered().title(" Pages ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    let mut y = inner.y;
    for group in Group::ALL {
        let pages: Vec<&crate::catalog::Page> = app
            .pages
            .iter()
            .filter(|p| p.group == group && p.engine.map(|e| e == app.engine).unwrap_or(true))
            .collect();
        if pages.is_empty() {
            continue;
        }
        lines.push(Line::from(Span::styled(
            format!(" {}", group.label()),
            Style::default().fg(DIM).add_modifier(Modifier::BOLD),
        )));
        y += 1;
        for p in pages {
            let selected = p.id == app.current_page().id;
            let page_index = app
                .pages
                .iter()
                .position(|other| other.id == p.id)
                .unwrap_or(0);
            let marker = if selected { "▸" } else { " " };
            let style = if selected {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", marker), style),
                Span::styled(p.label, style),
            ]));
            hits.push(Hit {
                y,
                x0: inner.x,
                x1: inner.x + inner.width,
                action: HitAction::GotoPage(page_index),
            });
            y += 1;
        }
    }
    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_content(f: &mut Frame, app: &App, area: Rect, hits: &mut Vec<Hit>) {
    let title = format!(
        " {} · {}/{} ",
        app.current_page().label,
        if app.rows.is_empty() { 0 } else { app.sel + 1 },
        app.rows.len()
    );
    let block = Block::bordered().title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let height = inner.height as usize;
    let mut offset = app.sel.saturating_sub(height / 2);
    if offset + height > app.rows.len() {
        offset = app.rows.len().saturating_sub(height);
    }

    let width = inner.width as usize;
    let mut lines: Vec<Line> = Vec::new();
    for (i, row) in app.rows.iter().enumerate().skip(offset).take(height) {
        let selected = i == app.sel;
        let y = inner.y + (i - offset) as u16;
        hits.push(Hit {
            y,
            x0: inner.x,
            x1: inner.x + inner.width,
            action: HitAction::SelectRow(i),
        });
        if let Row::Control(ctrl) = row
            && let Some(c) = app.controls.get(*ctrl)
            && matches!(c.kind, Kind::Stepper { .. })
        {
            let value = control_value_spans(app, *ctrl, selected);
            let value_width: usize = value.iter().map(span_width).sum();
            let x_value = inner.x + 2 + LABEL_WIDTH as u16 + 1;
            hits.push(Hit {
                y,
                x0: x_value,
                x1: x_value + 3,
                action: HitAction::Adjust {
                    ctrl: *ctrl,
                    delta: -1,
                },
            });
            hits.push(Hit {
                y,
                x0: x_value + 4 + value_width as u16,
                x1: x_value + 7 + value_width as u16,
                action: HitAction::Adjust {
                    ctrl: *ctrl,
                    delta: 1,
                },
            });
        }
        lines.push(render_row(app, row, selected, width));
    }
    f.render_widget(Paragraph::new(Text::from(lines)), inner);

    if app.rows.len() > height {
        let mut state = ScrollbarState::new(app.rows.len()).position(app.sel);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            inner,
            &mut state,
        );
    }
}

const LABEL_WIDTH: usize = 34;

fn render_row(app: &App, row: &Row, selected: bool, width: usize) -> Line<'static> {
    let base = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let dim = if selected {
        base
    } else {
        Style::default().fg(DIM)
    };
    let accent = if selected {
        base
    } else {
        Style::default().fg(ACCENT)
    };

    match row {
        Row::Section(title) => {
            let fill = "─".repeat(width.saturating_sub(title.chars().count() + 5));
            Line::from(vec![
                Span::styled("── ", Style::default().fg(DIM)),
                Span::styled(title.clone(), Style::default().fg(DIM)),
                Span::styled(format!(" {}", fill), Style::default().fg(DIM)),
            ])
        }
        Row::Info(label, key) => {
            let value = app.info_value(key);
            let mut spans = vec![
                Span::styled("  ", dim),
                Span::styled(pad(label, LABEL_WIDTH), base),
                Span::styled(value, dim),
            ];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::Control(idx) => {
            let label = app
                .controls
                .get(*idx)
                .map(|c| c.label.clone())
                .unwrap_or_default();
            let mut spans = vec![
                Span::styled("  ", dim),
                Span::styled(pad(&label, LABEL_WIDTH), base),
            ];
            spans.extend(control_value_spans(app, *idx, selected));
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::Keybind(i) => {
            let b = app.keybinds.get(*i).cloned().unwrap_or_default();
            let field = |k: &str| b.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let combo = format!("{} {}", field("mods"), field("key"));
            let detail = format!(
                "[{}] {} {}",
                field("type"),
                field("dispatcher"),
                field("command")
            );
            let mut spans = vec![
                Span::styled("  ⌨ ", accent),
                Span::styled(pad(&combo, 22), base),
                Span::styled(detail, dim),
            ];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::Startup(i) => {
            let cmd = app
                .startup
                .get(*i)
                .and_then(|s| s.get("command"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut spans = vec![Span::styled("  ▸ ", accent), Span::styled(cmd, base)];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::ZoneHeader(z) => {
            let zones = zones_of(app);
            let zone = zones.get(*z);
            let id = zone
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let align = zone
                .and_then(|v| v.get("align"))
                .and_then(|v| v.as_str())
                .unwrap_or("start");
            let mut spans = vec![
                Span::styled(format!(" ◈ {} ", id), accent),
                Span::styled(format!("align={}", align), dim),
            ];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::ZoneField { zone, field } => {
            let zones = zones_of(app);
            let value = zone_field_value(zones.get(*zone), *field);
            let mut spans = vec![
                Span::styled("     ", dim),
                Span::styled(pad(ZONE_FIELD_LABELS[*field], 30), base),
                Span::styled(value, dim),
            ];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::ZoneModule { zone, idx } => {
            let zones = zones_of(app);
            let m = zones
                .get(*zone)
                .and_then(|z| z.get("modules"))
                .and_then(|v| v.as_array())
                .and_then(|a| a.get(*idx));
            let id = m
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let enabled = m
                .and_then(|v| v.get("enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let glyph = if enabled { "●" } else { "○" };
            let style = if selected {
                base
            } else if enabled {
                Style::default().fg(OK)
            } else {
                dim
            };
            let mut spans = vec![
                Span::styled("       ", dim),
                Span::styled(format!("{} {}", glyph, id), style),
            ];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::ClassicItem { sec, idx } => {
            let items = classic_items_of(app, *sec);
            let item = items.get(*idx).cloned().unwrap_or_default();
            let label = crate::classic::item_label(&item);
            let grouped = matches!(item, serde_json::Value::Array(_));
            let mut spans = vec![
                Span::styled("   ", dim),
                Span::styled(
                    if grouped {
                        format!("▣ {}", label)
                    } else {
                        format!("▢ {}", label)
                    },
                    base,
                ),
            ];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::PaletteItem(i) => {
            let p = app.palettes.get(*i);
            let (name, slug, cat) = p
                .map(|e| (e.name.clone(), e.slug.clone(), e.category.clone()))
                .unwrap_or_default();
            let active = app
                .settings
                .get_str(&["bar", "palette"])
                .map(|s| s == slug)
                .unwrap_or(false);
            let mut spans = vec![Span::styled("  ", dim)];
            if active {
                spans.push(Span::styled("● ", Style::default().fg(OK)));
            } else {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(pad(&name, 28), base));
            spans.push(Span::styled(format!("{} · {}", slug, cat), dim));
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::PaletteSlot(slot) => {
            let (hex, label) = app
                .palette
                .as_ref()
                .map(|p| {
                    (
                        p.slot(*slot).to_string(),
                        crate::palette::PaletteFile::slot_label(*slot),
                    )
                })
                .unwrap_or_default();
            let color = parse_hex_color(&hex).unwrap_or(Color::Gray);
            let mut spans = vec![
                Span::styled("  ", dim),
                Span::styled("██ ", Style::default().fg(color)),
                Span::styled(pad(&label, 14), base),
                Span::styled(hex, dim),
            ];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
        Row::MonitorField { mon, field } => {
            let value = monitor_value(app, *mon, *field);
            let mut spans = vec![
                Span::styled("  ", dim),
                Span::styled(pad(MONITOR_FIELDS[*field].0, 24), base),
                Span::styled(value, dim),
            ];
            fill_line(&mut spans, width, base);
            Line::from(spans)
        }
    }
}

fn control_value_spans(app: &App, idx: usize, selected: bool) -> Vec<Span<'static>> {
    let base = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let dim = if selected {
        base
    } else {
        Style::default().fg(DIM)
    };
    let accent = if selected {
        base
    } else {
        Style::default().fg(ACCENT)
    };
    let Some(c) = app.controls.get(idx) else {
        return Vec::new();
    };
    match &c.kind {
        Kind::Toggle => {
            let on = app
                .control_value(c)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if selected {
                vec![Span::styled(if on { "  ● ON" } else { "  ○ OFF" }, base)]
            } else if on {
                vec![Span::styled("  ● ON", Style::default().fg(OK))]
            } else {
                vec![Span::styled("  ○ OFF", dim)]
            }
        }
        Kind::Stepper { unit, .. } => {
            let value = app
                .control_value(c)
                .and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
                .map(crate::app::fmt_num)
                .unwrap_or_else(|| "—".into());
            let display = if unit.is_empty() {
                value
            } else {
                format!("{} {}", value, unit)
            };
            vec![
                Span::styled("[-] ", accent),
                Span::styled(display, base),
                Span::styled(" [+]", accent),
            ]
        }
        Kind::Options(opts) => {
            let cur = app.control_value(c).map(value_string).unwrap_or_default();
            let label = opts
                .iter()
                .find(|o| o.value == cur)
                .map(|o| o.label.clone())
                .unwrap_or(cur);
            vec![
                Span::styled("‹ ", accent),
                Span::styled(label, base),
                Span::styled(" ›", accent),
            ]
        }
        Kind::StateOptions {
            state_key, options, ..
        } => {
            let cur = app.vstate.get(state_key).cloned().unwrap_or_default();
            let label = options
                .iter()
                .find(|o| o.value == cur)
                .map(|o| o.label.clone())
                .unwrap_or_else(|| "—".into());
            vec![
                Span::styled("‹ ", accent),
                Span::styled(label, base),
                Span::styled(" ›", accent),
            ]
        }
        Kind::Text { placeholder, .. } => {
            let value = app
                .control_value(c)
                .map(value_string)
                .filter(|s| !s.is_empty());
            match value {
                Some(v) => vec![Span::styled(v, base)],
                None => vec![Span::styled(format!("({})", placeholder), dim)],
            }
        }
        Kind::Action(_) => vec![Span::styled("›", accent)],
        Kind::Info(key) => vec![Span::styled(app.info_value(key), dim)],
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let mut help = String::new();
    if let Some(row) = app.selected_row() {
        match &row {
            Row::Control(i) => {
                if let Some(c) = app.controls.get(*i) {
                    help = c.help.clone();
                }
            }
            Row::Keybind(_) => {
                help = "Enter: edit · a: add · d: delete · w: save and reload Hyprland".into()
            }
            Row::Startup(_) => {
                help = "Enter: edit · a: add · d: delete · w: save and reload".into()
            }
            Row::ZoneField { field, .. } => {
                help = match field {
                    7 => "Enter: delete the zone (at least one must remain)".into(),
                    _ => "←/→ or click [-] [+]: change value".to_string(),
                }
            }
            Row::ZoneModule { .. } => {
                help = "Enter: enable/disable · Shift+←/→: move (crosses zones)".into()
            }
            Row::ClassicItem { .. } => {
                help = "↑/↓: reorder · m: move section · g: group · u: ungroup".into()
            }
            Row::PaletteItem(_) => help = "Enter or click: activate palette".into(),
            Row::PaletteSlot(_) => help = "Enter: edit hex (#rrggbb)".into(),
            Row::MonitorField { field, .. } => {
                help = if *field == 8 || *field == 9 {
                    "←/→: move 10px; Apply to commit".into()
                } else {
                    "Enter: choose · ←/→: cycle; Apply to commit".into()
                }
            }
            _ => {}
        }
    }
    let status = if app.status.is_empty() {
        String::new()
    } else {
        format!("  ·  {}", app.status)
    };
    let keys =
        "  ↑↓ move · ←→ adjust · Enter select · / search · Tab page · 1-6 group · ? help · q quit";
    let lines = Text::from(vec![
        Line::from(Span::styled(help, Style::default().fg(ACCENT))),
        Line::from(vec![
            Span::styled(keys, Style::default().fg(DIM)),
            Span::styled(status, Style::default().fg(WARN)),
        ]),
    ]);
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}

fn render_search(f: &mut Frame, state: &SearchState, area: Rect, hits: &mut Vec<Hit>) {
    let width = 78.min(area.width.saturating_sub(4));
    let height = (area.height.saturating_sub(4)).min(20);
    let rect = centered(area, width, height);
    f.render_widget(Clear, rect);
    let block = Block::bordered()
        .border_style(Style::default().fg(ACCENT))
        .title(" Search — type to filter ");
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    if inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let mut spans = vec![Span::styled("› ", Style::default().fg(ACCENT))];
    spans.push(Span::styled(state.query.clone(), Style::default()));
    spans.push(Span::styled(
        " ",
        Style::default().add_modifier(Modifier::REVERSED),
    ));
    lines.push(Line::from(spans));
    lines.push(Line::from(Span::styled(
        "─".repeat(inner.width as usize),
        Style::default().fg(DIM),
    )));

    let list_height = inner.height.saturating_sub(3) as usize;
    let mut offset = state.sel.saturating_sub(list_height.saturating_sub(1));
    if offset + list_height > state.results.len() {
        offset = state.results.len().saturating_sub(list_height);
    }
    for (i, entry) in state
        .results
        .iter()
        .enumerate()
        .skip(offset)
        .take(list_height)
    {
        let selected = i == state.sel;
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let ctx_style = if selected {
            style
        } else {
            Style::default().fg(DIM)
        };
        let available = inner.width as usize;
        let text = truncate(&entry.text, available.saturating_sub(6));
        let ctx = truncate(&entry.context, 34);
        let used = text.chars().count() + ctx.chars().count() + 2;
        let pad = available.saturating_sub(used + 2);
        lines.push(Line::from(vec![
            Span::styled("  ", style),
            Span::styled(text, style),
            Span::styled(" ".repeat(pad), style),
            Span::styled(ctx, ctx_style),
            Span::styled("  ", style),
        ]));
        hits.push(Hit {
            y: inner.y + 2 + (i - offset) as u16,
            x0: inner.x,
            x1: inner.x + inner.width,
            action: HitAction::SearchSelect(i),
        });
    }
    let hint = if state.results.is_empty() {
        "no results · Esc to close".to_string()
    } else {
        format!(
            "{} result(s) · Enter open · ↑↓ move · Esc close",
            state.results.len()
        )
    };
    lines.push(Line::from(Span::styled(hint, Style::default().fg(DIM))));
    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_chooser(
    f: &mut Frame,
    app: &App,
    target: &crate::app::ChooserTarget,
    idx: usize,
    area: Rect,
    hits: &mut Vec<Hit>,
) {
    let (title, opts) = match target {
        crate::app::ChooserTarget::Control { opts, .. } => ("Select".to_string(), opts.clone()),
        crate::app::ChooserTarget::Keybind { field, .. } => (
            KEYBIND_FIELDS[*field].1.to_string(),
            crate::app::KEYBIND_OPTIONS[*field]
                .iter()
                .map(|(l, v)| crate::catalog::Opt {
                    label: (*l).into(),
                    value: (*v).into(),
                })
                .collect(),
        ),
        crate::app::ChooserTarget::Monitor { mon, field } => (
            MONITOR_FIELDS[*field].0.to_string(),
            app.monitor_field_options_pub(*mon, *field),
        ),
    };
    let height = ((opts.len() + 2) as u16).min(area.height.saturating_sub(4));
    let width = 52.min(area.width.saturating_sub(4));
    let rect = centered(area, width, height);
    f.render_widget(Clear, rect);
    let items: Vec<ListItem> = opts
        .iter()
        .map(|o| ListItem::new(o.label.clone()))
        .collect();
    let list = List::new(items)
        .block(Block::bordered().title(format!(" {} ", title)))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(idx.min(opts.len().saturating_sub(1))));
    f.render_stateful_widget(list, rect, &mut state);
    // hit zones: one row per visible option
    let inner = rect.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    let visible = inner.height as usize;
    let offset = state.offset();
    for i in offset..(offset + visible).min(opts.len()) {
        hits.push(Hit {
            y: inner.y + (i - offset) as u16,
            x0: inner.x,
            x1: inner.x + inner.width,
            action: HitAction::ChooserSelect(i),
        });
    }
}

fn render_confirm(f: &mut Frame, msg: &str, area: Rect) {
    let rect = centered(area, 60.min(area.width.saturating_sub(4)), 5);
    f.render_widget(Clear, rect);
    let text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(WARN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  y / Enter: confirmar        n / Esc: cancelar",
            Style::default().fg(DIM),
        )),
    ]);
    f.render_widget(
        Paragraph::new(text).block(Block::bordered().title(" Confirm ")),
        rect,
    );
}

fn render_form(f: &mut Frame, app: &App, kb: usize, field: usize, area: Rect) {
    let rect = centered(area, 70.min(area.width.saturating_sub(4)), 8);
    f.render_widget(Clear, rect);
    let b = app.keybinds.get(kb).cloned().unwrap_or_default();
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    for (i, (key, label)) in KEYBIND_FIELDS.iter().enumerate() {
        let value = b
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let marker = if i == field { "▸ " } else { "  " };
        let style = if i == field {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {}", marker), style),
            Span::styled(format!("{:<12}", label), style),
            Span::styled(value, Style::default().fg(DIM)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑↓ field · Enter edit · r record shortcut · Esc close · w save keybinds",
        Style::default().fg(DIM),
    )));
    f.render_widget(
        Paragraph::new(Text::from(lines)).block(Block::bordered().title(" Keybind ")),
        rect,
    );
}

fn render_record(f: &mut Frame, app: &App, kb: usize, area: Rect) {
    let rect = centered(area, 62.min(area.width.saturating_sub(4)), 6);
    f.render_widget(Clear, rect);
    let bind = app.keybinds.get(kb).cloned().unwrap_or_default();
    let mods = bind.get("mods").and_then(|v| v.as_str()).unwrap_or("");
    let key = bind.get("key").and_then(|v| v.as_str()).unwrap_or("");
    let lines = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Press the key combination to record...",
            Style::default().fg(ACCENT),
        )),
        Line::from(Span::styled(
            format!("  current: {} {}", mods, key),
            Style::default().fg(DIM),
        )),
        Line::from(Span::styled("  Esc cancels", Style::default().fg(DIM))),
    ]);
    f.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(" Record shortcut ")),
        rect,
    );
}

fn render_help(f: &mut Frame, area: Rect) {
    let rect = centered(
        area,
        76.min(area.width.saturating_sub(4)),
        26.min(area.height.saturating_sub(2)),
    );
    f.render_widget(Clear, rect);
    let lines = vec![
        Line::from(Span::styled("  Navigation", Style::default().fg(ACCENT))),
        Line::from("  ↑/↓, j/k        move selection"),
        Line::from("  ←/→, h/l        adjust steppers / cycle options"),
        Line::from("  Enter / Space   edit, toggle, open chooser, run action"),
        Line::from("  ↑/↓ edges       wrap to the previous / next page"),
        Line::from("  Ctrl+←/→, [ ]   previous / next page"),
        Line::from("  /  or Ctrl+P    search any option (search palette)"),
        Line::from("  Tab / Shift+Tab next / previous page"),
        Line::from("  1..6            jump to group Shell, Bar, Theme, Behavior, Widgets, System"),
        Line::from("  r               reload settings.json from disk"),
        Line::from("  ?               this help"),
        Line::from("  q / Ctrl+C      quit (flushes pending Hyprland effects)"),
        Line::from(""),
        Line::from(Span::styled("  Mouse / touch", Style::default().fg(ACCENT))),
        Line::from("  click           select (rail: switch page)"),
        Line::from("  double-tap      open / run the selected row"),
        Line::from("  [-] / [+]       pulsar o arrastrar en steppers"),
        Line::from("  wheel           move selection up / down"),
        Line::from(""),
        Line::from(Span::styled("  Lists", Style::default().fg(ACCENT))),
        Line::from("  a / d / w       add / delete / save (keybinds, startup)"),
        Line::from("  r (in form)     record the shortcut by pressing it"),
        Line::from(""),
        Line::from(Span::styled(
            "  Zones / Classic",
            Style::default().fg(ACCENT),
        )),
        Line::from("  Shift+←/→       move a module across zones"),
        Line::from("  m / g / u       move section / group / ungroup (classic)"),
        Line::from(""),
        Line::from(Span::styled(
            "  The TUI writes tmp+mv atomically, same as the QML panel.",
            Style::default().fg(DIM),
        )),
        Line::from(Span::styled(
            "  (press any key to close)",
            Style::default().fg(DIM),
        )),
    ];
    f.render_widget(
        Paragraph::new(Text::from(lines)).block(Block::bordered().title(" Help ")),
        rect,
    );
}

// ───────────────────────── helpers ─────────────────────────

fn zones_of(app: &App) -> Vec<serde_json::Value> {
    app.settings
        .get(&["bar", "zones"])
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn zone_field_value(zone: Option<&serde_json::Value>, field: usize) -> String {
    let Some(z) = zone else { return "—".into() };
    let s = |k: &str| z.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let b = |k: &str| z.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    let n = |k: &str| z.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
    match field {
        0 => s("align"),
        1 => on_off(b("unify")),
        2 => {
            let bg = s("zoneBg");
            if bg.is_empty() {
                "○ OFF".into()
            } else {
                format!("● ON ({})", bg)
            }
        }
        3 => {
            let bg = s("zoneBg");
            if bg.is_empty() { "—".into() } else { bg }
        }
        4 => on_off(b("zoneBgSolid")),
        5 => format!("{} px", n("borderWidth") as i64),
        6 => s("borderColor"),
        _ => String::new(),
    }
}

fn classic_items_of(app: &App, sec: usize) -> Vec<serde_json::Value> {
    app.settings
        .get(&["classicbar", "modules", crate::classic::SECTIONS[sec]])
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn monitor_value(app: &App, mon: usize, field: usize) -> String {
    let Some(m) = app.monitors.get(mon) else {
        return "—".into();
    };
    match field {
        0 => {
            if m.disabled {
                "○ disabled".into()
            } else {
                "● enabled".into()
            }
        }
        1 => format!("{}x{}", m.w, m.h),
        2 => format!("{} Hz", m.rate.round() as u32),
        3 => format!("{}°", m.transform),
        4 => match m.vrr {
            0 => "Off".into(),
            1 => "On".into(),
            _ => "Fullscreen".into(),
        },
        5 => format!("{}-bit", m.bitdepth),
        6 => m.cm.clone(),
        7 => {
            if m.mirror == "none" {
                "None".into()
            } else {
                m.mirror.clone()
            }
        }
        8 => format!("{}", m.x),
        9 => format!("{}", m.y),
        _ => "—".into(),
    }
}

fn on_off(v: bool) -> String {
    if v { "● ON".into() } else { "○ OFF".into() }
}

fn value_string(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                n.as_f64().map(crate::app::fmt_num).unwrap_or_default()
            }
        }
        other => other.to_string(),
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let s = hex.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn pad(s: &str, width: usize) -> String {
    let mut out = s.to_string();
    let len = out.chars().count();
    if len < width {
        out.push_str(&" ".repeat(width - len));
    }
    out
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    let mut out: String = s.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn span_width(span: &Span) -> usize {
    span.content.chars().count()
}

fn line_width(line: &Line) -> usize {
    line.spans.iter().map(span_width).sum()
}

fn fill_line(spans: &mut Vec<Span<'static>>, width: usize, fill: Style) {
    let used: usize = spans.iter().map(span_width).sum();
    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), fill));
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}
