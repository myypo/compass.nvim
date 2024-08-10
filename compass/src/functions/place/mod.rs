mod completion;
pub use completion::*;

mod opts;
use opts::*;

use crate::{
    state::{ChangeTypeRecord, Record, SyncTracker, TrackList, TypeRecord},
    ui::record_mark::RecordMarkTime,
    Result,
};

use nvim_oxi::api::{get_current_buf, get_current_win, Buffer, Window};

pub fn get_place(sync_tracker: SyncTracker) -> impl Fn(Option<PlaceOptions>) -> Result<()> {
    move |opts: Option<PlaceOptions>| {
        let opts = opts.unwrap_or_default();

        let mut tracker = sync_tracker.lock()?;

        match opts {
            PlaceOptions::Change(ChangeOptions { try_update }) => {
                let buf_curr = get_current_buf();
                let win_curr = get_current_win();

                match try_update {
                    true => {
                        let pos_curr = get_current_win().get_cursor()?.into();

                        let Some((i, old_record)) =
                            tracker.list.iter_mut_from_future().enumerate().find(
                                |(
                                    _,
                                    Record {
                                        buf, lazy_extmark, ..
                                    },
                                )| {
                                    buf_curr == *buf && {
                                        lazy_extmark.pos(buf_curr.clone()).is_nearby(&pos_curr)
                                    }
                                },
                            )
                        else {
                            return new_change_manual_record(buf_curr, win_curr, &mut tracker.list);
                        };

                        old_record.update(
                            buf_curr,
                            TypeRecord::Change(ChangeTypeRecord::Manual(old_record.typ.tick())),
                            pos_curr,
                            RecordMarkTime::PastClose,
                        )?;
                        tracker.list.make_close_past(i);

                        Ok(())
                    }

                    false => new_change_manual_record(buf_curr, win_curr, &mut tracker.list),
                }
            }
        }
    }
}

fn new_change_manual_record(
    buf: Buffer,
    win: Window,
    record_list: &mut TrackList<Record>,
) -> Result<()> {
    let record_new = Record::try_new(
        buf,
        TypeRecord::Change(ChangeTypeRecord::Manual(None)),
        &win.get_cursor()?.into(),
    )?;

    record_list.push(record_new);

    Ok(())
}
