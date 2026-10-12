//! Core implementation for the FantasyLCSC command-line tool.
pub mod browser;
pub mod http;
pub mod lcsc;
pub mod maintenance;
pub mod models;

use serde::Serialize;
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize)]
pub struct Error {
    pub code: String,
    pub message: String,
    #[serde(skip)]
    pub fallback: bool,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            fallback: false,
        }
    }
    pub fn fallback(mut self) -> Self {
        self.fallback = true;
        self
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_argument", message)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new("io_error", e.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::new("schema_changed", format!("JSON 格式异常：{e}"))
    }
}

pub fn fetched_at() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Write on the same filesystem and only publish complete files.
pub fn atomic_write(path: &std::path::Path, bytes: &[u8], overwrite: bool) -> Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    if overwrite {
        temp.persist(path).map_err(|e| Error::from(e.error))?;
    } else {
        temp.persist_noclobber(path)
            .map_err(|e| Error::from(e.error))?;
    }
    Ok(())
}
