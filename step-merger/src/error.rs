use std::sync::Arc;

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("Failed to parse assembly JSON")]
    LoadAssembly(#[source] Arc<serde_json::Error>),

    #[error("Invalid child index {0} in node {1}")]
    InvalidFormat(usize, String),

    #[error("Failed to write step file")]
    StepFileWrite(#[source] Arc<std::io::Error>),

    #[error("Data section not found")]
    NoDataSection(),

    #[error("Unexpected identifier: {0}")]
    UnexpectedIdentifier(String),

    #[error("Failed to parsing token")]
    ParsingTokenError(),

    #[error("Unexpected token. Expected {0}, got {1}")]
    UnexpectedToken(String, String),

    #[error("Invalid number")]
    InvalidNumber(String),

    #[error("Failed to read sequence")]
    FailedSequence(#[source] Box<Error>),

    #[error("Failed to read line from file")]
    ReadLineError(#[source] Arc<std::io::Error>),

    #[error("Unexpected end of input")]
    EndOfInput(),

    #[error("Failed to open file: {1}")]
    FailedOpenFile(#[source] Arc<std::io::Error>, String),

    #[error("Failed to read")]
    IO(#[source] Arc<std::io::Error>),

    #[error("Failed to parse utf8 string: {0}")]
    UTF8(#[from] std::str::Utf8Error),

    #[error("No APPLICATION_CONTEXT entry found in step file {0}")]
    AppContextMissing(String),
}

/// The result type used in this crate.
pub type Result<T> = std::result::Result<T, Error>;
