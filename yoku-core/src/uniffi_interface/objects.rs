use chrono::{NaiveDate, NaiveDateTime};
use log::{debug, warn};

use crate::db;

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
    pub date: chrono::NaiveDateTime,
}

#[uniffi::export]
impl WorkoutSession {
    /// Return the session id.
    fn id(&self) -> i64 {
        self.id
    }

    /// Return the session name.
    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn date(&self) -> String {
        // Return a date-only representation (ISO-like) for the session date.
        self.date.format("%Y-%m-%d").to_string()
    }
}

impl From<db::models::WorkoutSession> for WorkoutSession {
    fn from(s: db::models::WorkoutSession) -> Self {
        debug!("Attempting to convert datestring {}", &s.date);

        // Try several common formats: full datetime, date-only, and ISO-like.
        // If none parse, fall back to an epoch start to avoid panics.
        let date = if let Ok(dt) = NaiveDateTime::parse_from_str(&s.date, "%Y-%m-%d %H:%M:%S") {
            dt
        } else if let Ok(d) = NaiveDate::parse_from_str(&s.date, "%Y-%m-%d") {
            d.and_hms_opt(0, 0, 0).expect("Failed to create time")
        } else if let Ok(dt) = NaiveDateTime::parse_from_str(&s.date, "%Y-%m-%dT%H:%M:%S") {
            dt
        } else {
            warn!("Failed to parse date '{}' falling back to epoch", &s.date);
            NaiveDate::from_ymd_opt(1970, 1, 1)
                .expect("Failed to create date")
                .and_hms_opt(0, 0, 0)
                .expect("Failed to create time")
        };

        WorkoutSession {
            id: s.id,
            name: s.name,
            date,
        }
    }
}

#[derive(uniffi::Object)]
pub struct WorkoutSet {
    pub id: i64,
    pub exercise_id: i64,
    pub weight: f64,
    pub reps: i64,
}

#[uniffi::export]
impl WorkoutSet {
    /// Return the set id.
    fn id(&self) -> i64 {
        self.id
    }

    /// Return the associated exercise id.
    fn exercise_id(&self) -> i64 {
        self.exercise_id
    }

    /// Return the weight of the set.
    fn weight(&self) -> f64 {
        self.weight
    }

    /// Return the number of reps in the set.
    fn reps(&self) -> i64 {
        self.reps
    }
}
