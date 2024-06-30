use crate::config::SignText;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Signs {
    #[serde(default)]
    pub past: SignText,
    #[serde(default)]
    pub close_past: SignText,

    #[serde(default)]
    pub future: SignText,
    #[serde(default)]
    pub close_future: SignText,
}
