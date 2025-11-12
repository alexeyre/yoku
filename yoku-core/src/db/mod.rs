pub mod graph;
pub mod models;
pub mod operations;
pub mod schema;

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
