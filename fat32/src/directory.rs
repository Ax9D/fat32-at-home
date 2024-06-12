use std::{ffi::OsStr, io::{Cursor, Read}};

use crate::{cluster_start_sector, read_sector, util::read_bytes, Driver, Fat32Error, Fat32Result};

pub const DIR_ATTR_READ_ONLY: u8 = 0x01;
pub const DIR_ATTR_HIDDEN: u8 = 0x02;
pub const DIR_ATTR_SYSTEM: u8 = 0x04;
pub const DIR_ATTR_VOLUME_ID: u8 = 0x08;
pub const DIR_ATTR_DIRECTORY: u8 = 0x10;
pub const DIR_ATTR_ARCHIVE: u8 = 0x20;
pub const DIR_ATTR_LONG_FILE_NAME: u8 = 0x0F;

pub const FAT32_DIR_SIZE: usize = 32;

pub struct Directory {
    /// DIR_Name
    name: [u8; 11],
    /// DIR_Attr
    attr: u8,
    /// DIR_NTRes
    nt_res: u8,
    /// DIR_CrtTimeTenth
    crt_time_tenth: u8,
    /// DIR_CrtTime
    crt_time: u16,
    /// DIR_CrtDate
    crt_date: u16,
    /// DIR_LstAccDate
    lst_acc_date: u16,
    /// DIR_FstClusHI
    fst_clus_hi: u16,
    /// DIR_WrtTime
    wrt_time: u16,
    /// DIR_WrtDate
    wrt_date: u16,
    /// DIR_FstClusLO
    fst_clus_lo: u16,
    /// DIR_FileSize
    file_size: u32 
}

impl Directory {
    pub fn read(buf: &[u8]) -> Fat32Result<Option<Self>> {
        let mut reader = Cursor::new(buf);
        
        let mut name = [0; 11];

        reader.read_exact(&mut name).map_err(|err| Fat32Error::IOError(err))?;

        // Empty dir entry: End of Directory Marker
        if name[0] == 0 {
            return Ok(None);
        } 

        let attr = read_bytes!(u8, reader)?;
        let nt_res = read_bytes!(u8, reader)?;
        let crt_time_tenth = read_bytes!(u8, reader)?;
        let crt_time = read_bytes!(u16, reader)?;
        let crt_date = read_bytes!(u16, reader)?;
        let lst_acc_date = read_bytes!(u16, reader)?;
        let fst_clus_hi = read_bytes!(u16, reader)?;
        let wrt_time = read_bytes!(u16, reader)?;
        let wrt_date = read_bytes!(u16, reader)?;
        let fst_clus_lo = read_bytes!(u16, reader)?;
        let file_size = read_bytes!(u32, reader)?;


        Ok(Some(Self {
            name,
            attr,
            nt_res,
            crt_time_tenth,
            crt_time,
            crt_date,
            lst_acc_date,
            fst_clus_hi,
            wrt_time,
            wrt_date,
            fst_clus_lo,
            file_size,
        }))
    }
    pub fn cluster_num(&self) -> usize {
        (self.fst_clus_lo as usize) | (self.fst_clus_hi as usize) << 16
    }
    pub fn short_name(&self) -> &OsStr {
        assert!(!self.is_deleted());
        &OsStr::new(std::str::from_utf8(&self.name).unwrap())
    }
    pub fn matches_attr(&self, attrs: u8) -> bool {
        self.attr & attrs == attrs
    }
    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }
    pub fn is_dir(&self) -> bool {
        self.matches_attr(DIR_ATTR_DIRECTORY)
    }
    pub fn is_deleted(&self) -> bool {
        self.name[0] == 0xE5
    }
    /// Checks if directory is .
    pub fn is_current_dir(&self) -> bool {
        self.name[0] == b'.' && self.name[1] == b' '
    }
    /// Checks if directory is ..
    pub fn is_parent_dir(&self) -> bool {
        self.name[0] == b'.' && self.name[1] == b'.'
    }
    pub fn file_size(&self) -> usize {
        self.file_size as usize
    }
    pub fn is_lfn_phony(&self) -> bool {
        self.matches_attr(DIR_ATTR_LONG_FILE_NAME)
    }
}

pub struct LFN {
    /// LDIR_Ord
    ord: u8,
    /// LDIR_Name1
    name1: [u8; 10],
    /// LDIR_Attr
    attr: u8,
    /// LDIR_Type
    type_: u8,
    /// LDIR_Chksum
    chksum: u8,
    /// LDIR_Name2
    name2: [u8; 12],
    /// LDIR_FstClusLO
    fst_clus_lo: u16,
    /// LDIR_Name3
    name3: [u8; 4]
}

impl LFN {
    pub fn read(buf: &[u8]) -> Fat32Result<Self> {
        let mut reader = Cursor::new(buf);
        
        let ord = read_bytes!(u8, reader)?;

        let mut name1 = [0; 10];
        reader.read_exact(&mut name1).map_err(|err| Fat32Error::IOError(err))?;

        let attr = read_bytes!(u8, reader)?;
        let type_ = read_bytes!(u8, reader)?;
        let chksum = read_bytes!(u8, reader)?;

        let mut name2 = [0; 12];
        reader.read_exact(&mut name2).map_err(|err| Fat32Error::IOError(err))?;

        let fst_clus_lo = read_bytes!(u16, reader)?;

        let mut name3 = [0; 4];
        reader.read_exact(&mut name3).map_err(|err| Fat32Error::IOError(err))?;

        Ok(Self {
            ord,
            name1,
            attr,
            type_,
            chksum,
            name2,
            fst_clus_lo,
            name3,
        })
    } 
}

pub enum DirectoryType {
    Root,
    SubDirectory(Directory)
}
pub struct Directories<'d> {
    driver: &'d Driver,
    start_sector: usize,
    byte_offset: usize
}

impl<'d> Directories<'d> {
    pub fn new(driver: &'d Driver, directory_type: DirectoryType) -> Self {
        let start_sector = match directory_type {
            DirectoryType::Root => {
                driver.bpb.data_start_sector()
            },
            DirectoryType::SubDirectory(directory) => {
                cluster_start_sector(driver, directory.cluster_num())
            }
        };

        Self {
            driver,
            start_sector,
            byte_offset: 0
        }
    }
    fn fetch_directory(&mut self) -> Fat32Result<Option<Directory>> {
        let mut buf = [0; FAT32_DIR_SIZE];
        read_sector(self.driver, self.start_sector, self.byte_offset, &mut buf)?;
        self.byte_offset += FAT32_DIR_SIZE;

        Directory::read(&buf)
    }
    pub fn next(&mut self) -> Fat32Result<Option<Directory>> {
        loop {
            let directory = self.fetch_directory()?;

            if let Some(directory) =  directory {
                if directory.is_lfn_phony() {
                    continue;
                }
                

                return Ok(Some(directory));
            } else {
                return Ok(None);
            }
        }
    }
}