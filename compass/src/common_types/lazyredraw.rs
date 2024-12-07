use nvim_oxi::api::{get_option, set_option};

use crate::Result;

pub struct LazyRedraw(bool);

const LAZY_REDRAW: &str = "lazyredraw";

impl LazyRedraw {
    pub fn start() -> Result<Self> {
        let lazyredraw = get_option::<bool>(LAZY_REDRAW)?;
        set_option(LAZY_REDRAW, true)?;

        Ok(Self(lazyredraw))
    }
}

impl Drop for LazyRedraw {
    fn drop(&mut self) {
        let _ = set_option(LAZY_REDRAW, self.0);
    }
}
