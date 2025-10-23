pub mod deploy;
pub mod kube;
pub mod service;

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum ControllerError {
        #[allow(dead_code)]
        #[error("KubeError : {0}")]
        KubeError(#[from] kube::Error),

        #[allow(dead_code)]
        #[error("ResourceDataError : {0}")]
        ResourceDataError(String),

        #[allow(dead_code)]
        #[error("SerdeJsonError : {0}")]
        SerdeJsonError(#[from] serde_json::Error),
    }
}
