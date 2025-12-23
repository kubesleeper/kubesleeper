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
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub replicas: i32,

    pub store_replicas: i32,
}
impl TryFrom<&Deployment> for Deploy {
    type Error = error::Resource;

    fn try_from(deploy: &Deployment) -> std::result::Result<Self, Self::Error> {
        // --- explicit data

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

        // replicas
        let store_replicas = if replicas == 0 {
            annotations
                .get(ANNOTATION_STORE_REPLICAS_KEY)
                .map(|raw_store_replicas| {
                    raw_store_replicas.parse::<i32>().map_err(|err| {
                        error::ResourceParse::ParseFailed {
                            id: format!("{id}"),
                            value: format!(
                                ".annotations.{}{}",
                                KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_REPLICAS_KEY
                            ),
                            error: format!("{err}"),
                        }
                    })
                })
                .unwrap_or(Err(error::ResourceParse::MissingAnnotationInSleepState {
                    id: format!("{id}"),
                    annotation: format!(
                        "{}{}",
                        KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_REPLICAS_KEY
                    ),
                }))?
        } else {
            replicas
        };

        Ok(Deploy {
            id,
            name,
            namespace,
            replicas,
            store_replicas,
        })
    }
}

impl super::TargetResource<'static> for Deploy {
    type K8sResource = Deployment;

    fn is_asleep(&self) -> bool {
        self.replicas == 0
    }

    async fn wake(&mut self) -> Result<(), error::Resource> {
        // skip if resource as already a 'awake' stored state
        if !self.is_asleep() {
            debug!(
                "State of deployment '{}' already marked as '{}', skipping sleep action",
                self.id,
                StateKind::Awake
            );
            return Ok(());
        }

        // edit resource to set it in a awake state
        self.replicas = self.store_replicas;

        // patch related k8s resource
        self.patch().await?;

        self.wait_ready().await
    }

    async fn sleep(&mut self) -> Result<(), error::Resource> {
        // skip if resource as already a 'asleep' stored state

        if self.is_asleep() {
            debug!(
                "State of deployment '{}' already marked as '{}', skipping sleep action",
                self.id,
                StateKind::Asleep
            );
            return Ok(());
        }

        // edit resource to set it in a asleep state
        self.store_replicas = self.replicas;
        self.replicas = 0;

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

    async fn get_all() -> Result<Vec<Self>, error::Resource> {
        let lp = ListParams::default().match_any().fields(&format!(
            "metadata.name!={},metadata.namespace!=kube-system",
            KUBESLLEPER_APP_NAME
        ));

        Self::get_k8s_api(None)
            .await?
            .list(&lp)
            .await?
            .iter()
            .map(|d| Self::try_from(d))
            .collect()
    }

    async fn patch(&self) -> Result<(), error::Resource> {
        let patch = serde_json::json!({
            "spec" : {
                "replicas": self.replicas
            },
            "metadata": {
                "annotations": {
                    format!("{KUBESLEEPER_ANNOTATION_PREFIX}{ANNOTATION_STORE_REPLICAS_KEY}"): self.store_replicas.to_string()
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

    fn id(&self) -> String {
        return self.id.clone();
    }
}

use super::TargetResource;

impl Deploy {
    pub async fn get_ready_replicas_count(&self) -> Result<i32, error::Resource> {
        Ok(self
            .get_k8s_resource()
            .await?
            .status
            .ok_or(error::ResourceParse::MissingValue {
                id: format!("{}", self.id),
                value: "status".to_string(),
            })?
            .ready_replicas
            .unwrap_or_default())
    }

    pub async fn wait_ready(&self) -> Result<(), error::Resource> {
        let mut total_duration = 0;
        for i in 0_u32..1000 {
            let current_ready_replicas = self.get_ready_replicas_count().await?;
            if self.replicas - self.get_ready_replicas_count().await? == 0 {
                info!("Deploy {} just woke up.", self.id);
                return Ok(());
            }

            info!(
                "Deploy {} is waking up. Waiting for replicas to be ready : {}/{}",
                self.id, current_ready_replicas, self.replicas
            );

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

impl Deploy {
    /// Check if a (unique) kubesleeper deploy is found
    pub async fn check_kubesleeper() -> Result<(), error::Resource> {
        let kubesleeper_field_identifier = format!("metadata.name={}", KUBESLLEPER_APP_NAME);

        let lp = ListParams::default()
            .match_any()
            .fields(&kubesleeper_field_identifier);

        let kubesleeper = Deploy::get_k8s_api(None)
            .await?
            .list(&lp)
            .await
            .map_err(crate::core::resource::error::Resource::from)?;

        let nb_kubesleeper = kubesleeper.iter().count();
        if nb_kubesleeper > 1 {
            return Err(error::Resource::TooMuchKubesleeperDeploy(nb_kubesleeper));
        };

        let ks = match kubesleeper.into_iter().next() {
            Some(k) => Ok(k),
            None => Err(error::Resource::MissingKubesleeperDeploy),
        }?;

        let id = format!(
            "kubesleeper deployment ({}/{})",
            ks.metadata.namespace.as_deref().unwrap_or("?"),
            ks.metadata.name.as_deref().unwrap_or("?")
        );

        let labels = ks
            .metadata
            .labels
            .as_ref()
            .ok_or_else(
                || crate::core::resource::error::ResourceParse::MissingValue {
                    id: format!("kubesleeper ({id})"),
                    value: ".metadata.labels".to_string(),
                },
            )
            .map_err(crate::core::resource::error::Resource::from)?;

        if let Some(app_value) = labels.get(KUBESLEEPER_SELECTOR_KEY) {
            if app_value != KUBESLEEPER_SELECTOR_VALUE {
                return Err(error::ResourceParse::ParseFailed {
                    id: format!("kubesleeper ({id})"),
                    value: format!(".metadata.labels.{}", KUBESLEEPER_SELECTOR_KEY),
                    error: format!(
                        "Must be '{}' but found '{}'",
                        KUBESLEEPER_SELECTOR_VALUE, app_value
                    ),
                }
                .into());
            }
        } else {
            return Err(error::ResourceParse::MissingValue {
                id: format!("kubesleeper ({id})"),
                value: format!(".metadata.labels.{KUBESLEEPER_SELECTOR_KEY}"),
            }
            .into());
        };
        debug!("Kubesleeper deployment found : {id}");
        Ok(())
    }
}
