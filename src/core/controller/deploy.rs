use k8s_openapi::api::apps::v1::Deployment;
use kube::api::{ListParams, Patch, PatchParams};
use kube::runtime::reflector::Lookup;
use kube::{Api, Client, ResourceExt};
use serde::Serialize;

use crate::core::controller::constantes::*;
use crate::core::controller::error;
use crate::core::state::state_kind::StateKind;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Serialize)]
pub struct Deploy {
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
            name,
            namespace,
            replicas,
            store_replicas,
            store_state,
        })
    }
}
impl Deploy {
    async fn get_k8s_api(namespace: &str) -> Result<Api<Deployment>, error::Controller> {
        let client = Client::try_default().await?;

        let deployments: Api<Deployment> = Api::namespaced(client, namespace);
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
        Deploy::get_k8s_api(&self.namespace)
            .await?
            .patch(&self.name, &params, &patch)
            .await?;

        Ok(())
    }
}

impl Deploy {
    pub async fn wake(&mut self) -> Result<(), error::Controller> {
        if self.store_state == Some(StateKind::Awake) {
            println!(
                "State already marked as '{}', skipping wake action",
                StateKind::Awake.to_string()
            );
            return Ok(());
        }

        self.replicas = self
            .store_replicas
            .ok_or(error::ResourceParse::MissingValue {
                id: format!("{}/{}", self.name, self.namespace),
                value: format!(
                    "{}{}",
                    KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_STORE_REPLICAS_KEY
                ),
            })?;
        self.store_state = Some(StateKind::Awake);
        self.patch().await
    }

    pub async fn sleep(&mut self) -> Result<(), error::Controller> {
        if self.store_state == Some(StateKind::Asleep) {
            println!(
                "State already marked as '{}', skipping sleep action",
                StateKind::Asleep.to_string()
            );
            return Ok(());
        }

        self.store_replicas = Some(self.replicas);
        self.replicas = 0;
        self.store_state = Some(StateKind::Asleep);
        self.patch().await
    }

    pub async fn get_all(namespace: &str) -> Result<Vec<Deploy>, error::Controller> {
        let mut res: Vec<Deploy> = vec![];
        let mut is_kubesleeper_deploy_detected = false;

        let deploys = Deploy::get_k8s_api(namespace)
            .await?
            .list(&ListParams::default())
            .await?;

        for deploy in deploys.iter() {
            if deploy
                .metadata
                .labels
                .as_ref()
                .unwrap_or(&BTreeMap::new())
                .get(KUBESLEEPER_SERVER_LABEL_KEY)
                != Some(&KUBESLEEPER_SERVER_LABEL_VALUE.to_string())
            {
                match Deploy::try_from(deploy) {
                    Ok(d) => res.push(d),
                    Err(e) => return Err(e),
                }
            } else {
                if is_kubesleeper_deploy_detected { // if one kubesleeper deploy has already been detected
                    return Err(error::Controller::TooMuchKubesleeperDeploy);
                }
                is_kubesleeper_deploy_detected = true;
            }
        }
        if !is_kubesleeper_deploy_detected {
            return Err(error::Controller::MissingKubesleeperDeploy);
        }
        Ok(res)
    }

    pub async fn change_all_state(state: StateKind) -> Result<(), error::Controller> {
        let deploys = Deploy::get_all("ks").await?;

        for mut deploy in deploys {
            match state {
                StateKind::Asleep => deploy.sleep().await?,
                StateKind::Awake => deploy.wake().await?,
            }
        }
        Ok(())
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
