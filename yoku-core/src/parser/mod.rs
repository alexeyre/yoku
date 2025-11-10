// parser module

mod check;
pub mod llm;

use crate::db::models::NewSet;
use anyhow::{anyhow, Result};


use serde;
use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedSet {
    pub exercise: String,
    pub weight: Option<f32>,
    pub reps: Option<f32>,
    pub rpe: Option<f32>,
    pub tags: Vec<String>,
    pub aoi: Option<String>,
    #[serde(skip_deserializing)]
    pub original_string: String
}


impl ParsedSet {
    pub fn empty() -> ParsedSet {
        Self {
            exercise: "".into(),
            weight: None,
            reps: None,
            rpe: None,
            tags: Vec::new(),
            aoi: None,
            original_string: "".into()
        }
    }

    pub fn with_original(p: ParsedSet, original: String) -> ParsedSet {
        Self {
            exercise: p.exercise,
            weight: p.weight,
            reps: p.reps,
            rpe: p.rpe,
            tags: p.tags,
            aoi: p.aoi,
            original_string: original
        }
    }
}
