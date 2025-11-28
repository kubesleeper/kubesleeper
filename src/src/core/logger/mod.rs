use tracing::level_filters::LevelFilter;
use tracing_subscriber::{ fmt, layer::SubscriberExt};

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
        let stdout_layer = fmt::layer()
            .pretty()
            .with_target(false)
            .with_ansi(true)
            .without_time()
            .with_writer(std::io::stderr);

        let subscriber = tracing_subscriber::Registry::default()
            .with(filter)
            .with(stdout_layer);
        tracing::subscriber::set_global_default(subscriber)
    } else {
        let stdout_layer = fmt::layer()
            .compact()
            .with_target(false)
            .with_ansi(true)
            .with_writer(std::io::stderr);

        let file_json_layer = fmt::layer()
            .json()
            .with_span_list(false)
            .with_writer(std::io::stdout);

        let subscriber = tracing_subscriber::Registry::default()
            .with(filter)
            .with(stdout_layer)
            .with(file_json_layer);
        tracing::subscriber::set_global_default(subscriber)
    }?;

    Ok(())
}
