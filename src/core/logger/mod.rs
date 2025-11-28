use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt};
use std::io::{self, Write};

pub static VERBOSE_MODE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
pub static HUMAN_READABLE_MODE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub mod error {
    use tracing::subscriber::SetGlobalDefaultError;
    use tracing_subscriber::filter::ParseError;

    #[derive(Debug, thiserror::Error)]
    pub enum Logger {
        #[error(transparent)]
        SetGlobalDefaultError(#[from] SetGlobalDefaultError),

        #[error(transparent)]
        ParseError(#[from] ParseError),
    }
}

pub fn init_logger() -> Result<(), error::Logger> {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::OFF.into())
        .from_env()
        .expect("Logger : Failed to build EnvFilter")
        .add_directive(
            format!(
                "{}={}",
                env!("CARGO_PKG_NAME"),
                if *VERBOSE_MODE.get().unwrap_or(&false) {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                }
            )
            .parse()?,
        );

    if *HUMAN_READABLE_MODE.get().unwrap_or(&false) {
        tracing_subscriber::FmtSubscriber::builder()
            .pretty()
            .with_env_filter(filter)
            .with_writer(io::stdout)
            .with_target(true)
            .with_level(true)
            .init();
    } else {
        
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_target(true)
            .with_level(true)
            .init();
    };

    Ok(())
}
