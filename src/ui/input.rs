use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::commands::SUGGESTIONS;

use super::theme::*;
use super::{AppState, Hitboxes};

const PROMPT: &str = "› ";
const PROMPT_COLS: u16 = 2;
const MIN_INPUT_HEIGHT: u16 = 5;
const MAX_VISIBLE_LINES: usize = 8;

#[derive(Clone, Debug)]
struct VisualSegment {
    start: usize,
    end: usize,
    show_prompt: bool,
}

pub fn height(state: &AppState, width: u16) -> u16 {
    let inner_w = inner_width(width);
    let lines = visual_segments(&state.input, inner_w).len().max(1);
    let visible = lines.clamp(1, MAX_VISIBLE_LINES);
    (visible as u16 + 2).max(MIN_INPUT_HEIGHT)
}

fn inner_width(outer_width: u16) -> u16 {
    outer_width.saturating_sub(6).max(8)
}

pub fn draw(frame: &mut Frame, state: &AppState, area: Rect, hitboxes: &mut Hitboxes) {
    let can_type = state.can_edit_input();

    let accent = mode_accent(state.mode);
    let block = Block::default()
        .title(" COMPOSE ")
        .title_style(accent_fg(accent))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if can_type { border_focus() } else { border() })
        .style(elevated());

    let inner = block.inner(area);
    let inner_w = inner.width.max(1);
    hitboxes.input_outer = area;
    hitboxes.input_text = inner;
    hitboxes.input_wrap_width = inner_w;
    let segments = visual_segments(&state.input, inner_w);
    let viewport = inner.height.max(1) as usize;
    let scroll = state.input_scroll.min(segments.len().saturating_sub(1));

    let mut lines = Vec::new();
    if state.input.is_empty() {
        let hint = if state.awaiting_approval {
            state
                .approval_summary
                .clone()
                .unwrap_or_else(|| "approval required".to_string())
        } else {
            placeholder_for(state, can_type).to_string()
        };
        let blink = state.cursor_blink && can_type;
        if hint.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(PROMPT, prompt()),
                Span::styled(" ", cursor_block(blink)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(PROMPT, prompt()),
                Span::styled(hint, ghost()),
            ]));
        }
    } else {
        let cursor = state.cursor.min(state.input.len());
        let blink = state.cursor_blink && can_type;
        for (vis_idx, seg) in segments.iter().enumerate().skip(scroll).take(viewport) {
            lines.push(build_segment_line(
                &state.input,
                seg,
                cursor,
                blink && can_type,
                vis_idx,
                &segments,
            ));
        }
        if lines.is_empty() {
            lines.push(Line::from(Span::raw("")));
        }
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn placeholder_for(state: &AppState, can_type: bool) -> &'static str {
    if !can_type {
        return "waiting...";
    }
    if state.has_token {
        return "message or /command  (ctrl+enter newline)";
    }
    "set token to start — type /token"
}

fn build_segment_line(
    input: &str,
    seg: &VisualSegment,
    cursor: usize,
    blink: bool,
    vis_idx: usize,
    segments: &[VisualSegment],
) -> Line<'static> {
    let text = &input[seg.start..seg.end];
    let mut spans = Vec::new();

    if seg.show_prompt {
        spans.push(Span::styled(PROMPT, prompt()));
    } else {
        spans.push(Span::raw(" ".repeat(PROMPT_COLS as usize)));
    }

    if cursor < seg.start || cursor > seg.end {
        spans.push(Span::styled(text.to_string(), cmd()));
    } else {
        let local = cursor - seg.start;
        let (before, after) = text.split_at(local);
        spans.push(Span::styled(before.to_string(), cmd()));
        if after.is_empty() {
            spans.push(Span::styled(" ", cursor_block(blink)));
        } else {
            let mut chars = after.chars();
            let c = chars.next().unwrap();
            spans.push(Span::styled(c.to_string(), cursor_block(blink)));
            let rest: String = chars.collect();
            if !rest.is_empty() {
                spans.push(Span::styled(rest, cmd()));
            }
        }
    }

    let _ = vis_idx;
    let _ = segments;
    Line::from(spans)
}

fn visual_segments(input: &str, width: u16) -> Vec<VisualSegment> {
    let width = width as usize;
    let prompt_w = PROMPT_COLS as usize;
    let mut segments = Vec::new();

    if input.is_empty() {
        return segments;
    }

    let logical_lines: Vec<&str> = if input.contains('\n') {
        input.split('\n').collect()
    } else {
        vec![input]
    };

    let mut byte_offset = 0usize;
    let mut first_segment_ever = true;
    for (i, logical) in logical_lines.iter().enumerate() {
        let first_width = width.saturating_sub(prompt_w).max(1);
        let cont_width = width.max(1);
        let mut chunk_start = 0usize;
        let mut first_wrap = true;

        loop {
            let avail = if first_wrap { first_width } else { cont_width };
            if chunk_start >= logical.len() {
                if first_wrap {
                    segments.push(VisualSegment {
                        start: byte_offset,
                        end: byte_offset,
                        show_prompt: first_segment_ever,
                    });
                    first_segment_ever = false;
                }
                break;
            }

            let chunk_end = wrap_end(logical, chunk_start, avail);
            segments.push(VisualSegment {
                start: byte_offset + chunk_start,
                end: byte_offset + chunk_end,
                show_prompt: first_segment_ever,
            });
            first_segment_ever = false;
            chunk_start = chunk_end;
            first_wrap = false;
            if chunk_start >= logical.len() {
                break;
            }
        }

        byte_offset += logical.len();
        if i + 1 < logical_lines.len() {
            byte_offset += 1;
        }
    }

    segments
}

fn wrap_end(line: &str, start: usize, max_cols: usize) -> usize {
    if line.is_empty() {
        return 0;
    }
    let slice = &line[start..];
    let char_count = slice.chars().count();
    if char_count <= max_cols {
        return start + slice.len();
    }

    let mut cols = 0usize;
    let mut last_space: Option<usize> = None;
    let mut end_byte = start;
    for (byte_idx, ch) in slice.char_indices() {
        if ch == ' ' {
            last_space = Some(byte_idx);
        }
        cols += 1;
        end_byte = start + byte_idx + ch.len_utf8();
        if cols >= max_cols {
            if let Some(sp) = last_space {
                if sp > 0 {
                    return start + sp;
                }
            }
            break;
        }
    }
    end_byte.min(line.len())
}

fn cursor_visual_index(segments: &[VisualSegment], cursor: usize) -> usize {
    for (i, seg) in segments.iter().enumerate() {
        if cursor <= seg.end {
            return i;
        }
    }
    segments.len().saturating_sub(1)
}

fn ensure_cursor_visible(state: &mut AppState, width: u16) {
    let inner_w = inner_width(width);
    let segments = visual_segments(&state.input, inner_w);
    if segments.is_empty() {
        state.input_scroll = 0;
        return;
    }
    let vis = cursor_visual_index(&segments, state.cursor);
    let viewport = MAX_VISIBLE_LINES.min(segments.len()).max(1);
    if vis < state.input_scroll {
        state.input_scroll = vis;
    } else if vis >= state.input_scroll + viewport {
        state.input_scroll = vis + 1 - viewport;
    }
}

pub fn cursor_from_click(
    column: u16,
    row: u16,
    text_area: Rect,
    input: &str,
    scroll: usize,
) -> usize {
    let inner_w = text_area.width.max(1);
    let segments = visual_segments(input, inner_w);
    let rel_row = row.saturating_sub(text_area.y) as usize;
    let vis_idx = scroll + rel_row;
    let Some(seg) = segments.get(vis_idx) else {
        return input.len();
    };

    let rel_col = column.saturating_sub(text_area.x + PROMPT_COLS);
    if rel_col == 0 {
        return seg.start;
    }

    let text = &input[seg.start..seg.end];
    for (col, (byte_idx, _)) in text.char_indices().enumerate() {
        if col as u16 >= rel_col {
            return seg.start + byte_idx;
        }
    }
    seg.end
}

impl AppState {
    pub fn can_edit_input(&self) -> bool {
        !self.is_thinking && !self.awaiting_approval && !self.has_overlay()
    }

    pub fn insert_char(&mut self, c: char) {
        if !self.can_edit_input() {
            return;
        }
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.refresh_input_suggestions();
    }

    pub fn insert_newline(&mut self) {
        if !self.can_edit_input() {
            return;
        }
        self.input.insert(self.cursor, '\n');
        self.cursor += 1;
        self.refresh_input_suggestions();
    }

    pub fn backspace(&mut self) {
        if !self.can_edit_input() || self.cursor == 0 {
            return;
        }
        let prev = self.input[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.input.replace_range(prev..self.cursor, "");
        self.cursor = prev;
        self.refresh_input_suggestions();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.input[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.cursor = prev;
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let next = self.input[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.input.len());
        self.cursor = next;
    }

    pub fn move_cursor_up_line(&mut self, width: u16) {
        let inner_w = inner_width(width);
        let segments = visual_segments(&self.input, inner_w);
        if segments.is_empty() {
            return;
        }
        let vis = cursor_visual_index(&segments, self.cursor);
        if vis == 0 {
            return;
        }
        let prev = &segments[vis - 1];
        let local = self.cursor.saturating_sub(segments[vis].start);
        let prev_len = prev.end.saturating_sub(prev.start);
        let target_local = local.min(prev_len);
        self.cursor = prev.start + target_local;
        self.ensure_input_scroll(width);
    }

    pub fn move_cursor_down_line(&mut self, width: u16) {
        let inner_w = inner_width(width);
        let segments = visual_segments(&self.input, inner_w);
        if segments.is_empty() {
            return;
        }
        let vis = cursor_visual_index(&segments, self.cursor);
        if vis + 1 >= segments.len() {
            return;
        }
        let next = &segments[vis + 1];
        let local = self.cursor.saturating_sub(segments[vis].start);
        let next_len = next.end.saturating_sub(next.start);
        let target_local = local.min(next_len);
        self.cursor = next.start + target_local;
        self.ensure_input_scroll(width);
    }

    pub fn ensure_input_scroll(&mut self, width: u16) {
        ensure_cursor_visible(self, width);
    }

    pub fn set_cursor_from_click(&mut self, column: u16, row: u16, text_area: Rect) {
        self.cursor = cursor_from_click(column, row, text_area, &self.input, self.input_scroll);
        self.ensure_input_scroll(text_area.width);
    }

    pub fn apply_suggestion(&mut self, index: usize) {
        let cmd = match self.suggestion_matches.get(index) {
            Some(c) => *c,
            None => return,
        };
        let usage = SUGGESTIONS
            .iter()
            .find(|s| s.cmd == cmd)
            .map(|s| s.usage)
            .unwrap_or(cmd);
        self.input = usage.to_string();
        self.cursor = self.input.len();
        self.suggestion_pick = index;
        self.input_scroll = 0;
        self.refresh_input_suggestions();
    }

    pub fn suggestion_up(&mut self) {
        if self.suggestion_matches.is_empty() {
            return;
        }
        if self.suggestion_pick == 0 {
            self.suggestion_pick = self.suggestion_matches.len() - 1;
        } else {
            self.suggestion_pick -= 1;
        }
    }

    pub fn suggestion_down(&mut self) {
        if self.suggestion_matches.is_empty() {
            return;
        }
        self.suggestion_pick = (self.suggestion_pick + 1) % self.suggestion_matches.len();
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.input_scroll = 0;
        self.suggestion_pick = 0;
        self.suggestion_matches.clear();
    }

    pub fn scroll_input_up(&mut self, width: u16) {
        self.input_scroll = self.input_scroll.saturating_sub(1);
        let _ = width;
    }

    pub fn scroll_input_down(&mut self, width: u16) {
        let inner_w = inner_width(width);
        let segs = visual_segments(&self.input, inner_w);
        let viewport = MAX_VISIBLE_LINES.min(segs.len()).max(1);
        let max_scroll = segs.len().saturating_sub(viewport);
        self.input_scroll = (self.input_scroll + 1).min(max_scroll);
    }

    fn refresh_input_suggestions(&mut self) {
        let first_line = self.input.lines().next().unwrap_or("").to_string();
        super::suggestions::refresh_matches_first_line(self, &first_line);
    }
}
