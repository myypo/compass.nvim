mod signs;
pub use signs::*;

mod update_range;
pub use update_range::*;

use serde::Deserialize;

#[derive(Default, Debug, Deserialize)]
pub struct MarksConfig {
    #[serde(default)]
    pub update_range: UpdateRange,
    #[serde(default)]
    pub signs: Signs,
}
