use std::hash::Hash;

use crate::{common_types::CursorPosition, state::get_namespace, Result};

use nvim_oxi::api::{opts::GetExtmarkByIdOpts, Buffer};

use super::CursorRange;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Extmark {
    pub id: u32,
    init_pos: CursorPosition,
}

impl Extmark {
    pub fn try_new(id: u32, buf: Buffer) -> Result<Self> {
        let init_pos = get_extmark_pos(id, buf)?;

        Ok(Self { id, init_pos })
    }

    pub fn new(id: u32, init_pos: CursorPosition) -> Self {
        Self { id, init_pos }
    }

    pub fn pos(&self, buf: Buffer) -> CursorPosition {
        let Ok(pos) = get_extmark_pos(self.id, buf) else {
            return self.init_pos.clone();
        };

        // HACK: when (2, 0) we assume the buf was not yet properly opened
        if pos == (2, 0).into() {
            self.init_pos.clone()
        } else {
            pos
        }
    }

    pub fn get_range(&self, buf: Buffer) -> CursorRange {
        Into::<CursorRange>::into(&self.pos(buf))
    }

    pub fn delete(&self, mut buf: Buffer) -> Result<()> {
        Ok(buf.del_extmark(get_namespace().into(), self.into())?)
    }
}

// Returns a 1,0 indexed cursor position
fn get_extmark_pos(id: u32, buf: Buffer) -> Result<CursorPosition> {
    let (line, col, _) = buf.get_extmark_by_id(
        get_namespace().into(),
        id,
        &GetExtmarkByIdOpts::builder()
            .details(false)
            .hl_name(false)
            .build(),
    )?;

    Ok((line + 1, col).into())
}

impl From<&Extmark> for u32 {
    fn from(val: &Extmark) -> Self {
        val.id
    }
}
