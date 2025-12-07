use std::io::{self, Write};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt};

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

pub fn init_logger(verbose: bool, readable_log: bool) -> Result<(), error::Logger> {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::OFF.into())
        .from_env()
        .expect("Logger : Failed to build EnvFilter")
        .add_directive(
            format!(
                "{}={}",
                env!("CARGO_PKG_NAME"),
                if verbose {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                }
            )
            .parse()?,
        );

    if readable_log {
        tracing_subscriber::FmtSubscriber::builder()
            .pretty()
            .with_env_filter(filter)
            .with_writer(std::io::stdout)
            .with_target(false)
            .without_time()
            .with_level(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(std::io::stdout)
            .with_target(true)
            .with_level(true)
            .init();
    };

    Ok(())
}
