use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::core::config::ConfigError;
use crate::core::config::ConfigError::IdentifierParsing;
use crate::core::resource::resource_name::error::ResourceNameError;
use crate::core::resource::resource_name::ResourceName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    name: String,
    deploys: Vec<Identifier>,
    services: Vec<Identifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
struct Identifier {
    namespace: ResourceName,
    name: String,
}

impl TryFrom<String> for Identifier {
    type Error = ConfigError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Some((namespace, name)) = value.split_once("/") {
            trace!("'{value}' parsed: '{namespace}' as namespace and '{name}' as name");
            Ok(Identifier {
                namespace: ResourceName::try_from(namespace.to_string()).map_err(|e| {
                    IdentifierParsing {
                        field_name: namespace.to_string(),
                        error: e,
                    }
                })?,
                name: name.to_string(),
            })
        } else {
            Err(IdentifierParsing {
                field_name: value.to_string(),
                error: ResourceNameError::InvalidName("".to_string()),
            })
        }
    }
}

impl Into<String> for Identifier {
    fn into(self) -> String {
        format!("{}/{}", self.namespace, self.name)
    }
}
