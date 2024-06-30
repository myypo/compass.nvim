use crate::Result;
use std::time::Duration;

use super::SyncTracker;

pub struct Worker {
    pub tracker: Option<SyncTracker>,
}

impl Worker {
    pub fn new(tracker: Option<SyncTracker>) -> Self {
        Self { tracker }
    }

    pub fn run_jobs(mut self) -> Result<()> {
        std::thread::spawn(move || -> Result<()> {
            loop {
                std::thread::sleep(Duration::from_millis(200));

                if let Some(tracker) = &mut self.tracker {
                    tracker.run()?;
                };
            }
        });

        Ok(())
    }
}
