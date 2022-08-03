use crate::loki::tracing::level_filters::LevelFilter;
use loki::tracing::dispatcher::DefaultGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// Create a subscriber to collect all logs
// that are created **in all threads**.
// Warning : this function will panic if called twice in the same program
// https://docs.rs/tracing/latest/tracing/dispatcher/index.html
pub fn init_logger() {
    let default_level = LevelFilter::INFO;
    let rust_log =
        std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_| default_level.to_string());
    let env_filter_subscriber = EnvFilter::try_new(rust_log).unwrap_or_else(|err| {
        eprintln!(
            "invalid {}, falling back to level '{}' - {}",
            EnvFilter::DEFAULT_ENV,
            default_level,
            err,
        );
        EnvFilter::new(default_level.to_string())
    });
    let format = tracing_subscriber::fmt::format()
        .with_thread_ids(false) // set to true to display id of the thread emitting the log
        .with_source_location(true) // set to true to include source file and line number in log
        .with_target(false) // set to true to include module name in logs
        .with_ansi(true) // set to false to remove color in output
        .compact();
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().event_format(format))
        .with(env_filter_subscriber);
    loki::tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber.");
}

#[must_use]
// Create a subscriber to collect all logs
// that are created **in the current thread** while DefaultGuard is alive
// https://docs.rs/tracing/latest/tracing/dispatcher/index.html
//
// This logger support libtest's output capturing
// https://docs.rs/tracing-subscriber/0.3.3/tracing_subscriber/fmt/struct.Layer.html#method.with_test_writer
pub fn init_test_logger() -> DefaultGuard {
    let default_level = LevelFilter::DEBUG;
    let rust_log =
        std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_| default_level.to_string());
    let env_filter_subscriber = EnvFilter::try_new(rust_log).unwrap_or_else(|err| {
        eprintln!(
            "invalid {}, falling back to level '{}' - {}",
            EnvFilter::DEFAULT_ENV,
            default_level,
            err,
        );
        EnvFilter::new(default_level.to_string())
    });
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .with(env_filter_subscriber)
        .set_default()
}

// Create a subscriber to collect all logs
// that are created **in all threads**.
// Warning : this function will panic if called twice in the same program
// https://docs.rs/tracing/latest/tracing/dispatcher/index.html
//
// This logger support libtest's output capturing
// https://docs.rs/tracing-subscriber/0.3.3/tracing_subscriber/fmt/struct.Layer.html#method.with_test_writer
pub fn init_global_test_logger() {
    let default_level = LevelFilter::DEBUG;
    let rust_log =
        std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_| default_level.to_string());
    let env_filter_subscriber = EnvFilter::try_new(rust_log).unwrap_or_else(|err| {
        eprintln!(
            "invalid {}, falling back to level '{}' - {}",
            EnvFilter::DEFAULT_ENV,
            default_level,
            err,
        );
        EnvFilter::new(default_level.to_string())
    });
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .with(env_filter_subscriber);
    loki::tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber.");
}
