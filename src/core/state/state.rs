use crate::core::controller::deploy::Deploy;
use crate::core::controller::service::Service;
use crate::core::ingress::traefik::Traefik;

use crate::core::ingress::IngressType;
use crate::core::state::StateError;
use crate::core::state::notification::{Notification, NotificationKind};
use crate::core::state::state_kind::StateKind;

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio_cron_scheduler::{Job, JobScheduler};

// - - - - - - - - - - - - -
lazy_static! {
    pub static ref STATE: Mutex<State> = Mutex::new(State::default());
}

//pub const ANNOTATION_STORE_STATE_KEY: &str = "store.state";

const MAX_SLEEPNESS_DURATION: Duration = Duration::new(15, 0);
/// TODO : dynamic from config
// - - - - - - - - - - - - -

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
                        return Ok(Notification::new(NotificationKind::Activity));
                    }
                    if stored_total_connection.map_or(false, |stored| stored <= total_connection) {
                        // there was a activity cas a ingress pod has proceed at least 1 connection
                        return Ok(Notification::new(NotificationKind::Activity));
                    }
                }
            } else {
                // Service is not already in the state, so it's a new one, by default its
                // considered as 'activity' to prevent instante sleeping when resources are created
                return Ok(Notification::new(NotificationKind::Activity));
            }
        }
        // Finally, if no new service, no new ingress pod and no new connections
        Ok(Notification::new(NotificationKind::NoActivity))
    }

    pub async fn update_from_notification(notification: Notification) -> Result<(), StateError> {
        let mut action: Option<StateKind> = None;
        {
            let mut state = STATE
                .lock()
                .map_err(|e| StateError::LockError(format!("{e:?}")))?;

            match (&state.since.kind, &notification.kind) {
                (NotificationKind::Activity, NotificationKind::Activity) => return Ok(()),
                (NotificationKind::Activity, NotificationKind::NoActivity) => {
                    state.since = notification; // new state kind since this new notification
                }
                (NotificationKind::NoActivity, NotificationKind::NoActivity) => {
                    let sleepness_duration = notification.timestamp - state.since.timestamp;
                    if sleepness_duration >= MAX_SLEEPNESS_DURATION {
                        // The application has been in slpeepness mode for too long; it must set asleep.
                        state.kind = StateKind::Asleep;
                        action = Some(StateKind::Asleep);
                    }
                }
                (NotificationKind::NoActivity, NotificationKind::Activity) => {
                    // The application has received a connection but is asleep, must be waked up.
                    state.since = notification;
                    state.kind = StateKind::Awake;
                    action = Some(StateKind::Awake);
                }
            }
        }

        match action {
            Some(StateKind::Asleep) => {
                Deploy::change_all_state(StateKind::Asleep).await?;
                Service::change_all_state(StateKind::Asleep).await?;
                Ok(())
            }
            Some(StateKind::Awake) => {
                Deploy::change_all_state(StateKind::Awake).await?;
                Service::change_all_state(StateKind::Awake).await?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub async fn update_from_metrics(
        new_metrics: HashMap<String, HashMap<String, u64>>,
    ) -> Result<(), StateError> {
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

pub async fn create_schedule() -> JobScheduler {
    let sched = JobScheduler::new().await.unwrap();

    sched
        .add(
            Job::new_async("1/5 * * * * *", |uuid, mut l| {
                Box::pin(async move {
                    {
                        let metrics = Traefik::get_metrics().await;

                        State::update_from_metrics(metrics.unwrap()).await.unwrap();
                    }

                    // Query the next execution time for this job
                    let _next_tick = l.next_tick_for_job(uuid).await;
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();

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
