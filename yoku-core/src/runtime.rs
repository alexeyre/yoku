use log::debug;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

/// A synchronous global runtime initializer usable from non-async contexts (e.g. FFI callers).
/// We keep an async-compatible `init_global_runtime()` wrapper for callers that `.await` it,
/// but the actual initialization is performed synchronously so it does not require an existing
/// Tokio reactor to run.
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

/// Initialize the global runtime from a synchronous context. Returns a reference to the runtime.
pub fn init_global_runtime_blocking() -> &'static Runtime {
    GLOBAL_RUNTIME.get_or_init(|| build_runtime())
}

/// Async-compatible initializer that simply defers to the blocking initializer.
/// Because the body does not await anything, callers can safely `.await` this even when no
/// existing reactor is present.
pub async fn init_global_runtime() -> &'static Runtime {
    init_global_runtime_blocking()
}
