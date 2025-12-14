use clap::Subcommand;
use serde::Serialize;
use tracing::info;

use crate::{
    Error,
    core::{
        config::Config,
        controller::{deploy::Deploy, service::Service, set_kubesleeper_namespace},
        ingress::IngressType,
        state::state_kind::StateKind,
    },
};

#[derive(Subcommand)]
pub enum Message {
    /// Describe the kubesleeper cluster status
    Status,

    /// Dump the computed configuration
    DumpConfig,

    /// Set namespace to the desired state
    Set {
        /// The target state to which the namespace will be set
        state: StateKind,
    },

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

pub mod error {

    #[derive(Debug, thiserror::Error)]
    pub enum Msg {
        #[error("Resource '{resource_id}' not found")]
        ResourceNotFound { resource_id: String },

        #[error(transparent)]
        ServerError(#[from] crate::core::server::error::ServerError),
    }
}

async fn status() -> Result<(), Error> {
    set_kubesleeper_namespace().await?;
    let deploys = crate::core::controller::deploy::Deploy::get_all_target().await?;

    let services = crate::core::controller::service::Service::get_all_target("ks").await?;

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

async fn set(state: StateKind) -> Result<(), Error> {
    set_kubesleeper_namespace().await?;
    info!("Making all Deploy '{state}'");
    Deploy::change_all_state(state).await?;
    info!("Making all Service '{state}'");
    Service::change_all_state(state).await?;
    Ok(())
}

async fn set_deploy(state: StateKind, resource_name: String) -> Result<(), Error> {
    set_kubesleeper_namespace().await?;
    let mut deploys = Deploy::get_all_target().await?;
    let target = deploys.iter_mut().find(|d| d.name == resource_name).ok_or(
        error::Msg::ResourceNotFound {
            resource_id: resource_name,
        },
    )?;
    match state {
        StateKind::Asleep => target.sleep().await?,
        StateKind::Awake => target.wake().await?,
    };
    Ok(())
}
async fn set_service(state: StateKind, resource_name: String) -> Result<(), Error> {
    set_kubesleeper_namespace().await?;
    let mut services = Deploy::get_all_target().await?;
    let target = services
        .iter_mut()
        .find(|d| d.name == resource_name)
        .ok_or(error::Msg::ResourceNotFound {
            resource_id: resource_name,
        })?;
    match state {
        StateKind::Asleep => target.sleep().await?,
        StateKind::Awake => target.wake().await?,
    };
    Ok(())
}

fn dump_config(config: Config) -> Result<(), Error> {
    println!(
        "{}",
        serde_yaml::to_string(&config).unwrap_or(format!("{config:?}"))
    );
    Ok(())
}

pub async fn vvv(msg: Message, config: Config) -> Result<(), Error> {
    match msg {
        Message::Status => status().await,
        Message::Set { state } => set(state).await,
        Message::SetDeploy { resource_id, state } => set_deploy(state, resource_id).await,
        Message::SetService { resource_id, state } => set_service(state, resource_id).await,
        Message::StartServer => crate::core::server::start(config.server.port)
            .await
            .map_err(|e| e.into()),
        Message::DumpConfig => dump_config(config),
    }
}
