use crate::errors::AppError;
use tokio::process::Command;

pub async fn run(command: &str) -> Result<String, AppError> {
    // No allowlist — full unrestricted access
    // Gemini can run ANY command on this machine
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .output()
            .await
            .map_err(|e| AppError::Tool(format!("Failed to run: {e}")))?
    } else {
        Command::new("sh")
            .args(["-c", command])
            .output()
            .await
            .map_err(|e| AppError::Tool(format!("Failed to run: {e}")))?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    let mut combined = String::new();
    if !stdout.is_empty() {
        combined.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !combined.is_empty() && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }

    if combined.is_empty() {
        combined = format!("Command finished with exit code {code} (no output)");
    } else if !output.status.success() {
        combined.push_str(&format!("\n[exit code: {code}]"));
    }

    Ok(combined)
}
