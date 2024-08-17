use std::ops::Deref;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SignText(String);

impl From<SignText> for String {
    fn from(value: SignText) -> String {
        value.0
    }
}

impl From<String> for SignText {
    fn from(value: String) -> SignText {
        SignText(value)
    }
}

impl Deref for SignText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
