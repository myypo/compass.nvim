mod completion;
use std::sync::Mutex;

pub use completion::*;

mod opts;
use opts::*;

use crate::{
    state::{ChangeTypeRecord, PlaceTypeRecord, Record, TrackList, Tracker},
    ui::record_mark::RecordMarkTime,
    Result,
};

use nvim_oxi::api::{get_current_buf, get_current_win, Buffer, Window};

pub fn get_place(tracker: &'static Mutex<Tracker>) -> impl Fn(Option<PlaceOptions>) -> Result<()> {
    move |opts: Option<PlaceOptions>| {
        let opts = opts.unwrap_or_default();

        let mut tracker = tracker.lock()?;

        match opts {
            PlaceOptions::Change(ChangeOptions {}) => {
                let buf_curr = get_current_buf();
                let win_curr = get_current_win();

                let pos_curr = get_current_win().get_cursor()?.into();

                tracker.activate_first()?;
                let Some((i, old_record)) = tracker.list.iter_mut_from_future().enumerate().find(
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
                ) else {
                    return new_change_manual_record(buf_curr, win_curr, &mut tracker.list);
                };

                old_record.update(
                    buf_curr,
                    PlaceTypeRecord::Change(ChangeTypeRecord::Manual(old_record.place_type.tick())),
                    pos_curr,
                    RecordMarkTime::PastClose,
                )?;
                tracker.list.make_close_past(i);

                Ok(())
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
        PlaceTypeRecord::Change(ChangeTypeRecord::Manual(None)),
        &win.get_cursor()?.into(),
    )?;

    record_list.push(record_new);

    Ok(())
}
