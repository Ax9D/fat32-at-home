use std::{ffi::{c_int, OsStr}, path::PathBuf, sync::Arc, time::Duration};

use fat32::{Driver, Fat32Result, FatDirectory};
use fuser::{FileAttr, FileType, Filesystem, MountOption};
use nix::libc;
use parking_lot::Mutex;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::inode::InodeResolver;

macro_rules! try_io {
    ($fat32_result: expr, $reply: expr) => {
        {
            match $fat32_result {
                Ok(something) => {
                    something
                },
                Err(fat32::Fat32Error::NotFound) => {
                    log::debug!("ENOENT");
                    $reply.error(libc::ENOENT);
                    return;
                },
                Err(fat32::Fat32Error::NotADir) => {
                    log::debug!("ENOTDIR");
                    $reply.error(libc::ENOTDIR);
                    return;
                },
                Err(fat32::Fat32Error::IsDir) => {
                    log::debug!("EISDIR");
                    $reply.error(libc::EISDIR);
                    return;
                },
                Err(_) => {
                    log::debug!("EIO");
                    $reply.error(libc::EIO);
                    return;
                }
            }
        }
    };
}
fn file_type_of(directory: &FatDirectory) -> FileType {
    if directory.is_file() {
        FileType::RegularFile
    } else {
        FileType::Directory
    }
}

const FMODE_EXEC: i32 = 0x20;

pub struct Fat32 {
    driver: Arc<Driver>,
    inode_resolver: Mutex<InodeResolver>,
    mount_permissions_mask: u16,
    mount_uid: u32,
    mount_gid: u32, 
    tp: ThreadPool
}
impl Fat32 {
    pub fn new(driver: Driver, uid: u32, gid: u32, mount_options: &Vec<MountOption>) -> Self {
        let mut mount_permissions_mask = 0;

        let mut rwx = 0b000;
        for option in mount_options {
            match option {
                MountOption::RO => {
                    rwx |= 0b100;
                },
                MountOption::Exec => {
                    rwx |= 0b001;
                },
                MountOption::RW => unimplemented!("RW mount unsupported"),
                _=> {}
            }
        }

        mount_permissions_mask = 0o644;
        Self {
            driver: Arc::new(driver),
            mount_permissions_mask,
            inode_resolver: Mutex::new(InodeResolver::new()),
            mount_uid: uid,
            mount_gid: gid,
            tp: ThreadPoolBuilder::new().build().unwrap()
        }
    }
    fn file_attr_of(&self, directory: &FatDirectory, inode: u64, req: &fuser::Request) -> Fat32Result<FileAttr> {
        let file_attr = FileAttr {
            ino: inode,
            size: directory.file_size() as u64,
            blocks: directory.n_clusters(&self.driver)? as u64,
            atime: directory.access_time(),
            mtime: directory.write_time(),
            ctime: directory.write_time(),
            crtime: directory.create_time(),
            kind: file_type_of(directory),
            perm: self.mount_permissions_mask,
            nlink: 1,
            uid: self.mount_uid,
            gid: self.mount_gid,
            rdev: 0,
            blksize: self.driver.bytes_per_cluster() as u32,
            flags: 0,
        };

        Ok(file_attr)
    }
    fn permissions(&self, directory: &FatDirectory) -> u16 {


        todo!()
    }
    fn get_path(&self, inode: u64) -> PathBuf {
        let inode_resolver = self.inode_resolver.lock();
        inode_resolver.path(inode).to_owned()
    } 
    fn check_access(&self, _read: bool, write: bool, _execute: bool) -> bool {
        // Read Only FS for now
        if write {
            return false;
        }   

        true
    }
}
impl Filesystem for Fat32 {
    fn init(&mut self, req: &fuser::Request<'_>, config: &mut fuser::KernelConfig) -> Result<(), c_int> {
 
        Ok(())
    }
    fn lookup(&mut self, req: &fuser::Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEntry) {
        
        let mut inode_resolver = self.inode_resolver.lock();
        
        let parent_path = inode_resolver.path(parent);
        let path = parent_path.join(name);
        log::debug!("lookup {:?}", name);
        let found = try_io!(self.driver.search_by_path(&path), reply);
        let inode = inode_resolver.get_or_assign_inode(parent, name);
        log::debug!("lookup {:?} = {}", path, inode);
        
        let file_attr = try_io!(self.file_attr_of(&found, inode, req), reply);
        reply.entry(&Duration::new(0, 0), &file_attr, 0);

    }
    fn getattr(&mut self, req: &fuser::Request<'_>, inode: u64, reply: fuser::ReplyAttr) {
        let path = self.get_path(inode);

        let file = try_io!(self.driver.search_by_path(&path), reply);
        let attr = try_io!(self.file_attr_of(&file, inode, req), reply);

        reply.attr(&Duration::new(0, 0), &attr);
    }
    fn opendir(&mut self, _req: &fuser::Request<'_>, inode: u64, flags: i32, reply: fuser::ReplyOpen) {
        let (_access_mask, read, write) = match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                // Behavior is undefined, but most filesystems return EACCES
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                (libc::R_OK, true, false)
            }
            libc::O_WRONLY => (libc::W_OK, false, true),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true),
            // Exactly one access mode flag must be specified
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        if !self.check_access(read, write, false) {
            reply.error(libc::EACCES);
            return;
        }
        
        let path = self.get_path(inode);

        println!("Open {}", path.display());
        let fh = try_io!(self.driver.open_dir(&path), reply);
        println!("Opened {}={}", path.display(), fh);

        reply.opened(fh, 0);
    }
    fn releasedir(
            &mut self,
            _req: &fuser::Request<'_>,
            _inode: u64,
            fh: u64,
            _flags: i32,
            reply: fuser::ReplyEmpty,
        ) {
        try_io!(self.driver.close_dir(fh), reply);
        reply.ok();
    }
    fn readdir(
            &mut self,
            _req: &fuser::Request<'_>,
            ino: u64,
            fh: u64,
            offset: i64,
            mut reply: fuser::ReplyDirectory,
        ) {
            let mut offset = offset as usize;
            
            while let Some(file) = try_io!(self.driver.read_dir(fh, offset), reply) {
                offset += 1;
                println!("{:?}", file.name());
                let buffer_full = reply.add(ino, offset as i64, file_type_of(&file), file.name());
            
                if buffer_full {
                    break;
                }
            }

            reply.ok()
    }
    fn open(&mut self, _req: &fuser::Request<'_>, inode: u64, flags: i32, reply: fuser::ReplyOpen) {
        let (access_mask, read, write, exec) = match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                // Behavior is undefined, but most filesystems return EACCES
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                if flags & FMODE_EXEC != 0 {
                    // Open is from internal exec syscall
                    (libc::X_OK, true, false, true)
                } else {
                    (libc::R_OK, true, false, false)
                }
            }
            libc::O_WRONLY => (libc::W_OK, false, true, false),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true, false),
            // Exactly one access mode flag must be specified
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let path = self.get_path(inode);
        let fh = try_io!(self.driver.open(&path), reply);

        if !self.check_access(read, write, exec) {
            reply.error(libc::EACCES);
            return;
        }

        reply.opened(fh, 0)
    }
    fn release(
            &mut self,
            _req: &fuser::Request<'_>,
            _ino: u64,
            fh: u64,
            _flags: i32,
            _lock_owner: Option<u64>,
            _flush: bool,
            reply: fuser::ReplyEmpty,
        ) {
        try_io!(self.driver.close(fh), reply);

        reply.ok()
    }
    fn read(
            &mut self,
            _req: &fuser::Request<'_>,
            _inode: u64,
            fh: u64,
            offset: i64,
            size: u32,
            flags: i32,
            _lock_owner: Option<u64>,
            reply: fuser::ReplyData,
        ) {
        let driver = self.driver.clone();

        let (_access_mask, read, write, exec) = match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                // Behavior is undefined, but most filesystems return EACCES
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                if flags & FMODE_EXEC != 0 {
                    // Open is from internal exec syscall
                    (libc::X_OK, true, false, true)
                } else {
                    (libc::R_OK, true, false, false)
                }
            }
            libc::O_WRONLY => (libc::W_OK, false, true, false),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true, false),
            // Exactly one access mode flag must be specified
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        if !self.check_access(read, write, exec) {
            reply.error(libc::EACCES);
            return;
        }

        // self.tp.spawn(move|| {
            let offset = offset as usize;

            let mut read_buf = vec![0; size as usize];
            let byte_offset = offset as usize;
    
            let nbytes = try_io!(driver.read(fh, &mut read_buf, byte_offset), reply);
     
            reply.data(&read_buf[0..nbytes])
        // });

    }
}