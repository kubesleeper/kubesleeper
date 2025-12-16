extern crate rocket;
mod core;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use tokio_cron_scheduler::JobSchedulerError;

use crate::core::config;
use crate::core::ingress::IngressType;
use crate::core::resource::TargetResource;
use crate::core::resource::deploy::Deploy;
use crate::core::resource::service::Service;
use crate::core::state::state::SLEEPINESS_DURATION;
use crate::core::state::state_kind::StateKind;
use crate::core::{
    ingress::error::IngressError,
    logger::{self, init_logger},
    resource, server,
    server::error::ServerError,
    state::state::create_schedule,
};

mod msg;
use crate::msg::{Message, error};

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

    /// Describe k8s status with
    Status,

    #[command(subcommand)]
    /// Execute specific action
    Msg(Message),
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ResourceKind {
    Deploy,
    Service,
}

#[derive(Subcommand)]
enum Manual {
    /// Set a specific Deployment or Service to the desired state
    SetDeploy {
        /// the kube resournce id (like {namespace}/{name}) to target,
        /// namespace 'default' will be used if id is simply {name}
        #[arg(value_name("NAMESPACE/NAME"))]
        resource_id: String,

        /// The target state to which the resource will be set
        state: StateKind,
    },

    /// Set a specific Deployment or Service to the desired state
    SetService {
        /// the kube resournce id like {namespace}/{name},
        /// namespace 'default' will be used if id is simply {name}
        #[arg(value_name("NAMESPACE/NAME"))]
        resource_id: String,

        /// The target state to which the resource will be set
        state: StateKind,
    },
    /// Start web server alone (without kube resource management)
    StartServer,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    ControllerError(#[from] resource::error::Resource),

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
            create_schedule(config.controller.refresh_interval)
                .await
                .start()
                .await?;
            server::start(config.server.port).await?;
        }
        Commands::Msg(e) => msg::process(e, config).await?,
        Commands::Status => status().await?,
    };
    Ok(())
}

#[tokio::main]
async fn main() {
    match process() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error : {}", e);
            process::exit(1);
        }
    }
}

async fn status() -> Result<(), Error> {
    let deploys = Deploy::get_all().await?;

    let services = Service::get_all().await?;

    let traefik_metrics_pods = crate::core::ingress::traefik::Traefik::get_ingress_pods()
        .await?
        .into_iter()
        .map(|pod| pod.metadata.name)
        .collect::<Vec<_>>();

    #[derive(Serialize)]
    struct MetricPodsClass {
        traefik: Vec<Option<String>>,
    }

    #[derive(Serialize)]
    struct Status {
        deploys: Vec<Deploy>,
        services: Vec<Service>,
        #[serde(rename = "metric pods")]
        metric_pods: MetricPodsClass,
    }

    println!(
        "{}",
        serde_yaml::to_string(&Status {
            deploys: deploys,
            services: services,
            metric_pods: MetricPodsClass {
                traefik: traefik_metrics_pods
            }
        })
        .unwrap_or_else(|e| format!("{e} : Status structure should be serealizable at this point"))
    );
    Ok(())
}
