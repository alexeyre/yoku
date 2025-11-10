use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    db::get_conn,
    db::models::{Exercise, NewExercise, NewSet, NewWorkout, Set, UpdateSet, Workout},
    db::schema::{exercises, sets, workouts},
};

// Workouts
pub async fn create_workout(name: Option<String>, notes: Option<String>) -> Result<Workout> {
    let mut conn = get_conn().await;
    diesel::insert_into(workouts::table)
        .values(&NewWorkout {
            name,
            notes,
            performed_at: None,
        })
        .get_result::<Workout>(&mut conn)
        .await
        .map_err(Into::into)
}

pub async fn get_workout(workout_id: &i32) -> Result<Workout> {
    let mut conn = get_conn().await;
    workouts::table
        .find(workout_id)
        .first::<Workout>(&mut conn)
        .await
        .map_err(Into::into)
}

pub async fn get_all_workouts() -> Result<Vec<Workout>> {
    let mut conn = get_conn().await;
    workouts::table
        .load::<Workout>(&mut conn)
        .await
        .map_err(Into::into)
}

pub async fn delete_workout(workout_id: i32) -> Result<usize> {
    let mut conn = get_conn().await;
    diesel::delete(workouts::table.find(workout_id))
        .execute(&mut conn)
        .await
        .map_err(Into::into)
}

// Exercises
pub async fn get_exercise(exercise_id: i32) -> Result<Exercise> {
    let mut conn = get_conn().await;
    exercises::table
        .find(exercise_id)
        .first::<Exercise>(&mut conn)
        .await
        .map_err(Into::into)
}

pub async fn get_or_create_exercise(exercise_name: &str) -> Result<Exercise> {
    let mut conn = get_conn().await;

    if let Ok(exercise) = exercises::table
        .filter(exercises::name.eq(exercise_name))
        .first::<Exercise>(&mut conn)
        .await
    {
        return Ok(exercise);
    }

    diesel::insert_into(exercises::table)
        .values(&NewExercise {
            name: exercise_name.to_string(),
            equipment: None,
            primary_muscle: None,
            secondary_muscle: None,
            description: None,
        })
        .get_result::<Exercise>(&mut conn)
        .await
        .map_err(Into::into)
}

// Sets
pub async fn add_set_to_workout(
    workout_id: &i32,
    exercise_id: &i32,
    weight: &f32,
    reps: &i32,
    rpe: Option<f32>,
) -> Result<Set> {
    let mut conn = get_conn().await;
    diesel::insert_into(sets::table)
        .values(&NewSet {
            workout_id: *workout_id,
            exercise_id: *exercise_id,
            weight: *weight,
            reps: *reps,
            rpe,
            set_number: None,
        })
        .get_result::<Set>(&mut conn)
        .await
        .map_err(Into::into)
}

pub async fn get_sets_for_workout(workout_id: i32) -> Result<Vec<Set>> {
    let mut conn = get_conn().await;
    sets::table
        .filter(sets::workout_id.eq(workout_id))
        .load::<Set>(&mut conn)
        .await
        .map_err(Into::into)
}

pub async fn update_set(set_id: i32, update: &UpdateSet) -> Result<Set> {
    let mut conn = get_conn().await;
    diesel::update(sets::table.find(set_id))
        .set(update)
        .get_result::<Set>(&mut conn)
        .await
        .map_err(Into::into)
}

pub async fn delete_set(set_id: i32) -> Result<usize> {
    let mut conn = get_conn().await;
    diesel::delete(sets::table.find(set_id))
        .execute(&mut conn)
        .await
        .map_err(Into::into)
}
