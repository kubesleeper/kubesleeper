use super::error::ControllerError;
use crate::core::controller::kube::KUBESLEEPER_ANNOTATION_PREFIX;
use k8s_openapi::api::core::v1::Service as K8sService;
use kube::api::{ListParams, Patch, PatchParams};
use kube::runtime::reflector::Lookup;
use kube::{Api, Client, ResourceExt};
use std::collections::HashMap;
use std::fmt;
use std::string::ToString;

const ANNOTATION_SELECTOR_KEY: &str = "store.selectors";
const SERVER_SELECTOR: (&str, &str) = ("app", "kubesleeper");

pub struct Service {
    pub annotations: HashMap<String, String>,
    pub name: String,
    pub namespace: String,
    pub selectors: HashMap<String, String>,
}

impl Service {
    async fn get_k8s_api(namespace: &str) -> Result<Api<K8sService>, ControllerError> {
        let client = Client::try_default().await?;

        let services: Api<K8sService> = Api::namespaced(client, namespace);
        return Ok(services);
    }

    pub async fn get_all() -> Result<Vec<Service>, ControllerError> {
        let services = Service::get_k8s_api("ks").await?;
        Ok(services
            .list(&ListParams::default())
            .await?
            .iter()
            .filter_map(|deployment| Service::try_from(deployment).ok())
            .collect())
    }

    async fn set_selectors(
        &self,
        selectors: &HashMap<String, String>,
    ) -> Result<(), ControllerError> {
        let services = Service::get_k8s_api(&self.namespace).await?;

        let patch = serde_json::json!({
            "selector": selectors
        });

        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        let _patched = services.patch(&self.name, &params, &patch).await?;
        Ok(())
    }

    pub async fn redirect_to_server(&self) -> Result<(), ControllerError> {
        // add store annotation (remember the targeted selectors)
        let services = Service::get_k8s_api(&self.namespace).await?;
        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                     format!(
                        "{}/{}",
                        KUBESLEEPER_ANNOTATION_PREFIX,
                        ANNOTATION_SELECTOR_KEY
                    ): serde_json::to_string(&self.selectors)?
                }
            }
        });
        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        services.patch(&self.name, &params, &patch).await?;

        self.set_selectors(&HashMap::from([(
            SERVER_SELECTOR.0.to_string(),
            SERVER_SELECTOR.1.to_string(),
        )]))
        .await?;
        Ok(())
    }

    pub async fn redirect_to_origin(&self) -> Result<(), ControllerError> {
        // fetch targeted replicas count thanks to store/ annotation
        let raw_selectors = self
            .annotations
            .get(ANNOTATION_SELECTOR_KEY)
            .ok_or_else(|| {
                ControllerError::ResourceDataError(format!(
                    "Missing required annotation '{}'",
                    ANNOTATION_SELECTOR_KEY
                ))
            })?;

        let selectors = &serde_json::from_str::<HashMap<String, String>>(raw_selectors)?;

        self.set_selectors(selectors).await?;

        // remove store annotation (forgetting the targeted selectors)
        let services = Service::get_k8s_api(&self.namespace).await?;
        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                     format!("{}/{}",KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_SELECTOR_KEY): null
                }
            }
        });
        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        services.patch(&self.name, &params, &patch).await?;

        Ok(())
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}",
            format!(
                r#"Name        : {}
Namespace   : {}
Selectors   : {}
Annotations : {}"#,
                self.name,
                self.namespace,
                self.selectors
                    .iter()
                    .map(|(k, v)| format!("\n  {}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n"),
                if self.annotations.is_empty() {
                    "-".to_string()
                } else {
                    self.annotations
                        .iter()
                        .map(|(k, v)| format!("\n  {}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            )
        )?;
        Ok(())
    }
}

impl TryFrom<&K8sService> for Service {
    type Error = ControllerError;

    fn try_from(service: &K8sService) -> std::result::Result<Self, Self::Error> {
        let raw_annotations = service.metadata.annotations.as_ref();
        let annotations = super::kube::extract_kube_annoations(raw_annotations);

        let name = service
            .name()
            .ok_or(ControllerError::ResourceDataError(
                "Missing 'name' field".to_string(),
            ))?
            .to_string();
        let namespace = ResourceExt::namespace(service).ok_or(
            ControllerError::ResourceDataError("Missing 'namespace' field".to_string()),
        )?;

        let selectors: HashMap<String, String> = service
            .spec
            .as_ref()
            .and_then(|s| s.selector.clone())
            .ok_or(ControllerError::ResourceDataError(
                "Missing selector values".to_string(),
            ))?
            .into_iter()
            .collect();

        Ok(Service {
            annotations,
            name,
            namespace,
            selectors,
        })
    }
}
