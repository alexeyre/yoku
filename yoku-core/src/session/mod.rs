use anyhow::Result;
use tokio::sync::Mutex;

use crate::db::operations::{
    add_set_to_workout, create_workout, get_or_create_exercise, get_workout, update_set_from_parsed,
};
use crate::parser::ParsedSet;

pub struct Session {
    pub workout_id: Mutex<Option<i32>>,
}

impl Session {
    pub async fn new_blank() -> Self {
        Self {
            workout_id: Mutex::new(None),
        }
    }

    pub async fn set_workout_id(&mut self, workout_id: i32) -> Result<()> {
        get_workout(&workout_id).await?;
        *self.workout_id.lock().await = Some(workout_id);
        Ok(())
    }

    pub async fn new_workout(&mut self) -> Result<()> {
        let workout = create_workout(None, None).await?;
        self.set_workout_id(workout.id).await
    }

    pub async fn new_workout_with_name(&mut self, name: &str) -> Result<()> {
        let workout = create_workout(Some(name.into()), None).await?;
        self.set_workout_id(workout.id).await
    }

    pub async fn get_workout_id(&self) -> Option<i32> {
        *self.workout_id.lock().await
    }

    pub async fn replace_set_from_parsed(&self, set_id: i32, parsed: &ParsedSet) -> Result<()> {
        update_set_from_parsed(set_id, parsed).await?;
        Ok(())
    }

    pub async fn add_set_from_parsed(&self, parsed: &ParsedSet) -> Result<()> {
        let workout_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        // Get or create the exercise
        let exercise = get_or_create_exercise(&parsed.exercise).await?;

        let weight = parsed.weight.unwrap_or(0.0);
        let reps = parsed.reps.unwrap_or(0);

        // Add the set to the workout
        add_set_to_workout(&workout_id, &exercise.id, &weight, &reps, parsed.rpe).await?;

        Ok(())
    }
}
