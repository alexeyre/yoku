use diesel_async::RunQueryDsl;
use anyhow::Result;

use crate::{
    db::get_conn,
    db::models::{Exercise, NewExercise, Workout, NewWorkout, Set, NewSet},
    db::schema::{exercises, workouts, sets},
};

// pub async fn insert_exercise(data: NewExercise) -> Result<Exercise> {
//     let pool = get_pool().await;
//     let mut conn = pool.get().await?;
//     let result = diesel::insert_into(exercises::table)
//         .values(&data)
//         .get_result::<Exercise>(&mut conn)
//         .await?;
//     Ok(result)
// }

pub async fn get_exercises() -> Result<Vec<Exercise>> {
    use crate::db::schema::exercises::dsl::*;
    let mut conn = get_conn().await;
    let result = exercises
        .load::<Exercise>(&mut conn)
        .await?;
    Ok(result)
}

pub async fn create_workout() -> Result<Workout> {
    use crate::db::schema::workouts;

    let new_workout = NewWorkout {
        name: None,
        notes: None,
        performed_at: None
    };

    let mut conn = get_conn().await;
    let result = diesel::insert_into(workouts::table)
        .values(&new_workout)
        .get_result::<Workout>(&mut conn).await?;
    Ok(result)
}

pub async fn add_set(exercise: &str, weight: &f32, reps: &i32) -> Result<Set> {
    // create a new workout
    todo!()
}
