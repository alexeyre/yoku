use anyhow::Result;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db::operations::{
    add_multiple_sets_to_workout, add_workout_set, create_request_string_for_username,
    create_workout_session, get_or_create_exercise, get_workout_session,
    update_workout_set_from_parsed,
};
use crate::parser::ParsedSet;

pub struct Session {
    pub workout_id: Mutex<Option<Uuid>>,
}

impl Session {
    pub async fn new_blank() -> Self {
        Self {
            workout_id: Mutex::new(None),
        }
    }

    pub async fn set_workout_id(&mut self, workout_id: Uuid) -> Result<()> {
        get_workout_session(&workout_id).await?;
        *self.workout_id.lock().await = Some(workout_id);
        Ok(())
    }

    pub async fn new_workout(&mut self) -> Result<()> {
        let workout = create_workout_session(None, None, None, None).await?;
        self.set_workout_id(workout.id).await
    }

    pub async fn new_workout_with_name(&mut self, name: &str) -> Result<()> {
        let workout = create_workout_session(None, Some(name.into()), None, None).await?;
        self.set_workout_id(workout.id).await
    }

    pub async fn get_workout_id(&self) -> Option<Uuid> {
        *self.workout_id.lock().await
    }

    pub async fn replace_set_from_parsed(&self, set_id: Uuid, parsed: &ParsedSet) -> Result<()> {
        update_workout_set_from_parsed(set_id, parsed).await?;
        Ok(())
    }

    pub async fn add_set_from_parsed(&self, parsed: &ParsedSet) -> Result<()> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        // Get or create the exercise
        let exercise = get_or_create_exercise(&parsed.exercise).await?;

        let weight = parsed.weight.unwrap_or(0.0);
        let reps = parsed.reps.unwrap_or(0);
        let set_count = parsed.set_count.unwrap_or(1).max(1); // Default to 1 set, minimum 1

        // The DB requires a `request_string_id` for each workout set.
        // Create a `request_strings` row for this parsed set so we maintain
        // a real reference and can inspect the original user input later.
        // We use a CLI/system username here; you can adapt this to use a
        // real authenticated user when available.
        let request_str_content = if !parsed.original_string.is_empty() {
            parsed.original_string.clone()
        } else {
            // Fallback brief summary if the parser didn't set original_string.
            format!(
                "{} {} reps rpe:{:?}",
                parsed.exercise,
                parsed.reps.unwrap_or(0),
                parsed.rpe
            )
        };
        let req = create_request_string_for_username("cli", request_str_content).await?;
        let request_string_id = req.id;

        // Add multiple sets if set_count > 1
        if set_count > 1 {
            add_multiple_sets_to_workout(
                &session_id,
                &exercise.id,
                &request_string_id,
                &weight,
                &reps,
                parsed.rpe,
                set_count,
            )
            .await?;
        } else {
            // Add a single set
            add_workout_set(
                &session_id,
                &exercise.id,
                &request_string_id,
                &weight,
                &reps,
                parsed.rpe,
            )
            .await?;
        }

        Ok(())
    }
}
