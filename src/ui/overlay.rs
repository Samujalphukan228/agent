use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::mode::AgentMode;

use super::theme::*;
use super::AppState;

const SETTINGS_ROWS: usize = 3;
const MASK_CHAR: char = '*';

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Overlay {
    #[default]
    None,
    TokenEntry {
        input: String,
        cursor: usize,
        error: Option<String>,
        dismissable: bool,
    },
    Settings {
        pick: usize,
        masked_token: String,
        mode: AgentMode,
        config_path: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum OverlayAction {
    None,
    Close,
    SaveToken(String),
    SetMode(AgentMode),
    OpenToken,
}

pub fn draw(frame: &mut Frame, state: &AppState, area: Rect) {
    draw_backdrop(frame, area);

    match &state.overlay {
        Overlay::None => {}
        Overlay::TokenEntry { .. } => draw_token(frame, state, area),
        Overlay::Settings { .. } => draw_settings(frame, state, area),
    }
}

fn draw_backdrop(frame: &mut Frame, area: Rect) {
    frame.render_widget(Block::default().style(overlay_scrim()), area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width.saturating_sub(4)).max(36);
    let h = height.min(area.height.saturating_sub(4)).max(8);
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w, h)
}

fn modal_shell<'a>(title: &'a str, mode: AgentMode) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border())
        .title(format!(" {DIAMOND} {title} "))
        .title_style(accent(mode))
        .style(overlay_body())
}

fn modal_divider(width: u16) -> Paragraph<'static> {
    let w = width.max(8) as usize;
    Paragraph::new(Span::styled("─".repeat(w), overlay_muted_border())).style(overlay_body())
}

fn modal_footer(text: &str) -> Paragraph<'static> {
    let footer_block = Block::default()
        .borders(Borders::TOP)
        .border_style(overlay_muted_border())
        .style(inset());

    Paragraph::new(Line::from(vec![
        Span::styled(DOT, ghost()),
        Span::styled(format!(" {text} "), ghost()),
        Span::styled(DOT, ghost()),
    ]))
    .alignment(Alignment::Center)
    .block(footer_block)
}

fn field_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border())
        .style(overlay_field())
}

fn draw_token(frame: &mut Frame, state: &AppState, area: Rect) {
    let Overlay::TokenEntry {
        input,
        cursor,
        error,
        dismissable,
    } = &state.overlay
    else {
        return;
    };

    let mode = state.mode;
    let popup = centered_rect(54, 14, area);
    let shell = modal_shell("API TOKEN", mode);
    let inner = shell.inner(popup);
    frame.render_widget(shell, popup);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(2),
    ])
    .split(inner);

    let headline = if *dismissable {
        "Update your Gemini API key"
    } else {
        "Connect to continue"
    };
    frame.render_widget(Paragraph::new(headline).style(text()), rows[0]);
    frame.render_widget(
        Paragraph::new("Required for the agent to respond").style(overlay_label()),
        rows[1],
    );
    frame.render_widget(modal_divider(rows[2].width), rows[2]);

    let masked: String = input.chars().map(|_| MASK_CHAR).collect();
    let show_placeholder = input.is_empty();
    let display = if show_placeholder {
        "paste your key here"
    } else {
        &masked
    };
    let cursor_pos = if show_placeholder {
        0
    } else {
        input[..(*cursor).min(input.len())].chars().count()
    };
    let input_line = masked_input_line(display, cursor_pos, state.cursor_blink, show_placeholder);

    let input_block = field_block();
    let input_inner = input_block.inner(rows[3]);
    frame.render_widget(input_block, rows[3]);
    let mut line = input_line;
    line.spans.insert(0, Span::styled(PROMPT, prompt()));
    frame.render_widget(Paragraph::new(line), input_inner);

    let err_row = rows[4];
    if let Some(err) = error {
        frame.render_widget(Paragraph::new(err.as_str()).style(status_err()), err_row);
    } else {
        frame.render_widget(
            Paragraph::new("aistudio.google.com/apikey").style(overlay_value()),
            err_row,
        );
    }

    let hint = if *dismissable {
        "enter save  ·  esc cancel"
    } else {
        "enter save to continue"
    };
    frame.render_widget(modal_footer(hint), rows[5]);
}

fn masked_input_line(
    display: &str,
    cursor: usize,
    blink: bool,
    placeholder: bool,
) -> Line<'static> {
    let style = if placeholder { ghost() } else { cmd() };
    let chars: Vec<char> = display.chars().collect();
    let cursor = cursor.min(chars.len());

    let mut spans = Vec::new();
    for (i, ch) in chars.iter().enumerate() {
        if i == cursor {
            spans.push(Span::styled(ch.to_string(), cursor_block(blink)));
        } else {
            spans.push(Span::styled(ch.to_string(), style));
        }
    }
    if cursor == chars.len() {
        spans.push(Span::styled(" ", cursor_block(blink)));
    }

    Line::from(spans)
}

fn draw_settings(frame: &mut Frame, state: &AppState, area: Rect) {
    let Overlay::Settings {
        pick,
        masked_token,
        mode,
        config_path,
    } = &state.overlay
    else {
        return;
    };

    let popup = centered_rect(56, 14, area);
    let shell = modal_shell("SETTINGS", *mode);
    let inner = shell.inner(popup);
    frame.render_widget(shell, popup);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(2),
    ])
    .split(inner);

    frame.render_widget(
        Paragraph::new("Configure agent runtime").style(overlay_label()),
        rows[0],
    );
    frame.render_widget(modal_divider(rows[1].width), rows[1]);

    let items = settings_items(masked_token, *mode);
    for (i, row) in items.iter().enumerate() {
        let line = settings_row_line(*pick == i, row, *mode);
        frame.render_widget(Paragraph::new(line), rows[2 + i]);
    }

    frame.render_widget(modal_divider(rows[5].width), rows[5]);
    frame.render_widget(
        Paragraph::new(format!("config  {config_path}")).style(overlay_value()),
        rows[6],
    );
    frame.render_widget(
        modal_footer("up/down navigate  ·  enter select  ·  esc close"),
        rows[7],
    );
}

struct SettingsRow {
    label: &'static str,
    value: String,
    hint: &'static str,
    row_kind: SettingsRowKind,
}

enum SettingsRowKind {
    Token,
    Mode(AgentMode),
}

fn settings_items(masked_token: &str, mode: AgentMode) -> [SettingsRow; SETTINGS_ROWS] {
    [
        SettingsRow {
            label: "Token",
            value: masked_token.to_string(),
            hint: "change",
            row_kind: SettingsRowKind::Token,
        },
        SettingsRow {
            label: "Mode",
            value: "base".to_string(),
            hint: if mode == AgentMode::Base {
                "active"
            } else {
                "select"
            },
            row_kind: SettingsRowKind::Mode(AgentMode::Base),
        },
        SettingsRow {
            label: "Mode",
            value: "god".to_string(),
            hint: if mode == AgentMode::God {
                "active"
            } else {
                "select"
            },
            row_kind: SettingsRowKind::Mode(AgentMode::God),
        },
    ]
}

fn settings_row_line(selected: bool, row: &SettingsRow, current: AgentMode) -> Line<'static> {
    let marker = if selected { RAIL } else { "  " };
    let marker_style = if selected { accent(current) } else { ghost() };

    let label_style = if selected { bright() } else { muted() };

    let value_style = match &row.row_kind {
        SettingsRowKind::Token => {
            if selected {
                text()
            } else {
                overlay_value()
            }
        }
        SettingsRowKind::Mode(m) => {
            if row.hint == "active" {
                accent(*m)
            } else if selected {
                dim()
            } else {
                overlay_value()
            }
        }
    };

    let hint_style = if row.hint == "active" {
        overlay_active()
    } else if selected {
        dim()
    } else {
        ghost()
    };

    Line::from(vec![
        Span::styled(marker, marker_style),
        Span::styled(format!(" {:<8}", row.label), label_style),
        Span::styled(row.value.clone(), value_style),
        Span::raw("  "),
        Span::styled(row.hint, hint_style),
    ])
}

impl AppState {
    pub fn has_overlay(&self) -> bool {
        !matches!(self.overlay, Overlay::None)
    }

    pub fn overlay_hint_esc(&self) -> bool {
        match &self.overlay {
            Overlay::None => false,
            Overlay::TokenEntry { dismissable, .. } => *dismissable,
            Overlay::Settings { .. } => true,
        }
    }

    pub fn open_token_overlay(&mut self, dismissable: bool) {
        self.overlay = Overlay::TokenEntry {
            input: String::new(),
            cursor: 0,
            error: None,
            dismissable,
        };
        self.cursor_blink = true;
    }

    pub fn open_settings_overlay(
        &mut self,
        masked_token: String,
        mode: AgentMode,
        config_path: String,
    ) {
        self.overlay = Overlay::Settings {
            pick: 0,
            masked_token,
            mode,
            config_path,
        };
    }

    pub fn close_overlay(&mut self) {
        self.overlay = Overlay::None;
    }

    pub fn handle_overlay_key(&mut self, code: crossterm::event::KeyCode) -> OverlayAction {
        match &mut self.overlay {
            Overlay::None => OverlayAction::None,
            Overlay::TokenEntry {
                input,
                cursor,
                error,
                dismissable,
            } => handle_token_key(input, cursor, error, *dismissable, code),
            Overlay::Settings { .. } => handle_settings_key(code, &mut self.overlay),
        }
    }
}

fn handle_token_key(
    input: &mut String,
    cursor: &mut usize,
    error: &mut Option<String>,
    dismissable: bool,
    code: crossterm::event::KeyCode,
) -> OverlayAction {
    use crossterm::event::KeyCode;

    match code {
        KeyCode::Esc if dismissable => {
            *error = None;
            OverlayAction::Close
        }
        KeyCode::Enter => {
            let token = input.trim().to_string();
            if token.is_empty() {
                *error = Some("Token cannot be empty".to_string());
                return OverlayAction::None;
            }
            OverlayAction::SaveToken(token)
        }
        KeyCode::Backspace if *cursor > 0 => {
            let prev = input[..*cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            input.replace_range(prev..*cursor, "");
            *cursor = prev;
            *error = None;
            OverlayAction::None
        }
        KeyCode::Left => {
            if *cursor > 0 {
                let prev = input[..*cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                *cursor = prev;
            }
            OverlayAction::None
        }
        KeyCode::Right => {
            if *cursor < input.len() {
                let next = input[*cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor + i)
                    .unwrap_or(input.len());
                *cursor = next;
            }
            OverlayAction::None
        }
        KeyCode::Home => {
            *cursor = 0;
            OverlayAction::None
        }
        KeyCode::End => {
            *cursor = input.len();
            OverlayAction::None
        }
        KeyCode::Char(c) if !c.is_control() => {
            input.insert(*cursor, c);
            *cursor += c.len_utf8();
            *error = None;
            OverlayAction::None
        }
        _ => OverlayAction::None,
    }
}

fn handle_settings_key(code: crossterm::event::KeyCode, overlay: &mut Overlay) -> OverlayAction {
    use crossterm::event::KeyCode;

    let Overlay::Settings {
        pick,
        mode: mode_ref,
        ..
    } = overlay
    else {
        return OverlayAction::None;
    };

    match code {
        KeyCode::Esc => OverlayAction::Close,
        KeyCode::Up => {
            *pick = pick.saturating_sub(1);
            OverlayAction::None
        }
        KeyCode::Down => {
            *pick = (*pick + 1).min(SETTINGS_ROWS - 1);
            OverlayAction::None
        }
        KeyCode::Enter => match *pick {
            0 => OverlayAction::OpenToken,
            1 => {
                *mode_ref = AgentMode::Base;
                OverlayAction::SetMode(AgentMode::Base)
            }
            2 => {
                *mode_ref = AgentMode::God;
                OverlayAction::SetMode(AgentMode::God)
            }
            _ => OverlayAction::None,
        },
        KeyCode::Char('k') | KeyCode::Char('K') => {
            *pick = pick.saturating_sub(1);
            OverlayAction::None
        }
        KeyCode::Char('j') | KeyCode::Char('J') => {
            *pick = (*pick + 1).min(SETTINGS_ROWS - 1);
            OverlayAction::None
        }
        _ => OverlayAction::None,
    }
}
