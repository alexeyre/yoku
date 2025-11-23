pub mod models;
pub mod operations;

use anyhow::Result;
use log::{debug, info};
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
    sqlx::query("DELETE FROM workout_sets")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM workout_sessions")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM request_strings")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM users").execute(pool).await?;
    sqlx::query("DELETE FROM exercise_muscles")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM exercises").execute(pool).await?;
    sqlx::query("DELETE FROM muscles").execute(pool).await?;
    Ok(())
}

pub async fn set_db_path(path: &str) -> Result<()> {
    DB_PATH
        .set(path.to_string())
        .map_err(|e| anyhow::anyhow!(format!("Failed to set DB_PATH: {:?}", e)))
}

struct Migration {
    name: &'static str,
    up_sql: &'static str,
}

const MIGRATION_2025_11_11_220309_0000_SETUP_TABLES: &str =
    include_str!("../../../migrations/2025-11-11-220309-0000_setup_tables/up.sql");

const MIGRATIONS: &[Migration] = &[Migration {
    name: "2025-11-11-220309-0000_setup_tables",
    up_sql: MIGRATION_2025_11_11_220309_0000_SETUP_TABLES,
}];

async fn init_migrations_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER NOT NULL PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            applied_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn is_migration_applied(pool: &SqlitePool, migration_name: &str) -> Result<bool> {
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM _migrations WHERE name = ?1",
    )
    .bind(migration_name)
    .fetch_one(pool)
    .await?;
    Ok(result > 0)
}

async fn mark_migration_applied(pool: &SqlitePool, migration_name: &str) -> Result<()> {
    sqlx::query("INSERT INTO _migrations (name) VALUES (?1)")
        .bind(migration_name)
        .execute(pool)
        .await?;
    Ok(())
}

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
    init_migrations_table(pool).await?;

    for migration in MIGRATIONS {
        if is_migration_applied(pool, migration.name).await? {
            debug!("Migration {} already applied, skipping", migration.name);
            continue;
        }

        info!("Applying migration: {}", migration.name);
        let statements = parse_sql_statements(migration.up_sql);

        for statement in statements {
            if !statement.trim().is_empty() {
                sqlx::query(&statement).execute(pool).await.map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to execute migration statement in {}: {} - Error: {}",
                        migration.name,
                        statement,
                        e
                    )
                })?;
            }
        }

        mark_migration_applied(pool, migration.name).await?;
        info!("Migration {} applied successfully", migration.name);
    }

    Ok(())
}
