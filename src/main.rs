extern crate rocket;
mod core;

use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use tokio_cron_scheduler::JobSchedulerError;

use crate::core::{
    controller,
    controller::{
        deploy::Deploy,
        error::{self, Controller},
        service::Service,
        set_kubesleeper_namespace,
    },
    ingress::{IngressType, error::IngressError},
    logger::{self, HUMAN_READABLE_MODE, VERBOSE_MODE, init_logger},
    server,
    server::error::ServerError,
    state::{state::create_schedule, state_kind::StateKind},
};
use tracing;

const LOG_FILE_PATH: &str = "kubesleeper.log";

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
}

#[derive(Subcommand)]
enum Commands {
    /// Describe the kubesleeper cluster status
    Status,

    /// start kubesleeper service
    Start,

    #[command(subcommand)]
    /// Exec specific manual action for testing
    Manual(Manual),
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
        /// the kube resournce id (like {name}/{namespace}) to target,
        /// namespace 'default' will be used if id is simply {name}
        resource_id: String,

        /// The target state to which the resource will be set
        state: StateKind,
    },

    /// Set a specific Deployment or Service to the desired state
    SetService {
        /// the kube resournce id like {name}/{namespace},
        /// namespace 'default' will be used if id is simply {name}
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
    ControllerError(#[from] controller::error::Controller),

    #[error(transparent)]
    IngressError(#[from] IngressError),

    #[error(transparent)]
    JobSchedulerError(#[from] JobSchedulerError),

    #[error(transparent)]
    ServerError(#[from] ServerError),

    #[error(transparent)]
    LoggerError(#[from] logger::error::Logger),
}

#[tokio::main]
async fn process() -> Result<(), Error> {
    set_kubesleeper_namespace().await?;
    let cli = Cli::parse();

    //logging setup
    VERBOSE_MODE
        .set(cli.verbose)
        .expect("Failed to set up verbose mode");
    HUMAN_READABLE_MODE
        .set(cli.readable_log)
        .expect("Failed to set up human-readable mode");
    init_logger()?;

    match cli.command {
        Commands::Start => {
            server::start().await?;
            create_schedule().await.start().await?;
        }
        Commands::Status => {
            let deploys = crate::core::controller::deploy::Deploy::get_all_target().await?;

            let services = crate::core::controller::service::Service::get_all_target("ks").await?;

            let traefik_metrics_pods = crate::core::ingress::traefik::Traefik::get_ingress_pods()
                .await?
                .into_iter()
                .map(|pod| pod.metadata.name)
                .collect::<Vec<_>>();

            #[derive(Serialize)]
            pub struct MetricPodsClass {
                traefik: Vec<Option<String>>,
            }

            #[derive(Serialize)]
            pub struct Status {
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
                .unwrap_or_else(|e| format!(
                    "{e} : Status structure should be serealizable at this point"
                ))
            )
        }

        Commands::Manual(subcmd) => match &subcmd {
            // merge 2 cases to not have code duplication for splitting resource-name and namespace
            Manual::SetDeploy { resource_id, state }
            | Manual::SetService { resource_id, state } => {
                let (rsc_name, rsc_ns) =
                    if let Some((rsc_name, rsc_ns)) = resource_id.split_once('/') {
                        (rsc_name, rsc_ns)
                    } else {
                        (resource_id.as_str(), "default")
                    };

                tracing::debug!(
                    "Set '{}' deployment '{}' of namespace '{}'",
                    state,
                    rsc_name,
                    rsc_ns
                );

                let missing_target_message =
                    format!("'{}' of namespace '{}' not found", rsc_name, rsc_ns);
                match subcmd {
                    Manual::SetDeploy { .. } => {
                        if let Some(deploy) = Deploy::get_all_target()
                            .await?
                            .iter_mut()
                            .find(|x| x.name == rsc_name)
                        {
                            match state {
                                StateKind::Asleep => deploy.sleep().await?,
                                StateKind::Awake => deploy.wake().await?,
                            }
                        } else {
                            eprintln!("Error : Deployment {}", missing_target_message);
                        }
                    }
                    Manual::SetService { .. } => {
                        if let Some(service) = Deploy::get_all_target()
                            .await?
                            .iter_mut()
                            .find(|x| x.name == rsc_name)
                        {
                            match state {
                                StateKind::Asleep => service.sleep().await?,
                                StateKind::Awake => service.wake().await?,
                            }
                        } else {
                            eprintln!("Error : Service {}", missing_target_message);
                        }
                    }
                    _ => {}
                }
            }
            Manual::StartServer => {
                server::start().await?;
            }
        },
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
