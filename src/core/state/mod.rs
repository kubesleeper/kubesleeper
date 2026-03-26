use crate::core::k8s::{
    deployment::{KsDeploymentFetchingError, KsDeploymentInteractError}, service::{KsServiceFetchingError, KsServiceInteractError},
};

pub mod notification;
pub mod state;
pub mod state_kind;

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Kubernetes error : {0}")]
    KubeError(#[from] kube::Error),

    #[error("LockError : {0}")]
    LockError(String),

    #[error(transparent)]
    KsDeploymentInteractError(#[from] KsDeploymentInteractError),
    
    #[error(transparent)]
    KsServiceInteractError(#[from] KsServiceInteractError),
    
    #[error(transparent)]
    KsDeploymentFetchingError(#[from] KsDeploymentFetchingError),
    
    #[error(transparent)]
    KsServiceFetchingError(#[from] KsServiceFetchingError),

    #[error("Invalid State Kind: {0}")]
    InvalidStateKindError(String),
}
