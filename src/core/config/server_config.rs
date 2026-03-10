use serde::{Deserialize, Serialize};
use std::num::NonZeroU16;

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
