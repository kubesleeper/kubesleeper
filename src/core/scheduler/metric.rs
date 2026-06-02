use log::{debug, error};
use tokio::sync::watch::Sender;
use tracing::{Level, span};

use crate::core::ingress::{AllServiceConnections, IngressType, traefik::Traefik};
use std::{sync::Arc, time::Duration};
use tokio::time::interval;
pub type ArcAllServiceConnections = Arc<AllServiceConnections>;

pub async fn send_metric(tx: Sender<ArcAllServiceConnections>, refresh_interval: Duration) {
    let span = span!(Level::DEBUG, "send_metric");
    let _enter = span.enter();
    let mut interval = interval(refresh_interval);
    loop {
        interval.tick().await;
        debug!("tick");
        let metrics = match Traefik::get_metrics().await {
            Ok(metrics) => metrics,
            Err(err) => {
                error!(
                    "Failed to send the metrics : Failed to get the metrics : {}",
                    err
                );
                continue;
            }
        };
        let _ = tx.send(Arc::new(metrics)).inspect_err(|err| {
            error!(
                "Failed to send the metrics : Failed to send the data to the channel : {}",
                err
            )
        });
    }
}
