use k8s_openapi::api::core::v1::Pod;
use kube::{ResourceExt, runtime::reflector::Lookup};

use crate::core::k8s::{
    identifier::Identifier,
    resource_name::{ResourceName, error::ResourceNameError},
};

#[derive(Debug, thiserror::Error)]
pub enum KsPodParsingError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("Missing 'name'")]
    MissingNameError,
    #[error("Invalid 'name' : {0}")]
    InvalidNameError(ResourceNameError),

    #[error("Missing 'namespace'")]
    MissingNamespaceError,
    #[error("Invalid 'namespace' : {0}")]
    InvalidNamespaceError(ResourceNameError),
}

pub trait KsPod {
    fn ks_id(&self) -> Result<Identifier, KsPodParsingError>;
}

impl KsPod for Pod {
    fn ks_id(&self) -> Result<Identifier, KsPodParsingError> {
        let name: ResourceName = self
            .name()
            .ok_or(KsPodParsingError::MissingNameError)?
            .to_string()
            .try_into()
            .map_err(|e| KsPodParsingError::InvalidNameError(e))?;

        let namespace = ResourceExt::namespace(self)
            .ok_or(KsPodParsingError::MissingNamespaceError)?
            .try_into()
            .map_err(|e| KsPodParsingError::InvalidNamespaceError(e))?;

        Ok(Identifier { namespace, name })
    }
}
