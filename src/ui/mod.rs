mod chat;
mod chrome;
mod input;
mod overlay;
mod sidebar;
pub mod suggestions;
mod theme;

pub use chat::HistoryAnchor;

pub use overlay::{Overlay, OverlayAction};

use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::Block,
    Frame,
};

use crate::mode::AgentMode;

pub use theme::bg;

#[derive(Clone, Default)]
pub struct Hitboxes {
    pub chat: Rect,
    pub chat_inner: Rect,
    pub input_outer: Rect,
    pub input_text: Rect,
    pub suggestions: Option<Rect>,
    pub suggestion_rows: Vec<(Rect, usize)>,
    pub input_wrap_width: u16,
    pub sidebar_toggle: Rect,
    pub sidebar: Option<Rect>,
    pub sidebar_rows: Vec<(Rect, usize)>,
}

#[derive(Clone)]
pub enum ChatEntry {
    User {
        content: String,
    },
    Assistant {
        content: String,
    },
    System {
        content: String,
    },
    Tool {
        name: String,
        args: String,
        result: Option<String>,
        error: Option<String>,
    },
    Error {
        message: String,
    },
}

impl ChatEntry {
    pub(crate) fn is_running(&self) -> bool {
        matches!(
            self,
            ChatEntry::Tool {
                result: None,
                error: None,
                ..
            }
        )
    }
}

pub struct AppState {
    pub mode: AgentMode,
    pub has_token: bool,
    pub entries: Vec<ChatEntry>,
    pub input: String,
    pub cursor: usize,
    pub is_thinking: bool,
    pub awaiting_approval: bool,
    pub approval_summary: Option<String>,
    pub spinner_frame: usize,
    pub cursor_blink: bool,
    pub suggestion_pick: usize,
    pub suggestion_matches: Vec<&'static str>,
    pub hitboxes: Hitboxes,
    pub scroll: usize,
    pub last_line_count: usize,
    pub chat_viewport: usize,
    pub following: bool,
    pub overlay: Overlay,
    pub input_scroll: usize,
    pub sidebar_open: bool,
    pub sidebar_pick: usize,
    pub sidebar_scroll: usize,
    pub history_anchors: Vec<HistoryAnchor>,
    pub sidebar_viewport: usize,
    pub scroll_target: Option<usize>,
}

impl AppState {
    pub fn new(mode: AgentMode, has_token: bool) -> Self {
        Self {
            mode,
            has_token,
            entries: Vec::new(),
            input: String::new(),
            cursor: 0,
            is_thinking: false,
            awaiting_approval: false,
            approval_summary: None,
            spinner_frame: 0,
            cursor_blink: true,
            suggestion_pick: 0,
            suggestion_matches: Vec::new(),
            hitboxes: Hitboxes::default(),
            scroll: 0,
            last_line_count: 0,
            chat_viewport: 1,
            following: true,
            overlay: Overlay::None,
            input_scroll: 0,
            sidebar_open: false,
            sidebar_pick: 0,
            sidebar_scroll: 0,
            history_anchors: Vec::new(),
            sidebar_viewport: 1,
            scroll_target: None,
        }
    }

    pub fn max_scroll(&self) -> usize {
        self.last_line_count
            .saturating_sub(self.chat_viewport.max(1))
    }

    pub fn set_mode(&mut self, mode: AgentMode) {
        self.mode = mode;
    }

    pub fn set_has_token(&mut self, has_token: bool) {
        self.has_token = has_token;
    }

    pub fn clear_chat(&mut self) {
        self.entries.clear();
        self.awaiting_approval = false;
        self.approval_summary = None;
        self.history_anchors.clear();
        self.sidebar_pick = 0;
        self.sidebar_scroll = 0;
    }

    pub fn push_user(&mut self, content: &str) {
        self.entries.push(ChatEntry::User {
            content: content.to_string(),
        });
        self.scroll_to_bottom();
    }

    pub fn push_assistant(&mut self, content: &str) {
        self.entries.push(ChatEntry::Assistant {
            content: content.to_string(),
        });
        self.scroll_to_bottom();
    }

    pub fn push_system(&mut self, content: &str) {
        self.entries.push(ChatEntry::System {
            content: content.to_string(),
        });
        self.scroll_to_bottom();
    }

    pub fn start_tool(&mut self, name: &str, args: &str) {
        self.entries.push(ChatEntry::Tool {
            name: name.to_string(),
            args: args.to_string(),
            result: None,
            error: None,
        });
        self.scroll_to_bottom();
    }

    pub fn complete_tool(&mut self, result: &str) {
        if let Some(ChatEntry::Tool {
            result: r,
            error: e,
            ..
        }) = self.entries.last_mut()
        {
            *r = Some(result.to_string());
            *e = None;
        }
        self.scroll_to_bottom();
    }

    pub fn fail_tool(&mut self, error: &str) {
        if let Some(ChatEntry::Tool {
            result: r,
            error: e,
            ..
        }) = self.entries.last_mut()
        {
            *r = None;
            *e = Some(error.to_string());
        }
        self.scroll_to_bottom();
    }

    pub fn push_error(&mut self, message: &str) {
        self.entries.push(ChatEntry::Error {
            message: message.to_string(),
        });
        self.scroll_to_bottom();
    }

    pub fn request_approval(&mut self, summary: &str) {
        self.awaiting_approval = true;
        self.approval_summary = Some(summary.to_string());
        self.is_thinking = false;
    }

    pub fn clear_approval(&mut self) {
        self.awaiting_approval = false;
        self.approval_summary = None;
        self.is_thinking = true;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.cancel_smooth_scroll();
        self.following = true;
        self.scroll = self.max_scroll();
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.cancel_smooth_scroll();
        self.following = false;
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.cancel_smooth_scroll();
        self.scroll = (self.scroll + n).min(self.max_scroll());
        self.following = self.scroll >= self.max_scroll();
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.chat_viewport.saturating_sub(1).max(1));
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.chat_viewport.saturating_sub(1).max(1));
    }

    pub fn scroll_to_line(&mut self, line: usize) {
        self.begin_smooth_scroll_to(line);
    }

    pub fn begin_smooth_scroll_to(&mut self, line: usize) {
        self.following = false;
        let target = line.min(self.max_scroll());
        if self.scroll == target {
            self.scroll_target = None;
        } else {
            self.scroll_target = Some(target);
        }
    }

    pub fn cancel_smooth_scroll(&mut self) {
        self.scroll_target = None;
    }

    pub fn is_animating_scroll(&self) -> bool {
        self.scroll_target.is_some()
    }

    pub fn tick_smooth_scroll(&mut self) -> bool {
        let Some(target) = self.scroll_target else {
            return false;
        };
        if self.scroll == target {
            self.scroll_target = None;
            return false;
        }
        let diff = target as i32 - self.scroll as i32;
        let step = ((diff.unsigned_abs() as f32) * 0.28).ceil() as usize;
        let step = step.clamp(1, 5);
        if diff > 0 {
            self.scroll = (self.scroll + step).min(target);
        } else {
            self.scroll = self.scroll.saturating_sub(step).max(target);
        }
        true
    }

    pub fn needs_redraw(&self) -> bool {
        self.is_thinking
            || self.awaiting_approval
            || self.can_edit_input()
            || self.has_overlay()
            || self.is_animating_scroll()
    }

    pub fn tick(&mut self) {
        if self.is_thinking
            || self.awaiting_approval
            || self.can_edit_input()
            || self.has_overlay()
            || self.is_animating_scroll()
        {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
        if self.can_edit_input() || self.has_overlay() {
            self.cursor_blink = self.spinner_frame % 20 < 10;
        }
        self.tick_smooth_scroll();
    }
}

pub fn draw(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();
    frame.render_widget(Block::default().style(bg()), area);

    if state.has_overlay() {
        overlay::draw(frame, state, area);
        return;
    }

    let root = Layout::horizontal([Constraint::Length(2), Constraint::Min(0)]).split(area);
    chrome::draw_rail(frame, state, root[0]);

    let side_w = sidebar::width(state);
    let mut hitboxes = Hitboxes::default();

    let body = if side_w > 0 {
        let cols =
            Layout::horizontal([Constraint::Length(side_w), Constraint::Min(0)]).split(root[1]);
        sidebar::draw(frame, state, cols[0], &mut hitboxes);
        cols[1]
    } else {
        root[1]
    };

    let content_w = body.width;
    let sug_h = suggestions::height(state);

    let mut constraints = vec![
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(6),
    ];
    if sug_h > 0 {
        constraints.push(Constraint::Length(sug_h));
    }
    constraints.extend([
        Constraint::Length(input::height(state, content_w)),
        Constraint::Length(2),
    ]);

    let parts = Layout::vertical(constraints).margin(1).split(body);

    let mut idx = 0;
    chrome::draw_header(frame, state, parts[idx]);
    idx += 1;
    chrome::draw_spacer(frame, parts[idx]);
    idx += 1;

    let chat_area = parts[idx];
    idx += 1;

    chat::draw(frame, state, chat_area, &mut hitboxes);

    if sug_h > 0 {
        suggestions::draw(frame, state, parts[idx], &mut hitboxes);
        idx += 1;
    }

    input::draw(frame, state, parts[idx], &mut hitboxes);
    idx += 1;
    chrome::draw_footer(frame, state, parts[idx]);

    state.hitboxes = hitboxes;
}
