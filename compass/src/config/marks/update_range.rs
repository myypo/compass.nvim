use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdateRange {
    #[serde(default = "default_lines")]
    pub lines: Option<Range>,
    #[serde(default = "default_columns")]
    pub columns: Option<Range>,
}

#[derive(Debug, Deserialize)]
pub struct Range {
    #[serde(default)]
    pub single_max_distance: Option<usize>,
    #[serde(default)]
    pub combined_max_distance: Option<usize>,
}

fn default_lines() -> Option<Range> {
    Range {
        single_max_distance: Some(10),
        combined_max_distance: Some(25),
    }
    .into()
}

fn default_columns() -> Option<Range> {
    Range {
        single_max_distance: None,
        combined_max_distance: Some(25),
    }
    .into()
}

impl Default for UpdateRange {
    fn default() -> Self {
        Self {
            lines: default_lines(),
            columns: default_columns(),
        }
    }
}
