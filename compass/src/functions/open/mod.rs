pub mod completion;
pub use completion::*;

mod opts;
use opts::*;

use crate::{
    common_types::{CursorPosition, LazyRedraw},
    config::{get_config, WindowGridSize},
    state::{Record, TrackList, Tracker},
    ui::{
        grid::{open_grid, GridLayout},
        tab::{close_tab, open_tab},
    },
    InputError, Result,
};
use std::{collections::HashSet, sync::Mutex};

use nvim_oxi::api::{set_current_buf, Buffer};

pub fn get_open(tracker: &'static Mutex<Tracker>) -> impl Fn(Option<OpenOptions>) -> Result<()> {
    move |opts: Option<OpenOptions>| {
        let OpenOptions {
            record_types,
            max_windows,
        } = opts.unwrap_or_default();

        let tracker = &mut tracker.lock()?;
        if tracker.list.is_empty() {
            return Err(InputError::NoRecords("record list is empty".to_owned()))?;
        }

        tracker.activate_first()?;

        let record_list: Vec<&Record> = {
            let iter = get_unique_bufs_priority(max_windows, &mut tracker.list)?.into_iter();
            match record_types {
                Some(record_types) => iter
                    .filter(|&r| record_types.iter().any(|&f| f == r.place_type.into()))
                    .collect(),
                None => iter.collect(),
            }
        };
        let layout = {
            match record_list
                .first()
                .is_some_and(|f| record_list.iter().all(|r| r.buf == f.buf))
            {
                true => GridLayout::Follow,
                false => GridLayout::Open,
            }
        };

        open_grid(
            &record_list,
            max_windows,
            layout,
            get_config().picker.jump_keys.iter(),
        )?;

        Ok(())
    }
}

/// HACK: we have to do the whole tab opening and buf setting part because otherwise
/// the extmark positions will be broken if the extmarks were loaded from session management
pub fn get_unique_bufs_priority(
    max_windows: WindowGridSize,
    track_list: &mut TrackList<Record>,
) -> Result<Vec<&Record>> {
    let mut seen_bufs = HashSet::<Buffer>::with_capacity(max_windows.into());
    let mut idx_record_list = Vec::<usize>::with_capacity(max_windows.into());

    LazyRedraw::start()?;

    open_tab()?;

    let frecency_list = track_list.frecency();
    for (i, r) in frecency_list.iter() {
        if seen_bufs.insert(r.buf.clone()) {
            set_current_buf(&r.buf)?;
            idx_record_list.push(*i);

            if idx_record_list.len() >= max_windows.into() {
                break;
            }
        }
    }

    if idx_record_list.len() < max_windows.into() {
        let frecency_list = track_list.frecency();
        for (i, r) in frecency_list.iter() {
            if idx_record_list.iter().all(|&col_idx| col_idx != *i) {
                set_current_buf(&r.buf)?;
                idx_record_list.push(*i);
            }

            if idx_record_list.len() >= max_windows.into() {
                break;
            }
        }
    }

    close_tab()?;

    for i in &idx_record_list {
        if let Some(r) = track_list.get_mut(*i) {
            r.load_extmark()?;
        }
    }

    Ok({
        let mut records = Vec::with_capacity(idx_record_list.len());
        for i in idx_record_list {
            if let Some(r) = track_list.get(i) {
                records.push(r);
            }
        }
        records
    })
}

mod tests {
    use std::sync::Mutex;

    use crate::state::{ChangeTypeRecord, PlaceTypeRecord};

    use super::*;

    use nvim_oxi::api::{create_buf, Buffer};

    #[nvim_oxi::test]
    fn can_open_picker_with_empty_args() {
        let tracker = Box::leak(Box::new(Mutex::new(Tracker::default())));
        tracker.get_mut().unwrap().list.push(
            Record::try_new(
                Buffer::current(),
                PlaceTypeRecord::Change(ChangeTypeRecord::Tick(42.into())),
                &CursorPosition::from((1, 2)),
            )
            .unwrap(),
        );
        let open = get_open(tracker);

        open(None).unwrap();
    }

    #[nvim_oxi::test]
    fn can_open_picker_with_all_args() {
        let tracker = Box::leak(Box::new(Mutex::new(Tracker::default())));
        tracker.get_mut().unwrap().list.push(
            Record::try_new(
                Buffer::current(),
                PlaceTypeRecord::Change(ChangeTypeRecord::Tick(42.into())),
                &CursorPosition::from((1, 2)),
            )
            .unwrap(),
        );
        let open = get_open(tracker);

        open(Some(OpenOptions {
            record_types: Some(vec![RecordFilter::Change]),
            max_windows: 8.try_into().unwrap(),
        }))
        .unwrap();
    }

    #[nvim_oxi::test]
    fn can_get_unique_bufs() {
        let mut tracker = Tracker::default();
        let repeated_buf = create_buf(true, false).unwrap();
        let unique_buf = create_buf(true, false).unwrap();
        let rec1 = Record::try_new(
            repeated_buf.clone(),
            PlaceTypeRecord::Change(ChangeTypeRecord::Tick(4.into())),
            &CursorPosition::from((2, 2)),
        )
        .unwrap();
        tracker.list.push(rec1);
        let rec2 = Record::try_new(
            repeated_buf.clone(),
            PlaceTypeRecord::Change(ChangeTypeRecord::Tick(11.into())),
            &CursorPosition::from((4, 6)),
        )
        .unwrap();
        tracker.list.push(rec2);
        let rec3 = Record::try_new(
            unique_buf.clone(),
            PlaceTypeRecord::Change(ChangeTypeRecord::Tick(15.into())),
            &CursorPosition::from((5, 11)),
        )
        .unwrap();
        tracker.list.push(rec3.clone());
        let rec4 = Record::try_new(
            repeated_buf.clone(),
            PlaceTypeRecord::Change(ChangeTypeRecord::Tick(16.into())),
            &CursorPosition::from((15, 42)),
        )
        .unwrap();
        tracker.list.push(rec4.clone());

        let got = get_unique_bufs_priority(2.try_into().unwrap(), &mut tracker.list).unwrap();
        let mut iter = got.iter();

        assert_eq!(got.len(), 2);
        assert_eq!(**iter.next().unwrap(), rec4);
        assert_eq!(**iter.next().unwrap(), rec3);
    }
}
