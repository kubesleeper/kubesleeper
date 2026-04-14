use crate::core::ingress::AllServiceConnections;
use std::sync::Arc;

pub type ArcAllServiceConnections = Arc<AllServiceConnections>;

// #[instrument(
//     name = "schedule"
//     level = "info"
//     skip(uuid)
//     fields(uuid = %uuid)
// )]
// async fn process(uuid: Uuid) {
//     let metrics = Traefik::get_metrics().await;

//     State::update_from_metrics(metrics.map_err(|e| e.to_string()).unwrap())
//         .await
//         .map_err(|e| e.to_string())
//         .unwrap();
// }

// pub async fn create_schedule(refresh_interval: NonZeroU32) -> JobScheduler {
//     let sched = JobScheduler::new().await.unwrap();

//     sched
//         .add(
//             Job::new_async(format!("1/{refresh_interval} * * * * *"), |uuid, mut l| {
//                 Box::pin(async move {
//                     {
//                         process(Uuid::new_v4()).await
//                     }

//                     // Query the next execution time for this job
//                     let _next_tick = l.next_tick_for_job(uuid).await;
//                 })
//             })
//             .unwrap(),
//         )
//         .await
//         .unwrap();
//     info!("Running scheduler");
//     sched
// }
