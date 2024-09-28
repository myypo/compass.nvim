mod completion;
use std::sync::Mutex;

pub use completion::*;

mod opts;
use opts::*;

use crate::{common_types::Direction, state::Tracker, InputError, Result};

pub fn get_goto(tracker: &'static Mutex<Tracker>) -> impl Fn(Option<GotoOptions>) -> Result<()> {
    move |opts: Option<GotoOptions>| {
        let opts = opts.unwrap_or_default();

        let mut tracker = tracker.lock()?;

        match opts {
            GotoOptions::Relative(RelativeOptions { direction }) => match direction {
                Direction::Back => tracker.step_past(),
                Direction::Forward => tracker.step_future(),
            },

            GotoOptions::Absolute(AbsoluteOptions {
                target: AbsoluteTarget::Index(idx_record),
            }) => tracker.goto_absolute(idx_record),

            GotoOptions::Absolute(AbsoluteOptions {
                target: AbsoluteTarget::Time(t),
            }) => {
                let idx_record = tracker
                    .list
                    .iter_from_future()
                    .position(|r| r.buf == t.buf && r.frecency.latest_timestamp() == t.timestamp)
                    .ok_or_else(|| InputError::NoRecords("no such record identified".to_owned()))?;
                tracker.goto_absolute(idx_record)
            }

            GotoOptions::Absolute(AbsoluteOptions {
                target: AbsoluteTarget::Tick(t),
            }) => {
                let idx_record = tracker
                    .list
                    .iter_from_future()
                    .position(|r| {
                        r.buf == t.buf
                            && r.place_type
                                .tick()
                                .is_some_and(|rec_tick| rec_tick == t.tick)
                    })
                    .ok_or_else(|| InputError::NoRecords("no such record identified".to_owned()))?;
                tracker.goto_absolute(idx_record)
            }
        }
    }
}
