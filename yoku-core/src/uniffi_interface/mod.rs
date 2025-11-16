use crate::{session::Session, *};
use anyhow::Result;
use thiserror::Error as ThisError;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

use log::*;
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

#[derive(uniffi::Object)]
struct Exercise {
    id: u32,
    name: String,
}

#[derive(uniffi::Object)]
pub struct ExerciseSet {
    id: i32,
    exercise_id: i32,
}

#[uniffi::export]
impl Exercise {
    fn id(&self) -> u32 {
        self.id
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

#[uniffi::export]
impl ExerciseSet {
    fn id(&self) -> i32 {
        self.id
    }
    fn exercise_id(&self) -> i32 {
        self.exercise_id
    }
}

#[uniffi::export]
pub async fn create_session(db_path: &str, model: String) -> Result<Session, YokuError> {
    let session = Session::new(db_path, model)
        .await
        .map_err(|e| YokuError::Common(e.to_string()))?;
    Ok(session)
}

#[uniffi::export]
pub async fn add_set_from_string(session: &Session, request_string: &str) -> Result<(), YokuError> {
    debug!("Adding set from string: {}", request_string);
    let rt = crate::runtime::init_global_runtime().await;
    rt.block_on(session.add_set_from_string(request_string))
        .map_err(|e| YokuError::Common(e.to_string()))
}

#[uniffi::export]
pub async fn get_all_sets(
    session: &Session,
) -> Result<Vec<std::sync::Arc<ExerciseSet>>, YokuError> {
    let sets = session
        .get_all_sets()
        .await
        .map_err(|e| YokuError::Common(e.to_string()))?;

    let converted: Vec<std::sync::Arc<ExerciseSet>> = sets
        .into_iter()
        .map(|ws| {
            std::sync::Arc::new(ExerciseSet {
                id: ws.id,
                exercise_id: ws.exercise_id,
            })
        })
        .collect();

    Ok(converted)
}

#[uniffi::export]
async fn hello_session(session: std::sync::Arc<Session>) -> String {
    match session.get_workout_id().await {
        Some(id) => id.to_string(),
        None => "No workout active".to_string(),
    }
}

#[uniffi::export]
fn set_debug_log_level() {
    // Initialize a logger that writes to stdout and set global level to Trace.
    // Use try_init so repeated calls (or if the embedding app already initialized logging)
    // won't panic — we just ignore initialization errors.
    let _ = env_logger::Builder::new()
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "{}: {} - {}",
                record.level(),
                record.target(),
                record.args()
            )
        })
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Trace)
        .try_init();
    log::set_max_level(log::LevelFilter::Trace);
    debug!("Debug log level set to Trace (stdout)");
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
