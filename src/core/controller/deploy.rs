use k8s_openapi::api::apps::v1::Deployment;
use kube::api::{ListParams, Patch, PatchParams};
use kube::runtime::reflector::Lookup;
use kube::{Api, Client, ResourceExt};

use std::collections::BTreeMap;
use std::fmt;

use crate::core::controller::constantes::*;
use crate::core::controller::error::ControllerError;
use crate::core::state::state_kind::StateKind;

pub struct Deploy {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,

    pub store_state: StateKind,
    pub store_replicas: i32,
}
impl TryFrom<&Deployment> for Deploy {
    type Error = ControllerError;

    fn try_from(deploy: &Deployment) -> std::result::Result<Self, Self::Error> {
        // --- explicit data
        let name = deploy
            .name()
            .ok_or(ControllerError::ResourceDataError(
                "Missing 'name' field".to_string(),
            ))?
            .to_string();
        let namespace = ResourceExt::namespace(deploy).ok_or(
            ControllerError::ResourceDataError("Missing 'namespace' field".to_string()),
        )?;
        let deploy_id = format!("{name} ({namespace})");
        let replicas = deploy.spec.as_ref().and_then(|s| s.replicas).ok_or(
            ControllerError::ResourceDataError("Can't parse Deployment {deploy_id} : Missing '.spec' field".to_string()),
        )?;
        
        
        // --- store annotation
        let raw_annotations = deploy.metadata.annotations.as_ref();
        let annotations =
            super::annotations::Annotations::from(raw_annotations.unwrap_or(&BTreeMap::default()));
        
        // state
        let default_state = match replicas {
            0 => StateKind::Asleep.to_string(),
            _ => StateKind::Awake.to_string(),
        };
        let raw_store_state: &str = annotations
            .get(ANNOTATION_STORE_STATE_KEY)
            .unwrap_or(&default_state)
            .as_str();
        let store_state = StateKind::try_from(raw_store_state).map_err(|err| {
            ControllerError::ResourceDataError(format!(
                "Can't parse Deployment {deploy_id} : Can't parse annotation '{}{}' : {}",
                KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_STATE_KEY, err
            ))
        })?;
        
        // replicas
        let raw_store_replicas: &str = annotations
            .get(ANNOTATION_STORE_REPLICAS_KEY)
            .ok_or(ControllerError::ResourceDataError(format!(
                "Can't parse Deployment {deploy_id} : Missing required annotation '{}{}'",
                KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_REPLICAS_KEY
            )))?
            .as_str();
        let store_replicas = raw_store_replicas.parse::<i32>().map_err(|err| {
            ControllerError::ResourceDataError(format!(
                "Can't parse Deployment {deploy_id} : Can't parse annotation '{}{}' : {}",
                KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_STATE_KEY, err
            ))
        })?;


        Ok(Deploy {
            name,
            namespace,
            replicas,
            store_replicas,
            store_state,
        })
    }
}
impl Deploy {
    
    async fn get_k8s_api(namespace: &str) -> Result<Api<Deployment>, ControllerError> {
        let client = Client::try_default().await?;

        let deployments: Api<Deployment> = Api::namespaced(client, namespace);
        return Ok(deployments);
    }
    
    async fn patch(&self) -> Result<(), ControllerError> {
        let patch = serde_json::json!({
            "spec" : {
                "replicas": self.replicas
            },
            "metadata": {
                "annotations": {
                    ANNOTATION_STORE_STATE_KEY: self.store_state.to_string(),
                    ANNOTATION_STORE_REPLICAS_KEY: self.store_replicas
                }
            }
        });

        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        Deploy::get_k8s_api(&self.namespace).await?.patch(&self.name, &params, &patch).await?;

        Ok(())
    }
}

impl Deploy {    
    
    pub async fn wake(&mut self) -> Result<(), ControllerError> {
        if self.store_state == StateKind::Awake {
            println!(
                "State already marked as '{}', skipping wake action",
                StateKind::Awake.to_string()
            );
            return Ok(());
        }

        self.replicas = self.store_replicas;
        self.store_state = StateKind::Awake;
        self.patch().await
    }

    pub async fn sleep(&mut self) -> Result<(), ControllerError> {
        if self.store_state == StateKind::Asleep {
            println!(
                "State already marked as '{}', skipping sleep action",
                StateKind::Asleep.to_string()
            );
            return Ok(());
        }

        self.store_replicas = self.replicas;
        self.replicas = 0;
        self.store_state = StateKind::Asleep;
        self.patch().await
    }
    
    pub async fn get_all(namespace: &str) -> Result<Vec<Deploy>, ControllerError> {
        Deploy::get_k8s_api(namespace).await?
            .list(&ListParams::default())
            .await?
            .iter()
            .map(|d| Deploy::try_from(d))
            .collect()
    }
    
    pub async fn change_all_state(state: StateKind) -> Result<(), ControllerError>{
        let deploys = Deploy::get_all("ks").await?;
    
        for mut deploy in deploys {
            match state {
                StateKind::Asleep => { deploy.sleep().await? }
                StateKind::Awake  => { deploy.wake().await?  }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Deploy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.name)?;
        writeln!(f, "  Namespace        : {}", self.namespace)?;
        writeln!(f, "  Current Replicas : {}", self.replicas)?;
        writeln!(f, "  Target Replicas  : {} (store)", self.store_replicas)?;
        writeln!(f, "  Current state    : {} (store)", self.store_state)?;
        Ok(())
    }
}
