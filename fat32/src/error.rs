use thiserror::Error;

use crate::FileHandle;

#[derive(Error, Debug)]
pub enum Fat32Error {
    #[error("Error reading from fileystem: {0}")]
    IOError(std::io::Error),
    #[error("Invalid BPD: {0}")]
    InvalidBPB(&'static str),
    #[error("Bad cluster: {0}")]
    BadCluster(u32),
    #[error("File is corrupt, missing fat entry")]
    FileCorrupt,
    #[error("File/Directory not found")]
    NotFound,
    #[error("File is a directory")]
    IsDir,
    #[error("File is not a directory")]
    NotADir,
    #[error("Invalid file handle {0}")]
    InvalidFileHandle(FileHandle)
}

pub type Fat32Result<T> = Result<T, Fat32Error>;