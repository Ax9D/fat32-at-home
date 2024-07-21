use std::collections::HashMap;

use crate::{Driver, Fat32Error, Fat32Result, FatDirectory};

pub type FileHandle = u64;
pub struct File {
    directory: FatDirectory,
}

impl File {
    pub fn new(directory: FatDirectory) -> Fat32Result<Self> {
        assert!(directory.is_file());
        Ok(
        Self {
            directory,
        }
        )
    } 
    pub fn read(&self, driver: &Driver, byte_offset: usize, buffer: &mut [u8]) -> Fat32Result<usize> {
        let file_size = self.directory.file_size();

        let mut read_start_offset = byte_offset;
        if byte_offset >= file_size {
            read_start_offset = file_size - 1;
        }

        let mut read_len = buffer.len();

        if read_start_offset + read_len > file_size {
            read_len = file_size - read_start_offset;
        }

        let cluster_byte_size = driver.bpb.bytes_per_cluster();
        let mut n_skip_clusters = read_start_offset / cluster_byte_size;
        let mut cluster_relative_byte_offset = read_start_offset % cluster_byte_size;

        let starting_cluster = self.directory.cluster_num();
    
        let mut cluster = starting_cluster;

        while n_skip_clusters != 0 {
            let Some(next_cluster) = driver.read_fat(cluster)? else {
                return Err(Fat32Error::FileCorrupt)
            };

            cluster = next_cluster;
            n_skip_clusters-= 1;
        }

        let mut to_read = read_len;
        let mut buffer_ptr = 0;

        loop {
            let reading_in_current_cluster = usize::min(to_read, cluster_byte_size);
            let sub_buffer = &mut buffer[buffer_ptr..buffer_ptr + reading_in_current_cluster];
            
            // println!("reading {} bytes into sub buffer of len {}", reading_in_current_cluster, sub_buffer.len());

            driver.read_cluster(cluster, cluster_relative_byte_offset, sub_buffer)?;
            cluster_relative_byte_offset = 0;
            buffer_ptr += reading_in_current_cluster;
            to_read -= reading_in_current_cluster;

            if let Some(next_cluster) = driver.read_fat(cluster)? {
                cluster = next_cluster;

            } else {
                assert!(to_read == 0);
                break;
            }
        }

        Ok(read_len as usize)
    }
}


struct DirectoryState {
    directory: FatDirectory,
    files_cached: Vec<FatDirectory>
}
impl DirectoryState {
    pub fn new(directory: FatDirectory) -> Self {
        Self {
            directory,
            files_cached: vec![]
        }
    }
}
pub struct FileState {
    files: HashMap<FileHandle, File>,
    dirs: HashMap<FileHandle, DirectoryState>,
    next_file_handle: FileHandle,
    free_list: Vec<FileHandle>,
}

impl FileState {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            dirs: HashMap::new(),
            next_file_handle: 0,
            free_list: Vec::new(),
        }
    }
    fn alloc_handle(&mut self) -> FileHandle {
        if let Some(existing) = self.free_list.pop() {
            return existing;
        }

        self.next_file_handle += 1;

        self.next_file_handle
    }
    fn dealloc_handle(&mut self, handle: FileHandle) {
        self.free_list.push(handle);
    }

    pub fn open_dir(&mut self, directory: &FatDirectory) -> Fat32Result<FileHandle> {
        if !directory.is_dir() {
            return Err(Fat32Error::NotADir);
        }

        let handle = self.alloc_handle();
        
        self.dirs.insert(handle, DirectoryState::new(directory.clone()));

        Ok(handle)
    }
    fn get_dir_state(&mut self, handle: FileHandle) -> Fat32Result<&mut DirectoryState> {
        self.dirs.get_mut(&handle).ok_or(Fat32Error::InvalidFileHandle(handle))
    }
    pub fn read_dir(&mut self, driver: &Driver, handle: FileHandle, offset: usize) -> Fat32Result<Option<FatDirectory>> {
        let dir_state = self.get_dir_state(handle)?;
        
        if dir_state.files_cached.is_empty() {
            let mut files = driver.files(&dir_state.directory);
            while let Some(file) = files.next()? {
                dir_state.files_cached.push(file);
            }
        }

        Ok(dir_state.files_cached.get(offset).cloned())
    }
    pub fn close_dir(&mut self, handle: FileHandle) -> Fat32Result<()> {
        if self.dirs.remove(&handle).is_none() {
            return Err(Fat32Error::InvalidFileHandle(handle));
        }

        self.dealloc_handle(handle);

        Ok(())
    }
    pub fn open(&mut self, file: &FatDirectory) -> Fat32Result<FileHandle> {
        if file.is_dir() {
            return Err(Fat32Error::IsDir);
        }

        let handle = self.alloc_handle();

        let opened_file = File::new(file.clone())?;
        self.files.insert(handle, opened_file);

        Ok(handle)
    }
    pub fn close(&mut self, handle: FileHandle) -> Fat32Result<()> {
        if self.files.remove(&handle).is_none() {
            return Err(Fat32Error::InvalidFileHandle(handle));
        }

        self.dealloc_handle(handle);

        Ok(())
    }
    pub fn read(&self, driver: &Driver, handle: FileHandle, buffer: &mut [u8], byte_offset: usize) -> Fat32Result<usize> {
        let file = self.files.get(&handle).ok_or(Fat32Error::InvalidFileHandle(handle))?;
        file.read(driver, byte_offset, buffer)
    }
}