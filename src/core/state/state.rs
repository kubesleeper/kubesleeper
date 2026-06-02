use crate::core::ingress::AllServiceConnections;
use crate::core::k8s::deployment::KsDeployment;
use crate::core::k8s::identifier::Identifier;
use crate::core::k8s::service::KsService;

use crate::core::scheduler::metric::ArcAllServiceConnections;
use crate::core::state::{
    StateError,
    notification::{Notification, NotificationKind},
    state_kind::StateKind,
};

use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use lazy_static::lazy_static;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use tracing::{debug, info};

// - - - - - - - - - - - - -

pub static SLEEPINESS_DURATION: std::sync::OnceLock<Duration> = std::sync::OnceLock::new();

#[derive(Debug)]
pub struct State {
    pub kind: StateKind,
    pub since: Notification,
    pub metrics: ArcAllServiceConnections,
}

impl State {
    // TODO: review ingress suppression behavior ?

    fn create_notification_from_metrics(
        &self,
        service_targets: &Vec<Identifier>,
        // HashMap<ServiceId, HashMap<Ingress Pod Uid, nb of connections received>>
        metrics_data: &AllServiceConnections,
    ) -> Result<Notification, StateError> {
        for (service_id, connections) in metrics_data
            .iter()
            .filter(|(id, _)| service_targets.contains(id))
        {
            if let Some(stored_metric) = self.metrics.get(service_id) {
                // Service already exists in the state,
                // looking for update : is one of ingress pods has proceed at least 1 connection ?
                for (ingress_pod_id, total_connection) in connections {
                    let stored_total_connection = stored_metric.get(ingress_pod_id);
                    if stored_total_connection.is_none() {
                        // the ingress pod id is not in the state, so its a new ingress pod,
                        // to be registerd is must has received at least 1 connection, so there was activity
                        debug!("Ingress pod with id '{ingress_pod_id}' is new > Activity ");
                        return Ok(Notification::new(NotificationKind::Activity));
                    }

                    let nb_new_connection =
                        stored_total_connection.map_or(0, |stored| stored - total_connection);
                    if nb_new_connection > 0 {
                        debug!(
                            "Ingress pod with uid '{ingress_pod_id}' has proceed {nb_new_connection} new connection > Activity "
                        );
                        return Ok(Notification::new(NotificationKind::Activity));
                    }
                }
            } else {
                // Service is not already in the state, so it's a new one, by default its
                // considered as 'activity' to prevent instante sleeping when resources are created
                debug!("Service '{service_id}' is new > Activity ");
                return Ok(Notification::new(NotificationKind::Activity));
            }
        }
        // Finally, if no new service, no new ingress pod and no new connections
        debug!("No new service, no new ingress, no new connections > No Activity");
        Ok(Notification::new(NotificationKind::NoActivity))
    }

    pub async fn update_from_notification(
        &mut self,
        notification: Notification,
    ) -> Result<(), StateError> {
        // explaination of the error if remove this scoped block
        debug!("Update state from Notification");
        match (&self.since.kind, &notification.kind) {
            (NotificationKind::Activity, NotificationKind::Activity) => {
                info!("State do not change > {:?}", self.since.kind);
            }
            (NotificationKind::Activity, NotificationKind::NoActivity) => {
                info!("State change > {:?}", self.since.kind);
                self.since = notification; // new state kind since this new notification
            }
            (NotificationKind::NoActivity, NotificationKind::NoActivity) => {
                let sleepiness_duration = notification.timestamp - self.since.timestamp;
                let max_sleepiness_duration = match SLEEPINESS_DURATION.get() {
                    Some(s) => *s,
                    None => panic!("SLEEPINESS_DURATION should be set a this step"),
                };
                if sleepiness_duration >= max_sleepiness_duration && self.kind != StateKind::Asleep
                {
                    // The application has been in sleepiness mode for too long; it must set asleep.
                    debug!(
                        "Sleepiness duration exceeded: maximum sleepiness duration is {max_sleepiness_duration:?}s, but the state was in this condition {sleepiness_duration:?}s."
                    );
                    info!("State change > Asleep");
                    self.kind = StateKind::Asleep;
                    // action = Some(StateKind::Asleep);
                }
                info!("State do not change > {:?}", &self.since.kind);
            }
            (NotificationKind::NoActivity, NotificationKind::Activity) => {
                // The application has received a connection but is asleep, must be waked up.
                self.since = notification;
                self.kind = StateKind::Awake;
                info!("State change to Awake ");
                // action = Some(StateKind::Awake);
            }
        };
        Ok(())

        // match action {
        //     Some(StateKind::Asleep) => {
        //         debug!("Making all Deployments 'Asleep'");
        //         for deploy in Deployment::get_all().await?.iter_mut() {
        //             deploy.ks_sleep().await?;
        //         }
        //         debug!("Making all Services 'Asleep'");
        //         for service in Service::get_all().await?.iter_mut() {
        //             service.ks_sleep().await?;
        //         }
        //     }
        //     Some(StateKind::Awake) => {
        //         debug!("Making all Deployments 'Awake'");
        //         for deploy in Deployment::get_all().await?.iter_mut() {
        //             deploy.ks_wake().await?;
        //         }
        //         debug!("Making all Services 'Awake'");
        //         for service in Service::get_all().await?.iter_mut() {
        //             service.ks_wake().await?;
        //         }
        //     }
        //     None => {}
        // };
    }

    pub async fn update_from_metrics(
        &mut self,
        service_name: &Vec<Identifier>,
        new_metrics: ArcAllServiceConnections,
    ) -> Result<(), StateError> {
        debug!("Updating state from metrics");

        // Update notification
        self.update_from_notification(
            self.create_notification_from_metrics(service_name, &new_metrics)?
        )
        .await?;

        // Update metrics
        self.metrics = new_metrics;
        Ok(())
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            since: Notification {
                kind: NotificationKind::Activity,
                timestamp: Instant::now(),
            },
            kind: StateKind::Awake,
            metrics: Default::default(),
        }
    }
}
