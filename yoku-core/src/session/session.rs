use crate::db;
use crate::llm::LlmInterface;
use crate::recommendation::GraphManager;
use crate::recommendation::RecommendationEngine;
use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(uniffi::Object)]
pub struct Session {
    pub workout_id: Mutex<Option<i64>>,
    pub db_pool: SqlitePool,
    pub llm_backend: Arc<LlmInterface>,
    pub recommendation_engine: RecommendationEngine,
}

const fn get_openai_api_key() -> &'static str {
    dotenv!("OPENAI_KEY")
}

impl Session {
    pub async fn new(db_path: &str, model: String, graph_path: &str) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create DB pool: {}", e))?;

        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&pool)
            .await?;

        db::init_database(&pool).await?;

        let llm_backend = Arc::new(
            LlmInterface::new_openai(Some(get_openai_api_key().to_string()), Some(model)).await?,
        );

        let recommendation_engine = RecommendationEngine::new(GraphManager::new(graph_path).await?);

        Ok(Self {
            workout_id: Mutex::new(None),
            db_pool: pool,
            llm_backend,
            recommendation_engine,
        })
    }

    pub async fn get_workout_id(&self) -> Option<i64> {
        self.workout_id.lock().await.clone()
    }
}
