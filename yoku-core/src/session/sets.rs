use crate::db::models::{Exercise, UpdateWorkoutSet, WorkoutSet};
use crate::db::operations::{
    add_multiple_sets_to_workout, add_workout_set, create_request_string_for_username,
    delete_workout_set, get_exercise_entries, get_or_create_exercise, get_sets_for_session,
    update_workout_set, update_workout_set_from_parsed,
};
use crate::llm::ParsedSet;
use crate::session::Session;
use crate::uniffi_interface::modifications::{Modification, ModificationType};
use crate::uniffi_interface::objects::{
    Exercise as UniffiExercise, WorkoutSet as UniffiWorkoutSet,
};
use anyhow::Result;
use sqlx;
use std::sync::Arc;

impl Session {
    pub async fn delete_set(&self, set_id: i64) -> Result<u64> {
        delete_workout_set(&self.db_pool, set_id).await
    }

    pub async fn get_sets_for_exercise(
        &self,
        exercise_id: i64,
        limit: Option<i64>,
    ) -> Result<Vec<WorkoutSet>> {
        get_exercise_entries(&self.db_pool, exercise_id, limit).await
    }

    pub async fn get_all_sets(&self) -> Result<Vec<WorkoutSet>> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            get_sets_for_session(&self.db_pool, workout_id).await
        } else {
            Err(anyhow::anyhow!("No active workout"))
        }
    }

    pub async fn replace_set_from_parsed(&self, set_id: i64, parsed: &ParsedSet) -> Result<()> {
        update_workout_set_from_parsed(&self.db_pool, set_id, parsed).await?;
        Ok(())
    }

    pub async fn update_workout_set(
        &self,
        set_id: i64,
        update: &UpdateWorkoutSet,
    ) -> Result<WorkoutSet> {
        update_workout_set(&self.db_pool, set_id, update).await
    }

    pub async fn add_set_from_parsed(&self, parsed: &ParsedSet) -> Result<()> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        let request_str_content = if !parsed.original_string.is_empty() {
            parsed.original_string.clone()
        } else {
            format!(
                "{} {} reps rpe:{:?}",
                parsed.exercise,
                parsed.reps.unwrap_or(0),
                parsed.rpe
            )
        };

        let exercise_name = parsed.exercise.clone();
        let exercise = get_or_create_exercise(&self.db_pool, &exercise_name).await?;

        let weight = parsed.weight.unwrap_or(0.0) as f64;
        let reps = parsed.reps.unwrap_or(0) as i64;
        let set_count = parsed.set_count.unwrap_or(1).max(1) as i64;
        let parsed_rpe = parsed.rpe.map(|r| r as f64);

        let request =
            create_request_string_for_username(&self.db_pool, "cli", request_str_content.clone())
                .await?;

        if set_count > 1 {
            add_multiple_sets_to_workout(
                &self.db_pool,
                &session_id,
                &exercise.id,
                &request.id,
                &weight,
                &reps,
                parsed_rpe,
                set_count,
            )
            .await?;
        } else {
            add_workout_set(
                &self.db_pool,
                &session_id,
                &exercise.id,
                &request.id,
                &weight,
                &reps,
                parsed_rpe,
            )
            .await?;
        }

        Ok(())
    }

    async fn is_exercise_new_for_session(&self, exercise_id: i64) -> Result<bool> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            let existing_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM workout_sets WHERE session_id = ?1 AND exercise_id = ?2",
            )
            .bind(workout_id)
            .bind(exercise_id)
            .fetch_one(&self.db_pool)
            .await?;
            Ok(existing_count == 0)
        } else {
            Ok(false)
        }
    }

    pub async fn add_set_from_parsed_with_modifications(
        &self,
        parsed: &ParsedSet,
    ) -> Result<Vec<Modification>> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        let request_str_content = if !parsed.original_string.is_empty() {
            parsed.original_string.clone()
        } else {
            format!(
                "{} {} reps rpe:{:?}",
                parsed.exercise,
                parsed.reps.unwrap_or(0),
                parsed.rpe
            )
        };

        let exercise_name = parsed.exercise.clone();
        let exercise = get_or_create_exercise(&self.db_pool, &exercise_name).await?;
        let is_new_exercise = self.is_exercise_new_for_session(exercise.id).await?;
        let uniffi_exercise = Arc::new(UniffiExercise::from(exercise.clone()));

        let weight = parsed.weight.unwrap_or(0.0) as f64;
        let reps = parsed.reps.unwrap_or(0) as i64;
        let set_count = parsed.set_count.unwrap_or(1).max(1) as i64;
        let parsed_rpe = parsed.rpe.map(|r| r as f64);

        let request =
            create_request_string_for_username(&self.db_pool, "cli", request_str_content.clone())
                .await?;

        let mut modifications = Vec::new();

        if set_count > 1 {
            let created_sets = add_multiple_sets_to_workout(
                &self.db_pool,
                &session_id,
                &exercise.id,
                &request.id,
                &weight,
                &reps,
                parsed_rpe,
                set_count,
            )
            .await?;

            let set_ids: Vec<i64> = created_sets.iter().map(|s| s.id).collect();
            let uniffi_sets: Vec<Arc<UniffiWorkoutSet>> = created_sets
                .into_iter()
                .map(|s| Arc::new(UniffiWorkoutSet::from(s)))
                .collect();

            let modification_type = if is_new_exercise {
                ModificationType::ExerciseAdded
            } else {
                ModificationType::SetAdded
            };

            modifications.push(Modification {
                modification_type,
                set_id: Some(set_ids[0]),
                set_ids: set_ids.clone(),
                exercise_id: Some(exercise.id),
                set: Some(uniffi_sets[0].clone()),
                sets: Some(uniffi_sets),
                exercise: Some(uniffi_exercise.clone()),
            });
        } else {
            let created_set = add_workout_set(
                &self.db_pool,
                &session_id,
                &exercise.id,
                &request.id,
                &weight,
                &reps,
                parsed_rpe,
            )
            .await?;

            let uniffi_set = Arc::new(UniffiWorkoutSet::from(created_set.clone()));

            let modification_type = if is_new_exercise {
                ModificationType::ExerciseAdded
            } else {
                ModificationType::SetAdded
            };

            modifications.push(Modification {
                modification_type,
                set_id: Some(created_set.id),
                set_ids: vec![created_set.id],
                exercise_id: Some(exercise.id),
                set: Some(uniffi_set.clone()),
                sets: Some(vec![uniffi_set]),
                exercise: Some(uniffi_exercise),
            });
        }

        Ok(modifications)
    }

    pub async fn update_workout_set_with_modifications(
        &self,
        set_id: i64,
        update: &UpdateWorkoutSet,
    ) -> Result<(WorkoutSet, Vec<Modification>)> {
        let updated = update_workout_set(&self.db_pool, set_id, update).await?;
        let uniffi_set = Arc::new(UniffiWorkoutSet::from(updated.clone()));

        let exercise_id = updated.exercise_id;
        let exercise_opt = sqlx::query_as::<_, Exercise>("SELECT * FROM exercises WHERE id = ?")
            .bind(exercise_id)
            .fetch_optional(&self.db_pool)
            .await?;

        let uniffi_exercise = exercise_opt.map(|e| Arc::new(UniffiExercise::from(e)));

        let modifications = vec![Modification {
            modification_type: ModificationType::SetModified,
            set_id: Some(set_id),
            set_ids: vec![set_id],
            exercise_id: Some(updated.exercise_id),
            set: Some(uniffi_set.clone()),
            sets: Some(vec![uniffi_set]),
            exercise: uniffi_exercise,
        }];

        Ok((updated, modifications))
    }

    pub async fn delete_set_with_modifications(&self, set_id: i64) -> Result<Vec<Modification>> {
        let sets =
            get_sets_for_session(&self.db_pool, self.get_workout_id().await.unwrap()).await?;
        let exercise_id = sets.iter().find(|s| s.id == set_id).map(|s| s.exercise_id);

        delete_workout_set(&self.db_pool, set_id).await?;

        Ok(vec![Modification {
            modification_type: ModificationType::SetRemoved,
            set_id: Some(set_id),
            set_ids: vec![set_id],
            exercise_id,
            set: None,
            sets: None,
            exercise: None,
        }])
    }

    pub async fn get_all_exercises(&self) -> Result<Vec<Exercise>> {
        crate::db::operations::get_all_exercises(&self.db_pool).await
    }
}
