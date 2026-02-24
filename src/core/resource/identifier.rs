use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::core::resource::resource_name::error::ResourceNameError;
use crate::core::resource::{identifier::error::IdentifierError, resource_name::ResourceName};

pub mod error {
    use crate::core::resource::resource_name::error::ResourceNameError;

    #[derive(Debug, thiserror::Error)]
    pub enum IdentifierError {
        #[error("Invalid format: '{field_name}' do not match namespace/name: {error}")]
        IdentifierParsing {
            field_name: String,
            error: ResourceNameError,
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub struct Identifier {
    pub namespace: ResourceName,
    pub name: ResourceName,
}

impl TryFrom<String> for Identifier {
    type Error = IdentifierError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Some((namespace, name)) = value.split_once("/") {
            trace!("'{value}' parsed: '{namespace}' as namespace and '{name}' as name");
            Ok(Identifier {
                namespace: namespace.to_string().try_into().map_err(|e| {
                    Self::Error::IdentifierParsing {
                        field_name: namespace.to_string(),
                        error: e,
                    }
                })?,
                name: name
                    .to_string()
                    .try_into()
                    .map_err(|e| Self::Error::IdentifierParsing {
                        field_name: namespace.to_string(),
                        error: e,
                    })?,
            })
        } else {
            Err(Self::Error::IdentifierParsing {
                field_name: value.to_string(),
                error: ResourceNameError::InvalidName("".to_string()),
            })
        }
    }
}

impl FromStr for Identifier {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_string())
    }
}

impl Into<String> for Identifier {
    fn into(self) -> String {
        format!("{}/{}", self.namespace, self.name)
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.namespace, self.name)
    }
}
