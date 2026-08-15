use std::fmt::Display;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::core::k8s::deployment::{self, KsDeployment, KsDeploymentFetchingError};
use crate::core::k8s::service::{KsService, KsServiceFetchingError};
use crate::core::scheduler::metric::ArcAllServiceConnections;
use crate::core::state::state_kind::StateKind;
use crate::core::{k8s::identifier::Identifier, state::state::State};
use ::futures::future::{join_all, try_join_all};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use log::{debug, error, info, log};
use thiserror::Error;
use tokio::sync::watch::Receiver;
use tokio::task::{JoinSet, futures};
use tracing::{Level, span};

#[derive(Debug)]
pub struct Group {
    pub name: String,
    pub deployments: Vec<Identifier>,
    pub services: Vec<Identifier>,
    pub state: State,
}

async fn get_deployment(id: Identifier) -> Result<Deployment, KsDeploymentFetchingError> {
    Deployment::get(&id).await
}
async fn get_service(id: Identifier) -> Result<Service, KsServiceFetchingError> {
    Service::get(&id).await
}

#[derive(Error, Debug)]
enum CustomError<E: Display> {
    #[error("{0}")]
    FnError(E),
    #[error("A thread panicked: {0}")]
    TokioError(tokio::task::JoinError),
}



impl Group {
    pub(crate) async fn run(&mut self, mut rx: Receiver<ArcAllServiceConnections>) {
        let span = span!(Level::DEBUG, "group", name = self.name);
        let _enter = span.enter();

        // waiting for new metrics
        'main: while rx.changed().await.is_ok() {

            // checking if state should be changed
            let new_metrics = rx.borrow().clone();
            let legacy_state_kind = self.state.kind;
            self.state
                .update_from_metrics(&self.services, new_metrics)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to manage state of group '{}' : {e}", self.name)
                });
            if self.state.kind == legacy_state_kind {
                continue;
            }

            // updating resources state

            // get all deployments in parallel.
            // if an error occures the error is displayed and skiped
            let futures = self.deployments.clone().into_iter().map(|id| get_deployment(id));
            let deployments: Vec<Deployment> = match try_join_all(futures).await {
                Ok(deployments) => deployments,
                Err(e) => {error!("{}", e); break 'main;}
            };

            // setting all deployments asleep/awake
            let state_kind = self.state.kind;
            let futures = deployments.iter().map(|deployment| async move {
                let res = match state_kind {
                    StateKind::Asleep => deployment.ks_sleep().await,
                    StateKind::Awake => deployment.ks_wake().await,
                };
                (deployment, res)
            });
            for (deployment, res) in join_all(futures).await {
                if let Err(e) = res {
                    error!(
                        "Failed to manage resources of group '{}' : Failed to set Deployment '{}' {} : {e}",
                        self.name,
                        deployment.ks_id().unwrap_or(Identifier::new_unknow()),
                        state_kind
                    );
                }
            }

            // waiting all deployment to be in the desired state
            let time = Instant::now();
            loop {
                let mut is_all_deployment_ready = true;
                for deployments in deployments.iter() {
                    let (current, target) = match deployments.ks_rediness_state().await {
                        Ok(r) => r,
                        Err(e) => {
                            error!(
                                "Failed to manage resources of group '{}' : Failed to get rediness data of Deployment '{}' : {e}",
                                self.name,
                                deployments.ks_id().unwrap_or(Identifier::new_unknow())
                            );
                            (-1,-2)
                        }
                    };

                    if current != target {
                        debug!(
                            "Deployment '{}' not ready : {}/{} pod ready",
                            deployments.ks_id().unwrap_or(Identifier::new_unknow()),
                            if current == -1 { "?".to_string() } else { target.to_string() },
                            if target  == -2 { "?".to_string() } else { target.to_string() },
                        );
                        is_all_deployment_ready = false;
                    }
                }
                if is_all_deployment_ready {
                    break
                }

                let elapsed = time.elapsed().as_secs() as f32;

                let wait = ((4.5 * elapsed + 7.5) / (15 as f32)) as u64; // Backoff: linear from 0.5s to 5s over the first 15 minutes of waiting.
                debug!(
                    "Group '{}' not ready after {}m{}s : waiting {}s",
                    self.name,
                    (elapsed as i32 / 60),
                    (elapsed as i32 % 60),
                    wait
                );
                sleep(Duration::from_secs(wait));
            }



            // --- SERVICE ---
    
            // get all services in parallel.
            // if an error occures the error is displayed and skiped
            let futures = self.services.clone().into_iter().map(|id| get_service(id));
            let services: Vec<Service> = match try_join_all(futures).await {
                Ok(services) => services,
                Err(e) => {error!("{}", e); break 'main;}
            };
    
            // setting all services asleep/awake
            let state_kind = self.state.kind;
            let futures = services.iter().map(|service| async move {
                let res = match state_kind {
                    StateKind::Asleep => service.ks_sleep().await,
                    StateKind::Awake => service.ks_wake().await,
                };
                (service, res)
            });
            for (service, res) in join_all(futures).await {
                if let Err(e) = res {
                    error!(
                        "Failed to manage resources of group '{}' : Failed to set Service '{}' {} : {e}",
                        self.name,
                        service.ks_id().unwrap_or(Identifier::new_unknow()),
                        state_kind
                    );
                }
            }
        }
    }
}


impl From<crate::core::config::groups::Group> for Group {
    fn from(value: crate::core::config::groups::Group) -> Self {
        Group {
            name: value.name,
            deployments: value.deployments,
            services: value.services,
            state: State::default(),
        }
    }
}

