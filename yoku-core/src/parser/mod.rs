// parser module
pub mod llm;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedSet {
    pub exercise: String,
    pub weight: Option<f32>,
    pub reps: Option<f32>,
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
}
