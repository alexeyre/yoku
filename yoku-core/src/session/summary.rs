//! Workout summary and suggestions generation.

use crate::db::operations::{
    get_exercise_entries, get_sets_for_session, get_workout_session, update_workout_summary,
};
use crate::llm::{
    PromptBuilder, PromptContext, WorkoutSuggestion, WorkoutSummary, generate_workout_suggestions,
    generate_workout_summary,
};
use crate::session::Session;
use crate::uniffi_interface::objects::{
    ActiveWorkoutState, Exercise as UniffiExercise, WorkoutSession as UniffiWorkoutSession,
    WorkoutSet as UniffiWorkoutSet,
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

impl Session {
    /// Get the active workout state.
    pub async fn get_active_workout_state(&self) -> Result<ActiveWorkoutState> {
        let workout_id = self.get_workout_id().await;
        let Some(workout_id) = workout_id else {
            return Err(anyhow::anyhow!("No active workout"));
        };

        let workout = get_workout_session(&self.db_pool, workout_id).await?;
        let sets = get_sets_for_session(&self.db_pool, workout_id).await?;
        let exercises = self.get_all_exercises().await?;

        Ok(ActiveWorkoutState {
            workout: Arc::new(UniffiWorkoutSession::try_from(workout)?),
            exercises: exercises
                .into_iter()
                .map(|e| Arc::new(UniffiExercise::from(e)))
                .collect(),
            sets: sets
                .into_iter()
                .map(|s| Arc::new(UniffiWorkoutSet::from(s)))
                .collect(),
        })
    }

    /// Get workout suggestions.
    pub async fn get_workout_suggestions(&self) -> Result<Vec<WorkoutSuggestion>> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        let sets = get_sets_for_session(&self.db_pool, session_id).await?;
        let _workout = get_workout_session(&self.db_pool, session_id).await?;

        let mut exercise_counts: HashMap<i64, i64> = HashMap::new();
        for set in &sets {
            *exercise_counts.entry(set.exercise_id).or_insert(0) += 1;
        }

        let all_exercises = self.get_all_exercises().await?;
        let exercise_map: HashMap<i64, String> =
            all_exercises.into_iter().map(|e| (e.id, e.name)).collect();

        let current_exercises: Vec<(String, i64)> = exercise_counts
            .iter()
            .filter_map(|(ex_id, count)| exercise_map.get(ex_id).map(|name| (name.clone(), *count)))
            .collect();

        let mut past_performance_parts = Vec::new();
        for (ex_id, _count) in &exercise_counts {
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

        let known_exercises: Vec<String> = exercise_map.values().cloned().collect();
        let ctx = PromptContext {
            known_exercises,
            ..Default::default()
        };
        let builder = PromptBuilder::new(ctx);

        generate_workout_suggestions(
            self.llm_backend.as_ref(),
            &builder,
            &current_exercises,
            &past_performance,
        )
        .await
    }

    /// Get workout summary.
    pub async fn get_workout_summary(&self) -> Result<WorkoutSummary> {
        let session_id = self
            .get_workout_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("No active workout in session"))?;

        let workout = get_workout_session(&self.db_pool, session_id).await?;
        if let Some(cached_summary) = workout.summary {
            if let Ok(summary_json) = serde_json::from_str::<serde_json::Value>(&cached_summary) {
                if let (Some(message), Some(emoji)) = (
                    summary_json.get("message").and_then(|v| v.as_str()),
                    summary_json.get("emoji").and_then(|v| v.as_str()),
                ) {
                    return Ok(WorkoutSummary {
                        message: message.to_string(),
                        emoji: emoji.to_string(),
                    });
                }
            }
        }

        let sets = get_sets_for_session(&self.db_pool, session_id).await?;

        let mut exercise_counts: HashMap<i64, i64> = HashMap::new();
        for set in &sets {
            *exercise_counts.entry(set.exercise_id).or_insert(0) += 1;
        }

        let all_exercises = self.get_all_exercises().await?;
        let exercise_map: HashMap<i64, String> =
            all_exercises.into_iter().map(|e| (e.id, e.name)).collect();

        let mut exercise_details = Vec::new();
        for (ex_id, count) in &exercise_counts {
            if let Some(ex_name) = exercise_map.get(ex_id) {
                let exercise_sets: Vec<_> =
                    sets.iter().filter(|s| s.exercise_id == *ex_id).collect();

                if !exercise_sets.is_empty() {
                    let avg_weight = exercise_sets.iter().map(|s| s.weight).sum::<f64>()
                        / exercise_sets.len() as f64;
                    let avg_reps = exercise_sets.iter().map(|s| s.reps).sum::<i64>() as f64
                        / exercise_sets.len() as f64;
                    let avg_rpe = exercise_sets
                        .iter()
                        .filter_map(|s| s.rpe)
                        .collect::<Vec<_>>();
                    let avg_rpe_str = if !avg_rpe.is_empty() {
                        let rpe_avg = avg_rpe.iter().sum::<f64>() / avg_rpe.len() as f64;
                        format!(" @{:.1}RPE", rpe_avg)
                    } else {
                        String::new()
                    };

                    exercise_details.push(format!(
                        "{}: {} sets, avg {:.1}kg x {:.0} reps{}",
                        ex_name, count, avg_weight, avg_reps, avg_rpe_str
                    ));
                } else {
                    exercise_details.push(format!("{}: {} sets", ex_name, count));
                }
            }
        }

        let current_exercises: Vec<(String, i64)> = exercise_counts
            .iter()
            .filter_map(|(ex_id, count)| exercise_map.get(ex_id).map(|name| (name.clone(), *count)))
            .collect();

        if current_exercises.is_empty() {
            return Ok(WorkoutSummary {
                message: "No exercises added yet.".to_string(),
                emoji: "✨".to_string(),
            });
        }

        let known_exercises: Vec<String> = exercise_map.values().cloned().collect();
        let ctx = PromptContext {
            known_exercises,
            ..Default::default()
        };
        let builder = PromptBuilder::new(ctx);

        let detailed_exercises: Vec<(String, i64, String)> = exercise_counts
            .iter()
            .filter_map(|(ex_id, count)| {
                exercise_map.get(ex_id).map(|name| {
                    let detail = exercise_details
                        .iter()
                        .find(|d| d.starts_with(name))
                        .map(|d| d.clone())
                        .unwrap_or_else(|| format!("{}: {} sets", name, count));
                    (name.clone(), *count, detail)
                })
            })
            .collect();

        let summary = generate_workout_summary(
            self.llm_backend.as_ref(),
            &builder,
            &current_exercises,
            &detailed_exercises,
        )
        .await?;

        let summary_json = serde_json::json!({
            "message": summary.message.trim(),
            "emoji": summary.emoji.trim()
        });
        update_workout_summary(&self.db_pool, session_id, summary_json.to_string()).await?;

        Ok(summary)
    }
}
