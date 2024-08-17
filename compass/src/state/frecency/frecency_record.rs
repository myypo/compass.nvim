use super::{FrecencyType, FrecencyWeight};
use crate::common_types::Timestamp;

use bitcode::{Decode, Encode};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Decode, Encode)]
pub struct FrecencyRecord {
    pub timestamp: Timestamp,
    typ: FrecencyType,
}

impl FrecencyRecord {
    pub fn new(typ: FrecencyType) -> Self {
        Self {
            timestamp: Utc::now().into(),
            typ,
        }
    }

    pub fn score(&self) -> FrecencyWeight {
        let days = {
            let time = DateTime::from_timestamp(self.timestamp.into(), 0).unwrap();
            let dur = time.signed_duration_since(Utc::now());
            FrecencyWeight::from(dur)
        };

        let typ = self.typ.weight();

        days + typ
    }
}
