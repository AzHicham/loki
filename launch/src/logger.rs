use loki::tracing::dispatcher::DefaultGuard;
use std::str::FromStr;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

// Create a subscriber to collect all logs
// that are created **in all threads**.
// Warning : this function will panic if called twice in the same program
// https://docs.rs/tracing/latest/tracing/dispatcher/index.html
pub fn init_logger() {
    // This will enable all logs from loki* crates at INFO level, and deactivate all logs from other dependecies.
    // See https://docs.rs/tracing-subscriber/0.3.16/tracing_subscriber/filter/struct.EnvFilter.html#directives
    // for more details on how to configure log filtering
    let default_level = "loki=info";
    let env_filter = filter(default_level);

    let ansi_color = ansi_color(false);

    let format = tracing_subscriber::fmt::format()
        .with_thread_ids(false) // set to true to display id of the thread emitting the log
        .with_source_location(true) // set to true to include source file and line number in log
        .with_target(false) // set to true to include module name in logs
        .with_ansi(ansi_color) // set to false to remove color in output
        .compact();
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().event_format(format))
        .with(env_filter);
    loki::tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber.");
}

#[must_use]
pub fn subscriber_for_tests() -> impl SubscriberExt {
    // This will enable all logs from loki* crates at DEBUG level, and deactivate all logs from other dependecies.
    // See https://docs.rs/tracing-subscriber/0.3.16/tracing_subscriber/filter/struct.EnvFilter.html#directives
    // for more details on how to configure log filtering
    let default_level = "loki=debug";
    let env_filter = filter(default_level);

    let ansi_color = ansi_color(true);

    let format = tracing_subscriber::fmt::format()
        .with_thread_ids(false) // set to true to display id of the thread emitting the log
        .with_source_location(true) // set to true to include source file and line number in log
        .with_target(false) // set to true to include module name in logs
        .with_ansi(ansi_color) // set to false to remove color in output
        .compact();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .event_format(format)
                .with_test_writer(),
        )
        .with(env_filter)
}

#[must_use]
// Create a subscriber to collect all logs
// that are created **in the current thread** while DefaultGuard is alive
// https://docs.rs/tracing/latest/tracing/dispatcher/index.html
//
// This logger support libtest's output capturing
// https://docs.rs/tracing-subscriber/0.3.3/tracing_subscriber/fmt/struct.Layer.html#method.with_test_writer
pub fn init_test_logger() -> DefaultGuard {
    let subscriber = subscriber_for_tests();
    loki::tracing::subscriber::set_default(subscriber)
}

// Create a subscriber to collect all logs
// that are created **in all threads**.
// Warning : this function will panic if called twice in the same program
// https://docs.rs/tracing/latest/tracing/dispatcher/index.html
//
// This logger support libtest's output capturing
// https://docs.rs/tracing-subscriber/0.3.3/tracing_subscriber/fmt/struct.Layer.html#method.with_test_writer
pub fn init_global_test_logger() {
    let subscriber = subscriber_for_tests();
    loki::tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber.");
}

pub fn filter(default_level: &str) -> EnvFilter {
    let rust_log =
        std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_| default_level.to_string());
    EnvFilter::try_new(rust_log).unwrap_or_else(|err| {
        eprintln!(
            "invalid {}, falling back to filter '{}' - {}",
            EnvFilter::DEFAULT_ENV,
            default_level,
            err,
        );
        EnvFilter::new(default_level)
    })
}

pub fn ansi_color(default: bool) -> bool {
    let ansi_color_string = std::env::var("LOKI_LOG_COLOR").unwrap_or_default();
    bool::from_str(&ansi_color_string).unwrap_or(default)
}
