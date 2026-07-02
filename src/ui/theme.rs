use ratatui::style::{Color, Modifier, Style};

use crate::mode::AgentMode;

// void palette
pub const VOID: Color = Color::Rgb(8, 8, 8);
pub const BG: Color = Color::Rgb(12, 12, 12);
pub const PANEL: Color = Color::Rgb(20, 20, 20);
pub const INSET: Color = Color::Rgb(14, 14, 14);
pub const ELEVATED: Color = Color::Rgb(26, 26, 26);
pub const SELECTED_GLOW: Color = Color::Rgb(48, 48, 48);

pub const TEXT: Color = Color::Rgb(235, 235, 235);
pub const BRIGHT: Color = Color::Rgb(255, 255, 255);
pub const DIM: Color = Color::Rgb(160, 160, 160);
pub const MUTED: Color = Color::Rgb(100, 100, 100);
pub const GHOST: Color = Color::Rgb(58, 58, 58);

pub const RULE: Color = Color::Rgb(34, 34, 34);
pub const BORDER: Color = Color::Rgb(48, 48, 48);
pub const BORDER_FOCUS: Color = Color::Rgb(130, 130, 130);

pub const ACCENT_GOD: Color = Color::Rgb(245, 245, 245);
pub const ACCENT_BASE: Color = Color::Rgb(212, 168, 88);
pub const ACCENT_OK: Color = Color::Rgb(110, 210, 150);
pub const ACCENT_BUSY: Color = Color::Rgb(130, 175, 255);
pub const ACCENT_WARN: Color = Color::Rgb(230, 175, 90);
pub const ACCENT_ERR: Color = Color::Rgb(235, 105, 105);
pub const ACCENT_TOOL: Color = Color::Rgb(160, 195, 255);
pub const ACCENT_USER: Color = Color::Rgb(130, 195, 255);

pub const RAIL: &str = "▎";
pub const PROMPT: &str = "› ";
pub const DOT: &str = "·";
pub const PAD: &str = "  ";
pub const DIAMOND: &str = "◆";
pub const ARROW: &str = "▸";

pub const SPIN: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
pub const PULSE: &[&str] = &["○", "◔", "◑", "◕", "●", "◕", "◑", "◔"];

pub fn spin(frame: usize) -> &'static str {
    SPIN[frame % SPIN.len()]
}

pub fn pulse(frame: usize) -> &'static str {
    PULSE[frame % PULSE.len()]
}

pub fn mode_accent(mode: AgentMode) -> Color {
    match mode {
        AgentMode::God => ACCENT_GOD,
        AgentMode::Base => ACCENT_BASE,
    }
}

pub fn mode_title(mode: AgentMode) -> &'static str {
    match mode {
        AgentMode::God => "GOD",
        AgentMode::Base => "BASE",
    }
}

pub fn bg() -> Style {
    Style::default().bg(BG)
}

pub fn void() -> Style {
    Style::default().bg(VOID)
}

pub fn panel() -> Style {
    Style::default().bg(PANEL)
}

pub fn inset() -> Style {
    Style::default().bg(INSET)
}

pub fn elevated() -> Style {
    Style::default().bg(ELEVATED)
}

pub fn text() -> Style {
    Style::default().fg(TEXT)
}

pub fn bright() -> Style {
    Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD)
}

pub fn dim() -> Style {
    Style::default().fg(DIM)
}

pub fn muted() -> Style {
    Style::default().fg(MUTED)
}

pub fn ghost() -> Style {
    Style::default().fg(GHOST)
}

pub fn selected_glow() -> Style {
    Style::default().fg(BRIGHT).bg(SELECTED_GLOW)
}

pub fn border() -> Style {
    Style::default().fg(BORDER)
}

pub fn border_focus() -> Style {
    Style::default().fg(BORDER_FOCUS)
}

pub fn rule() -> Style {
    Style::default().fg(RULE)
}

pub fn prompt() -> Style {
    Style::default().fg(MUTED)
}

pub fn cmd() -> Style {
    Style::default().fg(BRIGHT)
}

pub fn accent(mode: AgentMode) -> Style {
    Style::default()
        .fg(mode_accent(mode))
        .add_modifier(Modifier::BOLD)
}

pub fn accent_fg(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

pub fn cursor_block(blink: bool) -> Style {
    if blink {
        Style::default().fg(INSET).bg(BRIGHT)
    } else {
        Style::default().fg(BRIGHT).bg(ELEVATED)
    }
}

pub fn status_ok() -> Style {
    Style::default().fg(ACCENT_OK)
}

pub fn status_busy() -> Style {
    Style::default().fg(ACCENT_BUSY)
}

pub fn status_warn() -> Style {
    Style::default().fg(ACCENT_WARN)
}

pub fn status_err() -> Style {
    Style::default().fg(ACCENT_ERR)
}

pub fn overlay_scrim() -> Style {
    Style::default().bg(VOID).fg(GHOST)
}

pub fn overlay_body() -> Style {
    Style::default().bg(PANEL).fg(TEXT)
}

pub fn overlay_field() -> Style {
    Style::default().bg(INSET).fg(TEXT)
}

pub fn overlay_label() -> Style {
    Style::default().fg(MUTED)
}

pub fn overlay_value() -> Style {
    Style::default().fg(DIM)
}

pub fn overlay_active() -> Style {
    Style::default().fg(ACCENT_OK)
}

pub fn overlay_muted_border() -> Style {
    Style::default().fg(RULE)
}

pub fn question_text() -> Style {
    Style::default().fg(DIM)
}

pub fn answer_text() -> Style {
    Style::default().fg(TEXT)
}

pub fn tool_text() -> Style {
    Style::default().fg(MUTED)
}
