use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    db::get_conn,
    db::models::{
        Exercise, NewExercise, UpdateExercise,
        NewSet, UpdateSet, Set,
        NewWorkout, UpdateWorkout, Workout,
    },
    db::schema::{exercises, sets, workouts},
};

pub async fn get_exercises() -> Result<Vec<Exercise>> {
    use crate::db::schema::exercises::dsl::*;
    let mut conn = get_conn().await;
    let result = exercises.load::<Exercise>(&mut conn).await?;
    Ok(result)
}

pub async fn create_workout(name: Option<String>, notes: Option<String>) -> Result<Workout> {
    let new_workout = NewWorkout {
        name: name,
        notes: notes,
        performed_at: None,
    };

    let mut conn = get_conn().await;
    let result = diesel::insert_into(workouts::table)
        .values(&new_workout)
        .get_result::<Workout>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn get_workout(workout_id: &i32) -> Result<Workout> {
    use crate::db::schema::workouts::dsl::*;
    let mut conn = get_conn().await;

    let result = workouts
        .filter(id.eq(workout_id))
        .first::<Workout>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn create_blank_workout() -> Result<Workout> {
    create_workout(None, None).await
}

pub async fn add_set_to_workout(
    p_workout_id: &i32,
    p_exercise_id: &i32,
    p_weight: &f32,
    p_reps: &i32,
) -> Result<Set> {
    let _workout = get_workout(p_workout_id).await?;

    let new_set = NewSet {
        workout_id: *p_workout_id,
        exercise_id: *p_exercise_id,
        weight: *p_weight,
        reps: *p_reps,
        rpe: None,
        set_number: None,
    };

    let mut conn = get_conn().await;
    let result = diesel::insert_into(sets::table)
        .values(&new_set)
        .get_result::<Set>(&mut conn)
        .await?;

    Ok(result)
}

pub async fn add_set_standalone(exercise_id: &i32, weight: &f32, reps: &i32) -> Result<Set> {
    let workout = create_blank_workout().await?;
    let workout_id = workout.id;
    add_set_to_workout(&workout_id, exercise_id, weight, reps).await
}


async fn get_all_workouts() -> Result<Vec<Workout>> {
    use crate::db::schema::workouts::dsl::*;
    let mut conn = get_conn().await;
    let result = workouts.load::<Workout>(&mut conn).await?;
    Ok(result)
}

pub async fn update_workout(workout_id: i32, update: &UpdateWorkout) -> Result<Workout> {
    use crate::db::schema::workouts::dsl::*;
    let mut conn = get_conn().await;
    let result = diesel::update(workouts.filter(id.eq(workout_id)))
        .set(update)
        .get_result::<Workout>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn delete_workout(workout_id: i32) -> Result<usize> {
    use crate::db::schema::workouts::dsl::*;
    let mut conn = get_conn().await;
    let rows_deleted = diesel::delete(workouts.filter(id.eq(workout_id)))
        .execute(&mut conn)
        .await?;
    Ok(rows_deleted)
}

pub async fn create_exercise(
    name: String,
    equipment: Option<String>,
    primary_muscle: Option<String>,
    secondary_muscle: Option<String>,
    description: Option<String>,
) -> Result<Exercise> {
    let new_exercise = NewExercise {
        name,
        equipment,
        primary_muscle,
        secondary_muscle,
        description,
    };
    let mut conn = get_conn().await;
    let result = diesel::insert_into(exercises::table)
        .values(&new_exercise)
        .get_result::<Exercise>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn get_exercise(exercise_id: i32) -> Result<Exercise> {
    use crate::db::schema::exercises::dsl::*;
    let mut conn = get_conn().await;
    let result = exercises
        .filter(id.eq(exercise_id))
        .first::<Exercise>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn update_exercise(p_exercise_id: i32, update: &UpdateExercise) -> Result<Exercise> {
    use crate::db::schema::exercises::dsl::*;
    let mut conn = get_conn().await;
    let result = diesel::update(exercises.filter(id.eq(p_exercise_id)))
        .set(update)
        .get_result::<Exercise>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn delete_exercise(p_exercise_id: i32) -> Result<usize> {
    use crate::db::schema::exercises::dsl::*;
    let mut conn = get_conn().await;
    let rows_deleted = diesel::delete(exercises.filter(id.eq(p_exercise_id)))
        .execute(&mut conn)
        .await?;
    Ok(rows_deleted)
}

pub async fn get_sets_for_workout(p_workout_id: i32) -> Result<Vec<Set>> {
    use crate::db::schema::sets::dsl::*;
    let mut conn = get_conn().await;
    let result = sets
        .filter(workout_id.eq(p_workout_id))
        .load::<Set>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn get_set(set_id: i32) -> Result<Set> {
    use crate::db::schema::sets::dsl::*;
    let mut conn = get_conn().await;
    let result = sets
        .filter(id.eq(set_id))
        .first::<Set>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn update_set(set_id: i32, update: &UpdateSet) -> Result<Set> {
    use crate::db::schema::sets::dsl::*;
    let mut conn = get_conn().await;
    let result = diesel::update(sets.filter(id.eq(set_id)))
        .set(update)
        .get_result::<Set>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn delete_set(set_id: i32) -> Result<usize> {
    use crate::db::schema::sets::dsl::*;
    let mut conn = get_conn().await;
    let rows_deleted = diesel::delete(sets.filter(id.eq(set_id)))
        .execute(&mut conn)
        .await?;
    Ok(rows_deleted)
}
