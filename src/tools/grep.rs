use std::path::{Path, PathBuf};

use tokio::fs;
use walkdir::WalkDir;

use crate::errors::AppError;

const MAX_MATCHES: usize = 500;
const MAX_FILE_SIZE: u64 = 2_000_000;

pub async fn grep(path: &str, pattern: &str, ignore_case: bool) -> Result<String, AppError> {
    if path.is_empty() {
        return Err(AppError::Tool("Missing path".into()));
    }
    if pattern.is_empty() {
        return Err(AppError::Tool("Missing pattern".into()));
    }

    let meta = fs::metadata(path)
        .await
        .map_err(|e| AppError::Tool(format!("Path not found '{path}': {e}")))?;

    let matches = if meta.is_file() {
        let mut m = Vec::new();
        grep_file(Path::new(path), pattern, ignore_case, &mut m).await?;
        m
    } else if meta.is_dir() {
        let root = PathBuf::from(path);
        let pat = pattern.to_string();
        let ic = ignore_case;
        tokio::task::spawn_blocking(move || grep_dir(&root, &pat, ic))
            .await
            .map_err(|e| AppError::Tool(format!("Grep task failed: {e}")))??
    } else {
        return Err(AppError::Tool(format!("Not a file or directory: {path}")));
    };

    if matches.is_empty() {
        return Ok(format!("No matches for '{pattern}' in '{path}'"));
    }

    let capped = matches.len() >= MAX_MATCHES;
    let mut out = matches.join("\n");
    if capped {
        out.push_str(&format!("\n… capped at {MAX_MATCHES} matches"));
    }
    Ok(out)
}

fn grep_dir(root: &Path, pattern: &str, ignore_case: bool) -> Result<Vec<String>, AppError> {
    let mut matches = Vec::new();
    for entry in WalkDir::new(root).max_depth(12).follow_links(false) {
        let entry = entry.map_err(|e| AppError::Tool(format!("Walk error: {e}")))?;
        if !entry.file_type().is_file() {
            continue;
        }
        grep_file_sync(entry.path(), pattern, ignore_case, &mut matches)?;
        if matches.len() >= MAX_MATCHES {
            break;
        }
    }
    Ok(matches)
}

async fn grep_file(
    path: &Path,
    pattern: &str,
    ignore_case: bool,
    matches: &mut Vec<String>,
) -> Result<(), AppError> {
    grep_file_sync(path, pattern, ignore_case, matches)
}

fn grep_file_sync(
    path: &Path,
    pattern: &str,
    ignore_case: bool,
    matches: &mut Vec<String>,
) -> Result<(), AppError> {
    let meta = std::fs::metadata(path)
        .map_err(|e| AppError::Tool(format!("Failed to stat '{}': {e}", path.display())))?;
    if meta.len() > MAX_FILE_SIZE {
        return Ok(());
    }

    let text = std::fs::read_to_string(path)
        .map_err(|e| AppError::Tool(format!("Failed to read '{}': {e}", path.display())))?;

    let needle = if ignore_case {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };

    for (i, line) in text.lines().enumerate() {
        let hay = if ignore_case {
            line.to_lowercase()
        } else {
            line.to_string()
        };
        if hay.contains(&needle) {
            matches.push(format!("{}:{}:{}", path.display(), i + 1, line));
            if matches.len() >= MAX_MATCHES {
                return Ok(());
            }
        }
    }
    Ok(())
}
