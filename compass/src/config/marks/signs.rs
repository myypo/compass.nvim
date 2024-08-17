use crate::config::SignText;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Signs {
    #[serde(default = "default_past")]
    pub past: SignText,
    #[serde(default = "default_past")]
    pub close_past: SignText,

    #[serde(default = "default_future")]
    pub future: SignText,
    #[serde(default = "default_future")]
    pub close_future: SignText,
}

fn default_past() -> SignText {
    "◀".to_owned().into()
}

fn default_future() -> SignText {
    "▶".to_owned().into()
}

impl Default for Signs {
    fn default() -> Self {
        Self {
            past: default_past(),
            close_past: default_past(),
            future: default_future(),
            close_future: default_future(),
        }
    }
}
