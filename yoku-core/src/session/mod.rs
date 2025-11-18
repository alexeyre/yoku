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
use futures::future;
use log::{error, warn};
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(uniffi::Object)]
pub struct Session {
    pub workout_id: Mutex<Option<i64>>,
    pub db_pool: SqlitePool,
    pub llm_backend: Arc<LlmInterface>, // Normal model for parsing and suggestions
}

const fn get_openai_api_key() -> &'static str {
    dotenv!("OPENAI_KEY")
}

impl Session {
    pub async fn new(db_path: &str, model: String) -> Result<Self> {
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
        Ok(Self {
            workout_id: Mutex::new(None),
            db_pool: pool,
            llm_backend,
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

    async fn is_exercise_new_for_session(&self, exercise_id: i64) -> Result<bool> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            let existing_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM workout_sets WHERE session_id = ?1 AND exercise_id = ?2",
            )
            .bind(workout_id)
            .bind(exercise_id)
            .fetch_one(&self.db_pool)
            .await?;
            Ok(existing_count == 0)
        } else {
            Ok(false)
        }
    }

    pub async fn add_set_from_parsed_with_modifications(
        &self,
        parsed: &ParsedSet,
    ) -> Result<Vec<crate::uniffi_interface::modifications::Modification>> {
        use crate::uniffi_interface::modifications::{Modification, ModificationType};

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

        let exercise_name = parsed.exercise.clone();
        let exercise = get_or_create_exercise(&self.db_pool, &exercise_name).await?;
        let is_new_exercise = self.is_exercise_new_for_session(exercise.id).await?;

        let weight = parsed.weight.unwrap_or(0.0) as f64;
        let reps = parsed.reps.unwrap_or(0) as i64;
        let set_count = parsed.set_count.unwrap_or(1).max(1) as i64;
        let parsed_rpe = parsed.rpe.map(|r| r as f64);

        let request =
            create_request_string_for_username(&self.db_pool, "cli", request_str_content.clone())
                .await?;

        let mut modifications = Vec::new();

        if set_count > 1 {
            let created_sets = add_multiple_sets_to_workout(
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

            if is_new_exercise {
                modifications.push(Modification {
                    modification_type: ModificationType::ExerciseAdded,
                    set_id: None,
                    exercise_id: Some(exercise.id),
                });
            } else {
                for set in created_sets {
                    modifications.push(Modification {
                        modification_type: ModificationType::SetAdded,
                        set_id: Some(set.id),
                        exercise_id: None,
                    });
                }
            }
        } else {
            let created_set = add_workout_set(
                &self.db_pool,
                &session_id,
                &exercise.id,
                &request.id,
                &weight,
                &reps,
                parsed_rpe,
            )
            .await?;

            if is_new_exercise {
                modifications.push(Modification {
                    modification_type: ModificationType::ExerciseAdded,
                    set_id: None,
                    exercise_id: Some(exercise.id),
                });
            } else {
                modifications.push(Modification {
                    modification_type: ModificationType::SetAdded,
                    set_id: Some(created_set.id),
                    exercise_id: None,
                });
            }
        }

        Ok(modifications)
    }

    pub async fn update_workout_set_with_modifications(
        &self,
        set_id: i64,
        update: &crate::db::models::UpdateWorkoutSet,
    ) -> Result<(db::models::WorkoutSet, Vec<crate::uniffi_interface::modifications::Modification>)> {
        use crate::uniffi_interface::modifications::{Modification, ModificationType};

        let updated = db::operations::update_workout_set(&self.db_pool, set_id, update).await?;

        let modifications = vec![Modification {
            modification_type: ModificationType::SetModified,
            set_id: Some(set_id),
            exercise_id: None,
        }];

        Ok((updated, modifications))
    }

    pub async fn delete_set_with_modifications(
        &self,
        set_id: i64,
    ) -> Result<Vec<crate::uniffi_interface::modifications::Modification>> {
        use crate::uniffi_interface::modifications::{Modification, ModificationType};

        delete_workout_set(&self.db_pool, set_id).await?;

        Ok(vec![Modification {
            modification_type: ModificationType::SetRemoved,
            set_id: Some(set_id),
            exercise_id: None,
        }])
    }

    pub async fn classify_and_process_input_with_modifications(
        &self,
        input: &str,
        selected_set_backend_id: Option<i64>,
        visible_set_backend_ids: Vec<i64>,
    ) -> Result<Vec<crate::uniffi_interface::modifications::Modification>> {
        let workout_id = self.get_workout_id().await;
        if workout_id.is_none() {
            return Err(anyhow::anyhow!("No active workout session"));
        }

        let workout_context = self.build_workout_context_string().await?;

        let known_exercises: Vec<String> = self
            .get_all_exercises()
            .await?
            .into_iter()
            .map(|exercise| exercise.name)
            .collect();
        let ctx = crate::llm::PromptContext {
            known_exercises,
            selected_set_backend_id,
            visible_set_backend_ids,
            ..Default::default()
        };
        let builder = crate::llm::PromptBuilder::new(ctx);

        let commands = crate::llm::classify_commands(
            self.llm_backend.as_ref(),
            &builder,
            input,
            &workout_context,
        )
        .await?;

        if commands.is_empty() {
            warn!("LLM returned empty command array for input: {}", input);
            return Ok(vec![]);
        }

        let sets = self.get_all_sets().await?;
        let exercises = self.get_all_exercises().await?;
        let exercise_map: std::collections::HashMap<i64, String> =
            exercises.iter().map(|e| (e.id, e.name.clone())).collect();

        let mut all_modifications = Vec::new();

        for command in commands {
            let mods = self
                .execute_command_with_modifications(command, &sets, &exercise_map)
                .await?;
            all_modifications.extend(mods);
        }

        Ok(all_modifications)
    }

    async fn execute_command_with_modifications(
        &self,
        command: crate::llm::Command,
        sets: &[crate::db::models::WorkoutSet],
        exercise_map: &std::collections::HashMap<i64, String>,
    ) -> Result<Vec<crate::uniffi_interface::modifications::Modification>> {
        match command {
            crate::llm::Command::AddSet {
                exercise,
                weight,
                reps,
                rpe,
                set_count,
                tags: _,
                aoi: _,
                original_string,
            } => {
                let parsed = ParsedSet {
                    exercise,
                    weight: weight.map(|w| w as f32),
                    reps: reps.map(|r| r as i32),
                    rpe: rpe.map(|r| r as f32),
                    set_count: set_count.map(|c| c as i32),
                    tags: vec![],
                    aoi: None,
                    original_string,
                };
                self.add_set_from_parsed_with_modifications(&parsed).await
            }
            crate::llm::Command::RemoveSet {
                set_id,
                description,
            } => {
                let resolved_id = if let Some(id) = set_id {
                    Some(id)
                } else if let Some(desc) = description {
                    self.resolve_set_id_from_description(&desc, sets, exercise_map)
                } else {
                    None
                };

                if let Some(id) = resolved_id {
                    self.delete_set_with_modifications(id).await
                } else {
                    Err(anyhow::anyhow!(
                        "Could not resolve set_id for remove_set command"
                    ))
                }
            }
            crate::llm::Command::EditSet {
                set_id,
                description,
                exercise,
                weight,
                reps,
                rpe,
            } => {
                let resolved_id = if let Some(id) = set_id {
                    Some(id)
                } else if let Some(desc) = description {
                    self.resolve_set_id_from_description(&desc, sets, exercise_map)
                } else {
                    None
                };

                if let Some(id) = resolved_id {
                    let exercise_id = if let Some(exercise_name) = exercise {
                        let ex = get_or_create_exercise(&self.db_pool, &exercise_name).await?;
                        Some(ex.id)
                    } else {
                        None
                    };

                    let update = crate::db::models::UpdateWorkoutSet {
                        session_id: None,
                        exercise_id,
                        request_string_id: None,
                        weight,
                        reps,
                        rpe,
                        set_index: None,
                        notes: None,
                    };
                    let (_, modifications) = self
                        .update_workout_set_with_modifications(id, &update)
                        .await?;
                    Ok(modifications)
                } else {
                    Err(anyhow::anyhow!(
                        "Could not resolve set_id for edit_set command"
                    ))
                }
            }
            crate::llm::Command::ChangeIntention { .. } => {
                Ok(vec![])
            }
            crate::llm::Command::Unknown { input } => {
                warn!("Unknown command for input: {}", input);
                let parsed = ParsedSet {
                    exercise: input.clone(),
                    weight: None,
                    reps: None,
                    rpe: None,
                    set_count: Some(1),
                    tags: vec![],
                    aoi: None,
                    original_string: input,
                };
                self.add_set_from_parsed_with_modifications(&parsed).await
            }
        }
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

    async fn build_workout_context_string(&self) -> Result<String> {
        let workout_id = self.get_workout_id().await;
        let Some(workout_id) = workout_id else {
            return Ok("No active workout session.".to_string());
        };

        // Get current workout
        let workout = get_workout_session(&self.db_pool, workout_id).await?;

        // Get all sets for current workout
        let sets = get_sets_for_session(&self.db_pool, workout_id).await?;

        // Get all exercises
        let exercises = self.get_all_exercises().await?;
        let exercise_map: std::collections::HashMap<i64, String> =
            exercises.iter().map(|e| (e.id, e.name.clone())).collect();

        // Sort sets by creation time (most recent first)
        let mut sorted_sets = sets.clone();
        sorted_sets.sort_by_key(|s| std::cmp::Reverse(s.created_at));

        // Build context string
        let mut context = String::new();

        // Current workout info
        context.push_str(&format!(
            "Current Workout: ID={}, Name={:?}\n",
            workout.id, workout.name
        ));
        if let Some(ref intention) = workout.intention {
            context.push_str(&format!("Intention: {}\n", intention));
        }
        context.push_str("\n");

        // Recent sets (last 10, most recent first)
        context.push_str("=== RECENT SETS (Most Recent First) ===\n");
        for (idx, set) in sorted_sets.iter().take(10).enumerate() {
            let exercise_name = exercise_map
                .get(&set.exercise_id)
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            let rpe_str = set
                .rpe
                .map(|r| format!(" @{:.1}RPE", r))
                .unwrap_or_default();
            context.push_str(&format!(
                "  [{}] Set ID={}, Exercise={}, Weight={:.1}kg, Reps={}, Set Index={}{}\n",
                idx + 1,
                set.id,
                exercise_name,
                set.weight,
                set.reps,
                set.set_index,
                rpe_str
            ));
        }
        context.push_str("\n");

        // All sets in workout (for reference)
        context.push_str("=== ALL SETS IN CURRENT WORKOUT ===\n");
        for set in &sets {
            let exercise_name = exercise_map
                .get(&set.exercise_id)
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            let rpe_str = set
                .rpe
                .map(|r| format!(" @{:.1}RPE", r))
                .unwrap_or_default();
            context.push_str(&format!(
                "  Set ID={}, Exercise={}, Weight={:.1}kg, Reps={}, Set Index={}{}, Created={}\n",
                set.id, exercise_name, set.weight, set.reps, set.set_index, rpe_str, set.created_at
            ));
        }
        context.push_str("\n");

        // Recent performance history per exercise (past 10 sets per exercise from all workouts)
        context.push_str("=== RECENT PERFORMANCE HISTORY (Past 10 sets per exercise) ===\n");
        let exercise_ids: std::collections::HashSet<i64> =
            sets.iter().map(|s| s.exercise_id).collect();
        for exercise_id in exercise_ids {
            if let Some(exercise_name) = exercise_map.get(&exercise_id) {
                match get_exercise_entries(&self.db_pool, exercise_id, Some(10)).await {
                    Ok(past_sets) if !past_sets.is_empty() => {
                        context.push_str(&format!("  {}:\n", exercise_name));
                        for past_set in past_sets.iter().take(10) {
                            let rpe_str = past_set
                                .rpe
                                .map(|r| format!(" @{:.1}RPE", r))
                                .unwrap_or_default();
                            context.push_str(&format!(
                                "    {:.1}kg x {} reps{}\n",
                                past_set.weight, past_set.reps, rpe_str
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(context)
    }

    fn resolve_set_id_from_description(
        &self,
        description: &str,
        sets: &[crate::db::models::WorkoutSet],
        exercise_map: &std::collections::HashMap<i64, String>,
    ) -> Option<i64> {
        let desc_lower = description.to_lowercase();

        // Sort sets by creation time (most recent first)
        let mut sorted_sets = sets.to_vec();
        sorted_sets.sort_by_key(|s| std::cmp::Reverse(s.created_at));

        // Try to match description
        if desc_lower.contains("most recent") || desc_lower.contains("last") || desc_lower == "that"
        {
            return sorted_sets.first().map(|s| s.id);
        }

        if desc_lower.contains("second to last") || desc_lower.contains("second last") {
            return sorted_sets.get(1).map(|s| s.id);
        }

        // Try to match by exercise name and position
        for exercise_name in exercise_map.values() {
            let ex_lower = exercise_name.to_lowercase();
            if desc_lower.contains(&ex_lower) {
                // Find sets for this exercise
                let exercise_sets: Vec<_> = sorted_sets
                    .iter()
                    .filter(|s| {
                        exercise_map
                            .get(&s.exercise_id)
                            .map(|n| n == exercise_name)
                            .unwrap_or(false)
                    })
                    .collect();

                if desc_lower.contains("last") || desc_lower.contains("most recent") {
                    return exercise_sets.first().map(|s| s.id);
                }

                // Try to match set index
                if let Some(idx_str) = desc_lower
                    .split_whitespace()
                    .find(|s| s.parse::<usize>().is_ok())
                {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if idx > 0 && idx <= exercise_sets.len() {
                            return Some(exercise_sets[idx - 1].id);
                        }
                    }
                }
            }
        }

        // Try to match by set index number
        if let Some(idx_str) = desc_lower
            .split_whitespace()
            .find(|s| s.parse::<usize>().is_ok())
        {
            if let Ok(idx) = idx_str.parse::<usize>() {
                if idx > 0 && idx <= sorted_sets.len() {
                    return Some(sorted_sets[idx - 1].id);
                }
            }
        }

        None
    }

    pub async fn classify_and_process_input(&self, input: &str) -> Result<()> {
        let workout_id = self.get_workout_id().await;
        if workout_id.is_none() {
            return Err(anyhow::anyhow!("No active workout session"));
        }

        // Build workout context
        let workout_context = self.build_workout_context_string().await?;

        // Get known exercises for prompt context
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

        // Classify input into commands using the fast model
        let commands = crate::llm::classify_commands(
            self.llm_backend.as_ref(),
            &builder,
            input,
            &workout_context,
        )
        .await?;

        if commands.is_empty() {
            warn!("LLM returned empty command array for input: {}", input);
            return Ok(());
        }

        // Get current sets and exercise map for set_id resolution
        let sets = self.get_all_sets().await?;
        let exercises = self.get_all_exercises().await?;
        let exercise_map: std::collections::HashMap<i64, String> =
            exercises.iter().map(|e| (e.id, e.name.clone())).collect();

        // Execute commands concurrently
        let mut tasks = Vec::new();
        for command in commands {
            let task = self.execute_command(command, &sets, &exercise_map);
            tasks.push(task);
        }

        // Collect results
        let results: Vec<Result<()>> = future::join_all(tasks).await;

        // Check for errors
        let mut errors = Vec::new();
        for (idx, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                error!("Command {} failed: {}", idx, e);
                errors.push(e);
            }
        }

        if !errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Some commands failed: {}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        Ok(())
    }

    async fn execute_command(
        &self,
        command: crate::llm::Command,
        sets: &[crate::db::models::WorkoutSet],
        exercise_map: &std::collections::HashMap<i64, String>,
    ) -> Result<()> {
        match command {
            crate::llm::Command::AddSet {
                exercise,
                weight,
                reps,
                rpe,
                set_count,
                tags: _,
                aoi: _,
                original_string,
            } => {
                // Convert to ParsedSet
                let parsed = ParsedSet {
                    exercise,
                    weight: weight.map(|w| w as f32),
                    reps: reps.map(|r| r as i32),
                    rpe: rpe.map(|r| r as f32),
                    set_count: set_count.map(|c| c as i32),
                    tags: vec![],
                    aoi: None,
                    original_string,
                };
                self.add_set_from_parsed(&parsed).await
            }
            crate::llm::Command::RemoveSet {
                set_id,
                description,
            } => {
                let resolved_id = if let Some(id) = set_id {
                    Some(id)
                } else if let Some(desc) = description {
                    self.resolve_set_id_from_description(&desc, sets, exercise_map)
                } else {
                    None
                };

                if let Some(id) = resolved_id {
                    self.delete_set(id).await?;
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Could not resolve set_id for remove_set command"
                    ))
                }
            }
            crate::llm::Command::EditSet {
                set_id,
                description,
                exercise,
                weight,
                reps,
                rpe,
            } => {
                let resolved_id = if let Some(id) = set_id {
                    Some(id)
                } else if let Some(desc) = description {
                    self.resolve_set_id_from_description(&desc, sets, exercise_map)
                } else {
                    None
                };

                if let Some(id) = resolved_id {
                    // Resolve exercise_id if exercise name is provided
                    let exercise_id = if let Some(exercise_name) = exercise {
                        let ex = get_or_create_exercise(&self.db_pool, &exercise_name).await?;
                        Some(ex.id)
                    } else {
                        None
                    };

                    // Build update
                    let update = crate::db::models::UpdateWorkoutSet {
                        session_id: None,
                        exercise_id,
                        request_string_id: None,
                        weight,
                        reps,
                        rpe,
                        set_index: None,
                        notes: None,
                    };
                    self.update_workout_set(id, &update).await?;
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Could not resolve set_id for edit_set command"
                    ))
                }
            }
            crate::llm::Command::ChangeIntention { intention } => {
                let intention_opt = if intention.trim().is_empty() {
                    None
                } else {
                    Some(intention.trim().to_string())
                };
                self.set_workout_intention(intention_opt).await
            }
            crate::llm::Command::Unknown { input } => {
                warn!("Unknown command for input: {}", input);
                // Fallback: try to treat as add_set
                self.add_set_from_string(&input).await
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
