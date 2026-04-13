use log::LevelFilter;

/// Initialize logging using env_logger.
/// By default, this reads the RUST_LOG environment variable for filtering.
pub fn init_logging() {
    env_logger::Builder::from_default_env()
        .filter(None, LevelFilter::Info)
        .init();
}
