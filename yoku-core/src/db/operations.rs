use anyhow::Result;
use sqlx::SqlitePool;
use log::{debug, error, info, warn};

use crate::{
    db::models::{
        Exercise, Muscle, RequestString, UpdateWorkoutSet, User, WorkoutSession, WorkoutSet,
    },
    llm::ParsedSet,
};

pub(crate) fn slugify(name: &str) -> String {
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

pub async fn create_workout_session(
    pool: &SqlitePool,
    user_id: Option<i64>,
    name: Option<String>,
    notes: Option<String>,
    duration_seconds: Option<i64>,
) -> Result<WorkoutSession> {
    debug!(
        "create_workout_session called user_id={:?} name={:?} duration_seconds={:?}",
        user_id, name, duration_seconds
    );

    let date = chrono::Utc::now().date_naive().to_string();
    let dur_secs = duration_seconds.unwrap_or(0) as i64;
    let now = chrono::Utc::now().timestamp();

    let res = sqlx::query_as::<_, WorkoutSession>(
        "INSERT INTO workout_sessions (user_id, name, date, duration_seconds, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
         RETURNING id, user_id, name, date, duration_seconds, notes, created_at, updated_at"
    )
    .bind(user_id)
    .bind(name)
    .bind(date)
    .bind(dur_secs)
    .bind(notes)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!("create_workout_session failed inserting to DB: {}", e);
        anyhow::Error::from(e)
    })?;

    info!("created workout session id={}", res.id);
    Ok(res)
}

pub async fn get_workout_session(pool: &SqlitePool, session_id: i64) -> Result<WorkoutSession> {
    debug!("get_workout_session called session_id={}", session_id);

    sqlx::query_as::<_, WorkoutSession>(
        "SELECT id, user_id, name, date, duration_seconds, notes, created_at, updated_at
         FROM workout_sessions WHERE id = ?1"
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        warn!("get_workout_session failed for id {}: {}", session_id, e);
        anyhow::Error::from(e)
    })
}

pub async fn get_all_workout_sessions(pool: &SqlitePool) -> Result<Vec<WorkoutSession>> {
    debug!("get_all_workout_sessions called");

    sqlx::query_as::<_, WorkoutSession>(
        "SELECT id, user_id, name, date, duration_seconds, notes, created_at, updated_at
         FROM workout_sessions"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        warn!("get_all_workout_sessions failed: {}", e);
        anyhow::Error::from(e)
    })
}

pub async fn delete_workout_session(pool: &SqlitePool, session_id: i64) -> Result<u64> {
    debug!("delete_workout_session called session_id={}", session_id);

    let result = sqlx::query("DELETE FROM workout_sessions WHERE id = ?1")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| {
            warn!("delete_workout_session failed for id {}: {}", session_id, e);
            anyhow::Error::from(e)
        })?;

    Ok(result.rows_affected())
}

pub async fn get_exercise(pool: &SqlitePool, exercise_id: i64) -> Result<Exercise> {
    debug!("get_exercise called exercise_id={}", exercise_id);

    sqlx::query_as::<_, Exercise>(
        "SELECT id, slug, name, description, created_at, updated_at
         FROM exercises WHERE id = ?1"
    )
    .bind(exercise_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        warn!("get_exercise failed for id {}: {}", exercise_id, e);
        anyhow::Error::from(e)
    })
}

pub async fn get_all_exercises(pool: &SqlitePool) -> Result<Vec<Exercise>> {
    debug!("get_all_exercises called");
    sqlx::query_as::<_, Exercise>(
        "SELECT id, slug, name, description, created_at, updated_at FROM exercises"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        warn!("get_all_exercises failed: {}", e);
        anyhow::Error::from(e)
    })
}

pub async fn get_or_create_exercise(
    pool: &SqlitePool,
    exercise_name: &str,
) -> Result<Exercise> {
    debug!("get_or_create_exercise called name={}", exercise_name);

    if let Some(exercise) = sqlx::query_as::<_, Exercise>(
        "SELECT id, slug, name, description, created_at, updated_at
         FROM exercises WHERE name = ?1"
    )
    .bind(exercise_name)
    .fetch_optional(pool)
    .await?
    {
        debug!(
            "found existing exercise id={} name={}",
            exercise.id, exercise.name
        );
        return Ok(exercise);
    }

    let slug = slugify(exercise_name);
    let now = chrono::Utc::now().timestamp();

    let created = sqlx::query_as::<_, Exercise>(
        "INSERT INTO exercises (slug, name, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)
         RETURNING id, slug, name, description, created_at, updated_at"
    )
    .bind(slug)
    .bind(exercise_name)
    .bind(None::<String>)
    .bind(now)
    .fetch_one(pool)
    .await
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

pub async fn get_or_create_muscle(pool: &SqlitePool, muscle_name: &str) -> Result<Muscle> {
    debug!("get_or_create_muscle called name={}", muscle_name);

    if let Some(muscle) = sqlx::query_as::<_, Muscle>(
        "SELECT id, name, created_at, updated_at
         FROM muscles WHERE name = ?1"
    )
    .bind(muscle_name)
    .fetch_optional(pool)
    .await?
    {
        debug!(
            "found existing muscle id={} name={}",
            muscle.id, muscle.name
        );
        return Ok(muscle);
    }

    let now = chrono::Utc::now().timestamp();

    let created = sqlx::query_as::<_, Muscle>(
        "INSERT INTO muscles (name, created_at, updated_at)
         VALUES (?1, ?2, ?2)
         RETURNING id, name, created_at, updated_at"
    )
    .bind(muscle_name)
    .bind(now)
    .fetch_one(pool)
    .await
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

pub async fn get_or_create_user(pool: &SqlitePool, username: &str) -> Result<User> {
    debug!("get_or_create_user called username={}", username);

    if let Some(u) = sqlx::query_as::<_, User>(
        "SELECT id, username, created_at, updated_at
         FROM users WHERE username = ?1"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?
    {
        debug!("found existing user id={} username={}", u.id, u.username);
        return Ok(u);
    }

    let now = chrono::Utc::now().timestamp();

    let created = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, created_at, updated_at)
         VALUES (?1, ?2, ?2)
         RETURNING id, username, created_at, updated_at"
    )
    .bind(username)
    .bind(now)
    .fetch_one(pool)
    .await
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

pub async fn create_request_string(
    pool: &SqlitePool,
    user_id: i64,
    input: String,
) -> Result<RequestString> {
    debug!(
        "create_request_string called user_id={} input_len={}",
        user_id,
        input.len()
    );

    let now = chrono::Utc::now().timestamp();

    sqlx::query_as::<_, RequestString>(
        "INSERT INTO request_strings (user_id, string, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         RETURNING id, user_id, string, created_at, updated_at"
    )
    .bind(user_id)
    .bind(input)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!(
            "create_request_string failed for user_id {}: {}",
            user_id, e
        );
        anyhow::Error::from(e)
    })
}

pub async fn create_request_string_for_username(
    pool: &SqlitePool,
    username: &str,
    input: String,
) -> Result<RequestString> {
    debug!(
        "create_request_string_for_username called username={}",
        username
    );
    let user = get_or_create_user(pool, username).await?;
    create_request_string(pool, user.id, input).await
}

pub async fn add_workout_set(
    pool: &SqlitePool,
    session_id: &i64,
    exercise_id: &i64,
    request_string_id: &i64,
    weight: &f64,
    reps: &i64,
    rpe: Option<f64>,
) -> Result<WorkoutSet> {
    debug!(
        "add_workout_set called session_id={} exercise_id={} weight={} reps={} rpe={:?}",
        session_id, exercise_id, weight, reps, rpe
    );

    let max_index: Option<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT MAX(set_index) FROM workout_sets WHERE session_id = ?1 AND exercise_id = ?2"
    )
    .bind(session_id)
    .bind(exercise_id)
    .fetch_optional(pool)
    .await?;

    let next_index = max_index.map(|n| n + 1).unwrap_or(1);
    let now = chrono::Utc::now().timestamp();

    let created = sqlx::query_as::<_, WorkoutSet>(
        "INSERT INTO workout_sets (session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         RETURNING id, session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at"
    )
    .bind(session_id)
    .bind(exercise_id)
    .bind(request_string_id)
    .bind(weight)
    .bind(reps)
    .bind(next_index)
    .bind(rpe)
    .bind(None::<String>)
    .bind(now)
    .fetch_one(pool)
    .await
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

pub async fn add_multiple_sets_to_workout(
    pool: &SqlitePool,
    session_id: &i64,
    exercise_id: &i64,
    request_string_id: &i64,
    weight: &f64,
    reps: &i64,
    rpe: Option<f64>,
    set_count: i64,
) -> Result<Vec<WorkoutSet>> {
    debug!(
        "add_multiple_sets_to_workout called session_id={} exercise_id={} set_count={}",
        session_id, exercise_id, set_count
    );

    let max_index: Option<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT MAX(set_index) FROM workout_sets WHERE session_id = ?1 AND exercise_id = ?2"
    )
    .bind(session_id)
    .bind(exercise_id)
    .fetch_optional(pool)
    .await?;

    let starting_index = max_index.map(|n| n + 1).unwrap_or(1);
    let now = chrono::Utc::now().timestamp();

    let mut created = Vec::new();
    for i in 0..set_count {
        let set_index = starting_index + i;
        let set = sqlx::query_as::<_, WorkoutSet>(
            "INSERT INTO workout_sets (session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             RETURNING id, session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at"
        )
        .bind(session_id)
        .bind(exercise_id)
        .bind(request_string_id)
        .bind(weight)
        .bind(reps)
        .bind(set_index)
        .bind(rpe)
        .bind(None::<String>)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!(
                "add_multiple_sets_to_workout failed inserting session_id={} exercise_id={}: {}",
                session_id, exercise_id, e
            );
            anyhow::Error::from(e)
        })?;
        created.push(set);
    }

    info!(
        "added {} workout sets starting_index={} session_id={} exercise_id={}",
        created.len(),
        starting_index,
        session_id,
        exercise_id
    );
    Ok(created)
}

pub async fn get_sets_for_session(
    pool: &SqlitePool,
    session_id: i64,
) -> Result<Vec<WorkoutSet>> {
    debug!("get_sets_for_session called session_id={}", session_id);
    sqlx::query_as::<_, WorkoutSet>(
        "SELECT id, session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at
         FROM workout_sets WHERE session_id = ?1 ORDER BY set_index ASC"
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        warn!(
            "get_sets_for_session failed for session_id {}: {}",
            session_id, e
        );
        anyhow::Error::from(e)
    })
}

pub async fn update_workout_set(
    pool: &SqlitePool,
    set_id: i64,
    update: &UpdateWorkoutSet,
) -> Result<WorkoutSet> {
    debug!("update_workout_set called set_id={}", set_id);
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query_as::<_, WorkoutSet>(
        "UPDATE workout_sets SET
         session_id = CASE WHEN ?1 IS NOT NULL THEN ?1 ELSE session_id END,
         exercise_id = CASE WHEN ?2 IS NOT NULL THEN ?2 ELSE exercise_id END,
         request_string_id = CASE WHEN ?3 IS NOT NULL THEN ?3 ELSE request_string_id END,
         weight = CASE WHEN ?4 IS NOT NULL THEN ?4 ELSE weight END,
         reps = CASE WHEN ?5 IS NOT NULL THEN ?5 ELSE reps END,
         set_index = CASE WHEN ?6 IS NOT NULL THEN ?6 ELSE set_index END,
         rpe = CASE WHEN ?7 IS NOT NULL THEN ?7 ELSE rpe END,
         notes = ?8,
         updated_at = ?9
         WHERE id = ?10
         RETURNING id, session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at"
    )
    .bind(update.session_id)
    .bind(update.exercise_id)
    .bind(update.request_string_id)
    .bind(update.weight)
    .bind(update.reps)
    .bind(update.set_index)
    .bind(update.rpe)
    .bind(update.notes.clone())
    .bind(now)
    .bind(set_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        warn!("update_workout_set failed for set_id {}: {}", set_id, e);
        anyhow::Error::from(e)
    })
}

async fn get_workout_set_by_id(pool: &SqlitePool, set_id: i64) -> Result<WorkoutSet> {
    sqlx::query_as::<_, WorkoutSet>(
        "SELECT id, session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at
         FROM workout_sets WHERE id = ?1"
    )
    .bind(set_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::Error::from(e))
}

pub async fn update_workout_set_from_parsed(
    pool: &SqlitePool,
    set_id: i64,
    parsed: &ParsedSet,
) -> Result<WorkoutSet> {
    debug!(
        "update_workout_set_from_parsed called set_id={} parsed={:?}",
        set_id, parsed
    );
    let original = get_workout_set_by_id(pool, set_id).await
        .map_err(|e| {
            error!("failed to load original set id {}: {}", set_id, e);
            anyhow::Error::from(e)
        })?;

    let exercise_id_opt = if !parsed.exercise.is_empty() {
        let exercise = get_or_create_exercise(pool, &parsed.exercise).await?;
        if exercise.id != original.exercise_id {
            Some(exercise.id)
        } else {
            None
        }
    } else {
        None
    };

    let weight_opt = parsed.weight.map(|w| w as f64);
    let rpe_opt = parsed.rpe.map(|r| r as f64);

    let update = UpdateWorkoutSet {
        session_id: None,
        exercise_id: exercise_id_opt,
        request_string_id: None,
        weight: weight_opt,
        reps: parsed.reps.map(|r| r as i64),
        set_index: None,
        rpe: rpe_opt,
        notes: None,
    };

    update_workout_set(pool, set_id, &update).await
        .map_err(|e| {
            error!("failed to update set id {}: {}", set_id, e);
            anyhow::Error::from(e)
        })
}

pub async fn delete_workout_set(pool: &SqlitePool, set_id: i64) -> Result<u64> {
    debug!("delete_workout_set called set_id={}", set_id);
    let result = sqlx::query("DELETE FROM workout_sets WHERE id = ?1")
        .bind(set_id)
        .execute(pool)
        .await
        .map_err(|e| {
            warn!("delete_workout_set failed for set_id {}: {}", set_id, e);
            anyhow::Error::from(e)
        })?;
    Ok(result.rows_affected())
}

pub async fn get_exercise_entries(
    pool: &SqlitePool,
    exercise_id: i64,
    limit: Option<i64>,
) -> Result<Vec<WorkoutSet>> {
    debug!(
        "get_exercise_entries called exercise_id={:?} limit={:?}",
        exercise_id, limit
    );
    
    let sets = if let Some(limit) = limit {
        sqlx::query_as::<_, WorkoutSet>(
            "SELECT id, session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at
             FROM workout_sets WHERE exercise_id = ?1 ORDER BY created_at ASC LIMIT ?2"
        )
        .bind(exercise_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, WorkoutSet>(
            "SELECT id, session_id, exercise_id, request_string_id, weight, reps, set_index, rpe, notes, created_at, updated_at
             FROM workout_sets WHERE exercise_id = ?1 ORDER BY created_at ASC"
        )
        .bind(exercise_id)
        .fetch_all(pool)
        .await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_database;
    use crate::llm::ParsedSet;
    use sqlx::SqlitePool;
    use std::sync::Once;

    static INIT: Once = Once::new();

    async fn setup_test_db() -> SqlitePool {
        INIT.call_once(|| {
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .init();
        });

        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&pool)
            .await
            .unwrap();

        init_database(&pool).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_workout_session() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();

        assert!(session.id > 0);
        assert_eq!(session.user_id, None);
        assert_eq!(session.name, None);
        assert_eq!(session.duration_seconds, 0);
    }

    #[tokio::test]
    async fn test_create_workout_session_with_fields() {
        let pool = setup_test_db().await;

        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let session = create_workout_session(
            &pool,
            Some(user.id),
            Some("Test Workout".to_string()),
            Some("Test notes".to_string()),
            Some(3600),
        )
        .await
        .unwrap();

        assert!(session.id > 0);
        assert_eq!(session.user_id, Some(user.id));
        assert_eq!(session.name, Some("Test Workout".to_string()));
        assert_eq!(session.notes, Some("Test notes".to_string()));
        assert_eq!(session.duration_seconds, 3600);
    }

    #[tokio::test]
    async fn test_get_workout_session() {
        let pool = setup_test_db().await;

        let created = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let retrieved = get_workout_session(&pool, created.id).await.unwrap();

        assert_eq!(created.id, retrieved.id);
        assert_eq!(created.user_id, retrieved.user_id);
        assert_eq!(created.name, retrieved.name);
    }

    #[tokio::test]
    async fn test_get_workout_session_not_found() {
        let pool = setup_test_db().await;

        let result = get_workout_session(&pool, 99999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_workout_sessions() {
        let pool = setup_test_db().await;

        create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();

        let sessions = get_all_workout_sessions(&pool).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_workout_session() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let rows = delete_workout_session(&pool, session.id).await.unwrap();
        assert_eq!(rows, 1);

        let result = get_workout_session(&pool, session.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_exercise() {
        let pool = setup_test_db().await;

        let created = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let retrieved = get_exercise(&pool, created.id).await.unwrap();

        assert_eq!(created.id, retrieved.id);
        assert_eq!(created.name, retrieved.name);
        assert_eq!(created.name, "Bench Press");
    }

    #[tokio::test]
    async fn test_get_exercise_not_found() {
        let pool = setup_test_db().await;

        let result = get_exercise(&pool, 99999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_exercises() {
        let pool = setup_test_db().await;

        get_or_create_exercise(&pool, "Squat").await.unwrap();
        get_or_create_exercise(&pool, "Deadlift").await.unwrap();

        let exercises = get_all_exercises(&pool).await.unwrap();
        assert_eq!(exercises.len(), 2);
    }

    #[tokio::test]
    async fn test_get_or_create_exercise_new() {
        let pool = setup_test_db().await;

        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        assert_eq!(exercise.name, "Bench Press");
        assert!(!exercise.slug.is_empty());
    }

    #[tokio::test]
    async fn test_get_or_create_exercise_existing() {
        let pool = setup_test_db().await;

        let first = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let second = get_or_create_exercise(&pool, "Bench Press").await.unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.name, second.name);
    }

    #[tokio::test]
    async fn test_get_or_create_muscle() {
        let pool = setup_test_db().await;

        let muscle = get_or_create_muscle(&pool, "Chest").await.unwrap();
        assert_eq!(muscle.name, "Chest");
        assert!(muscle.id > 0);

        let same = get_or_create_muscle(&pool, "Chest").await.unwrap();
        assert_eq!(muscle.id, same.id);
    }

    #[tokio::test]
    async fn test_get_or_create_user() {
        let pool = setup_test_db().await;

        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        assert_eq!(user.username, "testuser");
        assert!(user.id > 0);

        let same = get_or_create_user(&pool, "testuser").await.unwrap();
        assert_eq!(user.id, same.id);
    }

    #[tokio::test]
    async fn test_create_request_string() {
        let pool = setup_test_db().await;

        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        assert_eq!(request.user_id, user.id);
        assert_eq!(request.string, "100kg x 5");
        assert!(request.id > 0);
    }

    #[tokio::test]
    async fn test_create_request_string_for_username() {
        let pool = setup_test_db().await;

        let request = create_request_string_for_username(&pool, "testuser", "100kg x 5".to_string())
            .await
            .unwrap();

        assert_eq!(request.string, "100kg x 5");
        assert!(request.id > 0);
    }

    #[tokio::test]
    async fn test_add_workout_set() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        let set = add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            Some(8.0),
        )
        .await
        .unwrap();

        assert_eq!(set.session_id, session.id);
        assert_eq!(set.exercise_id, exercise.id);
        assert_eq!(set.weight, 100.0);
        assert_eq!(set.reps, 5);
        assert_eq!(set.set_index, 1);
        assert_eq!(set.rpe, Some(8.0));
    }

    #[tokio::test]
    async fn test_add_workout_set_index_increment() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        let set1 = add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        let set2 = add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        assert_eq!(set1.set_index, 1);
        assert_eq!(set2.set_index, 2);
    }

    #[tokio::test]
    async fn test_add_multiple_sets_to_workout() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        let sets = add_multiple_sets_to_workout(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            Some(8.0),
            3,
        )
        .await
        .unwrap();

        assert_eq!(sets.len(), 3);
        assert_eq!(sets[0].set_index, 1);
        assert_eq!(sets[1].set_index, 2);
        assert_eq!(sets[2].set_index, 3);
        assert_eq!(sets[0].weight, 100.0);
        assert_eq!(sets[0].reps, 5);
    }

    #[tokio::test]
    async fn test_get_sets_for_session() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        let sets = get_sets_for_session(&pool, session.id).await.unwrap();
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].set_index, 1);
        assert_eq!(sets[1].set_index, 2);
    }

    #[tokio::test]
    async fn test_update_workout_set() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        let set = add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        let update = UpdateWorkoutSet {
            weight: Some(105.0),
            reps: Some(6),
            rpe: Some(9.0),
            notes: Some("Updated set".to_string()),
            ..Default::default()
        };

        let updated = update_workout_set(&pool, set.id, &update).await.unwrap();

        assert_eq!(updated.weight, 105.0);
        assert_eq!(updated.reps, 6);
        assert_eq!(updated.rpe, Some(9.0));
        assert_eq!(updated.notes, Some("Updated set".to_string()));
        assert_eq!(updated.session_id, set.session_id);
        assert_eq!(updated.exercise_id, set.exercise_id);
    }

    #[tokio::test]
    async fn test_update_workout_set_partial() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        let set = add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            Some(8.0),
        )
        .await
        .unwrap();

        let update = UpdateWorkoutSet {
            weight: Some(105.0),
            ..Default::default()
        };

        let updated = update_workout_set(&pool, set.id, &update).await.unwrap();

        assert_eq!(updated.weight, 105.0);
        assert_eq!(updated.reps, set.reps);
        assert_eq!(updated.rpe, set.rpe);
    }

    #[tokio::test]
    async fn test_delete_workout_set() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        let set = add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        let rows = delete_workout_set(&pool, set.id).await.unwrap();
        assert_eq!(rows, 1);

        let sets = get_sets_for_session(&pool, session.id).await.unwrap();
        assert_eq!(sets.len(), 0);
    }

    #[tokio::test]
    async fn test_get_exercise_entries() {
        let pool = setup_test_db().await;

        let session1 = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let session2 = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request1 = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();
        let request2 = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        add_workout_set(
            &pool,
            &session1.id,
            &exercise.id,
            &request1.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        add_workout_set(
            &pool,
            &session2.id,
            &exercise.id,
            &request2.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        let entries = get_exercise_entries(&pool, exercise.id, None).await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_get_exercise_entries_with_limit() {
        let pool = setup_test_db().await;

        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();

        for i in 0..5 {
            let session = create_workout_session(&pool, None, None, None, None)
                .await
                .unwrap();
            let request = create_request_string(&pool, user.id, format!("100kg x 5 {}", i))
                .await
                .unwrap();

            add_workout_set(
                &pool,
                &session.id,
                &exercise.id,
                &request.id,
                &100.0,
                &5,
                None,
            )
            .await
            .unwrap();
        }

        let entries = get_exercise_entries(&pool, exercise.id, Some(3)).await.unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_update_workout_set_from_parsed() {
        let pool = setup_test_db().await;

        let session = create_workout_session(&pool, None, None, None, None)
            .await
            .unwrap();
        let exercise = get_or_create_exercise(&pool, "Bench Press").await.unwrap();
        let user = get_or_create_user(&pool, "testuser").await.unwrap();
        let request = create_request_string(&pool, user.id, "100kg x 5".to_string())
            .await
            .unwrap();

        let set = add_workout_set(
            &pool,
            &session.id,
            &exercise.id,
            &request.id,
            &100.0,
            &5,
            None,
        )
        .await
        .unwrap();

        let parsed = ParsedSet {
            exercise: "Squat".to_string(),
            weight: Some(150.0),
            reps: Some(3),
            rpe: Some(9.0),
            set_count: None,
            tags: vec![],
            aoi: None,
            original_string: "150kg x 3 @9".to_string(),
        };

        let updated = update_workout_set_from_parsed(&pool, set.id, &parsed)
            .await
            .unwrap();

        let new_exercise = get_exercise(&pool, updated.exercise_id).await.unwrap();
        assert_eq!(new_exercise.name, "Squat");
        assert_eq!(updated.weight, 150.0);
        assert_eq!(updated.reps, 3);
        assert_eq!(updated.rpe, Some(9.0));
    }

    #[tokio::test]
    async fn test_slugify() {
        let slug = slugify("Bench Press");
        assert_eq!(slug, "bench-press");

        let slug = slugify("Dead Lift");
        assert_eq!(slug, "dead-lift");

        let slug = slugify("  Squat  ");
        assert_eq!(slug, "squat");

        let slug = slugify("Cable Fly (Upper)");
        assert_eq!(slug, "cable-fly-upper");
    }
}

impl Default for UpdateWorkoutSet {
    fn default() -> Self {
        UpdateWorkoutSet {
            session_id: None,
            exercise_id: None,
            request_string_id: None,
            weight: None,
            reps: None,
            rpe: None,
            set_index: None,
            notes: None,
        }
    }
}
