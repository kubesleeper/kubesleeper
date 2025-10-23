use indoc::indoc;
use k8s_openapi::api::apps::v1::Deployment;
use kube::api::{ListParams, Patch, PatchParams};
use kube::runtime::reflector::Lookup;
use kube::{Api, Client, ResourceExt};
use std::collections::HashMap;
use std::fmt;

use super::{error::ControllerError, kube::KUBESLEEPER_ANNOTATION_PREFIX};

const ANNOTATION_REPLICAS_KEY: &str = "store.replicas";

pub struct Deploy {
    pub annotations: HashMap<String, String>,
    pub name: String,
    pub namespace: String,
}

impl Deploy {
    pub async fn set_all_asleep() -> Result<(), ControllerError> {
        let deploys = Deploy::get_all().await?;
        for deploy in deploys {
            deploy.sleep().await?
        }
        Ok(())
    }
    pub async fn set_all_awake() -> Result<(), ControllerError> {
        let deploys = Deploy::get_all().await?;
        for deploy in deploys {
            deploy.wake().await?
        }
        Ok(())
    }

    pub async fn get_all() -> Result<Vec<Deploy>, ControllerError> {
        let deploys = Deploy::get_k8s_api("ks").await?;
        Ok(deploys
            .list(&ListParams::default())
            .await?
            .iter()
            .filter_map(|deployment| Deploy::try_from(deployment).ok())
            .collect())
    }

    async fn get_k8s_api(namespace: &str) -> Result<Api<Deployment>, ControllerError> {
        let client = Client::try_default().await?;

        let deployments: Api<Deployment> = Api::namespaced(client, namespace);
        return Ok(deployments);
    }

    async fn set_replicas(&self, count: &i32) -> Result<(), ControllerError> {
        let deployments = Deploy::get_k8s_api(&self.namespace).await?;

        let patch = serde_json::json!({
            "spec": {
                "replicas": count
            }
        });
        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        let _patched = deployments.patch(&self.name, &params, &patch).await?;
        Ok(())
    }

    pub async fn wake(&self) -> Result<(), ControllerError> {
        // fetch targeted replicas count thanks to store/ annotation
        let count: i32 = self
            .annotations
            .get(ANNOTATION_REPLICAS_KEY)
            .ok_or_else(|| {
                ControllerError::ResourceDataError(format!(
                    "Missing required annotation '{}'",
                    ANNOTATION_REPLICAS_KEY
                ))
            })?
            .parse()
            .map_err(|err| {
                ControllerError::ResourceDataError(format!(
                    "Failed to parse '{}' annotation : {}",
                    ANNOTATION_REPLICAS_KEY, err
                ))
            })?;

        // set the replicas count to the targeted value
        self.set_replicas(&count).await?;

        // remove store/ annotation (forgetting the targeted replicas count)
        let deployments = Deploy::get_k8s_api(&self.namespace).await?;
        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                     format!("{}/{}",KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_REPLICAS_KEY): null
                }
            }
        });
        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        deployments.patch(&self.name, &params, &patch).await?;

        Ok(())
    }

    pub async fn sleep(&self) -> Result<(), ControllerError> {
        let deployments = Deploy::get_k8s_api(&self.namespace).await?;

        // get targeted replicas regarde native configuration
        let deploy = deployments.get(&self.name).await?;

        let count = deploy.spec.as_ref().and_then(|spec| spec.replicas).ok_or(
            ControllerError::ResourceDataError("Missing 'replicas' field".to_string()),
        )?;

        // storing targeted replicas to store/ annotation (remebering replicas count)
        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                     format!("{}/{}",KUBESLEEPER_ANNOTATION_PREFIX, ANNOTATION_REPLICAS_KEY): format!("{count}")
                }
            }
        });

        let params = PatchParams::default();
        let patch = Patch::Merge(&patch);
        deployments.patch(&self.name, &params, &patch).await?;

        // set replicas count to 0
        self.set_replicas(&0).await
    }
}

impl fmt::Display for Deploy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}",
            format!(
                r#"Name        : {}
Namespace   : {}
Annotations : {}"#,
                self.name,
                self.namespace,
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

impl TryFrom<&Deployment> for Deploy {
    type Error = ControllerError;

    fn try_from(deploy: &Deployment) -> std::result::Result<Self, Self::Error> {
        let raw_annotations = deploy.metadata.annotations.as_ref();
        let annotations = super::kube::extract_kube_annoations(raw_annotations);

        let name = deploy
            .name()
            .ok_or(ControllerError::ResourceDataError(
                "Missing 'name' field".to_string(),
            ))?
            .to_string();
        let namespace = ResourceExt::namespace(deploy).ok_or(
            ControllerError::ResourceDataError("Missing 'namespace' field".to_string()),
        )?;
        Ok(Deploy {
            annotations,
            name,
            namespace,
        })
    }
}
