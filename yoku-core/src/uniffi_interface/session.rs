use crate::db;
use crate::db::models::UpdateWorkoutSet;
use crate::session::Session;
use crate::uniffi_interface::errors::YokuError;
use crate::uniffi_interface::modifications::{Modification, UpdateWorkoutSetResult};
use crate::uniffi_interface::objects::{
    ActiveWorkoutState, Exercise, WorkoutSession, WorkoutSet, WorkoutSuggestion, WorkoutSummary,
};
use std::sync::Arc;

#[uniffi::export]
pub async fn create_session(
    db_path: &str,
    model: String,
) -> std::result::Result<Session, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let session = rt.block_on(Session::new(db_path, model))?;
    Ok(session)
}

#[uniffi::export]
pub async fn reset_database(session: &Session) -> std::result::Result<(), YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(async {
        db::drop_all_tables(&session.db_pool).await?;
        db::init_database(&session.db_pool).await?;
        Ok::<(), crate::uniffi_interface::errors::YokuError>(())
    })?;
    Ok(())
}

#[derive(uniffi::Object)]
pub struct LiftDataPoint {
    pub timestamp: i64,
    pub lift: f64,
}
#[uniffi::export]
impl LiftDataPoint {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn lift(&self) -> f64 {
        self.lift
    }
}

#[uniffi::export]
pub async fn delete_workout(session: &Session, id: i64) -> std::result::Result<u64, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.delete_workout(id))
        .map_err(|e| e.into())
}

#[uniffi::export]
pub async fn delete_set_from_workout(
    session: &Session,
    id: i64,
) -> std::result::Result<u64, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.delete_set(id)).map_err(|e| e.into())
}

#[uniffi::export]
pub async fn get_lifts_for_exercise(
    session: &Session,
    exercise_id: i64,
    limit: Option<i64>,
) -> std::result::Result<Vec<f64>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let sets = rt.block_on(session.get_sets_for_exercise(exercise_id, limit))?;

    let converted: Vec<f64> = sets.into_iter().map(|lift| lift.weight).collect();

    Ok(converted)
}

#[uniffi::export]
pub async fn delete_workout_session(session: &Session, id: i64) -> Result<(), YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.delete_workout(id))?;
    Ok(())
}

#[uniffi::export]
pub async fn delete_workout_set(
    session: &Session,
    id: i64,
) -> std::result::Result<Vec<Modification>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let modifications = rt.block_on(session.delete_set_with_modifications(id))?;
    Ok(modifications)
}

#[uniffi::export]
pub async fn get_all_workout_sessions(
    session: &Session,
) -> std::result::Result<Vec<Arc<WorkoutSession>>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let workouts = rt.block_on(session.get_all_workouts_including_in_progress())?;

    let converted: Vec<Arc<WorkoutSession>> = workouts
        .into_iter()
        .map(WorkoutSession::try_from)
        .collect::<Result<Vec<WorkoutSession>, YokuError>>()?
        .into_iter()
        .map(Arc::new)
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
                weight: ws.weight,
                reps: ws.reps,
                rpe: ws.rpe,
                notes: ws.notes,
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
    id: i64,
) -> std::result::Result<(), YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.set_workout_id(id))?;
    Ok(())
}

#[uniffi::export]
pub async fn create_blank_workout_session(
    session: &Session,
) -> std::result::Result<bool, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let had_existing = rt.block_on(session.new_workout())?;
    Ok(had_existing)
}

#[uniffi::export]
pub async fn complete_workout_session(
    session: &Session,
    duration_seconds: i64,
) -> std::result::Result<(), YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.complete_workout(duration_seconds))?;
    Ok(())
}

#[uniffi::export]
pub async fn get_in_progress_workout_session(
    session: &Session,
) -> std::result::Result<Option<Arc<WorkoutSession>>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let workout = rt.block_on(session.get_in_progress_workout())?;
    match workout {
        Some(w) => {
            let workout_uniffi: WorkoutSession = w.try_into()?;
            Ok(Some(Arc::new(workout_uniffi)))
        }
        None => Ok(None),
    }
}

#[uniffi::export]
pub async fn check_in_progress_workout_exists(
    session: &Session,
) -> std::result::Result<bool, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let exists = rt.block_on(session.check_in_progress_workout_exists())?;
    Ok(exists)
}

#[uniffi::export]
pub async fn update_workout_elapsed_time(
    session: &Session,
    elapsed_seconds: i64,
) -> std::result::Result<(), YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    rt.block_on(session.update_workout_elapsed_time(elapsed_seconds))?;
    Ok(())
}

#[uniffi::export]
pub async fn update_workout_set(
    session: &Session,
    set_id: i64,
    reps: Option<i64>,
    weight: Option<f64>,
) -> std::result::Result<UpdateWorkoutSetResult, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let update = UpdateWorkoutSet {
        reps,
        weight,
        ..Default::default()
    };
    let (workout_db, modifications) =
        rt.block_on(session.update_workout_set_with_modifications(set_id, &update))?;
    let workout_uniffi: WorkoutSet = workout_db.into();
    Ok(UpdateWorkoutSetResult {
        set: Arc::new(workout_uniffi),
        modifications,
    })
}

#[uniffi::export]
pub async fn get_session_workout_session(
    session: &Session,
) -> std::result::Result<WorkoutSession, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let workout_db = rt.block_on(session.get_workout_session())?;
    let workout_uniffi: WorkoutSession = workout_db.try_into()?;
    Ok(workout_uniffi)
}

#[uniffi::export]
pub async fn get_workout_suggestions(
    session: &Session,
) -> std::result::Result<Vec<Arc<WorkoutSuggestion>>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let suggestions = rt.block_on(session.get_workout_suggestions())?;
    let converted: Vec<Arc<WorkoutSuggestion>> = suggestions
        .into_iter()
        .map(|s| Arc::new(WorkoutSuggestion::from(s)))
        .collect();
    Ok(converted)
}

#[uniffi::export]
pub async fn get_workout_summary(
    session: &Session,
) -> std::result::Result<WorkoutSummary, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let summary = rt.block_on(session.get_workout_summary())?;
    Ok(WorkoutSummary::from(summary))
}

#[uniffi::export]
pub async fn classify_and_process_input(
    session: &Session,
    input: &str,
    selected_set_backend_id: Option<i64>,
    visible_set_backend_ids: Vec<i64>,
) -> std::result::Result<Vec<Modification>, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let modifications = rt.block_on(session.process_user_input(
        input,
        selected_set_backend_id,
        visible_set_backend_ids,
    ))?;
    Ok(modifications)
}

#[uniffi::export]
pub async fn get_active_workout_state(
    session: &Session,
) -> std::result::Result<ActiveWorkoutState, YokuError> {
    let rt = crate::runtime::init_global_runtime_blocking();
    let state = rt.block_on(session.get_active_workout_state())?;
    Ok(state)
}
