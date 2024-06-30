use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TrackerConfig {
    #[serde(default = "default_enable")]
    pub enable: bool,
}

fn default_enable() -> bool {
    true
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            enable: default_enable(),
        }
    }
}
