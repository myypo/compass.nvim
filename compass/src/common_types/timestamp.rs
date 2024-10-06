use std::fmt::Display;

use bitcode::{Decode, Encode};
use chrono::{DateTime, Utc};
use serde::de;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Decode, Encode)]
pub struct Timestamp(i64);

impl<'de> de::Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct TimestampVisitor;

        impl de::Visitor<'_> for TimestampVisitor {
            type Value = Timestamp;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("u64 seconds unix epoch timestamp")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                TryInto::<i64>::try_into(v)
                    .map(Timestamp)
                    .map_err(|e| de::Error::custom(e))
            }
        }

        deserializer.deserialize_u64(TimestampVisitor)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value.timestamp())
    }
}

impl From<Timestamp> for i64 {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(value: Timestamp) -> Self {
        DateTime::from_timestamp(value.into(), 0).unwrap()
    }
}
