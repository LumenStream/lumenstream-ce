//! Centralized logging management for LumenStream.
//!
//! Provides initialization, runtime level changes, and log rotation support.

use std::io;

use ls_config::LogConfig;
use parking_lot::RwLock;
use thiserror::Error;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, Registry, fmt,
    layer::SubscriberExt,
    reload::{self, Handle},
    util::SubscriberInitExt,
};

#[derive(Debug, Error)]
pub enum LogError {
    #[error("invalid log level: {0}")]
    InvalidLevel(String),
    #[error("failed to set log level: {0}")]
    ReloadFailed(String),
    #[error("logging already initialized")]
    AlreadyInitialized,
}

type FilterHandle = Handle<EnvFilter, Registry>;

/// Handle for runtime log level management.
///
/// Holds the reload handle for dynamic filter updates and any worker guards
/// that must be kept alive for the duration of the program.
pub struct LogHandle {
    filter_handle: FilterHandle,
    current_level: RwLock<String>,
    // Guards must be kept alive to ensure logs are flushed
    _guards: Vec<WorkerGuard>,
}

impl LogHandle {
    /// Set the log level at runtime.
    ///
    /// Accepts standard level names: trace, debug, info, warn, error
    /// or a full filter directive like "info,ls_api=debug"
    pub fn set_level(&self, level: &str) -> Result<(), LogError> {
        let filter = parse_filter(level)?;
        self.filter_handle
            .reload(filter)
            .map_err(|e| LogError::ReloadFailed(e.to_string()))?;
        *self.current_level.write() = level.to_string();
        Ok(())
    }

    /// Get the current log level setting.
    pub fn get_level(&self) -> String {
        self.current_level.read().clone()
    }
}

fn parse_filter(level: &str) -> Result<EnvFilter, LogError> {
    EnvFilter::try_new(level).map_err(|_| LogError::InvalidLevel(level.to_string()))
}

#[allow(dead_code)]
fn parse_level(level: &str) -> Result<Level, LogError> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(Level::TRACE),
        "debug" => Ok(Level::DEBUG),
        "info" => Ok(Level::INFO),
        "warn" | "warning" => Ok(Level::WARN),
        "error" => Ok(Level::ERROR),
        _ => Err(LogError::InvalidLevel(level.to_string())),
    }
}

/// Initialize logging based on configuration.
///
/// Returns a `LogHandle` that can be used to change log levels at runtime.
/// The handle must be kept alive for the duration of the program.
///
/// # Panics
///
/// Panics if called more than once (tracing subscriber already set).
pub fn init_logging(config: &LogConfig) -> LogHandle {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    let (filter_layer, filter_handle) = reload::Layer::new(filter);

    let mut guards = Vec::new();

    match config.output.as_str() {
        "file" => {
            let (writer, guard) = create_file_writer(config);
            guards.push(guard);
            init_with_writer(
                config,
                filter_layer,
                filter_handle.clone(),
                writer,
                &config.level,
                guards,
            )
        }
        "both" => {
            let (file_writer, file_guard) = create_file_writer(config);
            guards.push(file_guard);
            init_with_both(
                config,
                filter_layer,
                filter_handle.clone(),
                file_writer,
                &config.level,
                guards,
            )
        }
        _ => {
            // Default to stdout
            let (writer, guard) = tracing_appender::non_blocking(io::stdout());
            guards.push(guard);
            init_with_writer(
                config,
                filter_layer,
                filter_handle.clone(),
                writer,
                &config.level,
                guards,
            )
        }
    }
}

fn create_file_writer(
    config: &LogConfig,
) -> (tracing_appender::non_blocking::NonBlocking, WorkerGuard) {
    use std::path::Path;

    let path = Path::new(&config.file_path);
    let dir = path.parent().unwrap_or(Path::new("."));
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("lumenstream.log");

    let file_appender = tracing_appender::rolling::daily(dir, filename);
    tracing_appender::non_blocking(file_appender)
}

fn init_with_writer<W>(
    config: &LogConfig,
    filter_layer: reload::Layer<EnvFilter, Registry>,
    filter_handle: FilterHandle,
    writer: W,
    level: &str,
    guards: Vec<WorkerGuard>,
) -> LogHandle
where
    W: for<'writer> fmt::MakeWriter<'writer> + Send + Sync + 'static,
{
    let registry = Registry::default().with(filter_layer);

    if config.format == "json" {
        let fmt_layer = fmt::layer()
            .json()
            .with_writer(writer)
            .with_target(true)
            .with_level(true)
            .with_thread_ids(false)
            .with_thread_names(false);

        registry.with(fmt_layer).init();
    } else {
        let fmt_layer = fmt::layer()
            .with_writer(writer)
            .with_target(false)
            .with_level(true)
            .with_thread_ids(false)
            .with_thread_names(false);

        registry.with(fmt_layer).init();
    }

    LogHandle {
        filter_handle,
        current_level: RwLock::new(level.to_string()),
        _guards: guards,
    }
}

fn init_with_both<W>(
    config: &LogConfig,
    filter_layer: reload::Layer<EnvFilter, Registry>,
    filter_handle: FilterHandle,
    file_writer: W,
    level: &str,
    mut guards: Vec<WorkerGuard>,
) -> LogHandle
where
    W: for<'writer> fmt::MakeWriter<'writer> + Send + Sync + 'static,
{
    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(io::stdout());
    guards.push(stdout_guard);

    let registry = Registry::default().with(filter_layer);

    if config.format == "json" {
        let stdout_layer = fmt::layer()
            .json()
            .with_writer(stdout_writer)
            .with_target(true)
            .with_level(true);

        let file_layer = fmt::layer()
            .json()
            .with_writer(file_writer)
            .with_target(true)
            .with_level(true);

        registry.with(stdout_layer).with(file_layer).init();
    } else {
        let stdout_layer = fmt::layer()
            .with_writer(stdout_writer)
            .with_target(false)
            .with_level(true);

        let file_layer = fmt::layer()
            .with_writer(file_writer)
            .with_target(false)
            .with_level(true);

        registry.with(stdout_layer).with(file_layer).init();
    }

    LogHandle {
        filter_handle,
        current_level: RwLock::new(level.to_string()),
        _guards: guards,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_level_valid() {
        assert_eq!(parse_level("trace").unwrap(), Level::TRACE);
        assert_eq!(parse_level("DEBUG").unwrap(), Level::DEBUG);
        assert_eq!(parse_level("Info").unwrap(), Level::INFO);
        assert_eq!(parse_level("WARN").unwrap(), Level::WARN);
        assert_eq!(parse_level("warning").unwrap(), Level::WARN);
        assert_eq!(parse_level("error").unwrap(), Level::ERROR);
    }

    #[test]
    fn test_parse_level_invalid() {
        assert!(parse_level("invalid").is_err());
        assert!(parse_level("").is_err());
    }

    #[test]
    fn test_parse_filter_valid() {
        assert!(parse_filter("info").is_ok());
        assert!(parse_filter("debug,ls_api=trace").is_ok());
        assert!(parse_filter("warn,ls_infra=debug,actix_web=info").is_ok());
    }

    #[test]
    fn test_parse_filter_invalid() {
        // Invalid filter syntax
        assert!(parse_filter("not_a_level[invalid").is_err());
    }

    #[test]
    fn test_log_config_default_values() {
        let config = LogConfig::default();
        // Verify we can create a filter from default level
        assert!(parse_filter(&config.level).is_ok());
    }
}
