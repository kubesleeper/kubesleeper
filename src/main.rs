extern crate rocket;
mod core;

use clap::{Parser, Subcommand};

use crate::core::controller::service::Service;
use crate::core::{
    controller::deploy::Deploy, ingress::IngressType, server, state::create_schedule,
};

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
    Test(TestCommands),
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start => {
            create_schedule().await.start().await.unwrap();
            server::start().await.unwrap();
        }
        Commands::Status => {
            println!("━━━ Deploys\n");
            let deploys = crate::core::controller::deploy::Deploy::get_all()
                .await
                .unwrap();
            deploys.iter().for_each(|deploy| {
                println!("{deploy}");
            });

            println!("━━━ Services\n");
            let services = crate::core::controller::service::Service::get_all()
                .await
                .unwrap();
            services.iter().for_each(|service| {
                println!("{service}");
            });

            println!("\n\n━━━ Metrics\n");
            let metrics_pods = crate::core::ingress::traefik::Traefik::get_metrics_pods()
                .await
                .unwrap();

            println!(
                "Traefik metrics pods :\n{}",
                metrics_pods
                    .into_iter()
                    .map(|pod| format!("  {}", pod.metadata.name.unwrap_or("no name".to_string())))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            println!(
                "\nTraefik Metrics data :\n{:?}",
                crate::core::ingress::traefik::Traefik::get_metrics()
                    .await
                    .unwrap()
            );
        }

        Commands::Test(test_cmd) => match test_cmd {
            TestCommands::SetDeployAsleep { name, namespace } => {
                println!("Set asleep deploy '{}' from '{}'", name, namespace);

                if let Some(deploy) = Deploy::get_all()
                    .await
                    .unwrap()
                    .iter()
                    .find(|x| x.name == name)
                {
                    deploy.sleep().await.unwrap();
                } else {
                    eprintln!("Error : Deployment not found");
                }
            }
            TestCommands::SetDeployAwake { name, namespace } => {
                println!("Set asleep deploy '{}' from '{}'", name, namespace);

                if let Some(deploy) = Deploy::get_all()
                    .await
                    .unwrap()
                    .iter()
                    .find(|x| x.name == name)
                {
                    deploy.wake().await.unwrap();
                } else {
                    panic!("Deployment not found");
                }
            }
            TestCommands::RedirectServiceToServer { name, namespace } => {
                println!("Redirect service '{}' from '{}' to server", name, namespace);

                if let Some(service) = Service::get_all()
                    .await
                    .unwrap()
                    .iter()
                    .find(|x| x.name == name)
                {
                    service.redirect_to_server().await.unwrap();
                } else {
                    panic!("Service not found");
                }
            }
            TestCommands::RedirectServiceToOrigin { name, namespace } => {
                println!("Redirect service '{}' from '{}' to origin", name, namespace);

                if let Some(service) = Service::get_all()
                    .await
                    .unwrap()
                    .iter()
                    .find(|x| x.name == name)
                {
                    service.redirect_to_origin().await.unwrap();
                } else {
                    panic!("Service not found");
                }
            }
        },
    }
}
