mod completion;
pub use completion::*;

mod opts;
use opts::*;

use crate::{state::SyncTracker, InputError, Result};

use nvim_oxi::api::get_current_win;

pub fn get_pop(tracker: SyncTracker) -> impl Fn(Option<PopOptions>) -> Result<()> {
    move |opts: Option<PopOptions>| {
        let opts = opts.unwrap_or_default();

        let mut tracker = tracker.lock()?;

        match opts {
            PopOptions::Relative(RelativeOptions { direction }) => {
                let mut record = match direction {
                    Direction::Back => tracker.list.pop_past().ok_or_else(|| {
                        InputError::NoRecords("no more previous records to pop".to_owned())
                    })?,
                    Direction::Forward => tracker.list.pop_future().ok_or_else(|| {
                        InputError::NoRecords("no more next records to pop".to_owned())
                    })?,
                };

                let win = get_current_win();
                record.pop(win)?;
            }
        }

        Ok(())
    }
}
