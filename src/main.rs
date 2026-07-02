mod agent;
mod commands;
mod errors;
mod llm;
mod mode;
mod settings;
mod tools;
mod ui;

use std::io;
use std::sync::Arc;
use std::time::Duration;

use agent::{Agent, AgentUpdate};
use commands::{
    is_clear_marker, is_history_close_marker, is_history_open_marker, is_history_toggle_marker,
    is_open_settings_marker, is_open_token_marker, CommandResult,
};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        KeyboardEnhancementFlags, MouseButton, MouseEventKind, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Position, Terminal};
use reqwest::Client;
use settings::Settings;
use tokio::sync::{mpsc, Mutex};

use ui::{AppState, OverlayAction};

struct Runtime {
    agent_rx: Option<mpsc::UnboundedReceiver<AgentUpdate>>,
    approval_tx: Option<mpsc::UnboundedSender<bool>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::load();
    let has_token = settings.has_token();
    let mode = settings.mode();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        ),
        Hide
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let client = Client::new();
    let agent = Arc::new(Mutex::new(Agent::new(client, settings)));

    let mut state = AppState::new(mode, has_token);
    if !has_token {
        state.open_token_overlay(false);
    }
    let mut runtime = Runtime {
        agent_rx: None,
        approval_tx: None,
    };

    terminal.draw(|f| ui::draw(f, &mut state))?;

    let mut running = true;
    while running {
        let mut done = false;
        if let Some(rx) = runtime.agent_rx.as_mut() {
            let updates: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
            let had_updates = !updates.is_empty();
            for update in updates {
                apply_update(&mut state, update);
            }
            done = !state.is_thinking && !state.awaiting_approval;
            if had_updates {
                terminal.draw(|f| ui::draw(f, &mut state))?;
            }
        }
        if done {
            runtime.agent_rx = None;
            runtime.approval_tx = None;
        }

        if state.needs_redraw() {
            state.tick();
            terminal.draw(|f| ui::draw(f, &mut state))?;
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == event::KeyEventKind::Press => {
                    if state.has_overlay() {
                        let action = state.handle_overlay_key(key.code);
                        if let Some(msg) = apply_overlay_action(&agent, &mut state, action).await {
                            state.push_system(&msg);
                        }
                        terminal.draw(|f| ui::draw(f, &mut state))?;
                        continue;
                    }

                    if state.awaiting_approval {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                if let Some(tx) = &runtime.approval_tx {
                                    let _ = tx.send(true);
                                }
                                state.clear_approval();
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                if let Some(tx) = &runtime.approval_tx {
                                    let _ = tx.send(false);
                                }
                                state.clear_approval();
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    let showing_suggestions = ui::suggestions::visible(&state);

                    match key.code {
                        KeyCode::Char('c') | KeyCode::Char('q')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            running = false;
                        }

                        KeyCode::Esc if state.sidebar_open => {
                            state.close_sidebar();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Esc if state.can_edit_input() => {
                            state.clear_input();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Char('b')
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && !state.has_overlay() =>
                        {
                            state.toggle_sidebar();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Char('h')
                            if state.input.is_empty()
                                && state.can_edit_input()
                                && !state.has_overlay() =>
                        {
                            state.toggle_sidebar();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Up if state.sidebar_open => {
                            state.sidebar_up();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Down if state.sidebar_open => {
                            state.sidebar_down();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Enter if state.sidebar_open => {
                            state.jump_to_sidebar_pick();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Tab if showing_suggestions => {
                            state.apply_suggestion(state.suggestion_pick);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Up if showing_suggestions => {
                            state.suggestion_up();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Down if showing_suggestions => {
                            state.suggestion_down();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Up
                            if state.can_edit_input()
                                && !state.input.is_empty()
                                && !showing_suggestions
                                && !state.sidebar_open =>
                        {
                            let w = state.hitboxes.input_wrap_width.max(20);
                            state.move_cursor_up_line(w);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Down
                            if state.can_edit_input()
                                && !state.input.is_empty()
                                && !showing_suggestions
                                && !state.sidebar_open =>
                        {
                            let w = state.hitboxes.input_wrap_width.max(20);
                            state.move_cursor_down_line(w);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Up => {
                            state.scroll_up(1);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Down => {
                            state.scroll_down(1);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::PageUp => {
                            state.page_up();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::PageDown => {
                            state.page_down();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Left if state.can_edit_input() => {
                            state.move_cursor_left();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Right if state.can_edit_input() => {
                            state.move_cursor_right();
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Home if state.can_edit_input() => {
                            state.cursor = 0;
                            state.ensure_input_scroll(state.hitboxes.input_wrap_width.max(20));
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::End if state.can_edit_input() => {
                            state.cursor = state.input.len();
                            state.ensure_input_scroll(state.hitboxes.input_wrap_width.max(20));
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Char('k') if state.input.is_empty() => {
                            state.scroll_up(1);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }
                        KeyCode::Char('j') | KeyCode::Char('J')
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && state.can_edit_input() =>
                        {
                            insert_input_newline(&mut state, &mut terminal)?;
                        }

                        KeyCode::Char('j') if state.input.is_empty() => {
                            state.scroll_down(1);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Enter
                            if !state.is_thinking
                                && enter_inserts_newline(&key)
                                && state.can_edit_input() =>
                        {
                            insert_input_newline(&mut state, &mut terminal)?;
                        }

                        KeyCode::Enter if !state.is_thinking && !state.sidebar_open => {
                            if showing_suggestions {
                                let pick = state.suggestion_pick;
                                let cmd = state.suggestion_matches.get(pick).copied().unwrap_or("");
                                let current = state.input.trim();
                                if current != cmd
                                    && current != "/help"
                                    && current != "/clear"
                                    && current != "/settings"
                                    && current != "/history"
                                {
                                    state.apply_suggestion(pick);
                                    terminal.draw(|f| ui::draw(f, &mut state))?;
                                    continue;
                                }
                            }

                            let input = state.input.trim().to_string();
                            if input.is_empty() {
                                continue;
                            }
                            let cmd_line = input.lines().next().unwrap_or("").trim().to_string();
                            state.clear_input();

                            if cmd_line.starts_with('/') {
                                handle_command(&agent, &mut state, &cmd_line).await;
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                                continue;
                            }

                            {
                                let a = agent.lock().await;
                                if !a.has_token() {
                                    state.open_token_overlay(false);
                                    terminal.draw(|f| ui::draw(f, &mut state))?;
                                    continue;
                                }
                            }

                            state.push_user(&input);
                            state.is_thinking = true;
                            start_agent_turn(&agent, &mut runtime, input).await;
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Backspace if state.can_edit_input() => {
                            let w = state.hitboxes.input_wrap_width.max(20);
                            state.backspace();
                            state.ensure_input_scroll(w);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        KeyCode::Char(c) if state.can_edit_input() => {
                            let w = state.hitboxes.input_wrap_width.max(20);
                            state.insert_char(c);
                            state.ensure_input_scroll(w);
                            terminal.draw(|f| ui::draw(f, &mut state))?;
                        }

                        _ => {}
                    }
                }

                Event::Mouse(mouse) => {
                    let pos = Position::new(mouse.column, mouse.row);
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if state.sidebar_open
                                && state.hitboxes.sidebar.is_some_and(|r| r.contains(pos))
                            {
                                state.scroll_sidebar_list_up();
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            } else if state.hitboxes.input_text.contains(pos)
                                && state.can_edit_input()
                            {
                                state.scroll_input_up(state.hitboxes.input_wrap_width.max(20));
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            } else if state.hitboxes.chat_inner.contains(pos) {
                                state.scroll_up(3);
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            } else if state.hitboxes.suggestions.is_some_and(|r| r.contains(pos)) {
                                state.suggestion_up();
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if state.sidebar_open
                                && state.hitboxes.sidebar.is_some_and(|r| r.contains(pos))
                            {
                                state.scroll_sidebar_list_down();
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            } else if state.hitboxes.input_text.contains(pos)
                                && state.can_edit_input()
                            {
                                state.scroll_input_down(state.hitboxes.input_wrap_width.max(20));
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            } else if state.hitboxes.chat_inner.contains(pos) {
                                state.scroll_down(3);
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            } else if state.hitboxes.suggestions.is_some_and(|r| r.contains(pos)) {
                                state.suggestion_down();
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            }
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            let mut redraw = false;

                            if state.hitboxes.sidebar_toggle.contains(pos) {
                                state.toggle_sidebar();
                                redraw = true;
                            }

                            if !redraw {
                                for (rect, idx) in &state.hitboxes.sidebar_rows.clone() {
                                    if rect.contains(pos) {
                                        state.jump_to_history(*idx);
                                        redraw = true;
                                        break;
                                    }
                                }
                            }

                            for (rect, idx) in &state.hitboxes.suggestion_rows.clone() {
                                if rect.contains(pos) {
                                    state.apply_suggestion(*idx);
                                    redraw = true;
                                    break;
                                }
                            }

                            if !redraw
                                && (state.hitboxes.input_text.contains(pos)
                                    || state.hitboxes.input_outer.contains(pos))
                            {
                                state.set_cursor_from_click(
                                    mouse.column,
                                    mouse.row,
                                    state.hitboxes.input_text,
                                );
                                redraw = true;
                            }

                            if !redraw && state.hitboxes.chat_inner.contains(pos) {
                                let rel =
                                    mouse.row.saturating_sub(state.hitboxes.chat_inner.y) as usize;
                                state.scroll_to_line(state.scroll.saturating_add(rel));
                                redraw = true;
                            }

                            if redraw {
                                terminal.draw(|f| ui::draw(f, &mut state))?;
                            }
                        }
                        _ => {}
                    }
                }

                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        PopKeyboardEnhancementFlags,
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn enter_inserts_newline(key: &KeyEvent) -> bool {
    key.modifiers
        .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn insert_input_newline(
    state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let w = state.hitboxes.input_wrap_width.max(20);
    state.insert_newline();
    state.ensure_input_scroll(w);
    terminal.draw(|f| ui::draw(f, state))?;
    Ok(())
}

async fn handle_command(agent: &Arc<Mutex<Agent>>, state: &mut AppState, input: &str) {
    let mut agent_guard = agent.lock().await;
    match commands::handle(input, agent_guard.settings_mut()) {
        CommandResult::Handled { reply } => {
            if is_clear_marker(&reply) {
                agent_guard.clear_history();
                state.clear_chat();
                state.push_system("Conversation cleared.");
            } else if is_open_token_marker(&reply) {
                state.open_token_overlay(agent_guard.settings().has_token());
            } else if is_open_settings_marker(&reply) {
                open_settings_from_agent(state, &agent_guard);
            } else if is_history_toggle_marker(&reply) {
                state.toggle_sidebar();
                let msg = if state.sidebar_open {
                    "History sidebar opened."
                } else {
                    "History sidebar closed."
                };
                state.push_system(msg);
            } else if is_history_open_marker(&reply) {
                state.open_sidebar();
                state.push_system("History sidebar opened.");
            } else if is_history_close_marker(&reply) {
                state.close_sidebar();
                state.push_system("History sidebar closed.");
            } else {
                state.push_system(&reply);
            }
            state.set_mode(agent_guard.settings().mode());
            state.set_has_token(agent_guard.settings().has_token());
        }
        CommandResult::NotACommand => {}
    }
}

fn open_settings_from_agent(state: &mut AppState, agent: &Agent) {
    let settings = agent.settings();
    state.open_settings_overlay(
        settings.masked_token(),
        settings.mode(),
        settings::settings_path().display().to_string(),
    );
}

async fn apply_overlay_action(
    agent: &Arc<Mutex<Agent>>,
    state: &mut AppState,
    action: OverlayAction,
) -> Option<String> {
    match action {
        OverlayAction::None => None,
        OverlayAction::Close => {
            state.close_overlay();
            None
        }
        OverlayAction::SaveToken(token) => {
            let mut guard = agent.lock().await;
            guard.settings_mut().api_key = token;
            match guard.settings().save() {
                Ok(()) => {
                    state.set_has_token(true);
                    state.close_overlay();
                    Some("API token saved.".to_string())
                }
                Err(e) => {
                    if let ui::Overlay::TokenEntry { error, .. } = &mut state.overlay {
                        *error = Some(format!("Failed to save: {e}"));
                    }
                    None
                }
            }
        }
        OverlayAction::SetMode(mode) => {
            let mut guard = agent.lock().await;
            guard.settings_mut().set_mode(mode);
            match guard.settings().save() {
                Ok(()) => {
                    state.set_mode(mode);
                    if let ui::Overlay::Settings {
                        mode: m,
                        masked_token,
                        config_path,
                        ..
                    } = &mut state.overlay
                    {
                        *m = mode;
                        *masked_token = guard.settings().masked_token();
                        *config_path = settings::settings_path().display().to_string();
                    }
                    Some(format!(
                        "Mode set to {} — {}",
                        mode.label(),
                        mode.description()
                    ))
                }
                Err(e) => Some(format!("Failed to save mode: {e}")),
            }
        }
        OverlayAction::OpenToken => {
            let dismissable = {
                let guard = agent.lock().await;
                guard.settings().has_token()
            };
            state.open_token_overlay(dismissable);
            None
        }
    }
}

async fn start_agent_turn(agent: &Arc<Mutex<Agent>>, runtime: &mut Runtime, input: String) {
    let (update_tx, update_rx) = mpsc::unbounded_channel();
    let (approval_tx, approval_rx) = mpsc::unbounded_channel();

    runtime.agent_rx = Some(update_rx);
    runtime.approval_tx = Some(approval_tx);

    {
        let mut agent_guard = agent.lock().await;
        agent_guard.push_user(&input);
    }

    let agent = Arc::clone(agent);
    tokio::spawn(async move {
        let mut agent_guard = agent.lock().await;
        let mut approval_rx = approval_rx;
        agent_guard.run_turn(update_tx, &mut approval_rx).await;
    });
}

fn apply_update(state: &mut AppState, update: AgentUpdate) {
    match update {
        AgentUpdate::StartTool { name, args } => state.start_tool(&name, &args),
        AgentUpdate::CompleteTool(result) => state.complete_tool(&result),
        AgentUpdate::FailTool(error) => state.fail_tool(&error),
        AgentUpdate::Assistant(text) => {
            state.is_thinking = false;
            state.push_assistant(&text);
        }
        AgentUpdate::Error(message) => {
            state.is_thinking = false;
            state.push_error(&message);
        }
        AgentUpdate::ApprovalNeeded { summary } => {
            state.request_approval(&summary);
            state.push_system(&format!("Approval required: {summary}"));
        }
        AgentUpdate::Done => {
            if !state.awaiting_approval {
                state.is_thinking = false;
            }
        }
    }
}
