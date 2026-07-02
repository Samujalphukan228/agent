use std::path::Path;

use tokio::fs;

use crate::errors::AppError;

pub async fn read(path: &str) -> Result<String, AppError> {
    fs::read_to_string(path)
        .await
        .map_err(|e| AppError::Tool(format!("Failed to read file '{path}': {e}")))
}

pub async fn write(path: &str, content: &str) -> Result<String, AppError> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await.ok();
        }
    }
    fs::write(path, content)
        .await
        .map_err(|e| AppError::Tool(format!("Failed to write file '{path}': {e}")))?;

    Ok(format!("Successfully wrote to '{path}'"))
}

pub async fn list(path: &str) -> Result<String, AppError> {
    let mut entries = fs::read_dir(path)
        .await
        .map_err(|e| AppError::Tool(format!("Failed to read directory '{path}': {e}")))?;

    let mut files = Vec::new();

    loop {
        match entries.next_entry().await {
            Ok(Some(entry)) => {
                files.push(entry.file_name().to_string_lossy().to_string());
            }
            Ok(None) => break,
            Err(e) => {
                return Err(AppError::Tool(format!("Failed to read entry: {e}")));
            }
        }
    }

    if files.is_empty() {
        return Ok(format!("Directory '{path}' is empty"));
    }

    files.sort();
    Ok(files.join("\n"))
}

pub async fn edit(
    path: &str,
    old_text: &str,
    new_text: &str,
    replace_all: bool,
) -> Result<String, AppError> {
    if path.is_empty() {
        return Err(AppError::Tool("Missing path".into()));
    }
    if old_text.is_empty() {
        return Err(AppError::Tool("Missing old_text".into()));
    }

    let content = read(path).await?;
    if !content.contains(old_text) {
        return Err(AppError::Tool(format!("old_text not found in '{path}'")));
    }

    let updated = if replace_all {
        content.replace(old_text, new_text)
    } else {
        content.replacen(old_text, new_text, 1)
    };

    write(path, &updated).await?;
    let count = if replace_all {
        content.matches(old_text).count()
    } else {
        1
    };
    Ok(format!("Edited '{path}' ({count} replacement(s))"))
}

pub async fn delete_path(path: &str) -> Result<String, AppError> {
    if path.is_empty() {
        return Err(AppError::Tool("Missing path".into()));
    }

    let meta = fs::metadata(path)
        .await
        .map_err(|e| AppError::Tool(format!("Path not found '{path}': {e}")))?;

    if meta.is_dir() {
        fs::remove_dir_all(path)
            .await
            .map_err(|e| AppError::Tool(format!("Failed to delete directory '{path}': {e}")))?;
        Ok(format!("Deleted directory '{path}'"))
    } else {
        fs::remove_file(path)
            .await
            .map_err(|e| AppError::Tool(format!("Failed to delete file '{path}': {e}")))?;
        Ok(format!("Deleted file '{path}'"))
    }
}

pub async fn create_dir(path: &str) -> Result<String, AppError> {
    if path.is_empty() {
        return Err(AppError::Tool("Missing path".into()));
    }

    fs::create_dir_all(path)
        .await
        .map_err(|e| AppError::Tool(format!("Failed to create directory '{path}': {e}")))?;

    Ok(format!("Created directory '{path}'"))
}

pub async fn move_path(from: &str, to: &str) -> Result<String, AppError> {
    if from.is_empty() || to.is_empty() {
        return Err(AppError::Tool("Missing from or to path".into()));
    }

    if let Some(parent) = Path::new(to).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await.ok();
        }
    }

    fs::rename(from, to)
        .await
        .map_err(|e| AppError::Tool(format!("Failed to move '{from}' → '{to}': {e}")))?;

    Ok(format!("Moved '{from}' → '{to}'"))
}
