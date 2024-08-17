use super::{
    get_namespace, load_session, record::LazyExtmark, save_session, track_list::Mark, Session, Tick,
};
use crate::{
    common_types::CursorPosition,
    config::get_config,
    state::{ChangeTypeRecord, Record, TrackList, TypeRecord},
    ui::record_mark::RecordMarkTime,
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
            if !self.list.iter_from_future().all(|Record { buf, typ, .. }| {
                (match typ {
                    TypeRecord::Change(ChangeTypeRecord::Tick(t)) => tick_new != *t,
                    _ => true,
                }) || buf_new != *buf
            }) {
                return Ok(());
            }

            TypeRecord::Change(ChangeTypeRecord::Tick(tick_new))
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
            nearby_record.hide_update(buf_new, tick_new, pos_new, RecordMarkTime::PastClose)?;

            self.list.make_close_past(i);
            return Ok(());
        };

        let record_new = Record::try_new_hidden(buf_new, tick_new, pos_new)?;

        self.list.push(record_new);

        Ok(())
    }

    /// Merges closely placed marks into a single one by removing the older ones.
    /// HACK: Kinda necessary because of an existing race condition that might occur
    /// on, let's say, a continious undo, where new adjacent marks will be created.
    /// In a perfect world this should be optional.
    fn merge(&mut self, buf: Buffer) -> Result<()> {
        let mut list_idx: Vec<usize> = Vec::new();
        for (io, ro) in self
            .list
            .iter_from_future()
            .enumerate()
            .filter(|(_, r)| r.buf == buf)
        {
            let p = ro.lazy_extmark.pos(buf.clone());

            for (ii, ri) in self
                .list
                .iter_from_future()
                .enumerate()
                .skip(io + 1)
                .filter(|(_, r)| r.buf == buf && r.lazy_extmark.pos(buf.clone()).is_nearby(&p))
            {
                let _ = ri.lazy_extmark.delete(buf.clone());

                if let Some(ii) = ii.checked_sub(1) {
                    list_idx.push(ii);
                }
            }
        }

        for i in list_idx {
            self.list.remove(i);
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
                let _ = buf.del_extmark(ns_id, id);
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

        let conf = get_config();

        if conf.persistence.enable {
            tracker.renew_buf_record_marks(buf_curr)?;

            let path = conf.persistence.path.as_ref().ok_or_else(|| {
                anyhow!(
                    "changes tracker persistence enabled yet no specified save state path found"
                )
            })?;
            tracker.persist_state(path)?;
        };

        Ok(())
    }

    pub fn maintain(&mut self) -> Result<()> {
        let buf_curr = get_current_buf();

        let mut tracker = self.lock()?;

        tracker.merge(buf_curr.clone())?;
        tracker.delete_leaked_extmarks(buf_curr)?;

        Ok(())
    }

    pub fn show(&mut self) -> Result<()> {
        let conf = get_config();

        let curr_bufs: Vec<Buffer> = list_wins().filter_map(|w| w.get_buf().ok()).collect();

        let list = &mut self.lock()?.list;
        for r in list.iter_mut_from_future().filter(|r| {
            curr_bufs.iter().any(|b| b == &r.buf) &&
            !matches!(r.lazy_extmark, LazyExtmark::Hidden((_, _, i)) if i.elapsed() < conf.tracker.debounce_milliseconds.show)
        }) {
            r.load_extmark()?;
        }

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
