mod completion;
pub use completion::*;

mod opts;
use opts::*;

use crate::{state::SyncTracker, Result};

use nvim_oxi::api::{get_current_win, set_current_buf};

pub fn get_pop(tracker: SyncTracker) -> impl Fn(Option<PopOptions>) -> Result<()> {
    move |opts: Option<PopOptions>| {
        let opts = opts.unwrap_or_default();

        let mut tracker = tracker.lock()?;

        match opts {
            PopOptions::Relative(RelativeOptions { direction }) => {
                if let Some(mut record) = match direction {
                    Direction::Back => tracker.list.pop_past(),
                    Direction::Forward => tracker.list.pop_future(),
                } {
                    let win = get_current_win();
                    set_current_buf(&record.buf)?;
                    record.pop(win)?;
                };
            }
        }

        Ok(())
    }
}
