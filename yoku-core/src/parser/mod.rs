// parser module
pub mod llm;

use crate::db::models::UpdateSet;
use crate::db::operations::get_or_create_exercise;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedSet {
    pub exercise: String,
    pub weight: Option<f32>,
    pub reps: Option<i32>,
    pub rpe: Option<f32>,
    pub tags: Vec<String>,
    pub aoi: Option<String>,
    #[serde(skip_deserializing)]
    pub original_string: String,
}

impl ParsedSet {
    pub fn with_original(mut p: ParsedSet, original: String) -> ParsedSet {
        p.original_string = original;
        p
    }

    pub async fn to_update_set(&self) -> UpdateSet {
        let exercise_id = get_or_create_exercise(&self.exercise).await.unwrap().id;
        UpdateSet {
            workout_id: None,
            exercise_id: Some(exercise_id),
            reps: self.reps,
            weight: self.weight,
            rpe: Some(self.rpe),
            set_number: None // TODO
        }
    }
}
