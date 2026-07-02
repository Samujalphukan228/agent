use std::env;

use tokio::fs;

use crate::errors::AppError;

pub async fn info() -> Result<String, AppError> {
    let mut lines = Vec::new();

    lines.push(format!("os: {}", env::consts::OS));
    lines.push(format!("arch: {}", env::consts::ARCH));
    lines.push(format!("family: {}", env::consts::FAMILY));

    if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
        lines.push(format!("user: {user}"));
    }
    if let Ok(home) = env::var("HOME") {
        lines.push(format!("home: {home}"));
    }
    if let Ok(shell) = env::var("SHELL") {
        lines.push(format!("shell: {shell}"));
    }
    if let Ok(cwd) = env::current_dir() {
        lines.push(format!("cwd: {}", cwd.display()));
    }

    if let Ok(host) = fs::read_to_string("/etc/hostname").await {
        lines.push(format!("hostname: {}", host.trim()));
    }

    if let Ok(mem) = read_meminfo().await {
        lines.push(mem);
    }

    if let Ok(disk) = read_disk().await {
        lines.push(disk);
    }

    Ok(lines.join("\n"))
}

async fn read_meminfo() -> Result<String, AppError> {
    let data = fs::read_to_string("/proc/meminfo")
        .await
        .map_err(|e| AppError::Tool(format!("meminfo: {e}")))?;

    let mut total = None;
    let mut avail = None;
    for line in data.lines() {
        if line.starts_with("MemTotal:") {
            total = Some(line.trim().to_string());
        } else if line.starts_with("MemAvailable:") {
            avail = Some(line.trim().to_string());
        }
    }

    Ok(match (total, avail) {
        (Some(t), Some(a)) => format!("memory: {t}, {a}"),
        _ => "memory: unavailable".to_string(),
    })
}

async fn read_disk() -> Result<String, AppError> {
    let output = tokio::process::Command::new("df")
        .args(["-h", "--output=source,size,used,avail,pcent,target", "/"])
        .output()
        .await
        .map_err(|e| AppError::Tool(format!("df failed: {e}")))?;

    if !output.status.success() {
        return Ok("disk: unavailable".to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1).unwrap_or("").trim();
    if line.is_empty() {
        Ok("disk: unavailable".to_string())
    } else {
        Ok(format!("disk (root): {line}"))
    }
}
