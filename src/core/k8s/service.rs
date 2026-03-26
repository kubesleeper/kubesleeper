use std::collections::{BTreeMap, HashMap};

use crate::core::{
    k8s::{
        annotations::Annotations,
        constantes::{
            ANNOTATION_PORTS_KEY, ANNOTATION_SELECTOR_KEY, KUBESLEEPER_ANNOTATION_PREFIX,
            KUBESLEEPER_SELECTOR_KEY, KUBESLEEPER_SELECTOR_VALUE, KUBESLEEPER_SERVER_PORT,
            KUBESLLEPER_APP_NAME,
        },
        identifier::Identifier,
        resource_name::{ResourceName, error::ResourceNameError},
    },
    state::state_kind::StateKind,
};
use k8s_openapi::{
    api::core::v1::{Service, ServicePort},
    apimachinery::pkg::util::intstr::IntOrString,
};
use kube::{
    Api, Client, ResourceExt,
    api::{ListParams, Patch, PatchParams},
    runtime::reflector::Lookup,
};

#[derive(Debug, thiserror::Error)]
pub enum KsServiceParsingError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("Service don't have status")]
    MissingStatusError,

    // NAME
    #[error("Missing 'name'")]
    MissingNameError,
    #[error("Invalid 'name' : {0}")]
    InvalidNameError(ResourceNameError),

    // NAMESPACE
    #[error("Missing 'namespace'")]
    MissingNamespaceError,
    #[error("Invalid 'namespace' : {0}")]
    InvalidNamespaceError(ResourceNameError),

    // SELECTOR
    #[error("Service {0} : Missing '.spec.selector'")]
    MissingSelectorError(Identifier),
    #[error("Service {0} : Missing '{prefix}{key}' annotation", prefix = KUBESLEEPER_ANNOTATION_PREFIX, key = ANNOTATION_SELECTOR_KEY)]
    MissingSelectorAnnotationError(Identifier),
    #[error("Service {0} : Invalid '{prefix}{key}' : {0}", prefix = KUBESLEEPER_ANNOTATION_PREFIX, key = ANNOTATION_SELECTOR_KEY)]
    InvalidSelectorAnnotationError(Identifier,serde_json::Error),

    // PORTS
    #[error("Service {0} : Missing '.spec.ports'")]
    MissingPortsError(Identifier),
    #[error("Service {0} : Missing '{prefix}{key}'", prefix = KUBESLEEPER_ANNOTATION_PREFIX, key = ANNOTATION_PORTS_KEY)]
    MissingPortsAnnotationError(Identifier),
    #[error("Service {0} : Invalid '{prefix}{key}' : {0}", prefix = KUBESLEEPER_ANNOTATION_PREFIX, key = ANNOTATION_PORTS_KEY)]
    InvalidPortsAnnotationError(Identifier,serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum KsServiceFetchingError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("Service {0} not found")]
    ServiceNotFoundError(Identifier),
}

#[derive(Debug, thiserror::Error)]
pub enum KsServiceInteractError {
    #[error("Failed to set Service {0} Asleep : {1}")]
    ServiceSleepingError(Identifier, KsServiceParsingError),

    #[error("Failed to set Service {0} Awake : {1}")]
    ServiceWakingError(Identifier, KsServiceParsingError),

    #[error("Failed to get Service {0} state: {1}")]
    ServiceFetchingStateError(Identifier, KsServiceParsingError),

    #[error("Failed to patch Service {0}: {1}")]
    ServicePatchingError(Identifier, KsServiceParsingError),

    #[error("Failed to patch Service {0}: {1}")]
    ServiceCreateJsonPatchError(Identifier, serde_json::Error),
}

pub trait KsService {
    fn ks_id(&self) -> Result<Identifier, KsServiceParsingError>;
    fn ks_get_selector(&self) -> Result<HashMap<String, String>, KsServiceParsingError>;
    fn ks_get_ports(&self) -> Result<Vec<ServicePort>, KsServiceParsingError>;
    fn ks_get_target_selector(&self) -> Result<HashMap<String, String>, KsServiceParsingError>;
    fn ks_get_target_ports(&self) -> Result<Vec<ServicePort>, KsServiceParsingError>;

    async fn get_all() -> Result<Vec<Service>, KsServiceFetchingError>;
    async fn get(id: Identifier) -> Result<Service, KsServiceFetchingError>;

    fn ks_get_state(&self) -> Result<StateKind, KsServiceInteractError>;
    async fn ks_wake(&self) -> Result<(), KsServiceInteractError>;
    async fn ks_sleep(&self) -> Result<(), KsServiceInteractError>;
    async fn ks_patch(
        &self,
        ports: &Vec<ServicePort>,
        target_ports: &Vec<ServicePort>,
        selector: &HashMap<String, String>,
        target_selector: &HashMap<String, String>,
    ) -> Result<(), KsServiceInteractError>;
}

impl KsService for Service {
    fn ks_id(&self) -> Result<Identifier, KsServiceParsingError> {
        let name: ResourceName = self
            .name()
            .ok_or(KsServiceParsingError::MissingNameError)?
            .to_string()
            .try_into()
            .map_err(|e| KsServiceParsingError::InvalidNameError(e))?;

        let namespace = ResourceExt::namespace(self)
            .ok_or(KsServiceParsingError::MissingNamespaceError)?
            .try_into()
            .map_err(|e| KsServiceParsingError::InvalidNamespaceError(e))?;

        Ok(Identifier { namespace, name })
    }

    fn ks_get_selector(&self) -> Result<HashMap<String, String>, KsServiceParsingError> {
        Ok(self
            .spec
            .as_ref()
            .and_then(|s| s.selector.as_ref())
            .ok_or(KsServiceParsingError::MissingSelectorError(self.ks_id()?))?
            .clone()
            .into_iter()
            .collect())
    }

    fn ks_get_ports(&self) -> Result<Vec<ServicePort>, KsServiceParsingError> {
        Ok(self
            .spec
            .as_ref()
            .and_then(|s| s.ports.as_ref())
            .ok_or(KsServiceParsingError::MissingPortsError(self.ks_id()?))?
            .clone()
            .into_iter()
            .collect())
    }

    fn ks_get_target_selector(&self) -> Result<HashMap<String, String>, KsServiceParsingError> {
        let id = self.ks_id()?;
        let raw_annotations = self.metadata.annotations.as_ref();
        let annotations = Annotations::from(raw_annotations.unwrap_or(&BTreeMap::default()));
        Ok(annotations
            .get(ANNOTATION_SELECTOR_KEY)
            .map(|raw_store_selector| {
                serde_json::from_str(raw_store_selector)
                    .map_err(|e| KsServiceParsingError::InvalidSelectorAnnotationError(id.clone(),e))
            })
            .unwrap_or(Err(KsServiceParsingError::MissingSelectorAnnotationError(id)))?)
    }

    fn ks_get_target_ports(&self) -> Result<Vec<ServicePort>, KsServiceParsingError> {
        let id = self.ks_id()?;
        let raw_annotations = self.metadata.annotations.as_ref();
        let annotations = Annotations::from(raw_annotations.unwrap_or(&BTreeMap::default()));
        Ok(annotations
            .get(ANNOTATION_PORTS_KEY)
            .map(|raw_store_selector| {
                serde_json::from_str(raw_store_selector)
                    .map_err(|e| KsServiceParsingError::InvalidPortsAnnotationError(id.clone(),e))
            })
            .unwrap_or(Err(KsServiceParsingError::MissingPortsAnnotationError(id)))?)
    }

    fn ks_get_state(&self) -> Result<StateKind, KsServiceInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsServiceInteractError::ServiceFetchingStateError(Identifier::new_unknow(), e)
        })?;
        Ok(
            if let Some(ks_selector) = self
                .ks_get_selector()
                .map_err(|e| KsServiceInteractError::ServiceFetchingStateError(id, e))?
                .get(KUBESLEEPER_SELECTOR_KEY)
                && ks_selector == KUBESLEEPER_SELECTOR_VALUE
            {
                StateKind::Asleep
            } else {
                StateKind::Awake
            },
        )
    }

    async fn ks_patch(
        &self,
        ports: &Vec<ServicePort>,
        target_ports: &Vec<ServicePort>,
        selector: &HashMap<String, String>,
        target_selector: &HashMap<String, String>,
    ) -> Result<(), KsServiceInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsServiceInteractError::ServiceFetchingStateError(Identifier::new_unknow(), e)
        })?;

        let store_selector = serde_json::to_string(&target_selector)
            .map_err(|e| KsServiceInteractError::ServiceCreateJsonPatchError(id.clone(), e))?;
        let store_ports = serde_json::to_string(&target_ports)
            .map_err(|e| KsServiceInteractError::ServiceCreateJsonPatchError(id.clone(), e))?;

        let patch = serde_json::json!({
            "spec" : {
                "selector": selector,
                "ports": ports
            },
            "metadata": {
                "annotations": {
                    format!("{KUBESLEEPER_ANNOTATION_PREFIX}{ANNOTATION_SELECTOR_KEY}"): store_selector,
                    format!("{KUBESLEEPER_ANNOTATION_PREFIX}{ANNOTATION_PORTS_KEY}"): store_ports
                }
            }
        });

        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);

        let client = Client::try_default()
            .await
            .map_err(|e| KsServiceInteractError::ServicePatchingError(id.clone(), e.into()))?;
        let api: Api<Service> = Api::namespaced(client, &id.namespace.to_string());
        api.patch(&id.name.to_string(), &params, &patch)
            .await
            .map_err(|e| KsServiceInteractError::ServicePatchingError(id, e.into()))?;
        Ok(())
    }

    async fn ks_wake(&self) -> Result<(), KsServiceInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsServiceInteractError::ServiceFetchingStateError(Identifier::new_unknow(), e)
        })?;

        if self.ks_get_state()? == StateKind::Awake {
            return Ok(());
        }

        let target_ports = self
            .ks_get_target_ports()
            .map_err(|e| KsServiceInteractError::ServiceWakingError(id.clone(), e))?;
        let target_selector = self
            .ks_get_target_selector()
            .map_err(|e| KsServiceInteractError::ServiceWakingError(id.clone(), e))?;

        self.ks_patch(
            &target_ports,
            &target_ports,
            &target_selector,
            &target_selector,
        ).await?;
        Ok(())
    }

    async fn ks_sleep(&self) -> Result<(), KsServiceInteractError> {
        let id = self.ks_id().map_err(|e| {
            KsServiceInteractError::ServiceFetchingStateError(Identifier::new_unknow(), e)
        })?;

        if self.ks_get_state()? == StateKind::Asleep {
            return Ok(());
        }

        let mut sleep_selector = HashMap::new();
        sleep_selector.insert(
            KUBESLEEPER_SELECTOR_KEY.to_string(),
            KUBESLEEPER_SELECTOR_VALUE.to_string(),
        );

        let ports = &self
            .ks_get_ports()
            .map_err(|e| KsServiceInteractError::ServiceWakingError(id.clone(), e))?;
        let mut sleep_ports = ports.clone();
        sleep_ports
            .iter_mut()
            .for_each(|sp| sp.target_port = Some(IntOrString::Int(KUBESLEEPER_SERVER_PORT)));

        self.ks_patch(
            &sleep_ports,
            &ports,
            &sleep_selector,
            &self
                .ks_get_selector()
                .map_err(|e| KsServiceInteractError::ServiceWakingError(id.clone(), e))?,
        ).await?;
        Ok(())
    }

    async fn get_all() -> Result<Vec<Service>, KsServiceFetchingError> {
        let lp = ListParams::default().match_any().fields(&format!(
            "metadata.name!={},metadata.name!=kubernetes,metadata.namespace!=kube-system",
            KUBESLLEPER_APP_NAME
        ));

        let client = Client::try_default().await?;
        let api: Api<Service> = Api::all(client);

        Ok(api.list(&lp).await?.into_iter().collect())
    }

    async fn get(id: Identifier) -> Result<Service, KsServiceFetchingError> {
        let lp = ListParams::default().match_any().fields(&format!(
            "metadata.name!={},metadata.namespace!=kube-system,metadata.namespace=={},metadata.name=={}",
            KUBESLLEPER_APP_NAME,
            id.namespace,
            id.name
        ));

        let client = Client::try_default().await?;
        let api: Api<Service> = Api::all(client);

        Ok(api
            .list(&lp)
            .await?
            .into_iter()
            .collect::<Vec<Service>>()
            .first()
            .map(|d| d.clone())
            .ok_or(KsServiceFetchingError::ServiceNotFoundError(id))?)
    }
}
