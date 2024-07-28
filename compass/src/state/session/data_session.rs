use crate::{
    common_types::CursorPosition,
    frecency::Frecency,
    state::{record::LazyExtmark, Record, TrackList, TypeRecord},
    ui::record_mark::recreate_mark_time,
    Error, Result,
};

use bitcode::{Decode, Encode};

#[derive(Decode, Encode)]
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
    pub typ: TypeRecord,
    pub frecency: Frecency,
    pub cursor_pos: CursorPosition,
}

#[derive(Decode, Encode)]
pub enum Version {
    One = 1,
}

impl TryFrom<&Record> for PersistentRecord {
    type Error = Error;

    fn try_from(
        Record {
            buf,
            typ,
            lazy_extmark,
            frecency,
        }: &Record,
    ) -> Result<Self> {
        let cursor_pos = lazy_extmark.pos(buf.clone());

        Ok(Self {
            buf_handle: buf.handle(),
            typ: *typ,
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
            version: Version::One,
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

        for (i, r) in session.data.records.into_iter().enumerate() {
            track_list.push_plain(Record {
                buf: r.buf_handle.into(),
                typ: r.typ,
                lazy_extmark: LazyExtmark::Unloaded((
                    r.cursor_pos,
                    recreate_mark_time(i, track_list.pos),
                )),
                frecency: r.frecency,
            });
        }

        Ok(track_list)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: Version::One,
            data: DataSession::default(),
        }
    }
}
