use kube::Api;

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
    pub const KUBESLLEPER_APP_NAME              : &str = "kubesleeper";

    pub const KUBESLEEPER_SERVER_SELECTOR_VALUE : &str = "kubesleeper";
    #[allow(dead_code)]
    pub const KUBESLEEPER_SERVER_LABEL_KEY      : &str = "app";
    #[allow(dead_code)]
    pub const KUBESLEEPER_SERVER_LABEL_VALUE    : &str = "kubesleeper";
    pub const KUBESLEEPER_SERVER_PORT           : i32 = 8000;
}

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum Resource {
        #[allow(dead_code)]
        #[error("KubeError : {0}")]
        KubeError(#[from] kube::Error),

        #[error("Failed to parse kube resource : {0}")]
        ResourceParse(#[from] ResourceParse),

        #[allow(dead_code)]
        #[error("Failed to retreive k8s resource {id} form parsed resource")]
        K8sResourceNotFound { id: String },

        #[allow(dead_code)]
        #[error("SerdeJsonError : {0}")]
        SerdeJsonError(#[from] serde_json::Error),

        #[allow(dead_code)]
        #[error("StateKindError : {0}")]
        StateKindError(String),

        #[allow(dead_code)]
        #[error("No kubesleeper deployment found during deploy parsing")]
        MissingKubesleeperDeploy,

        #[allow(dead_code)]
        #[error("Found {0} kubesleeper deployments during deploy parsing")]
        TooMuchKubesleeperDeploy(usize),

        #[allow(dead_code)]
        #[error("Max waiting time exceeded ({max_waiting_time}s) for deployment {id}")]
        MaxWaitingWakeTime { id: String, max_waiting_time: u64 },
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ResourceParse {
        #[error("Resource '{id}' : Required value '{value}' is missing on")]
        MissingValue {
            /// Resource identifier (like "{namespace}/{name}")
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

pub trait TargetResource<'a>:
    std::fmt::Display + TryFrom<&'a Self::K8sResource, Error = error::Resource>
{
    type K8sResource: 'a;

    async fn get_all() -> Result<Vec<Self>, error::Resource>;
    async fn get_k8s_api(
        namespace: Option<&str>,
    ) -> Result<Api<Self::K8sResource>, error::Resource>;

    async fn wake(&mut self) -> Result<(), error::Resource>; // uses patch
    async fn sleep(&mut self) -> Result<(), error::Resource>; // uses patch
    async fn patch(&self) -> Result<(), error::Resource>;
    #[allow(dead_code)]
    async fn get_k8s_resource(&self) -> Result<Self::K8sResource, error::Resource>;
}
