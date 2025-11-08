use diesel::{Queryable, Insertable, Associations};
//use diesel::prelude::*;
use rust_decimal::Decimal;
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
    pub weight: Decimal,
    pub reps: i32,
    pub rpe: Option<Decimal>,
    pub set_number: Option<i32>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::sets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewSet {
    pub exercise_id: i32,
    pub workout_id: i32,
    pub weight: Decimal,
    pub reps: i32,
    pub rpe: Option<Decimal>,
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
