use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::mode::AgentMode;

const DEFAULT_MODEL: &str = "gemini-2.5-flash";

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub mode: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            mode: "base".to_string(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        load_dotenv();

        let mut settings = Self::load_file().unwrap_or_default();

        if settings.api_key.is_empty() {
            if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                settings.api_key = key;
            }
        }

        settings
    }

    pub fn save(&self) -> Result<(), AppError> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = toml::to_string_pretty(self)
            .map_err(|e| AppError::Config(format!("Failed to encode settings: {e}")))?;
        fs::write(&path, data)?;
        Ok(())
    }

    pub fn mode(&self) -> AgentMode {
        AgentMode::from_str(&self.mode).unwrap_or(AgentMode::Base)
    }

    pub fn set_mode(&mut self, mode: AgentMode) {
        self.mode = mode.label().to_string();
    }

    pub fn has_token(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    pub fn model() -> &'static str {
        DEFAULT_MODEL
    }

    pub fn masked_token(&self) -> String {
        let key = self.api_key.trim();
        if key.is_empty() {
            return "not set".to_string();
        }
        if key.len() <= 8 {
            return "••••••••".to_string();
        }
        format!("{}…{}", &key[..4], &key[key.len() - 4..])
    }

    fn load_file() -> Option<Self> {
        let path = settings_path();
        let data = fs::read_to_string(path).ok()?;
        toml::from_str(&data).ok()
    }
}

fn load_dotenv() {
    let _ = dotenvy::dotenv();
    if let Ok(cwd) = std::env::current_dir() {
        let _ = dotenvy::from_path(cwd.join(".env"));
    }
}

pub fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("godmode")
        .join("config.toml")
}

pub fn is_rate_limit_message(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("429")
        || lower.contains("resource_exhausted")
        || lower.contains("quota exceeded")
        || lower.contains("quota limit")
        || lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("too many requests")
}

pub fn is_auth_error_message(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("api key")
        || lower.contains("apikey")
        || lower.contains("api_key_invalid")
        || lower.contains("permission denied")
        || lower.contains("401")
        || lower.contains("403")
}

pub fn sanitize_error(msg: &str) -> String {
    let lower = msg.to_lowercase();
    if is_auth_error_message(msg) {
        return "Invalid or missing API token. Use /token to set one.".to_string();
    }
    if is_rate_limit_message(msg) {
        return "API rate limit reached. Try again shortly.".to_string();
    }
    if lower.contains("network") || lower.contains("connection") || lower.contains("timeout") {
        return "Connection failed. Check your network.".to_string();
    }
    if lower.contains("empty") {
        return "No response received. Try again.".to_string();
    }
    if msg.len() > 120 {
        return "Something went wrong. Check /settings.".to_string();
    }
    msg.to_string()
}
