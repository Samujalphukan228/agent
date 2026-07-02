use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::errors::AppError;
use crate::llm::{self, Message};
use crate::mode::{needs_approval, AgentMode};
use crate::settings::{sanitize_error, Settings};
use crate::tools;

const MAX_TOOL_OUTPUT: usize = 12_000;

pub enum AgentUpdate {
    StartTool { name: String, args: String },
    CompleteTool(String),
    FailTool(String),
    Assistant(String),
    Error(String),
    ApprovalNeeded { summary: String },
    Done,
}

pub struct Agent {
    client: Client,
    settings: Settings,
    history: Vec<Message>,
}

impl Agent {
    pub fn new(client: Client, settings: Settings) -> Self {
        Self {
            client,
            settings,
            history: Vec::new(),
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn has_token(&self) -> bool {
        self.settings.has_token()
    }

    pub fn push_user(&mut self, text: &str) {
        self.history.push(Message::user_text(text));
    }

    pub async fn run_turn(
        &mut self,
        tx: mpsc::UnboundedSender<AgentUpdate>,
        approval_rx: &mut mpsc::UnboundedReceiver<bool>,
    ) {
        let mark = self.history.len();
        if let Err(err) = self.turn(&tx, approval_rx).await {
            self.history.truncate(mark);
            let _ = tx.send(AgentUpdate::Error(sanitize_error(&err.to_string())));
        }
        let _ = tx.send(AgentUpdate::Done);
    }

    async fn turn(
        &mut self,
        tx: &mpsc::UnboundedSender<AgentUpdate>,
        approval_rx: &mut mpsc::UnboundedReceiver<bool>,
    ) -> Result<(), AppError> {
        if !self.settings.has_token() {
            return Err(AppError::Api(
                "No API token set. Use /token <your-key>".into(),
            ));
        }

        let mode = self.settings.mode();
        let prompt = system_prompt(mode);

        loop {
            let response = llm::chat(
                &self.client,
                &self.settings.api_key,
                Settings::model(),
                Some(prompt),
                &self.history,
            )
            .await?;

            let part = response
                .candidates
                .into_iter()
                .next()
                .and_then(|c| c.content.parts.into_iter().next());

            let Some(part) = part else {
                return Err(AppError::Api("Empty response".into()));
            };

            if let Some(fc) = part.function_call {
                let args_display = format_tool_args(&fc.name, &fc.args);
                let _ = tx.send(AgentUpdate::StartTool {
                    name: fc.name.clone(),
                    args: args_display,
                });

                self.history.push(Message::model_function_call(
                    fc.name.clone(),
                    fc.args.clone(),
                ));

                let result = self
                    .run_tool(&fc.name, &fc.args, mode, tx, approval_rx)
                    .await;
                let output = truncate_tool_output(&result);

                match &result {
                    Ok(_) => {
                        let _ = tx.send(AgentUpdate::CompleteTool(output.clone()));
                    }
                    Err(AppError::Denied) => {
                        let msg = "Denied by user".to_string();
                        let _ = tx.send(AgentUpdate::FailTool(msg.clone()));
                        self.history.push(Message::tool_result(fc.name, msg));
                        continue;
                    }
                    Err(err) => {
                        let _ = tx.send(AgentUpdate::FailTool(err.to_string()));
                    }
                }

                self.history.push(Message::tool_result(fc.name, output));
                continue;
            }

            if let Some(text) = part.text {
                if text.trim().is_empty() {
                    return Err(AppError::Api("Empty response".into()));
                }

                self.history.push(Message::model_text(&text));
                let _ = tx.send(AgentUpdate::Assistant(text));
                return Ok(());
            }

            return Err(AppError::Api("Unexpected response".into()));
        }
    }

    async fn run_tool(
        &self,
        name: &str,
        args: &Value,
        mode: AgentMode,
        tx: &mpsc::UnboundedSender<AgentUpdate>,
        approval_rx: &mut mpsc::UnboundedReceiver<bool>,
    ) -> Result<String, AppError> {
        if let Some(summary) = needs_approval(mode, name, args) {
            let _ = tx.send(AgentUpdate::ApprovalNeeded { summary });
            let approved = approval_rx
                .recv()
                .await
                .ok_or(AppError::Api("Approval channel closed".into()))?;
            if !approved {
                return Err(AppError::Denied);
            }
        }

        execute_tool(name, args).await
    }
}

fn system_prompt(mode: AgentMode) -> &'static str {
    match mode {
        AgentMode::God => {
            r#"You are a helpful PC assistant with full access to the user's machine.
Tools: run_shell, read_file, write_file, list_dir, search_files, grep, edit_file,
delete_file, create_dir, move_file, http_get, env_info.
Use tools for system info, files, search, and network — never guess.
Be concise and practical."#
        }
        AgentMode::Base => {
            r#"You are a helpful PC assistant with access to the user's machine.
Tools: run_shell, read_file, write_file, list_dir, search_files, grep, edit_file,
delete_file, create_dir, move_file, http_get, env_info.
Prefer read-only tools when possible. Destructive or sensitive actions may need approval.
Be concise and practical."#
        }
    }
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn arg_bool(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn arg_u64(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

fn format_tool_args(name: &str, args: &Value) -> String {
    match name {
        "run_shell" => format!("command: {}", arg_str(args, "command").unwrap_or("")),
        "read_file" | "list_dir" | "delete_file" | "create_dir" => {
            format!("path: {}", arg_str(args, "path").unwrap_or(""))
        }
        "write_file" | "edit_file" => format!("path: {}", arg_str(args, "path").unwrap_or("")),
        "search_files" => format!(
            "root: {}  pattern: {}",
            arg_str(args, "root").unwrap_or(""),
            arg_str(args, "pattern").unwrap_or("")
        ),
        "grep" => format!(
            "path: {}  pattern: {}",
            arg_str(args, "path").unwrap_or(""),
            arg_str(args, "pattern").unwrap_or("")
        ),
        "move_file" => format!(
            "from: {}  to: {}",
            arg_str(args, "from").unwrap_or(""),
            arg_str(args, "to").unwrap_or("")
        ),
        "http_get" => format!("url: {}", arg_str(args, "url").unwrap_or("")),
        "env_info" => "system info".to_string(),
        _ => format!("{args}"),
    }
}

async fn execute_tool(name: &str, args: &Value) -> Result<String, AppError> {
    match name {
        "run_shell" => {
            let command = arg_str(args, "command").unwrap_or("");
            if command.is_empty() {
                return Err(AppError::Tool("Missing command".into()));
            }
            tools::shell::run(command).await
        }
        "read_file" => {
            let path = arg_str(args, "path").unwrap_or("");
            if path.is_empty() {
                return Err(AppError::Tool("Missing path".into()));
            }
            tools::file::read(path).await
        }
        "write_file" => {
            let path = arg_str(args, "path").unwrap_or("");
            let content = arg_str(args, "content").unwrap_or("");
            if path.is_empty() {
                return Err(AppError::Tool("Missing path".into()));
            }
            tools::file::write(path, content).await
        }
        "list_dir" => {
            let path = arg_str(args, "path").unwrap_or("");
            if path.is_empty() {
                return Err(AppError::Tool("Missing path".into()));
            }
            tools::file::list(path).await
        }
        "search_files" => {
            let root = arg_str(args, "root").unwrap_or("");
            let pattern = arg_str(args, "pattern").unwrap_or("");
            let depth = arg_u64(args, "max_depth", 8) as usize;
            tools::search::search(root, pattern, depth).await
        }
        "grep" => {
            let path = arg_str(args, "path").unwrap_or("");
            let pattern = arg_str(args, "pattern").unwrap_or("");
            let ignore_case = arg_bool(args, "ignore_case", false);
            tools::grep::grep(path, pattern, ignore_case).await
        }
        "edit_file" => {
            let path = arg_str(args, "path").unwrap_or("");
            let old_text = arg_str(args, "old_text").unwrap_or("");
            let new_text = arg_str(args, "new_text").unwrap_or("");
            let replace_all = arg_bool(args, "replace_all", false);
            tools::file::edit(path, old_text, new_text, replace_all).await
        }
        "delete_file" => {
            let path = arg_str(args, "path").unwrap_or("");
            tools::file::delete_path(path).await
        }
        "create_dir" => {
            let path = arg_str(args, "path").unwrap_or("");
            tools::file::create_dir(path).await
        }
        "move_file" => {
            let from = arg_str(args, "from").unwrap_or("");
            let to = arg_str(args, "to").unwrap_or("");
            tools::file::move_path(from, to).await
        }
        "http_get" => {
            let url = arg_str(args, "url").unwrap_or("");
            tools::http::get(url).await
        }
        "env_info" => tools::env::info().await,
        other => Err(AppError::Tool(format!("Unknown tool: {other}"))),
    }
}

fn truncate_tool_output(result: &Result<String, AppError>) -> String {
    match result {
        Ok(text) => {
            if text.len() <= MAX_TOOL_OUTPUT {
                text.clone()
            } else {
                format!(
                    "{}\n… truncated ({} chars total)",
                    &text[..MAX_TOOL_OUTPUT],
                    text.len()
                )
            }
        }
        Err(AppError::Denied) => "Denied by user".to_string(),
        Err(err) => format!("Error: {err}"),
    }
}
