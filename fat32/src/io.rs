use std::{fs::File, os::fd::{AsFd, AsRawFd, OwnedFd}};
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
    #[allow(unused)]
    fn seek_and_read(&self, buf: &mut [u8], offset: i64) -> nix::Result<usize> {
        nix::unistd::lseek(self.fd.as_raw_fd(), offset, nix::unistd::Whence::SeekSet)?;
        nix::unistd::read(self.fd.as_raw_fd(), buf)
    }
    fn pread(&self, buf: &mut [u8], offset: i64) -> nix::Result<usize> {
        nix::sys::uio::pread(&self.fd, buf, offset)
    }
    pub fn read(&self, buf: &mut [u8], offset: i64) -> Fat32Result<usize> {
        self.pread(buf, offset).map_err(|errno| {
            Fat32Error::IOError(std::io::Error::from(errno))
        })
    }
}