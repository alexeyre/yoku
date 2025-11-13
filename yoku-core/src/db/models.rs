// Dates are stored as ISO 8601 "YYYY-MM-DD" strings in SQLite.
// Model types use `String` for the `date` column to match the schema (date -> Text).
use diesel::{AsChangeset, Associations, Insertable, Queryable, Selectable};
use std::fmt;
// UUIDs are stored as 16-byte BLOBs (i32) when using SQLite.
// Application code should generate 16-byte UUID values and pass them as `i32`
// when inserting records.

use crate::db::schema::{
    exercise_muscles, exercises, muscles, request_strings, users, workout_sessions, workout_sets,
};

#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = muscles)]
pub struct Muscle {
    pub id: i32,
    pub name: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = muscles)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewMuscle {
    pub name: String,
}

#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = exercises)]
pub struct Exercise {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = exercises)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewExercise {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(belongs_to(Exercise))]
#[diesel(belongs_to(Muscle))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = exercise_muscles)]
pub struct ExerciseMuscle {
    pub exercise_id: i32,
    pub muscle_id: i32,
    pub relation_type: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = exercise_muscles)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewExerciseMuscle {
    pub exercise_id: i32,
    pub muscle_id: i32,
    pub relation_type: String,
}

#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewUser {
    pub username: String,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = request_strings)]
pub struct RequestString {
    pub id: i32,
    pub user_id: i32,
    pub string: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = request_strings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewRequestString {
    pub user_id: i32,
    pub string: String,
}

#[derive(Queryable, Selectable, Debug, Clone, Associations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(User))]
#[diesel(table_name = workout_sessions)]
pub struct WorkoutSession {
    pub id: i32,
    pub user_id: Option<i32>,
    pub name: Option<String>,
    pub date: String,
    pub duration_seconds: i32,
    pub notes: Option<String>,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = workout_sessions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
/// When inserting, ensure `date` is formatted as an ISO 8601 date string ("YYYY-MM-DD").
pub struct NewWorkoutSession {
    pub user_id: Option<i32>,
    pub name: Option<String>,
    pub date: String,
    pub duration_seconds: i32,
    pub notes: Option<String>,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(belongs_to(WorkoutSession, foreign_key = session_id))]
#[diesel(belongs_to(Exercise))]
#[diesel(belongs_to(RequestString, foreign_key = request_string_id))]
#[diesel(table_name = workout_sets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct WorkoutSet {
    pub id: i32,
    pub session_id: i32,
    pub exercise_id: i32,
    pub request_string_id: i32,
    pub weight: f32,
    pub reps: i32,
    pub set_index: i32,
    pub rpe: Option<f32>,
    pub notes: Option<String>,
    pub created_at: i32,
    pub updated_at: i32,
}

impl fmt::Display for WorkoutSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rpe_str = self.rpe.map(|r| format!(" @{:.1}", r)).unwrap_or_default();
        // Represent the 16-byte UUID as lowercase hex for display
        write!(
            f,
            "Exercise {}: {:.1} x {} reps{}",
            self.id, self.weight, self.reps, rpe_str
        )
    }
}

// Helper struct for displaying sets with exercise names
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

#[derive(Insertable, Clone)]
#[diesel(table_name = workout_sets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewWorkoutSet {
    pub session_id: i32,
    pub exercise_id: i32,
    pub request_string_id: i32,
    pub weight: f32,
    pub reps: i32,
    pub set_index: i32,
    pub rpe: Option<f32>,
    pub notes: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = workout_sets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateWorkoutSet {
    pub session_id: Option<i32>,
    pub exercise_id: Option<i32>,
    pub request_string_id: Option<i32>,
    pub weight: Option<f32>,
    pub reps: Option<i32>,
    pub rpe: Option<f32>,
    pub set_index: Option<i32>,
    pub notes: Option<String>,
}
