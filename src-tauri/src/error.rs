use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("TOML error: {0}")]
    Toml(String),

    #[error("INI error: {0}")]
    Ini(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("SHA256 mismatch: expected {expected}, got {actual}")]
    Sha256Mismatch { expected: String, actual: String },

    #[error("Admin privileges required")]
    AdminRequired,

    #[error("Process execution error: {0}")]
    Process(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Offline mode: {0}")]
    Offline(String),

    #[error("File in use: {0}")]
    FileInUse(String),

    #[error("Disk full: need {needed} bytes, only {available} bytes available")]
    DiskFull { needed: u64, available: u64 },

    #[error("{0}")]
    Generic(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        AppError::Toml(e.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(e: toml::ser::Error) -> Self {
        AppError::Toml(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Http(e.to_string())
    }
}

// Implement Serialize for Tauri command results
impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}
