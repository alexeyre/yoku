use chrono::{DateTime, NaiveDate, Utc};
use diesel::{AsChangeset, Associations, Insertable, Queryable};
use std::fmt;
use uuid::Uuid;

use crate::db::schema::{
    exercise_muscles, exercises, muscles, request_strings, users, workout_sessions, workout_sets,
};

#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = muscles)]
pub struct Muscle {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = muscles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMuscle {
    pub name: String,
}

#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = exercises)]
pub struct Exercise {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = exercises)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewExercise {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(belongs_to(Exercise))]
#[diesel(belongs_to(Muscle))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = exercise_muscles)]
pub struct ExerciseMuscle {
    pub exercise_id: Uuid,
    pub muscle_id: Uuid,
    pub relation_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = exercise_muscles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewExerciseMuscle {
    pub exercise_id: Uuid,
    pub muscle_id: Uuid,
    pub relation_type: String,
}

#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser {
    pub username: String,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = request_strings)]
pub struct RequestString {
    pub id: Uuid,
    pub user_id: Uuid,
    pub string: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = request_strings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRequestString {
    pub user_id: Uuid,
    pub string: String,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = workout_sessions)]
pub struct WorkoutSession {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: Option<String>,
    pub date: NaiveDate,
    pub duration_seconds: i32,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = workout_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewWorkoutSession {
    pub user_id: Option<Uuid>,
    pub name: Option<String>,
    pub date: NaiveDate,
    pub duration_seconds: i32,
    pub notes: Option<String>,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(belongs_to(WorkoutSession, foreign_key = session_id))]
#[diesel(belongs_to(Exercise))]
#[diesel(belongs_to(RequestString, foreign_key = request_string_id))]
#[diesel(table_name = workout_sets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WorkoutSet {
    pub id: Uuid,
    pub session_id: Uuid,
    pub exercise_id: Uuid,
    pub request_string_id: Uuid,
    pub weight: f32,
    pub reps: i32,
    pub set_index: i32,
    pub rpe: Option<f32>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Display for WorkoutSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rpe_str = self.rpe.map(|r| format!(" @{:.1}", r)).unwrap_or_default();
        write!(
            f,
            "Exercise {}: {:.1} x {} reps{}",
            self.exercise_id, self.weight, self.reps, rpe_str
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
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewWorkoutSet {
    pub session_id: Uuid,
    pub exercise_id: Uuid,
    pub request_string_id: Uuid,
    pub weight: f32,
    pub reps: i32,
    pub set_index: i32,
    pub rpe: Option<f32>,
    pub notes: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = workout_sets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdateWorkoutSet {
    pub session_id: Option<Uuid>,
    pub exercise_id: Option<Uuid>,
    pub request_string_id: Option<Uuid>,
    pub weight: Option<f32>,
    pub reps: Option<i32>,
    pub rpe: Option<f32>,
    pub set_index: Option<i32>,
    pub notes: Option<String>,
}
