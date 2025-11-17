pub mod models;
pub mod operations;

use anyhow::Result;
use sqlx::SqlitePool;
use std::env;
use tokio::sync::OnceCell;

static DB_PATH: OnceCell<String> = OnceCell::const_new();

#[inline(always)]
pub async fn get_db_path() -> &'static String {
    DB_PATH
        .get_or_init(async || {
            env::var("DATABASE_URL")
                .expect("DATABASE_URL must be specified or present in the environment")
        })
        .await
}

pub async fn drop_all_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM workout_sets").execute(pool).await?;
    sqlx::query("DELETE FROM workout_sessions").execute(pool).await?;
    sqlx::query("DELETE FROM request_strings").execute(pool).await?;
    sqlx::query("DELETE FROM users").execute(pool).await?;
    sqlx::query("DELETE FROM exercise_muscles").execute(pool).await?;
    sqlx::query("DELETE FROM exercises").execute(pool).await?;
    sqlx::query("DELETE FROM muscles").execute(pool).await?;
    Ok(())
}

pub async fn set_db_path(path: &str) -> Result<()> {
    DB_PATH
        .set(path.to_string())
        .map_err(|e| anyhow::anyhow!(format!("Failed to set DB_PATH: {:?}", e)))
}

const MIGRATION_2025_11_11_220309_0000_SETUP_TABLES: &str = include_str!("../../../migrations/2025-11-11-220309-0000_setup_tables/up.sql");

fn parse_sql_statements(sql: &str) -> Vec<String> {
    sql.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("--")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub async fn init_database(pool: &SqlitePool) -> Result<()> {
    let migrations: Vec<&str> = vec![
        MIGRATION_2025_11_11_220309_0000_SETUP_TABLES,
    ];
    
    for sql in migrations {
        let statements = parse_sql_statements(sql);
        
        for statement in statements {
            if !statement.trim().is_empty() {
                sqlx::query(&statement).execute(pool).await
                    .map_err(|e| anyhow::anyhow!("Failed to execute migration statement: {} - Error: {}", statement, e))?;
            }
        }
    }
    
    Ok(())
}
