use crate::{
    state::Record,
    ui::record_mark::{recreate_mark_time, update_record_mark},
    Result,
};

use nvim_oxi::api::set_current_buf;

pub fn ensure_buf_loaded(
    idx: usize,
    pos_record_list: Option<usize>,
    record: &Record,
) -> Result<()> {
    set_current_buf(&record.buf)?;
    update_record_mark(
        &record.extmark,
        record.buf.clone(),
        &record.extmark.get_pos(record.buf.clone()),
        recreate_mark_time(idx, pos_record_list),
    )?;

    Ok(())
}
