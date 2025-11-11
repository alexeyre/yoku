use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    db::get_conn,
    db::models::{Exercise, NewExercise, NewSet, NewWorkout, Set, UpdateSet, Workout},
    db::schema::{exercises, sets, workouts},
    parser::ParsedSet,
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

pub async fn get_all_exercises() -> Result<Vec<Exercise>> {
    let mut conn = get_conn().await;
    exercises::table
        .load::<Exercise>(&mut conn)
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

    // Get the next set number for this exercise in this workout
    let max_set_number: Option<i32> = sets::table
        .filter(sets::workout_id.eq(workout_id))
        .filter(sets::exercise_id.eq(exercise_id))
        .select(diesel::dsl::max(sets::set_number))
        .first(&mut conn)
        .await
        .ok()
        .flatten();

    let next_set_number = max_set_number.map(|n| n + 1).unwrap_or(1);

    diesel::insert_into(sets::table)
        .values(&NewSet {
            workout_id: *workout_id,
            exercise_id: *exercise_id,
            weight: *weight,
            reps: *reps,
            rpe,
            set_number: Some(next_set_number),
        })
        .get_result::<Set>(&mut conn)
        .await
        .map_err(Into::into)
}

/// Add multiple sets at once for an exercise in a workout
/// This is useful when a user logs "5 sets of 5 reps at 100kg"
pub async fn add_multiple_sets_to_workout(
    workout_id: &i32,
    exercise_id: &i32,
    weight: &f32,
    reps: &i32,
    rpe: Option<f32>,
    set_count: i32,
) -> Result<Vec<Set>> {
    let mut conn = get_conn().await;

    // Get the current max set number for this exercise in this workout
    let max_set_number: Option<i32> = sets::table
        .filter(sets::workout_id.eq(workout_id))
        .filter(sets::exercise_id.eq(exercise_id))
        .select(diesel::dsl::max(sets::set_number))
        .first(&mut conn)
        .await
        .ok()
        .flatten();

    let starting_set_number = max_set_number.map(|n| n + 1).unwrap_or(1);

    // Create all the sets
    let new_sets: Vec<NewSet> = (0..set_count)
        .map(|i| NewSet {
            workout_id: *workout_id,
            exercise_id: *exercise_id,
            weight: *weight,
            reps: *reps,
            rpe,
            set_number: Some(starting_set_number + i),
        })
        .collect();

    diesel::insert_into(sets::table)
        .values(&new_sets)
        .get_results::<Set>(&mut conn)
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

/// Updates a set from a parsed set description, intelligently merging with the original set.
/// Only fields provided in the parsed set will be updated; missing fields retain their original values.
pub async fn update_set_from_parsed(set_id: i32, parsed: &ParsedSet) -> Result<Set> {
    let mut conn = get_conn().await;

    // Get the original set first
    let original_set = sets::table.find(set_id).first::<Set>(&mut conn).await?;

    // Resolve exercise: only change if user specified a different one
    let exercise_id = if !parsed.exercise.is_empty() {
        let exercise = get_or_create_exercise(&parsed.exercise).await?;
        // Only update if different from original
        if exercise.id != original_set.exercise_id {
            Some(exercise.id)
        } else {
            None // No change needed
        }
    } else {
        None // User didn't specify, keep original
    };

    let update = UpdateSet {
        workout_id: None, // Never change workout_id on updates
        exercise_id,
        reps: parsed.reps,
        weight: parsed.weight,
        rpe: Some(parsed.rpe),
        set_number: None, // Keep original set_number
    };

    diesel::update(sets::table.find(set_id))
        .set(&update)
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
