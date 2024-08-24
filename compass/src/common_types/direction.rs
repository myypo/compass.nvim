use serde::Deserialize;
use strum_macros::EnumString;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Direction {
    #[default]
    Back,
    Forward,
}
