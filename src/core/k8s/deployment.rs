use std::{collections::BTreeMap, num::ParseIntError, time::Duration};

use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    Api, Client, ResourceExt,
    api::{ListParams, Patch, PatchParams},
    runtime::reflector::Lookup,
};
use tracing::info;

use crate::core::{
    k8s::{
        annotations::Annotations,
        constantes::{
            ANNOTATION_REPLICAS_KEY, KUBESLEEPER_ANNOTATION_PREFIX, KUBESLLEPER_APP_NAME,
        },
        identifier::Identifier,
        resource_name::{ResourceName, error::ResourceNameError},
    },
    state::state_kind::StateKind,
};

#[derive(Debug, thiserror::Error)]
pub enum KsDeploymentParsingError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("missing 'name'")]
    MissingNameError,

    #[error("invalid 'name': {0}")]
    InvalidNameError(ResourceNameError),

    #[error("deployment {0}: missing '.spec.replicas'")]
    MissingReplicasError(Identifier),

    #[error("missing 'namespace'")]
    MissingNamespaceError,

    #[error("invalid 'namespace': {0}")]
    InvalidNamespaceError(ResourceNameError),

    #[error("deployment {0}: missing '{prefix}{key}' annotation", prefix = KUBESLEEPER_ANNOTATION_PREFIX, key = ANNOTATION_REPLICAS_KEY)]
    MissingReplicasAnnotationError(Identifier),

    #[error("deployment {0}: invalid '{prefix}{key}' annotation: {1}", prefix = KUBESLEEPER_ANNOTATION_PREFIX, key = ANNOTATION_REPLICAS_KEY)]
    InvalidReplicasAnnotationError(Identifier, ParseIntError),

    #[error("deployment {0}: missing 'status'")]
    MissingStatusError(Identifier),
}

#[derive(Debug, thiserror::Error)]
pub enum KsDeploymentFetchingError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("deployment {0} not found")]
    DeploymentNotFoundError(Identifier),
}

#[derive(Debug, thiserror::Error)]
pub enum KsDeploymentInteractError {
    #[error("failed to set deployment {0} asleep: {1}")]
    DeploymentSleepingError(Identifier, KsDeploymentParsingError),

    #[error("failed to set deployment {0} awake: {1}")]
    DeploymentWakingError(Identifier, KsDeploymentParsingError),

    #[error("failed to get deployment {0} state: {1}")]
    DeploymentFetchingStateError(Identifier, KsDeploymentParsingError),

    #[error("failed to patch deployment {0}: {1}")]
    DeploymentPatchingError(Identifier, KsDeploymentParsingError),

    #[error("failed waiting for deployment {0} to wake up: {1}")]
    DeploymentWaitingWakeupError(Identifier, KsDeploymentParsingError),

    #[error("maximum waiting time for deployment {0} to wake up exceeded")]
    MaxWaitingWakeTimeError(Identifier),
}

pub trait KsDeployment {
    fn ks_id(&self) -> Result<Identifier, KsDeploymentParsingError>;
    fn ks_get_replicas(&self) -> Result<i32, KsDeploymentParsingError>;
    fn ks_get_target_replicas(&self) -> Result<i32, KsDeploymentParsingError>;
    async fn ks_get_ready_replicas_count(&self) -> Result<i32, KsDeploymentParsingError>;

    async fn get_all() -> Result<Vec<Deployment>, KsDeploymentFetchingError>;
    async fn get(id: &Identifier) -> Result<Deployment, KsDeploymentFetchingError>;

    async fn ks_patch(
        &self,
        replicas: i32,
        target_replicas: i32,
    ) -> Result<(), KsDeploymentInteractError>;
    fn ks_get_state(&self) -> Result<StateKind, KsDeploymentInteractError>;
    async fn wait_ready(&self) -> Result<(), KsDeploymentInteractError>;
    async fn ks_wake(&self) -> Result<(), KsDeploymentInteractError>;
    async fn ks_sleep(&self) -> Result<(), KsDeploymentInteractError>;
}

impl KsDeployment for Deployment {
    fn ks_id(&self) -> Result<Identifier, KsDeploymentParsingError> {
        let name: ResourceName = self
            .name()
            .ok_or(KsDeploymentParsingError::MissingNameError)?
            .to_string()
            .try_into()
            .map_err(|e| KsDeploymentParsingError::InvalidNameError(e))?;

        let namespace = ResourceExt::namespace(self)
            .ok_or(KsDeploymentParsingError::MissingNamespaceError)?
            .try_into()
            .map_err(|e| KsDeploymentParsingError::InvalidNamespaceError(e))?;

        Ok(Identifier { namespace, name })
    }

    async fn ks_patch(
        &self,
        replicas: i32,
        target_replicas: i32,
    ) -> Result<(), KsDeploymentInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsDeploymentInteractError::DeploymentPatchingError(Identifier::new_unknow(), e)
        })?;

        let patch = serde_json::json!({
            "spec" : {
                "replicas": replicas
            },
            "metadata": {
                "annotations": {
                    format!("{}{}",KUBESLEEPER_ANNOTATION_PREFIX,ANNOTATION_REPLICAS_KEY): target_replicas.to_string()
                }
            }
        });
        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);

        let client = Client::try_default().await.map_err(|e| {
            KsDeploymentInteractError::DeploymentPatchingError(id.clone(), e.into())
        })?;
        let api: Api<Deployment> = Api::namespaced(client, &id.namespace.to_string());
        api.patch(&id.name.to_string(), &params, &patch)
            .await
            .map_err(|e| KsDeploymentInteractError::DeploymentPatchingError(id, e.into()))?;
        Ok(())
    }

    fn ks_get_state(&self) -> Result<StateKind, KsDeploymentInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsDeploymentInteractError::DeploymentFetchingStateError(Identifier::new_unknow(), e)
        })?;
        if self
            .ks_get_replicas()
            .map_err(|e| KsDeploymentInteractError::DeploymentFetchingStateError(id, e))?
            == 0
        {
            Ok(StateKind::Asleep)
        } else {
            Ok(StateKind::Awake)
        }
    }

    async fn ks_wake(&self) -> Result<(), KsDeploymentInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsDeploymentInteractError::DeploymentFetchingStateError(Identifier::new_unknow(), e)
        })?;
        if self.ks_get_state()? == StateKind::Awake {
            return Ok(());
        }
        let target_replicas = self
            .ks_get_target_replicas()
            .map_err(|e| KsDeploymentInteractError::DeploymentWakingError(id.clone(), e))?;
        self.ks_patch(target_replicas, target_replicas).await?;
        Ok(())
    }

    async fn ks_sleep(&self) -> Result<(), KsDeploymentInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsDeploymentInteractError::DeploymentFetchingStateError(Identifier::new_unknow(), e)
        })?;
        if self.ks_get_state()? == StateKind::Asleep {
            return Ok(());
        }
        let let_replicas = self
            .ks_get_replicas()
            .map_err(|e| KsDeploymentInteractError::DeploymentSleepingError(id.clone(), e))?;

        self.ks_patch(0, let_replicas).await?;
        Ok(())
    }

    fn ks_get_replicas(&self) -> Result<i32, KsDeploymentParsingError> {
        Ok(self
            .spec
            .as_ref()
            .ok_or(KsDeploymentParsingError::MissingReplicasError(
                self.ks_id()?,
            ))?
            .replicas
            .ok_or(KsDeploymentParsingError::MissingReplicasError(
                self.ks_id()?,
            ))?)
    }

    fn ks_get_target_replicas(&self) -> Result<i32, KsDeploymentParsingError> {
        let id = self.ks_id()?;

        let raw_annotations = self.metadata.annotations.as_ref();
        let annotations = Annotations::from(raw_annotations.unwrap_or(&BTreeMap::default()));
        annotations
            .get(ANNOTATION_REPLICAS_KEY)
            .map(|raw_target_replicas| {
                raw_target_replicas.parse::<i32>().map_err(|err| {
                    KsDeploymentParsingError::InvalidReplicasAnnotationError(id.clone(), err)
                })
            })
            .transpose()?
            .ok_or(KsDeploymentParsingError::MissingReplicasAnnotationError(id))
            .map_err(KsDeploymentParsingError::from)
    }

    async fn ks_get_ready_replicas_count(&self) -> Result<i32, KsDeploymentParsingError> {
        Ok(self
            .status
            .as_ref()
            .ok_or(KsDeploymentParsingError::MissingStatusError(self.ks_id()?))?
            .ready_replicas
            .unwrap_or_default())
    }

    /// TODO: remove this, should be better to loop from the parent call no the Deployment itself
    async fn wait_ready(&self) -> Result<(), KsDeploymentInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsDeploymentInteractError::DeploymentFetchingStateError(Identifier::new_unknow(), e)
        })?;
        let replicas = self
            .ks_get_replicas()
            .map_err(|e| KsDeploymentInteractError::DeploymentWaitingWakeupError(id.clone(), e))?;
        for i in 0_u32..1000 {
            let current_ready_replicas = self.ks_get_ready_replicas_count().await.map_err(|e| {
                KsDeploymentInteractError::DeploymentWaitingWakeupError(id.clone(), e)
            })?;
            if replicas - current_ready_replicas == 0 {
                info!("Deploy {} just woke up.", id);
                return Ok(());
            }

            info!(
                "Deploy {} is waking up. Waiting for replicas to be ready : {}/{}",
                id, current_ready_replicas, replicas
            );

            let duration = 100 * 2_u64.pow([i, 7].into_iter().min().expect("Couldn't be empty"));
            tokio::time::sleep(Duration::from_millis(duration)).await;
        }

        Err(KsDeploymentInteractError::MaxWaitingWakeTimeError(id))
    }

    async fn get_all() -> Result<Vec<Deployment>, KsDeploymentFetchingError> {
        let lp = ListParams::default().match_any().fields(&format!(
            "metadata.name!={},metadata.namespace!=kube-system",
            KUBESLLEPER_APP_NAME
        ));

        let client = Client::try_default().await?;
        let api: Api<Deployment> = Api::all(client);

        Ok(api.list(&lp).await?.into_iter().collect())
    }

    async fn get(id: &Identifier) -> Result<Deployment, KsDeploymentFetchingError> {
        let lp = ListParams::default().match_any().fields(&format!(
            "metadata.name!={},metadata.namespace!=kube-system,metadata.namespace=={},metadata.name=={}",
            KUBESLLEPER_APP_NAME,
            id.namespace,
            id.name
        ));

        let client = Client::try_default().await?;
        let api: Api<Deployment> = Api::all(client);

        Ok(api
            .list(&lp)
            .await?
            .into_iter()
            .collect::<Vec<Deployment>>()
            .first()
            .map(|d| d.clone())
            .ok_or(KsDeploymentFetchingError::DeploymentNotFoundError(id))?)
    }
}
