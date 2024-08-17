use serde::Deserialize;

use crate::state::frecency::FrecencyWeight;

#[derive(Default, Debug, Deserialize)]
pub struct FrecencyConfig {
    #[serde(default)]
    pub time_bucket: BucketTimeConfig,
    #[serde(default)]
    pub visit_type: VisitTypeConfig,
    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: i64,
}

#[derive(Debug, Deserialize)]
pub struct VisitTypeConfig {
    #[serde(default = "default_create_visit")]
    pub create: FrecencyWeight,
    #[serde(default = "default_update_visit")]
    pub update: FrecencyWeight,
    #[serde(default = "default_relative_goto_visit")]
    pub relative_goto: FrecencyWeight,
    #[serde(default = "default_absolute_goto_visit")]
    pub absolute_goto: FrecencyWeight,
}

fn default_create_visit() -> FrecencyWeight {
    50.into()
}
fn default_update_visit() -> FrecencyWeight {
    100.into()
}
fn default_relative_goto_visit() -> FrecencyWeight {
    50.into()
}
fn default_absolute_goto_visit() -> FrecencyWeight {
    100.into()
}

impl Default for VisitTypeConfig {
    fn default() -> Self {
        Self {
            create: default_create_visit(),
            update: default_update_visit(),
            relative_goto: default_relative_goto_visit(),
            absolute_goto: default_absolute_goto_visit(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BucketTimeConfig {
    #[serde(default = "default_thresholds_bucket")]
    pub thresholds: Vec<BucketTime>,
    #[serde(default = "default_fallback_bucket")]
    pub fallback: FrecencyWeight,
}

fn default_thresholds_bucket() -> Vec<BucketTime> {
    vec![
        BucketTime {
            hours: 4,
            weight: 100.into(),
        },
        BucketTime {
            hours: 14,
            weight: 70.into(),
        },
        BucketTime {
            hours: 31,
            weight: 50.into(),
        },
        BucketTime {
            hours: 90,
            weight: 30.into(),
        },
    ]
}
fn default_fallback_bucket() -> FrecencyWeight {
    10.into()
}

impl Default for BucketTimeConfig {
    fn default() -> Self {
        Self {
            thresholds: default_thresholds_bucket(),
            fallback: default_fallback_bucket(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BucketTime {
    pub hours: i64,
    pub weight: FrecencyWeight,
}

fn default_cooldown_seconds() -> i64 {
    60
}
