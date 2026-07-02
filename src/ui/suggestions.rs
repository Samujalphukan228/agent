use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::commands::{self, CommandSuggestion};

use super::theme::*;
use super::{AppState, Hitboxes};

pub fn visible(state: &AppState) -> bool {
    state.can_edit_input()
        && state.input.lines().next().unwrap_or("").starts_with('/')
        && !state.suggestion_matches.is_empty()
}

pub fn height(state: &AppState) -> u16 {
    if !visible(state) {
        return 0;
    }
    let rows = state.suggestion_matches.len().min(5) as u16;
    rows + 2
}

pub fn refresh_matches_first_line(state: &mut AppState, first_line: &str) {
    state.suggestion_matches = commands::matching_suggestions(first_line)
        .into_iter()
        .map(|s| s.cmd)
        .collect();
    if state.suggestion_pick >= state.suggestion_matches.len() {
        state.suggestion_pick = 0;
    }
}

pub fn draw(frame: &mut Frame, state: &mut AppState, area: Rect, hitboxes: &mut Hitboxes) {
    hitboxes.suggestion_rows.clear();

    if area.height < 2 || !visible(state) {
        hitboxes.suggestions = None;
        return;
    }

    let matches: Vec<&CommandSuggestion> = state
        .suggestion_matches
        .iter()
        .filter_map(|cmd| commands::SUGGESTIONS.iter().find(|s| s.cmd == *cmd))
        .collect();

    let count = matches.len();
    let accent = mode_accent(state.mode);

    let block = Block::default()
        .title(format!(" {ARROW} COMMANDS · {count} "))
        .title_style(accent_fg(accent))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border())
        .style(elevated());

    let inner = block.inner(area);
    hitboxes.suggestions = Some(area);

    let max_rows = inner.height.max(1) as usize;
    let mut lines = Vec::new();

    for (row, sug) in matches.iter().enumerate().take(max_rows) {
        let picked = row == state.suggestion_pick;
        if picked {
            lines.push(Line::from(vec![
                Span::styled(RAIL, accent_fg(accent)),
                Span::styled(format!(" {:<14}", sug.usage), selected_glow()),
                Span::styled(sug.desc, dim()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:<14}", sug.usage), cmd()),
                Span::styled(sug.desc, ghost()),
            ]));
        }

        hitboxes.suggestion_rows.push((
            Rect {
                x: inner.x,
                y: inner.y.saturating_add(row as u16),
                width: inner.width,
                height: 1,
            },
            row,
        ));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
