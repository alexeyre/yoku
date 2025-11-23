use super::GraphManager;
use crate::db::models::*;
use anyhow::Result;
pub struct RecommendationEngine {
    pub graph_manager: GraphManager,
}

impl RecommendationEngine {
    pub fn new(graph_manager: GraphManager) -> Self {
        RecommendationEngine { graph_manager }
    }

    pub async fn recommend_exercises(
        &self,
        user_id: &str,
        workout_session_id: i64,
    ) -> Result<Vec<Exercise>> {
        Ok(vec![])
    }

    pub async fn recommend_set(
        &self,
        user_id: &str,
        workout_session_id: i64,
        exercise_id: i64,
    ) -> Result<Vec<(Exercise, i64, f64)>> {
        Ok(vec![])
    }
}
