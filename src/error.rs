use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("{0}")]
    Fingerprint(String),
    #[error("input tidak valid: {0}")]
    InvalidInput(String),
    #[error("operasi fingerprint gagal dijalankan: {0}")]
    Task(String),
    #[error("fingerprint USB hanya didukung di macOS, Linux, dan Windows")]
    UnsupportedPlatform,
}

#[cfg(desktop)]
impl From<crate::uru4500::Error> for Error {
    fn from(error: crate::uru4500::Error) -> Self {
        Self::Fingerprint(error.to_string())
    }
}

#[cfg(desktop)]
impl From<crate::template::Error> for Error {
    fn from(error: crate::template::Error) -> Self {
        Self::Fingerprint(error.to_string())
    }
}

#[cfg(mobile)]
impl From<tauri::plugin::mobile::PluginInvokeError> for Error {
    fn from(error: tauri::plugin::mobile::PluginInvokeError) -> Self {
        Self::Fingerprint(error.to_string())
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
