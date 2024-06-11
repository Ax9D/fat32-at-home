use thiserror::Error;

#[derive(Error, Debug)]
pub enum Fat32Error {
    #[error("Error reading from fileystem: {0}")]
    InvalidBPB(String),
}

pub type Fat32Result<T> = Result<T, Fat32Error>;