use crate::core::k8s::deployment::KsDeployment;
use crate::core::k8s::service::KsService;
use crate::core::scheduler::metric::ArcAllServiceConnections;
use crate::core::state::state::STATE;
use crate::core::state::state_kind::StateKind;
use crate::core::{
    ingress::AllServiceConnections, k8s::identifier::Identifier, state::state::State,
};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use log::error;
use rocket::futures::TryFutureExt;
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
            let legacy_state_kind = self.state.kind; 
            self.state
                .update_from_metrics(&self.services, new_metrics)
                .await
                .unwrap_or_else(|e| error!("Failed to manage state of group '{}' : {e}",self.name));
            
            if self.state.kind == legacy_state_kind{
                continue;
            }
            
            for deploy_id in self.deployments.iter() {
                
                let deploy = match Deployment::get(&deploy_id).await {
                    Err(e) => {
                        error!("Failed to manage resources of group '{}' : Failed to get Deployment '{}' : {e}", self.name, deploy_id);
                        continue;
                    },
                    Ok(d) => d
                };
                
                if let Err(e) = match self.state.kind{
                    StateKind::Asleep => deploy.ks_sleep().await,
                    StateKind::Awake => deploy.ks_wake().await,
                }{
                    error!("Failed to manage resources of group '{}' : Failed to set Deployment '{}' {} : {e}", self.name, deploy_id, self.state.kind);
                    continue;
                }
            }
            
            for service_id in self.services.iter() {
                
                let service = match Deployment::get(&service_id).await {
                    Err(e) => {
                        error!("Failed to manage resources of group '{}' : Failed to get Service '{}' : {e}", self.name, service_id);
                        continue;
                    },
                    Ok(d) => d
                };
                
                if let Err(e) = match self.state.kind{
                    StateKind::Asleep => service.ks_sleep().await,
                    StateKind::Awake => service.ks_wake().await,
                }{
                    error!("Failed to manage resources of group '{}' : Failed to set Service '{}' {} : {e}", self.name, service_id, self.state.kind);
                    continue;
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

// job(
//     wait 30s
//     global data = get_metrics()
//     channel_casting(data)
// )

