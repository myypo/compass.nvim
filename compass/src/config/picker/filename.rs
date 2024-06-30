use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Filename {
    #[serde(default = "default_enable")]
    pub enable: bool,
    #[serde(default = "default_depth")]
    pub depth: usize,
}

fn default_enable() -> bool {
    true
}

fn default_depth() -> usize {
    2
}

impl Default for Filename {
    fn default() -> Self {
        Self {
            enable: default_enable(),
            depth: default_depth(),
        }
    }
}
