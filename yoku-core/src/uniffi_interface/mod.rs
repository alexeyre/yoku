use crate::*;
use anyhow::Result;
use thiserror::Error as ThisError;
use uniffi::Error;
#[derive(Debug, ThisError, Error)]
pub enum YokuError {
    #[error("error: {0}")]
    Common(String),
}
#[uniffi::export]
fn test() -> String {
    "Hello, World!".to_string()
}
#[uniffi::export]
async fn setup_database(path: &str) -> Result<(), YokuError> {
    db::set_db_path(path)
        .await
        .map_err(|e| YokuError::Common(e.to_string()))?;

    db::init_database()
        .await
        .map_err(|e| YokuError::Common(e.to_string()))?;

    Ok(())
}

#[uniffi::export]
async fn backend_logs_count() -> u32 {
    0
}

#[uniffi::export]
async fn backend_logs_since(start_index: u32) -> Vec<String> {
    vec![]
}

#[uniffi::export]
async fn start_blank_workout() -> u32 {
    0
}
