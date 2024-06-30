use crate::Result;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use serde::{de, Deserialize, Serialize};
use strum::VariantNames;

#[derive(Debug, Serialize)]
pub struct PersistenceConfig {
    #[serde(default = "default_enable")]
    pub enable: bool,
    #[serde(default)]
    pub path: Option<PathBuf>,
}

fn default_enable() -> bool {
    true
}

fn default_path() -> Result<PathBuf> {
    let dir_path = if cfg!(target_os = "windows") {
        env::var("LOCALAPPDATA")
            .map(|ev| {
                let path: PathBuf = ev.into();
                path.join("nvim-data")
            })
            .map_err(|e| anyhow!("{e}"))
    } else if let Ok(ev) = env::var("XDG_DATA_HOME") {
        let path: PathBuf = ev.into();
        Ok(path.join("nvim"))
    } else {
        env::var("HOME")
            .map(|ev| {
                let path: PathBuf = ev.into();
                path.join(".local/share/nvim")
            })
            .map_err(|e| anyhow!("{e}"))
    }?
    .join("compass");

    get_storage_file_path(dir_path)
}

fn get_storage_file_path(path: PathBuf) -> Result<PathBuf> {
    if !path.try_exists().map_err(|e| anyhow!("{e}"))? {
        fs::create_dir_all(&path).map_err(|e| anyhow!("{e}"))?;
    };

    let esc_path = escaped_path(&env::current_dir().map_err(|e| anyhow!("{e}"))?)?;

    Ok(path.join(esc_path))
}

fn escaped_path(path: &Path) -> Result<String> {
    let mut cwd = path
        .as_os_str()
        .to_str()
        .ok_or_else(|| anyhow!("failed to convert os path to utf-8"))?
        .to_owned();

    Ok(match cfg!(target_os = "windows") {
        true => {
            cwd.insert(0, '"');
            cwd.push('"');
            cwd
        }
        false => cwd.replace("/", "_"),
    })
}

#[derive(Deserialize, strum_macros::VariantNames)]
#[strum(serialize_all = "lowercase")]
#[serde(field_identifier, rename_all = "lowercase")]
enum PersistenceField {
    Enable,
    Path,
}

impl<'de> de::Deserialize<'de> for PersistenceConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct PersistenceVisitor;

        impl<'de> de::Visitor<'de> for PersistenceVisitor {
            type Value = PersistenceConfig;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a persistent compass.nvim storage config")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut enable = None;
                let mut path = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        PersistenceField::Enable => {
                            if enable.is_some() {
                                return Err(de::Error::duplicate_field("enable"));
                            }
                            enable = Some(map.next_value()?);
                        }
                        PersistenceField::Path => {
                            if path.is_some() {
                                return Err(de::Error::duplicate_field("path"));
                            }
                            let base_path: PathBuf = map.next_value()?;
                            let maybe_full_path = get_storage_file_path(base_path);
                            match maybe_full_path {
                                Ok(p) => {
                                    path = Some(p);
                                }
                                Err(e) => return Err(de::Error::custom(e)),
                            };
                        }
                    }
                }

                let enable = enable.unwrap_or_else(default_enable);
                if enable {
                    path = match path {
                        Some(p) => Some(p),
                        None => match default_path() {
                            Ok(p) => Some(p),
                            Err(e) => return Err(de::Error::custom(e)),
                        },
                    }
                }

                Ok(PersistenceConfig { enable, path })
            }
        }

        deserializer.deserialize_struct(
            "PersistenceConfig",
            PersistenceField::VARIANTS,
            PersistenceVisitor,
        )
    }
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enable: true,
            path: match default_path() {
                Ok(p) => Some(p),
                Err(_) => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn correct_xdg_default_path_on_unix() {
        let mut path = PathBuf::from(env::var("HOME").unwrap());
        path.push(".local/share");
        // SAFETY: should be fine in the test env
        unsafe { env::set_var("XDG_DATA_HOME", path) };

        let got = default_path().unwrap();

        assert!(got.to_str().unwrap().contains("/.local/share/nvim/compass"));
    }

    #[cfg(unix)]
    #[test]
    fn correct_home_default_path_on_unix() {
        // SAFETY: should be fine in the test env
        unsafe { env::remove_var("XDG_DATA_HOME") };

        let got = default_path().unwrap();

        assert!(got.to_str().unwrap().contains("/.local/share/nvim/compass"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn correct_default_path_on_windows() {
        let got = default_path().unwrap();

        assert!(got
            .to_str()
            .unwrap()
            .contains("\\AppData\\Local\\nvim-data\\compass"));
    }
}
