use crate::{
    common_types::CursorPosition,
    state::{
        frecency::Frecency, record::LazyExtmark, ChangeTypeRecord, PlaceTypeRecord, Record,
        TrackList,
    },
    ui::record_mark::recreate_mark_time,
    Error, Result,
};

use bitcode::{Decode, Encode};
use nvim_oxi::api::Buffer;

#[derive(Decode, Encode, Default)]
pub struct Session {
    pub version: Version,
    pub data: DataSession,
}

#[derive(Default, Decode, Encode)]
pub struct DataSession {
    pub pos: Option<usize>,
    pub records: Vec<PersistentRecord>,
}

#[derive(Decode, Encode)]
pub struct PersistentRecord {
    pub buf_handle: i32,
    pub place_type: PlaceTypeRecord,
    pub frecency: Frecency,
    pub cursor_pos: CursorPosition,
}

#[derive(Decode, Encode, Default)]
pub enum Version {
    #[default]
    One = 1,
}

impl TryFrom<&Record> for PersistentRecord {
    type Error = Error;

    fn try_from(
        Record {
            buf,
            lazy_extmark,
            frecency,
            ..
        }: &Record,
    ) -> Result<Self> {
        let cursor_pos = lazy_extmark.pos(buf.clone());

        Ok(Self {
            buf_handle: buf.handle(),
            place_type: PlaceTypeRecord::Change(ChangeTypeRecord::Restored),
            cursor_pos,
            // TODO: this is bad
            frecency: frecency.clone(),
        })
    }
}

impl TryFrom<&TrackList<Record>> for Session {
    type Error = Error;

    fn try_from(data: &TrackList<Record>) -> Result<Self> {
        let mut records: Vec<PersistentRecord> = Vec::with_capacity(data.len());
        for r in data.iter_from_future() {
            records.push(r.try_into()?);
        }

        Ok(Self {
            version: Version::default(),
            data: DataSession {
                pos: data.pos,
                records,
            },
        })
    }
}

impl TryFrom<Session> for TrackList<Record> {
    type Error = Error;

    fn try_from(session: Session) -> Result<Self> {
        let mut track_list: TrackList<Record> =
            TrackList::with_capacity(session.data.records.len(), session.data.pos);

        for r in session.data.records.into_iter().enumerate().filter_map(
            |(
                i,
                PersistentRecord {
                    buf_handle,
                    place_type,
                    frecency,
                    cursor_pos,
                },
            )| {
                let buf: Buffer = buf_handle.into();

                if !buf.is_valid() {
                    return None;
                }

                Some(Record {
                    buf,
                    place_type,
                    lazy_extmark: LazyExtmark::Unloaded((
                        cursor_pos,
                        recreate_mark_time(i, session.data.pos),
                    )),
                    frecency,
                })
            },
        ) {
            track_list.push_plain(r);
        }

        Ok(track_list)
    }
}
