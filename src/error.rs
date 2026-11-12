use thiserror::Error;

/// Unified error type across the terminal card game.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Pack generation error: {0}")]
    PackGeneration(String),

    #[error("Terminal graphics error: {0}")]
    TerminalGraphics(String),
}

pub type AppResult<T> = Result<T, AppError>;

