use serde::{Deserialize, Deserializer, Serialize};
use std::time::Duration;

#[derive(Serialize, Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ControllerConfig {
    /// Sleepiness duration
    #[serde(deserialize_with = "deserialize_sleepiness_duration")]
    pub sleepiness_duration: Duration,

    /// Time between two activity check
    pub refresh_interval: Duration,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        ControllerConfig {
            sleepiness_duration: const { Duration::new(15, 0) },
            refresh_interval: const { Duration::new(5, 0) },
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
