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
    pub const KUBESLEEPER_SERVER_PORT           : i32 = 8080;
    // pub const SERVER_SELECTOR                   : (&str, &str) = ("app", "kubesleeper");
}


pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum Controller {
        #[allow(dead_code)]
        #[error("KubeError : {0}")]
        KubeError(#[from] kube::Error),

        #[error("Can't parse resource : {0}")]
        ParseResource(#[from] ParseResource),

        #[allow(dead_code)]
        #[error("SerdeJsonError : {0}")]
        SerdeJsonError(#[from] serde_json::Error),

        #[allow(dead_code)]
        #[error("StateKindError : {0}")]
        StateKindError(String),
    }



    #[derive(Debug, thiserror::Error)]
    pub enum ParseResource {
        #[error("Required value '{resource}' is missing on resource '{id}'.")]
        MissingValue{
            /// Resource identifier (like "{name}/{namespace}")
            id: String,
            /// name of the missing value
            resource: String
        },
        
        #[error("Failed to parse value '{resource}' of resource '{id}' : {error}.")]
        Failed {
            /// Resource identifier (like "namespace/name").
            id: String,
            /// Name of the value that can't be parsed (e.g., ".spec.replicas").
            resource: String,
            /// Parsing error message (e.g., "invalid digit found in string").
            error: String,
        },
    }
}


