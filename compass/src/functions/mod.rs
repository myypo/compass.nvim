//! Plugin's functions exposed to lua
//! Uses a macro to generate enums for each submodule
//! `CommandNames` enum represents functions that need a user command to be generated for them

pub mod setup;

pub mod goto;

pub mod open;

pub mod place;

pub mod follow;

pub mod pop;

macros::functions_and_commands!("./src/functions");
