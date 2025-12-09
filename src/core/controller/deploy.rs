use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    Api, Client, ResourceExt,
    api::{ListParams, Patch, PatchParams},
    runtime::reflector::Lookup,
};
use serde::Serialize;
use tracing::{debug, info};

use crate::core::controller::deploy;
use crate::core::state::StateError::Controller;
use crate::core::{
    controller::{constantes::*, error},
    state::state_kind::StateKind,
};
use rocket::futures::future::err;
use std::time::Duration;
use std::{collections::BTreeMap, fmt, thread};

#[derive(Serialize)]
pub struct Deploy {
    pub uid: String,
    pub name: String,
    pub namespace: String,
    pub replicas: i32,

    #[serde(rename = "stored state")]
    pub store_state: Option<StateKind>,
    #[serde(rename = "stored replicas")]
    pub store_replicas: Option<i32>,
}
impl TryFrom<&Deployment> for Deploy {
    type Error = error::Controller;

    fn try_from(deploy: &Deployment) -> std::result::Result<Self, Self::Error> {
        // --- explicit data
        let uid = ResourceExt::uid(deploy).ok_or(error::ResourceParse::MissingValue {
            id: format!("?"),
            value: "uid".to_string(),
        })?;

        let name = deploy
            .name()
            .ok_or(error::ResourceParse::MissingValue {
                id: "?/?".to_string(),
                value: "name".to_string(),
            })?
            .to_string();

        let namespace =
            ResourceExt::namespace(deploy).ok_or(error::ResourceParse::MissingValue {
                id: format!("{name}"),
                value: "namespace".to_string(),
            })?;
        let deploy_id = format!("{namespace}/{name}");

        let replicas = deploy.spec.as_ref().and_then(|s| s.replicas).ok_or(
            error::ResourceParse::MissingValue {
                id: format!("{deploy_id}"),
                value: ".spec".to_string(),
            },
        )?;

        // --- store annotation
        let raw_annotations = deploy.metadata.annotations.as_ref();
        let annotations =
            super::annotations::Annotations::from(raw_annotations.unwrap_or(&BTreeMap::default()));

        // state
        let store_state = annotations
            .get(ANNOTATION_STORE_STATE_KEY)
            .map(|raw_store_state| {
                StateKind::try_from(raw_store_state).map_err(|err| {
                    error::ResourceParse::ParseFailed {
                        id: format!("{deploy_id}"),
                        value: format!(
                            ".annotations.{}{}",
                            KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_STATE_KEY
                        ),
                        error: format!("{err}"),
                    }
                })
            })
            .transpose()?;

        // replicas
        let store_replicas = annotations
            .get(ANNOTATION_STORE_REPLICAS_KEY)
            .map(|raw_store_replicas| {
                raw_store_replicas
                    .parse::<i32>()
                    .map_err(|err| error::ResourceParse::ParseFailed {
                        id: format!("{deploy_id}"),
                        value: format!(
                            ".annotations.{}{}",
                            KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_STATE_KEY
                        ),
                        error: format!("{err}"),
                    })
            })
            .transpose()?;
        Ok(Deploy {
            uid,
            name,
            namespace,
            replicas,
            store_replicas,
            store_state,
        })
    }
}
impl Deploy {
    async fn get_k8s_api(namespace: Option<&str>) -> Result<Api<Deployment>, error::Controller> {
        let client = Client::try_default().await?;

        let deployments: Api<Deployment> = if let Some(namespace_name) = namespace {
            Api::namespaced(client, namespace_name)
        } else {
            Api::all(client)
        };

        return Ok(deployments);
    }

    async fn patch(&self) -> Result<(), error::Controller> {
        let patch = serde_json::json!({
            "spec" : {
                "replicas": self.replicas
            },
            "metadata": {
                "annotations": {
                    format!("{KUBESLEEPER_ANNOTATION_PREFIX}{ANNOTATION_STORE_STATE_KEY}"): &self.store_state,
                    format!("{KUBESLEEPER_ANNOTATION_PREFIX}{ANNOTATION_STORE_REPLICAS_KEY}"): self.store_replicas
                        .expect("Logically store_replicas must be a Some at this stage")
                        .to_string()
                }
            }
        });
        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        Deploy::get_k8s_api(Some(&self.namespace))
            .await?
            .patch(&self.name, &params, &patch)
            .await?;

        Ok(())
    }
}

impl Deploy {
    pub async fn wake(&mut self) -> Result<(), error::Controller> {
        if self.store_state == Some(StateKind::Awake) {
            debug!(
                "State of deployment '{}/{}' already marked as '{}', skipping sleep action",
                self.name,
                self.namespace,
                StateKind::Awake.to_string()
            );
            return Ok(());
        }

        self.replicas = self
            .store_replicas
            .ok_or(error::ResourceParse::MissingValue {
                id: format!("{}/{}", self.namespace, self.name),
                value: format!(
                    "{}{}",
                    KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_REPLICAS_KEY
                ),
            })?;
        self.store_state = Some(StateKind::Awake);
        self.patch().await?;

        self.wait_ready().await
    }

    pub async fn sleep(&mut self) -> Result<(), error::Controller> {
        if self.store_state == Some(StateKind::Asleep) {
            debug!(
                "State of deployment '{}/{}' already marked as '{}', skipping sleep action",
                self.namespace,
                self.name,
                StateKind::Asleep.to_string()
            );
            return Ok(());
        }

        self.store_replicas = Some(self.replicas);
        self.replicas = 0;
        self.store_state = Some(StateKind::Asleep);
        self.patch().await
    }
    async fn get_all_k8s_deployments(
        namespace: Option<&str>,
    ) -> Result<Vec<Deployment>, error::Controller> {
        Ok(Deploy::get_k8s_api(namespace)
            .await?
            .list(&ListParams::default())
            .await?
            .into_iter()
            .collect())
    }

    pub async fn get_kubesleeper() -> Result<Deploy, error::Controller> {
        debug!("Fetching kubesleeper deployment");
        let ks_deploys: Vec<Deployment> = Self::get_all_k8s_deployments(None)
            .await?
            .into_iter()
            .filter(|deploy| {
                deploy
                    .metadata
                    .labels
                    .as_ref()
                    .unwrap_or(&BTreeMap::new())
                    .get(KUBESLEEPER_SERVER_LABEL_KEY)
                    == Some(&KUBESLEEPER_SERVER_LABEL_VALUE.to_string())
            })
            .collect();
        match ks_deploys.len() {
            0 => Err(error::Controller::MissingKubesleeperDeploy),
            1 => {
                let ks = Self::try_from(
                    ks_deploys
                        .get(0)
                        .expect("Deploys should logically have exactly 1 element at this point"),
                )?;
                debug!(
                    "kubesleeper deployment found : {}/{} ({})",
                    ks.namespace, ks.name, ks.uid
                );
                Ok(ks)
            }
            x => Err(error::Controller::TooMuchKubesleeperDeploy(x)),
        }
    }

    pub async fn get_all_target() -> Result<Vec<Deploy>, error::Controller> {
        let deploys = Self::get_all_k8s_deployments(Some(KUBESLEEPER_NAMESPACE.get().unwrap()))
            .await?
            .iter()
            .map(|deploy| Self::try_from(deploy))
            .collect::<Result<Vec<Deploy>, error::Controller>>()?;
        let ks_deploy = Self::get_kubesleeper().await?;

        Ok(deploys
            .into_iter()
            .filter(|deploy| deploy.uid != ks_deploy.uid)
            .collect())
    }

    pub async fn change_all_state(state: StateKind) -> Result<(), error::Controller> {
        let deploys = Deploy::get_all_target().await?;
        info!("Set {} deployments {:?}", deploys.len(), state);
        for mut deploy in deploys {
            debug!(
                "Set deployment {}/{} {:?}",
                deploy.namespace, deploy.name, state
            );
            match state {
                StateKind::Asleep => deploy.sleep().await?,
                StateKind::Awake => deploy.wake().await?,
            }
        }
        Ok(())
    }

    async fn is_ready(&self) -> Result<bool, error::Controller> {
        let id = format!("{}/{}", self.namespace, self.name);

        let deploy = Self::get_all_k8s_deployments(Some(KUBESLEEPER_NAMESPACE.get().unwrap()))
            .await?
            .into_iter()
            .find(|deploy| self.uid == ResourceExt::uid(deploy).unwrap_or_default())
            .ok_or(error::Controller::RetrieveK8S {
                id: format!("{id}"),
            })?;

        let current_ready_replicas = deploy
            .status
            .ok_or(error::ResourceParse::MissingValue {
                id: format!("{id}"),
                value: "status".to_string(),
            })?
            .ready_replicas.unwrap_or(-1); // Prevent case where store_replicas is 0

        let store_replicas = self.store_replicas.unwrap_or_default();

        match store_replicas - current_ready_replicas {
            0 => {
                info!("Deploy {id} just woke up.");
                Ok(true)
            }
            _ => {
                info!(
                    "Deploy {id} is waking up. Waiting for replicas to be ready : {current_ready_replicas}/{store_replicas}",
                );
                Ok(false)
            }
        }
    }

    pub async fn wait_ready(&self) -> Result<(), error::Controller> {
        for i in 0_u32..16 {
            if self.is_ready().await? {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(2_u64.pow(i))).await;
        }

        Err(error::Controller::MaxWaitingWakeTime {
            id: format!("{}/{}", self.namespace, self.name),
        })
    }
}

// enum Ready {
//     True,
//     False(i32),
// }

// pub async fn is_all_deploys_ready(deploys: Vec<Deploy>) -> Result<bool, error::Controller> {
//     let mut all_deploys_ready = true;
//     for deploy in deploys {
//         match deploy.is_ready().await? {
//             Ready::True => (),
//             _ => {all_deploys_ready = false; info!("Deploy {}/{} is waking up.", deploy.namespace, deploy.name);}
//         }
//     }
//
//     Ok(all_deploys_ready)
// }

impl fmt::Display for Deploy {
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
