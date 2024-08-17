mod frecency_record;
use frecency_record::*;

mod frecency_type;
pub use frecency_type::FrecencyType;

mod frecency_weight;
pub use frecency_weight::FrecencyWeight;

use crate::{common_types::Timestamp, config::get_config};

use bitcode::{Decode, Encode};
use chrono::{DateTime, Utc};

pub trait FrecencyScore {
    fn total_score(&self) -> FrecencyWeight;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Decode, Encode)]
pub struct Frecency {
    vec: Vec<FrecencyRecord>,
}

impl Frecency {
    pub fn new() -> Self {
        Self {
            vec: Vec::from([FrecencyRecord::new(FrecencyType::Create)]),
        }
    }

    pub fn add_record(&mut self, typ: FrecencyType) {
        if self.vec.last().is_some_and(|r| {
            Utc::now().signed_duration_since(Into::<DateTime<Utc>>::into(r.timestamp))
                > chrono::Duration::seconds(get_config().frecency.cooldown_seconds)
        }) {
            self.vec.push(FrecencyRecord::new(typ));
        }
    }

    pub fn latest_timestamp(&self) -> Timestamp {
        // Should never panic since there must always be at least a single record
        self.vec.last().unwrap().timestamp
    }
}

impl FrecencyScore for Frecency {
    fn total_score(&self) -> FrecencyWeight {
        self.vec
            .iter()
            .fold(Into::<FrecencyWeight>::into(0), |acc, r| r.score() + acc)
    }
}
