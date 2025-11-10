use diesel::{AsChangeset, Associations, Insertable, Queryable};
use std::fmt;

use crate::db::schema;

// Exercise models
#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Exercise {
    pub id: i32,
    pub name: String,
    pub equipment: Option<String>,
    pub primary_muscle: Option<String>,
    pub secondary_muscle: Option<String>,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::exercises)]
pub struct NewExercise {
    pub name: String,
    pub equipment: Option<String>,
    pub primary_muscle: Option<String>,
    pub secondary_muscle: Option<String>,
    pub description: Option<String>,
}

// Workout models
#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Workout {
    pub id: i32,
    pub name: Option<String>,
    pub performed_at: Option<chrono::NaiveDateTime>,
    pub notes: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::workouts)]
pub struct NewWorkout {
    pub name: Option<String>,
    pub performed_at: Option<chrono::NaiveDateTime>,
    pub notes: Option<String>,
}

// Set models
#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(belongs_to(Exercise))]
#[diesel(belongs_to(Workout))]
#[diesel(table_name = schema::sets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Set {
    pub id: i32,
    pub exercise_id: i32,
    pub workout_id: i32,
    pub weight: f32,
    pub reps: i32,
    pub rpe: Option<f32>,
    pub set_number: Option<i32>,
}

impl fmt::Display for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rpe_str = self.rpe.map(|r| format!(" @{:.1}", r)).unwrap_or_default();

        write!(
            f,
            "Exercise #{}: {:.1}lbs x {} reps{}",
            self.exercise_id, self.weight, self.reps, rpe_str
        )
    }
}

// Helper struct for displaying sets with exercise names
pub struct DisplayableSet {
    pub set: Set,
    pub exercise_name: String,
}

impl DisplayableSet {
    pub fn new(set: Set, exercise_name: String) -> Self {
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
            "{}: {:.1}kg x {} reps{}",
            self.exercise_name, self.set.weight, self.set.reps, rpe_str
        )
    }
}

#[derive(Insertable)]
#[diesel(table_name = schema::sets)]
pub struct NewSet {
    pub exercise_id: i32,
    pub workout_id: i32,
    pub weight: f32,
    pub reps: i32,
    pub rpe: Option<f32>,
    pub set_number: Option<i32>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = schema::sets)]
pub struct UpdateSet {
    pub exercise_id: Option<i32>,
    pub workout_id: Option<i32>,
    pub weight: Option<f32>,
    pub reps: Option<i32>,
    pub rpe: Option<Option<f32>>,
    pub set_number: Option<Option<i32>>,
}
