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
pub struct ChangeOptions {
    #[serde(default = "default_try_update")]
    pub try_update: bool,
}

fn default_try_update() -> bool {
    true
}

impl<'a> TryFrom<CompassArgs<'a>> for PlaceOptions {
    type Error = Error;

    fn try_from(value: CompassArgs<'a>) -> Result<Self> {
        let Some(&sub) = value.sub_cmds.first() else {
            Err(InputError::FunctionArguments(
                "no `place` subcommand provided".to_owned(),
            ))?
        };

        match sub {
            "change" => {
                let try_update = value
                    .map_args
                    .get("try_update")
                    .map(|&s| s.parse::<bool>())
                    .transpose()
                    .map_err(InputError::Bool)?
                    .unwrap_or_else(default_try_update);

                Ok(Self::Change(ChangeOptions { try_update }))
            }

            sub => Err(InputError::FunctionArguments(format!(
                "unknown `place` subcommand provided: {}",
                sub
            )))?,
        }
    }
}
