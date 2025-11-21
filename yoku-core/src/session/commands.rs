use crate::db::models::{UpdateWorkoutSet, WorkoutSet};
use crate::db::operations::{get_or_create_exercise, get_workout_session};
use crate::llm::{Command, ParsedSet, PromptBuilder, PromptContext, classify_commands};
use crate::session::Session;
use crate::uniffi_interface::modifications::Modification;
use anyhow::Result;
use futures::future::try_join_all;
use log::warn;
use std::collections::HashMap;

impl Session {
    pub async fn process_user_input(
        &self,
        input: &str,
        selected_set_backend_id: Option<i64>,
        visible_set_backend_ids: Vec<i64>,
    ) -> Result<Vec<Modification>> {
        let workout_id = self.get_workout_id().await;
        if workout_id.is_none() {
            return Err(anyhow::anyhow!("No active workout session"));
        }
        let workout_id = workout_id.unwrap();

        let current_summary = get_workout_session(&self.db_pool, workout_id)
            .await
            .ok()
            .and_then(|w| w.summary);

        let exercises = self.get_all_exercises().await?;
        let exercise_map: HashMap<i64, String> =
            exercises.iter().map(|e| (e.id, e.name.clone())).collect();
        let known_exercises: Vec<String> = exercises.iter().map(|e| e.name.clone()).collect();

        let workout_context = self.build_workout_context_string().await?;

        let ctx = PromptContext {
            known_exercises,
            selected_set_backend_id,
            visible_set_backend_ids,
            current_summary,
            ..Default::default()
        };
        let builder = PromptBuilder::new(ctx);

        let commands =
            classify_commands(self.llm_backend.as_ref(), &builder, input, &workout_context).await?;

        if commands.is_empty() {
            warn!("LLM returned empty command array for input: {}", input);
            return Ok(vec![]);
        }

        let sets = self.get_all_sets().await?;

        let modification_futures: Vec<_> = commands
            .into_iter()
            .map(|command| self.execute_command(command, &sets, &exercise_map))
            .collect();

        let modification_results = try_join_all(modification_futures).await?;
        let all_modifications: Vec<Modification> =
            modification_results.into_iter().flatten().collect();

        Ok(all_modifications)
    }

    async fn execute_command(
        &self,
        command: Command,
        sets: &[WorkoutSet],
        exercise_map: &HashMap<i64, String>,
    ) -> Result<Vec<Modification>> {
        match command {
            Command::AddSet {
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
            Command::RemoveSet {
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
            Command::EditSet {
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

                    let update = UpdateWorkoutSet {
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
            Command::UpdateSummary { message, emoji } => {
                let session_id = self
                    .get_workout_id()
                    .await
                    .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

                let summary_json = serde_json::json!({
                    "message": message.trim(),
                    "emoji": emoji.trim()
                });

                crate::db::operations::update_workout_summary(
                    &self.db_pool,
                    session_id,
                    summary_json.to_string(),
                )
                .await?;
                Ok(vec![])
            }
            Command::Unknown { input } => {
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
}
