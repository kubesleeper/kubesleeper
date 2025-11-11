use crate::core::controller::annotations::Annotations;
use crate::core::controller::constantes::*;

use crate::core::state::state_kind::StateKind;

use super::error::ControllerError;
use k8s_openapi::api::core::v1::Service as K8sService;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::{ListParams, Patch, PatchParams};
use kube::runtime::reflector::Lookup;
use kube::{Api, Client, ResourceExt};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::string::ToString;

pub struct Service {
    pub name: String,
    pub namespace: String,
    pub selector: HashMap<String, String>,
    pub store_selector: HashMap<String, String>,

    // using i32 as key and not name cause name is optional.
    // IntOrString cause it could be map 80:myPort service side,
    // to match a port named myPort in target container side
    pub ports: Vec<(i32, IntOrString)>,
    pub store_ports: Vec<(i32, IntOrString)>,

    pub store_state: StateKind,
}

impl Service {
    async fn get_k8s_api(namespace: &str) -> Result<Api<K8sService>, ControllerError> {
        let client = Client::try_default().await?;

        let services: Api<K8sService> = Api::namespaced(client, namespace);
        Ok(services)
    }
    
    async fn patch(&self) -> Result<(), ControllerError> {
        let patch = serde_json::json!({
            "spec" : {
                "selector": self.selector,
                "ports": self.ports
            },
            "metadata": {
                "annotations": {
                    ANNOTATION_STORE_STATE_KEY: self.store_state.to_string(),
                    ANNOTATION_STORE_SELECTOR_KEY: self.store_selector,
                    ANNOTATION_STORE_PORTS_KEY: self.store_ports
                }
            }
        });

        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        Service::get_k8s_api(&self.namespace).await?.patch(&self.name, &params, &patch).await?;

        Ok(())
    }
}

impl Service {
 
    pub async fn wake(&mut self) -> Result<(), ControllerError> {
        if self.store_state == StateKind::Asleep {
            println!(
                "State already marked as '{}', skipping wake action",
                StateKind::Awake.to_string()
            );
            return Ok(());
        }

        // point to target
        self.selector = self.store_selector.clone();

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

        self.store_selector = self.selector.clone();

        // point to server
        self.selector = HashMap::new();
        self.selector.insert(
            KUBESLEEPER_SERVER_SELECTOR_KEY.to_string(),
            KUBESLEEPER_SERVER_SELECTOR_VALUE.to_string(),
        );

        self.store_state = StateKind::Asleep;
        self.patch().await
    }

    
    pub async fn get_all(namespace: &str) -> Result<Vec<Service>, ControllerError> {
        Ok(Service::get_k8s_api(namespace).await?
            .list(&ListParams::default())
            .await?
            .iter()
            .filter_map(|d| Service::try_from(d).ok())
            .collect::<Vec<Service>>()
        )
    }
    
    pub async fn change_all_state(state: StateKind) -> Result<(), ControllerError>{
        let services = Service::get_all("ks").await?;
    
        for mut service in services {
            match state {
                StateKind::Asleep => { service.sleep().await? }
                StateKind::Awake  => { service.wake().await?  }
            }
        }
        Ok(())
    }

}

use std::fmt::Debug;

impl fmt::Display for Service {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn indent4<T: Debug>(val: &T) -> String {
            format!("{:?}", val)
                .lines()
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        }

        writeln!(f, "{}", self.name)?;
        writeln!(f, "  Namespace        : {}"  , indent4(&self.namespace))?;
        writeln!(f, "  Current Selector : {:?}", indent4(&self.selector))?;
        writeln!(f, "  Store Selector   : {:?}", indent4(&self.store_selector))?;
        writeln!(f, "  Current Ports    : {:?}", indent4(&self.store_ports))?;
        writeln!(f, "  Store Ports      : {:?}", indent4(&self.ports))?;
        writeln!(f, "  Current state    : {}"  , indent4(&self.store_state))?;
        Ok(())
    }
}

impl TryFrom<&K8sService> for Service {
    type Error = ControllerError;

    fn try_from(service: &K8sService) -> std::result::Result<Self, Self::Error> {
        // --- explicit data
        println!(">");
        let name = service
            .name()
            .ok_or_else(|| ControllerError::ResourceDataError("Missing 'name' field".to_string()))?
            .to_string();
        let namespace = ResourceExt::namespace(service).ok_or(
            ControllerError::ResourceDataError("Missing 'namespace' field".to_string()),
        )?;
        println!(">");

        let service_id = format!("{name} ({namespace})");
        
        let selector : HashMap<String, String> = service
            .spec
            .as_ref()
            .and_then(|s| s.selector.as_ref())
            .ok_or_else(|| {
                ControllerError::ResourceDataError("Can't parse Service {service_id} : Missing 'spec.selector' field".to_string())
            })?
            .clone()
            .into_iter()
            .collect();
        println!(">");
        let ports: Vec<(i32, IntOrString)> = service
            .spec
            .as_ref()
            .and_then(|s| s.ports.as_ref())
            .ok_or_else(|| {
                ControllerError::ResourceDataError("Can't parse Service {service_id} : Missing 'spec.ports' field".to_string())
            })?
            .clone()
            .into_iter()
            .map(|svc_port| {
                (
                    svc_port.port,
                    svc_port
                        .target_port
                        .unwrap_or(IntOrString::Int(svc_port.port)),
                )
            })
            .collect();
        println!(">");
        let raw_annotations = service.metadata.annotations.as_ref();
        let annotations = Annotations::from(raw_annotations.unwrap_or(&BTreeMap::default()));

        
        println!(">");
        // --- store annotation
        let default_state = match selector.get(KUBESLEEPER_SERVER_SELECTOR_KEY) {
            Some(_) => StateKind::Asleep.to_string(),
            None => StateKind::Awake.to_string(),
        };
        let raw_store_state: &str = annotations
            .get(ANNOTATION_STORE_STATE_KEY)
            .unwrap_or(&default_state)
            .as_str();
        let store_state = StateKind::try_from(raw_store_state).map_err(|err| {
            ControllerError::ResourceDataError(format!(
                "Can't parse Service {service_id} : Can't parse annotation '{}{}' : {}",
                KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_STATE_KEY, err
            ))
        })?;
        println!(">");
        let raw_store_selector: &str = annotations
            .get(ANNOTATION_STORE_SELECTOR_KEY)
            .ok_or(ControllerError::ResourceDataError(format!(
                "Can't parse Service {service_id} : Missing required annotation '{}{}'",
                KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_SELECTOR_KEY
            )))?
            .as_str();
        let store_selector: HashMap<String, String> = serde_json::from_str(raw_store_selector)?;

        let raw_store_ports: &str = annotations
            .get(ANNOTATION_STORE_PORTS_KEY)
            .ok_or(ControllerError::ResourceDataError(format!(
                "Can't parse Service {service_id} : Missing required annotation '{}{}'",
                KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_PORTS_KEY
            )))?
            .as_str();
        let store_ports: Vec<(i32, IntOrString)> = serde_json::from_str(raw_store_ports)?;
        println!(">");

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
