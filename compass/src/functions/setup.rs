use crate::{
    config::{get_config, set_config, Config},
    highlights::{apply_highlights, HighlightList},
    state::{SyncTracker, Worker},
    Result,
};

use anyhow::anyhow;

pub fn get_setup(mut tracker: SyncTracker) -> impl FnOnce(Option<Config>) -> Result<()> {
    |user_conf: Option<Config>| {
        set_config(user_conf.unwrap_or_default());

        let conf = get_config();

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

        apply_highlights(HighlightList::default())?;

        let worker = Worker::new(conf.tracker.enable.then_some(tracker));
        worker.run_jobs();

        Ok(())
    }
}
