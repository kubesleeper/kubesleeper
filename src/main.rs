extern crate rocket;
mod core;

use std::process;

use clap::{Parser, Subcommand};
use serde::Serialize;
use tokio_cron_scheduler::JobSchedulerError;

use crate::core::controller::error::Controller;
use crate::core::controller::service::Service;
use crate::core::ingress::error::IngressError;
use crate::core::state::state::create_schedule;
use crate::core::{controller::deploy::Deploy, ingress::IngressType, server};

#[derive(Parser)]
#[command(name = "kubesleeper", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Describe the kubesleeper cluster status
    Status,

    /// start kubesleeper service
    Start,

    #[command(subcommand)]
    /// Exec specific manual action for testing
    Manual(TestCommands),
}

#[derive(Subcommand)]
enum TestCommands {
    /// Set Asleep a specific Deployment
    SetDeployAsleep { namespace: String, name: String },

    /// Set Awake a specifc Deployment
    SetDeployAwake { namespace: String, name: String },

    /// Redirect a specific Service to kubesleeper server
    RedirectServiceToServer { namespace: String, name: String },

    /// Redirect a specific Service to it origin target
    RedirectServiceToOrigin { namespace: String, name: String },
}



#[derive(Debug, thiserror::Error)]
enum Error{
    #[error(transparent)]
    ControllerError(#[from] Controller),
    
    #[error(transparent)]
    IngressError(#[from] IngressError),
    
    #[error(transparent)]
    JobSchedulerError(#[from] JobSchedulerError)
}



#[tokio::main]
async fn process() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start => {
            create_schedule().await.start().await?;
            server::start().await.unwrap(); // TODO: use ? by adding the correct error type in Error struct
        }
        Commands::Status => {
        
            let deploys = crate::core::controller::deploy::Deploy::get_all("ks")
                .await?;

            let services = crate::core::controller::service::Service::get_all("ks")
                .await?;

            let traefik_metrics_pods = crate::core::ingress::traefik::Traefik::get_ingress_pods()
                .await?
                .into_iter()
                .map(|pod| pod.metadata.name)
                .collect::<Vec<_>>();
            
            
            #[derive(Serialize)]
            pub struct MetricPodsClass {
                traefik: Vec<Option<String>>,
            }
            
            #[derive(Serialize)]
            pub struct Status {
                deploys: Vec<Deploy>,
                services: Vec<Service>,
                #[serde(rename = "metric pods")]
                metricPods: MetricPodsClass
            }
            
            println!("{}",serde_yaml::to_string(
                &Status{
                    deploys: deploys,
                    services: services,
                    metricPods: MetricPodsClass{
                        traefik: traefik_metrics_pods
                    }
                }
            ).unwrap_or_else(|e| format!("{e} : Status structur should be serealizable at this point")))
        }

        Commands::Manual(test_cmd) => match test_cmd {
            TestCommands::SetDeployAsleep { name, namespace } => {
                println!("Set asleep deploy '{}' from '{}'", name, namespace);
                if let Some(deploy) = Deploy::get_all("ks")
                    .await?
                    .iter_mut()
                    .find(|x| x.name == name)
                {
                    deploy.sleep().await?;
                } else {
                    eprintln!("Error : Deployment not found");
                }
            }
            TestCommands::SetDeployAwake { name, namespace } => {
                println!("Set asleep deploy '{}' from '{}'", name, namespace);

                if let Some(deploy) = Deploy::get_all("ks")
                    .await?
                    .iter_mut()
                    .find(|x| x.name == name)
                {
                    deploy.wake().await?;
                } else {
                    panic!("Deployment not found");
                }
            }
            TestCommands::RedirectServiceToServer { name, namespace } => {
                println!("Redirect service '{}' from '{}' to server", name, namespace);

                if let Some(service) = Service::get_all("ks")
                    .await?
                    .iter_mut()
                    .find(|x| x.name == name)
                {
                    service.sleep().await?;
                } else {
                    panic!("Service not found");
                }
            }
            TestCommands::RedirectServiceToOrigin { name, namespace } => {
                println!("Redirect service '{}' from '{}' to origin", name, namespace);

                if let Some(service) = Service::get_all("ks")
                    .await?
                    .iter_mut()
                    .find(|x| x.name == name)
                {
                    service.wake().await?;
                } else {
                    panic!("Service not found");
                }
            }
        },
    }
    Ok(())
}


fn main() {
    match process() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error : {}", e);
            process::exit(1);
        }
    }
}
