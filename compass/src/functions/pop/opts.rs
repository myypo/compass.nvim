use crate::{common_types::Direction, viml::CompassArgs, Error, InputError, Result};
use macros::FromLua;

use serde::Deserialize;

#[derive(Debug, Deserialize, FromLua)]
#[serde(rename_all = "lowercase")]
pub enum PopOptions {
    Relative(RelativeOptions),
}

impl Default for PopOptions {
    fn default() -> Self {
        Self::Relative(RelativeOptions::default())
    }
}

#[derive(Debug, Deserialize)]
pub struct RelativeOptions {
    #[serde(default)]
    pub direction: Direction,
}

impl Default for RelativeOptions {
    fn default() -> Self {
        Self {
            direction: Direction::Back,
        }
    }
}

impl<'a> TryFrom<CompassArgs<'a>> for PopOptions {
    type Error = Error;

    fn try_from(value: CompassArgs) -> Result<Self> {
        let Some(&sub) = value.sub_cmds.first() else {
            Err(InputError::FunctionArguments(
                "no `pop` subcommand provided".to_owned(),
            ))?
        };

        match sub {
            "relative" => {
                let direction: Direction = value
                    .map_args
                    .get("direction")
                    .copied()
                    .ok_or_else(|| {
                        InputError::FunctionArguments(
                            "have chosen `relative` but not specified the direction".to_owned(),
                        )
                    })?
                    .try_into()
                    .map_err(InputError::EnumParse)?;

                Ok(Self::Relative(RelativeOptions { direction }))
            }

            sub => Err(InputError::FunctionArguments(format!(
                "unknown `pop` subcommand provided: {}",
                sub
            )))?,
        }
    }
}
