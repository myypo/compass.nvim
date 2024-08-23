use crate::{
    common_types::{CursorPosition, CursorRange, Extmark},
    state::frecency::{Frecency, FrecencyScore, FrecencyType, FrecencyWeight},
    state::track_list::IndicateCloseness,
    ui::record_mark::{create_record_mark, update_record_mark, RecordMarkTime},
    Result,
};
use std::{fmt::Display, time::Instant};

use bitcode::{Decode, Encode};
use nvim_oxi::api::{command, set_current_buf, Buffer, Window};
use serde::Deserialize;

use super::track_list::{Active, Mark};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Record {
    pub buf: Buffer,
    pub typ: TypeRecord,
    pub lazy_extmark: LazyExtmark,
    pub frecency: Frecency,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LazyExtmark {
    Loaded(Extmark),
    Unloaded((CursorPosition, RecordMarkTime)),
    Inactive((CursorPosition, RecordMarkTime, Instant)),
}

impl LazyExtmark {
    pub fn pos(&self, buf: Buffer) -> CursorPosition {
        match self {
            Self::Loaded(e) => e.pos(buf),
            Self::Unloaded((p, _)) => p.clone(),
            Self::Inactive((p, _, _)) => p.clone(),
        }
    }

    pub fn id(&self) -> Option<u32> {
        match self {
            Self::Loaded(e) => Some(e.id),
            _ => None,
        }
    }

    pub fn delete(&self, buf: Buffer) -> Result<()> {
        match self {
            Self::Loaded(e) => e.delete(buf),
            Self::Unloaded(_) => Ok(()),
            Self::Inactive(_) => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Decode, Encode)]
pub enum TypeRecord {
    Change(ChangeTypeRecord),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Decode, Encode, Deserialize)]
#[serde(transparent)]
pub struct Tick(i32);

impl Display for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for Tick {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Decode, Encode)]
pub enum ChangeTypeRecord {
    Tick(Tick),
    Manual(Option<Tick>),
}

impl TypeRecord {
    pub fn tick(self) -> Option<Tick> {
        match self {
            Self::Change(c) => match c {
                ChangeTypeRecord::Tick(t) => Some(t),
                ChangeTypeRecord::Manual(t) => t,
            },
        }
    }
}

impl Record {
    pub fn try_new(buf: Buffer, typ: TypeRecord, pos: &CursorPosition) -> Result<Self> {
        let extmark = create_record_mark(buf.clone(), &pos.into(), RecordMarkTime::PastClose)?;

        Ok(Self {
            buf,
            typ,
            lazy_extmark: LazyExtmark::Loaded(extmark),
            frecency: Frecency::new(),
        })
    }

    pub fn try_new_inactive(buf: Buffer, typ: TypeRecord, pos: CursorPosition) -> Result<Self> {
        Ok(Self {
            buf,
            typ,
            lazy_extmark: LazyExtmark::Inactive((pos, RecordMarkTime::PastClose, Instant::now())),
            frecency: Frecency::new(),
        })
    }

    pub fn get_or_init_extmark(&mut self) -> Result<Extmark> {
        Ok(match &self.lazy_extmark {
            LazyExtmark::Loaded(e) => e.clone(),
            LazyExtmark::Unloaded((p, t)) => {
                let extmark =
                    create_record_mark(self.buf.clone(), &Into::<CursorRange>::into(p), *t)?;
                self.lazy_extmark = LazyExtmark::Loaded(extmark.clone());

                extmark
            }
            LazyExtmark::Inactive((p, t, _)) => {
                let extmark =
                    create_record_mark(self.buf.clone(), &Into::<CursorRange>::into(p), *t)?;
                self.lazy_extmark = LazyExtmark::Loaded(extmark.clone());

                extmark
            }
        })
    }

    pub fn update(
        &mut self,
        buf: Buffer,
        typ: TypeRecord,
        pos: CursorPosition,
        time: RecordMarkTime,
    ) -> Result<()> {
        match &self.lazy_extmark {
            LazyExtmark::Loaded(e) => {
                update_record_mark(e, buf.clone(), &Into::<CursorRange>::into(&pos), time)?
            }
            LazyExtmark::Unloaded(_) => {
                let extmark =
                    create_record_mark(self.buf.clone(), &Into::<CursorRange>::into(&pos), time)?;
                self.lazy_extmark = LazyExtmark::Loaded(extmark.clone());
            }
            LazyExtmark::Inactive(_) => {
                let extmark =
                    create_record_mark(self.buf.clone(), &Into::<CursorRange>::into(&pos), time)?;
                self.lazy_extmark = LazyExtmark::Loaded(extmark.clone());
            }
        };

        self.typ = typ;
        self.frecency.add_record(FrecencyType::Update);

        Ok(())
    }

    pub fn deact_update(
        &mut self,
        buf: Buffer,
        typ: TypeRecord,
        pos: CursorPosition,
        time: RecordMarkTime,
    ) -> Result<()> {
        if let LazyExtmark::Loaded(e) = &self.lazy_extmark {
            e.delete(buf)?;
        }

        self.lazy_extmark = LazyExtmark::Inactive((pos, time, Instant::now()));
        self.typ = typ;
        self.frecency.add_record(FrecencyType::Update);

        Ok(())
    }

    fn jump(&mut self, mut win: Window) -> Result<()> {
        // Leave an entry in the jumplist
        command("normal! m'")?;

        let CursorPosition { line, col } = self.get_or_init_extmark()?.pos(self.buf.clone());
        win.set_cursor(line, col)?;

        Ok(())
    }

    pub fn goto(&mut self, win: Window, typ: FrecencyType) -> Result<()> {
        self.jump(win)?;

        self.frecency.add_record(typ);

        Ok(())
    }

    pub fn pop(&mut self, win: Window) -> Result<()> {
        self.jump(win)?;
        self.get_or_init_extmark()?.delete(self.buf.clone())?;

        Ok(())
    }

    fn set_time(&mut self, time: RecordMarkTime) {
        match &self.lazy_extmark {
            LazyExtmark::Loaded(e) => {
                let _ =
                    update_record_mark(e, self.buf.clone(), &e.get_range(self.buf.clone()), time);
            }
            LazyExtmark::Unloaded((p, _)) => {
                self.lazy_extmark = LazyExtmark::Unloaded((p.clone(), time));
            }
            LazyExtmark::Inactive((p, _, i)) => {
                self.lazy_extmark = LazyExtmark::Inactive((p.clone(), time, *i));
            }
        }
    }
}

impl IndicateCloseness for Record {
    fn as_past(&mut self) {
        self.set_time(RecordMarkTime::Past);
    }
    fn as_future(&mut self) {
        self.set_time(RecordMarkTime::Future);
    }
    fn as_close_past(&mut self) {
        self.set_time(RecordMarkTime::PastClose);
    }
    fn as_close_future(&mut self) {
        self.set_time(RecordMarkTime::FutureClose);
    }
}

impl FrecencyScore for Record {
    fn total_score(&self) -> FrecencyWeight {
        self.frecency.total_score()
    }
}

impl Mark for Record {
    fn load_extmark(&mut self) -> Result<()> {
        match &self.lazy_extmark {
            LazyExtmark::Unloaded((p, t)) => {
                let extmark =
                    create_record_mark(self.buf.clone(), &Into::<CursorRange>::into(p), *t)?;
                self.lazy_extmark = LazyExtmark::Loaded(extmark.clone());

                Ok(())
            }
            LazyExtmark::Inactive((p, t, _)) => {
                let extmark =
                    create_record_mark(self.buf.clone(), &Into::<CursorRange>::into(p), *t)?;
                self.lazy_extmark = LazyExtmark::Loaded(extmark.clone());

                Ok(())
            }

            LazyExtmark::Loaded(_) => Ok(()),
        }
    }

    fn open_buf(&self) -> Result<()> {
        set_current_buf(&self.buf)?;
        Ok(())
    }
}

impl Active for Record {
    fn is_active(&self) -> bool {
        !matches!(self.lazy_extmark, LazyExtmark::Inactive(_))
    }
}

mod tests {
    use core::panic;

    use crate::ui::namespace::get_namespace;

    use super::*;

    use nvim_oxi::api::{get_current_buf, opts::GetExtmarksOpts, types::ExtmarkPosition};

    #[nvim_oxi::test]
    fn can_create_record_in_current_buffer() {
        let buf = get_current_buf();
        let pos = CursorPosition::from((1, 0));

        let got = Record::try_new(
            buf.clone(),
            TypeRecord::Change(ChangeTypeRecord::Tick(15.into())),
            &pos,
        )
        .unwrap();

        assert!(buf
            .get_extmarks(
                get_namespace().into(),
                ExtmarkPosition::ByTuple((0, 0)),
                ExtmarkPosition::ByTuple((100, 100)),
                &GetExtmarksOpts::builder().build(),
            )
            .unwrap()
            .any(|e| e.0
                == Into::<u32>::into(match &got.lazy_extmark {
                    LazyExtmark::Loaded(e) => e,
                    _ => panic!(""),
                })));
    }
}
