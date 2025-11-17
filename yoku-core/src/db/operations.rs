use anyhow::Result;
use diesel::RunQueryDsl;
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use log::{debug, error, info, warn};

use crate::{
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

pub fn create_workout_session(
    conn: &mut SqliteConnection,
    user_id: Option<i32>,
    name: Option<String>,
    notes: Option<String>,
    duration_seconds: Option<i32>,
) -> Result<WorkoutSession> {
    debug!(
        "create_workout_session called user_id={:?} name={:?} duration_seconds={:?}",
        user_id, name, duration_seconds
    );

    let date = chrono::Utc::now().date_naive().to_string();
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
        .get_result(conn)
        .map_err(|e| {
            error!("create_workout_session failed inserting to DB: {}", e);
            anyhow::Error::from(e)
        })?;

    info!("created workout session id={}", res.id);
    Ok(res)
}

pub fn get_workout_session(conn: &mut SqliteConnection, session_id: i32) -> Result<WorkoutSession> {
    debug!("get_workout_session called session_id={}", session_id);

    workout_sessions::table
        .find(session_id)
        .first::<WorkoutSession>(conn)
        .map_err(|e| {
            warn!("get_workout_session failed for id {}: {}", session_id, e);
            e.into()
        })
}

pub fn get_all_workout_sessions(conn: &mut SqliteConnection) -> Result<Vec<WorkoutSession>> {
    debug!("get_all_workout_sessions called");

    workout_sessions::table
        .load::<WorkoutSession>(conn)
        .map_err(|e| {
            warn!("get_all_workout_sessions failed: {}", e);
            e.into()
        })
}

pub fn delete_workout_session(conn: &mut SqliteConnection, session_id: i32) -> Result<usize> {
    debug!("delete_workout_session called session_id={}", session_id);

    diesel::delete(workout_sessions::table.find(session_id))
        .execute(conn)
        .map_err(|e| {
            warn!("delete_workout_session failed for id {}: {}", session_id, e);
            e.into()
        })
}

pub fn get_exercise(conn: &mut SqliteConnection, exercise_id: i32) -> Result<Exercise> {
    debug!("get_exercise called exercise_id={}", exercise_id);

    exercises::table
        .find(exercise_id)
        .first::<Exercise>(conn)
        .map_err(|e| {
            warn!("get_exercise failed for id {}: {}", exercise_id, e);
            e.into()
        })
}

pub fn get_all_exercises(conn: &mut SqliteConnection) -> Result<Vec<Exercise>> {
    debug!("get_all_exercises called");
    exercises::table.load::<Exercise>(conn).map_err(|e| {
        warn!("get_all_exercises failed: {}", e);
        e.into()
    })
}

pub fn get_or_create_exercise(
    conn: &mut SqliteConnection,
    exercise_name: &str,
) -> Result<Exercise> {
    debug!("get_or_create_exercise called name={}", exercise_name);

    if let Ok(exercise) = exercises::table
        .filter(exercises::name.eq(exercise_name))
        .first::<Exercise>(conn)
    {
        debug!(
            "found existing exercise id={} name={}",
            exercise.id, exercise.name
        );
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
        .get_result::<Exercise>(conn)
        .map_err(|e| {
            error!(
                "get_or_create_exercise failed inserting {}: {}",
                exercise_name, e
            );
            anyhow::Error::from(e)
        })?;

    info!("created exercise id={} name={}", created.id, created.name);
    Ok(created)
}

pub fn get_or_create_muscle(conn: &mut SqliteConnection, muscle_name: &str) -> Result<Muscle> {
    debug!("get_or_create_muscle called name={}", muscle_name);

    if let Ok(muscle) = muscles::table
        .filter(muscles::name.eq(muscle_name))
        .first::<Muscle>(conn)
    {
        debug!(
            "found existing muscle id={} name={}",
            muscle.id, muscle.name
        );
        return Ok(muscle);
    }

    let new = NewMuscle {
        name: muscle_name.to_string(),
    };

    let created = diesel::insert_into(muscles::table)
        .values(&new)
        .get_result::<Muscle>(conn)
        .map_err(|e| {
            error!(
                "get_or_create_muscle failed inserting {}: {}",
                muscle_name, e
            );
            anyhow::Error::from(e)
        })?;

    info!("created muscle id={} name={}", created.id, created.name);
    Ok(created)
}

pub fn get_or_create_user(conn: &mut SqliteConnection, username: &str) -> Result<User> {
    use crate::db::schema::users::dsl as users_dsl;

    debug!("get_or_create_user called username={}", username);

    if let Ok(u) = users_dsl::users
        .filter(users_dsl::username.eq(username))
        .first::<User>(conn)
    {
        debug!("found existing user id={} username={}", u.id, u.username);
        return Ok(u);
    }

    let new = NewUser {
        username: username.to_string(),
    };

    let created = diesel::insert_into(users_dsl::users)
        .values(&new)
        .get_result::<User>(conn)
        .map_err(|e| {
            error!("get_or_create_user failed inserting {}: {}", username, e);
            anyhow::Error::from(e)
        })?;

    info!(
        "created user id={} username={}",
        created.id, created.username
    );
    Ok(created)
}

pub fn create_request_string(
    conn: &mut SqliteConnection,
    user_id: i32,
    input: String,
) -> Result<RequestString> {
    debug!(
        "create_request_string called user_id={} input_len={}",
        user_id,
        input.len()
    );

    let new = NewRequestString {
        user_id,
        string: input,
    };

    diesel::insert_into(request_strings::table)
        .values(&new)
        .get_result::<RequestString>(conn)
        .map_err(|e| {
            error!(
                "create_request_string failed for user_id {}: {}",
                user_id, e
            );
            anyhow::Error::from(e)
        })
}

pub fn create_request_string_for_username(
    conn: &mut SqliteConnection,
    username: &str,
    input: String,
) -> Result<RequestString> {
    debug!(
        "create_request_string_for_username called username={}",
        username
    );
    let user = get_or_create_user(conn, username)?;
    create_request_string(conn, user.id, input)
}

pub fn add_workout_set(
    conn: &mut SqliteConnection,
    session_id: &i32,
    exercise_id: &i32,
    request_string_id: &i32,
    weight: &f32,
    reps: &i32,
    rpe: Option<f32>,
) -> Result<WorkoutSet> {
    debug!(
        "add_workout_set called session_id={} exercise_id={} weight={} reps={} rpe={:?}",
        session_id, exercise_id, weight, reps, rpe
    );

    let max_index: Option<i32> = workout_sets::table
        .filter(workout_sets::session_id.eq(session_id))
        .filter(workout_sets::exercise_id.eq(exercise_id))
        .select(max(workout_sets::set_index))
        .first(conn)
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
        .get_result::<WorkoutSet>(conn)
        .map_err(|e| {
            error!(
                "add_workout_set failed inserting session_id={} exercise_id={}: {}",
                session_id, exercise_id, e
            );
            anyhow::Error::from(e)
        })?;

    info!(
        "added workout set id={} session_id={} exercise_id={} set_index={}",
        created.id, created.session_id, created.exercise_id, created.set_index
    );
    Ok(created)
}

pub fn add_multiple_sets_to_workout(
    conn: &mut SqliteConnection,
    session_id: &i32,
    exercise_id: &i32,
    request_string_id: &i32,
    weight: &f32,
    reps: &i32,
    rpe: Option<f32>,
    set_count: i32,
) -> Result<Vec<WorkoutSet>> {
    debug!(
        "add_multiple_sets_to_workout called session_id={} exercise_id={} set_count={}",
        session_id, exercise_id, set_count
    );

    let max_index: Option<i32> = workout_sets::table
        .filter(workout_sets::session_id.eq(session_id))
        .filter(workout_sets::exercise_id.eq(exercise_id))
        .select(max(workout_sets::set_index))
        .first(conn)
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
        .get_results::<WorkoutSet>(conn)
        .map_err(|e| {
            error!(
                "add_multiple_sets_to_workout failed inserting session_id={} exercise_id={}: {}",
                session_id, exercise_id, e
            );
            anyhow::Error::from(e)
        })?;

    info!(
        "added {} workout sets starting_index={} session_id={} exercise_id={}",
        created.len(),
        starting_index,
        session_id,
        exercise_id
    );
    Ok(created)
}

pub fn get_sets_for_session(
    conn: &mut SqliteConnection,
    session_id: i32,
) -> Result<Vec<WorkoutSet>> {
    debug!("get_sets_for_session called session_id={}", session_id);
    workout_sets::table
        .filter(workout_sets::session_id.eq(session_id))
        .order_by(workout_sets::set_index.asc())
        .load::<WorkoutSet>(conn)
        .map_err(|e| {
            warn!(
                "get_sets_for_session failed for session_id {}: {}",
                session_id, e
            );
            e.into()
        })
}

pub fn update_workout_set(
    conn: &mut SqliteConnection,
    set_id: i32,
    update: &UpdateWorkoutSet,
) -> Result<WorkoutSet> {
    debug!("update_workout_set called set_id={}", set_id);
    diesel::update(workout_sets::table.find(set_id))
        .set(update)
        .get_result::<WorkoutSet>(conn)
        .map_err(|e| {
            warn!("update_workout_set failed for set_id {}: {}", set_id, e);
            e.into()
        })
}

pub fn update_workout_set_from_parsed(
    conn: &mut SqliteConnection,
    set_id: i32,
    parsed: &ParsedSet,
) -> Result<WorkoutSet> {
    debug!(
        "update_workout_set_from_parsed called set_id={} parsed={:?}",
        set_id, parsed
    );
    let original = workout_sets::table
        .find(set_id)
        .first::<WorkoutSet>(conn)
        .map_err(|e| {
            error!("failed to load original set id {}: {}", set_id, e);
            anyhow::Error::from(e)
        })?;

    let exercise_id_opt = if !parsed.exercise.is_empty() {
        let exercise = get_or_create_exercise(conn, &parsed.exercise)?;
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
        .get_result::<WorkoutSet>(conn)
        .map_err(|e| {
            error!("failed to update set id {}: {}", set_id, e);
            anyhow::Error::from(e)
        })
}

pub fn delete_workout_set(conn: &mut SqliteConnection, set_id: i32) -> Result<usize> {
    debug!("delete_workout_set called set_id={}", set_id);
    diesel::delete(workout_sets::table.find(set_id))
        .execute(conn)
        .map_err(|e| {
            warn!("delete_workout_set failed for set_id {}: {}", set_id, e);
            e.into()
        })
}

pub fn get_exercise_entries(
    conn: &mut SqliteConnection,
    exercise_id: i32,
    limit: Option<i64>,
) -> Result<Vec<WorkoutSet>> {
    debug!(
        "get_exercise_entries called exercise_id={:?} limit={:?}",
        exercise_id, limit
    );
    let query = workout_sets::table
        .filter(workout_sets::exercise_id.eq(exercise_id))
        .order(workout_sets::created_at.asc());

    let sets = if let Some(limit) = limit {
        query.limit(limit).load::<WorkoutSet>(conn)
    } else {
        query.load::<WorkoutSet>(conn)
    }
    .map_err(|e| {
        error!(
            "failed to load exercise entries for exercise id {}: {}",
            exercise_id, e
        );
        anyhow::Error::from(e)
    })?;

    Ok(sets)
}
