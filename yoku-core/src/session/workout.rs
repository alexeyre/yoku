use crate::db::models::{WorkoutSession, WorkoutStatus};
use crate::db::operations::{
    check_in_progress_workout_exists, complete_workout_session, create_workout_session,
    get_in_progress_workout, get_workout_session, update_workout_duration,
};
use crate::session::Session;
use anyhow::Result;

impl Session {
    pub async fn delete_workout(&self, workout_id: i64) -> Result<u64> {
        crate::db::operations::delete_workout_session(&self.db_pool, workout_id).await
    }

    pub async fn set_workout_id(&self, workout_id: i64) -> Result<()> {
        let _ = get_workout_session(&self.db_pool, workout_id).await?;
        *self.workout_id.lock().await = Some(workout_id);
        Ok(())
    }

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
            Some(WorkoutStatus::InProgress),
        )
        .await?;
        self.set_workout_id(workout.id).await?;
        Ok(had_existing)
    }

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
            Some(WorkoutStatus::InProgress),
        )
        .await?;
        self.set_workout_id(workout.id).await?;
        Ok(had_existing)
    }

    pub async fn get_workout_session(&self) -> Result<WorkoutSession> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            get_workout_session(&self.db_pool, workout_id).await
        } else {
            Err(anyhow::anyhow!("No active workout"))
        }
    }

    pub async fn get_all_workouts(&self) -> Result<Vec<WorkoutSession>> {
        crate::db::operations::get_all_workout_sessions(
            &self.db_pool,
            Some(WorkoutStatus::Completed),
        )
        .await
    }

    pub async fn get_all_workouts_including_in_progress(&self) -> Result<Vec<WorkoutSession>> {
        crate::db::operations::get_all_workout_sessions(&self.db_pool, None).await
    }

    pub async fn get_in_progress_workout(&self) -> Result<Option<WorkoutSession>> {
        get_in_progress_workout(&self.db_pool).await
    }

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

    pub async fn update_workout_elapsed_time(&self, elapsed_seconds: i64) -> Result<()> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            update_workout_duration(&self.db_pool, workout_id, elapsed_seconds).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active workout to update"))
        }
    }

    pub async fn check_in_progress_workout_exists(&self) -> Result<bool> {
        check_in_progress_workout_exists(&self.db_pool).await
    }
}
