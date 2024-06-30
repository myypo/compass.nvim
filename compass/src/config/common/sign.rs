use std::ops::Deref;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SignText(String);

impl Default for SignText {
    fn default() -> Self {
        Self("●".to_owned())
    }
}

impl From<SignText> for String {
    fn from(value: SignText) -> String {
        value.0
    }
}

impl Deref for SignText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
