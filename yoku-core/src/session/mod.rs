use crate::db::operations::{
    add_multiple_sets_to_workout, add_workout_set, create_request_string_for_username,
    create_workout_session, get_or_create_exercise, get_sets_for_session, get_workout_session,
    update_workout_set_from_parsed,
};
use crate::llm::{LlmInterface, ParsedSet};
use anyhow::Result;
use diesel::SqliteConnection;
use tokio::sync::Mutex;

#[derive(uniffi::Object)]
pub struct Session {
    pub workout_id: Mutex<Option<i32>>,
    pub db_conn: Mutex<SqliteConnection>,
    pub llm_backend: Mutex<LlmInterface>,
}

const fn get_openai_api_key() -> &'static str {
    dotenv!("OPENAI_KEY")
}

impl Session {
    pub async fn new(db_path: &str, model: String) -> Result<Self> {
        let conn = crate::db::get_conn_from_uri(db_path).await?;
        let llm_backend =
            LlmInterface::new_openai(Some(get_openai_api_key().to_string()), Some(model)).await?;
        Ok(Self {
            workout_id: Mutex::new(None),
            db_conn: Mutex::new(conn),
            llm_backend: Mutex::new(llm_backend),
        })
    }

    pub async fn set_workout_id(&self, workout_id: i32) -> Result<()> {
        let mut db_conn = self.db_conn.lock().await;
        let _ = get_workout_session(&mut *db_conn, workout_id).await?;
        *self.workout_id.lock().await = Some(workout_id);
        Ok(())
    }

    pub async fn new_workout(&self) -> Result<()> {
        let mut db_conn = self.db_conn.lock().await;
        let workout = create_workout_session(&mut *db_conn, None, None, None, None).await?;
        self.set_workout_id(workout.id).await
    }

    pub async fn new_workout_with_name(&self, name: &str) -> Result<()> {
        let mut db_conn = self.db_conn.lock().await;
        let workout =
            create_workout_session(&mut *db_conn, None, Some(name.into()), None, None).await?;
        self.set_workout_id(workout.id).await
    }

    pub async fn get_workout_id(&self) -> Option<i32> {
        self.workout_id.lock().await.clone()
    }

    pub async fn replace_set_from_parsed(&self, set_id: i32, parsed: &ParsedSet) -> Result<()> {
        let mut db_conn = self.db_conn.lock().await;
        update_workout_set_from_parsed(&mut *db_conn, set_id, parsed).await?;
        Ok(())
    }

    pub async fn get_all_sets(&self) -> Result<Vec<crate::db::models::WorkoutSet>> {
        let workout_id = self.get_workout_id().await;
        if let Some(workout_id) = workout_id {
            let mut db_conn = self.db_conn.lock().await;
            let sets = get_sets_for_session(&mut *db_conn, workout_id).await?;
            Ok(sets)
        } else {
            Err(anyhow::anyhow!("No active workout"))
        }
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

        let mut db_conn = self.db_conn.lock().await;
        let exercise = get_or_create_exercise(&mut *db_conn, &parsed.exercise).await?;

        let weight = parsed.weight.unwrap_or(0.0);
        let reps = parsed.reps.unwrap_or(0);
        let set_count = parsed.set_count.unwrap_or(1).max(1);

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

        let req =
            create_request_string_for_username(&mut *db_conn, "cli", request_str_content).await?;
        let request_string_id = req.id;

        if set_count > 1 {
            add_multiple_sets_to_workout(
                &mut *db_conn,
                &session_id,
                &exercise.id,
                &request_string_id,
                &weight,
                &reps,
                parsed.rpe,
                set_count,
            )
            .await?;
        } else {
            add_workout_set(
                &mut *db_conn,
                &session_id,
                &exercise.id,
                &request_string_id,
                &weight,
                &reps,
                parsed.rpe,
            )
            .await?;
        }

        Ok(())
    }
}
