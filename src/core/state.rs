use crate::core::controller::deploy::Deploy;
use crate::core::controller::error::ControllerError;
use crate::core::ingress::IngressType as _;
use crate::core::ingress::traefik::Traefik;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio_cron_scheduler::{Job, JobScheduler};

// - - - - - - - - - - - - -
lazy_static! {
    pub static ref STATE: Mutex<State> = Mutex::new(State::default());
}

const MAX_SLEEPNESS_DURATION: Duration = Duration::new(15, 0);
/// TODO : dynamic from config
// - - - - - - - - - - - - -

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("LockError : {0}")]
    LockError(String),

    #[error(transparent)]
    ControllerError(#[from] ControllerError),
}

#[derive(Eq, PartialEq, Debug)]
pub enum NotificationKind {
    Activity,
    NoActivity,
}
#[derive(Debug)]
pub struct Notification {
    pub kind: NotificationKind,
    timestamp: Instant,
}

impl Notification {
    pub fn new(kind: NotificationKind) -> Notification {
        Notification {
            kind,
            timestamp: Instant::now(),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum StateKind {
    Asleep,
    Awake,
    // Sleepness,
}
pub struct State {
    pub kind: StateKind,
    pub first_same_kind_notification: Notification,
    /// HashMap<"namespace-service-port", HashMap<"uid_pod_ingress", total_connection>>
    pub metrics: HashMap<String, HashMap<String, u64>>,
}

impl State {
    // TODO: review ingress suppression behavior ?

    fn create_notification_from_metrics(
        new_metrics: &HashMap<String, HashMap<String, u64>>,
    ) -> Result<Notification, StateError> {
        // let old_metrics = mem::replace(&mut self.metrics, new_metrics);
        let state = STATE
            .lock()
            .map_err(|e| StateError::LockError(format!("{e:?}")))?;
        for (service, metric) in new_metrics {
            if let Some(hm) = state.metrics.get(service) {
                // If service already exist
                for (uid, total_connection) in metric {
                    if let Some(total_connection_old) = hm.get(uid)
                        && total_connection <= total_connection_old
                    {
                        // If no new connection detected, keep looking into next metric pod
                        continue;
                    }
                    // Activity detected
                    return Ok(Notification::new(NotificationKind::Activity));
                }
            } else {
                // Activity detected
                return Ok(Notification::new(NotificationKind::Activity));
            }
        }
        // Finally, if no new service, no new metric pod and no new connection detected
        Ok(Notification::new(NotificationKind::NoActivity))
    }

    pub async fn update_from_notif(new_notification: Notification) -> Result<(), StateError> {
        let mut action: Option<StateKind> = None;
        {
            println!("{:?}", new_notification);
            let mut state = STATE
                .lock()
                .map_err(|e| StateError::LockError(format!("{e:?}")))?;

            match (
                &state.first_same_kind_notification.kind,
                &new_notification.kind,
            ) {
                (NotificationKind::Activity, NotificationKind::Activity) => return Ok(()),
                (NotificationKind::Activity, NotificationKind::NoActivity) => {
                    state.first_same_kind_notification = new_notification;
                }
                (NotificationKind::NoActivity, NotificationKind::NoActivity) => {
                    let sleepness_duration =
                        new_notification.timestamp - state.first_same_kind_notification.timestamp;
                    if sleepness_duration >= MAX_SLEEPNESS_DURATION {
                        println!("{:?}", sleepness_duration);
                        state.kind = StateKind::Asleep;
                        action = Some(StateKind::Asleep);
                    }
                }
                (NotificationKind::NoActivity, NotificationKind::Activity) => {
                    state.first_same_kind_notification = new_notification;
                    state.kind = StateKind::Awake;
                    action = Some(StateKind::Awake);
                }
            }
        }

        match action {
            Some(StateKind::Asleep) => Deploy::set_all_asleep().await,
            Some(StateKind::Awake) => Deploy::set_all_awake().await,
            None => Ok(()),
        }
        .map_err(|err| StateError::ControllerError(err))
    }

    pub async fn update_from_metrics(
        new_metrics: HashMap<String, HashMap<String, u64>>,
    ) -> Result<(), StateError> {
        let notif: Notification = State::create_notification_from_metrics(&new_metrics)?;

        // Update notification
        State::update_from_notif(notif).await?;

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
            first_same_kind_notification: Notification {
                kind: NotificationKind::Activity,
                timestamp: Instant::now(),
            },
            kind: StateKind::Awake,
            metrics: Default::default(),
        }
    }
}
