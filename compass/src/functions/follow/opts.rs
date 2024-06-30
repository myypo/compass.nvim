use crate::{
    config::{get_config, WindowGridSize},
    viml::CompassArgs,
    Error, InputError, Result,
};
use macros::FromLua;

use nvim_oxi::api::{get_current_buf, Buffer};
use serde::Deserialize;

#[derive(Deserialize, FromLua)]
pub enum FollowOptions {
    Buf(BufOptions),
}

impl Default for FollowOptions {
    fn default() -> Self {
        Self::Buf(BufOptions::default())
    }
}

#[derive(Deserialize)]
pub struct BufOptions {
    #[serde(default = "get_current_buf")]
    pub target: Buffer,
    #[serde(default = "default_max_windows")]
    pub max_windows: WindowGridSize,
}

fn default_max_windows() -> WindowGridSize {
    get_config().picker.max_windows
}

impl Default for BufOptions {
    fn default() -> Self {
        Self {
            target: get_current_buf(),
            max_windows: default_max_windows(),
        }
    }
}

impl<'a> TryFrom<CompassArgs<'a>> for FollowOptions {
    type Error = Error;

    fn try_from(value: CompassArgs<'a>) -> Result<Self> {
        let Some(&sub) = value.sub_cmds.first() else {
            Err(InputError::FunctionArguments(
                "no `follow` subcommand provided".to_owned(),
            ))?
        };

        match sub {
            "buf" => {
                let target = value
                    .map_args
                    .get("target")
                    .map(|&s| s.parse::<i32>())
                    .transpose()
                    .map_err(InputError::Int)?
                    .map(Into::into)
                    .unwrap_or_else(get_current_buf);

                let max_windows = value
                    .map_args
                    .get("max_windows")
                    .map(|&s| s.try_into())
                    .transpose()?
                    .unwrap_or_default();

                Ok(Self::Buf(BufOptions {
                    target,
                    max_windows,
                }))
            }

            sub => Err(InputError::FunctionArguments(format!(
                "unknown `follow` subcommand provided: {}",
                sub
            )))?,
        }
    }
}
