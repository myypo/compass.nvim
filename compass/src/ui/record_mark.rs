use super::{grid::GridLayout, highlights::RecordHighlightNames};
use crate::{
    common_types::{CursorRange, Extmark},
    config::{get_config, SignText},
    state::get_namespace,
    ui::highlights::{HintHighlightList, RecordHighlightList},
    Error, Result,
};
use std::path::{Component, Path, PathBuf};

use nvim_oxi::api::{
    opts::{SetExtmarkOpts, SetExtmarkOptsBuilder},
    types::{ExtmarkHlMode, ExtmarkVirtTextPosition},
    Buffer,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum RecordMarkTime {
    Past,
    Future,
    PastClose,
    FutureClose,
}

pub fn recreate_mark_time(i: usize, pos: Option<usize>) -> RecordMarkTime {
    match pos {
        Some(p) => {
            if i == p {
                RecordMarkTime::FutureClose
            } else if i.checked_sub(1).is_some_and(|j| j == p) {
                RecordMarkTime::PastClose
            } else if i < p {
                RecordMarkTime::Future
            } else {
                RecordMarkTime::Past
            }
        }
        None => {
            if i == 0 {
                RecordMarkTime::PastClose
            } else {
                RecordMarkTime::Past
            }
        }
    }
}

impl From<RecordMarkTime> for RecordHighlightNames {
    fn from(value: RecordMarkTime) -> Self {
        RecordHighlightList::record_hl_names(value)
    }
}

impl From<RecordMarkTime> for &SignText {
    fn from(value: RecordMarkTime) -> Self {
        let signs = &get_config().marks.signs;

        match value {
            RecordMarkTime::Past => &signs.past,
            RecordMarkTime::PastClose => &signs.close_past,
            RecordMarkTime::Future => &signs.future,
            RecordMarkTime::FutureClose => &signs.close_future,
        }
    }
}

/// Returns and accepts a 0,0 indexed position with column being end-inclusive
fn get_non_blank_pos(buf: Buffer, line: usize, col: usize) -> (usize, usize) {
    fn get_first_non_blank_col(
        buf: Buffer,
        line: usize,
        maybe_col: Option<usize>,
    ) -> Option<usize> {
        let str_line = buf
            .get_lines(line..=line, true)
            .ok()?
            .next()
            .map(|s| s.to_string())?;
        if str_line.is_empty() {
            return None;
        }

        if let Some(col) = maybe_col {
            let len_line = str_line.chars().count();
            if len_line <= col + 1 {
                return len_line.checked_sub(1);
            }
        }

        str_line.char_indices().position(|(i, c)| {
            !c.is_whitespace() && c != '\t' && {
                if let Some(col) = maybe_col {
                    i >= col
                } else {
                    true
                }
            }
        })
    }

    if let Some(nb_col) = get_first_non_blank_col(buf.clone(), line, Some(col)) {
        return (line, nb_col);
    };

    for i in 1..=5 {
        let add_line = line + i;
        if let Some(nb_col) = get_first_non_blank_col(buf.clone(), add_line, None) {
            return (add_line, nb_col);
        };

        if let Some(sub_line) = line.checked_sub(i) {
            if let Some(nb_col) = get_first_non_blank_col(buf.clone(), sub_line, None) {
                return (sub_line, nb_col);
            };
        }
    }

    (line, col)
}

pub fn create_record_mark(
    mut buf: Buffer,
    &CursorRange { line, col }: &CursorRange,
    time: RecordMarkTime,
) -> Result<Extmark> {
    let (line, col) = get_non_blank_pos(buf.clone(), line, col);

    let set_opts = &basic_mark_builder(&mut SetExtmarkOpts::builder(), line, col, time).build();

    buf.set_extmark(get_namespace().into(), line, col, set_opts)
        .map(|id| Extmark::new(id, (line, col).into()))
        .map_err(Into::<Error>::into)
}

pub fn update_record_mark(
    extmark: &Extmark,
    mut buf: Buffer,
    &CursorRange { line, col }: &CursorRange,
    time: RecordMarkTime,
) -> Result<()> {
    let (line, col) = get_non_blank_pos(buf.clone(), line, col);

    let set_opts = &basic_mark_builder(&mut SetExtmarkOpts::builder(), line, col, time)
        .id(Into::<u32>::into(extmark))
        .build();

    buf.set_extmark(get_namespace().into(), line, col, set_opts)
        .map(|_| ())
        .map_err(Into::into)
}

fn basic_mark_builder(
    builder: &mut SetExtmarkOptsBuilder,
    line: usize,
    col: usize,
    time: RecordMarkTime,
) -> &mut SetExtmarkOptsBuilder {
    let hl: RecordHighlightNames = time.into();
    builder
        .hl_mode(ExtmarkHlMode::Combine)
        .hl_group(hl.mark)
        .sign_hl_group(hl.sign)
        .sign_text(Into::<&SignText>::into(time))
        .end_row(line)
        .end_col(col + 1)
        .invalidate(false)
        .strict(false)
        .undo_restore(false);

    builder
}

// TODO: make hints scoped to a single window once the namespace API is stabilized
pub fn create_hint_mark(
    mut buf: Buffer,
    &CursorRange { line, col }: &CursorRange,
    name: &str,
    typ: GridLayout,
) -> Result<(Extmark, Option<Extmark>)> {
    let hl = HintHighlightList::hint_hl_names(typ);

    let label = {
        buf.set_extmark(
            get_namespace().into(),
            line,
            col,
            &SetExtmarkOpts::builder()
                .virt_text([(name, hl.mark)])
                .virt_text_pos(ExtmarkVirtTextPosition::Overlay)
                .virt_text_hide(false)
                .build(),
        )
        .map(|id| Extmark::try_new(id, buf.clone()))
        .map_err(Into::<Error>::into)?
        .map_err(Into::<Error>::into)?
    };

    let path = {
        let filename = &get_config().picker.filename;
        match filename.enable {
            true => Some(
                buf.set_extmark(
                    get_namespace().into(),
                    line,
                    0,
                    &SetExtmarkOpts::builder()
                        .virt_text([(
                            truncate_path(&buf.get_name()?, filename.depth).as_path(),
                            hl.path,
                        )])
                        .virt_text_pos(ExtmarkVirtTextPosition::Eol)
                        .virt_text_hide(false)
                        .build(),
                )
                .map(|id| Extmark::try_new(id, buf))
                .map_err(Into::<Error>::into)?
                .map_err(Into::<Error>::into)?,
            ),
            false => None,
        }
    };

    Ok((label, path))
}

fn truncate_path(path: &Path, depth: usize) -> PathBuf {
    let mut components = path
        .components()
        .rev()
        .take(depth)
        .collect::<Vec<Component>>();
    components.reverse();
    components.iter().collect()
}

mod tests {
    use super::*;

    #[nvim_oxi::test]
    fn gets_first_non_blank_col() {
        let mut buf = Buffer::current();
        buf.set_lines(
            ..,
            true,
            [
                "\t\topener line",
                "\t\t    processed line   ",
                "l",
                "  finish line",
            ],
        )
        .unwrap();

        let got = get_non_blank_pos(buf, 1, 0);

        assert_eq!(got, (1, 6));
    }

    #[nvim_oxi::test]
    fn get_first_non_blank_line_and_col_downside() {
        let mut buf = Buffer::current();
        buf.set_lines(
            ..,
            true,
            [
                "\t\topener line",
                "",
                "", // processed line
                "",
                "\t\texpected line",
                "\t\t",
            ],
        )
        .unwrap();

        let got = get_non_blank_pos(buf, 2, 0);

        assert_eq!(got, (4, 2));
    }

    #[nvim_oxi::test]
    fn get_first_non_blank_line_and_col_upside() {
        let mut buf = Buffer::current();
        buf.set_lines(
            ..,
            true,
            [
                "\t\topener line",
                "\t\texpected line",
                "\t\t",
                "", // processed line
                "",
                "\t\t",
                "",
            ],
        )
        .unwrap();

        let got = get_non_blank_pos(buf, 3, 0);

        assert_eq!(got, (1, 2));
    }

    #[nvim_oxi::test]
    fn get_first_non_blank_line_and_col_when_end_line() {
        let mut buf = Buffer::current();
        buf.set_lines(.., true, ["\t\topener line", "\t 234567", "  finish line"])
            .unwrap();

        let got = get_non_blank_pos(buf, 1, 7);

        assert_eq!(got, (1, 7));
    }

    #[nvim_oxi::test]
    fn recreates_mark_track_respecting_pos_middle() {
        let pos = Some(1);

        let r = 0..4;
        let mut hl_vec = Vec::new();
        for i in r {
            hl_vec.push(recreate_mark_time(i, pos));
        }

        let want = Vec::from([
            RecordMarkTime::Future,
            RecordMarkTime::FutureClose,
            RecordMarkTime::PastClose,
            RecordMarkTime::Past,
        ]);
        assert_eq!(hl_vec, want);
    }

    #[nvim_oxi::test]
    fn recreates_mark_track_respecting_pos_start() {
        let pos = None;

        let r = 0..4;
        let mut hl_vec = Vec::new();
        for i in r {
            hl_vec.push(recreate_mark_time(i, pos));
        }

        let want = Vec::from([
            RecordMarkTime::PastClose,
            RecordMarkTime::Past,
            RecordMarkTime::Past,
            RecordMarkTime::Past,
        ]);
        assert_eq!(hl_vec, want);
    }
}
