mod groups;
pub mod server_config;
pub mod controller_config;

use crate::core::config::controller_config::ControllerConfig;
use crate::core::config::groups::Group;
use crate::core::config::server_config::ServerConfig;
use crate::core::resource::identifier::Identifier;
use crate::core::resource::resource_name::ResourceName;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::num::{NonZeroU16, NonZeroU32};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tracing::log::info;
use tracing::{debug, warn};



const DEFAULT_CONFIG_FILE_PATH: &str = "kubesleeper.yaml";

#[derive(Default, Serialize, Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
// TODO: Rename ServerConfig to Server and rename ControllerConfig to another name more explicit than "controller" for key
// TODO: Verify that there is no overlap between groups and auto_managed_namespace
// TODD: check non-empty list
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub controller: ControllerConfig,

    #[serde(default)]
    pub groups: Vec<Group>,

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

pub fn display_validation_errors(errors : &Vec<ValidationError>) -> String{
    errors.iter().fold(String::new(), |acc, val| format!("{}\n  - {}", acc, val.to_string()))
}



#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Service '{0}' is present in different groups.",)]
    ServiceConflict(Identifier),

    #[error("Deployement '{0}' is present in different groups.")]
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


fn validator(config: &Config) -> Result<(),ConfigError> {
    
    let mut validation_errors = Vec::<ValidationError>::new();
        
    for (i, Group{name: r_name,services: r_services, deploys: r_deploys}) in config.groups.iter().enumerate() {
        for Group{name: l_name ,services: l_services, deploys: l_deploys} in config.groups[i+1..].iter() {
            if l_name != r_name  {
                
                // services
                let dup_services = l_services.iter().filter(|e| r_services.contains(e)).collect::<Vec<&Identifier>>();
                validation_errors.append(
                    &mut dup_services.into_iter().map(
                        |d| ValidationError::ServiceConflict(d.clone())
                    )
                    .collect::<Vec<ValidationError>>()
                );
                
                // deploys
                let dup_deploys = l_deploys.iter().filter(|e| r_deploys.contains(e)).collect::<Vec<&Identifier>>();
                validation_errors.append(
                    &mut dup_deploys.into_iter().map(
                        |d| ValidationError::DeployConflict(d.clone())
                    )
                    .collect::<Vec<ValidationError>>()
                );
            }
        }
    }
    
    // TODO : check auto_manage <=> groups
    // TODO : check auto_manage <=> auto_manage

    match validation_errors.len() {
        0 => Ok(()),
        _ => Err(ConfigError::ValidationErrors(validation_errors)) 
    }
}
