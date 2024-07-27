use crate::{
    common_types::{CursorRange, Extmark},
    config::{JumpKeymap, WindowGridSize},
    state::{ChangeTypeRecord, Record, TypeRecord},
    ui::{record_mark::create_hint_mark, tab::open_tab},
    InputError, Result,
};

use anyhow::Context;
use nvim_oxi::{
    api::{
        command, create_augroup, create_autocmd, create_buf, del_augroup_by_id, get_current_win,
        get_option_value, open_win,
        opts::{CreateAugroupOpts, CreateAutocmdOpts, OptionOpts, SetKeymapOpts},
        set_current_win, set_option_value,
        types::{Mode, WindowConfig, WindowRelativeTo},
        Buffer, Window,
    },
    Function,
};

#[derive(Clone, Copy)]
pub enum GridLayout {
    Open,
    Follow,
}

pub fn open_grid<'a>(
    slice_record: &[&Record],
    limit_win: WindowGridSize,
    layout: GridLayout,
    mut jump_iter: impl Iterator<Item = &'a JumpKeymap>,
) -> Result<()> {
    let len_record = slice_record.len();
    if len_record == 0 {
        Err(InputError::NoRecords("record list is empty".to_owned()))?
    };

    open_tab()?;

    let mut result: Vec<(Window, Buffer, (Extmark, Option<Extmark>))> =
        Vec::with_capacity(limit_win.into());

    let (conf_horiz, conf_vert) = new_split_config();
    let (first_len_half, second_len_half) = calc_halves(len_record, limit_win);

    let mut record_iter = slice_record.iter();

    let (mut hidden_win, hidden_buf, old_guicursor) = open_hidden_float()?;
    let mut create_hint =
        |record: &Record, ran: &CursorRange| -> Result<(Extmark, Option<Extmark>)> {
            let jump_keymap = jump_iter
                .next()
                .with_context(|| "no jump keymap to create a hint with")?;

            set_buffer_jump_keymap(hidden_buf.clone(), jump_keymap, record, &layout, limit_win)?;
            create_hint_mark(record.buf.clone(), ran, jump_keymap.follow.as_str(), layout)
        };

    for i in 0..first_len_half {
        let record = record_iter
            .next()
            .with_context(|| "no next buffer to iterate over")?;

        let pos = record.lazy_extmark.get_pos(record.buf.clone());
        let mut win = {
            if i == 0 {
                let mut w = get_current_win();
                w.set_buf(&record.buf)?;
                w
            } else {
                open_win(&record.buf, false, &conf_vert)?
            }
        };
        win.set_cursor(pos.line, pos.col)?;
        command("normal! zz")?;

        let hint = create_hint(record, &Into::<CursorRange>::into(&pos))?;

        result.push((win, record.buf.clone(), hint));
    }
    for i in 0..second_len_half {
        let record = record_iter
            .next()
            .with_context(|| "no next buffer to iterate over")?;
        set_current_win({
            let (win, _, _) = &result.get(i).with_context(|| "no win in the results vec")?;
            win
        })?;

        let pos = record.lazy_extmark.get_pos(record.buf.clone());
        let mut win = open_win(&record.buf, false, &conf_horiz)?;
        win.set_cursor(pos.line, pos.col)?;
        command("normal! zz")?;

        let hint = create_hint(record, &Into::<CursorRange>::into(&pos))?;

        result.push((win, record.buf.clone(), hint));
    }

    set_current_win(&hidden_win)?;
    hidden_win.set_buf(&hidden_buf)?;

    cleanup(hidden_buf.clone(), old_guicursor, result)?;

    Ok(())
}

fn cleanup(
    hidden_buf: Buffer,
    old_guicursor: String,
    result: Vec<(Window, Buffer, (Extmark, Option<Extmark>))>,
) -> Result<()> {
    let group = create_augroup(
        "CompassGridCleanupGroup",
        &CreateAugroupOpts::builder().clear(true).build(),
    )?;

    // TODO: will not close the tab if the hidden window is somehow closed by the user for example with :q
    // all what I have tried so far makes nvim instance freeze
    create_autocmd(
        ["BufLeave"],
        &CreateAutocmdOpts::builder()
            .once(true)
            .buffer(hidden_buf.clone())
            .group(group)
            .callback(Function::from_fn_once(move |_| -> Result<bool> {
                set_option_value("guicursor", old_guicursor, &OptionOpts::builder().build())?;
                command("hi Cursor blend=0")?;

                for (win, buf, (hint, path)) in result {
                    hint.delete(buf.clone())?;
                    if let Some(path) = path {
                        path.delete(buf.clone())?;
                    }
                    win.close(true)?;
                }

                del_augroup_by_id(group)?;

                Ok(true)
            }))
            .build(),
    )?;

    Ok(())
}

fn calc_halves(record_len: usize, limit_win: WindowGridSize) -> (usize, usize) {
    let half_limit_win = Into::<usize>::into(limit_win) / 2;

    let first = record_len / 2 + {
        if record_len % 2 == 0 {
            0
        } else {
            1
        }
    };
    let second = record_len / 2;

    (
        std::cmp::min(half_limit_win, first),
        std::cmp::min(half_limit_win, second),
    )
}

fn open_hidden_float() -> Result<(Window, Buffer, String)> {
    let buf = create_buf(false, true)?;

    let win = open_win(
        &buf,
        false,
        &WindowConfig::builder()
            .relative(WindowRelativeTo::Editor)
            .hide(true)
            .width(1)
            .height(1)
            .row(0)
            .col(0)
            .noautocmd(true)
            .build(),
    )?;

    command("hi Cursor blend=100")?;
    let old_guicursor: String = get_option_value("guicursor", &OptionOpts::builder().build())?;
    set_option_value(
        "guicursor",
        "a:Cursor/lCursor",
        &OptionOpts::builder().build(),
    )?;

    Ok((win, buf, old_guicursor))
}

fn get_goto_string(record: &Record) -> String {
    match record.typ {
        TypeRecord::Change(typ) => match typ {
            ChangeTypeRecord::Tick(t) => {
                format!(r#"tick={{"buf":{},"tick":{}}}"#, record.buf.handle(), t)
            }

            ChangeTypeRecord::Manual(_) => {
                format!(
                    r#"time={{"buf":{},"millis":{}}}"#,
                    record.buf.handle(),
                    record.frecency.latest_timestamp()
                )
            }
        },
    }
}

fn set_buffer_jump_keymap(
    mut buf: Buffer,
    JumpKeymap { follow, immediate }: &JumpKeymap,
    record: &Record,
    layout: &GridLayout,
    limit_win: WindowGridSize,
) -> Result<()> {
    buf.set_keymap(
        Mode::Normal,
        immediate.as_str(),
        format!(
            r#":tabclose<CR>:Compass goto absolute {}<CR>"#,
            get_goto_string(record)
        )
        .as_str(),
        &SetKeymapOpts::builder()
            .noremap(true)
            .nowait(true)
            .silent(true)
            .build(),
    )?;

    buf.set_keymap(
        Mode::Normal,
        follow.as_str(),
        match layout {
            GridLayout::Open => format!(
                r#":tabclose<CR>:Compass follow buf target={} max_windows={}<CR>"#,
                record.buf.handle(),
                Into::<i32>::into(limit_win),
            ),
            GridLayout::Follow => format!(
                r#":tabclose<CR>:Compass goto absolute {}<CR>"#,
                get_goto_string(record)
            ),
        }
        .as_str(),
        &SetKeymapOpts::builder()
            .noremap(true)
            .nowait(true)
            .silent(true)
            .build(),
    )?;

    Ok(())
}

fn new_split_config() -> (WindowConfig, WindowConfig) {
    (
        WindowConfig::builder()
            .noautocmd(true)
            .focusable(false)
            .vertical(false)
            .build(),
        WindowConfig::builder()
            .noautocmd(true)
            .focusable(false)
            .vertical(true)
            .build(),
    )
}
