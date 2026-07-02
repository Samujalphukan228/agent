use crate::mode::AgentMode;
use crate::settings::Settings;

pub enum CommandResult {
    Handled { reply: String },
    NotACommand,
}

pub fn handle(input: &str, settings: &mut Settings) -> CommandResult {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return CommandResult::NotACommand;
    }

    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match cmd.as_str() {
        "/help" | "/commands" => CommandResult::Handled {
            reply: HELP_TEXT.to_string(),
        },

        "/token" | "/apikey" | "/key" => {
            if arg.is_empty() {
                return CommandResult::Handled {
                    reply: "__OPEN_TOKEN__".to_string(),
                };
            }
            settings.api_key = arg.to_string();
            match settings.save() {
                Ok(()) => CommandResult::Handled {
                    reply: "API token saved.".to_string(),
                },
                Err(e) => CommandResult::Handled {
                    reply: format!("Failed to save token: {e}"),
                },
            }
        }

        "/mode" => {
            if arg.is_empty() {
                let mode = settings.mode();
                return CommandResult::Handled {
                    reply: format!(
                        "Mode: {} — {}\nUse /mode base or /mode god",
                        mode.label(),
                        mode.description()
                    ),
                };
            }
            match AgentMode::from_str(arg) {
                Some(mode) => {
                    settings.set_mode(mode);
                    match settings.save() {
                        Ok(()) => CommandResult::Handled {
                            reply: format!("Mode set to {} — {}", mode.label(), mode.description()),
                        },
                        Err(e) => CommandResult::Handled {
                            reply: format!("Failed to save mode: {e}"),
                        },
                    }
                }
                None => CommandResult::Handled {
                    reply: "Unknown mode. Use /mode base or /mode god".to_string(),
                },
            }
        }

        "/settings" => CommandResult::Handled {
            reply: "__OPEN_SETTINGS__".to_string(),
        },

        "/clear" => CommandResult::Handled {
            reply: "__CLEAR__".to_string(),
        },

        "/history" | "/sidebar" => match arg.to_lowercase().as_str() {
            "" | "toggle" => CommandResult::Handled {
                reply: "__HISTORY_TOGGLE__".to_string(),
            },
            "open" => CommandResult::Handled {
                reply: "__HISTORY_OPEN__".to_string(),
            },
            "close" => CommandResult::Handled {
                reply: "__HISTORY_CLOSE__".to_string(),
            },
            _ => CommandResult::Handled {
                reply: "Use /history open, /history close, or /history toggle".to_string(),
            },
        },

        _ => CommandResult::Handled {
            reply: format!("Unknown command: {cmd}. Type /help"),
        },
    }
}

const HELP_TEXT: &str = r#"Commands:
  /token         open token entry (or /token <key>)
  /mode base     safe mode — harmful actions need approval
  /mode god      unrestricted mode
  /mode          show current mode
  /settings      open settings panel
  /clear         clear conversation
  /history       toggle question history sidebar
  /history open  open history sidebar
  /history close close history sidebar
  /help          show this help"#;

pub fn is_clear_marker(reply: &str) -> bool {
    reply == "__CLEAR__"
}

pub fn is_open_token_marker(reply: &str) -> bool {
    reply == "__OPEN_TOKEN__"
}

pub fn is_open_settings_marker(reply: &str) -> bool {
    reply == "__OPEN_SETTINGS__"
}

pub fn is_history_toggle_marker(reply: &str) -> bool {
    reply == "__HISTORY_TOGGLE__"
}

pub fn is_history_open_marker(reply: &str) -> bool {
    reply == "__HISTORY_OPEN__"
}

pub fn is_history_close_marker(reply: &str) -> bool {
    reply == "__HISTORY_CLOSE__"
}

#[derive(Clone, Copy, Debug)]
pub struct CommandSuggestion {
    pub cmd: &'static str,
    pub desc: &'static str,
    pub usage: &'static str,
}

pub const SUGGESTIONS: &[CommandSuggestion] = &[
    CommandSuggestion {
        cmd: "/token",
        desc: "set your API token",
        usage: "/token ",
    },
    CommandSuggestion {
        cmd: "/mode",
        desc: "switch base or god mode",
        usage: "/mode ",
    },
    CommandSuggestion {
        cmd: "/settings",
        desc: "open settings panel",
        usage: "/settings",
    },
    CommandSuggestion {
        cmd: "/clear",
        desc: "clear conversation",
        usage: "/clear",
    },
    CommandSuggestion {
        cmd: "/history",
        desc: "open/close question history",
        usage: "/history ",
    },
    CommandSuggestion {
        cmd: "/help",
        desc: "list all commands",
        usage: "/help",
    },
];

pub fn matching_suggestions(input: &str) -> Vec<&'static CommandSuggestion> {
    if !input.starts_with('/') {
        return Vec::new();
    }
    let q = input.to_lowercase();
    SUGGESTIONS
        .iter()
        .filter(|s| s.cmd.starts_with(q.as_str()) || q.starts_with(s.cmd))
        .collect()
}
