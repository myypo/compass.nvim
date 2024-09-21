use serde::Deserialize;
use strum_macros::EnumString;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Deserialize, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Direction {
    #[default]
    Back,
    Forward,
}
