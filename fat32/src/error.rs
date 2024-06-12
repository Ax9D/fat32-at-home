

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Fat32Error {
    #[error("Error reading from fileystem: {0}")]
    IOError(std::io::Error),
    #[error("Invalid BPD: {0}")]
    InvalidBPB(&'static str)
}

pub type Fat32Result<T> = Result<T, Fat32Error>;