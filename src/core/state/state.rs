use crate::core::{
    controller::{deploy::Deploy, service::Service},
    ingress::traefik::Traefik,
};

use crate::core::{
    ingress::IngressType,
    state::{
        StateError,
        notification::{Notification, NotificationKind},
        state_kind::StateKind,
    },
};

use lazy_static::lazy_static;
use std::fmt::format;
use std::num::NonZeroU32;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, info, instrument};
use uuid::Uuid;

// - - - - - - - - - - - - -
lazy_static! {
    pub static ref STATE: Mutex<State> = Mutex::new(State::default());
}

//pub const ANNOTATION_STORE_STATE_KEY: &str = "store.state";

pub static SLEEPINESS_DURATION: std::sync::OnceLock<Duration> = std::sync::OnceLock::new();

#[derive(Debug)]
pub struct State {
    pub kind: StateKind,
    pub since: Notification,
    pub metrics: HashMap<String, HashMap<String, u64>>,
}

impl State {
    // TODO: review ingress suppression behavior ?

    fn create_notification_from_metrics(
        // HashMap<ServiceId, HashMap<Ingress Pod Uid, nb of connections received>>
        metrics_data: &HashMap<String, HashMap<String, u64>>,
    ) -> Result<Notification, StateError> {
        let state = STATE
            .lock()
            .map_err(|e| StateError::LockError(format!("{e:?}")))?;

        for (service_id, metric) in metrics_data {
            if let Some(stored_metric) = state.metrics.get(service_id) {
                // Service already exists in the state,
                // looking for update : is one of ingress pods has proceed at least 1 connection ?
                for (ingress_pod_uid, total_connection) in metric {
                    let stored_total_connection = stored_metric.get(ingress_pod_uid);
                    if stored_total_connection.is_none() {
                        // the ingress pod uid is not in the state, so its a new ingress pod,
                        // to be registerd is must has received at least 1 connection, so there was activity
                        debug!("Ingress pod with uid '{ingress_pod_uid}' is new > Activity ");
                        return Ok(Notification::new(NotificationKind::Activity));
                    }

                    let nb_new_connection =
                        stored_total_connection.map_or(0, |stored| stored - total_connection);
                    if nb_new_connection > 0 {
                        debug!(
                            "Ingress pod with uid '{ingress_pod_uid}' has proceed {nb_new_connection} new connection > Activity "
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

    pub async fn update_from_notification(notification: Notification) -> Result<(), StateError> {
        let mut action: Option<StateKind> = None;

        {
            // explaination of the error if remove this scoped block
            debug!("Update state from Notification");
            let mut state = STATE
                .lock()
                .map_err(|e| StateError::LockError(format!("{e:?}")))?;

            match (&state.since.kind, &notification.kind) {
                (NotificationKind::Activity, NotificationKind::Activity) => {
                    info!("State do not change > {:?}", &state.since.kind);
                }
                (NotificationKind::Activity, NotificationKind::NoActivity) => {
                    info!("State change > {:?}", &state.since.kind);
                    state.since = notification; // new state kind since this new notification
                }
                (NotificationKind::NoActivity, NotificationKind::NoActivity) => {
                    let sleepiness_duration = notification.timestamp - state.since.timestamp;
                    let max_sleepiness_duration = match SLEEPINESS_DURATION.get() {
                        Some(s) => *s,
                        None => panic!("SLEEPINESS_DURATION should be set a this step"),
                    };
                    if sleepiness_duration >= max_sleepiness_duration
                        && state.kind != StateKind::Asleep
                    {
                        // The application has been in sleepiness mode for too long; it must set asleep.
                        debug!(
                            "Sleepiness duration exceeded: maximum sleepiness duration is {max_sleepiness_duration:?}s, but the state was in this condition {sleepiness_duration:?}s."
                        );
                        info!("State change > Asleep");
                        state.kind = StateKind::Asleep;
                        action = Some(StateKind::Asleep);
                    }
                    info!("State do not change > {:?}", &state.since.kind);
                }
                (NotificationKind::NoActivity, NotificationKind::Activity) => {
                    // The application has received a connection but is asleep, must be waked up.
                    state.since = notification;
                    state.kind = StateKind::Awake;
                    info!("State change to Awake ");
                    action = Some(StateKind::Awake);
                }
            };
        }

        match action {
            Some(StateKind::Asleep) => {
                debug!("Making all Deploy 'Asleep'");
                Deploy::change_all_state(StateKind::Asleep).await?;
                debug!("Making all Service 'Asleep'");
                Service::change_all_state(StateKind::Asleep).await?;
            }
            Some(StateKind::Awake) => {
                debug!("Making all Deploy 'Awake'");
                Deploy::change_all_state(StateKind::Awake).await?;
                debug!("Making all Service 'Awake'");
                Service::change_all_state(StateKind::Awake).await?;
            }
            None => {}
        };
        Ok(())
    }

    pub async fn update_from_metrics(
        new_metrics: HashMap<String, HashMap<String, u64>>,
    ) -> Result<(), StateError> {
        debug!("Updating state from metrics");
        // Update notification
        State::update_from_notification(State::create_notification_from_metrics(&new_metrics)?)
            .await?;

        // Update metrics
        STATE
            .lock()
            .map_err(|e| StateError::LockError(format!("{e:?}")))?
            .metrics = new_metrics;
        Ok(())
    }
}

#[instrument(
    name = "schedule"
    level = "info"
    skip(uuid)
    fields(uuid = %uuid)
)]
async fn process(uuid: Uuid) {
    let metrics = Traefik::get_metrics().await;

    State::update_from_metrics(metrics.map_err(|e| e.to_string()).unwrap())
        .await
        .map_err(|e| e.to_string())
        .unwrap();
}

pub async fn create_schedule(refresh_interval: NonZeroU32) -> JobScheduler {
    let sched = JobScheduler::new().await.unwrap();

    sched
        .add(
            Job::new_async(format!("1/{refresh_interval} * * * * *"), |uuid, mut l| {
                Box::pin(async move {
                    {
                        process(Uuid::new_v4()).await
                    }

                    // Query the next execution time for this job
                    let _next_tick = l.next_tick_for_job(uuid).await;
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();
    info!("Running scheduler");
    sched
}
impl Default for State {
    fn default() -> Self {
        // TODO: chose first awake or asleep from config
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
