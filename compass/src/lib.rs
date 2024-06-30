mod common_types;

mod config;

mod frecency;

mod functions;

mod bootstrap;

mod error;
use error::*;

mod ui;

mod viml;

mod state;

#[nvim_oxi::plugin]
pub fn compass() -> Result<nvim_oxi::Dictionary> {
    bootstrap::init()
}
