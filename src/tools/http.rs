use reqwest::Client;

use crate::errors::AppError;

const MAX_BODY: usize = 50_000;
const TIMEOUT_SECS: u64 = 30;

pub async fn get(url: &str) -> Result<String, AppError> {
    if url.is_empty() {
        return Err(AppError::Tool("Missing url".into()));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::Tool(
            "URL must start with http:// or https://".into(),
        ));
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .user_agent("godmode-agent/0.1")
        .build()
        .map_err(|e| AppError::Tool(format!("HTTP client error: {e}")))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Tool(format!("Request failed: {e}")))?;

    let status = response.status();
    let headers: Vec<String> = response
        .headers()
        .iter()
        .map(|(k, v)| format!("{k}: {}", v.to_str().unwrap_or("?")))
        .collect();

    let body = response
        .text()
        .await
        .map_err(|e| AppError::Tool(format!("Failed to read body: {e}")))?;

    let body_preview = if body.len() > MAX_BODY {
        format!(
            "{}\n… body truncated ({} bytes total)",
            &body[..MAX_BODY],
            body.len()
        )
    } else {
        body
    };

    Ok(format!(
        "status: {status}\nheaders:\n{}\n\nbody:\n{body_preview}",
        headers.join("\n")
    ))
}
