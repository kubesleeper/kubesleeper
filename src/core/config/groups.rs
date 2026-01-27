use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::core::config::ConfigError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    name: String,
    deploys: Vec<Identifier>,
    services: Vec<Identifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
struct Identifier {
    namespace: String,
    name: String,
}

impl TryFrom<String> for Identifier {
    type Error = ConfigError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Some((namespace, name)) = value.split_once("/") {
            trace!("'{value}' parsed: '{namespace}' as namespace and '{name}' as name");
            Ok(Identifier {
                namespace: namespace.to_string(),
                name: name.to_string(),
            })
        } else {
            Err(ConfigError::IdentifierParsing(value))
        }
    }
}

impl Into<String> for Identifier {
    fn into(self) -> String {
        format!("{}/{}", self.namespace, self.name)
    }
}
