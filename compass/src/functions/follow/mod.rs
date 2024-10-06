mod completion;
use std::sync::Mutex;

pub use completion::*;

mod opts;
use opts::*;

use crate::{
    config::{get_config, JumpKeymap, WindowGridSize},
    functions::open::get_unique_bufs_priority,
    state::{Record, TrackList, Tracker},
    ui::grid::{open_grid, GridLayout},
    InputError, Result,
};

use anyhow::anyhow;
use nvim_oxi::api::Buffer;

pub fn get_follow(
    tracker: &'static Mutex<Tracker>,
) -> impl Fn(Option<FollowOptions>) -> Result<()> {
    move |opts: Option<FollowOptions>| {
        let opts = opts.unwrap_or_default();

        let tracker = &mut tracker.lock()?;
        if tracker.list.is_empty() {
            return Err(InputError::NoRecords("record list is empty".to_owned()))?;
        }

        tracker.activate_first()?;

        match opts {
            FollowOptions::Buf(BufOptions {
                target,
                max_windows,
            }) => {
                {
                    let mut records_iter = tracker
                        .list
                        .iter_from_future()
                        .enumerate()
                        .filter(|(_, r)| r.buf == target);

                    if let Some((i, _)) = records_iter.next() {
                        if records_iter.next().is_none() {
                            return tracker.goto_absolute(i);
                        };
                    };
                };

                let (record_vec, jump_keymaps) =
                    follow_buf(target, &mut tracker.list, max_windows)?;

                open_grid(
                    &record_vec,
                    max_windows,
                    GridLayout::Follow,
                    jump_keymaps.into_iter(),
                )?;

                Ok(())
            }
        }
    }
}

fn follow_buf(
    target: Buffer,
    record_list: &mut TrackList<Record>,
    max_windows: WindowGridSize,
) -> Result<(Vec<&Record>, Vec<&JumpKeymap>)> {
    let mut record_vec = Vec::<&Record>::new();
    let mut reused_keymap_vec: Vec<usize> = Vec::new();

    for (i, r) in get_unique_bufs_priority(max_windows, record_list)?
        .iter()
        .enumerate()
    {
        if record_vec.len() >= max_windows.into() {
            break;
        };

        if r.buf == target {
            record_vec.push(r);
            reused_keymap_vec.push(i);
        }
    }

    let jump_keymaps = {
        let mut jump_vec: Vec<&JumpKeymap> = Vec::new();
        for &i in reused_keymap_vec.iter() {
            let v = match get_config().picker.jump_keys.get(i) {
                Some(k) => k,
                None => get_config()
                    .picker
                    .jump_keys
                    .iter()
                    .find(|k| !jump_vec.contains(k))
                    .ok_or_else(|| anyhow!("unexpectedly there is not enough jump keys to be used for follow command"))?,
            };

            jump_vec.push(v);
        }

        jump_vec
    };

    Ok((record_vec, jump_keymaps))
}

mod tests {
    use crate::state::{ChangeTypeRecord, PlaceTypeRecord};

    use super::*;

    use nvim_oxi::api::create_buf;

    #[nvim_oxi::test]
    fn can_reuse_keymap_with_follow_buf() {
        let buf1 = create_buf(true, false).unwrap();
        let buf2 = create_buf(true, false).unwrap();
        let mut record_list: TrackList<Record> = TrackList::default();
        record_list.push(
            Record::try_new(
                buf1.clone(),
                PlaceTypeRecord::Change(ChangeTypeRecord::Tick(3.into())),
                &(3, 3).into(),
            )
            .unwrap(),
        ); // k
        record_list.push(
            Record::try_new(
                buf2,
                PlaceTypeRecord::Change(ChangeTypeRecord::Tick(2.into())),
                &(2, 2).into(),
            )
            .unwrap(),
        ); // f
        record_list.push(
            Record::try_new(
                buf1.clone(),
                PlaceTypeRecord::Change(ChangeTypeRecord::Tick(1.into())),
                &(1, 1).into(),
            )
            .unwrap(),
        ); // j

        let (record_vec, jump_keymaps) =
            follow_buf(buf1, &mut record_list, WindowGridSize::default()).unwrap();

        assert_eq!(
            record_vec.first().unwrap().place_type,
            PlaceTypeRecord::Change(ChangeTypeRecord::Tick(1.into()))
        );
        assert_eq!(
            record_vec.get(1).unwrap().place_type,
            PlaceTypeRecord::Change(ChangeTypeRecord::Tick(3.into()))
        );
        let def_keymaps = &get_config().picker.jump_keys;
        assert_eq!(*jump_keymaps.first().unwrap(), def_keymaps.get(0).unwrap()); // j
        assert_eq!(*jump_keymaps.get(1).unwrap(), def_keymaps.get(2).unwrap()); // k
    }
}
