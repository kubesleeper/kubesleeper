#[macro_use]
extern crate rocket;
mod core;

use crate::core::watcher::ingress_type::IngressType;
use crate::core::watcher::ingress_type::traefik::Traefik;
use clap::{Parser, Subcommand};
use core::{controller, server};
use rocket::form::validate::Contains;

use crate::core::state::create_schedule;

#[derive(Parser)]
#[command(name = "kubesleeper", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Put targets to sleep
    Sleep {
        /// Filter target by specified groups
        #[arg(short = 'g', long = "group")]
        group: Vec<String>,
    },

    /// Wake up targets
    Wake {
        /// Filter target by specified groups
        #[arg(short = 'g', long = "group")]
        group: Vec<String>,
    },

    /// List targets
    List {
        /// List only target with specific groups
        #[arg(short = 'g', long = "group")]
        groups: Vec<String>,
    },
    Start,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start => {
            create_schedule().await.start().await.unwrap();
            server::start().await.unwrap();
        }
        Commands::Sleep { group } => {
            for deploy in controller::kube::get_all_deploys()
                .await
                .unwrap()
                .iter()
                .filter(|deploy| {
                    (group.len() == 0)
                        || deploy
                            .annotations
                            .group
                            .to_owned()
                            .filter(|g| group.contains(g))
                            .is_some()
                })
            {
                deploy.sleep().await.unwrap();
            }
        }
        Commands::Wake { group } => {
            for deploy in controller::kube::get_all_deploys()
                .await
                .unwrap()
                .iter()
                .filter(|deploy| {
                    (group.len() == 0)
                        || deploy
                            .annotations
                            .group
                            .to_owned()
                            .filter(|g| group.contains(g))
                            .is_some()
                })
            {
                deploy.wake().await.unwrap();
            }
        }
        Commands::List { groups } => {
            println!("━━━━━━ KubeSleeper's Deploys Targets ━━━━━━");
            let filtered_deploys = controller::kube::get_deploys_filtered_by_groups(groups)
                .await
                .unwrap();
            if filtered_deploys.len() == 0 {
                println!("No deploy found for the specified groups");
            } else {
                filtered_deploys.iter().for_each(|deploy| {
                    println!("{deploy}");
                })
            }
            println!("━━━━━━━━━━━━");
            println!("{:?}", Traefik::get_metrics().await);
        }
    }
}
