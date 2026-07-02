use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::types::*;
use crate::errors::AppError;
use crate::settings::{is_auth_error_message, is_rate_limit_message};

#[derive(Debug, Deserialize)]
struct GoogleApiErrorResponse {
    error: GoogleApiErrorBody,
}

#[derive(Debug, Deserialize)]
struct GoogleApiErrorBody {
    message: Option<String>,
    status: Option<String>,
}

fn parse_api_error(status: u16, body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<GoogleApiErrorResponse>(body) {
        let message = parsed.error.message.unwrap_or_default();
        let api_status = parsed.error.status.unwrap_or_default().to_ascii_uppercase();

        if is_auth_error_message(&message)
            || api_status == "UNAUTHENTICATED"
            || api_status == "PERMISSION_DENIED"
            || (status == 400 && message.to_lowercase().contains("api key"))
        {
            return "Invalid or missing API token. Use /token to set one.".to_string();
        }

        if status == 429 || api_status == "RESOURCE_EXHAUSTED" || is_rate_limit_message(&message) {
            if message.contains("limit: 0") {
                return format!(
                    "Free tier quota for this model is unavailable on your API key. \
                     The app now uses {} — restart godmode and try again. \
                     Or enable billing at ai.google.dev.",
                    crate::settings::Settings::model()
                );
            }
            return "API rate limit reached. Wait a minute and try again.".to_string();
        }

        if !message.is_empty() {
            return truncate_message(&message, 200);
        }
    }

    classify_raw_api_error(status, body)
}

fn classify_raw_api_error(status: u16, body: &str) -> String {
    if status == 401 || status == 403 {
        return "Invalid or missing API token. Use /token to set one.".to_string();
    }
    if status == 429 {
        return "API rate limit reached. Try again shortly.".to_string();
    }
    if is_auth_error_message(body) {
        return "Invalid or missing API token. Use /token to set one.".to_string();
    }
    if is_rate_limit_message(body) {
        return "API rate limit reached. Try again shortly.".to_string();
    }
    truncate_message(body, 200)
}

fn truncate_message(msg: &str, max: usize) -> String {
    if msg.len() <= max {
        msg.to_string()
    } else {
        format!("{}…", &msg[..max])
    }
}

fn tool_declarations() -> Vec<FunctionDeclaration> {
    vec![
        FunctionDeclaration {
            name: "run_shell".into(),
            description: "Runs a shell command and returns stdout/stderr.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to execute" }
                },
                "required": ["command"]
            }),
        },
        FunctionDeclaration {
            name: "read_file".into(),
            description: "Reads a file and returns its contents.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Full file path" }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "write_file".into(),
            description: "Writes content to a file (create or overwrite).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "list_dir".into(),
            description: "Lists files and directories at a path.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "search_files".into(),
            description: "Find files by glob pattern under a root directory.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "root": { "type": "string", "description": "Directory to search under" },
                    "pattern": { "type": "string", "description": "Glob pattern e.g. *.rs" },
                    "max_depth": { "type": "integer", "description": "Max depth (default 8)" }
                },
                "required": ["root", "pattern"]
            }),
        },
        FunctionDeclaration {
            name: "grep".into(),
            description: "Search for a text pattern in a file or directory.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File or directory path" },
                    "pattern": { "type": "string", "description": "Text to search for" },
                    "ignore_case": { "type": "boolean", "description": "Case insensitive (default false)" }
                },
                "required": ["path", "pattern"]
            }),
        },
        FunctionDeclaration {
            name: "edit_file".into(),
            description: "Replace text in a file without rewriting the whole file.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old_text": { "type": "string", "description": "Text to find" },
                    "new_text": { "type": "string", "description": "Replacement text" },
                    "replace_all": { "type": "boolean", "description": "Replace all occurrences (default false)" }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
        FunctionDeclaration {
            name: "delete_file".into(),
            description: "Delete a file or directory.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "create_dir".into(),
            description: "Create a directory (mkdir -p).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "move_file".into(),
            description: "Move or rename a file or directory.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string" },
                    "to": { "type": "string" }
                },
                "required": ["from", "to"]
            }),
        },
        FunctionDeclaration {
            name: "http_get".into(),
            description: "Fetch a URL and return status, headers, and body.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "http:// or https:// URL" }
                },
                "required": ["url"]
            }),
        },
        FunctionDeclaration {
            name: "env_info".into(),
            description: "Return structured OS, user, memory, and disk info.".into(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

pub async fn chat(
    client: &Client,
    api_key: &str,
    model: &str,
    system_prompt: Option<&str>,
    messages: &[Message],
) -> Result<GeminiResponse, AppError> {
    let contents: Vec<GeminiMessage> = messages.iter().map(message_to_gemini).collect();

    let system_instruction = system_prompt.map(|p| SystemInstruction {
        parts: vec![RequestPart {
            text: Some(p.to_string()),
            function_call: None,
            function_response: None,
        }],
    });

    let tools = Some(vec![Tool {
        function_declarations: tool_declarations(),
    }]);

    let body = GeminiRequest {
        system_instruction,
        contents,
        tools,
    };

    let url =
        format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent");

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", api_key)
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_else(|_| String::new());
        return Err(AppError::Api(parse_api_error(status, &error_text)));
    }

    let parsed: GeminiResponse = response.json().await?;
    Ok(parsed)
}
