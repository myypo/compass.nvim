use crate::Result;
use std::time::{Duration, Instant};

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
            let mut run_inst = Instant::now();
            let mut maint_inst = Instant::now();

            let run_interv = Duration::from_millis(0);
            let maint_interv = Duration::from_millis(500);

            loop {
                let now = Instant::now();

                if now.duration_since(run_inst) >= run_interv {
                    if let Some(tracker) = &mut self.tracker {
                        tracker.run()?;
                    };
                    run_inst = now;
                }

                if now.duration_since(maint_inst) >= maint_interv {
                    if let Some(tracker) = &mut self.tracker {
                        tracker.maintain()?;
                    };
                    maint_inst = now;
                }

                std::thread::sleep(Duration::from_millis(200));
            }
        });

        Ok(())
    }
}
