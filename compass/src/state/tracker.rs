use super::{
    frecency::FrecencyType, load_session, record::LazyExtmark, save_session, track_list::Mark,
    Session, Tick,
};
use crate::{
    common_types::CursorPosition,
    config::get_config,
    state::{ChangeTypeRecord, PlaceTypeRecord, Record, TrackList},
    ui::{
        namespace::get_namespace,
        record_mark::{recreate_mark_time, RecordMarkTime},
    },
    InputError, Result,
};
use std::{
    path::Path,
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
    renewed_bufs: Vec<Buffer>,
    latest_change: Option<(Buffer, Tick)>,
}

fn is_initial_tick(tick: Tick) -> bool {
    const INITIAL_CHANGEDTICK: i32 = 2;
    tick == INITIAL_CHANGEDTICK.into()
}

impl Tracker {
    pub fn persist_state(&mut self, path: &Path) -> Result<()> {
        if self.latest_flush.elapsed() >= Duration::from_secs(5) {
            save_session(Session::try_from(&self.list)?, path)?;
            self.latest_flush = Instant::now();
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

    pub fn track(&mut self) -> Result<()> {
        let buf_new = get_current_buf();
        if get_config().persistence.enable {
            self.renew_buf_record_marks(buf_new.clone())?;
        }

        if let Ok(GotMode { mode, .. }) = get_mode() {
            match mode {
                Mode::CmdLine | Mode::InsertCmdLine | Mode::Terminal => {
                    // Reset it to avoid placing marks on %s and other changing commands
                    self.latest_change = None;
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

        let conf = get_config();
        if conf.tracker.ignored_patterns.is_match(buf_new.get_name()?) {
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

            // Ignore the first change in the buffer we just moved
            // to make sure we do not place a mark for a non-interactive change
            if let Some((latest_buf, latest_tick)) = &self.latest_change {
                if *latest_buf != buf_new {
                    self.latest_change = Some((buf_new, tick_new));
                    return Ok(());
                }
                if *latest_tick == tick_new {
                    return Ok(());
                }
            } else {
                self.latest_change = Some((buf_new, tick_new));
                return Ok(());
            }

            if is_initial_tick(tick_new) {
                return Ok(());
            }
            if self.list.iter_from_future().any(|r| {
                r.buf == buf_new
                    && matches!(
                        r.place_type,
                        PlaceTypeRecord::Change(ChangeTypeRecord::Tick(t)) if t == tick_new
                    )
            }) {
                return Ok(());
            }

            PlaceTypeRecord::Change(ChangeTypeRecord::Tick(tick_new))
        };

        let win = get_current_win();
        let pos_new: CursorPosition = win.get_cursor()?.into();

        if let Some(i) = self.list.iter_from_future().position(
            |Record {
                 buf, lazy_extmark, ..
             }| {
                buf_new == *buf && { lazy_extmark.pos(buf_new.clone()).is_nearby(&pos_new) }
            },
        ) {
            self.activate_all()?;
            if let Some(nearby_record) = self.list.get_mut(i) {
                nearby_record.deact_update(
                    buf_new,
                    tick_new,
                    pos_new,
                    RecordMarkTime::PastClose,
                )?;
            }
            self.list.make_close_past_inactive(i);

            return Ok(());
        };

        self.activate_all()?;
        let record_new = Record::try_new_inactive(buf_new, tick_new, pos_new)?;
        self.list.push_inactive(record_new);

        Ok(())
    }

    pub fn activate_all(&mut self) -> Result<()> {
        let pos = self.list.pos;
        for (i, r) in self.list.iter_mut_from_future().enumerate() {
            r.sync_extmark(recreate_mark_time(i, pos))?;
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

    pub fn maintain(&mut self) -> Result<()> {
        let buf_curr = get_current_buf();
        self.merge(buf_curr.clone())?;
        self.delete_leaked_extmarks(buf_curr)?;
        Ok(())
    }

    pub fn activate(&mut self, activate_debounce: Duration) -> Result<()> {
        fn in_insert_mode() -> bool {
            let Ok(GotMode { mode, .. }) = get_mode() else {
                return true;
            };
            matches!(mode, Mode::Insert)
        }

        let list = &mut self.list;
        let pos = list.pos;
        if !in_insert_mode() && list.iter_from_future().any(|r| matches!(r.lazy_extmark, LazyExtmark::Inactive((_, _, inst)) if inst.elapsed() >= activate_debounce)) {
            for (i, r) in list.iter_mut_from_future().enumerate() {
                r.sync_extmark(recreate_mark_time(i, pos))?;
            }
        }

        Ok(())
    }

    pub fn load_state(&mut self, path: &Path) -> Result<()> {
        self.list = load_session(path).unwrap_or_default();
        Ok(())
    }

    pub fn step_past(&mut self) -> Result<()> {
        let Some(record) = self.list.step_past() else {
            return Ok(());
        };

        record.goto(get_current_win(), FrecencyType::RelativeGoto)
    }

    pub fn step_future(&mut self) -> Result<()> {
        self.activate_all()?;
        let Some(record) = self.list.step_future() else {
            return Ok(());
        };
        record.goto(get_current_win(), FrecencyType::RelativeGoto)
    }

    pub fn goto_absolute(&mut self, idx_record: usize) -> Result<()> {
        self.activate_all()?;
        let record = self.list.get_mut(idx_record).ok_or_else(|| {
            InputError::NoRecords(format!(
                "non-existent index for absolute goto provided: {}",
                idx_record
            ))
        })?;
        record.goto(get_current_win(), FrecencyType::AbsoluteGoto)
    }

    pub fn pop_past(&mut self) -> Result<()> {
        let Some(mut record) = self.list.pop_past() else {
            return Ok(());
        };
        record.pop(get_current_win())?;
        self.activate_all()
    }

    pub fn pop_future(&mut self) -> Result<()> {
        let Some(mut record) = self.list.pop_future() else {
            return Ok(());
        };
        record.pop(get_current_win())?;
        self.activate_all()
    }
}

impl Default for Tracker {
    fn default() -> Self {
        Self {
            list: TrackList::default(),
            latest_flush: Instant::now(),
            renewed_bufs: Vec::default(),
            latest_change: None,
        }
    }
}
