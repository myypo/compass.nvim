use super::{
    frecency::FrecencyType,
    load_session,
    record::LazyExtmark,
    save_session,
    track_list::{Active, IndicateCloseness, Mark},
    Session, Tick,
};
use crate::{
    common_types::CursorPosition,
    config::get_config,
    state::{Record, TrackList},
    ui::{namespace::get_namespace, record_mark::RecordMarkTime},
    InputError, Result,
};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use nvim_oxi::api::{
    get_current_buf, get_current_win, get_mode, get_option_value,
    opts::{GetExtmarksOpts, OptionOpts, OptionScope},
    types::{ExtmarkPosition, GotMode, Mode},
    Buffer,
};

#[derive(Debug)]
pub struct Tracker {
    pub list: TrackList<Record>,
    latest_flush: std::time::Instant,
    visited_bufs: HashMap<Buffer, Tick>,
    latest_buf: Option<Buffer>,
}

const INITIAL_CHANGEDTICK: Tick = Tick(2);

impl Tracker {
    pub fn persist_state(&mut self, path: &Path) -> Result<()> {
        if self.latest_flush.elapsed() >= Duration::from_secs(5) {
            save_session(Session::try_from(&self.list)?, path)?;
            self.latest_flush = Instant::now();
        }

        Ok(())
    }

    /// Record the buf as visited through the program's runtime
    /// returns a buffer if there are any changes to be recorded
    fn record_first_buf_visit(&mut self, curr_buf: Buffer, tick_new: Tick) -> Option<Buffer> {
        let latest_buf = self.latest_buf.clone();
        self.latest_buf = Some(curr_buf.clone());

        if let Some(&prev_tick) = self.visited_bufs.get(&curr_buf) {
            if prev_tick == tick_new {
                return None;
            }
            self.visited_bufs.insert(curr_buf.clone(), tick_new);
            return latest_buf;
        };

        // Recreate the buffer's marks after opening it for the first time in restored session
        // HACK: if we were to recreate extmarks on setup, their positions would have been broken
        // placing them all at the bottom of the file
        if get_config().persistence.enable {
            for r in self
                .list
                .iter_mut_from_future()
                .filter(|r| r.buf == curr_buf)
            {
                let _ = r.load_extmark();
            }
        }

        self.visited_bufs
            .insert(curr_buf.clone(), INITIAL_CHANGEDTICK);
        None
    }

    /// Unset the latest buf to skip the next change
    // TODO: it is a flaky method, because the changes might appear
    // after the special buffer is closed
    fn unset_latest_buf(&mut self) {
        self.latest_buf = None
    }

    pub fn track(&mut self) -> Result<()> {
        let buf_new = get_current_buf();

        // Skip special buffers
        if !get_option_value::<String>(
            "buftype",
            &OptionOpts::builder().scope(OptionScope::Local).build(),
        )?
        .is_empty()
        {
            // Unset it to avoid placing marks on changes that appear after interacting with special buffers
            self.unset_latest_buf();
            return Ok(());
        }

        if let Ok(GotMode { mode, .. }) = get_mode() {
            match mode {
                Mode::CmdLine | Mode::InsertCmdLine | Mode::Terminal => {
                    // Unset it to avoid placing marks on %s and other changing commands
                    self.unset_latest_buf();
                    return Ok(());
                }
                Mode::Insert => {
                    return Ok(());
                }
                _ => {}
            }
        } else {
            return Ok(());
        }

        let Ok(tick_new) = buf_new
            .get_var::<i32>("changedtick")
            .map(Into::<Tick>::into)
        else {
            return Ok(());
        };
        let Some(latest_buf) = self.record_first_buf_visit(buf_new.clone(), tick_new) else {
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

        if get_config()
            .tracker
            .ignored_patterns
            .is_match(buf_new.get_name()?)
        {
            return Ok(());
        }

        // Ignore the first change in the buffer we just moved to
        // to make sure we do not place a mark for a non-interactive change
        if latest_buf != buf_new.clone() {
            return Ok(());
        }

        let pos_new: CursorPosition = get_current_win().get_cursor()?.into();

        self.activate_first()?;
        if let Some(i) = self.list.iter_from_future().position(
            |Record {
                 buf, lazy_extmark, ..
             }| {
                buf_new == *buf && { lazy_extmark.pos(buf_new.clone()).is_nearby(&pos_new) }
            },
        ) {
            if let Some(nearby_record) = self.list.get_mut(i) {
                nearby_record.deact_update(
                    buf_new,
                    tick_new.into(),
                    pos_new,
                    RecordMarkTime::PastClose,
                )?;
            }
            self.list.make_inactive(i);

            return Ok(());
        };

        let record_new = Record::try_new_inactive(buf_new, tick_new.into(), pos_new)?;
        self.list.push_inactive(record_new);

        Ok(())
    }

    /// Assumes there is always at most a single inactive mark
    pub fn activate_first(&mut self) -> Result<()> {
        if let Some(i) = self.list.iter_from_future().position(|r| !r.is_active()) {
            if let Some(r) = self.list.get_mut(i) {
                r.load_extmark()?;
            }
            if let Some(r) = self.list.get_mut(i + 1) {
                r.as_past();
            }
        }

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
            if self
                .list
                .iter_from_future()
                .all(|r| !(r.buf == buf && Some(id) == r.lazy_extmark.id()))
            {
                buf.del_extmark(ns_id, id)?;
            }
        }

        Ok(())
    }

    fn remove_deleted_file_records(&mut self) -> Result<()> {
        let existing_bufs: Vec<Buffer> = {
            let bufs = self
                .list
                .iter_from_future()
                .map(|r| r.buf.clone())
                .collect::<HashSet<Buffer>>()
                .into_iter()
                .collect::<Vec<Buffer>>();

            bufs.into_iter()
                .filter(|b| {
                    b.get_name()
                        .ok()
                        .and_then(|f: PathBuf| f.try_exists().ok())
                        .unwrap_or(false)
                })
                .collect()
        };

        let del_indices: Vec<usize> = self
            .list
            .iter_from_future()
            .enumerate()
            .filter_map(|(i, r)| -> Option<usize> {
                if !existing_bufs.contains(&r.buf) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        for i in del_indices.into_iter().rev() {
            self.list.remove(i);
        }

        Ok(())
    }

    pub fn maintain(&mut self) -> Result<()> {
        self.remove_deleted_file_records()?;
        let buf_curr = get_current_buf();
        self.merge(buf_curr.clone())?;
        self.delete_leaked_extmarks(buf_curr)?;
        Ok(())
    }

    pub fn activate_ready(&mut self, activate_debounce: Duration) -> Result<()> {
        fn in_insert_mode() -> bool {
            let Ok(GotMode { mode, .. }) = get_mode() else {
                return true;
            };
            matches!(mode, Mode::Insert)
        }

        if !in_insert_mode() && self.list.iter_from_future().any(|r| matches!(r.lazy_extmark, LazyExtmark::Inactive((_, _, inst)) if inst.elapsed() >= activate_debounce)) {
            self.activate_first()?;
        }

        Ok(())
    }

    pub fn load_state(&mut self, path: &Path) -> Result<()> {
        self.list = load_session(path).unwrap_or_default();
        Ok(())
    }

    pub fn step_past(&mut self) -> Result<()> {
        let Some(record) = self.list.step_past(get_current_win()) else {
            return Ok(());
        };
        record.frecency.add_record(FrecencyType::RelativeGoto);
        if let Some(r) = self.list.iter_mut_from_future().find(|r| !r.is_active()) {
            r.load_extmark()?;
        }
        Ok(())
    }

    pub fn step_future(&mut self) -> Result<()> {
        if let Some(r) = self.list.iter_mut_from_future().find(|r| !r.is_active()) {
            r.load_extmark()?;
        }
        let Some(record) = self.list.step_future(get_current_win()) else {
            return Ok(());
        };
        record.frecency.add_record(FrecencyType::RelativeGoto);
        Ok(())
    }

    pub fn goto_absolute(&mut self, idx_record: usize) -> Result<()> {
        self.activate_first()?;
        let record = self.list.get_mut(idx_record).ok_or_else(|| {
            InputError::NoRecords(format!(
                "non-existent index for absolute goto provided: {}",
                idx_record
            ))
        })?;
        record.jump(get_current_win())?;
        record.frecency.add_record(FrecencyType::AbsoluteGoto);
        Ok(())
    }

    pub fn pop_past(&mut self) -> Result<()> {
        let Some(mut record) = self.list.pop_past(get_current_win()) else {
            return Ok(());
        };
        record.delete()?;
        if let Some(r) = self.list.iter_mut_from_future().find(|r| !r.is_active()) {
            r.load_extmark()?;
        }
        Ok(())
    }

    pub fn pop_future(&mut self) -> Result<()> {
        self.activate_first()?;
        let Some(mut record) = self.list.pop_future(get_current_win()) else {
            return Ok(());
        };
        record.delete()
    }
}

impl Default for Tracker {
    fn default() -> Self {
        Self {
            list: TrackList::default(),
            latest_flush: Instant::now(),
            visited_bufs: HashMap::default(),
            latest_buf: None,
        }
    }
}
