use super::{load_session, record::LazyExtmark, save_session, track_list::Mark, Session, Tick};
use crate::{
    common_types::CursorPosition,
    config::get_config,
    state::{ChangeTypeRecord, PlaceTypeRecord, Record, TrackList},
    ui::{namespace::get_namespace, record_mark::RecordMarkTime},
    Result,
};
use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use nvim_oxi::api::{
    get_current_buf, get_current_win, get_mode, get_option_value, list_wins,
    opts::{GetExtmarksOpts, OptionOpts, OptionScope},
    types::{ExtmarkPosition, GotMode, Mode},
    Buffer,
};

pub struct Tracker {
    pub list: TrackList<Record>,
    last_flush: std::time::Instant,

    renewed_bufs: Vec<Buffer>,
}

fn is_initial_tick(tick: Tick) -> bool {
    const INITIAL_CHANGEDTICK: i32 = 2;
    tick == INITIAL_CHANGEDTICK.into()
}

impl Tracker {
    fn persist_state(&mut self, path: &Path) -> Result<()> {
        if self.last_flush.elapsed() >= Duration::from_secs(5) {
            save_session(Session::try_from(&self.list)?, path)?;
            self.last_flush = Instant::now();
        }

        Ok(())
    }

    /// Recreate the buffer's marks after opening it for the first time in restored session.
    /// HACK: if we were to recreate extmarks on setup, their positions would have been broken
    /// placing them all at the bottom of the file.
    pub fn renew_buf_record_marks(&mut self, curr_buf: Buffer) -> Result<()> {
        if self.renewed_bufs.iter().any(|b| b.clone() == curr_buf) {
            return Ok(());
        };

        for r in self
            .list
            .iter_mut_from_future()
            .filter(|r| r.buf == curr_buf)
        {
            r.load_extmark()?;
        }

        self.renewed_bufs.push(curr_buf);

        Ok(())
    }

    fn track(&mut self, buf_new: Buffer) -> Result<()> {
        let Ok(GotMode {
            mode: Mode::Normal, ..
        }) = get_mode()
        else {
            return Ok(());
        };

        let conf = get_config();
        if conf.tracker.ignored_patterns.is_match(buf_new.get_name()?) {
            return Ok(());
        }

        let modified: bool = get_option_value(
            "modified",
            &OptionOpts::builder().scope(OptionScope::Local).build(),
        )
        .unwrap_or(true);
        if !modified {
            return Ok(());
        }

        let type_buf: String = get_option_value(
            "buftype",
            &OptionOpts::builder().scope(OptionScope::Local).build(),
        )?;
        // Skip special buffers
        if !type_buf.is_empty() {
            return Ok(());
        }

        let tick_new = {
            let Ok(tick_new) = buf_new
                .get_var::<i32>("changedtick")
                .map(Into::<Tick>::into)
            else {
                return Ok(());
            };
            if is_initial_tick(tick_new) {
                return Ok(());
            };
            if !self.list.iter_from_future().all(
                |Record {
                     buf, place_type, ..
                 }| {
                    (match place_type {
                        PlaceTypeRecord::Change(ChangeTypeRecord::Tick(t)) => tick_new != *t,
                        _ => true,
                    }) || buf_new != *buf
                },
            ) {
                return Ok(());
            }

            PlaceTypeRecord::Change(ChangeTypeRecord::Tick(tick_new))
        };

        let win = get_current_win();
        let pos_new: CursorPosition = win.get_cursor()?.into();

        if let Some((i, nearby_record)) = self.list.iter_mut_from_future().enumerate().find(
            |(
                _,
                Record {
                    buf, lazy_extmark, ..
                },
            )| {
                buf_new == *buf && { lazy_extmark.pos(buf_new.clone()).is_nearby(&pos_new) }
            },
        ) {
            nearby_record.deact_update(buf_new, tick_new, pos_new, RecordMarkTime::PastClose)?;

            self.list.make_close_past(i);
            return Ok(());
        };

        let record_new = Record::try_new_inactive(buf_new, tick_new, pos_new)?;

        self.list.push(record_new);

        Ok(())
    }

    /// Merges closely placed marks into a single one by removing the older ones.
    /// HACK: Kinda necessary because of an existing race condition that might occur
    /// on, let's say, a continuous undo, where new adjacent marks will be created.
    /// In a perfect world this should be optional.
    fn merge(&mut self, buf: Buffer) -> Result<()> {
        let mut del_indices = Vec::new();
        for (i, r) in self
            .list
            .iter_from_future()
            .enumerate()
            .filter(|(_, r)| r.buf == buf)
        {
            let pos = r.lazy_extmark.pos(buf.clone());
            if self
                .list
                .iter_from_past()
                .take(self.list.len() - i - 1)
                .any(|r| r.buf == buf && r.lazy_extmark.pos(buf.clone()).is_nearby(&pos))
            {
                del_indices.push(i);
            }
        }

        for i in del_indices.into_iter().rev() {
            if let Some(e) = self.list.remove(i).map(|r| r.lazy_extmark) {
                e.delete(buf.clone())?;
            }
        }

        Ok(())
    }

    /// HACK: In some cases extmarks created by the plugin become untracked
    /// by our datastructures, so we have to delete them manually.
    fn delete_leaked_extmarks(&self, mut buf: Buffer) -> Result<()> {
        let ns_id: u32 = get_namespace().into();
        for (id, _, _, _) in buf.clone().get_extmarks(
            ns_id,
            ExtmarkPosition::ByTuple((0, 0)),
            // TODO: can't use the -1 sentinel since usize, oops
            ExtmarkPosition::ByTuple((9999999999, 9999999999)),
            &GetExtmarksOpts::builder().build(),
        )? {
            if !self
                .list
                .iter_from_future()
                .any(|r| r.buf == buf && Some(id) == r.lazy_extmark.id())
            {
                buf.del_extmark(ns_id, id)?;
            }
        }

        Ok(())
    }
}

impl Default for Tracker {
    fn default() -> Self {
        Self {
            list: TrackList::default(),
            last_flush: Instant::now(),
            renewed_bufs: Vec::default(),
        }
    }
}

#[derive(Clone)]
pub struct SyncTracker(Arc<Mutex<Tracker>>);

impl SyncTracker {
    pub fn run(&mut self) -> Result<()> {
        let buf_curr = get_current_buf();

        let mut tracker = self.lock()?;

        tracker.track(buf_curr.clone())?;
        if get_config().persistence.enable {
            tracker.renew_buf_record_marks(buf_curr)?;
        }

        Ok(())
    }

    pub fn maintain(&mut self) -> Result<()> {
        let buf_curr = get_current_buf();

        let mut tracker = self.lock()?;

        tracker.merge(buf_curr.clone())?;
        tracker.delete_leaked_extmarks(buf_curr)?;

        Ok(())
    }

    pub fn activate(&mut self) -> Result<()> {
        let conf = get_config();

        let curr_bufs: Vec<Buffer> = list_wins().filter_map(|w| w.get_buf().ok()).collect();

        let list = &mut self.lock()?.list;
        for r in list.iter_mut_from_future().filter(|r| {
            curr_bufs.iter().any(|b| b == &r.buf) &&
            matches!(r.lazy_extmark, LazyExtmark::Inactive((_, _, i)) if i.elapsed() > conf.tracker.debounce_milliseconds.activate)
        }) {
            r.load_extmark()?;
        }

        Ok(())
    }

    pub fn persist_state(&mut self, path: &Path) -> Result<()> {
        let tracker = &mut self.lock()?;
        tracker.persist_state(path)?;

        Ok(())
    }

    pub fn lock(&self) -> Result<MutexGuard<Tracker>> {
        Ok(self.0.lock().map_err(|e| anyhow!("{e}"))?)
    }

    pub fn load_state(&mut self, path: &Path) -> Result<()> {
        let mut tracker = self.lock()?;

        tracker.list = load_session(path).unwrap_or_default();

        Ok(())
    }
}

impl Default for SyncTracker {
    fn default() -> Self {
        Tracker::default().into()
    }
}

impl From<Tracker> for SyncTracker {
    fn from(value: Tracker) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}
