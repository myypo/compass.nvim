use anyhow::anyhow;
use nvim_oxi::api::{notify, opts::NotifyOpts, types::LogLevel};

use crate::{config::get_config, Result};
use std::time::Instant;

use super::SyncTracker;

pub struct Worker {
    pub tracker: Option<SyncTracker>,
}

macro_rules! min {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => (::std::cmp::min($x, min!($($z),*)));
}

impl Worker {
    pub fn new(tracker: Option<SyncTracker>) -> Self {
        Self { tracker }
    }

    pub fn run_jobs(mut self) {
        std::thread::spawn(move || {
            let (debounce, persistence) = {
                let conf = &get_config();
                (&conf.tracker.debounce_milliseconds, &conf.persistence)
            };
            let min_deb = min!(debounce.run, debounce.maintenance, debounce.activate);

            let mut run_inst = Instant::now();
            let mut maint_inst = Instant::now();
            let mut persist_inst = Instant::now();

            let mut jobs = || -> Result<()> {
                let now = Instant::now();

                if now.duration_since(run_inst) >= debounce.run {
                    if let Some(tracker) = &mut self.tracker {
                        tracker.run()?;
                    };
                    run_inst = now;
                }

                if now.duration_since(maint_inst) >= debounce.maintenance {
                    if let Some(tracker) = &mut self.tracker {
                        tracker.maintain()?;
                    };
                    maint_inst = now;
                }

                if persistence.enable
                    && now.duration_since(persist_inst) >= persistence.interval_milliseconds
                {
                    if let Some(tracker) = &mut self.tracker {
                        let path = persistence.path.as_ref().ok_or_else(|| {
                            anyhow!(
                    "changes tracker persistence enabled yet no specified save state path found"
                )
                        })?;
                        tracker.persist_state(path)?;
                    };
                    persist_inst = now;
                }

                if let Some(tracker) = &mut self.tracker {
                    tracker.activate()?;
                };

                Ok(())
            };

            loop {
                if let Err(e) = jobs() {
                    let _ = notify(
                        &e.to_string(),
                        LogLevel::Error,
                        &NotifyOpts::builder().build(),
                    );
                };

                std::thread::sleep(min_deb);
            }
        });
    }
}
