use super::FrecencyWeight;
use crate::config::get_config;

use bitcode::{Decode, Encode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Decode, Encode)]
pub enum FrecencyType {
    Create,
    Update,
    RelativeGoto,
    AbsoluteGoto,
}

impl FrecencyType {
    pub fn weight(self) -> FrecencyWeight {
        let conf = &get_config().frecency.visit_type;

        match self {
            Self::Create => conf.create,
            Self::Update => conf.update,
            Self::RelativeGoto => conf.relative_goto,
            Self::AbsoluteGoto => conf.absolute_goto,
        }
    }
}
