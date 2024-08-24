use std::time::Duration;

use globset::{Glob, GlobSet};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct TrackerConfig {
    #[serde(default = "default_enable")]
    pub enable: bool,

    #[serde(default)]
    pub debounce_milliseconds: Debounce,

    #[serde(default = "default_ignored_patterns")]
    pub ignored_patterns: GlobSet,
}

fn default_enable() -> bool {
    true
}

fn default_ignored_patterns() -> GlobSet {
    GlobSet::builder()
        .add(Glob::new("**/.git/**").unwrap())
        .build()
        .unwrap()
}

fn duration_from_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(millis))
}

#[derive(Debug, Deserialize)]
pub struct Debounce {
    #[serde(default = "default_run", deserialize_with = "duration_from_millis")]
    pub run: Duration,
    #[serde(
        default = "default_maintenance",
        deserialize_with = "duration_from_millis"
    )]
    pub maintenance: Duration,
    #[serde(
        default = "default_activate",
        deserialize_with = "duration_from_millis"
    )]
    pub activate: Duration,
}

fn default_run() -> Duration {
    Duration::from_millis(200)
}

fn default_maintenance() -> Duration {
    Duration::from_millis(500)
}

fn default_activate() -> Duration {
    Duration::from_millis(3000)
}

impl Default for Debounce {
    fn default() -> Self {
        Self {
            run: default_run(),
            maintenance: default_maintenance(),
            activate: default_activate(),
        }
    }
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            enable: default_enable(),
            debounce_milliseconds: Debounce::default(),
            ignored_patterns: default_ignored_patterns(),
        }
    }
}
