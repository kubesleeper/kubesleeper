use crate::core::scheduler::metric::ArcAllServiceConnections;
use crate::core::state::state::STATE;
use crate::core::{
    ingress::AllServiceConnections, k8s::identifier::Identifier, state::state::State,
};
use log::error;
use tokio::sync::watch::Receiver;

#[derive(Debug)]
pub struct Group {
    pub name: String,
    pub deployments: Vec<Identifier>,
    pub services: Vec<Identifier>,
    pub state: State,
}

impl Group {
    pub(crate) async fn run(&mut self, mut rx: Receiver<ArcAllServiceConnections>) {
        while rx.changed().await.is_ok() {
            let new_metrics = rx.borrow().clone();
            self.state
                .update_from_metrics(&self.services, new_metrics)
                .await
                .unwrap_or_else(|e| error!("{e}"));
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

// scheduler{
//     group.foreach -> job({
//         while(listen(channel))

//         state.update(data)
//     })
// }
