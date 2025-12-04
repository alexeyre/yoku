#![allow(dead_code)]
use super::GraphManager;
use crate::db::models::*;
use crate::db::operations::get_all_exercises_except;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub struct RecommendationEngine<T: indradb::Datastore> {
    graph_manager: GraphManager<T>,
    db_pool: sqlx::SqlitePool,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Joint {
    LeftKnee,
}

#[derive(Debug, Clone)]
pub struct PlannedSet {
    pub exercise_id: i64,
    pub exercise_name: String,
    pub movement_pattern: ExercisePatternType,
    pub set_number: usize, // 1.. for this exercise
    pub reps: i64,
    pub weight: f64,
    pub intended_rpe: f64,
    pub rest_seconds: i64,

    pub systemic_fatigue: f64,
    pub joint_fatigue: HashMap<Joint, f64>,
}

#[derive(Debug, Clone)]
pub struct WorkoutPlan {
    pub sets: Vec<PlannedSet>,
}

#[derive(Clone)]
struct ExerciseInProgress {
    pub exercise_id: i64,
    pub exercise_name: String,
    pub movement_pattern: ExercisePatternType,
    pub equipment: Vec<i64>,
    pub muscles: Vec<(i64, f64)>,
    pub joints: Vec<Joint>,
    pub sets_completed: usize,
    pub base_weight: f64,
}

#[derive(Clone)]
struct BeamState {
    pub sets: Vec<PlannedSet>,
    pub current_exercise: Option<ExerciseInProgress>,
    pub completed_exercises: HashSet<i64>,
    pub muscle_accumulator: HashMap<i64, f64>,
    pub total_systemic_fatigue: f64,
    pub total_joint_fatigue: HashMap<Joint, f64>,
    pub total_duration: f64,
    pub score: f64,
}

impl<T: indradb::Datastore> RecommendationEngine<T> {
    pub fn new(graph_manager: GraphManager<T>, db_pool: sqlx::SqlitePool) -> Self {
        RecommendationEngine {
            graph_manager,
            db_pool,
        }
    }

    pub fn expand_muscle_groups(&self, group_proportions: &[(&str, f64)]) -> Vec<(i64, f64)> {
        let mut result: HashMap<i64, f64> = HashMap::new();

        for (group_name, proportion) in group_proportions {
            let group_vertex = match self.graph_manager.get_muscle_group_by_name(group_name) {
                Ok(v) => v,
                Err(_) => {
                    if let Ok(muscle) = self.graph_manager.get_muscle_by_name(group_name) {
                        if let Ok(db_ids) = self.graph_manager.get_muscle_db_ids_in_group(muscle.id)
                        {
                            if let Some(&db_id) = db_ids.first() {
                                *result.entry(db_id as i64).or_insert(0.0) += proportion;
                            }
                        }
                    }
                    continue;
                }
            };

            let muscle_db_ids = match self
                .graph_manager
                .get_muscle_db_ids_in_group(group_vertex.id)
            {
                Ok(ids) => ids,
                Err(_) => continue,
            };

            if muscle_db_ids.is_empty() {
                continue;
            }

            let per_muscle = proportion / muscle_db_ids.len() as f64;
            for db_id in muscle_db_ids {
                *result.entry(db_id as i64).or_insert(0.0) += per_muscle;
            }
        }

        let total: f64 = result.values().sum();
        if total > 0.0 {
            result.iter().map(|(&k, &v)| (k, v / total)).collect()
        } else {
            result.into_iter().collect()
        }
    }

    #[allow(unused, dead_code)]
    pub async fn plan_workout(
        &self,

        // user's ID to get their history
        user_id: i64,

        // muscle IDs to work in proportions $sum{x} = 1$
        target_muscle_id_proportions: HashMap<i64, f64>,

        // equipment the user has access to
        available_equipment_ids: Vec<i64>,

        // the target duration in minutes
        target_duration_in_minutes: i32,

        // size of the window to consider "recent history"
        history_window_days: i32,

        // maximum number of exercises to include in the plan
        maximum_exercise_count: Option<i32>,

        // exercises to avoid
        avoid_exercises: Vec<i64>,

        // the session style to use, e.g. hypertrophy, strength, etc.
        session_style: SessionStyle,
    ) -> Result<WorkoutPlan> {
        let possible_sql_exercises =
            get_all_exercises_except(&self.db_pool, &avoid_exercises).await?;

        let available_equipment_set: std::collections::HashSet<i64> =
            available_equipment_ids.iter().cloned().collect();

        let possible_exercises = possible_sql_exercises
            .iter()
            .map(|ex| (ex, self.graph_manager.get_exercise_vert(ex).unwrap()))
            .filter(|(ex, exercise_id)| {
                let required_equipment = self
                    .graph_manager
                    .get_required_equipment_db_ids_for_exercise(*exercise_id)
                    .unwrap_or_default();
                required_equipment.is_empty()
                    || required_equipment
                        .iter()
                        .all(|eq_id| available_equipment_set.contains(eq_id))
            })
            .collect::<Vec<_>>();

        let max_sets = 30;

        let initial_state = BeamState {
            sets: vec![],
            current_exercise: None,
            completed_exercises: HashSet::new(),
            muscle_accumulator: HashMap::new(),
            total_systemic_fatigue: 0.0,
            total_joint_fatigue: HashMap::new(),
            total_duration: 0.0,
            score: 0.0,
        };
        let mut beam = vec![initial_state];

        const BEAM_WIDTH: usize = 10;

        for _step in 0..max_sets {
            let mut candidates: Vec<BeamState> = Vec::new();
            for state in &beam {
                match &state.current_exercise {
                    Some(exercise) => {
                        // so we have a previous exercise, we can either
                        // 1. do another set of this exercise
                        // 2. do a different exercise
                    }
                    None => {
                        // no previous exercise, pick one to start
                        for (sql_ex, ex_vert) in &possible_exercises {
                            if !Self::is_exercise_allowed(sql_ex, state) {
                                continue;
                            }
                        }
                    }
                }
            }
            if candidates.is_empty() {
                break;
            }

            candidates.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            beam = candidates.into_iter().take(BEAM_WIDTH).collect();

            // terminate if we've reached the maximum duration
        }
        todo!()
    }

    fn is_exercise_allowed(sql_ex: &Exercise, state: &BeamState) -> bool {
        true
    }

    fn score_transition() -> f64 {
        0.0
    }
}
