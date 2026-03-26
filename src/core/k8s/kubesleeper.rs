use std::env;

use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    Api, Client,
    api::{ListParams, ObjectList},
};
use tracing::debug;

use crate::core::k8s::constantes::{
    KUBESLEEPER_SELECTOR_KEY, KUBESLEEPER_SELECTOR_VALUE, KUBESLLEPER_APP_NAME,
};

#[derive(Debug, thiserror::Error)]
pub enum KubesleeperError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("No kubesleeper deployment found")]
    MissingKubesleeperDeploy,

    #[error("Found {0} kubesleeper deployments, kubesleeper deployment must be unique")]
    TooMuchKubesleeperDeploy(usize),

    #[error("Missing label {label}", label = KUBESLEEPER_SELECTOR_KEY)]
    MissingLabel(),

    #[error("Label {label} as invalid value, get {0} expect {value}", label = KUBESLEEPER_SELECTOR_KEY, value = KUBESLEEPER_SELECTOR_VALUE)]
    InvalidLabel(String),
}

pub async fn check_kubesleeper() -> Result<(), KubesleeperError> {
    if let Ok(external_mod) = env::var("KS_EXTERNAL_MOD") && external_mod == "true" {
        return Ok(());
    }
    
    // Get all kubesleeper candidates
    let kubesleeper_field_identifier = format!("metadata.name={KUBESLLEPER_APP_NAME}");

    let lp = ListParams::default()
        .match_any()
        .fields(&kubesleeper_field_identifier);

    let client = Client::try_default().await?;
    let kubesleeper: ObjectList<Deployment> = Api::all(client).list(&lp).await?;

    // Check if only 1 kubesleeper exist
    let nb_kubesleeper = kubesleeper.iter().count();
    if nb_kubesleeper > 1 {
        return Err(KubesleeperError::TooMuchKubesleeperDeploy(nb_kubesleeper));
    };

    let ks = match kubesleeper.into_iter().next() {
        Some(k) => Ok(k),
        None => Err(KubesleeperError::MissingKubesleeperDeploy),
    }?;

    let id = format!(
        "kubesleeper deployment ({}/{})",
        ks.metadata.namespace.as_deref().unwrap_or("?"),
        ks.metadata.name.as_deref().unwrap_or("?")
    );

    // Check label
    let labels = ks
        .metadata
        .labels
        .as_ref()
        .ok_or(KubesleeperError::MissingLabel())?;

    if let Some(app_value) = labels.get(KUBESLEEPER_SELECTOR_KEY) {
        if app_value != KUBESLEEPER_SELECTOR_VALUE {
            return Err(KubesleeperError::InvalidLabel(app_value.to_string()));
        }
    } else {
        return Err(KubesleeperError::MissingLabel());
    };

    debug!("Kubesleeper deployment found : {id}");
    Ok(())
}
