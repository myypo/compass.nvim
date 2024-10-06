use crate::{
    common_types::{Direction, Timestamp},
    state::{frecency::FrecencyType, Tick},
    viml::CompassArgs,
    Error, InputError, Result,
};
use macros::FromLua;

use nvim_oxi::api::Buffer;
use serde::Deserialize;

#[derive(Debug, Deserialize, FromLua)]
#[serde(rename_all = "snake_case")]
pub enum GotoOptions {
    Relative(RelativeOptions),
    Absolute(AbsoluteOptions),
}

impl Default for GotoOptions {
    fn default() -> Self {
        Self::Relative(RelativeOptions::default())
    }
}

#[derive(Debug, Deserialize)]
pub struct RelativeOptions {
    pub direction: Direction,
}

impl Default for RelativeOptions {
    fn default() -> Self {
        Self {
            direction: Direction::Back,
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum AbsoluteTarget {
    Time(TimeTarget),
    Tick(TickTarget),
    Index(usize),
}

#[derive(Debug, Deserialize)]
pub struct TickTarget {
    pub buf: Buffer,
    pub tick: Tick,
}

#[derive(Debug, Deserialize)]
pub struct TimeTarget {
    pub buf: Buffer,
    pub timestamp: Timestamp,
}

#[derive(Debug, Deserialize)]
pub struct AbsoluteOptions {
    pub target: AbsoluteTarget,
}

impl TryFrom<CompassArgs<'_>> for GotoOptions {
    type Error = Error;

    fn try_from(value: CompassArgs) -> Result<Self> {
        let Some(&sub) = value.sub_cmds.first() else {
            Err(InputError::FunctionArguments(
                "no `goto` subcommand provided".to_owned(),
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
            "absolute" => {
                if let Some(str_tick) = value.map_args.get("tick").copied() {
                    let target_tick: TickTarget =
                        serde_json::from_str(str_tick).map_err(InputError::Json)?;
                    return Ok(Self::Absolute(AbsoluteOptions {
                        target: AbsoluteTarget::Tick(target_tick),
                    }));
                }

                if let Some(str_time) = value.map_args.get("time").copied() {
                    let target_time: TimeTarget =
                        serde_json::from_str(str_time).map_err(InputError::Json)?;
                    return Ok(Self::Absolute(AbsoluteOptions {
                        target: AbsoluteTarget::Time(target_time),
                    }));
                }

                if let Some(index_str) = value.map_args.get("index").copied() {
                    let index: usize = index_str.parse().map_err(InputError::Int)?;
                    return Ok(Self::Absolute(AbsoluteOptions {
                        target: AbsoluteTarget::Index(index),
                    }));
                };

                Err(InputError::FunctionArguments(
                    "have chosen `absolute` but did not provide coordinates in a valid format"
                        .to_owned(),
                ))?
            }

            sub => Err(InputError::FunctionArguments(format!(
                "unknown `goto` subcommand provided: {}",
                sub
            )))?,
        }
    }
}

impl From<GotoOptions> for FrecencyType {
    fn from(value: GotoOptions) -> Self {
        match value {
            GotoOptions::Relative(_) => Self::RelativeGoto,
            GotoOptions::Absolute(_) => Self::AbsoluteGoto,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn can_turn_compass_args_object() {
        let mut map_args: HashMap<&str, &str> = HashMap::new();
        map_args.insert("tick", r#"{"buf":153,"tick":42}"#);

        let args = CompassArgs {
            main_cmd: "goto",
            sub_cmds: vec!["absolute"],
            map_args,
        };

        let got: GotoOptions = args.try_into().unwrap();

        match got {
            GotoOptions::Absolute(AbsoluteOptions { target }) => match target {
                AbsoluteTarget::Tick(TickTarget { tick, .. }) => {
                    assert_eq!(tick, 42.into());
                }

                _ => panic!("got: {:?}", target),
            },

            _ => panic!("got: {:?}", got),
        }
    }
}
