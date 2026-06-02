use std::fmt::Display;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::core::k8s::deployment::{self, KsDeployment, KsDeploymentFetchingError};
use crate::core::scheduler::metric::ArcAllServiceConnections;
use crate::core::state::state_kind::StateKind;
use crate::core::{k8s::identifier::Identifier, state::state::State};
use k8s_openapi::api::apps::v1::Deployment;
use log::{debug, error};
use thiserror::Error;
use tokio::sync::watch::Receiver;
use tokio::task::JoinSet;
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

#[derive(Error, Debug)]
enum CustomError<E: Display> {
    #[error("{0}")]
    FnError(E),
    #[error("A thread panicked: {0}")]
    TokioError(tokio::task::JoinError),
}

//isation
async fn run_parallel<T, O, E, F, Fut>(
    elements: impl IntoIterator<Item = T>,
    task: F,
) -> Result<Vec<O>, CustomError<E>>
where
    F: Fn(T) -> Fut,
    Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
    E: Send + Display + 'static,
    O: Send + 'static,
{
    let mut set = JoinSet::new();
    for element in elements.into_iter() {
        set.spawn(task(element));
    }

    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(ok)) => {
                results.push(ok);
            }
            Err(e) => {
                return Err(CustomError::TokioError(e));
            }
            Ok(Err(e)) => {
                return Err(CustomError::FnError(e));
            }
        }
    }
    Ok(results)
}

impl Group {
    pub(crate) async fn run(&mut self, mut rx: Receiver<ArcAllServiceConnections>) {
        let span = span!(Level::DEBUG, "group", name = self.name);
        let _enter = span.enter();
        'main: while rx.changed().await.is_ok() {
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

            // --- DEPLOYMENT ---
            let deployments: Vec<Deployment> =
                match run_parallel(self.deployments.clone(), get_deployment).await {
                    Ok(deployment) => deployment,
                    Err(e) => {
                        error!("{}", e);
                        continue 'main;
                    }
                };

            for deployments in deployments.iter() {
                if let Err(e) = match self.state.kind {
                    StateKind::Asleep => deployments.ks_sleep().await,
                    StateKind::Awake => deployments.ks_wake().await,
                } {
                    error!(
                        "Failed to manage resources of group '{}' : Failed to set Deployment '{}' {} : {e}",
                        self.name,
                        deployments.ks_id().unwrap_or(Identifier::new_unknow()),
                        self.state.kind
                    );
                    continue 'main;
                }
            }

            if self.state.kind == StateKind::Asleep {
                continue 'main;
            }

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
                            continue 'main;
                        }
                    };

                    if current != target {
                        debug!(
                            "Deployment '{}' not ready : {current}/{target} pod ready",
                            deployments.ks_id().unwrap_or(Identifier::new_unknow())
                        );
                        is_all_deployment_ready = false;
                    }
                }
                if is_all_deployment_ready {
                    continue 'main;
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
        }

        // --- SERVICE ---
        for service_id in self.services.iter() {
            let service = match Deployment::get(&service_id).await {
                Err(e) => {
                    error!(
                        "Failed to manage resources of group '{}' : Failed to get Service '{}' : {e}",
                        self.name, service_id
                    );
                    continue;
                }
                Ok(d) => d,
            };

            if let Err(e) = match self.state.kind {
                StateKind::Asleep => service.ks_sleep().await,
                StateKind::Awake => service.ks_wake().await,
            } {
                error!(
                    "Failed to manage resources of group '{}' : Failed to set Service '{}' {} : {e}",
                    self.name, service_id, self.state.kind
                );
                continue;
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

// job(
//     wait 30s
//     global data = get_metrics()
//     channel_casting(data)
// )
