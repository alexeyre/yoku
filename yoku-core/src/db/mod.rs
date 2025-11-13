pub mod models;
pub mod operations;
pub mod schema;

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrations");

use anyhow::Result;
use diesel::SqliteConnection;
use diesel::prelude::*;
use std::env;
use std::fs;
use std::path::Path;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;

static DB_PATH: OnceCell<String> = OnceCell::const_new();
static DB_CONN: OnceCell<Mutex<SqliteConnection>> = OnceCell::const_new();

#[inline(always)]
pub async fn get_db_path() -> &'static String {
    DB_PATH
        .get_or_init(async || {
            env::var("DATABASE_URL")
                .expect("DATABASE_URL must be specified or present in the environment")
        })
        .await
}

pub async fn set_db_path(path: &str) -> Result<()> {
    DB_PATH
        .set(path.to_string())
        .map_err(|e| anyhow::anyhow!(format!("Failed to set DB_PATH: {:?}", e)))
}

pub async fn get_conn() -> &'static Mutex<SqliteConnection> {
    DB_CONN
        .get_or_init(|| async {
            let database_url = get_db_path().await;
            let conn = SqliteConnection::establish(&database_url).unwrap();
            Mutex::new(conn)
        })
        .await
}

pub fn is_database_initialized() -> bool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = SqliteConnection::establish(&database_url);
    conn.is_ok()
}

pub async fn init_database() -> Result<()> {
    let db_path = get_db_path().await;

    // Make sure parent dir exists
    if let Some(parent) = Path::new(db_path).parent() {
        fs::create_dir_all(parent)?;
    }

    // Try to connect (creates file if missing)
    let mut conn = SqliteConnection::establish(db_path)
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {e}"))?;

    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow::anyhow!("Migration failed: {e}"))?;

    Ok(())
}
