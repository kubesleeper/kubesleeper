use serde::{Deserialize, Deserializer, Serialize};
use std::num::{NonZeroU16, NonZeroU32};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
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
}

#[derive(Serialize, Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Port of the kubesleeper server
    pub port: NonZeroU16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: const { NonZeroU16::new(8000).unwrap() },
        }
    }
}

#[derive(Serialize, Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ControllerConfig {
    /// Sleepiness duration in second
    #[serde(deserialize_with = "deserialize_sleepiness_duration")]
    pub sleepiness_duration: Duration,

    /// Time between two activity check in second
    pub refresh_interval: NonZeroU32,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        ControllerConfig {
            sleepiness_duration: const { Duration::new(15, 0) },
            refresh_interval: const { NonZeroU32::new(5).unwrap() },
        }
    }
}

fn deserialize_sleepiness_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(seconds))
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
    FileNotFoud(String),
}

pub fn parse(path: Option<PathBuf>) -> Result<Config, ConfigError> {
    let path: Option<PathBuf> = match path {
        // Config file path explicitly set
        Some(p) => {
            match p.exists() {
                true => {
                    debug!("Config file found : {}", p.to_str().unwrap_or_default());
                    Ok(())
                }
                false => Err(ConfigError::FileNotFoud(
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
                .expect("Default config file path must be parsable at this point");
            match p.exists() {
                true => {
                    debug!("Default config file ({DEFAULT_CONFIG_FILE_PATH}) found");
                    Some(p)
                }
                false => {
                    warn!(
                        "Default config file ({DEFAULT_CONFIG_FILE_PATH}) not found : using default values"
                    );
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

    Ok(config)
}
