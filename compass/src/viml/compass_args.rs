use crate::{Error, InputError, Result, VimlError};
use std::collections::HashMap;

pub struct CompassArgs<'a> {
    pub main_cmd: &'a str,
    pub sub_cmds: Vec<&'a str>,
    pub map_args: HashMap<&'a str, &'a str>,
}

impl<'a> TryFrom<Vec<&'a str>> for CompassArgs<'a> {
    type Error = Error;

    fn try_from(value: Vec<&'a str>) -> Result<Self> {
        let mut iter = value.iter().peekable();

        let main_cmd = iter
            .next()
            .ok_or(InputError::Viml(VimlError::InvalidCommand(format!(
                "no main command specified in: {:?}",
                value
            ))))?;

        let mut sub_cmds: Vec<&str> = Vec::new();
        while let Some(s) = iter.peek() {
            if s.contains('=') {
                break;
            }

            // We know it is Some because of the outer peek
            sub_cmds.push(iter.next().unwrap());
        }

        let map_args = iter
            .map(|s| -> Option<(&str, &str)> { s.split_once('=') })
            .collect::<Option<HashMap<&str, &str>>>()
            .ok_or(InputError::Viml(VimlError::InvalidCommand(format!(
                "nothing found after = sign in: {:?}",
                value,
            ))))?;

        Ok(Self {
            main_cmd,
            sub_cmds,
            map_args,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_basic_command() {
        let cmd: Vec<&str> = vec!["goto", "relative", "direction=forward", "some_id=42"];

        let got = CompassArgs::try_from(cmd).unwrap();

        assert_eq!(got.main_cmd, "goto".to_owned());
        assert_eq!(*got.sub_cmds.first().unwrap(), "relative");

        let mut want_map = HashMap::new();
        want_map.insert("direction", "forward");
        want_map.insert("some_id", "42");
        assert_eq!(got.map_args, want_map)
    }

    #[test]
    fn can_parse_dictionary_command() {
        let cmd: Vec<&str> = vec![
            "find_files",
            "hidden=true",
            r#"layout_config={"prompt_position":"top"}"#,
        ];

        let got = CompassArgs::try_from(cmd).unwrap();

        assert_eq!(got.main_cmd, "find_files".to_owned());

        let mut want_map = HashMap::new();
        want_map.insert("hidden", "true");
        want_map.insert("layout_config", r#"{"prompt_position":"top"}"#);
        assert_eq!(got.map_args, want_map)
    }
}
