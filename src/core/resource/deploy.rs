use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    Api, Client, ResourceExt,
    api::{ListParams, Patch, PatchParams},
    runtime::reflector::Lookup,
};
use serde::Serialize;
use tracing::debug;

use crate::core::{
    resource::{constantes::*, error},
    state::state_kind::StateKind,
};
use std::time::Duration;
use std::{collections::BTreeMap, fmt};
use tracing::log::info;

#[derive(Serialize)]
pub struct Deploy {
    pub uid: String,
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub replicas: i32,

    #[serde(rename = "stored state")]
    pub store_state: Option<StateKind>,
    #[serde(rename = "stored replicas")]
    pub store_replicas: Option<i32>,
}
impl TryFrom<&Deployment> for Deploy {
    type Error = error::Resource;

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

        let id = format!("{namespace}/{name}");

        let replicas = deploy.spec.as_ref().and_then(|s| s.replicas).ok_or(
            error::ResourceParse::MissingValue {
                id: format!("{id}"),
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
                        id: format!("{id}"),
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
                        id: format!("{id}"),
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
            id,
            name,
            namespace,
            replicas,
            store_replicas,
            store_state,
        })
    }
}

impl super::TargetResource<'static> for Deploy {
    type Resource = Deploy;
    type K8sResource = Deployment;

    async fn wake(&mut self) -> Result<(), error::Resource> {
        // skip if resource as already a 'awake' stored state
        if self.store_state == Some(StateKind::Awake) {
            debug!(
                "State of deployment '{}' already marked as '{}', skipping sleep action",
                self.id,
                StateKind::Awake
            );
            return Ok(());
        }

        // edit resource to set it in a awake state
        self.replicas = self
            .store_replicas
            .ok_or(error::ResourceParse::MissingValue {
                id: format!("{}", self.id),
                value: format!(
                    "{}{}",
                    KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_REPLICAS_KEY
                ),
            })?;
        self.store_state = Some(StateKind::Awake);

        // patch related k8s resource
        self.patch().await?;

        self.wait_ready().await
    }

    async fn sleep(&mut self) -> Result<(), error::Resource> {
        // skip if resource as already a 'asleep' stored state
        if self.store_state == Some(StateKind::Asleep) {
            debug!(
                "State of deployment '{}' already marked as '{}', skipping sleep action",
                self.id,
                StateKind::Asleep
            );
            return Ok(());
        }

        // edit resource to set it in a asleep state
        self.store_replicas = Some(self.replicas);
        self.replicas = 0;
        self.store_state = Some(StateKind::Asleep);

        // patch related k8s resource
        self.patch().await
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
        let lp = ListParams::default()
            .match_any()
            .fields(&format!("metadata.name!={}", KUBESLLEPER_APP_NAME));

        Self::get_k8s_api(None)
            .await?
            .list(&lp)
            .await?
            .iter()
            .map(|d| Deploy::try_from(d))
            .collect()
    }

    async fn patch(&self) -> Result<(), error::Resource> {
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
        Self::get_k8s_api(Some(&self.namespace))
            .await?
            .patch(&self.name, &params, &patch)
            .await?;

        Ok(())
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

use super::TargetResource;

impl Deploy {
    #[allow(dead_code)]
    pub async fn check_kubesleeper() -> Result<Deployment, error::Resource> {
        let lp = ListParams::default()
            .match_any()
            .fields(&format!("metadata.name={}", KUBESLLEPER_APP_NAME));

        let kubesleeper = Self::get_k8s_api(None).await?.list(&lp).await?;
        let nb_kubesleeper = kubesleeper.iter().count();
        if nb_kubesleeper > 1 {
            return Err(error::Resource::TooMuchKubesleeperDeploy(nb_kubesleeper));
        };
        kubesleeper
            .into_iter()
            .next()
            .ok_or(error::Resource::MissingKubesleeperDeploy)
    }

    async fn is_ready(&self) -> Result<bool, error::Resource> {
        let id = format!("{}/{}", self.namespace, self.name);

        let current_ready_replicas = self
            .get_k8s_resource()
            .await?
            .status
            .ok_or(error::ResourceParse::MissingValue {
                id: format!("{id}"),
                value: "status".to_string(),
            })?
            .ready_replicas
            .unwrap_or_default();
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

    pub async fn wait_ready(&self) -> Result<(), error::Resource> {
        let mut total_duration = 0;
        for i in 0_u32..1000 {
            if self.is_ready().await? {
                return Ok(());
            }

            let duration = 100 * 2_u64.pow([i, 7].into_iter().min().expect("Couldn't be empty"));
            tokio::time::sleep(Duration::from_millis(duration)).await;
            total_duration += duration;
        }

        Err(error::Resource::MaxWaitingWakeTime {
            id: format!("{}", self.id),
            max_waiting_time: total_duration,
        })
    }
}

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
