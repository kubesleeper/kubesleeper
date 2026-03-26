use clap::{Subcommand, ValueEnum};
use k8s_openapi::api::{apps::v1::Deployment, core::v1::Service};
use tracing::info;

use crate::core::{
    config::Config,
    k8s::{
        deployment::{KsDeployment, KsDeploymentFetchingError, KsDeploymentInteractError},
        identifier::Identifier,
        kubesleeper::{KubesleeperError, check_kubesleeper},
        service::{KsService, KsServiceFetchingError, KsServiceInteractError},
    },
    state::state_kind::StateKind,
};

#[derive(Subcommand)]
pub enum Message {
    /// Dump the computed configuration
    DumpConfig,

    /// Set namespace to the desired state
    Set {
        /// The target state to which the cluster will be set
        state: StateKind,
    },

    /// Set a specific Deployment or Service to the desired state
    SetRsc {
        /// the kubernetes shortname of resource
        resource_type: ResourceType,

        /// the kube resournce id like {namespace}/{name},
        /// namespace 'default' will be used if id is simply {name}
        #[arg(value_name("NAMESPACE/NAME"), value_parser = clap::value_parser!(Identifier))]
        resource_id: Identifier,

        /// The target state to which the resource will be set
        state: StateKind,
    },
    /// Start web server alone (without kube resource management)
    StartServer,
}

#[derive(Debug, thiserror::Error)]
pub enum MsgError {
    #[error(transparent)]
    ServerError(#[from] crate::core::server::error::ServerError),

    #[error(transparent)]
    KubesleeperError(#[from] KubesleeperError),

    #[error(transparent)]
    KsDeploymentInteractError(#[from] KsDeploymentInteractError),

    #[error(transparent)]
    KsServiceInteractError(#[from] KsServiceInteractError),

    #[error(transparent)]
    KsServiceFetchingError(#[from] KsServiceFetchingError),

    #[error(transparent)]
    KsDeploymentFetchingError(#[from] KsDeploymentFetchingError),
}

async fn set_one_deployment_asleep(
    deployment: &Deployment,
    state: StateKind,
) -> Result<(), MsgError> {
    match state {
        StateKind::Asleep => deployment.ks_sleep().await?,
        StateKind::Awake => deployment.ks_wake().await?,
    }
    Ok(())
}
async fn set_one_service_asleep(service: &Service, state: StateKind) -> Result<(), MsgError> {
    match state {
        StateKind::Asleep => service.ks_sleep().await?,
        StateKind::Awake => service.ks_wake().await?,
    }
    Ok(())
}
async fn set_all(state: StateKind) -> Result<(), MsgError> {
    info!("Making all Deploy and Service '{state}'");
    check_kubesleeper().await?;

    for deploy in Deployment::get_all().await?.iter() {
        set_one_deployment_asleep(deploy, state).await?;
    }
    for service in Service::get_all().await?.iter() {
        set_one_service_asleep(service, state).await?;
    }

    Ok(())
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ResourceType {
    Svc,
    Deploy,
}

fn dump_config(config: Config) -> Result<(), MsgError> {
    println!(
        "{}",
        serde_yaml::to_string(&config).unwrap_or(format!("{config:?}"))
    );
    Ok(())
}

pub async fn process(msg: Message, config: Config) -> Result<(), MsgError> {
    match msg {
        Message::Set { state } => set_all(state).await,
        Message::SetRsc {
            resource_type,
            resource_id,
            state,
        } => match resource_type {
            ResourceType::Svc => {
                let svc = Service::get(resource_id.clone()).await?;
                set_one_service_asleep(&svc, state).await?;
                Ok(())
            }
            ResourceType::Deploy => {
                let deploy = Deployment::get(resource_id.clone()).await?;
                set_one_deployment_asleep(&deploy, state).await?;
                Ok(())
            }
        },
        Message::StartServer => crate::core::server::start(config.server.port)
            .await
            .map_err(|e| e.into()),
        Message::DumpConfig => dump_config(config),
    }
}
