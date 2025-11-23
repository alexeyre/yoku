use sqlx::{Decode, Encode, FromRow, Sqlite, Type};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, FromRow)]
pub struct Muscle {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewMuscle {
    pub name: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct Exercise {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewExercise {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ExerciseMuscle {
    pub exercise_id: i64,
    pub muscle_id: i64,
    pub relation_type: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewExerciseMuscle {
    pub exercise_id: i64,
    pub muscle_id: i64,
    pub relation_type: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewUser {
    pub username: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct RequestString {
    pub id: i64,
    pub user_id: i64,
    pub string: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewRequestString {
    pub user_id: i64,
    pub string: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkoutStatus {
    InProgress,
    Completed,
}

impl WorkoutStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkoutStatus::InProgress => "in_progress",
            WorkoutStatus::Completed => "completed",
        }
    }
}

impl FromStr for WorkoutStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in_progress" => Ok(WorkoutStatus::InProgress),
            "completed" => Ok(WorkoutStatus::Completed),
            _ => Err(format!("Invalid workout status: {}", s)),
        }
    }
}

impl fmt::Display for WorkoutStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Type<Sqlite> for WorkoutStatus {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for WorkoutStatus {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <&str as Encode<'q, Sqlite>>::encode_by_ref(&self.as_str(), args)
    }
}

impl<'r> Decode<'r, Sqlite> for WorkoutStatus {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as Decode<Sqlite>>::decode(value)?;
        WorkoutStatus::from_str(s).map_err(|e| e.into())
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct WorkoutSession {
    pub id: i64,
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub duration_seconds: i64,
    pub notes: Option<String>,
    pub status: WorkoutStatus,
    pub summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewWorkoutSession {
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub duration_seconds: i64,
    pub notes: Option<String>,
    pub status: Option<WorkoutStatus>,
}

#[derive(Debug, Clone, FromRow)]
pub struct WorkoutSet {
    pub id: i64,
    pub session_id: i64,
    pub exercise_id: i64,
    pub request_string_id: i64,
    pub weight: f64,
    pub reps: i64,
    pub set_index: i64,
    pub rpe: Option<f64>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl fmt::Display for WorkoutSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rpe_str = self.rpe.map(|r| format!(" @{:.1}", r)).unwrap_or_default();
        write!(
            f,
            "Exercise {}: {:.1} x {} reps{}",
            self.id, self.weight, self.reps, rpe_str
        )
    }
}
pub struct DisplayableSet {
    pub set: WorkoutSet,
    pub exercise_name: String,
}

impl DisplayableSet {
    pub fn new(set: WorkoutSet, exercise_name: String) -> Self {
        Self { set, exercise_name }
    }
}

impl fmt::Display for DisplayableSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rpe_str = self
            .set
            .rpe
            .map(|r| format!(" @{:.1}", r))
            .unwrap_or_default();

        write!(
            f,
            "{} (set #{}): {:.1} x {} reps{}",
            self.exercise_name, self.set.set_index, self.set.weight, self.set.reps, rpe_str
        )
    }
}

#[derive(Clone)]
pub struct NewWorkoutSet {
    pub session_id: i64,
    pub exercise_id: i64,
    pub request_string_id: i64,
    pub weight: f64,
    pub reps: i64,
    pub set_index: i64,
    pub rpe: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug)]
pub struct UpdateWorkoutSet {
    pub session_id: Option<i64>,
    pub exercise_id: Option<i64>,
    pub request_string_id: Option<i64>,
    pub weight: Option<f64>,
    pub reps: Option<i64>,
    pub rpe: Option<f64>,
    pub set_index: Option<i64>,
    pub notes: Option<String>,
}
