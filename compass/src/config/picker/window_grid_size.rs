use crate::{Error, InputError, Result};

use anyhow::anyhow;
use serde::de;

#[derive(Debug, Clone, Copy)]
pub struct WindowGridSize {
    half: u32,
}

impl Default for WindowGridSize {
    fn default() -> Self {
        Self { half: 3 }
    }
}

impl<'de> de::Deserialize<'de> for WindowGridSize {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct WindowGridSizeVisitor;

        impl de::Visitor<'_> for WindowGridSizeVisitor {
            type Value = WindowGridSize;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an even natural number bigger than 0")
            }

            fn visit_i64<E>(self, n: i64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                match n.try_into() {
                    Ok(n) => Ok(n),
                    _ => Err(E::invalid_value(
                        de::Unexpected::Signed(n),
                        &"an even natural number bigger than 0",
                    )),
                }
            }
        }

        deserializer.deserialize_i64(WindowGridSizeVisitor)
    }
}

impl TryFrom<i32> for WindowGridSize {
    type Error = Error;

    fn try_from(num: i32) -> Result<Self> {
        let num: u32 = num.try_into().map_err(|e| anyhow!("{e}"))?;
        if num % 2 != 0 {
            return Err(InputError::FunctionArguments(format!("provided i32 number: {} is not an even number, but it should be for the window grid size", num)))?;
        }
        if num == 0 {
            return Err(InputError::FunctionArguments(
                "provided i32 number is 0 which is invalid for window grid size".to_owned(),
            ))?;
        }

        Ok(WindowGridSize { half: num / 2 })
    }
}

impl TryFrom<i64> for WindowGridSize {
    type Error = Error;

    fn try_from(num: i64) -> Result<Self> {
        let num: u32 = num.try_into().map_err(|e| anyhow!("{e}"))?;
        if num % 2 != 0 {
            return Err(InputError::FunctionArguments(format!("provided i64 number: {} is not an even number, but it should be for the window grid size", num)))?;
        }
        if num == 0 {
            return Err(InputError::FunctionArguments(
                "provided i64 number is 0 which is invalid for window grid size".to_owned(),
            ))?;
        }

        Ok(WindowGridSize { half: num / 2 })
    }
}

impl From<WindowGridSize> for i32 {
    fn from(val: WindowGridSize) -> Self {
        (val.half * 2).try_into().unwrap()
    }
}

impl From<WindowGridSize> for u32 {
    fn from(val: WindowGridSize) -> Self {
        val.half * 2
    }
}

impl From<WindowGridSize> for usize {
    fn from(val: WindowGridSize) -> Self {
        (val.half * 2).try_into().unwrap()
    }
}

impl<'a> TryFrom<&'a str> for WindowGridSize {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self> {
        let i = value.parse::<i32>().map_err(|_| {
            InputError::FunctionArguments(format!(
                "provided a non integer value: {} as window grid size",
                value
            ))
        })?;

        i.try_into()
    }
}
