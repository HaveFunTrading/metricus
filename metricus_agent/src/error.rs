use std::fmt::Display;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("other error: {0}")]
    Other(String),
}

impl Error {
    pub fn other<T: Display>(msg: T) -> Self {
        Self::Other(msg.to_string())
    }
}
