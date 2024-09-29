use std::{
    collections::vec_deque::{Iter, IterMut, VecDeque},
    iter::Rev,
};

use crate::{state::frecency::FrecencyScore, ui::record_mark::RecordMarkTime, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackList<T> {
    ring: VecDeque<T>,
    pub pos: Option<usize>,
}

impl<T> Default for TrackList<T> {
    fn default() -> Self {
        Self {
            ring: VecDeque::default(),
            pos: None,
        }
    }
}

pub trait Mark {
    fn load_extmark(&mut self) -> Result<()>;
    fn sync_extmark(&mut self, time: RecordMarkTime) -> Result<()>;
    fn open_buf(&self) -> Result<()>;
}

pub trait Active {
    fn is_active(&self) -> bool;
}

impl<T> TrackList<T> {
    pub fn with_capacity(capacity: usize, pos: Option<usize>) -> Self {
        Self {
            ring: VecDeque::with_capacity(capacity),
            pos,
        }
    }

    pub fn len(&self) -> usize {
        self.ring.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    pub fn iter_from_future(&self) -> Iter<T> {
        self.ring.iter()
    }

    pub fn iter_mut_from_future(&mut self) -> IterMut<T> {
        self.ring.iter_mut()
    }

    pub fn iter_from_past(&self) -> Rev<Iter<T>> {
        self.ring.iter().rev()
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        self.ring.get(i)
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        self.ring.get_mut(i)
    }

    /// Push an element without triggering domain side effects
    pub fn push_plain(&mut self, val: T) {
        self.ring.push_back(val);
    }
}

impl<T> TrackList<T>
where
    T: IndicateCloseness + Mark + Active,
{
    fn active_close_past_idx(&self) -> Option<usize> {
        self.ring
            .iter()
            .enumerate()
            .skip(self.pos.map(|p| p + 1).unwrap_or(0))
            .find_map(|(i, v)| match v.is_active() {
                true => Some(i),
                false => None,
            })
    }

    pub fn push(&mut self, val: T) {
        match self.pos {
            Some(p) => {
                if let Some(old_close) = self.ring.get_mut(p + 1) {
                    old_close.as_past();
                };

                self.ring.insert(p + 1, val);
            }
            None => {
                if let Some(first) = self.ring.front_mut() {
                    first.as_past();
                };

                self.ring.push_front(val);
            }
        }
    }

    pub fn push_inactive(&mut self, val: T) {
        match self.pos {
            Some(p) => self.ring.insert(p + 1, val),
            None => self.ring.push_front(val),
        }
    }

    pub fn step_past(&mut self) -> Option<&mut T> {
        let pos = self.active_close_past_idx()?;
        self.pos = Some(pos);

        {
            let curr = self.ring.get_mut(pos)?;
            curr.open_buf().ok()?;
            let _ = curr.load_extmark();
            curr.as_close_future();
        };

        if let Some(close_past) = self.ring.get_mut(pos + 1) {
            let _ = close_past.load_extmark();
            close_past.as_close_past();
        };
        if let Some(i) = pos.checked_sub(1) {
            if let Some(fut) = self.ring.get_mut(i) {
                let _ = fut.load_extmark();
                fut.as_future();
            };
        };

        self.ring.get_mut(pos)
    }

    pub fn step_future(&mut self) -> Option<&mut T> {
        let pos = self.pos?;

        {
            let curr = self.ring.get_mut(pos)?;
            curr.open_buf().ok()?;
            let _ = curr.load_extmark();
            curr.as_close_past();
        }

        if let Some(past) = self.ring.get_mut(pos + 1) {
            let _ = past.load_extmark();
            past.as_past();
        };

        self.pos = pos.checked_sub(1);
        if let Some(i) = self.pos {
            if let Some(close_fut) = self.ring.get_mut(i) {
                let _ = close_fut.load_extmark();
                close_fut.as_close_future();
            };
        };

        self.ring.get_mut(pos)
    }

    pub fn pop_past(&mut self) -> Option<T> {
        let idx = self.active_close_past_idx()?;

        // Handling skip of inactive marks
        let skipped = self.pos.map(|p| p + 1).unwrap_or(0).abs_diff(idx);
        if skipped != 0 {
            if let Some(p) = self.pos {
                self.pos = Some(p + skipped);
            } else {
                self.pos = skipped.checked_sub(1);
            }
            if let Some(p) = self.pos.and_then(|p| p.checked_sub(1)) {
                if let Some(f) = self.get_mut(p) {
                    f.as_future();
                }
            }
            if let Some(p) = self.pos {
                if let Some(cf) = self.get_mut(p) {
                    cf.as_close_future();
                }
            }
        }

        let popped = self.ring.remove(idx);

        if let Some(cp) = self.get_mut(self.pos.map(|p| p + 1).unwrap_or(0)) {
            cp.as_close_past();
        }

        popped
    }

    pub fn pop_future(&mut self) -> Option<T> {
        let pos = self.pos?;
        let new_pos = pos.checked_sub(1);

        if let Some(i) = new_pos {
            if let Some(new_close) = self.ring.get_mut(i) {
                new_close.as_close_future();
            }
        }

        self.pos = new_pos;

        self.ring.remove(pos)
    }

    pub fn remove(&mut self, i: usize) -> Option<T> {
        match self.pos {
            Some(p) => match i {
                _ if i + 1 == p => {
                    if let Some(next_past) = self.ring.get_mut(i + 1) {
                        next_past.as_close_past();
                    };
                }

                _ if i == p => {
                    let nfi = p.checked_sub(1);
                    self.pos = nfi;

                    if let Some(nfi) = p.checked_sub(1) {
                        if let Some(next_fut) = self.ring.get_mut(nfi) {
                            next_fut.as_close_future();
                        };
                    }
                }

                _ if i < p => {
                    let nfi = p.checked_sub(1);
                    self.pos = nfi;
                }

                _ => {}
            },
            None if i == 0 => {
                if let Some(next_past) = self.ring.get_mut(i + 1) {
                    next_past.as_close_past();
                };
            }
            _ => {}
        }

        self.ring.remove(i)
    }

    pub fn make_close_past(&mut self, idx: usize) -> Option<()> {
        if let Some(p) = self.pos {
            if p + 1 == idx {
                return Some(());
            }
        }

        let val = self.ring.get_mut(idx)?;
        val.as_close_past();

        let len = self.len();
        if len == 1 {
            self.pos = None;
            return Some(());
        }

        match self.pos {
            Some(p) => {
                match idx.ge(&p) {
                    // past -> close past
                    true => {
                        if let Some(old_close) = match self.pos {
                            Some(p) => self.get_mut(p + 1),
                            None => self.get_mut(0),
                        } {
                            old_close.as_past();
                        }

                        self.ring.swap(idx, p);

                        if p + 1 < idx {
                            self.ring.make_contiguous()[p..=idx].rotate_right(1);
                        } else {
                            self.pos = p.checked_sub(1);
                        }

                        Some(())
                    }

                    // future -> close past
                    false => {
                        match p.checked_sub(1) {
                            Some(new_pos) => {
                                if p == idx {
                                    if let Some(close_new) = self.ring.get_mut(new_pos) {
                                        close_new.as_close_future();
                                    };
                                }

                                self.pos = Some(new_pos);
                                self.ring.swap(idx, p);
                                self.ring.make_contiguous()[idx..=new_pos].rotate_right(1);
                            }
                            None => {
                                self.pos = None;
                            }
                        }

                        Some(())
                    }
                }
            }
            None => {
                if idx != 0 {
                    self.ring.front_mut()?.as_past();
                }

                self.ring.make_contiguous()[0..=idx].rotate_right(1);

                Some(())
            }
        }
    }

    pub fn make_close_past_inactive(&mut self, idx: usize) -> Option<()> {
        if let Some(v) = self.get_mut(idx + 1) {
            v.as_close_past();
        }
        if let Some(p) = self.get_mut(idx + 2) {
            p.as_past();
        }
        if let Some(idx) = idx.checked_sub(1) {
            if let Some(cf) = self.get_mut(idx) {
                cf.as_close_future();
            }
        }
        if let Some(idx) = idx.checked_sub(2) {
            if let Some(f) = self.get_mut(idx) {
                f.as_future();
            }
        }

        if let Some(p) = self.pos {
            if p + 1 == idx {
                return Some(());
            }
        }

        let len = self.len();
        if len == 1 {
            self.pos = None;
            return Some(());
        }

        match self.pos {
            Some(p) => {
                match idx.ge(&p) {
                    // past -> close past
                    true => {
                        self.ring.swap(idx, p);

                        if p + 1 < idx {
                            self.ring.make_contiguous()[p..=idx].rotate_right(1);
                        } else {
                            self.pos = p.checked_sub(1);
                        }

                        Some(())
                    }

                    // future -> close past
                    false => {
                        match p.checked_sub(1) {
                            Some(new_pos) => {
                                self.pos = Some(new_pos);
                                self.ring.swap(idx, p);
                                self.ring.make_contiguous()[idx..=new_pos].rotate_right(1);
                            }
                            None => {
                                self.pos = None;
                            }
                        }

                        Some(())
                    }
                }
            }
            None => {
                self.ring.make_contiguous()[0..=idx].rotate_right(1);

                Some(())
            }
        }
    }
}

impl<T> TrackList<T>
where
    T: FrecencyScore,
{
    pub fn frecency(&self) -> Vec<(usize, &T)> {
        let mut vec: Vec<(usize, &T)> = self.ring.iter().enumerate().collect();
        vec.sort_by_key(|(_, v)| v.total_score());
        vec
    }
}

pub trait IndicateCloseness {
    fn as_past(&mut self);
    fn as_future(&mut self);

    fn as_close_past(&mut self);
    fn as_close_future(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    struct Stub {
        id: i32,
        active: bool,
    }
    impl Stub {
        fn new(id: i32, active: bool) -> Stub {
            Stub { id, active }
        }
    }
    impl PartialEq for Stub {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl From<i32> for Stub {
        fn from(id: i32) -> Self {
            Stub::new(id, true)
        }
    }
    impl IndicateCloseness for Stub {
        fn as_past(&mut self) {}
        fn as_future(&mut self) {}
        fn as_close_past(&mut self) {}
        fn as_close_future(&mut self) {}
    }
    impl Mark for Stub {
        fn load_extmark(&mut self) -> Result<()> {
            Ok(())
        }
        fn sync_extmark(&mut self, _: RecordMarkTime) -> Result<()> {
            Ok(())
        }
        fn open_buf(&self) -> Result<()> {
            Ok(())
        }
    }
    impl Active for Stub {
        fn is_active(&self) -> bool {
            self.active
        }
    }

    #[test]
    fn can_go_to_oldest() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());
        list.push(4.into());

        assert_eq!(list.ring.len(), 4);

        assert!(list.pos.is_none());

        assert_eq!(list.step_past().unwrap(), &4.into());
        assert_eq!(list.step_past().unwrap(), &3.into());

        assert_eq!(list.step_future().unwrap(), &3.into());

        assert_eq!(list.step_past().unwrap(), &3.into());
        assert_eq!(list.step_past().unwrap(), &2.into());
        assert_eq!(list.step_past().unwrap(), &1.into());
    }

    #[test]
    fn prevents_out_of_bounds() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());

        assert!(list.step_future().is_none());
        assert!(list.step_future().is_none());

        assert!(list.pos.is_none());
        assert!(list.step_future().is_none());
        assert!(list.step_future().is_none());

        assert_eq!(list.step_past().unwrap(), &3.into());

        list.push(22.into());

        assert_eq!(list.step_past().unwrap(), &22.into());
        assert_eq!(list.step_past().unwrap(), &2.into());
        assert_eq!(list.step_past().unwrap(), &1.into());

        assert!(list.step_past().is_none());
        assert_eq!(list.get_mut(list.pos.unwrap()).unwrap(), &1.into());
        assert!(list.step_past().is_none());
        assert_eq!(list.step_future().unwrap(), &1.into());
        assert_eq!(list.step_future().unwrap(), &2.into());
        assert_eq!(list.step_future().unwrap(), &22.into());
        assert_eq!(list.step_future().unwrap(), &3.into());
    }

    #[test]
    fn inserts_to_the_right_when_not_at_start() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());

        assert_eq!(list.step_past().unwrap(), &3.into());
        list.push(33.into());
        assert_eq!(list.step_past().unwrap(), &33.into());
    }

    #[test]
    fn can_push_when_at_end() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());

        assert_eq!(list.step_past().unwrap(), &3.into());
        assert_eq!(list.step_past().unwrap(), &2.into());
        assert_eq!(list.step_past().unwrap(), &1.into());

        list.push(0.into());
        assert_eq!(list.step_past().unwrap(), &0.into());
    }

    #[test]
    fn does_not_stuck_with_single_element() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());

        assert_eq!(list.step_past().unwrap(), &1.into());
        assert!(list.step_past().is_none());
        assert_eq!(list.step_future().unwrap(), &1.into());
        assert!(list.step_future().is_none());
        assert!(list.step_future().is_none());
        assert_eq!(list.step_past().unwrap(), &1.into());
    }

    #[test]
    fn can_make_second_oldest_element_close_past_while_inside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into()); // will become close past
        list.push(3.into());
        list.push(4.into());
        list.push(5.into()); // we are here
        list.pos = Some(0);

        list.make_close_past(3);

        let want = VecDeque::<Stub>::from([5.into(), 2.into(), 4.into(), 3.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn can_make_newest_element_close_past_while_inside_middle() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into()); // we are here
        list.push(3.into());
        list.push(4.into()); // will become close past
        list.pos = Some(2);

        list.make_close_past(0);

        let want = VecDeque::<Stub>::from([3.into(), 2.into(), 4.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(1));
    }

    #[test]
    fn can_make_newest_element_close_past_while_inside_middle_next_to() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into()); // we are here
        list.push(3.into()); // will become close past
        list.push(4.into());
        list.pos = Some(2);

        list.make_close_past(1);

        let want = VecDeque::<Stub>::from([4.into(), 2.into(), 3.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(1));
    }

    #[test]
    fn can_make_oldest_element_close_past_while_inside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into()); // will become close past
        list.push(2.into());
        list.push(3.into());
        list.push(4.into());
        list.push(5.into()); // we are here
        list.pos = Some(0);

        list.make_close_past(4);

        let want = VecDeque::<Stub>::from([5.into(), 1.into(), 4.into(), 3.into(), 2.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn can_make_middle_element_close_past_while_outside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into()); // will become close past
        list.push(4.into());
        list.push(5.into());
        // we are here
        list.pos = None;

        list.make_close_past(2);

        let want = VecDeque::<Stub>::from([3.into(), 5.into(), 4.into(), 2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_second_element_close_past_while_outside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());
        list.push(4.into()); // will become close past
        list.push(5.into());
        // we are here
        list.pos = None;

        list.make_close_past(1);

        let want = VecDeque::<Stub>::from([4.into(), 5.into(), 3.into(), 2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_fourth_element_close_past_while_outside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into()); // will become close past
        list.push(3.into());
        list.push(4.into());
        list.push(5.into());
        // we are here
        list.pos = None;

        list.make_close_past(3);

        let want = VecDeque::<Stub>::from([2.into(), 5.into(), 4.into(), 3.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_last_elem_of_two_close_past_while_outside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into()); // will become close past
        list.push(2.into());
        // we are here
        list.pos = None;

        list.make_close_past(1);

        let want = VecDeque::<Stub>::from([1.into(), 2.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_first_elem_of_two_close_past_while_outside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into()); // will become close past (already is)
                             // we are here
        list.pos = None;

        list.make_close_past(0);

        let want = VecDeque::<Stub>::from([2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_last_elem_of_two_close_past_while_inside_it() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into()); // we are here and it will become close past
        list.push(2.into());
        list.pos = Some(1);

        list.make_close_past(1);

        let want = VecDeque::<Stub>::from([2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn can_make_first_elem_of_two_close_past_while_inside_it() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into()); // we are here and it will become close past
        list.pos = Some(0);

        list.make_close_past(0);

        let want = VecDeque::<Stub>::from([2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);

        list.make_close_past(0);

        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);

        list.make_close_past(1);

        let want = VecDeque::<Stub>::from([1.into(), 2.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_last_elem_close_past_while_outside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into()); // will become close past
        list.push(2.into());
        list.push(3.into());
        list.push(4.into());
        // we are here
        list.pos = None;

        list.make_close_past(3);

        let want = VecDeque::<Stub>::from([1.into(), 4.into(), 3.into(), 2.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_first_elem_close_past_while_outside() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into()); // will become close past (it already is)
                             // we are here
        list.pos = None;

        list.make_close_past(0);

        let want = VecDeque::<Stub>::from([3.into(), 2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_single_elem_close_past() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.pos = Some(0);
        list.make_close_past(0);
        let want = VecDeque::<Stub>::from([1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);

        let mut list = TrackList::<Stub>::default();
        list.push(1.into());
        list.pos = None;
        list.make_close_past(0);
        let want = VecDeque::<Stub>::from([1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn make_close_past_does_not_get_stuck() {
        let mut list = TrackList::<Stub>::default();
        // we are here
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());
        list.push(4.into());
        list.pos = Some(3);

        list.make_close_past(1);

        let want = VecDeque::<Stub>::from([4.into(), 2.into(), 1.into(), 3.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(2));
    }

    #[test]
    fn make_close_past_last_among_two_when_in_middle() {
        let mut list = TrackList::<Stub>::default();
        list.push(1.into()); // will become close past
        list.push(2.into()); // we are here
        list.pos = Some(0);

        list.make_close_past(1);

        let want = VecDeque::<Stub>::from([2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn pop_past_when_outside() {
        let mut list = TrackList::<Stub>::default();
        let popped: Stub = 3.into();
        list.push(1.into());
        list.push(2.into());
        list.push(popped);
        // we are here
        list.pos = None;

        assert_eq!(list.pop_past().unwrap(), popped);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn pop_past_when_inside() {
        let mut list = TrackList::<Stub>::default();
        let popped: Stub = 1.into();
        list.push(popped);
        list.push(2.into()); // we are here
        list.push(3.into());
        list.pos = Some(1);

        assert_eq!(list.pop_past().unwrap(), popped);
        assert_eq!(list.pos, Some(1));
    }

    #[test]
    fn pop_future_when_outside_get_nothing() {
        let mut list = TrackList::<Stub>::default();
        let popped: Stub = 3.into();
        list.push(1.into());
        list.push(2.into());
        list.push(popped);
        // we are here
        list.pos = None;

        assert!(list.pop_future().is_none());
        assert_eq!(list.pos, None);
    }

    #[test]
    fn pop_future_when_inside() {
        let mut list = TrackList::<Stub>::default();
        let popped: Stub = 2.into();
        list.push(1.into());
        list.push(popped); // we are here
        list.push(3.into());
        list.pos = Some(1);

        assert_eq!(list.pop_future().unwrap(), popped);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn pop_future_when_in_end() {
        let mut list = TrackList::<Stub>::default();
        let popped: Stub = 1.into();
        list.push(popped); // we are here
        list.push(2.into());
        list.push(3.into());
        list.pos = Some(2);

        assert_eq!(list.pop_future().unwrap(), popped);
        assert_eq!(list.pos, Some(1));
    }

    #[test]
    fn pop_future_when_at_start() {
        let mut list = TrackList::<Stub>::default();
        let popped: Stub = 3.into();
        list.push(1.into());
        list.push(2.into());
        list.push(popped); // we are here
        list.pos = Some(0);

        assert_eq!(list.pop_future().unwrap(), popped);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn pop_past_when_outside_ignoring_inactive() {
        let mut list = TrackList::<Stub>::default();
        let popped: Stub = 1.into();
        let inactive: Stub = Stub::new(2, false);
        list.push(popped);
        list.push(inactive); // will become close future
                             // we are here
        list.pos = None;

        assert_eq!(list.pop_past().unwrap(), popped);
        assert_eq!(list.pos, Some(0));
    }
}
