use crate::core::resource::TargetResource;
use crate::core::resource::{annotations::Annotations, constantes::*};

use crate::core::state::state_kind::StateKind;

use super::error;
use k8s_openapi::{
    api::core::v1::Service as K8sService, apimachinery::pkg::util::intstr::IntOrString,
};
use kube::{
    Api, Client, ResourceExt,
    api::{ListParams, Patch, PatchParams},
    runtime::reflector::Lookup,
};
use rocket::serde::Deserialize;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    string::ToString,
};
use tracing::debug;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServicePort {
    pub port: i32,
    #[serde(rename = "targetPort")]
    pub target_port: IntOrString,
}

#[derive(Debug, Serialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub selector: HashMap<String, String>,

    // using i32 as key and not name cause name is optional.
    // IntOrString cause it could be map 80:myPort service side,
    // to match a port named myPort in target container side
    pub ports: Vec<ServicePort>,

    #[serde(rename = "stored state")]
    pub store_state: Option<StateKind>,
    #[serde(rename = "stored selector")]
    pub store_selector: Option<HashMap<String, String>>,
    #[serde(rename = "stored ports")]
    pub store_ports: Option<Vec<ServicePort>>,
}

impl TargetResource<'static> for Service {
    type Resource = Service;
    type K8sResource = K8sService;

    async fn wake(&mut self) -> Result<(), error::Resource> {
        // skip if resource as already a 'awake' stored state
        if self.store_state == Some(StateKind::Awake) {
            debug!(
                "State of service '{}' already marked as '{}', skipping wake action",
                self.id,
                StateKind::Awake.to_string()
            );
            return Ok(());
        }

        // edit resource to set it in a awake state

        // point to target
        self.selector = self
            .store_selector
            .clone()
            .ok_or(error::ResourceParse::MissingValue {
                id: format!("{}/{}", self.name, self.namespace),
                value: format!(
                    ".annotations.{}/{}",
                    KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_SELECTOR_KEY
                ),
            })?;

        self.ports = self
            .store_ports
            .clone()
            .ok_or(error::ResourceParse::MissingValue {
                id: format!("{}/{}", self.name, self.namespace),
                value: format!(
                    ".annotations.{}/{}",
                    KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_PORTS_KEY
                ),
            })?;

        self.store_state = Some(StateKind::Awake);
        self.patch().await
    }

    async fn sleep(&mut self) -> Result<(), error::Resource> {
        // skip if resource as already a 'asleep' stored state
        if self.store_state == Some(StateKind::Asleep) {
            debug!(
                "State of service '{}' already marked as '{}', skipping wake action",
                self.id,
                StateKind::Awake.to_string()
            );
            return Ok(());
        }

        // edit resource to set it in a asleep state
        self.store_selector = Some(self.selector.clone());
        self.store_ports = Some(self.ports.clone());
        self.store_state = Some(StateKind::Asleep);

        self.selector = HashMap::new();
        self.selector.insert(
            KUBESLEEPER_SERVER_SELECTOR_KEY.to_string(),
            KUBESLEEPER_SERVER_SELECTOR_VALUE.to_string(),
        );

        self.ports
            .iter_mut()
            .for_each(|sp| sp.target_port = IntOrString::Int(KUBESLEEPER_SERVER_PORT));

        self.patch().await
    }

    async fn patch(&self) -> Result<(), error::Resource> {
        let store_selector = serde_json::to_string(
            self.store_selector
                .as_ref()
                .expect("Logically store_selector must be a Some at this stage"),
        )?;
        let store_ports = serde_json::to_string(
            self.store_ports
                .as_ref()
                .expect("Logically store_ports must be a Some at this stage"),
        )?;

        let patch = serde_json::json!({
            "spec" : {
                "selector": self.selector,
                "ports": self.ports
            },
            "metadata": {
                "annotations": {
                    format!("{KUBESLEEPER_ANNOTATION_PREFIX}{ANNOTATION_STORE_STATE_KEY}"): &self.store_state,
                    format!("{KUBESLEEPER_ANNOTATION_PREFIX}{ANNOTATION_STORE_SELECTOR_KEY}"): store_selector,
                    format!("{KUBESLEEPER_ANNOTATION_PREFIX}{ANNOTATION_STORE_PORTS_KEY}"): store_ports
                }
            }
        });

        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        Service::get_k8s_api(Some(&self.namespace))
            .await?
            .patch(&self.name, &params, &patch)
            .await?;

        Ok(())
    }

    async fn get_k8s_api(
        namespace: Option<&str>,
    ) -> Result<Api<Self::K8sResource>, error::Resource> {
        let client = Client::try_default().await?;

        let deployments: Api<Self::K8sResource> = if let Some(namespace_name) = namespace {
            Api::namespaced(client, namespace_name)
        } else {
            Api::all(client)
        };

        return Ok(deployments);
    }

    async fn get_all() -> Result<Vec<Self::Resource>, error::Resource> {
        Self::get_k8s_api(None)
            .await?
            .list(&ListParams::default())
            .await?
            .iter()
            .map(|d| Self::Resource::try_from(d))
            .collect()
    }

    async fn get_k8s_resource(&self) -> Result<Self::K8sResource, error::Resource> {
        let lp = ListParams::default()
            .match_any()
            .fields(&format!("metadata.name={}", self.name));

        Self::get_k8s_api(Some(&self.namespace))
            .await?
            .list(&lp)
            .await?
            .into_iter()
            .next()
            .ok_or(error::Resource::K8sResourceNotFound {
                id: self.id.clone(),
            })
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}",
            serde_yaml::to_string(&self)
                .unwrap_or_else(|e| format!(
                    "{e} : The structure should always be serializable at this point"
                ))
                .trim()
        )?;
        Ok(())
    }
}

impl TryFrom<&K8sService> for Service {
    type Error = error::Resource;

    fn try_from(service: &K8sService) -> std::result::Result<Self, Self::Error> {
        // --- explicit data
        let name = service
            .name()
            .ok_or_else(|| error::ResourceParse::MissingValue {
                id: format!("?/?"),
                value: format!("name"),
            })?
            .to_string();
        let namespace =
            ResourceExt::namespace(service).ok_or(error::ResourceParse::MissingValue {
                id: format!("{name}/?"),
                value: format!("namespace"),
            })?;

        let id = format!("{name}/{namespace}");

        let selector: HashMap<String, String> = service
            .spec
            .as_ref()
            .and_then(|s| s.selector.as_ref())
            .ok_or_else(|| error::ResourceParse::MissingValue {
                id: format!("{id}"),
                value: format!(".spec.selector"),
            })?
            .clone()
            .into_iter()
            .collect();

        let ports: Vec<ServicePort> = service
            .spec
            .as_ref()
            .and_then(|s| s.ports.as_ref())
            .ok_or_else(|| error::ResourceParse::MissingValue {
                id: format!("{id}"),
                value: format!(".spec.ports"),
            })?
            .clone()
            .into_iter()
            .map(|svc_port| ServicePort {
                port: svc_port.port,
                target_port: svc_port
                    .target_port
                    .unwrap_or(IntOrString::Int(svc_port.port)),
            })
            .collect();
        let raw_annotations = service.metadata.annotations.as_ref();
        let annotations = Annotations::from(raw_annotations.unwrap_or(&BTreeMap::default()));

        // --- store annotation
        let store_state = annotations
            .get(ANNOTATION_STORE_STATE_KEY)
            .map(|raw_store_state| {
                StateKind::try_from(raw_store_state).map_err(|_| {
                    error::ResourceParse::MissingValue {
                        id: format!("{id}"),
                        value: format!(
                            ".annotations.{}{}",
                            KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_STATE_KEY
                        ),
                    }
                })
            })
            .transpose()?;

        let store_selector = annotations
            .get(ANNOTATION_STORE_SELECTOR_KEY)
            .map(|raw_store_selector| {
                serde_json::from_str(raw_store_selector).map_err(|err| {
                    error::ResourceParse::ParseFailed {
                        id: format!("{id}"),
                        value: format!(
                            ".annotation.{}{}",
                            KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_SELECTOR_KEY
                        ),
                        error: format!("{err}"),
                    }
                })
            })
            .transpose()?;

        let store_ports = annotations
            .get(ANNOTATION_STORE_PORTS_KEY)
            .map(|raw_store_ports| {
                serde_json::from_str(raw_store_ports).map_err(|err| {
                    error::ResourceParse::ParseFailed {
                        id: format!("service_id"),
                        value: format!(
                            ".annotations.{}{}",
                            KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_PORTS_KEY
                        ),
                        error: format!("{err}"),
                    }
                })
            })
            .transpose()?;

        Ok(Service {
            id,
            name,
            namespace,
            selector,
            ports,
            store_selector,
            store_ports,
            store_state,
        })
    }
}
