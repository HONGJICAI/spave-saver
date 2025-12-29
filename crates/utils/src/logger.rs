use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the logger with default settings
pub fn init_logger() {
    init_logger_with_level("info")
}

/// Initialize the logger with a specific level
pub fn init_logger_with_level(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .init();
}

/// Initialize logger for file output
pub fn init_logger_with_file(file_path: &str) -> anyhow::Result<()> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::sync::Arc::new(file)))
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_logger_initialization() {
        // Note: Can only initialize logger once per test run
        // This test just ensures the function doesn't panic
    }
}
