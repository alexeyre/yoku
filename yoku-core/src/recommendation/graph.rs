use anyhow::Result;
use indradb::{Database, RocksdbDatastore};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GraphManager {
    db: Arc<RwLock<Database<RocksdbDatastore>>>,
}

impl GraphManager {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db: Database<RocksdbDatastore> = RocksdbDatastore::new_db(db_path)?;
        Ok(Self {
            db: Arc::new(RwLock::new(db)),
        })
    }
}
