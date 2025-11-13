pub mod models;
pub mod operations;
pub mod schema;

use diesel::SqliteConnection;
use diesel::prelude::*;
use std::env;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;

static DB_CONN: OnceCell<Mutex<SqliteConnection>> = OnceCell::const_new();

pub async fn get_conn() -> &'static Mutex<SqliteConnection> {
    DB_CONN
        .get_or_init(|| async {
            let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            let conn = SqliteConnection::establish(&database_url).unwrap();
            Mutex::new(conn)
        })
        .await
}
