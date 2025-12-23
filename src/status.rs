use std::collections::HashMap;

use serde::Serialize;

use crate::core::{
    ingress::IngressType,
    resource::{
        TargetResource,
        deploy::Deploy,
        service::{Service, ServicePort},
    },
};

#[derive(Serialize)]
struct DeployStatus {
    id: String,
    state: String,
    stored_replicas: i32,
}

#[derive(Serialize)]
struct ServiceStatus {
    id: String,
    state: String,
    stored_selector: HashMap<String, String>,
    stored_ports: Vec<ServicePort>,
}

pub async fn status() -> Result<(), crate::Error> {
    let deploys = Deploy::get_all().await?;
    let mut deploys_status = Vec::new();
    for d in deploys {
        let state = if d.is_asleep() {
            "asleep".to_string()
        } else {
            let ready_replicas_count = d.get_ready_replicas_count().await?;

            if ready_replicas_count != d.replicas {
                format!("waking up ({}/{})", ready_replicas_count, d.replicas)
            } else {
                "awake".to_string()
            }
        };

        deploys_status.push(DeployStatus {
            id: d.id,
            state,
            stored_replicas: d.store_replicas,
        });
    }

    let services = Service::get_all().await?;

    let mut services_status = Vec::new();
    for s in services {
        let state = if s.is_asleep() {
            "asleep".to_string()
        } else {
            "awake".to_string()
        };

        services_status.push(ServiceStatus {
            id: s.id.clone(),
            state,
            stored_selector: s.store_selector,
            stored_ports: s.store_ports,
        });
    }

    let traefik_metrics_pods = crate::core::ingress::traefik::Traefik::get_ingress_pods()
        .await?
        .into_iter()
        .map(|pod| pod.metadata.name)
        .collect::<Vec<_>>();

    let json = serde_json::json!({
        "Deployments" : deploys_status,
        "Services" : services_status,
        "Metric Pods": {
            "Traefik" : traefik_metrics_pods
        }
    });

    println!(
        "{}",
        serde_yaml::to_string(&json).unwrap_or_else(|e| format!(
            "{e} : Status structure should be serealizable at this point"
        ))
    );
    Ok(())
}
