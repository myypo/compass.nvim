mod completion;
pub use completion::*;

mod opts;
use opts::*;

use crate::{
    common_types::Direction,
    state::{frecency::FrecencyType, SyncTracker},
    InputError, Result,
};

use nvim_oxi::api::{get_current_win, set_current_buf};

pub fn get_goto(tracker: SyncTracker) -> impl Fn(Option<GotoOptions>) -> Result<()> {
    move |opts: Option<GotoOptions>| {
        let opts = opts.unwrap_or_default();

        let mut tracker = tracker.lock()?;

        match opts {
            GotoOptions::Relative(RelativeOptions { direction }) => {
                let Some(record) = (match direction {
                    Direction::Back => tracker.list.step_past(),
                    Direction::Forward => tracker.list.step_future(),
                }) else {
                    return Ok(());
                };

                let win = get_current_win();
                record.goto(win, FrecencyType::RelativeGoto)
            }

            GotoOptions::Absolute(AbsoluteOptions {
                target: AbsoluteTarget::Index(idx_record),
            }) => {
                let win = get_current_win();

                let record = tracker.list.get_mut(idx_record).ok_or_else(|| {
                    InputError::NoRecords(format!(
                        "non-existent index for absolute goto provided: {}",
                        idx_record
                    ))
                })?;

                set_current_buf(&record.buf)?;
                record.goto(win, FrecencyType::AbsoluteGoto)
            }

            GotoOptions::Absolute(AbsoluteOptions {
                target: AbsoluteTarget::Time(t),
            }) => {
                let win = get_current_win();

                let record = tracker
                    .list
                    .iter_mut_from_future()
                    .find(|r| r.buf == t.buf && r.frecency.latest_timestamp() == t.timestamp)
                    .ok_or_else(|| InputError::NoRecords("no such record identified".to_owned()))?;

                set_current_buf(&record.buf)?;
                record.goto(win, FrecencyType::AbsoluteGoto)
            }

            GotoOptions::Absolute(AbsoluteOptions {
                target: AbsoluteTarget::Tick(t),
            }) => {
                let win = get_current_win();

                let record = tracker
                    .list
                    .iter_mut_from_future()
                    .find(|r| {
                        r.buf == t.buf
                            && r.place_type
                                .tick()
                                .is_some_and(|rec_tick| rec_tick == t.tick)
                    })
                    .ok_or_else(|| InputError::NoRecords("no such record identified".to_owned()))?;

                set_current_buf(&record.buf)?;
                record.goto(win, FrecencyType::AbsoluteGoto)
            }
        }
    }
}
