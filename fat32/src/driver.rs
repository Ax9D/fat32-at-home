use std::ffi::OsStr;
use std::path::{Component, Path};

use parking_lot::Mutex;

use crate::{Fat32Error, FatDirectory, FileHandle, FileState, Files};

use super::io::Drive;
use super::{boot::BPB, Fat32Result};

pub struct Driver {
    pub(crate) drive: Drive,
    pub(crate) bpb: BPB,
    file_state: Mutex<FileState>,
}

impl Driver {
    pub fn new(drive: Drive) -> Fat32Result<Self> {
        let bpb = BPB::read_from(&drive)?;

        println!("{:#?}", bpb);

        Ok(Self {
            drive,
            bpb,
            file_state: Mutex::new(FileState::new()),
        })
    }
    pub fn bytes_per_cluster(&self) -> usize {
        self.bpb.bytes_per_cluster()
    }
    pub(crate) fn read_sector(&self, n: usize, byte_offset: usize, buffer: &mut [u8]) -> Fat32Result<()>{
        let sector_byte_offset = self.bpb.bytes_per_sector() * n;
        let offset = sector_byte_offset + byte_offset;
    
        self.drive.read(buffer, offset as i64)?;
    
        Ok(())
    }
    #[allow(unused)]
    pub(crate) fn read_cluster(&self, n: usize, byte_offset: usize, buffer: &mut [u8]) -> Fat32Result<()> {
        let start_sector = self.bpb.cluster_start_sector(n);
        self.read_sector( start_sector, byte_offset, buffer)?;
    
        Ok(())
    }
    /// Returns the next cluster number according to the FAT table
    pub(crate) fn read_fat(&self, cluster_num: usize) -> Fat32Result<Option<usize>> {
        let bpb = &self.bpb;

        let sector = bpb.bpb_rsvd_sec_cnt as usize + (cluster_num * 4 / bpb.bytes_per_sector());
        let entry_offset = (cluster_num * 4) % bpb.bytes_per_sector();

        let mut bytes = [0; 4];

        self.read_sector(sector, entry_offset, &mut bytes)?;

        let cluster_val = u32::from_le_bytes(bytes) & 0x0FFFFFFF;

        let cluster_num = if fat_is_eoc(cluster_val) {
            None
        } else {
            Some(cluster_val as usize)
        };

        Ok(cluster_num)
    }
    pub fn files(&self, directory: &FatDirectory) -> Files {
        Files::new(self, &directory)
    }
    pub fn search(&self, directory: &FatDirectory, name: &OsStr) -> Fat32Result<FatDirectory> {
        let mut files = self.files(directory);
        while let Some(file) = files.next()? {
            if file.name() == name {
                return Ok(file);
            }
        }

        Err(Fat32Error::NotFound)
    }
    pub fn search_by_path(&self, path: &Path) -> Fat32Result<FatDirectory> {
        let mut components = path.components();

        let mut current_directory = FatDirectory::root(self);

        // First component should always be root (/)
        components.next().unwrap();

        //a, b, c
        for component in components {
            match component {
                Component::Normal(name) => {
                    let result = self.search(&current_directory, name);
                    current_directory = result?;
                }
                _=> unreachable!()
            }
        }

        Ok(current_directory)
    }
    pub fn open_dir(&self, path: &Path) -> Fat32Result<FileHandle> {
        let directory = self.search_by_path(path)?;
        let mut file_state = self.file_state.lock();

        let handle = file_state.open_dir(&directory)?;

        Ok(handle)
    }
    pub fn read_dir(&self, handle: FileHandle, offset: usize) -> Fat32Result<Option<FatDirectory>>{
        let mut file_state = self.file_state.lock();

        file_state.read_dir(self, handle, offset)
    }
    pub fn close_dir(&self, handle: FileHandle) -> Fat32Result<()> {
        let mut file_state = self.file_state.lock();
        file_state.close_dir(handle)
    }
    pub fn open(&self, path: &Path) -> Fat32Result<FileHandle> {
        let file = self.search_by_path(path)?;
        let mut file_state = self.file_state.lock();

        let handle = file_state.open(&file)?;

        Ok(handle)
    }
    pub fn close(&self, handle: FileHandle) -> Fat32Result<()> {
        let mut file_state = self.file_state.lock();

        file_state.close(handle)
    }
    pub fn read(&self, handle: FileHandle, buffer: &mut [u8], byte_offset: usize) -> Fat32Result<usize> {
        let mut file_state = self.file_state.lock();

        file_state.read(self, handle, buffer, byte_offset)
    }
}


pub fn fat_is_eoc(value: u32) -> bool {
    value >= 0x0FFFFFF8 && value <= 0x0FFFFFFF
}
pub fn fat_is_free(value: u32) -> bool {
    value == 0
}
pub fn fat_is_bad(value: u32) -> bool {
    value == 0x0FFFFFF7
}
pub fn fat_is_reserved(value: u32) -> bool {
    value == 1
}