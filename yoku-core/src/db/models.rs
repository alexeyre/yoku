use diesel::{Associations, Insertable, Queryable, AsChangeset};
use crate::db::schema;

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
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewExercise {
    pub name: String,
    pub equipment: Option<String>,
    pub primary_muscle: Option<String>,
    pub secondary_muscle: Option<String>,
    pub description: Option<String>,
}

#[derive(Queryable, Debug, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Tag {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = schema::tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTag {
    pub name: String,
}

#[derive(Queryable, Debug, Clone)]
#[diesel(table_name = crate::db::schema::workouts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Workout {
    pub id: i32,
    pub name: Option<String>,
    pub performed_at: Option<chrono::NaiveDateTime>,
    pub notes: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::workouts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewWorkout {
    pub name: Option<String>,
    pub performed_at: Option<chrono::NaiveDateTime>,
    pub notes: Option<String>,
}

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

#[derive(Insertable)]
#[diesel(table_name = schema::sets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewSet {
    pub exercise_id: i32,
    pub workout_id: i32,
    pub weight: f32,
    pub reps: i32,
    pub rpe: Option<f32>,
    pub set_number: Option<i32>,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(table_name = schema::settags)]
#[diesel(belongs_to(Set, foreign_key = set_id))]
#[diesel(belongs_to(Tag, foreign_key = tag_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SetTag {
    pub id: i32,
    pub set_id: i32,
    pub tag_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = schema::settags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewSetTag {
    pub set_id: i32,
    pub tag_id: i32,
}

#[derive(Queryable, Debug, Clone, Associations)]
#[diesel(table_name = schema::exercisetags)]
#[diesel(belongs_to(Exercise, foreign_key = exercise_id))]
#[diesel(belongs_to(Tag, foreign_key = tag_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExerciseTag {
    pub id: i32,
    pub exercise_id: i32,
    pub tag_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = schema::exercisetags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewExerciseTag {
    pub exercise_id: i32,
    pub tag_id: i32,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = schema::exercises)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdateExercise {
    pub name: Option<String>,
    pub equipment: Option<Option<String>>,
    pub primary_muscle: Option<Option<String>>,
    pub secondary_muscle: Option<Option<String>>,
    pub description: Option<Option<String>>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = schema::tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdateTag {
    pub name: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = schema::workouts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdateWorkout {
    pub name: Option<Option<String>>,
    pub performed_at: Option<Option<chrono::NaiveDateTime>>,
    pub notes: Option<Option<String>>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = schema::sets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdateSet {
    pub exercise_id: Option<i32>,
    pub workout_id: Option<i32>,
    pub weight: Option<f32>,
    pub reps: Option<i32>,
    pub rpe: Option<Option<f32>>,
    pub set_number: Option<Option<i32>>,
}
