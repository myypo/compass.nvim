use std::collections::vec_deque::{Iter, IterMut, VecDeque};

use crate::{frecency::FrecencyScore, Result};

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
    fn open_buf(&self) -> Result<()>;
}

impl<T> TrackList<T>
where
    T: Mark + IndicateCloseness,
{
    pub fn close_past_mut(&mut self) -> Option<&mut T> {
        match self.pos {
            Some(p) => self.get_mut(p + 1),
            None => self.get_mut(0),
        }
    }

    pub fn close_future_mut(&mut self) -> Option<&mut T> {
        match self.pos {
            Some(p) => self.get_mut(p),
            None => None,
        }
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
                        if let Some(old_close) = self.close_past_mut() {
                            old_close.as_past()
                        };

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

    pub fn iter_from_future(&self) -> Iter<T> {
        self.ring.iter()
    }

    pub fn iter_mut_from_future(&mut self) -> IterMut<T> {
        self.ring.iter_mut()
    }

    fn past_exists(&self) -> bool {
        if self.ring.is_empty() {
            return false;
        }

        let Some(p) = self.pos else {
            return true;
        };

        p + 1 < self.ring.len()
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
    T: IndicateCloseness + Mark,
{
    pub fn push(&mut self, val: T) {
        match self.pos {
            Some(p) => {
                if let Some(old_close) = self.ring.get_mut(p + 1) {
                    old_close.as_past()
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

    pub fn step_past(&mut self) -> Option<&mut T> {
        if !self.past_exists() {
            return None;
        };

        let pos = self.pos.map(|p| p + 1).unwrap_or(0);
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
        if !self.past_exists() {
            return None;
        };

        let pos = self.pos.map(|p| p + 1).unwrap_or(0);

        if let Some(new_close) = self.ring.get_mut(pos + 1) {
            new_close.as_close_past();
        };

        self.ring.remove(pos)
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
}

impl<T> TrackList<T>
where
    T: FrecencyScore,
{
    pub fn frecency(&self) -> Vec<&T> {
        let mut vec: Vec<&T> = self.ring.iter().collect();
        vec.sort_by_key(|v| v.total_score());
        vec
    }

    pub fn frecency_mut(&mut self) -> Vec<&mut T> {
        let mut vec: Vec<&mut T> = self.ring.iter_mut().collect();
        vec.sort_by_key(|v| v.total_score());
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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Num(i32);
    impl From<i32> for Num {
        fn from(value: i32) -> Self {
            Num(value)
        }
    }
    impl IndicateCloseness for Num {
        fn as_past(&mut self) {}
        fn as_future(&mut self) {}
        fn as_close_past(&mut self) {}
        fn as_close_future(&mut self) {}
    }
    impl Mark for Num {
        fn load_extmark(&mut self) -> Result<()> {
            Ok(())
        }
        fn open_buf(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn can_go_to_oldest() {
        let mut list = TrackList::<Num>::default();
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
        let mut list = TrackList::<Num>::default();
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
        assert_eq!(list.get(list.pos.unwrap()).unwrap(), &1.into());
        assert!(list.step_past().is_none());
        assert_eq!(list.step_future().unwrap(), &1.into());
        assert_eq!(list.step_future().unwrap(), &2.into());
        assert_eq!(list.step_future().unwrap(), &22.into());
        assert_eq!(list.step_future().unwrap(), &3.into());
    }

    #[test]
    fn inserts_to_the_right_when_not_at_start() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());

        assert_eq!(list.step_past().unwrap(), &3.into());
        list.push(33.into());
        assert_eq!(list.step_past().unwrap(), &33.into());
    }

    #[test]
    fn can_push_when_at_end() {
        let mut list = TrackList::<Num>::default();
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
        let mut list = TrackList::<Num>::default();
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
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into()); // will become close past
        list.push(3.into());
        list.push(4.into());
        list.push(5.into()); // we are here
        list.pos = Some(0);

        list.make_close_past(3);

        let want = VecDeque::<Num>::from([5.into(), 2.into(), 4.into(), 3.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn can_make_newest_element_close_past_while_inside_middle() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into()); // we are here
        list.push(3.into());
        list.push(4.into()); // will become close past
        list.pos = Some(2);

        list.make_close_past(0);

        let want = VecDeque::<Num>::from([3.into(), 2.into(), 4.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(1));
    }

    #[test]
    fn can_make_newest_element_close_past_while_inside_middle_next_to() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into()); // we are here
        list.push(3.into()); // will become close past
        list.push(4.into());
        list.pos = Some(2);

        list.make_close_past(1);

        let want = VecDeque::<Num>::from([4.into(), 2.into(), 3.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(1));
    }

    #[test]
    fn can_make_oldest_element_close_past_while_inside() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into()); // will become close past
        list.push(2.into());
        list.push(3.into());
        list.push(4.into());
        list.push(5.into()); // we are here
        list.pos = Some(0);

        list.make_close_past(4);

        let want = VecDeque::<Num>::from([5.into(), 1.into(), 4.into(), 3.into(), 2.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn can_make_middle_element_close_past_while_outside() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into()); // will become close past
        list.push(4.into());
        list.push(5.into());
        // we are here
        list.pos = None;

        list.make_close_past(2);

        let want = VecDeque::<Num>::from([3.into(), 5.into(), 4.into(), 2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_second_element_close_past_while_outside() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());
        list.push(4.into()); // will become close past
        list.push(5.into());
        // we are here
        list.pos = None;

        list.make_close_past(1);

        let want = VecDeque::<Num>::from([4.into(), 5.into(), 3.into(), 2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_fourth_element_close_past_while_outside() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into()); // will become close past
        list.push(3.into());
        list.push(4.into());
        list.push(5.into());
        // we are here
        list.pos = None;

        list.make_close_past(3);

        let want = VecDeque::<Num>::from([2.into(), 5.into(), 4.into(), 3.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_last_elem_of_two_close_past_while_outside() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into()); // will become close past
        list.push(2.into());
        // we are here
        list.pos = None;

        list.make_close_past(1);

        let want = VecDeque::<Num>::from([1.into(), 2.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_first_elem_of_two_close_past_while_outside() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into()); // will become close past (already is)
                             // we are here
        list.pos = None;

        list.make_close_past(0);

        let want = VecDeque::<Num>::from([2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_last_elem_of_two_close_past_while_inside_it() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into()); // we are here and it will become close past
        list.push(2.into());
        list.pos = Some(1);

        list.make_close_past(1);

        let want = VecDeque::<Num>::from([2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn can_make_first_elem_of_two_close_past_while_inside_it() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into()); // we are here and it will become close past
        list.pos = Some(0);

        list.make_close_past(0);

        let want = VecDeque::<Num>::from([2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);

        list.make_close_past(0);

        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);

        list.make_close_past(1);

        let want = VecDeque::<Num>::from([1.into(), 2.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_last_elem_close_past_while_outside() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into()); // will become close past
        list.push(2.into());
        list.push(3.into());
        list.push(4.into());
        // we are here
        list.pos = None;

        list.make_close_past(3);

        let want = VecDeque::<Num>::from([1.into(), 4.into(), 3.into(), 2.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_first_elem_close_past_while_outside() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.push(2.into());
        list.push(3.into()); // will become close past (it already is)
                             // we are here
        list.pos = None;

        list.make_close_past(0);

        let want = VecDeque::<Num>::from([3.into(), 2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn can_make_single_elem_close_past() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.pos = Some(0);
        list.make_close_past(0);
        let want = VecDeque::<Num>::from([1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);

        let mut list = TrackList::<Num>::default();
        list.push(1.into());
        list.pos = None;
        list.make_close_past(0);
        let want = VecDeque::<Num>::from([1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, None);
    }

    #[test]
    fn make_close_past_does_not_get_stuck() {
        let mut list = TrackList::<Num>::default();
        // we are here
        list.push(1.into());
        list.push(2.into());
        list.push(3.into());
        list.push(4.into());
        list.pos = Some(3);

        list.make_close_past(1);

        let want = VecDeque::<Num>::from([4.into(), 2.into(), 1.into(), 3.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(2));
    }

    #[test]
    fn make_close_past_last_among_two_when_in_middle() {
        let mut list = TrackList::<Num>::default();
        list.push(1.into()); // will become close past
        list.push(2.into());
        list.pos = Some(0);

        list.make_close_past(1);

        let want = VecDeque::<Num>::from([2.into(), 1.into()]);
        assert_eq!(list.ring, want);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn pop_past_when_outside() {
        let mut list = TrackList::<Num>::default();
        let popped: Num = 3.into();
        list.push(1.into());
        list.push(2.into());
        list.push(popped);
        // we are here
        list.pos = None;

        assert_eq!(list.pop_past().unwrap(), popped);
    }

    #[test]
    fn pop_past_when_inside() {
        let mut list = TrackList::<Num>::default();
        let popped: Num = 1.into();
        list.push(popped);
        list.push(2.into()); // we are here
        list.push(3.into());
        list.pos = Some(1);

        assert_eq!(list.pop_past().unwrap(), popped);
    }

    #[test]
    fn pop_future_when_outside_get_nothing() {
        let mut list = TrackList::<Num>::default();
        let popped: Num = 3.into();
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
        let mut list = TrackList::<Num>::default();
        let popped: Num = 2.into();
        list.push(1.into());
        list.push(popped); // we are here
        list.push(3.into());
        list.pos = Some(1);

        assert_eq!(list.pop_future().unwrap(), popped);
        assert_eq!(list.pos, Some(0));
    }

    #[test]
    fn pop_future_when_in_end() {
        let mut list = TrackList::<Num>::default();
        let popped: Num = 1.into();
        list.push(popped); // we are here
        list.push(2.into());
        list.push(3.into());
        list.pos = Some(2);

        assert_eq!(list.pop_future().unwrap(), popped);
        assert_eq!(list.pos, Some(1));
    }

    #[test]
    fn pop_future_when_at_start() {
        let mut list = TrackList::<Num>::default();
        let popped: Num = 3.into();
        list.push(1.into());
        list.push(2.into());
        list.push(popped); // we are here
        list.pos = Some(0);

        assert_eq!(list.pop_future().unwrap(), popped);
        assert_eq!(list.pos, None);
    }
}
