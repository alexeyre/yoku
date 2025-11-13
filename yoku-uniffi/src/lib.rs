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
