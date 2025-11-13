use anyhow::Result;
use diesel::RunQueryDsl;
use diesel::dsl::max;
use diesel::prelude::*;

use crate::{
    db::get_conn,
    db::models::{
        Exercise, Muscle, NewExercise, NewMuscle, NewRequestString, NewUser, NewWorkoutSession,
        NewWorkoutSet, RequestString, UpdateWorkoutSet, User, WorkoutSession, WorkoutSet,
    },
    db::schema::{exercises, muscles, request_strings, workout_sessions, workout_sets},
    llm::ParsedSet,
};

fn slugify(name: &str) -> String {
    let mut s = name.trim().to_lowercase();
    s = s
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' => c,
            ' ' | '_' | '-' => '-',
            _ => '-',
        })
        .collect();

    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for ch in s.chars() {
        if ch == '-' {
            if !prev_dash {
                out.push(ch);
                prev_dash = true;
            }
        } else {
            out.push(ch);
            prev_dash = false;
        }
    }

    out.trim_matches('-').to_string()
}

/// Create a new workout session. The DB `id` is generated (UUID v4 bytes) by this function.
pub async fn create_workout_session(
    user_id: Option<i32>,
    name: Option<String>,
    notes: Option<String>,
    duration_seconds: Option<i32>,
) -> Result<WorkoutSession> {
    // lock the shared SqliteConnection mutex
    let mut conn = get_conn().await.lock().await;

    // store date as ISO-8601 string (schema defines `date` as Text)
    let date = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let dur_secs = duration_seconds.unwrap_or(0);

    let new = NewWorkoutSession {
        user_id,
        name,
        date,
        duration_seconds: dur_secs,
        notes,
    };

    let res = diesel::insert_into(workout_sessions::table)
        .values(&new)
        .returning(WorkoutSession::as_returning())
        .get_result(&mut *conn)
        .map_err(anyhow::Error::from)?;

    Ok(res)
}

pub async fn get_workout_session(session_id: i32) -> Result<WorkoutSession> {
    let mut conn = get_conn().await.lock().await;
    workout_sessions::table
        .find(session_id)
        .first::<WorkoutSession>(&mut *conn)
        .map_err(Into::into)
}

pub async fn get_all_workout_sessions() -> Result<Vec<WorkoutSession>> {
    let mut conn = get_conn().await.lock().await;
    workout_sessions::table
        .load::<WorkoutSession>(&mut *conn)
        .map_err(Into::into)
}

pub async fn delete_workout_session(session_id: i32) -> Result<usize> {
    let mut conn = get_conn().await.lock().await;
    diesel::delete(workout_sessions::table.find(session_id))
        .execute(&mut *conn)
        .map_err(Into::into)
}

pub async fn get_exercise(exercise_id: i32) -> Result<Exercise> {
    let mut conn = get_conn().await.lock().await;
    exercises::table
        .find(exercise_id)
        .first::<Exercise>(&mut *conn)
        .map_err(Into::into)
}

pub async fn get_all_exercises() -> Result<Vec<Exercise>> {
    let mut conn = get_conn().await.lock().await;
    exercises::table
        .load::<Exercise>(&mut *conn)
        .map_err(Into::into)
}

/// Get an exercise by name or create it. When creating, generate a UUID v4 (16 bytes) for `id`.
pub async fn get_or_create_exercise(exercise_name: &str) -> Result<Exercise> {
    let mut conn = get_conn().await.lock().await;

    if let Ok(exercise) = exercises::table
        .filter(exercises::name.eq(exercise_name))
        .first::<Exercise>(&mut *conn)
    {
        return Ok(exercise);
    }

    let slug = slugify(exercise_name);

    let new = NewExercise {
        slug,
        name: exercise_name.to_string(),
        description: None,
    };

    let created = diesel::insert_into(exercises::table)
        .values(&new)
        .get_result::<Exercise>(&mut *conn)
        .map_err(anyhow::Error::from)?;

    Ok(created)
}

/// Get a muscle by name or create it (generates id).
pub async fn get_or_create_muscle(muscle_name: &str) -> Result<Muscle> {
    let mut conn = get_conn().await.lock().await;

    if let Ok(muscle) = muscles::table
        .filter(muscles::name.eq(muscle_name))
        .first::<Muscle>(&mut *conn)
    {
        return Ok(muscle);
    }

    let new = NewMuscle {
        name: muscle_name.to_string(),
    };

    let created = diesel::insert_into(muscles::table)
        .values(&new)
        .get_result::<Muscle>(&mut *conn)
        .map_err(anyhow::Error::from)?;

    Ok(created)
}

/// Get or create a user by username. When creating, generate id.
pub async fn get_or_create_user(username: &str) -> Result<User> {
    use crate::db::schema::users::dsl as users_dsl;
    let mut conn = get_conn().await.lock().await;

    if let Ok(u) = users_dsl::users
        .filter(users_dsl::username.eq(username))
        .first::<User>(&mut *conn)
    {
        return Ok(u);
    }

    let new = NewUser {
        username: username.to_string(),
    };

    let created = diesel::insert_into(users_dsl::users)
        .values(&new)
        .get_result::<User>(&mut *conn)
        .map_err(anyhow::Error::from)?;

    Ok(created)
}

/// Create a request string row. Generates id for the row.
pub async fn create_request_string(user_id: i32, input: String) -> Result<RequestString> {
    let mut conn = get_conn().await.lock().await;
    let new = NewRequestString {
        user_id,
        string: input,
    };

    diesel::insert_into(request_strings::table)
        .values(&new)
        .get_result::<RequestString>(&mut *conn)
        .map_err(anyhow::Error::from)
}

pub async fn create_request_string_for_username(
    username: &str,
    input: String,
) -> Result<RequestString> {
    let user = get_or_create_user(username).await?;
    create_request_string(user.id, input).await
}

/// Add a single workout set (generates an id for the set row).
pub async fn add_workout_set(
    session_id: &i32,
    exercise_id: &i32,
    request_string_id: &i32,
    weight: &f32,
    reps: &i32,
    rpe: Option<f32>,
) -> Result<WorkoutSet> {
    let mut conn = get_conn().await.lock().await;

    let max_index: Option<i32> = workout_sets::table
        .filter(workout_sets::session_id.eq(session_id))
        .filter(workout_sets::exercise_id.eq(exercise_id))
        .select(max(workout_sets::set_index))
        .first(&mut *conn)
        .ok()
        .flatten();

    let next_index = max_index.map(|n| n + 1).unwrap_or(1);

    let new = NewWorkoutSet {
        session_id: session_id.clone(),
        exercise_id: exercise_id.clone(),
        request_string_id: request_string_id.clone(),
        weight: *weight,
        reps: *reps,
        set_index: next_index,
        rpe,
        notes: None,
    };

    let created = diesel::insert_into(workout_sets::table)
        .values(&new)
        .get_result::<WorkoutSet>(&mut *conn)
        .map_err(anyhow::Error::from)?;

    Ok(created)
}

/// Add multiple sets to a workout. Each set gets its own generated id.
pub async fn add_multiple_sets_to_workout(
    session_id: &i32,
    exercise_id: &i32,
    request_string_id: &i32,
    weight: &f32,
    reps: &i32,
    rpe: Option<f32>,
    set_count: i32,
) -> Result<Vec<WorkoutSet>> {
    let mut conn = get_conn().await.lock().await;

    let max_index: Option<i32> = workout_sets::table
        .filter(workout_sets::session_id.eq(session_id))
        .filter(workout_sets::exercise_id.eq(exercise_id))
        .select(max(workout_sets::set_index))
        .first(&mut *conn)
        .ok()
        .flatten();

    let starting_index = max_index.map(|n| n + 1).unwrap_or(1);

    let new_sets: Vec<NewWorkoutSet> = (0..set_count)
        .map(|i| NewWorkoutSet {
            session_id: session_id.clone(),
            exercise_id: exercise_id.clone(),
            request_string_id: request_string_id.clone(),
            weight: *weight,
            reps: *reps,
            set_index: starting_index + i,
            rpe,
            notes: None,
        })
        .collect();

    let created = diesel::insert_into(workout_sets::table)
        .values(&new_sets)
        .get_results::<WorkoutSet>(&mut *conn)
        .map_err(anyhow::Error::from)?;

    Ok(created)
}

pub async fn get_sets_for_session(session_id: i32) -> Result<Vec<WorkoutSet>> {
    let mut conn = get_conn().await.lock().await;
    workout_sets::table
        .filter(workout_sets::session_id.eq(session_id))
        .order_by(workout_sets::set_index.asc())
        .load::<WorkoutSet>(&mut *conn)
        .map_err(Into::into)
}

pub async fn update_workout_set(set_id: i32, update: &UpdateWorkoutSet) -> Result<WorkoutSet> {
    let mut conn = get_conn().await.lock().await;
    diesel::update(workout_sets::table.find(set_id))
        .set(update)
        .get_result::<WorkoutSet>(&mut *conn)
        .map_err(Into::into)
}

pub async fn update_workout_set_from_parsed(set_id: i32, parsed: &ParsedSet) -> Result<WorkoutSet> {
    let mut conn = get_conn().await.lock().await;

    let original = workout_sets::table
        .find(set_id)
        .first::<WorkoutSet>(&mut *conn)
        .map_err(anyhow::Error::from)?;

    let exercise_id_opt = if !parsed.exercise.is_empty() {
        let exercise = get_or_create_exercise(&parsed.exercise).await?;
        if exercise.id != original.exercise_id {
            Some(exercise.id)
        } else {
            None
        }
    } else {
        None
    };

    let weight_opt = parsed.weight;
    let rpe_opt = parsed.rpe;

    let update = UpdateWorkoutSet {
        session_id: None,
        exercise_id: exercise_id_opt,
        request_string_id: None,
        weight: weight_opt,
        reps: parsed.reps,
        set_index: None,
        rpe: rpe_opt,
        notes: None,
    };

    diesel::update(workout_sets::table.find(set_id))
        .set(&update)
        .get_result::<WorkoutSet>(&mut *conn)
        .map_err(anyhow::Error::from)
}

pub async fn delete_workout_set(set_id: i32) -> Result<usize> {
    let mut conn = get_conn().await.lock().await;
    diesel::delete(workout_sets::table.find(set_id))
        .execute(&mut *conn)
        .map_err(Into::into)
}
