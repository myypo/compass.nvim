use std::sync::Mutex;

use crate::{
    config::{get_config, set_config, Config},
    highlights::{apply_highlights, HighlightList},
    state::{Tracker, Worker},
    Result,
};

use anyhow::anyhow;

pub fn get_setup(tracker: &'static Mutex<Tracker>) -> impl FnOnce(Option<Config>) -> Result<()> {
    move |user_conf: Option<Config>| {
        set_config(user_conf.unwrap_or_default());
        let conf = get_config();

        {
            let mut tracker = tracker.lock()?;
            if conf.persistence.enable {
                if let Some(path) = &conf.persistence.path {
                    tracker.load_state(path)?;
                } else {
                    return Err(anyhow!(
                        "tracker persistence enabled yet no specified load state path found"
                    )
                    .into());
                };
            };
        };

        apply_highlights(HighlightList::default())?;

        let worker = Worker::new(tracker);
        worker.run_jobs();

        Ok(())
    }
}
