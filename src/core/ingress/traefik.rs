use crate::core::{
    ingress::{IngressType, error::IngressError},
    k8s::identifier::Identifier,
};

use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client, Config, api::ListParams};
use regex::Regex;
use std::collections::HashMap;

const SELECTOR: &str = "app.kubernetes.io/name=traefik";
const TRAEFIK_REGEXP_METRIC: &str = r#"traefik_service_requests_total\{.*service="(.+)".*\} (\d+)"#;

pub struct Traefik {}

impl IngressType for Traefik {
    async fn get_ingress_pods() -> Result<Vec<Pod>, IngressError> {
        let mut config = Config::infer().await?;
        config.default_namespace = "kube-system".to_string();
        let client = Client::try_from(config)?;

        Ok(Api::<Pod>::all(client)
            .list(&ListParams::default().labels(SELECTOR))
            .await?
            .into_iter()
            .collect())
    }

    async fn parse_prometheus_metrics(
        raw_metrics_dump: String,
    ) -> Result<HashMap<Identifier, u64>, IngressError> {
        let re = Regex::new(TRAEFIK_REGEXP_METRIC).unwrap();
        let captured = re.captures_iter(&raw_metrics_dump);

        let mut res = HashMap::<Identifier, u64>::new();

        for capture in captured {
            // Can panic if the regexp is modified regarding the group count
            let (full, groups): (&str, [&str; 2]) = capture.extract();
            assert_eq!(
                groups.len(),
                2,
                "Wrong groups number were found in '{full}' with regex '{TRAEFIK_REGEXP_METRIC}'"
            );
            let raw_service_name = groups[0].to_string();
            let service_name: Vec<_> = raw_service_name.split('-').collect();
            let id: Identifier = format!(
                "{}/{}",
                service_name.get(0).unwrap_or(&""),
                service_name.get(1).unwrap_or(&"")
            )
            .try_into()?;

            let nb: u64 = groups[1].parse().map_err(|err| {
                IngressError::ParsingMetricError(format!(
                    "Can't parse nomber of calls received : {err}"
                ))
            })?;

            *res.entry(id).or_insert(0) += nb;
        }

        Ok(res)
    }
}
