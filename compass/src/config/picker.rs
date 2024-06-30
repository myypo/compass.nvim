mod window_grid_size;
pub use window_grid_size::*;

mod filename;
pub use filename::*;

mod jump_keymaps;
pub use jump_keymaps::*;

use serde::de;

#[derive(Debug, Default)]
pub struct PickerConfig {
    pub max_windows: WindowGridSize,
    pub jump_keys: JumpKeymapList,
    pub filename: Filename,
}

impl<'de> de::Deserialize<'de> for PickerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        enum Field {
            MaxWindows,
            JumpKeys,
            Filename,
        }
        impl<'de> de::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`max_windows`, `jump_keys`, or `filename`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "max_windows" => Ok(Field::MaxWindows),
                            "jump_keys" => Ok(Field::JumpKeys),
                            "filename" => Ok(Field::Filename),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct PickerConfigVisitor;

        impl<'de> de::Visitor<'de> for PickerConfigVisitor {
            type Value = PickerConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct PickerConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PickerConfig, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut max_windows = None;
                let mut jump_keys = None;
                let mut filename = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::MaxWindows => {
                            if max_windows.is_some() {
                                return Err(de::Error::duplicate_field("max_windows"));
                            }
                            max_windows = Some(map.next_value()?);
                        }
                        Field::JumpKeys => {
                            if jump_keys.is_some() {
                                return Err(de::Error::duplicate_field("jump_keys"));
                            }
                            jump_keys = Some(map.next_value()?);
                        }
                        Field::Filename => {
                            if filename.is_some() {
                                return Err(de::Error::duplicate_field("filename"));
                            }
                            filename = Some(map.next_value()?);
                        }
                    }
                }

                let max_windows: WindowGridSize = max_windows.unwrap_or_default();
                let jump_keys: JumpKeymapList = jump_keys.unwrap_or_default();
                if Into::<usize>::into(max_windows) > jump_keys.len() {
                    return Err(de::Error::invalid_length(
                        jump_keys.len(),
                        &"length of jump_keys must be equal or bigger that the max_windows value",
                    ));
                };

                let filename = filename.unwrap_or_default();

                Ok(PickerConfig {
                    max_windows,
                    jump_keys,
                    filename,
                })
            }
        }

        const FIELDS: &[&str] = &["max_windows", "jump_keys", "filename"];
        deserializer.deserialize_struct("PickerConfig", FIELDS, PickerConfigVisitor)
    }
}
