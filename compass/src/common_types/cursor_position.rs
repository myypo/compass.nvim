use crate::config::get_config;

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// 1,0 indexed
#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize, Serialize, Decode, Encode)]
pub struct CursorPosition {
    pub line: usize,
    pub col: usize,
}

/// 0,0 indexed
#[derive(Debug, PartialEq, Eq)]
pub struct CursorRange {
    pub line: usize,
    pub col: usize,
}

impl CursorPosition {
    pub fn is_nearby(&self, &Self { line, col }: &Self) -> bool {
        let conf = &get_config().marks.update_range;

        let near_line = {
            if let Some(opt_lines) = &conf.lines {
                if opt_lines
                    .single_max_distance
                    .is_some_and(|single| single >= self.line.abs_diff(line))
                {
                    return true;
                } else {
                    opt_lines
                        .combined_max_distance
                        .is_some_and(|combined| combined >= self.line.abs_diff(line))
                }
            } else {
                true
            }
        };

        let near_col = {
            if let Some(opt_cols) = &conf.columns {
                if opt_cols
                    .single_max_distance
                    .is_some_and(|single| single >= self.col.abs_diff(col))
                {
                    return true;
                } else {
                    opt_cols
                        .combined_max_distance
                        .is_some_and(|combined| combined >= self.col.abs_diff(col))
                }
            } else {
                true
            }
        };

        near_line && near_col
    }
}

impl From<(usize, usize)> for CursorPosition {
    fn from((line, col): (usize, usize)) -> Self {
        Self { line, col }
    }
}

impl From<&mut CursorPosition> for CursorRange {
    fn from(value: &mut CursorPosition) -> Self {
        Self {
            line: value.line.saturating_sub(1),
            col: value.col,
        }
    }
}

impl From<&CursorPosition> for CursorRange {
    fn from(value: &CursorPosition) -> Self {
        Self {
            line: value.line.saturating_sub(1),
            col: value.col,
        }
    }
}

impl From<(usize, usize)> for CursorRange {
    fn from((line, col): (usize, usize)) -> Self {
        Self { line, col }
    }
}

impl From<&CursorRange> for CursorPosition {
    fn from(value: &CursorRange) -> Self {
        Self {
            line: value.line + 1,
            col: value.col,
        }
    }
}

mod tests {
    use super::*;

    #[nvim_oxi::test]
    fn can_identify_close_same_line_position() {
        let pos1: CursorPosition = (420, 0).into();
        let pos2: CursorPosition = (420, 6969).into();

        assert!(pos1.is_nearby(&pos2))
    }

    #[nvim_oxi::test]
    fn can_identify_close_multi_line_position() {
        let pos1: CursorPosition = (29, 42).into();
        let pos2: CursorPosition = (28, 45).into();

        assert!(pos1.is_nearby(&pos2))
    }
}
