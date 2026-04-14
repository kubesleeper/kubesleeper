use tokio::sync::watch::Receiver;

use crate::core::{ingress::AllServiceConnections, k8s::identifier::Identifier, state::state::State};

#[derive(Debug)]
pub struct Group {
    pub name: String,
    pub deployments: Vec<Identifier>,
    pub services: Vec<Identifier>,
    pub state: State
}

impl Group{
    async fn run(&self, mut rx: Receiver<AllServiceConnections>) {
        while rx.changed().await.is_ok() {
            self.state. // <- HERE
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
