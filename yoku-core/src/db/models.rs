use std::fmt;
use sqlx::FromRow;

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

#[derive(Debug, Clone, FromRow)]
pub struct WorkoutSession {
    pub id: i64,
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub date: String,
    pub duration_seconds: i64,
    pub notes: Option<String>,
    pub intention: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewWorkoutSession {
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub date: String,
    pub duration_seconds: i64,
    pub notes: Option<String>,
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
