use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Connection error")]
    Http(#[from] reqwest::Error),

    #[error("{0}")]
    Api(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("{0}")]
    Tool(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Denied by user")]
    Denied,
}
