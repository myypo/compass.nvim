mod completion;
use std::sync::Mutex;

pub use completion::*;

mod opts;
use opts::*;

use crate::{common_types::Direction, state::Tracker, Result};

pub fn get_pop(tracker: &'static Mutex<Tracker>) -> impl Fn(Option<PopOptions>) -> Result<()> {
    move |opts: Option<PopOptions>| {
        let opts = opts.unwrap_or_default();

        let mut tracker = tracker.lock()?;

        match opts {
            PopOptions::Relative(RelativeOptions { direction }) => match direction {
                Direction::Back => tracker.pop_past(),
                Direction::Forward => tracker.pop_future(),
            },
        }
    }
}
