pub mod common;
pub use common::*;

pub mod frecency;
pub use frecency::*;

pub mod marks;
pub use marks::*;

pub mod picker;
pub use picker::*;

pub mod persistence;
pub use persistence::*;

pub mod tracker;
pub use tracker::*;

static CONFIG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(Config::default)
}
pub fn set_config(conf: Config) {
    // We do not care if the cell was initialized
    let _ = CONFIG.set(conf);
}

#[derive(Debug, serde::Deserialize, Default, macros::FromLua)]
#[serde(default)]
pub struct Config {
    #[serde(default)]
    pub picker: PickerConfig,

    #[serde(default)]
    pub tracker: TrackerConfig,

    #[serde(default)]
    pub marks: MarksConfig,

    #[serde(default)]
    pub persistence: PersistenceConfig,

    #[serde(default)]
    pub frecency: FrecencyConfig,
}
