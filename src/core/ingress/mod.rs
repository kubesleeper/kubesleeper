use std::collections::HashMap;

use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;
use reqwest;

use crate::core::ingress::error::IngressError;

pub mod traefik;

pub mod error {
    use std::num::ParseIntError;

    use kube::config::InferConfigError;

    #[derive(Debug, thiserror::Error)]
    pub enum IngressError {
        #[allow(dead_code)]
        #[error("ReqwestError : {0}")]
        ReqwestError(#[from] reqwest::Error),

        #[allow(dead_code)]
        #[error("KubeError : {0}")]
        KubeError(#[from] kube::Error),

        #[allow(dead_code)]
        #[error("ResourceDataError : {0}")]
        ResourceDataError(String),

        #[error("InferConfigError : {0}")]
        InferConfigError(#[from] InferConfigError),

        #[error("ParseIntError : {0}")]
        ParseIntError(#[from] ParseIntError),
    }
}

pub trait IngressType {
    /// Retrieve all child pod of the ingress
    async fn get_metrics_pods() -> Result<Vec<Pod>, IngressError>;

    /// Parse the raw prometheus metric dump of a single ingress metric pod (fetched by `get_metrics_pods`)
    /// to get all the number of connection that occurs for a specific service
    ///
    /// Return a couple (service name, nb connection)
    async fn parse_prometheus_metrics(
        raw_metrics_dump: String,
    ) -> Result<HashMap<String, u64>, IngressError>;

    /// Fetch all activity metrics (number of total connection for a specific service)
    ///
    /// Return a HashMap of 'service name' : { 'metric po': 'nb connection' }
    async fn get_metrics() -> Result<HashMap<String, HashMap<String, u64>>, IngressError> {
        let metrics_pods = Self::get_metrics_pods().await?;
        let mut res: HashMap<String, HashMap<String, u64>> = HashMap::new();

        for metrics_pod in metrics_pods {
            let dump = get_prometheus_raw_metrics_dump(&metrics_pod).await?;

            for (service_name, nb_connection) in Self::parse_prometheus_metrics(dump).await? {
                let uid = metrics_pod.metadata.uid.to_owned().unwrap();
                *res.entry(service_name).or_default().entry(uid).or_default() += nb_connection;
            }
        }
        Ok(res)
    }
}

const PROMETHEUS_PORT_ANNOTATION: &str = "prometheus.io/port";
const PROMETHEUS_PATH_ANNOTATION: &str = "prometheus.io/path";

pub async fn get_prometheus_raw_metrics_dump(pod: &Pod) -> Result<String, IngressError> {
    // let pod_id = format!(
    //     "{} {}",
    //     pod.metadata.name.clone().unwrap_or_default(),
    //     pod.metadata.namespace.clone().unwrap_or_default()
    // );

    let port = pod.annotations().get(PROMETHEUS_PORT_ANNOTATION).ok_or(
        IngressError::ResourceDataError("No port annotation".to_string()),
    )?;

    let ip = match &pod.status {
        Some(status) => match &status.pod_ip {
            Some(ip) => Ok(ip),
            None => Err(IngressError::ResourceDataError("No ip".to_string())),
        },
        None => Err(IngressError::ResourceDataError("No status".to_string())),
    }?;

    let path = pod.annotations().get(PROMETHEUS_PATH_ANNOTATION).ok_or(
        IngressError::ResourceDataError("No path annotation".to_string()),
    )?;

    let url = format!("http://{}:{}/{}", ip, port, path);

    Ok(reqwest::get(url).await?.text().await?)
}
