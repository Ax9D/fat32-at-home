use std::{ffi::{OsStr, OsString}, io::{Cursor, Read}, num::Wrapping, ops::Add, time::SystemTime};

use chrono::{Local, TimeZone};

use crate::{util::read_bytes, Driver, Fat32Error, Fat32Result};

pub const DIR_ATTR_READ_ONLY: u8 = 0x01;
pub const DIR_ATTR_HIDDEN: u8 = 0x02;
pub const DIR_ATTR_SYSTEM: u8 = 0x04;
pub const DIR_ATTR_VOLUME_ID: u8 = 0x08;
pub const DIR_ATTR_DIRECTORY: u8 = 0x10;
pub const DIR_ATTR_ARCHIVE: u8 = 0x20;
pub const DIR_ATTR_LONG_FILE_NAME: u8 = 0x0F;

pub const FAT32_DIR_SIZE: usize = 32;

#[derive(Clone, Debug)]
pub struct FatEntry {
    /// DIR_Name
    name: [u8; 11],
    /// DIR_Attr
    attr: u8,
    /// DIR_NTRes
    nt_res: u8,

    #[allow(unused)]
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

impl FatEntry {
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
    pub fn root(driver: &Driver) -> Self {
        let cluster = driver.bpb.data_start_sector();

        let fst_clus_hi = (cluster >> 16) as u16;
        let fst_clus_lo = (cluster & 0xFFFF) as u16;

        Self {
            name: [b'/', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            attr: DIR_ATTR_DIRECTORY,
            nt_res: 0,
            crt_time_tenth: 0,
            crt_time: 0, // 00:00:00
            crt_date: 0x0021, // January 1, 1980
            lst_acc_date: 0x0021, // January 1, 1980
            wrt_time: 0, // 00:00:00
            wrt_date: 0x0021, // January 1, 1980
            fst_clus_hi,
            fst_clus_lo,
            file_size: 0,
        }
    }
    pub fn short_name(&self) -> OsString {
        let file_name = &self.name[0..8];
        let extension = &self.name[8..];

        let file_name = std::str::from_utf8(file_name).unwrap().trim();
        let extension = std::str::from_utf8(extension).unwrap().trim();

        let file_name = if self.nt_res >> 3 & 1 == 1 {
           file_name.to_lowercase()
        } else {
            file_name.to_owned()
        };

        let extension = if self.nt_res >> 4 & 1 == 1 {
            extension.to_lowercase()
        } else {
            extension.to_owned()
        };

        let name = if !extension.is_empty() {
            format!("{}.{}", file_name, extension)
        } else {
           file_name.to_owned()
        };

        OsString::from(name)
    }
    pub fn name_checksum(&self) -> u8 {
        let mut sum: Wrapping<u8> = Wrapping(0);
        for i in 0..11 {
            sum = (sum >> 1).add(sum << 7).add(Wrapping(self.name[i]));       
        }

        sum.0
    }
}

#[allow(unused)]
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


        assert!(type_ == 0);
        assert!(fst_clus_lo == 0);
        
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
    pub fn construct_name(&self, buffer: &mut OsString) {
        let mut name = [0; 26];
        name[0..10].copy_from_slice(&self.name1);
        name[10..22].copy_from_slice(&self.name2);
        name[22..].copy_from_slice(&self.name3);

        let mut name_utf16: [u16; 13] = [0; 13];

        for (ix, chunks) in name.chunks(2).enumerate() {
            name_utf16[ix] = u16::from_le_bytes([chunks[0], chunks[1]]);
        }

        let name_string = String::from_utf16(&name_utf16).expect("Invalid UTF-16");
        
        buffer.push(name_string)
    }
}

#[derive(Clone)]
pub struct FatDirectory {
    name: OsString,
    entry: FatEntry
}

impl FatDirectory {
    pub fn new(entry: FatEntry, lfn_parts: &[LFN]) -> Self {
        let sfn_checksum = entry.name_checksum();

        let name = if lfn_parts.is_empty() {
            entry.short_name()
        } else {
            let mut full_name = OsString::new();

            for part in lfn_parts.iter().rev() {
                if part.chksum != sfn_checksum {
                    todo!("Invalid name checksum")
                }
                part.construct_name(&mut full_name);
            }

            full_name
        };

        Self {
            name,
            entry
        }
    }
    pub fn root(driver: &Driver) -> Self {
        Self {
            name: OsString::from("/"),
            entry: FatEntry::root(driver)
        }
    }
    pub fn name(&self) -> &OsStr {
        &self.name
    }
    pub fn cluster_num(&self) -> usize {
        (self.entry.fst_clus_lo as usize) | (self.entry.fst_clus_hi as usize) << 16
    }
    pub fn matches_attr(&self, attrs: u8) -> bool {
        self.entry.attr & attrs == attrs
    }
    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }
    pub fn is_dir(&self) -> bool {
        self.matches_attr(DIR_ATTR_DIRECTORY)
    }
    pub fn is_deleted(&self) -> bool {
        self.entry.name[0] == 0xE5
    }
    /// Checks if directory is .
    pub fn is_current_dir(&self) -> bool {
        self.entry.name[0] == b'.' && self.entry.name[1] == b' '
    }
    /// Checks if directory is ..
    pub fn is_parent_dir(&self) -> bool {
        self.entry.name[0] == b'.' && self.entry.name[1] == b'.'
    }
    pub fn file_size(&self) -> usize {
        self.entry.file_size as usize
    }
    pub fn name_checksum(&self) -> u8 {
        self.entry.name_checksum()
    }
    pub fn n_clusters(&self, driver: &Driver) -> Fat32Result<usize> {
        let mut n_clusters = 0;
        let mut current_cluster = self.cluster_num();
        while let Some(next_cluster) = driver.read_fat(current_cluster)? {
            n_clusters += 1;
            current_cluster = next_cluster;
        }

        Ok(n_clusters)
    }
    fn fat32_get_time(time: u16) -> (u32, u32, u32) {
        let two_second_count = time & 0b11111;
        let seconds = two_second_count * 2;
        let minutes = time >> 5 & 0b111111;
        let hours = time >> 15;

        (hours as u32, minutes as u32, seconds as u32)
    }
    fn fat32_get_date(date: u16) -> (i32, u32, u32) {
        let day = date & 0b11111;
        let month = date >> 5 & 0b1111; 
        let year_offset = date >> 9;
        let year = 1980 + year_offset;
        
        (year as i32, month as u32, day as u32)
    }
    pub fn create_time(&self) -> SystemTime {
        let (hour, minute, second) = Self::fat32_get_time(self.entry.crt_time);
        let (year, month, day) = Self::fat32_get_date(self.entry.crt_date);
        let datetime = Local.with_ymd_and_hms(year, month, day, hour, minute, second).unwrap();
        
        SystemTime::from(datetime)
    }
    pub fn write_time(&self) -> SystemTime {
        let (hour, minute, second) = Self::fat32_get_time(self.entry.wrt_time);
        let (year, month, day) = Self::fat32_get_date(self.entry.wrt_date);
        
        let datetime = Local.with_ymd_and_hms(year, month, day, hour, minute, second).unwrap();
        
        SystemTime::from(datetime)
    }
    pub fn access_time(&self) -> SystemTime {
        let (year, month, day) = Self::fat32_get_date(self.entry.lst_acc_date);

        let datetime = Local.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap();

        SystemTime::from(datetime)
    }
}

pub struct Files<'d> {
    driver: &'d Driver,
    start_sector: usize,
    byte_offset: usize
}

impl<'d> Files<'d> {
    pub fn new(driver: &'d Driver, directory: &FatDirectory) -> Self {
        let start_sector = if directory.cluster_num() == driver.bpb.data_start_sector() {
            directory.cluster_num()
        } else {
            driver.bpb.cluster_start_sector(directory.cluster_num())
        };

        Self {
            driver,
            start_sector,
            byte_offset: 0
        }
    }
    fn fetch_directory(&mut self) -> Fat32Result<Option<FatDirectory>> {
        let mut buf = [0; FAT32_DIR_SIZE];

        let mut lfn_parts = vec![];
        loop {
            self.driver.read_sector(self.start_sector, self.byte_offset, &mut buf)?;
            self.byte_offset += FAT32_DIR_SIZE;
    
            let attrs = buf[11];
    
            fn is_lfn_entry(attrs: u8) -> bool {
                (attrs & DIR_ATTR_LONG_FILE_NAME) == DIR_ATTR_LONG_FILE_NAME
            }

            if is_lfn_entry(attrs) {
                lfn_parts.push(LFN::read(&buf)?);

                continue;
            } else {
                let entry = FatEntry::read(&buf)?;

                let result = match entry {
                    Some(entry) => Ok(Some(FatDirectory::new(entry, &lfn_parts))),
                    None => Ok(None)
                };

                return result;
            };


        }
    }
    pub fn next(&mut self) -> Fat32Result<Option<FatDirectory>> {
        loop {
            let directory = self.fetch_directory()?;

            if let Some(directory) =  directory {
                return Ok(Some(directory));
            } else {
                return Ok(None);
            }
        }
    }
}