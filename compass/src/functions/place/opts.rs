use crate::{viml::CompassArgs, Error, InputError, Result};
use macros::FromLua;

use serde::Deserialize;

#[derive(Deserialize, FromLua)]
pub enum PlaceOptions {
    Change(ChangeOptions),
}

impl Default for PlaceOptions {
    fn default() -> Self {
        PlaceOptions::Change(ChangeOptions::default())
    }
}

#[derive(Default, Deserialize)]
pub struct ChangeOptions {}

impl<'a> TryFrom<CompassArgs<'a>> for PlaceOptions {
    type Error = Error;

    fn try_from(value: CompassArgs<'a>) -> Result<Self> {
        let Some(&sub) = value.sub_cmds.first() else {
            Err(InputError::FunctionArguments(
                "no `place` subcommand provided".to_owned(),
            ))?
        };

        match sub {
            "change" => Ok(Self::Change(ChangeOptions {})),

            sub => Err(InputError::FunctionArguments(format!(
                "unknown `place` subcommand provided: {}",
                sub
            )))?,
        }
    }
}
