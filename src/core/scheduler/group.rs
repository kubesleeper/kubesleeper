use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::core::k8s::deployment::KsDeployment;
use crate::core::scheduler::metric::ArcAllServiceConnections;
use crate::core::state::state_kind::StateKind;
use crate::core::{
    k8s::identifier::Identifier, state::state::State,
};
use k8s_openapi::api::apps::v1::Deployment;
use log::{debug, error};
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
        'main: while rx.changed().await.is_ok() {
            let new_metrics = rx.borrow().clone();
            let legacy_state_kind = self.state.kind; 
            self.state
                .update_from_metrics(&self.services, new_metrics)
                .await
                .unwrap_or_else(|e| error!("Failed to manage state of group '{}' : {e}",self.name));
            
            if self.state.kind == legacy_state_kind{
                continue;
            }
            
            // --- SERVICE ---
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
            
            
            // --- DEPLOYMENT ---
            let mut deployments : Vec<Deployment> = Vec::new();
            for deploy_id in self.deployments.iter() {
                let deploy = match Deployment::get(&deploy_id).await {
                    Err(e) => {
                        error!("Failed to manage resources of group '{}' : Failed to get Deployment '{}' : {e}", self.name, deploy_id);
                        continue 'main;
                    },
                    Ok(d) => d
                };
                deployments.push(deploy);
            }
            
            for deployments in deployments.iter() {        
                if let Err(e) = match self.state.kind{
                    StateKind::Asleep => deployments.ks_sleep().await,
                    StateKind::Awake => deployments.ks_wake().await,
                }{
                    error!("Failed to manage resources of group '{}' : Failed to set Deployment '{}' {} : {e}", self.name, deployments.ks_id().unwrap_or(Identifier::new_unknow()), self.state.kind);
                    continue 'main;
                }
            }
            
            if self.state.kind == StateKind::Asleep{
                continue 'main;
            }
            
 
            let time = Instant::now();
            loop {
                let mut is_all_deployment_ready = true; 
                for deployments in deployments.iter() {
                    let (current,target) = match deployments.ks_rediness_state().await {
                        Ok(r) => r,
                        Err(e) => {
                            error!("Failed to manage resources of group '{}' : Failed to get rediness data of Deployment '{}' : {e}", self.name, deployments.ks_id().unwrap_or(Identifier::new_unknow()));
                            continue 'main;
                        }
                    };
                    
                    if current != target {
                        debug!("Deployment '{}' not ready : {current}/{target} pod ready",deployments.ks_id().unwrap_or(Identifier::new_unknow()));
                        is_all_deployment_ready = false;
                    }
                }
                if is_all_deployment_ready {
                    continue 'main;
                }
                
                let max_waiting_time = (15*60) as f32;
                let elapsed = time.elapsed().as_secs() as f32;
                if elapsed >= max_waiting_time {
                    // ?? what to do in this case ? try to sleep the group ? force the state to be asleep ?
                    // because the Deployment will be seen as 'awake' so re-awake them will juste be skiped
                    // so if nothing done, manual intervention is needed
                    error!("Group '{}' failed to be ready after {}m{}s after being set awake", 
                        self.name,
                        (max_waiting_time as i32 / 60),
                        (max_waiting_time as i32 % 60)
                    );
                    continue 'main;
                }
                let wait = ((4.5 * elapsed + 7.5)/(15 as f32)) as u64; // ?? linear interpolation bettwen 'first waiting time is 0.5s' and 'at 15min of waiting, wait 5s'  
                debug!("Group '{}' not ready after {}m{}s : waiting {}s",
                    self.name,
                    (elapsed as i32 / 60),
                    (elapsed as i32 % 60),
                    wait
                );
                sleep(Duration::from_secs(wait));
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

