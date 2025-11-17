use log::LevelFilter;
use std::io::Write;

fn init_logger(level: LevelFilter) {
    let mut builder = env_logger::Builder::new();
    builder
        .format(move |buf, record| {
            writeln!(
                buf,
                "{}: {} - {}",
                record.level(),
                record.target(),
                record.args()
            )
        })
        .target(env_logger::Target::Stdout)
        .filter_level(level);

    let _ = builder.try_init();

    log::set_max_level(level);
}

#[uniffi::export]
pub fn set_debug_log_level() {
    init_logger(LevelFilter::Trace);
}

#[uniffi::export]
pub fn set_log_level(level: &str) -> bool {
    let lvl = match level.to_lowercase().as_str() {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" | "warning" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => return false,
    };

    init_logger(lvl);
    true
}
