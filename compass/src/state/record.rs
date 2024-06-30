use crate::{
    common_types::{CursorPosition, Extmark},
    frecency::{Frecency, FrecencyScore, FrecencyType, FrecencyWeight},
    state::track_list::IndicateCloseness,
    ui::record_mark::{create_record_mark, update_record_mark, RecordMarkTime},
    Result,
};
use std::fmt::Display;

use bitcode::{Decode, Encode};
use nvim_oxi::api::{command, Buffer, Window};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Record {
    pub buf: Buffer,
    pub typ: TypeRecord,
    pub extmark: Extmark,
    pub frecency: Frecency,
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
        let extmark = create_record_mark(buf.clone(), pos, RecordMarkTime::PastClose)?;

        Ok(Self {
            buf,
            typ,
            extmark,
            frecency: Frecency::new(),
        })
    }

    pub fn update(
        &mut self,
        buf: Buffer,
        typ: TypeRecord,
        pos: &CursorPosition,
        hl: RecordMarkTime,
    ) -> Result<()> {
        update_record_mark(&self.extmark, buf.clone(), pos, hl)?;

        self.typ = typ;

        self.frecency.add_record(FrecencyType::Update);

        Ok(())
    }

    fn jump(&mut self, mut win: Window) -> Result<()> {
        let CursorPosition { line, col } = self.extmark.get_pos(self.buf.clone());

        command("normal! m'")?;
        win.set_buf(&self.buf)?;
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

        self.extmark.delete(self.buf.clone())?;

        Ok(())
    }
}

impl IndicateCloseness for Record {
    fn as_past(&self) {
        let _ = update_record_mark(
            &self.extmark,
            self.buf.clone(),
            &self.extmark.get_pos(self.buf.clone()),
            RecordMarkTime::Past,
        );
    }
    fn as_future(&self) {
        let _ = update_record_mark(
            &self.extmark,
            self.buf.clone(),
            &self.extmark.get_pos(self.buf.clone()),
            RecordMarkTime::Future,
        );
    }
    fn as_close_past(&self) {
        let _ = update_record_mark(
            &self.extmark,
            self.buf.clone(),
            &self.extmark.get_pos(self.buf.clone()),
            RecordMarkTime::PastClose,
        );
    }
    fn as_close_future(&self) {
        let _ = update_record_mark(
            &self.extmark,
            self.buf.clone(),
            &self.extmark.get_pos(self.buf.clone()),
            RecordMarkTime::FutureClose,
        );
    }
}

impl FrecencyScore for Record {
    fn total_score(&self) -> FrecencyWeight {
        self.frecency.total_score()
    }
}

mod tests {
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
            .any(|e| e.0 == Into::<u32>::into(&got.extmark)));
    }
}
