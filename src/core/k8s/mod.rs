pub(crate) mod annotations;
pub mod deployment;
pub mod identifier;
pub mod kubesleeper;
pub mod pod;
pub mod resource_name;
pub mod service;

#[rustfmt::skip]
pub mod constantes{
    pub const KUBESLEEPER_ANNOTATION_PREFIX     : &str = "kubesleeper/";
    pub const ANNOTATION_REPLICAS_KEY     : &str = "replicas";
    pub const ANNOTATION_SELECTOR_KEY     : &str = "selectors";
    pub const ANNOTATION_PORTS_KEY        : &str = "ports";
    pub const KUBESLLEPER_APP_NAME              : &str = "kubesleeper";

    pub const KUBESLEEPER_SELECTOR_KEY   : &str = "app";
    pub const KUBESLEEPER_SELECTOR_VALUE : &str = "kubesleeper";
    
    pub const KUBESLEEPER_SERVER_PORT     : i32 = 8000;
}
