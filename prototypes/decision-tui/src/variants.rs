// Throwaway prototype rendering for wayfinder ticket #7.
// Variant 1 "Focus": the decision takes the whole screen (modal takeover).
// Variant 2 "Inline": the decision appears inside the agent's activity stream.
// Variant 3 "Mission": persistent panes; activity left, decision pane right, journal on `h`.
// Switch variants with [ and ]. Nothing here is production code.

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::script::{
    choice_at, App, Choice, Kind, LogEvent, Phase, VStage, AFFECTED, ARTIFACT, BLOCK_ATTEMPT,
    BLOCK_REASON, BLOCK_STEER, CONTEXT, OPTIONS, RECORD_PATH, STATEMENT, WARNINGS,
};

pub fn variant_name(variant: usize) -> &'static str {
    match variant {
        0 => "Focus -- the decision takes the screen",
        1 => "Inline -- the decision lives in the stream",
        _ => "Mission -- panes + journal",
    }
}

fn kind_glyph(kind: Kind) -> (&'static str, Color) {
    match kind {
        Kind::Task => ("◆", Color::Cyan),
        Kind::Info => ("·", Color::Gray),
        Kind::Read => ("◦", Color::Blue),
        Kind::Edit => ("✎", Color::Yellow),
        Kind::Test => ("↺", Color::Magenta),
        Kind::Decision => ("⚑", Color::LightRed),
        Kind::Ack => ("⚠", Color::Yellow),
        Kind::Grant => ("✔", Color::Green),
        Kind::Block => ("⛔", Color::Red),
        Kind::Done => ("★", Color::Green),
    }
}

fn event_line(event: &LogEvent) -> Line<'static> {
    let (glyph, color) = kind_glyph(event.kind);
    let text_style = match event.kind {
        Kind::Info => Style::new().fg(Color::Gray),
        _ => Style::new(),
    };
    Line::from(vec![
        Span::styled(format!(" {glyph} "), Style::new().fg(color)),
        Span::styled(event.text.clone(), text_style),
    ])
}

fn centered(area: Rect, w_pct: u16, h_pct: u16) -> Rect {
    let w_pct = if area.width < 100 { 98 } else { w_pct };
    let h_pct = if area.height < 30 { 98 } else { h_pct };
    let vert = Layout::vertical([
        Constraint::Percentage((100 - h_pct) / 2),
        Constraint::Percentage(h_pct),
        Constraint::Percentage((100 - h_pct) / 2),
    ])
    .areas::<3>(area);
    let horiz = Layout::horizontal([
        Constraint::Percentage((100 - w_pct) / 2),
        Constraint::Percentage(w_pct),
        Constraint::Percentage((100 - w_pct) / 2),
    ])
    .areas::<3>(vert[1]);
    horiz[1]
}

// ---------------------------------------------------------------- app render

pub fn render_app(f: &mut Frame, app: &App) {
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas::<2>(f.area());
    match app.variant {
        0 => render_v1(app, f, rows[0]),
        1 => render_v2(app, f, rows[0]),
        _ => render_v3(app, f, rows[0]),
    }
    render_chrome(app, f, rows[1]);
    if app.show_journal {
        render_journal(app, f, rows[0]);
    }
}

fn render_chrome(app: &App, f: &mut Frame, area: Rect) {
    let label = format!(
        " PROTOTYPE -- wayfinder #7 (throwaway) · variant {}/3: {} · [ ] switch · ctrl+r replay · space advance · h journal · q quit ",
        app.variant + 1,
        variant_name(app.variant)
    );
    f.render_widget(
        Paragraph::new(label).style(Style::new().bg(Color::Yellow).fg(Color::Black)),
        area,
    );
}

fn render_journal(app: &App, f: &mut Frame, area: Rect) {
    let box_area = centered(area, 84, 72);
    f.render_widget(Clear, box_area);
    let block = Block::bordered()
        .title(Line::from(Span::styled(
            " task journal -- what Forge recorded ",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(box_area);
    f.render_widget(block, box_area);
    let mut lines: Vec<Line> = Vec::new();
    for entry in &app.journal {
        lines.push(event_line(entry));
    }
    if app.journal.is_empty() {
        lines.push(Line::from(Span::styled(
            " (nothing recorded yet)",
            Style::new().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " h / esc -- close",
        Style::new().fg(Color::DarkGray),
    )));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ------------------------------------------------------------------ variant 1

fn render_v1(app: &App, f: &mut Frame, area: Rect) {
    match app.phase {
        Phase::Paused => {
            let stage = app.verdict.as_ref().map(|v| v.stage);
            match stage {
                Some(VStage::Selecting) => v1_select(app, f, area),
                Some(VStage::Acking) => v1_ack(app, f, area),
                Some(VStage::Rationale) => v1_rationale(app, f, area),
                None => {}
            }
        }
        Phase::Blocked => v1_block(f, area),
        _ => v1_stream(app, f, area),
    }
}

fn v1_stream(app: &App, f: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(
                " forge ",
                Style::new().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " · task: make payment capture retry-safe",
                Style::new().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
    ];
    let avail = area.height.saturating_sub(4) as usize;
    let skip = app.events.len().saturating_sub(avail);
    lines.extend(app.events.iter().skip(skip).map(event_line));
    lines.push(Line::from(""));
    let state = match app.phase {
        Phase::Complete => "complete -- ctrl+r to replay".to_string(),
        Phase::Aborted => "aborted -- ctrl+r to replay".to_string(),
        _ => format!("running -- space advances the scripted run · {} events", app.events.len()),
    };
    lines.push(Line::from(Span::styled(format!(" {state}"), Style::new().fg(Color::DarkGray))));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn v1_select(app: &App, f: &mut Frame, area: Rect) {
    let v = app.verdict.as_ref().unwrap();
    let box_area = centered(area, 90, 86);
    let block = Block::bordered()
        .title(Line::from(Span::styled(
            " DECISION REQUIRED -- execution paused ",
            Style::new().fg(Color::LightRed).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::new().fg(Color::LightRed));
    let inner = block.inner(box_area);
    f.render_widget(block, box_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        STATEMENT.to_string(),
        Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {CONTEXT}"),
        Style::new().fg(Color::Gray),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Options",
        Style::new().add_modifier(Modifier::BOLD),
    )));
    push_option_lines(&mut lines, v.highlight, true);
    lines.push(Line::from(""));
    if v.highlight < OPTIONS.len() {
        let option = &OPTIONS[v.highlight];
        lines.push(Line::from(Span::styled(
            "  trade-offs",
            Style::new().fg(Color::Cyan),
        )));
        for tradeoff in option.tradeoffs {
            lines.push(Line::from(Span::styled(
                format!("    + {tradeoff}"),
                Style::new().fg(Color::Gray),
            )));
        }
        for risk in option.risks {
            lines.push(Line::from(Span::styled(
                format!("    ⚠ {risk}"),
                Style::new().fg(Color::Yellow),
            )));
        }
        lines.push(Line::from(Span::styled(
            format!("    → if chosen: {}", option.consequences),
            Style::new().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  (control -- no record will be written)",
            Style::new().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!("  affected: {}", AFFECTED.join(" · ")),
            Style::new().fg(Color::Blue),
        ),
        Span::styled(format!("   record: {ARTIFACT}"), Style::new().fg(Color::Blue)),
    ]));
    lines.push(Line::from(Span::styled(
        "  ↑↓ move · 1-5 select · enter confirm",
        Style::new().fg(Color::DarkGray),
    )));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn push_option_lines(lines: &mut Vec<Line<'static>>, highlight: usize, digits: bool) {
    for (i, option) in OPTIONS.iter().enumerate() {
        let selected = i == highlight;
        let mut spans = vec![
            Span::styled(
                if selected { "  ❯ " } else { "    " }.to_string(),
                Style::new().fg(Color::LightRed),
            ),
            Span::styled(
                format!("{} · ", i + 1),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                option.label.to_string(),
                Style::new().add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() }),
            ),
        ];
        if option.recommended {
            spans.push(Span::styled(
                "  [recommended]",
                Style::new().fg(Color::Green),
            ));
        }
        spans.push(Span::styled(
            format!("   {}", option.gist),
            Style::new().fg(Color::DarkGray),
        ));
        lines.push(Line::from(spans));
    }
    for (i, label) in ["Reject -- replan", "Abort task"].iter().enumerate() {
        let idx = OPTIONS.len() + i;
        let selected = idx == highlight;
        let mut spans = vec![
            Span::styled(
                if selected { "  ❯ " } else { "    " }.to_string(),
                Style::new().fg(Color::LightRed),
            ),
            Span::styled(
                format!("{} · ", idx + 1),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                label.to_string(),
                Style::new().fg(Color::Gray).add_modifier(if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
        ];
        if !digits {
            spans.clear();
        }
        lines.push(Line::from(spans));
    }
}

fn v1_ack(app: &App, f: &mut Frame, area: Rect) {
    let v = app.verdict.as_ref().unwrap();
    let box_area = centered(area, 76, 48);
    let block = Block::bordered()
        .title(Line::from(Span::styled(
            " ACKNOWLEDGMENT REQUIRED -- explicit, one at a time ",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::new().fg(Color::Yellow));
    let inner = block.inner(box_area);
    f.render_widget(block, box_area);
    let mut lines: Vec<Line> = Vec::new();
    for (i, warning) in WARNINGS.iter().enumerate() {
        let line = if i < v.acked {
            Line::from(Span::styled(
                format!("  ✔ {warning}"),
                Style::new().fg(Color::Green),
            ))
        } else if i == v.acked {
            Line::from(Span::styled(
                format!("  → ⚠ {warning}"),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))
        } else {
            Line::from(Span::styled(
                format!("    {warning}"),
                Style::new().fg(Color::DarkGray),
            ))
        };
        lines.push(line);
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        "  Acknowledgment is journaled before the verdict completes.",
        Style::new().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            "  space / enter -- acknowledge ({} of {})",
            (v.acked + 1).min(WARNINGS.len()),
            WARNINGS.len()
        ),
        Style::new().fg(Color::DarkGray),
    )));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn v1_rationale(app: &App, f: &mut Frame, area: Rect) {
    let v = app.verdict.as_ref().unwrap();
    let box_area = centered(area, 76, 40);
    let block = Block::bordered()
        .title(Line::from(Span::styled(
            " AMENDMENTS / RATIONALE -- optional ",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(box_area);
    f.render_widget(block, box_area);
    let lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  Anything to amend, or a note to record with the verdict?",
            Style::new().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ❯ ", Style::new().fg(Color::Yellow)),
            Span::styled(v.rationale.clone(), Style::new()),
            Span::styled("█", Style::new().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  enter -- submit · empty accepts the choice as-is",
            Style::new().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn v1_block(f: &mut Frame, area: Rect) {
    let box_area = centered(area, 90, 60);
    let block = Block::bordered()
        .title(Line::from(Span::styled(
            " ⛔ HARD BLOCK -- execution stopped ",
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::new().fg(Color::Red));
    let inner = block.inner(box_area);
    f.render_widget(block, box_area);
    let lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("  attempted    ", Style::new().fg(Color::Red)),
            Span::styled(BLOCK_ATTEMPT.to_string(), Style::new()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  reason       ", Style::new().fg(Color::Red)),
            Span::styled(BLOCK_REASON.to_string(), Style::new().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  steer        ", Style::new().fg(Color::Red)),
            Span::styled(BLOCK_STEER.to_string(), Style::new().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  applied      ", Style::new().fg(Color::Red)),
            Span::styled(
                "tool call denied · nothing written · journaled",
                Style::new().fg(Color::Gray),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  enter -- acknowledge and continue",
            Style::new().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ------------------------------------------------------------------ variant 2

fn render_v2(app: &App, f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas::<2>(area);
    let mut lines: Vec<Line> = app.events.iter().map(event_line).collect();
    match app.phase {
        Phase::Paused => {
            let v = app.verdict.as_ref().unwrap();
            match v.stage {
                VStage::Selecting => v2_select_lines(app, &mut lines),
                VStage::Acking => v2_ack_lines(v, &mut lines),
                VStage::Rationale => v2_rationale_lines(v, &mut lines),
            }
        }
        Phase::Blocked => v2_block_lines(&mut lines),
        Phase::Complete => lines.push(Line::from(Span::styled(
            "    · ctrl+r -- replay",
            Style::new().fg(Color::DarkGray),
        ))),
        Phase::Aborted => lines.push(Line::from(Span::styled(
            " · task aborted -- ctrl+r to replay",
            Style::new().fg(Color::Gray),
        ))),
        Phase::Running => {}
    }

    let height = rows[0].height as usize;
    let skip = lines.len().saturating_sub(height);
    let visible: Vec<Line> = lines.into_iter().skip(skip).collect();
    f.render_widget(
        Paragraph::new(visible).wrap(Wrap { trim: false }),
        rows[0],
    );

    let (state, color) = match app.phase {
        Phase::Running => ("RUNNING", Color::Green),
        Phase::Paused => ("PAUSED -- awaiting developer", Color::Yellow),
        Phase::Blocked => ("BLOCKED", Color::Red),
        Phase::Complete => ("COMPLETE", Color::Green),
        Phase::Aborted => ("ABORTED", Color::Gray),
    };
    let status = format!(
        " {state}  ·  elapsed {}  ·  {} events",
        app.elapsed(),
        app.events.len()
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(status, Style::new().fg(color)))),
        rows[1],
    );
}

fn v2_select_lines(app: &App, lines: &mut Vec<Line<'static>>) {
    let v = app.verdict.as_ref().unwrap();
    lines.push(Line::from(Span::styled(
        format!(" ⛔ DECISION REQUIRED -- {STATEMENT}"),
        Style::new().fg(Color::LightRed).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("    {CONTEXT}"),
        Style::new().fg(Color::Gray),
    )));
    lines.push(Line::from(""));
    for (i, option) in OPTIONS.iter().enumerate() {
        let selected = i == v.highlight;
        let mut spans = vec![
            Span::styled(
                if selected { "    ❯ " } else { "      " }.to_string(),
                Style::new().fg(Color::LightRed),
            ),
            Span::styled(
                format!("{} · ", i + 1),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} -- {}", option.label, option.gist),
                Style::new().add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() }),
            ),
        ];
        if option.recommended {
            spans.push(Span::styled(" [recommended]", Style::new().fg(Color::Green)));
        }
        lines.push(Line::from(spans));
    }
    for (i, label) in ["Reject -- replan", "Abort task"].iter().enumerate() {
        let idx = OPTIONS.len() + i;
        let selected = idx == v.highlight;
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "    ❯ " } else { "      " }.to_string(),
                Style::new().fg(Color::LightRed),
            ),
            Span::styled(
                format!("{} · ", idx + 1),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(label.to_string(), Style::new().fg(Color::Gray)),
        ]));
    }
    if app.v2_detail && v.highlight < OPTIONS.len() {
        let option = &OPTIONS[v.highlight];
        lines.push(Line::from(""));
        for tradeoff in option.tradeoffs {
            lines.push(Line::from(Span::styled(
                format!("      + {tradeoff}"),
                Style::new().fg(Color::Gray),
            )));
        }
        for risk in option.risks {
            lines.push(Line::from(Span::styled(
                format!("      ⚠ {risk}"),
                Style::new().fg(Color::Yellow),
            )));
        }
        lines.push(Line::from(Span::styled(
            format!("      → if chosen: {}", option.consequences),
            Style::new().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
    }
    lines.push(Line::from(Span::styled(
        format!("    affected: {} · record: {ARTIFACT}", AFFECTED.join(" · ")),
        Style::new().fg(Color::Blue),
    )));
    lines.push(Line::from(Span::styled(
        "    ↑↓ move · 1-5 select · enter confirm · d details · h journal",
        Style::new().fg(Color::DarkGray),
    )));
}

fn v2_ack_lines(v: &crate::script::Verdict, lines: &mut Vec<Line<'static>>) {
    for (i, warning) in WARNINGS.iter().enumerate() {
        if i < v.acked {
            lines.push(Line::from(Span::styled(
                format!("    ✔ acknowledged: {warning}"),
                Style::new().fg(Color::Green),
            )));
        } else if i == v.acked {
            lines.push(Line::from(Span::styled(
                format!("    ⚠ ACKNOWLEDGE ({} of {}) -- {warning}", i + 1, WARNINGS.len()),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "      space -- explicit acknowledgment (journaled)",
                Style::new().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("      {warning}"),
                Style::new().fg(Color::DarkGray),
            )));
        }
    }
}

fn v2_rationale_lines(v: &crate::script::Verdict, lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(vec![
        Span::styled("    > amendments / rationale (optional): ", Style::new().fg(Color::Cyan)),
        Span::styled(v.rationale.clone(), Style::new()),
        Span::styled("█", Style::new().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(Span::styled(
        "      enter -- submit · empty accepts the choice as-is",
        Style::new().fg(Color::DarkGray),
    )));
}

fn v2_block_lines(lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(Span::styled(
        format!(" ⛔ HARD BLOCK -- {BLOCK_ATTEMPT}"),
        Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("    {BLOCK_REASON}"),
        Style::new().fg(Color::Red),
    )));
    lines.push(Line::from(Span::styled(
        format!("    steer: {BLOCK_STEER}"),
        Style::new().fg(Color::Yellow),
    )));
    lines.push(Line::from(Span::styled(
        "    enter -- acknowledge (journaled)",
        Style::new().fg(Color::DarkGray),
    )));
}

// ------------------------------------------------------------------ variant 3

fn render_v3(app: &App, f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas::<3>(area);
    v3_status(app, f, rows[0]);
    let cols =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas::<2>(rows[1]);
    v3_activity(app, f, cols[0]);
    v3_panel(app, f, cols[1]);
    f.render_widget(
        Paragraph::new(Span::styled(
            " [ ] variant · space advance · h journal · ctrl+r replay · q quit",
            Style::new().fg(Color::DarkGray),
        )),
        rows[2],
    );
}

fn v3_status(app: &App, f: &mut Frame, area: Rect) {
    let (state, color) = match app.phase {
        Phase::Running => ("RUNNING", Color::Green),
        Phase::Paused => ("PAUSED -- awaiting developer", Color::Yellow),
        Phase::Blocked => ("BLOCKED", Color::Red),
        Phase::Complete => ("COMPLETE", Color::Green),
        Phase::Aborted => ("ABORTED", Color::Gray),
    };
    let spans = vec![
        Span::styled(
            " forge ",
            Style::new().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" task: payment capture retry-safe   ", Style::new()),
        Span::styled(format!(" {state} "), Style::new().fg(Color::Black).bg(color)),
        Span::styled(
            format!(
                "  elapsed {} · decisions {} · blocks {}",
                app.elapsed(),
                app.accepted_count(),
                app.block_count()
            ),
            Style::new().fg(Color::Gray),
        ),
    ];
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn v3_activity(app: &App, f: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(Line::from(Span::styled(" activity ", Style::new().fg(Color::DarkGray))))
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let height = inner.height as usize;
    let skip = app.events.len().saturating_sub(height);
    let lines: Vec<Line> = app.events.iter().skip(skip).map(event_line).collect();
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn v3_panel(app: &App, f: &mut Frame, area: Rect) {
    let border = match app.phase {
        Phase::Paused => Color::Yellow,
        Phase::Blocked => Color::Red,
        Phase::Complete => Color::Green,
        _ => Color::DarkGray,
    };
    let block = Block::bordered()
        .title(Line::from(Span::styled(
            " decisions ",
            Style::new().fg(border).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::new().fg(border));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    match app.phase {
        Phase::Paused => {
            let v = app.verdict.as_ref().unwrap();
            match v.stage {
                VStage::Selecting => {
                    lines.push(Line::from(Span::styled(
                        STATEMENT.to_string(),
                        Style::new().add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));
                    push_option_lines(&mut lines, v.highlight, true);
                    lines.push(Line::from(""));
                    if v.highlight < OPTIONS.len() {
                        let option = &OPTIONS[v.highlight];
                        for tradeoff in option.tradeoffs {
                            lines.push(Line::from(Span::styled(
                                format!("  + {tradeoff}"),
                                Style::new().fg(Color::Gray),
                            )));
                        }
                        for risk in option.risks {
                            lines.push(Line::from(Span::styled(
                                format!("  ⚠ {risk}"),
                                Style::new().fg(Color::Yellow),
                            )));
                        }
                    }
                    lines.push(Line::from(Span::styled(
                        format!("  affected: {}", AFFECTED.join(" · ")),
                        Style::new().fg(Color::Blue),
                    )));
                    lines.push(Line::from(Span::styled(
                        format!("  record: {ARTIFACT}"),
                        Style::new().fg(Color::Blue),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  ↑↓ · 1-5 · enter",
                        Style::new().fg(Color::DarkGray),
                    )));
                }
                VStage::Acking => {
                    lines.push(Line::from(Span::styled(
                        format!("  ⚠ ACKNOWLEDGMENT {}/{}", v.acked + 1, WARNINGS.len()),
                        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));
                    for (i, warning) in WARNINGS.iter().enumerate() {
                        let style = if i < v.acked {
                            Style::new().fg(Color::Green)
                        } else if i == v.acked {
                            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        } else {
                            Style::new().fg(Color::DarkGray)
                        };
                        let marker = if i < v.acked { "✔" } else if i == v.acked { "→" } else { "·" };
                        lines.push(Line::from(Span::styled(format!("  {marker} {warning}"), style)));
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  space -- acknowledge (journaled)",
                        Style::new().fg(Color::DarkGray),
                    )));
                }
                VStage::Rationale => {
                    lines.push(Line::from(Span::styled(
                        "  amendments / rationale (optional)",
                        Style::new().fg(Color::Cyan),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("  ❯ ", Style::new().fg(Color::Yellow)),
                        Span::styled(v.rationale.clone(), Style::new()),
                        Span::styled("█", Style::new().fg(Color::Yellow)),
                    ]));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  enter -- submit",
                        Style::new().fg(Color::DarkGray),
                    )));
                }
            }
        }
        Phase::Blocked => {
            lines.push(Line::from(Span::styled(
                "  ⛔ HARD BLOCK",
                Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  attempted  {BLOCK_ATTEMPT}"),
                Style::new(),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  reason     {BLOCK_REASON}"),
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  steer      {BLOCK_STEER}"),
                Style::new().fg(Color::Yellow),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  enter -- acknowledge",
                Style::new().fg(Color::DarkGray),
            )));
        }
        Phase::Complete => {
            lines.push(Line::from(Span::styled(
                "  ✔ task complete",
                Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  record -- {RECORD_PATH}"),
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                "  grant released -- task-scoped",
                Style::new().fg(Color::Gray),
            )));
        }
        _ => {
            lines.push(Line::from(Span::styled(
                "  no open decision",
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  authority -- routine work auto-allows",
                Style::new().fg(Color::DarkGray),
            )));
            if app.accepted_count() > 0 {
                lines.push(Line::from(Span::styled(
                    "  last -- TDD-004 accepted (idempotency table)",
                    Style::new().fg(Color::Green),
                )));
                lines.push(Line::from(Span::styled(
                    "  grant -- src/payments/** · migrations/**",
                    Style::new().fg(Color::Green),
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  h -- open the task journal",
                Style::new().fg(Color::DarkGray),
            )));
        }
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ------------------------------------------------------------------ input

fn selecting(app: &mut App, key: KeyEvent, allow_detail: bool) -> bool {
    let n = OPTIONS.len() + 2;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            let v = app.verdict.as_mut().unwrap();
            v.highlight = (v.highlight + n - 1) % n;
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let v = app.verdict.as_mut().unwrap();
            v.highlight = (v.highlight + 1) % n;
            true
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(d) = c.to_digit(10) {
                let idx = d as usize;
                if idx >= 1 && idx <= n {
                    app.choose(choice_at(idx - 1));
                }
            }
            true
        }
        KeyCode::Enter => {
            let idx = app.verdict.as_ref().unwrap().highlight;
            app.choose(choice_at(idx));
            true
        }
        KeyCode::Char('d') if allow_detail => {
            app.v2_detail = !app.v2_detail;
            true
        }
        _ => false,
    }
}

fn rationale_input(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => {
            app.submit_rationale();
            true
        }
        KeyCode::Backspace => {
            if let Some(v) = app.verdict.as_mut() {
                v.rationale.pop();
            }
            true
        }
        KeyCode::Char(c) => {
            if let Some(v) = app.verdict.as_mut() {
                v.rationale.push(c);
            }
            true
        }
        _ => false,
    }
}

fn handle_common(app: &mut App, key: KeyEvent, allow_detail: bool) -> bool {
    match app.phase {
        Phase::Paused => {
            let stage = app.verdict.as_ref().map(|v| v.stage);
            match stage {
                Some(VStage::Selecting) => selecting(app, key, allow_detail),
                Some(VStage::Acking) => match key.code {
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        app.acknowledge();
                        true
                    }
                    _ => false,
                },
                Some(VStage::Rationale) => rationale_input(app, key),
                None => false,
            }
        }
        Phase::Blocked => match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => {
                app.acknowledge_block();
                true
            }
            _ => false,
        },
        Phase::Running => match key.code {
            KeyCode::Char(' ') => {
                app.advance();
                true
            }
            _ => false,
        },
        _ => false,
    }
}

pub fn handle_v1(app: &mut App, key: KeyEvent) -> bool {
    handle_common(app, key, false)
}

pub fn handle_v2(app: &mut App, key: KeyEvent) -> bool {
    handle_common(app, key, true)
}

pub fn handle_v3(app: &mut App, key: KeyEvent) -> bool {
    handle_common(app, key, false)
}

// ------------------------------------------------------------------ snapshot

pub fn print_snapshot() {
    let states = [
        "running",
        "paused-select",
        "paused-ack",
        "paused-rationale",
        "blocked",
        "complete",
    ];
    for variant in 0..3 {
        for state in states {
            let mut app = App::new_with_variant(variant);
            seed(&mut app, state);
            let out = render_to_string(&app, 104, 32);
            println!(
                "=============== variant {} ({}) · {} ===============",
                variant + 1,
                variant_name(variant),
                state
            );
            println!("{out}");
        }
    }
}

fn seed(app: &mut App, state: &str) {
    while app.phase == Phase::Running && app.beat < 7 {
        app.advance();
    }
    match state {
        "running" => {}
        "paused-select" => {
            app.advance();
        }
        "paused-ack" => {
            app.advance();
            app.choose(Choice::Option(1));
        }
        "paused-rationale" => {
            app.advance();
            app.choose(Choice::Option(1));
            app.acknowledge();
            app.acknowledge();
            if let Some(v) = app.verdict.as_mut() {
                v.rationale = "Cap retries at 3; keep the 24h replay window".into();
            }
        }
        "blocked" => {
            app.advance();
            app.choose(Choice::Option(1));
            app.acknowledge();
            app.acknowledge();
            app.submit_rationale();
            while app.phase == Phase::Running {
                app.advance();
            }
        }
        "complete" => {
            app.advance();
            app.choose(Choice::Option(1));
            app.acknowledge();
            app.acknowledge();
            app.submit_rationale();
            while app.phase == Phase::Running {
                app.advance();
            }
            app.acknowledge_block();
            while app.phase == Phase::Running {
                app.advance();
            }
        }
        _ => {}
    }
}

fn render_to_string(app: &App, width: u16, height: u16) -> String {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| render_app(f, app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}
