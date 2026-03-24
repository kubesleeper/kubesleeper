pub mod controller_config;
mod groups;
pub mod server_config;
mod validator;

use crate::core::config::controller_config::ControllerConfig;
use crate::core::config::groups::Group;
use crate::core::config::server_config::ServerConfig;
use crate::core::config::validator::validator;
use crate::core::resource::identifier::Identifier;
use crate::core::resource::resource_name::ResourceName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::log::info;
use tracing::{debug, warn};

const DEFAULT_CONFIG_FILE_PATH: &str = "kubesleeper.yaml";

#[derive(Default, Serialize, Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
// TODO: Rename ServerConfig to Server and rename ControllerConfig to another name more explicit than "controller" for key
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub controller: ControllerConfig,

    #[serde(default)]
    pub groups: HashMap<String, Group>,

    #[serde(default)]
    pub auto_managed_namespace: Vec<ResourceName>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Can't open {path} : {err}.")]
    IOError { path: PathBuf, err: std::io::Error },

    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error("Invalid config file extension '{0}': expected yaml.")]
    InvalidFileExtension(String),

    #[error("File not found : '{0}'")]
    FileNotFound(String),

    #[error("Validation error(s) : {}", display_validation_errors(.0))]
    ValidationErrors(Vec<ValidationError>),
}

pub fn display_validation_errors(errors: &Vec<ValidationError>) -> String {
    errors.iter().fold(String::new(), |acc, val| {
        format!("{}\n  - {}", acc, val.to_string())
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Service '{0}' is present in different groups.")]
    ServiceConflict(Identifier),

    #[error("Deployment '{0}' is present in different groups.")]
    DeployConflict(Identifier),
}

pub fn parse(path: Option<PathBuf>) -> Result<Config, ConfigError> {
    let path: Option<PathBuf> = match path {
        // Config file path explicitly set
        Some(p) => {
            match p.exists() {
                true => {
                    info!("Config file found : {}", p.to_str().unwrap_or_default());
                    Ok(())
                }
                false => Err(ConfigError::FileNotFound(
                    p.to_str().unwrap_or_default().to_string(),
                )),
            }?;

            match p.extension().unwrap().to_str().unwrap_or_default() {
                "yaml" => Ok(()),
                "yml" => {
                    warn!("Config file has extension 'yml'. It is recommended to use yaml.");
                    Ok(())
                }
                e => Err(ConfigError::InvalidFileExtension(e.to_string())),
            }?;
            Some(p)
        }

        None => {
            let p = PathBuf::from_str(DEFAULT_CONFIG_FILE_PATH)
                .expect("Config file path must be parsable at this point");
            match p.exists() {
                true => {
                    debug!("Config file ({DEFAULT_CONFIG_FILE_PATH}) found");
                    Some(p)
                }
                false => {
                    warn!("No config found : using default values");
                    None
                }
            }
        }
    };

    let config = match path {
        None => Config::default(),
        Some(path) => {
            let file =
                std::fs::File::open(&path).map_err(|err| ConfigError::IOError { path, err })?;

            serde_yaml::from_reader(file)?
        }
    };

    validator(&config)?;

    Ok(config)
}
