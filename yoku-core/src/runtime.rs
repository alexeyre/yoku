use log::debug;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

static GLOBAL_RUNTIME: OnceCell<Runtime> = OnceCell::const_new();
pub async fn init_global_runtime() -> &'static Runtime {
    GLOBAL_RUNTIME
        .get_or_init(async || {
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
        })
        .await
}
