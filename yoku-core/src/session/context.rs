use crate::db::models::WorkoutSet;
use crate::db::operations::{get_exercise_entries, get_sets_for_session, get_workout_session};
use crate::session::Session;
use anyhow::Result;
use std::collections::HashMap;

impl Session {
    pub async fn build_workout_context_string(&self) -> Result<String> {
        let workout_id = self.get_workout_id().await;
        let Some(workout_id) = workout_id else {
            return Ok("No active workout session.".to_string());
        };

        let workout = get_workout_session(&self.db_pool, workout_id).await?;
        let sets = get_sets_for_session(&self.db_pool, workout_id).await?;
        let exercises = self.get_all_exercises().await?;
        let exercise_map: HashMap<i64, String> =
            exercises.iter().map(|e| (e.id, e.name.clone())).collect();

        let mut sorted_sets = sets.clone();
        sorted_sets.sort_by_key(|s| std::cmp::Reverse(s.created_at));

        let mut context = String::new();

        context.push_str(&format!(
            "Current Workout: ID={}, Name={:?}\n",
            workout.id, workout.name
        ));

        if let Some(summary_json) = &workout.summary {
            if let Ok(summary_value) = serde_json::from_str::<serde_json::Value>(summary_json) {
                let message = summary_value
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let emoji = summary_value
                    .get("emoji")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                context.push_str(&format!(
                    "Cached Summary → message: \"{}\" | emoji: {}\n",
                    message, emoji
                ));
            } else {
                context.push_str("Cached Summary → (invalid JSON)\n");
            }
        } else {
            context.push_str("Cached Summary → (none)\n");
        }
        context.push_str("\n");

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

    pub fn resolve_set_id_from_description(
        &self,
        description: &str,
        sets: &[WorkoutSet],
        exercise_map: &HashMap<i64, String>,
    ) -> Option<i64> {
        let desc_lower = description.to_lowercase();

        let mut sorted_sets = sets.to_vec();
        sorted_sets.sort_by_key(|s| std::cmp::Reverse(s.created_at));

        if desc_lower.contains("most recent") || desc_lower.contains("last") || desc_lower == "that"
        {
            return sorted_sets.first().map(|s| s.id);
        }

        if desc_lower.contains("second to last") || desc_lower.contains("second last") {
            return sorted_sets.get(1).map(|s| s.id);
        }

        for exercise_name in exercise_map.values() {
            let ex_lower = exercise_name.to_lowercase();
            if desc_lower.contains(&ex_lower) {
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
}
