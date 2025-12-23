use clap::{Subcommand, ValueEnum};
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
        /// The target state to which the cluster will be set
        state: StateKind,
    },

    /// Set a specific Deployment or Service to the desired state
    SetRsc {
        /// the kubernetes shortname of resource
        resource_type: ResourceType,

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
    Deploy::check_kubesleeper().await?;

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

#[derive(Debug, Clone, ValueEnum)]
pub enum ResourceType {
    Svc,
    Deploy,
}

async fn set_rsc_process<T>(state: StateKind, resource_name: String) -> Result<(), Error>
where
    T: TargetResource<'static>,
{
    // On garde la vérification commune
    Deploy::check_kubesleeper().await?;

    // On récupère les ressources du type T
    let mut resources = T::get_all().await?;

    let target = resources
        .iter_mut()
        .find(|r| r.id() == resource_name)
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
        Message::SetRsc {
            resource_type,
            resource_id,
            state,
        } => match resource_type {
            ResourceType::Svc => set_rsc_process::<Service>(state, resource_id).await,
            ResourceType::Deploy => set_rsc_process::<Deploy>(state, resource_id).await,
        },
        Message::StartServer => crate::core::server::start(config.server.port)
            .await
            .map_err(|e| e.into()),
        Message::DumpConfig => dump_config(config),
    }
}
