use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime};
use log::{debug, warn};

use crate::{
    db,
    uniffi_interface::errors::{self, YokuError},
};

#[derive(uniffi::Object)]
pub struct Exercise {
    id: i64,
    name: String,
}

#[uniffi::export]
impl Exercise {
    fn id(&self) -> i64 {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl From<db::models::Exercise> for Exercise {
    fn from(e: db::models::Exercise) -> Self {
        Exercise {
            id: e.id,
            name: e.name,
        }
    }
}

#[derive(uniffi::Object)]
pub struct WorkoutSession {
    pub id: i64,
    pub name: Option<String>,
    pub date: chrono::NaiveDate,
    pub status: String,
    pub duration_seconds: i64,
    pub summary: Option<String>,
}

#[uniffi::export]
impl WorkoutSession {
        fn id(&self) -> i64 {
        self.id
    }

        fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn date(&self) -> String {
        self.date.format("%Y-%m-%d").to_string()
    }

        fn status(&self) -> String {
        self.status.clone()
    }

                fn duration_seconds(&self) -> i64 {
        self.duration_seconds
    }

        fn summary(&self) -> Option<String> {
        self.summary.clone()
    }
}

impl TryFrom<db::models::WorkoutSession> for WorkoutSession {
    type Error = errors::YokuError;
    fn try_from(s: db::models::WorkoutSession) -> Result<Self, errors::YokuError> {
        debug!("Attempting to convert date {}", s.date);
        let date = chrono::NaiveDate::parse_from_str(&s.date, "%Y-%m-%d")
            .map_err(|e| errors::YokuError::Common(e.to_string()))?;
        debug!(
            "Successfully parsed date {} to {}",
            s.date,
            date.to_string()
        );
        Ok(WorkoutSession {
            id: s.id,
            name: s.name,
            date,
            status: s.status,
            duration_seconds: s.duration_seconds,
            summary: s.summary,
        })
    }
}

#[derive(uniffi::Object)]
pub struct WorkoutSet {
    pub id: i64,
    pub exercise_id: i64,
    pub weight: f64,
    pub reps: i64,
    pub rpe: Option<f64>,
    pub notes: Option<String>,
}

#[uniffi::export]
impl WorkoutSet {
        fn id(&self) -> i64 {
        self.id
    }

        fn exercise_id(&self) -> i64 {
        self.exercise_id
    }

        fn weight(&self) -> f64 {
        self.weight
    }

        fn reps(&self) -> i64 {
        self.reps
    }

        fn rpe(&self) -> Option<f64> {
        debug!("RPE: {:?}", self.rpe);
        self.rpe
    }

        fn notes(&self) -> Option<String> {
        self.notes.clone()
    }
}

impl From<db::models::WorkoutSet> for WorkoutSet {
    fn from(s: db::models::WorkoutSet) -> Self {
        WorkoutSet {
            id: s.id,
            exercise_id: s.exercise_id,
            weight: s.weight,
            reps: s.reps,
            rpe: s.rpe,
            notes: s.notes,
        }
    }
}

#[derive(uniffi::Object)]
pub struct WorkoutSuggestion {
    pub title: String,
    pub subtitle: Option<String>,
    pub suggestion_type: String,
    pub exercise_name: Option<String>,
    pub reasoning: Option<String>,
}

#[uniffi::export]
impl WorkoutSuggestion {
    fn title(&self) -> String {
        self.title.clone()
    }

    fn subtitle(&self) -> Option<String> {
        self.subtitle.clone()
    }

    fn suggestion_type(&self) -> String {
        self.suggestion_type.clone()
    }

    fn exercise_name(&self) -> Option<String> {
        self.exercise_name.clone()
    }

    fn reasoning(&self) -> Option<String> {
        self.reasoning.clone()
    }
}

impl From<crate::llm::WorkoutSuggestion> for WorkoutSuggestion {
    fn from(s: crate::llm::WorkoutSuggestion) -> Self {
        WorkoutSuggestion {
            title: s.title,
            subtitle: s.subtitle,
            suggestion_type: s.suggestion_type,
            exercise_name: s.exercise_name,
            reasoning: s.reasoning,
        }
    }
}

#[derive(uniffi::Object)]
pub struct WorkoutSummary {
    pub message: String,
    pub emoji: String,
}

#[uniffi::export]
impl WorkoutSummary {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn emoji(&self) -> String {
        self.emoji.clone()
    }
}

impl From<crate::llm::WorkoutSummary> for WorkoutSummary {
    fn from(summary: crate::llm::WorkoutSummary) -> Self {
        WorkoutSummary {
            message: summary.message,
            emoji: summary.emoji,
        }
    }
}
