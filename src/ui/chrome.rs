use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::theme::*;
use super::AppState;

pub fn draw_rail(frame: &mut Frame, state: &AppState, area: Rect) {
    let accent = mode_accent(state.mode);
    let ch = if state.is_thinking {
        spin(state.spinner_frame)
    } else if state.awaiting_approval {
        "!"
    } else {
        "┃"
    };

    let h = area.height.max(1);
    let mut lines = Vec::with_capacity(h as usize);
    for i in 0..h {
        let style = if i == 0 || i == h - 1 {
            Style::default().fg(ghost().fg.unwrap_or(GHOST))
        } else {
            Style::default().fg(accent)
        };
        let c = if i == 1 { DIAMOND } else { ch };
        lines.push(Line::from(Span::styled(c, style)));
    }

    frame.render_widget(Paragraph::new(lines).style(void()), area);
}

pub fn draw_header(frame: &mut Frame, state: &AppState, area: Rect) {
    let cols = Layout::horizontal([Constraint::Min(10), Constraint::Length(22)]).split(area);

    let mode = state.mode;
    let mode_label = mode.label().to_uppercase();
    let token_ok = state.has_token;

    let title = mode_title(mode);

    let left = Line::from(vec![
        Span::styled(DIAMOND, accent(mode)),
        Span::raw(" "),
        Span::styled(title, accent(mode)),
        Span::styled(format!(" {DOT} "), ghost()),
        Span::styled("agent", muted()),
        Span::raw("  "),
        Span::styled(
            format!(" {mode_label} "),
            Style::default()
                .fg(mode_accent(mode))
                .bg(PANEL)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            if token_ok { "token ok" } else { "no token" },
            if token_ok { status_ok() } else { status_warn() },
        ),
    ]);

    let (status_label, status_style, icon) = if state.awaiting_approval {
        ("APPROVE", status_warn(), "!")
    } else if state.is_thinking {
        ("BUSY", status_busy(), spin(state.spinner_frame))
    } else {
        ("READY", status_ok(), pulse(state.spinner_frame))
    };

    let right = Line::from(vec![
        Span::styled(icon, status_style),
        Span::styled(format!(" {status_label} "), status_style),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border())
        .style(panel());

    frame.render_widget(Paragraph::new(left).block(block.clone()), cols[0]);
    frame.render_widget(
        Paragraph::new(right)
            .alignment(Alignment::Right)
            .block(block.style(panel())),
        cols[1],
    );
}

pub fn draw_footer(frame: &mut Frame, state: &AppState, area: Rect) {
    let line = if state.has_overlay() {
        footer_spans(
            &[("enter", "confirm"), ("esc", "close")],
            state.overlay_hint_esc(),
        )
    } else if state.awaiting_approval {
        footer_spans(&[("y", "approve"), ("n", "deny"), ("esc", "cancel")], false)
    } else if super::suggestions::visible(state) {
        footer_spans(
            &[("up/down", "pick"), ("tab", "fill"), ("enter", "run")],
            false,
        )
    } else if state.sidebar_open {
        footer_spans(
            &[("up/down", "pick"), ("enter", "jump"), ("esc", "close")],
            false,
        )
    } else if !state.input.is_empty() {
        footer_spans(
            &[
                ("ctrl+enter", "newline"),
                ("enter", "send"),
                ("up/down", "move"),
            ],
            false,
        )
    } else {
        footer_spans(
            &[
                ("/history", "sidebar"),
                ("h", "toggle"),
                ("^b", "toggle"),
                ("/", "cmds"),
                ("enter", "send"),
            ],
            false,
        )
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(rule())
        .style(inset());

    frame.render_widget(Paragraph::new(line).block(block), area);
}

fn footer_spans(keys: &[(&str, &str)], show_esc: bool) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (key, label)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", ghost()));
        }
        spans.push(Span::styled(key.to_string(), bright()));
        spans.push(Span::styled(format!(" {label}"), ghost()));
    }
    if show_esc {
        spans.push(Span::styled("   ", ghost()));
        spans.push(Span::styled("esc", muted()));
        spans.push(Span::styled(" close", ghost()));
    }
    Line::from(spans)
}

pub fn draw_spacer(frame: &mut Frame, area: Rect) {
    frame.render_widget(Paragraph::new("").style(bg()), area);
}
