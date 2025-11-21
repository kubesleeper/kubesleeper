use crate::core::controller::{annotations::Annotations, constantes::*};

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
use tracing::{debug, info};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    fmt::{Display, Formatter},
    string::ToString,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServicePort {
    pub port: i32,
    #[serde(rename = "targetPort")]
    pub target_port: IntOrString,
}

// impl Display for ServicePort {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "{{port: {}, target_port: {}}}",
//             self.port,
//             match &self.target_port {
//                 IntOrString::Int(target_port) => target_port.to_string(),
//                 IntOrString::String(target_port) => format!("\"{}\"", target_port),
//             }
//         )
//     }
// }

#[derive(Debug, Serialize)]
pub struct Service {
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

impl Service {
    async fn get_k8s_api(namespace: &str) -> Result<Api<K8sService>, error::Controller> {
        let client = Client::try_default().await?;

        let services: Api<K8sService> = Api::namespaced(client, namespace);
        Ok(services)
    }

    async fn patch(&self) -> Result<(), error::Controller> {
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
        Service::get_k8s_api(&self.namespace)
            .await?
            .patch(&self.name, &params, &patch)
            .await?;

        Ok(())
    }
}

impl Service {
    pub async fn wake(&mut self) -> Result<(), error::Controller> {
        if self.store_state == Some(StateKind::Awake) {
            debug!(
                "State of service '{}/{}' already marked as '{}', skipping wake action",
                self.name, self.namespace,
                StateKind::Awake.to_string()
            );
            return Ok(());
        }

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

    pub async fn sleep(&mut self) -> Result<(), error::Controller> {
        if self.store_state == Some(StateKind::Asleep) {
            debug!(
                "State of service '{}/{}' already marked as '{}', skipping wake action",
                self.name, self.namespace,
                StateKind::Awake.to_string()
            );
            return Ok(());
        }

        self.store_selector = Some(self.selector.clone());
        self.store_ports = Some(self.ports.clone());

        // point to server
        self.selector = HashMap::new();
        self.selector.insert(
            KUBESLEEPER_SERVER_SELECTOR_KEY.to_string(),
            KUBESLEEPER_SERVER_SELECTOR_VALUE.to_string(),
        );

        self.ports
            .iter_mut()
            .for_each(|sp| sp.target_port = IntOrString::Int(KUBESLEEPER_SERVER_PORT));

        self.store_state = Some(StateKind::Asleep);
        self.patch().await
    }

    pub async fn get_all_target(namespace: &str) -> Result<Vec<Service>, error::Controller> {
        Service::get_k8s_api(namespace)
            .await?
            .list(&ListParams::default())
            .await?
            .iter()
            .map(|d| Service::try_from(d))
            .collect()
    }

    pub async fn change_all_state(state: StateKind) -> Result<(), error::Controller> {
        let services = Service::get_all_target("ks").await?;
        info!("Set {} services {:?}",services.len(),state);
        for mut service in services {
            debug!("Set service {}/{} {:?}",service.name, service.namespace,state);
            match state {
                StateKind::Asleep => service.sleep().await?,
                StateKind::Awake => service.wake().await?,
            }
        }
        Ok(())
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
    type Error = error::Controller;

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

        let service_id = format!("{name}/{namespace}");

        let selector: HashMap<String, String> = service
            .spec
            .as_ref()
            .and_then(|s| s.selector.as_ref())
            .ok_or_else(|| error::ResourceParse::MissingValue {
                id: format!("{service_id}"),
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
                id: format!("{service_id}"),
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
                        id: format!("{service_id}"),
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
                        id: format!("{service_id}"),
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
