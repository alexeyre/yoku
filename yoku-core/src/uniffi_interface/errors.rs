use thiserror::Error as ThisError;
use uniffi::Error;

#[derive(Debug, ThisError, Error)]
#[non_exhaustive]
pub enum YokuError {
    #[error("error: {0}")]
    Common(String),
}

impl From<anyhow::Error> for YokuError {
    fn from(e: anyhow::Error) -> Self {
        YokuError::Common(e.to_string())
    }
}

impl From<String> for YokuError {
    fn from(s: String) -> Self {
        YokuError::Common(s)
    }
}

impl From<&str> for YokuError {
    fn from(s: &str) -> Self {
        YokuError::Common(s.to_string())
    }
}

impl YokuError {
        pub fn with_display<D: std::fmt::Display>(d: D) -> Self {
        YokuError::Common(d.to_string())
    }
}
