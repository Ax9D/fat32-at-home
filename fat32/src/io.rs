use std::{fs::File, os::fd::{AsFd, OwnedFd}};
use std::io::Result;

use crate::{Fat32Error, Fat32Result};


pub struct Drive {
    fd: OwnedFd
}

impl Drive {
    pub fn from_file(file: File) -> Result<Self>  {
        Ok(Self {
            fd: file.as_fd().try_clone_to_owned()?
        })
    }
    pub fn read(&self, buf: &mut [u8], offset: i64) -> Fat32Result<usize> {
        nix::sys::uio::pread(&self.fd, buf, offset).map_err(|errno| {
            Fat32Error::IOError(std::io::Error::from(errno))
        })
    }
}