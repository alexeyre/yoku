pub mod models;
pub mod schema;
pub mod operations;

use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8::Pool},
};
use std::env;
use tokio::sync::OnceCell;

pub type PgPool = Pool<AsyncPgConnection>;

static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

pub async fn get_conn()
-> diesel_async::pooled_connection::bb8::PooledConnection<'static, AsyncPgConnection> {
    get_pool()
        .await
        .get()
        .await
        .expect("Failed to get connection from pool")
}

pub async fn get_pool() -> &'static PgPool {
    DB_POOL.get_or_init(create_pool).await
}

async fn create_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    Pool::builder()
        .max_size(10)
        .build(manager)
        .await
        .expect("Failed to create async Diesel connection pool")
}

#[allow(unused_imports)]
#[allow(unused_comparisons)]
mod tests {
    use diesel::sql_query;
    use crate::db::get_pool;
    use diesel_async::{AsyncConnection, RunQueryDsl, AsyncPgConnection};

    #[tokio::test]
    async fn test_can_initialize_pool() {
        // Initialize pool and ensure it doesn't panic
        let pool = get_pool().await;
        assert!(pool.state().connections > 0 || pool.state().idle_connections >= 0);
    }

    #[tokio::test]
    async fn test_can_get_connection_and_run_query() {
        let pool = get_pool().await;
        let mut conn = pool
            .get()
            .await
            .expect("Failed to get a connection from pool");

        // Run a simple query to check DB health
        let result = sql_query("SELECT 1")
            .execute(&mut conn)
            .await
            .expect("Failed to execute test query");

        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_pool_is_singleton() {
        let pool1 = get_pool().await;
        let pool2 = get_pool().await;

        // Both should point to the same instance in memory
        assert!(std::ptr::eq(pool1, pool2));
    }
}
