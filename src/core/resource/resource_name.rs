use std::fmt::{Display, Formatter};
use crate::core::resource::resource_name::error::ResourceNameError;
use regex::Regex;
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum ResourceNameError {
        #[error(
            "'{0}' should follow RFC1123 (https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#dns-label-names)"
        )]
        InvalidName(String),
    }
}

/// Resource name should follow RFC1123:
/// * Name should contain no more than 63 characters.
/// * Name should contain only lowercase alphanumeric characters, `-` or `.`.
/// * Name should start with an alphanumeric character.
/// * Name should end with an alphanumeric character.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ResourceName(String);

impl TryFrom<String> for ResourceName {
    type Error = ResourceNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"^[a-z]([a-z0-9\-.]{0,61}[a-z])?$").unwrap());

        if re.is_match(&*value) {
            Ok(ResourceName(value))
        } else {
            Err(ResourceNameError::InvalidName(value.to_string()))
        }
    }
}

impl Display for ResourceName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Into<String> for ResourceName {
    fn into(self) -> String {
        format!("{}", self.0)
    }
}
