use clap::Subcommand;
use tracing::info;

use crate::{
    Error,
    core::{
        config::Config,
        resource::{TargetResource, deploy::Deploy, service::Service},
        state::state_kind::StateKind,
    },
};

#[derive(Subcommand)]
pub enum Message {
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

async fn set(state: StateKind) -> Result<(), Error> {
    info!("Making all Deploy and Service '{state}'");

    match state {
        StateKind::Asleep => {
            for deploy in Deploy::get_all().await?.iter_mut() {
                deploy.sleep().await?
            }
            for service in Service::get_all().await?.iter_mut() {
                service.sleep().await?
            }
        }
        StateKind::Awake => {
            for deploy in Deploy::get_all().await?.iter_mut() {
                deploy.wake().await?
            }
            for service in Service::get_all().await?.iter_mut() {
                service.wake().await?
            }
        }
    }
    Ok(())
}

async fn set_deploy(state: StateKind, resource_name: String) -> Result<(), Error> {
    let mut deploys = Deploy::get_all().await?;
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
    let mut services = Deploy::get_all().await?;
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

pub async fn process(msg: Message, config: Config) -> Result<(), Error> {
    match msg {
        Message::Set { state } => set(state).await,
        Message::SetDeploy { resource_id, state } => set_deploy(state, resource_id).await,
        Message::SetService { resource_id, state } => set_service(state, resource_id).await,
        Message::StartServer => crate::core::server::start(config.server.port)
            .await
            .map_err(|e| e.into()),
        Message::DumpConfig => dump_config(config),
    }
}
