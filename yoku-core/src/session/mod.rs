use anyhow::Result;
use tokio::sync::Mutex;

use crate::db::operations::{
    add_multiple_sets_to_workout, add_workout_set, create_request_string_for_username,
    create_workout_session, get_or_create_exercise, get_workout_session,
    update_workout_set_from_parsed,
};
use crate::llm::ParsedSet;

/// Session holds a mutable optional active workout id for CLI flows.
/// Workout ids are stored as 16-byte UUID bytes (`i32`).
pub struct Session {
    pub workout_id: Mutex<Option<i32>>,
}

impl Session {
    /// Create a new blank session (no active workout).
    pub async fn new_blank() -> Self {
        Self {
            workout_id: Mutex::new(None),
        }
    }

    /// Set the active workout id for this session.
    /// Validates the id exists in the DB by calling `get_workout_session`.
    pub async fn set_workout_id(&self, workout_id: i32) -> Result<()> {
        // Validate the session exists (will return Err if not found)
        let _ = get_workout_session(workout_id.clone()).await?;
        *self.workout_id.lock().await = Some(workout_id);
        Ok(())
    }

    /// Create a new (empty) workout session and set it as the active workout.
    pub async fn new_workout(&self) -> Result<()> {
        let workout = create_workout_session(None, None, None, None).await?;
        // workout.id is i32
        self.set_workout_id(workout.id).await
    }

    /// Create a new workout session with the provided name and set it active.
    pub async fn new_workout_with_name(&self, name: &str) -> Result<()> {
        let workout = create_workout_session(None, Some(name.into()), None, None).await?;
        self.set_workout_id(workout.id).await
    }

    /// Return the currently active workout id (16-byte UUID bytes), if any.
    pub async fn get_workout_id(&self) -> Option<i32> {
        self.workout_id.lock().await.clone()
    }

    /// Replace an existing set with parsed data. `set_id` is the 16-byte UUID bytes of the set.
    pub async fn replace_set_from_parsed(&self, set_id: i32, parsed: &ParsedSet) -> Result<()> {
        update_workout_set_from_parsed(set_id, parsed).await?;
        Ok(())
    }

    /// Add a parsed set into the active workout session.
    /// This will create/get the exercise and the request_string row as needed and then add one or more sets.
    pub async fn add_set_from_parsed(&self, parsed: &ParsedSet) -> Result<()> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        // Ensure exercise exists (or create it)
        let exercise = get_or_create_exercise(&parsed.exercise).await?;

        let weight = parsed.weight.unwrap_or(0.0);
        let reps = parsed.reps.unwrap_or(0);
        let set_count = parsed.set_count.unwrap_or(1).max(1);

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

        let req = create_request_string_for_username("cli", request_str_content).await?;
        let request_string_id = req.id;

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
