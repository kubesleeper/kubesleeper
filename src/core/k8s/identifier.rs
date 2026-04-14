use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::core::k8s::{
    identifier::error::IdentifierError,
    resource_name::{ResourceName, error::ResourceNameError},
};

pub mod error {
    use crate::core::k8s::resource_name::error::ResourceNameError;

    #[derive(Debug, thiserror::Error)]
    pub enum IdentifierError {
        #[error("Invalid format: '{field_name}' do not match namespace/name: {error}")]
        IdentifierParsingError {
            field_name: String,
            error: ResourceNameError,
        },

        #[error("Invalid format: namespace couldn't be '*', found '{0}'")]
        WildCardInNamesapceError(String),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
            let namespace: ResourceName = namespace.to_string().try_into().map_err(|e| {
                Self::Error::IdentifierParsingError {
                    field_name: value.to_string(),
                    error: e,
                }
            })?;
            if namespace.is_star() {
                return Err(Self::Error::WildCardInNamesapceError(value));
            }

            let name =
                name.to_string()
                    .try_into()
                    .map_err(|e| Self::Error::IdentifierParsingError {
                        field_name: value.to_string(),
                        error: e,
                    })?;

            Ok(Identifier { namespace, name })
        } else {
            Err(Self::Error::IdentifierParsingError {
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

impl From<Identifier> for String {
    fn from(val: Identifier) -> Self {
        format!("{}/{}", val.namespace, val.name)
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.namespace, self.name)
    }
}

impl Identifier {
    pub fn new_unknow() -> Identifier {
        let r = ResourceName::try_from("unknow".to_string()).unwrap();
        Identifier {
            namespace: r.clone(),
            name: r,
        }
    }
}
