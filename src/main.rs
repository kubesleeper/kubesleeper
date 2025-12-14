extern crate rocket;
mod core;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use tokio_cron_scheduler::JobSchedulerError;

use crate::core::config;
use crate::core::state::state::SLEEPINESS_DURATION;
use crate::core::{
    controller::{self, set_kubesleeper_namespace},
    ingress::error::IngressError,
    logger::{self, init_logger},
    server,
    server::error::ServerError,
    state::state::create_schedule,
};
mod msg;
use crate::msg::{Message, error, vvv};

#[derive(Parser)]
#[command(name = "kubesleeper", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short)]
    /// verbose mode for logging
    verbose: bool,

    #[arg(short, long)]
    /// Human readable mode for logging
    readable_log: bool,

    #[arg(long)]
    /// Path to the kubesleeper YAML configuration file
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// start kubesleeper service
    Start,

    #[command(subcommand)]
    /// Execute specific action
    Msg(Message),
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ResourceKind {
    Deploy,
    Service,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    ControllerError(#[from] controller::error::Controller),

    #[error(transparent)]
    MsgError(#[from] error::Msg),

    #[error(transparent)]
    IngressError(#[from] IngressError),

    #[error(transparent)]
    JobSchedulerError(#[from] JobSchedulerError),

    #[error(transparent)]
    ServerError(#[from] ServerError),

    #[error(transparent)]
    LoggerError(#[from] logger::error::Logger),

    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),
}

#[tokio::main]
async fn process() -> Result<(), Error> {
    let cli = Cli::parse();

    //logging setup
    init_logger(cli.verbose, cli.readable_log)?;

    let config = config::parse(cli.config)?;

    match cli.command {
        Commands::Start => {
            SLEEPINESS_DURATION
                .set(config.controller.sleepiness_duration)
                .expect("Failed to set up sleepiness duration");

            set_kubesleeper_namespace().await?;
            create_schedule(config.controller.refresh_interval)
                .await
                .start()
                .await?;
            server::start(config.server.port).await?;
        }
        Commands::Msg(e) => vvv(e, config).await?,
    };
    Ok(())
}

fn main() {
    match process() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error : {}", e);
            process::exit(1);
        }
    }
}
