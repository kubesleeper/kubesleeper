use std::collections::HashMap;

use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{Service, ServicePort},
};

use serde::{Serialize, Serializer};
use tracing::debug;

use crate::core::{
    ingress::{IngressType, error::IngressError},
    k8s::{
        deployment::{
            KsDeployment, KsDeploymentFetchingError, KsDeploymentInteractError,
            KsDeploymentParsingError,
        },
        identifier::Identifier,
        kubesleeper::KubesleeperError,
        service::{
            KsService, KsServiceFetchingError, KsServiceInteractError, KsServiceParsingError,
        },
    },
    state::state_kind::StateKind,
};

#[derive(Serialize)]
struct DeployStatus {
    id: Identifier,
    state: String,
    #[serde(serialize_with = "serialize_opt_i32")]
    target_replicas: Option<i32>,
}
fn serialize_opt_i32<S>(val: &Option<i32>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match val {
        Some(v) => s.serialize_i32(*v),
        None => s.serialize_str("unknown"),
    }
}

#[derive(Serialize)]
struct ServiceStatus {
    id: Identifier,
    state: String,
    selector: HashMap<String, String>,
    ports: Vec<ServicePort>,
    #[serde(serialize_with = "serialize_opt_map")]
    target_selector: Option<HashMap<String, String>>,
    #[serde(serialize_with = "serialize_opt_vec")]
    target_ports: Option<Vec<ServicePort>>,
}
fn serialize_opt_map<S>(val: &Option<HashMap<String, String>>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match val {
        Some(m) if !m.is_empty() => m.serialize(s),
        _ => s.serialize_str("unknown"),
    }
}
fn serialize_opt_vec<S>(val: &Option<Vec<ServicePort>>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match val {
        Some(m) if !m.is_empty() => m.serialize(s),
        _ => s.serialize_str("unknown"),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error(transparent)]
    ServerError(#[from] crate::core::server::error::ServerError),

    #[error("Failed to check kubesleeper deployment health : {0}")]
    KubesleeperError(#[from] KubesleeperError),

    #[error(transparent)]
    KsDeploymentInteractError(#[from] KsDeploymentInteractError),

    #[error(transparent)]
    KsServiceInteractError(#[from] KsServiceInteractError),

    #[error(transparent)]
    KsServiceFetchingError(#[from] KsServiceFetchingError),

    #[error(transparent)]
    KsDeploymentFetchingError(#[from] KsDeploymentFetchingError),

    #[error(transparent)]
    KsServiceParsingError(#[from] KsServiceParsingError),

    #[error(transparent)]
    KsDeploymentParsingError(#[from] KsDeploymentParsingError),

    #[error(transparent)]
    IngressError(#[from] IngressError),
}

pub async fn status() -> Result<(), StatusError> {
    let deploys = Deployment::get_all().await?;
    debug!("All ({}) deployment fetched", deploys.len());

    let mut deploys_status = Vec::new();
    for deploy in deploys {
        let state = match deploy.ks_get_state()? {
            StateKind::Asleep => "asleep".to_string(),
            StateKind::Awake => {
                let ready_replicas_count = deploy.ks_get_ready_replicas_count().await?;
                let replicas = deploy.ks_get_replicas()?;
                if ready_replicas_count != replicas {
                    format!("waking up ({}/{})", ready_replicas_count, replicas)
                } else {
                    "awake".to_string()
                }
            }
        };

        deploys_status.push(DeployStatus {
            id: deploy.ks_id()?,
            state,
            target_replicas: match deploy.ks_get_target_replicas() {
                Ok(i) => Some(i),
                Err(KsDeploymentParsingError::MissingReplicasAnnotationError(_)) => None,
                Err(e) => return Err(e.into()),
            },
        });
    }

    let services = Service::get_all().await?;

    let mut services_status = Vec::new();
    for s in services {
        let state = match s.ks_get_state()? {
            StateKind::Asleep => "asleep".to_string(),
            StateKind::Awake => "awake".to_string(),
        };

        services_status.push(ServiceStatus {
            id: s.ks_id()?,
            state,
            selector: s.ks_get_selector()?,
            ports: s.ks_get_ports()?,
            target_selector: match s.ks_get_target_selector() {
                Ok(s) => Some(s),
                Err(KsServiceParsingError::MissingSelectorAnnotationError(_)) => None,
                Err(e) => return Err(e.into()),
            },
            target_ports: match s.ks_get_target_ports() {
                Ok(s) => Some(s),
                Err(KsServiceParsingError::MissingPortsAnnotationError(_)) => None,
                Err(e) => return Err(e.into()),
            },
        });
    }

    let traefik_metrics_pods = crate::core::ingress::traefik::Traefik::get_ingress_pods()
        .await?
        .into_iter()
        .map(|pod| pod.metadata.name)
        .collect::<Vec<_>>();

    let json = serde_json::json!({
        "Deployments" : deploys_status,
        "Services" : services_status,
        "Metric Pods": {
            "Traefik" : traefik_metrics_pods
        }
    });

    println!(
        "{}",
        serde_yaml::to_string(&json).unwrap_or_else(|e| format!(
            "{e} : Status structure should be serealizable at this point"
        ))
    );
    Ok(())
}
