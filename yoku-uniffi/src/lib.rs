uniffi::setup_scaffolding!();

use yoku_core::*;

#[uniffi::export]
pub fn hello() -> String {
    "Hello from uniffi 123, World!".to_string()
}

#[uniffi::export]
pub async fn setup_database(path: &str) {
    db::set_db_path(path).await.unwrap();
    db::init_database().await.unwrap();
}

#[uniffi::export]
pub async fn create_user(username: String) -> i32 {
    db::operations::get_or_create_user(&username)
        .await
        .unwrap()
        .id
}

#[uniffi::export]
pub async fn get_workout_session(id: i32) -> WorkoutSession {
    WorkoutSession {
        id,
        name: "hello workout".to_string(),
    }
}

#[derive(uniffi::Object)]
pub struct WorkoutSession {
    id: i32,
    pub name: String,
}
#[uniffi::export]
impl WorkoutSession {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn id(&self) -> i32 {
        self.id
    }
}
