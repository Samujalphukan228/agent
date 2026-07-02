use std::path::PathBuf;

use glob::Pattern;
use walkdir::WalkDir;

use crate::errors::AppError;

const MAX_RESULTS: usize = 200;

pub async fn search(root: &str, pattern: &str, max_depth: usize) -> Result<String, AppError> {
    if root.is_empty() {
        return Err(AppError::Tool("Missing root path".into()));
    }
    if pattern.is_empty() {
        return Err(AppError::Tool("Missing pattern".into()));
    }

    let glob = Pattern::new(pattern)
        .map_err(|e| AppError::Tool(format!("Invalid glob pattern '{pattern}': {e}")))?;

    let root_buf = PathBuf::from(root);
    let depth = max_depth.clamp(1, 32);

    let matches = tokio::task::spawn_blocking(move || {
        let mut out = Vec::new();
        for entry in WalkDir::new(&root_buf)
            .max_depth(depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if glob.matches(name) {
                out.push(path.display().to_string());
                if out.len() >= MAX_RESULTS {
                    break;
                }
            }
        }
        out
    })
    .await
    .map_err(|e| AppError::Tool(format!("Search task failed: {e}")))?;

    if matches.is_empty() {
        return Ok(format!("No files matching '{pattern}' under '{root}'"));
    }

    let mut result = matches.join("\n");
    if matches.len() >= MAX_RESULTS {
        result.push_str(&format!("\n… capped at {MAX_RESULTS} results"));
    }
    Ok(result)
}
