//! Workout session management operations.

use crate::db::models::WorkoutSession;
use crate::db::operations::{
    check_in_progress_workout_exists, complete_workout_session, create_workout_session,
    get_in_progress_workout, get_workout_session, update_workout_duration,
};
use crate::session::Session;
use anyhow::Result;

impl Session {
    /// Delete a workout session by ID.
    pub async fn delete_workout(&self, workout_id: i64) -> Result<u64> {
        crate::db::operations::delete_workout_session(&self.db_pool, workout_id).await
    }

    /// Set the active workout ID for this session.
    pub async fn set_workout_id(&self, workout_id: i64) -> Result<()> {
        let _ = get_workout_session(&self.db_pool, workout_id).await?;
        *self.workout_id.lock().await = Some(workout_id);
        Ok(())
    }

    /// Create a new workout session, completing any existing in-progress workout.
    pub async fn new_workout(&self) -> Result<bool> {
        let had_existing = check_in_progress_workout_exists(&self.db_pool).await?;

        if had_existing {
            if let Some(existing_workout) = get_in_progress_workout(&self.db_pool).await? {
                complete_workout_session(&self.db_pool, existing_workout.id, 0).await?;
                let current_id = self.get_workout_id().await;
                if current_id == Some(existing_workout.id) {
                    *self.workout_id.lock().await = None;
                }
            }
        }

        let workout = create_workout_session(
            &self.db_pool,
            None,
            None,
            None,
            None,
            Some("in_progress".to_string()),
        )
        .await?;
        self.set_workout_id(workout.id).await?;
        Ok(had_existing)
    }

    /// Create a new workout session with a name.
    pub async fn new_workout_with_name(&self, name: &str) -> Result<bool> {
        let had_existing = check_in_progress_workout_exists(&self.db_pool).await?;

        if had_existing {
            if let Some(existing_workout) = get_in_progress_workout(&self.db_pool).await? {
                complete_workout_session(&self.db_pool, existing_workout.id, 0).await?;
                let current_id = self.get_workout_id().await;
                if current_id == Some(existing_workout.id) {
                    *self.workout_id.lock().await = None;
                }
            }
        }

        let workout = create_workout_session(
            &self.db_pool,
            None,
            Some(name.to_string()),
            None,
            None,
            Some("in_progress".to_string()),
        )
        .await?;
        self.set_workout_id(workout.id).await?;
        Ok(had_existing)
    }

    /// Get the current workout session.
    pub async fn get_workout_session(&self) -> Result<WorkoutSession> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            get_workout_session(&self.db_pool, workout_id).await
        } else {
            Err(anyhow::anyhow!("No active workout"))
        }
    }

    /// Get all completed workout sessions.
    pub async fn get_all_workouts(&self) -> Result<Vec<WorkoutSession>> {
        crate::db::operations::get_all_workout_sessions(&self.db_pool, Some("completed")).await
    }

    /// Get all workout sessions including in-progress ones.
    pub async fn get_all_workouts_including_in_progress(&self) -> Result<Vec<WorkoutSession>> {
        crate::db::operations::get_all_workout_sessions(&self.db_pool, None).await
    }

    /// Get the in-progress workout if one exists.
    pub async fn get_in_progress_workout(&self) -> Result<Option<WorkoutSession>> {
        get_in_progress_workout(&self.db_pool).await
    }

    /// Complete the current workout session.
    pub async fn complete_workout(&self, duration_seconds: i64) -> Result<()> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            complete_workout_session(&self.db_pool, workout_id, duration_seconds).await?;
            *self.workout_id.lock().await = None;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active workout to complete"))
        }
    }

    /// Update the elapsed time for the current workout.
    pub async fn update_workout_elapsed_time(&self, elapsed_seconds: i64) -> Result<()> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            update_workout_duration(&self.db_pool, workout_id, elapsed_seconds).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active workout to update"))
        }
    }

    /// Check if an in-progress workout exists.
    pub async fn check_in_progress_workout_exists(&self) -> Result<bool> {
        check_in_progress_workout_exists(&self.db_pool).await
    }
}
