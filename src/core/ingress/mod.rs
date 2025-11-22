use std::collections::HashMap;

use k8s_openapi::api::core::v1::Pod;
use kube::{ResourceExt, runtime::reflector::Lookup};
use reqwest;
use tracing::{debug};

use crate::core::ingress::error::IngressError;

pub mod traefik;

pub mod error {
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
        #[error("Failed to parse kube resource : {0}")]
        ResourceParse(#[from] ResourceParse),

        #[allow(dead_code)]
        #[error("ParsingMetricError : {0}")]
        ParsingMetricError(String),

        #[error("InferConfigError : {0}")]
        InferConfigError(#[from] InferConfigError),
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ResourceParse {
        #[error("Resource '{id}' : Required value '{value}' is missing on.")]
        MissingValue {
            /// Resource identifier (like "{name}/{namespace}")
            id: String,
            /// name of the missing value
            value: String,
        },
    }
}

pub trait IngressType {
    /// Retrieve all child pod of the ingress
    async fn get_ingress_pods() -> Result<Vec<Pod>, IngressError>;

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
        debug!("Get metrics from traefik ingress");
        let ingress_pods = Self::get_ingress_pods().await?;
        let mut res: HashMap<String, HashMap<String, u64>> = HashMap::new();

        for ingress_pod in ingress_pods {
            let dump = get_prometheus_raw_metrics_dump(&ingress_pod).await?;

            for (service_name, nb_connection) in Self::parse_prometheus_metrics(dump).await? {
                let ingress_pod_uid = ingress_pod.metadata.uid.to_owned().unwrap();
                *res.entry(service_name)
                    .or_default()
                    .entry(ingress_pod_uid)
                    .or_default() += nb_connection;
            }
        }
        Ok(res)
    }
}

const PROMETHEUS_PORT_ANNOTATION: &str = "prometheus.io/port";
const PROMETHEUS_PATH_ANNOTATION: &str = "prometheus.io/path";

pub async fn get_prometheus_raw_metrics_dump(pod: &Pod) -> Result<String, IngressError> {
    let pod_id = format!(
        "{}/{}",
        pod.name().unwrap_or("?".into()),
        ResourceExt::namespace(pod).unwrap_or("?".into())
    );

    let port = pod.annotations().get(PROMETHEUS_PORT_ANNOTATION).ok_or(
        error::ResourceParse::MissingValue {
            id: format!("{pod_id}"),
            value: format!(".annotation.{}", PROMETHEUS_PORT_ANNOTATION),
        },
    )?;

    let ip = match &pod.status {
        Some(status) => match &status.pod_ip {
            Some(ip) => Ok(ip),
            None => Err(error::ResourceParse::MissingValue {
                id: format!("{pod_id}"),
                value: format!(".status.ip"),
            }),
        },
        None => Err(error::ResourceParse::MissingValue {
            id: format!("{pod_id}"),
            value: format!(".status"),
        }),
    }?;

    let path = pod.annotations().get(PROMETHEUS_PATH_ANNOTATION).ok_or(
        error::ResourceParse::MissingValue {
            id: format!("{pod_id}"),
            value: format!(".annotations.{}", PROMETHEUS_PATH_ANNOTATION),
        },
    )?;

    let url = format!("http://{}:{}/{}", ip, port, path);

    Ok(reqwest::get(url).await?.text().await?)
}
