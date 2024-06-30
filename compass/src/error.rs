use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid input provided: {0}")]
    Input(#[from] InputError),

    #[error("nvim api error: {0}")]
    Api(#[from] nvim_oxi::api::Error),

    #[error("internal compass.nvim error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum InputError {
    #[error("invalid function arguments: {0}")]
    FunctionArguments(String),

    #[error("could not parse json-like input: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid viml: {0}")]
    Viml(#[from] VimlError),

    #[error("invalid enum variant provided: {0}")]
    EnumParse(#[from] strum::ParseError),

    #[error("provided string can't be parsed to an integer: {0}")]
    Int(#[from] std::num::ParseIntError),

    #[error("provided string can't be parsed to a bool: {0}")]
    Bool(#[from] std::str::ParseBoolError),

    #[error("no records satistying the action were found: {0}")]
    NoRecords(String),

    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum VimlError {
    #[error("invalid user command: {0}")]
    InvalidCommand(String),
}
