mod common_types;

mod config;

mod functions;

mod bootstrap;

mod error;
use error::*;

mod ui;

mod viml;

mod state;

mod highlights;

#[nvim_oxi::plugin]
pub fn compass() -> Result<nvim_oxi::Dictionary> {
    bootstrap::init()
}
