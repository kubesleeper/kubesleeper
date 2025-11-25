use crate::core::config::config::ConfigError::IOError;
use crate::core::ingress::error::IngressError;
use crate::core::server::error::ServerError;
use crate::core::{controller, logger};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::num::NonZeroU16;
use std::path::PathBuf;
use tokio_cron_scheduler::JobSchedulerError;
use tracing::{error, info, warn};

#[derive(Default, Serialize, Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    server_config: ServerConfig,

    #[serde(default)]
    controller_config: ControllerConfig,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Port of the kubesleeper server
    port: NonZeroU16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: const { NonZeroU16::new(8000).unwrap() },
        }
    }
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct ControllerConfig {
    /// Sleepiness duration in second
    sleepiness_duration: u32,

    /// Time between two activity check in second
    refresh_interval: u32,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        ControllerConfig {
            sleepiness_duration: 15,
            refresh_interval: 5,
        }
    }
}

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum Controller {
        #[allow(dead_code)]
        #[error("File {0} not found")]
        FileNotFound(#[from] kube::Error),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Can't open {path} : {err}.")]
    IOError { path: PathBuf, err: std::io::Error },

    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error("Invalid config file extension '{0}': expected yaml.")]
    InvalidFileExtension(String),
}

pub fn parse(path: PathBuf) -> Result<Config, ConfigError> {
    match &*path.extension().unwrap().to_string_lossy() {
        "yaml" => Ok(()),
        "yml" => {
            warn!("Config file has extension 'yml'. It is recommended to use yaml.");
            Ok(())
        }
        e => Err(ConfigError::InvalidFileExtension(e.to_string())),
    }?;

    let file = std::fs::File::open(&path).map_err(|err| ConfigError::IOError { path, err })?;
    let config: Config = serde_yaml::from_reader(file)?;

    Ok(config)
}
