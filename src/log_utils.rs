use std::error::Error;
use tokio::fs;
use tracing::error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

pub async fn init_tracing(log_path: &str) -> Result<WorkerGuard, Box<dyn Error>> {
    // Ensure logs directory exists
    fs::create_dir_all(log_path)
        .await
        .unwrap_or_else(|_| {
            error!("Failed to create logs directory");
        });

    // Set up the environment filter using the RUST_LOG environment variable
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    // Configure file appender
    let file_appender = RollingFileAppender::new(Rotation::DAILY, format!("{}", log_path), "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Console logger
    let console_layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_writer(std::io::stdout);

    // File logger
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_writer(non_blocking);

    // Combine the layers into a subscriber with the EnvFilter
    let subscriber = Registry::default()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer);

    // Initialize the subscriber
    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global tracing subscriber");

    Ok(_guard)
}
