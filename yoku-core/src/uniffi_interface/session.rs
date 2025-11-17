use crate::db;
use crate::session::Session;
use crate::uniffi_interface::errors::YokuError;
use crate::uniffi_interface::objects::{Exercise, WorkoutSession, WorkoutSet};
use log::*;
use std::sync::Arc;

#[uniffi::export]
pub async fn create_session(
    db_path: &str,
    model: String,
) -> std::result::Result<Session, YokuError> {
    // Ensure a global runtime exists when being invoked from foreign runtimes.
    let rt = crate::runtime::init_global_runtime_blocking();
    let session = rt.block_on(Session::new(db_path, model))?;
    Ok(session)
}

#[uniffi::export]
pub async fn reset_database(session: &Session) -> std::result::Result<(), YokuError> {
    // Ensure runtime is initialized and run database reset on it.
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(async {
        // Use the runtime to get the global connection and run migrations/reset.
        let mut db_conn = crate::db::get_conn().await.lock().await;
        db::drop_all_tables(&mut db_conn).await?;
        db::init_database(&mut db_conn).await?;
        Ok::<(), crate::uniffi_interface::errors::YokuError>(())
    })?;
    Ok(())
}

#[uniffi::export]
pub async fn add_set_from_string(
    session: &Session,
    request_string: &str,
) -> std::result::Result<(), YokuError> {
    debug!("Adding set from string: {}", request_string);
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.add_set_from_string(request_string))?;
    Ok(())
}

#[uniffi::export]
pub async fn get_all_workout_sessions(
    session: &Session,
) -> std::result::Result<Vec<Arc<WorkoutSession>>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let workouts = rt.block_on(session.get_all_workouts())?;

    let converted: Vec<Arc<WorkoutSession>> = workouts
        .into_iter()
        .map(|ws| Arc::new(WorkoutSession::from(ws)))
        .collect();

    Ok(converted)
}

#[uniffi::export]
pub async fn get_all_sets(
    session: &Session,
) -> std::result::Result<Vec<Arc<WorkoutSet>>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let sets = rt.block_on(session.get_all_sets())?;

    let converted: Vec<Arc<WorkoutSet>> = sets
        .into_iter()
        .map(|ws| {
            Arc::new(WorkoutSet {
                id: ws.id,
                exercise_id: ws.exercise_id,
            })
        })
        .collect();

    Ok(converted)
}

#[uniffi::export]
pub async fn get_all_exercises(
    session: &Session,
) -> std::result::Result<Vec<Arc<Exercise>>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let exercises = rt.block_on(session.get_all_exercises())?;

    let converted: Vec<Arc<Exercise>> = exercises
        .into_iter()
        .map(|e| Arc::new(Exercise::from(e)))
        .collect();

    Ok(converted)
}

#[uniffi::export]
pub async fn set_session_workout_session_id(
    session: &Session,
    id: i32,
) -> std::result::Result<(), YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.set_workout_id(id))?;
    Ok(())
}

#[uniffi::export]
pub async fn create_blank_workout_session(session: &Session) -> std::result::Result<(), YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.new_workout())?;
    Ok(())
}

#[uniffi::export]
pub async fn get_session_workout_session(
    session: &Session,
) -> std::result::Result<WorkoutSession, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let workout_db = rt.block_on(session.get_workout_session())?;
    let workout_uniffi: WorkoutSession = workout_db.into();
    Ok(workout_uniffi)
}
