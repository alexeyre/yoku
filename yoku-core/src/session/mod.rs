use crate::db;
use crate::db::models::{Exercise, WorkoutSession};
use crate::db::operations::{
    add_multiple_sets_to_workout, add_workout_set, create_request_string_for_username,
    create_workout_session, delete_workout_session, delete_workout_set, get_exercise_entries,
    get_or_create_exercise, get_sets_for_session, get_workout_session, update_workout_intention,
    update_workout_set_from_parsed,
};
use crate::llm::{LlmInterface, ParsedSet};
use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(uniffi::Object)]
pub struct Session {
    pub workout_id: Mutex<Option<i64>>,
    pub db_pool: SqlitePool,
    pub llm_backend: Arc<LlmInterface>, // Normal model for parsing and suggestions
    pub fast_llm_backend: Arc<LlmInterface>, // Fast model for classification
}

const fn get_openai_api_key() -> &'static str {
    dotenv!("OPENAI_KEY")
}

impl Session {
    pub async fn new(db_path: &str, model: String, fast_model: String) -> Result<Self> {
        // Create SQLx pool - SQLite will create the database file if it doesn't exist
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Failed to create DB pool: {}", e)))?;

        // Set SQLite PRAGMAs
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&pool)
            .await?;

        // Run migrations - this will create tables if they don't exist
        db::init_database(&pool).await?;

        let llm_backend = Arc::new(
            LlmInterface::new_openai(Some(get_openai_api_key().to_string()), Some(model)).await?,
        );
        let fast_llm_backend = Arc::new(
            LlmInterface::new_openai(Some(get_openai_api_key().to_string()), Some(fast_model))
                .await?,
        );
        Ok(Self {
            workout_id: Mutex::new(None),
            db_pool: pool,
            llm_backend,
            fast_llm_backend,
        })
    }

    pub async fn delete_workout(&self, workout_id: i64) -> Result<u64> {
        delete_workout_session(&self.db_pool, workout_id).await
    }

    pub async fn delete_set(&self, set_id: i64) -> Result<u64> {
        delete_workout_set(&self.db_pool, set_id).await
    }

    pub async fn set_workout_id(&self, workout_id: i64) -> Result<()> {
        // Validate the workout exists
        let _ = get_workout_session(&self.db_pool, workout_id).await?;
        *self.workout_id.lock().await = Some(workout_id);
        Ok(())
    }

    pub async fn new_workout(&self) -> Result<()> {
        let workout = create_workout_session(&self.db_pool, None, None, None, None).await?;
        self.set_workout_id(workout.id).await
    }

    pub async fn new_workout_with_name(&self, name: &str) -> Result<()> {
        let workout =
            create_workout_session(&self.db_pool, None, Some(name.to_string()), None, None).await?;
        self.set_workout_id(workout.id).await
    }

    pub async fn get_sets_for_exercise(
        &self,
        exercise_id: i64,
        limit: Option<i64>,
    ) -> Result<Vec<crate::db::models::WorkoutSet>> {
        get_exercise_entries(&self.db_pool, exercise_id, limit).await
    }

    pub async fn get_workout_id(&self) -> Option<i64> {
        self.workout_id.lock().await.clone()
    }

    pub async fn get_workout_session(&self) -> Result<WorkoutSession> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            get_workout_session(&self.db_pool, workout_id).await
        } else {
            Err(anyhow::anyhow!("No active workout"))
        }
    }

    pub async fn replace_set_from_parsed(&self, set_id: i64, parsed: &ParsedSet) -> Result<()> {
        update_workout_set_from_parsed(&self.db_pool, set_id, parsed).await?;
        Ok(())
    }

    pub async fn get_all_sets(&self) -> Result<Vec<crate::db::models::WorkoutSet>> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            get_sets_for_session(&self.db_pool, workout_id).await
        } else {
            Err(anyhow::anyhow!("No active workout"))
        }
    }

    pub async fn update_workout_set(
        &self,
        set_id: i64,
        update: &crate::db::models::UpdateWorkoutSet,
    ) -> Result<db::models::WorkoutSet> {
        db::operations::update_workout_set(&self.db_pool, set_id, update).await
    }

    pub async fn get_all_exercises(&self) -> Result<Vec<Exercise>> {
        db::operations::get_all_exercises(&self.db_pool).await
    }

    pub async fn get_all_workouts(&self) -> Result<Vec<WorkoutSession>> {
        db::operations::get_all_workout_sessions(&self.db_pool).await
    }

    pub async fn add_set_from_string(&self, request_string: &str) -> Result<()> {
        let known_exercises: Vec<String> = self
            .get_all_exercises()
            .await?
            .into_iter()
            .map(|exercise| exercise.name)
            .collect();
        let ctx = crate::llm::PromptContext {
            known_exercises,
            ..Default::default()
        };
        let builder = crate::llm::PromptBuilder::new(ctx);
        let parsed =
            crate::llm::parse_set_string(self.llm_backend.as_ref(), &builder, request_string)
                .await?;
        self.add_set_from_parsed(&parsed).await
    }

    pub async fn classify_and_process_input(&self, input: &str) -> Result<()> {
        let known_exercises: Vec<String> = self
            .get_all_exercises()
            .await?
            .into_iter()
            .map(|exercise| exercise.name)
            .collect();
        let ctx = crate::llm::PromptContext {
            known_exercises,
            ..Default::default()
        };
        let builder = crate::llm::PromptBuilder::new(ctx);

        // Classify the input type using the fast model
        let input_type =
            crate::llm::classify_input_type(self.fast_llm_backend.as_ref(), &builder, input)
                .await?;

        match input_type {
            crate::llm::InputType::Intention => {
                // Extract intention from natural language
                // For now, use the input as-is, but could enhance with LLM extraction
                let intention = if input.trim().is_empty() {
                    None
                } else {
                    Some(input.trim().to_string())
                };
                self.set_workout_intention(intention).await
            }
            crate::llm::InputType::Set => {
                // Parse and add as a set
                self.add_set_from_string(input).await
            }
        }
    }

    pub async fn add_set_from_parsed(&self, parsed: &ParsedSet) -> Result<()> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

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

        // Use a transaction - operations need to be updated to accept transactions
        // For now, execute operations sequentially on the pool
        // TODO: Update operations to accept Executor trait for transaction support
        let exercise_name = parsed.exercise.clone();
        let exercise = get_or_create_exercise(&self.db_pool, &exercise_name).await?;

        let weight = parsed.weight.unwrap_or(0.0) as f64;
        let reps = parsed.reps.unwrap_or(0) as i64;
        let set_count = parsed.set_count.unwrap_or(1).max(1) as i64;
        let parsed_rpe = parsed.rpe.map(|r| r as f64);

        let request =
            create_request_string_for_username(&self.db_pool, "cli", request_str_content.clone())
                .await?;

        if set_count > 1 {
            add_multiple_sets_to_workout(
                &self.db_pool,
                &session_id,
                &exercise.id,
                &request.id,
                &weight,
                &reps,
                parsed_rpe,
                set_count,
            )
            .await?;
        } else {
            add_workout_set(
                &self.db_pool,
                &session_id,
                &exercise.id,
                &request.id,
                &weight,
                &reps,
                parsed_rpe,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn set_workout_intention(&self, intention: Option<String>) -> Result<()> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;
        update_workout_intention(&self.db_pool, session_id, intention).await
    }

    pub async fn get_workout_suggestions(&self) -> Result<Vec<crate::llm::WorkoutSuggestion>> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        // Get current workout data
        let sets = get_sets_for_session(&self.db_pool, session_id).await?;
        let workout = get_workout_session(&self.db_pool, session_id).await?;

        // Group sets by exercise
        let mut exercise_counts: std::collections::HashMap<i64, i64> =
            std::collections::HashMap::new();
        for set in &sets {
            *exercise_counts.entry(set.exercise_id).or_insert(0) += 1;
        }

        // Get exercise names
        let all_exercises = self.get_all_exercises().await?;
        let exercise_map: std::collections::HashMap<i64, String> =
            all_exercises.into_iter().map(|e| (e.id, e.name)).collect();

        // Build current exercises list
        let current_exercises: Vec<(String, i64)> = exercise_counts
            .iter()
            .filter_map(|(ex_id, count)| exercise_map.get(ex_id).map(|name| (name.clone(), *count)))
            .collect();

        // Build past performance summary
        let mut past_performance_parts = Vec::new();
        for (ex_id, count) in &exercise_counts {
            if let Some(ex_name) = exercise_map.get(ex_id) {
                let past_sets = get_exercise_entries(&self.db_pool, *ex_id, Some(10))
                    .await
                    .ok();
                if let Some(sets) = past_sets {
                    if !sets.is_empty() {
                        let avg_weight =
                            sets.iter().map(|s| s.weight).sum::<f64>() / sets.len() as f64;
                        let avg_reps = sets.iter().map(|s| s.reps).sum::<i64>() / sets.len() as i64;
                        past_performance_parts.push(format!(
                            "{}: avg {:.1}kg x {} reps (from {} recent sets)",
                            ex_name,
                            avg_weight,
                            avg_reps,
                            sets.len()
                        ));
                    }
                }
            }
        }
        let past_performance = if past_performance_parts.is_empty() {
            "No significant past performance data available.".to_string()
        } else {
            past_performance_parts.join("\n")
        };

        // Get known exercises for context
        let known_exercises: Vec<String> = exercise_map.values().cloned().collect();
        let ctx = crate::llm::PromptContext {
            known_exercises,
            ..Default::default()
        };
        let builder = crate::llm::PromptBuilder::new(ctx);

        // Generate suggestions using LLM
        crate::llm::generate_workout_suggestions(
            self.llm_backend.as_ref(),
            &builder,
            &current_exercises,
            workout.intention.as_deref(),
            &past_performance,
        )
        .await
    }
}
