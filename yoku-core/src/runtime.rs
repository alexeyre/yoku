use log::debug;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static GLOBAL_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn build_runtime() -> Runtime {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    debug!("Initializing global runtime with {} threads", threads);
    let threads = std::cmp::max(threads, 2);
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(threads)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
}

pub fn init_global_runtime_blocking() -> &'static Runtime {
    GLOBAL_RUNTIME.get_or_init(|| build_runtime())
}

pub async fn init_global_runtime() -> &'static Runtime {
    init_global_runtime_blocking()
}
