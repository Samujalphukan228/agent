use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::theme::*;
use super::{AppState, Hitboxes};

const OPEN_WIDTH: u16 = 32;
const TAB_WIDTH: u16 = 4;

pub fn width(state: &AppState) -> u16 {
    if state.sidebar_open {
        OPEN_WIDTH
    } else {
        TAB_WIDTH
    }
}

pub fn draw(frame: &mut Frame, state: &mut AppState, area: Rect, hitboxes: &mut Hitboxes) {
    hitboxes.sidebar_toggle = area;
    hitboxes.sidebar_rows.clear();

    if area.width == 0 {
        hitboxes.sidebar = None;
        return;
    }

    if !state.sidebar_open {
        draw_tab(frame, state, area, hitboxes);
        return;
    }

    hitboxes.sidebar = Some(area);
    let accent = mode_accent(state.mode);
    let count = state.history_anchors.len();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .title(format!(" {DIAMOND} HISTORY "))
        .title_style(accent_fg(accent))
        .style(panel());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    frame.render_widget(
        Paragraph::new(format!(
            "{count} question{}",
            if count == 1 { "" } else { "s" }
        ))
        .style(overlay_label()),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(rows[1].width.max(4) as usize),
            rule(),
        )),
        rows[1],
    );

    let list_area = rows[2];
    let viewport = list_area.height.max(1) as usize;
    state.sidebar_viewport = viewport;

    if count == 0 {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("no questions", muted())),
                Line::from(Span::styled("ask something", ghost())),
            ])
            .alignment(Alignment::Center),
            list_area,
        );
    } else {
        let max_scroll = count.saturating_sub(viewport);
        state.sidebar_scroll = state.sidebar_scroll.min(max_scroll);

        let active = state.active_history_index();
        let inner_w = list_area.width.saturating_sub(2).max(6) as usize;

        let mut lines = Vec::new();
        for (vis, anchor) in state
            .history_anchors
            .iter()
            .enumerate()
            .skip(state.sidebar_scroll)
            .take(viewport)
        {
            let row_idx = vis + state.sidebar_scroll;
            let selected = row_idx == state.sidebar_pick;
            let in_view = active == Some(row_idx);
            lines.push(history_row_line(
                row_idx + 1,
                &anchor.preview,
                inner_w,
                selected,
                in_view,
                accent,
            ));

            hitboxes.sidebar_rows.push((
                Rect {
                    x: list_area.x,
                    y: list_area.y.saturating_add(vis as u16),
                    width: list_area.width,
                    height: 1,
                },
                row_idx,
            ));
        }

        frame.render_widget(Paragraph::new(lines), list_area);
    }

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(DOT, ghost()),
            Span::styled(" esc close ", ghost()),
            Span::styled(DOT, ghost()),
        ]))
        .alignment(Alignment::Center)
        .style(inset()),
        rows[3],
    );
}

fn draw_tab(frame: &mut Frame, state: &AppState, area: Rect, hitboxes: &mut Hitboxes) {
    hitboxes.sidebar = None;
    let count = state.history_anchors.len();
    let accent = mode_accent(state.mode);

    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(accent))
        .style(inset());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let count_label = if count > 0 {
        format!("{count}")
    } else {
        "·".to_string()
    };

    let lines = vec![
        Line::from(Span::styled("☰", accent_fg(accent))),
        Line::from(Span::styled("H", muted())),
        Line::from(Span::styled(count_label, accent_fg(accent))),
        Line::from(Span::styled("»", ghost())),
    ];
    hitboxes.sidebar_toggle = inner;
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn history_row_line(
    num: usize,
    preview: &str,
    width: usize,
    selected: bool,
    in_view: bool,
    accent: ratatui::style::Color,
) -> Line<'static> {
    let preview_text = truncate_preview(preview, width.saturating_sub(8));
    let badge = format!("{num:>2}");

    if selected {
        return Line::from(vec![
            Span::styled(RAIL, accent_fg(accent)),
            Span::styled(
                format!(" {badge} "),
                Style::default().fg(BRIGHT).bg(SELECTED_GLOW),
            ),
            Span::styled(preview_text, Style::default().fg(BRIGHT).bg(SELECTED_GLOW)),
        ]);
    }

    let badge_style = if in_view { accent_fg(accent) } else { muted() };
    let preview_style = if in_view { text() } else { overlay_value() };
    let marker = if in_view { ARROW } else { " " };

    Line::from(vec![
        Span::styled(marker, if in_view { accent_fg(accent) } else { ghost() }),
        Span::styled(format!(" {badge} "), badge_style),
        Span::styled(preview_text, preview_style),
    ])
}

fn truncate_preview(s: &str, max: usize) -> String {
    let one_line = s.lines().next().unwrap_or(s).trim();
    if one_line.chars().count() <= max {
        return one_line.to_string();
    }
    let mut out = String::new();
    for (i, ch) in one_line.chars().enumerate() {
        if i + 1 >= max.saturating_sub(1) {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

impl AppState {
    pub fn open_sidebar(&mut self) {
        self.sidebar_open = true;
        self.sidebar_pick = self
            .active_history_index()
            .unwrap_or_else(|| self.history_anchors.len().saturating_sub(1));
        self.ensure_sidebar_pick_visible();
    }

    pub fn toggle_sidebar(&mut self) {
        if self.sidebar_open {
            self.close_sidebar();
        } else {
            self.open_sidebar();
        }
    }

    pub fn close_sidebar(&mut self) {
        self.sidebar_open = false;
    }

    pub fn sidebar_up(&mut self) {
        if self.history_anchors.is_empty() {
            return;
        }
        if self.sidebar_pick == 0 {
            self.sidebar_pick = self.history_anchors.len() - 1;
        } else {
            self.sidebar_pick -= 1;
        }
        self.ensure_sidebar_pick_visible();
    }

    pub fn sidebar_down(&mut self) {
        if self.history_anchors.is_empty() {
            return;
        }
        self.sidebar_pick = (self.sidebar_pick + 1) % self.history_anchors.len();
        self.ensure_sidebar_pick_visible();
    }

    pub fn scroll_sidebar_list_up(&mut self) {
        self.sidebar_scroll = self.sidebar_scroll.saturating_sub(1);
    }

    pub fn scroll_sidebar_list_down(&mut self) {
        let viewport = self.sidebar_viewport.max(1);
        let max = self.history_anchors.len().saturating_sub(viewport);
        self.sidebar_scroll = (self.sidebar_scroll + 1).min(max);
    }

    pub fn jump_to_history(&mut self, idx: usize) {
        if let Some(anchor) = self.history_anchors.get(idx) {
            self.sidebar_pick = idx;
            self.begin_smooth_scroll_to(anchor.line);
        }
    }

    pub fn jump_to_sidebar_pick(&mut self) {
        let pick = self.sidebar_pick;
        self.jump_to_history(pick);
    }

    pub fn active_history_index(&self) -> Option<usize> {
        if self.history_anchors.is_empty() {
            return None;
        }
        let scroll = self.scroll;
        for (i, anchor) in self.history_anchors.iter().enumerate().rev() {
            if anchor.line <= scroll.saturating_add(1) {
                return Some(i);
            }
        }
        Some(0)
    }

    fn ensure_sidebar_pick_visible(&mut self) {
        let viewport = self.sidebar_viewport.max(1);
        if self.sidebar_pick < self.sidebar_scroll {
            self.sidebar_scroll = self.sidebar_pick;
        } else if self.sidebar_pick >= self.sidebar_scroll + viewport {
            self.sidebar_scroll = self.sidebar_pick + 1 - viewport;
        }
    }
}
