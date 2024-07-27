use crate::{
    common_types::{CursorPosition, CursorRange, Extmark},
    frecency::{Frecency, FrecencyScore, FrecencyType, FrecencyWeight},
    state::track_list::IndicateCloseness,
    ui::record_mark::{create_record_mark, update_record_mark, RecordMarkTime},
    Result,
};
use std::fmt::Display;

use bitcode::{Decode, Encode};
use nvim_oxi::api::{command, set_current_buf, Buffer, Window};
use serde::Deserialize;

use super::track_list::Mark;

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
}

impl LazyExtmark {
    pub fn get_pos(&self, buf: Buffer) -> CursorPosition {
        match self {
            Self::Loaded(e) => e.get_pos(buf),
            Self::Unloaded((p, _)) => p.clone(),
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

    pub fn get_or_init_extmark(&mut self) -> Result<Extmark> {
        Ok(match &self.lazy_extmark {
            LazyExtmark::Loaded(e) => e.clone(),
            LazyExtmark::Unloaded((p, t)) => {
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
        pos: &CursorPosition,
        hl: RecordMarkTime,
    ) -> Result<()> {
        update_record_mark(&self.get_or_init_extmark()?, buf.clone(), &pos.into(), hl)?;

        self.typ = typ;

        self.frecency.add_record(FrecencyType::Update);

        Ok(())
    }

    fn jump(&mut self, mut win: Window) -> Result<()> {
        // Leave an entry in the jumplist
        command("normal! m'")?;

        let CursorPosition { line, col } = self.get_or_init_extmark()?.get_pos(self.buf.clone());
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

    fn loaded_extmark(&self) -> Option<&Extmark> {
        match &self.lazy_extmark {
            LazyExtmark::Loaded(e) => Some(e),
            _ => None,
        }
    }
}

impl IndicateCloseness for Record {
    fn as_past(&mut self) {
        let Some(ext) = self.loaded_extmark() else {
            return;
        };

        let _ = update_record_mark(
            ext,
            self.buf.clone(),
            &ext.get_range(self.buf.clone()),
            RecordMarkTime::Past,
        );
    }
    fn as_future(&mut self) {
        let Some(ext) = self.loaded_extmark() else {
            return;
        };

        let _ = update_record_mark(
            ext,
            self.buf.clone(),
            &ext.get_range(self.buf.clone()),
            RecordMarkTime::Future,
        );
    }
    fn as_close_past(&mut self) {
        let Some(ext) = self.loaded_extmark() else {
            return;
        };

        let _ = update_record_mark(
            ext,
            self.buf.clone(),
            &ext.get_range(self.buf.clone()),
            RecordMarkTime::PastClose,
        );
    }
    fn as_close_future(&mut self) {
        let Some(ext) = self.loaded_extmark() else {
            return;
        };

        let _ = update_record_mark(
            ext,
            self.buf.clone(),
            &ext.get_range(self.buf.clone()),
            RecordMarkTime::FutureClose,
        );
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

            _ => Ok(()),
        }
    }

    fn open_buf(&self) -> Result<()> {
        set_current_buf(&self.buf)?;
        Ok(())
    }
}

mod tests {
    use core::panic;

    use crate::state::get_namespace;

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
