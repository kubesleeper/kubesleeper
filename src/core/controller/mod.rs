use tracing::info;

use crate::core::controller::deploy::Deploy;

pub mod annotations;
pub mod deploy;
pub mod service;

#[rustfmt::skip]
pub mod constantes{
    pub const KUBESLEEPER_ANNOTATION_PREFIX     : &str = "kubesleeper/";
    pub const ANNOTATION_STORE_REPLICAS_KEY     : &str = "store.replicas";
    pub const ANNOTATION_STORE_STATE_KEY        : &str = "store.state";
    pub const ANNOTATION_STORE_SELECTOR_KEY     : &str = "store.selectors";
    pub const ANNOTATION_STORE_PORTS_KEY        : &str = "store.ports";
    pub const KUBESLEEPER_SERVER_SELECTOR_KEY   : &str = "app";

    pub const KUBESLEEPER_SERVER_SELECTOR_VALUE : &str = "kubesleeper";
    pub const KUBESLEEPER_SERVER_LABEL_KEY      : &str = "app";
    pub const KUBESLEEPER_SERVER_LABEL_VALUE    : &str = "kubesleeper";
    pub const KUBESLEEPER_SERVER_PORT           : i32 = 8000;
    pub static KUBESLEEPER_NAMESPACE            : std::sync::OnceLock<String> = std::sync::OnceLock::new();
}

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum Controller {
        #[allow(dead_code)]
        #[error("KubeError : {0}")]
        KubeError(#[from] kube::Error),

        #[error("Failed to parse kube resource : {0}")]
        ResourceParse(#[from] ResourceParse),

        #[allow(dead_code)]
        #[error("SerdeJsonError : {0}")]
        SerdeJsonError(#[from] serde_json::Error),

        #[allow(dead_code)]
        #[error("StateKindError : {0}")]
        StateKindError(String),
        
        #[error("No kubesleeper deployment found during deploy parsing.")]
        MissingKubesleeperDeploy,

        #[error("Found {0} kubesleeper deployments during deploy parsing.")]
        TooMuchKubesleeperDeploy(usize),
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ResourceParse {
        #[error("Resource '{id}' : Required value '{value}' is missing on.")]
        MissingValue {
            /// Resource identifier (like "{name}/{namespace}")
            id: String,
            /// name of the missing value
            value: String,
        },

        #[error("Resource '{id}' : Failed to parse value '{value}' : {error}")]
        ParseFailed {
            /// Resource identifier (like "namespace/name").
            id: String,
            /// Name of the value that can't be parsed (e.g., ".spec.replicas").
            value: String,
            /// Parsing error message (e.g., "invalid digit found in string").
            error: String,
        },
    }
}

pub async fn set_kubesleeper_namespace() -> Result<(), error::Controller>{
    constantes::KUBESLEEPER_NAMESPACE.set(Deploy::get_kubesleeper().await?.namespace).unwrap();
    info!("kubesleeper working namespace detected as {}",constantes::KUBESLEEPER_NAMESPACE.get().unwrap());
    Ok(())
}
