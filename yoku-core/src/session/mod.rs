use crate::db::models::{Exercise, WorkoutSession};
use crate::db::operations::{
    add_multiple_sets_to_workout, add_workout_set, create_request_string_for_username,
    create_workout_session, get_or_create_exercise, get_sets_for_session, get_workout_session,
    update_workout_set_from_parsed,
};
use crate::llm::{LlmInterface, ParsedSet};
use crate::*;
use anyhow::Result;
use diesel::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use tokio::sync::Mutex;

#[derive(uniffi::Object)]
pub struct Session {
    pub workout_id: Mutex<Option<i32>>,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub llm_backend: Mutex<LlmInterface>,
}

const fn get_openai_api_key() -> &'static str {
    dotenv!("OPENAI_KEY")
}

impl Session {
    pub async fn new(db_path: &str, model: String) -> Result<Self> {
        // Ensure migrations are run once using a direct connection (keeps behavior from before)
        let mut conn = crate::db::get_conn_from_uri(db_path).await?;
        db::init_database(&mut conn).await?;

        // Create an r2d2 pool with a max of 2 connections
        let manager = ConnectionManager::<SqliteConnection>::new(db_path);
        let pool = Pool::builder()
            .max_size(2)
            .build(manager)
            .map_err(|e| anyhow::anyhow!(format!("Failed to create DB pool: {}", e)))?;

        let llm_backend =
            LlmInterface::new_openai(Some(get_openai_api_key().to_string()), Some(model)).await?;
        Ok(Self {
            workout_id: Mutex::new(None),
            db_pool: pool,
            llm_backend: Mutex::new(llm_backend),
        })
    }

    pub async fn set_workout_id(&self, workout_id: i32) -> Result<()> {
        let pool = self.db_pool.clone();
        // Validate the workout exists using a blocking DB call
        let _ = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            get_workout_session(&mut conn, workout_id)
        })
        .await??;
        *self.workout_id.lock().await = Some(workout_id);
        Ok(())
    }

    pub async fn new_workout(&self) -> Result<()> {
        let pool = self.db_pool.clone();
        let workout_id = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            create_workout_session(&mut conn, None, None, None, None).map(|w| w.id)
        })
        .await??;
        self.set_workout_id(workout_id).await
    }

    pub async fn new_workout_with_name(&self, name: &str) -> Result<()> {
        let pool = self.db_pool.clone();
        let name_owned = name.to_string();
        let workout = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            create_workout_session(&mut conn, None, Some(name_owned), None, None)
        })
        .await??;
        self.set_workout_id(workout.id).await
    }

    pub async fn get_workout_id(&self) -> Option<i32> {
        self.workout_id.lock().await.clone()
    }

    pub async fn get_workout_session(&self) -> Result<WorkoutSession> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            let pool = self.db_pool.clone();
            let workout = tokio::task::spawn_blocking(move || {
                let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                get_workout_session(&mut conn, workout_id)
            })
            .await??;
            Ok(workout)
        } else {
            Err(anyhow::anyhow!("No active workout"))
        }
    }

    pub async fn replace_set_from_parsed(&self, set_id: i32, parsed: &ParsedSet) -> Result<()> {
        // Build an owned ParsedSet to move into the blocking closure
        let parsed_owned = ParsedSet {
            exercise: parsed.exercise.clone(),
            weight: parsed.weight,
            reps: parsed.reps,
            rpe: parsed.rpe,
            set_count: parsed.set_count,
            tags: parsed.tags.clone(),
            aoi: parsed.aoi.clone(),
            original_string: parsed.original_string.clone(),
        };
        let pool = self.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            update_workout_set_from_parsed(&mut conn, set_id, &parsed_owned)
        })
        .await??;
        Ok(())
    }

    pub async fn get_all_sets(&self) -> Result<Vec<crate::db::models::WorkoutSet>> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            let pool = self.db_pool.clone();
            let sets = tokio::task::spawn_blocking(move || {
                let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                get_sets_for_session(&mut conn, workout_id)
            })
            .await??;
            Ok(sets)
        } else {
            Err(anyhow::anyhow!("No active workout"))
        }
    }

    pub async fn get_all_exercises(&self) -> Result<Vec<Exercise>> {
        let pool = self.db_pool.clone();
        let exercises = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            db::operations::get_all_exercises(&mut conn)
        })
        .await??;
        Ok(exercises)
    }

    pub async fn get_all_workouts(&self) -> Result<Vec<WorkoutSession>> {
        let pool = self.db_pool.clone();
        let workouts = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            db::operations::get_all_workout_sessions(&mut conn)
        })
        .await??;
        Ok(workouts)
    }

    pub async fn add_set_from_string(&self, request_string: &str) -> Result<()> {
        let ctx = crate::llm::PromptContext {
            known_exercises: vec![],
            ..Default::default()
        };
        let builder = crate::llm::PromptBuilder::new(ctx);
        let backend = self.llm_backend.lock().await;
        let parsed = crate::llm::parse_set_string(&backend, &builder, &request_string).await?;
        self.add_set_from_parsed(&parsed).await
    }

    pub async fn add_set_from_parsed(&self, parsed: &ParsedSet) -> Result<()> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        let pool = self.db_pool.clone();

        // get or create exercise (blocking)
        let exercise = {
            let pool = pool.clone();
            let exercise_name = parsed.exercise.clone();
            tokio::task::spawn_blocking(move || {
                let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                get_or_create_exercise(&mut conn, &exercise_name)
            })
            .await??
        };

        let weight = parsed.weight.unwrap_or(0.0);
        let reps = parsed.reps.unwrap_or(0);
        let set_count = parsed.set_count.unwrap_or(1).max(1);
        let parsed_rpe = parsed.rpe;

        let request_str_content = if !parsed.original_string.is_empty() {
            parsed.original_string.clone()
        } else {
            format!(
                "{} {} reps rpe:{:?}",
                parsed.exercise,
                parsed.reps.unwrap_or(0),
                parsed.rpe
            )
        };

        // create request string (blocking) and get id
        let request_string_id = {
            let pool = pool.clone();
            let content = request_str_content.clone();
            tokio::task::spawn_blocking(move || {
                let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                let req = create_request_string_for_username(&mut conn, "cli", content)?;
                Ok::<i32, anyhow::Error>(req.id)
            })
            .await??
        };

        if set_count > 1 {
            let pool = pool.clone();
            tokio::task::spawn_blocking(move || {
                let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                add_multiple_sets_to_workout(
                    &mut conn,
                    &session_id,
                    &exercise.id,
                    &request_string_id,
                    &weight,
                    &reps,
                    parsed_rpe,
                    set_count,
                )
            })
            .await??;
        } else {
            let pool = pool.clone();
            tokio::task::spawn_blocking(move || {
                let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                add_workout_set(
                    &mut conn,
                    &session_id,
                    &exercise.id,
                    &request_string_id,
                    &weight,
                    &reps,
                    parsed_rpe,
                )
            })
            .await??;
        }

        Ok(())
    }
}
