use crate::{
    config::{get_config, WindowGridSize},
    state::PlaceTypeRecord,
    viml::CompassArgs,
    Error, InputError, Result,
};
use macros::FromLua;

use serde::Deserialize;

#[derive(Deserialize, FromLua)]
pub struct OpenOptions {
    pub record_types: Option<Vec<RecordFilter>>,
    pub max_windows: WindowGridSize,
}

impl Default for OpenOptions {
    fn default() -> Self {
        let conf = &get_config().picker;

        Self {
            record_types: None,
            max_windows: conf.max_windows,
        }
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordFilter {
    Change,
}

impl TryFrom<&str> for RecordFilter {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "change" => Ok(RecordFilter::Change),
            _ => Err(InputError::FunctionArguments(format!(
                "unkwnown filter provided: {}",
                value
            )))?,
        }
    }
}

impl From<PlaceTypeRecord> for RecordFilter {
    fn from(value: PlaceTypeRecord) -> Self {
        match value {
            PlaceTypeRecord::Change(_) => Self::Change,
        }
    }
}

impl<'a> TryFrom<CompassArgs<'a>> for OpenOptions {
    type Error = Error;

    fn try_from(value: CompassArgs<'a>) -> Result<Self> {
        let record_types = value
            .map_args
            .get("record_types")
            .map(|s| serde_json::from_str::<Vec<RecordFilter>>(s))
            .transpose()
            .map_err(InputError::Json)?;

        let max_windows = value
            .map_args
            .get("max_windows")
            .map(|&s| s.try_into())
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            record_types,
            max_windows,
        })
    }
}
