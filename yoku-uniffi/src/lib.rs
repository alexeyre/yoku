// yoku/yoku-uniffi/src/lib.rs
// NOTE: This file provides uniffi-exported stubs / TODO skeletons to match
// the iOS app's expectations. Implementations are intentionally lightweight
// placeholders so the Swift UI can call these APIs during UI development.
//
// TODO: Replace stubs with real implementations backed by `yoku_core` and
// your real database/session logic as needed.

uniffi::setup_scaffolding!();
use anyhow::Result;
use tokio::sync::Mutex;
use yoku_core::*;

/// Existing exported functions used by the app's startup flow and log polling.
/// Keep these as-is (small adaptions allowed).

#[uniffi::export]
pub async fn setup_database(path: &str) {
    db::set_db_path(path).await.unwrap();
    db::init_database().await.unwrap();
}

pub struct Session {}
