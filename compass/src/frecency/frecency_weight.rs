use crate::config::get_config;
use std::ops::Add;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy, Deserialize)]
#[serde(transparent)]
pub struct FrecencyWeight(i64);

impl Add for FrecencyWeight {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl From<i64> for FrecencyWeight {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<chrono::Duration> for FrecencyWeight {
    fn from(value: chrono::Duration) -> Self {
        let conf = &get_config().frecency.time_bucket;

        let hours = value.num_hours();
        conf.thresholds
            .iter()
            .find(|b| b.hours > hours)
            .map(|b| b.weight)
            .unwrap_or_else(|| conf.fallback)
    }
}
