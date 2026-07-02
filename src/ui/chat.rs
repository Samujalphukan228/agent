use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::mode::AgentMode;

use super::theme::*;
use super::{AppState, ChatEntry, Hitboxes};

const TOOL_LINES: usize = 14;

#[derive(Clone, Debug)]
pub struct HistoryAnchor {
    pub line: usize,
    pub preview: String,
}

pub fn draw(frame: &mut Frame, state: &mut AppState, area: Rect, hitboxes: &mut Hitboxes) {
    hitboxes.chat = area;

    let (lines, anchors) = build_chat_lines(state);
    state.history_anchors = anchors;
    if state.sidebar_pick >= state.history_anchors.len() && !state.history_anchors.is_empty() {
        state.sidebar_pick = state.history_anchors.len() - 1;
    }

    let total = lines.len().max(1);
    state.last_line_count = total;

    let mode = state.mode;
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(mode_accent(mode)))
        .style(bg());

    let inner = block.inner(area);
    hitboxes.chat_inner = inner;
    state.chat_viewport = inner.height.max(1) as usize;

    if state.following {
        state.scroll = state.max_scroll();
    }
    state.scroll = state.scroll.min(state.max_scroll());

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .style(bg())
            .scroll((state.scroll as u16, 0)),
        area,
    );

    if total > state.chat_viewport {
        let mut sb = ScrollbarState::new(total)
            .position(state.scroll)
            .viewport_content_length(state.chat_viewport);

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_symbol("▐")
                .track_symbol(Some("│"))
                .style(border()),
            inner,
            &mut sb,
        );
    }
}

pub fn build_chat_lines(state: &AppState) -> (Vec<Line<'static>>, Vec<HistoryAnchor>) {
    let show_welcome = state.entries.is_empty() && !state.is_thinking && !state.awaiting_approval;
    let mode = state.mode;
    let spinner = state.spinner_frame;
    let awaiting = state.awaiting_approval;
    let approval = state.approval_summary.clone();
    let thinking = state.is_thinking;
    let has_token = state.has_token;

    let mut lines = Vec::new();
    let mut anchors = Vec::new();

    if show_welcome {
        render_welcome(&mut lines, mode, has_token);
    }

    let mut idx = 0;
    while idx < state.entries.len() {
        match &state.entries[idx] {
            ChatEntry::User { content } => {
                let preview = content.lines().next().unwrap_or(content).trim().to_string();
                if !preview.is_empty() && !preview.starts_with('/') {
                    anchors.push(HistoryAnchor {
                        line: lines.len(),
                        preview,
                    });
                }

                let turn_start = lines.len();
                render_user_card(&mut lines, content);

                idx += 1;
                let mut got_response = false;
                while idx < state.entries.len() {
                    match &state.entries[idx] {
                        ChatEntry::Assistant { content } => {
                            render_assistant_card(&mut lines, content, mode);
                            got_response = true;
                            idx += 1;
                            break;
                        }
                        entry => {
                            render_inline_entry(&mut lines, entry, mode, spinner);
                            idx += 1;
                        }
                    }
                }

                if !got_response {
                    lines.push(card_close());
                }

                if lines.len() > turn_start {
                    lines.push(Line::from(""));
                }
            }
            entry => {
                render_standalone_entry(&mut lines, entry, mode, spinner);
                idx += 1;
            }
        }
    }

    if thinking && !state.entries.iter().any(ChatEntry::is_running) {
        lines.push(Line::from(""));
        lines.push(status_banner(
            ACCENT_BUSY,
            &format!("thinking {}", spin(spinner)),
        ));
    }

    if awaiting {
        lines.push(Line::from(""));
        let summary = approval.unwrap_or_else(|| "action required".to_string());
        lines.push(status_banner(ACCENT_WARN, &format!("approve? {summary}")));
    }

    (lines, anchors)
}

fn render_welcome(lines: &mut Vec<Line<'static>>, mode: AgentMode, has_token: bool) {
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(PAD, ghost()),
        Span::styled(DIAMOND, accent(mode)),
        Span::raw("  "),
        Span::styled(mode_title(mode), accent(mode)),
        Span::styled(format!(" {DOT} "), ghost()),
        Span::styled("agent", muted()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(PAD, ghost()),
        Span::styled("local agent runtime", muted()),
    ]));
    lines.push(Line::from(""));
    lines.push(card_open("WELCOME", mode_accent(mode)));
    lines.push(card_body(
        "Zero-config terminal agent. Full machine access via tools.",
        dim(),
    ));
    lines.push(card_body(
        "Type / for commands. ctrl+enter for newline.",
        dim(),
    ));
    lines.push(Line::from(""));
    if !has_token {
        lines.push(card_body(
            "Start: /token  or wait for the setup dialog",
            dim(),
        ));
    } else {
        lines.push(card_body(
            format!("Mode: {} — ask anything", mode.label()),
            dim(),
        ));
    }
    lines.push(card_body(
        "/help  /settings  /history  /mode god|base  /clear",
        dim(),
    ));
    lines.push(card_close());
    lines.push(Line::from(""));
}

fn card_open(label: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(PAD, ghost()),
        Span::styled("╭─ ", border()),
        Span::styled(label.to_string(), accent_fg(color)),
        Span::styled(" ", border()),
        Span::styled("─".repeat(24), border()),
    ])
}

fn card_bridge(label: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(PAD, ghost()),
        Span::styled("╰─╮", border()),
        Span::styled("  ", border()),
        Span::styled(label.to_string(), accent_fg(color)),
        Span::styled(" ", border()),
        Span::styled("─".repeat(20), border()),
    ])
}

fn card_close() -> Line<'static> {
    Line::from(vec![
        Span::styled(PAD, ghost()),
        Span::styled("╰", border()),
        Span::styled("─".repeat(30), border()),
        Span::styled("╯", border()),
    ])
}

fn card_body(text: impl Into<String>, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(PAD, ghost()),
        Span::styled("│ ", border()),
        Span::styled(text.into(), style),
    ])
}

fn status_banner(color: Color, text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(PAD, ghost()),
        Span::styled(RAIL, accent_fg(color)),
        Span::raw(" "),
        Span::styled(text.to_string(), accent_fg(color)),
    ])
}

fn render_user_card(lines: &mut Vec<Line<'static>>, content: &str) {
    lines.push(card_open("YOU", ACCENT_USER));
    if content.is_empty() {
        lines.push(card_body("", question_text()));
    } else {
        for line in content.lines() {
            lines.push(card_body(line, question_text()));
        }
    }
}

fn render_assistant_card(lines: &mut Vec<Line<'static>>, content: &str, mode: AgentMode) {
    lines.push(card_bridge("AGENT", mode_accent(mode)));
    if content.is_empty() {
        lines.push(card_body("", answer_text()));
    } else {
        for line in content.lines() {
            lines.push(card_body(line, answer_text()));
        }
    }
    lines.push(card_close());
}

fn render_inline_entry(
    lines: &mut Vec<Line<'static>>,
    entry: &ChatEntry,
    _mode: AgentMode,
    spinner: usize,
) {
    match entry {
        ChatEntry::Tool {
            name,
            args,
            result,
            error,
        } => {
            let label = if result.is_none() && error.is_none() {
                format!("{name} {}", spin(spinner))
            } else {
                name.clone()
            };
            lines.push(card_body(format!("{ARROW} {label}"), tool_text()));
            if !args.is_empty() {
                lines.push(card_body(format!("   {args}"), tool_text()));
            }
            if let Some(out) = result {
                if !out.trim().is_empty() {
                    push_tool_output(lines, out);
                }
            } else if let Some(err) = error {
                lines.push(card_body(err.clone(), status_err()));
            }
        }
        ChatEntry::System { content } => {
            for line in content.lines() {
                lines.push(card_body(line, muted()));
            }
        }
        ChatEntry::Error { message } => {
            lines.push(card_body(format!("error: {message}"), status_err()));
        }
        ChatEntry::User { .. } | ChatEntry::Assistant { .. } => {}
    }
}

fn render_standalone_entry(
    lines: &mut Vec<Line<'static>>,
    entry: &ChatEntry,
    mode: AgentMode,
    spinner: usize,
) {
    lines.push(Line::from(""));
    match entry {
        ChatEntry::Assistant { content } => {
            lines.push(card_open("AGENT", mode_accent(mode)));
            for line in content.lines() {
                lines.push(card_body(line, answer_text()));
            }
            lines.push(card_close());
        }
        ChatEntry::System { content } => {
            for line in content.lines() {
                lines.push(Line::from(vec![
                    Span::styled(PAD, ghost()),
                    Span::styled(format!(" {DOT} "), muted()),
                    Span::styled(line.to_string(), dim()),
                ]));
            }
        }
        ChatEntry::Tool {
            name,
            args,
            result,
            error,
        } => {
            let label = if result.is_none() && error.is_none() {
                format!("{name} {}", spin(spinner))
            } else {
                name.clone()
            };
            lines.push(card_open(&label.to_uppercase(), ACCENT_TOOL));
            if !args.is_empty() {
                lines.push(card_body(format!("{ARROW} {args}"), tool_text()));
            }
            if let Some(out) = result {
                if !out.trim().is_empty() {
                    push_tool_output(lines, out);
                }
            } else if let Some(err) = error {
                lines.push(card_body(err.clone(), status_err()));
            }
            lines.push(card_close());
        }
        ChatEntry::Error { message } => {
            lines.push(card_open("ERROR", ACCENT_ERR));
            lines.push(card_body(message.clone(), status_err()));
            lines.push(card_close());
        }
        ChatEntry::User { .. } => {}
    }
}

fn push_tool_output(lines: &mut Vec<Line<'static>>, output: &str) {
    let rows: Vec<&str> = output.lines().collect();
    let shown = rows.len().min(TOOL_LINES);

    for row in &rows[..shown] {
        lines.push(card_body((*row).to_string(), ghost()));
    }
    if rows.len() > TOOL_LINES {
        lines.push(card_body(
            format!("… {} more lines", rows.len() - TOOL_LINES),
            ghost(),
        ));
    }
}
