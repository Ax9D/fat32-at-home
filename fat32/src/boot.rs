use std::io::{Cursor, Read};

use crate::Fat32Error;
use crate::util::read_bytes;
use crate::io::Drive;

use super::Fat32Result;
use std::fmt::Debug;

#[derive(Clone)]
pub struct BPB {
    /// BS_jmpBoot
    pub bs_jmp_boot: [u8; 3],
    /// BS_OEMName 
    pub bs_oem_name: [u8; 8],
    /// BPB_BytsPerSec
    pub bpb_bytes_per_sec: u16,
    /// BPB_SecPerClus 
    pub bpb_sec_per_clus: u8,
    /// BPB_RsvdSecCnt
    pub bpb_rsvd_sec_cnt: u16,
    /// BPB_NumFATs
    pub bpb_num_fats: u8,
    /// BPB_RootEntCnt
    pub bpb_root_ent_cnt: u16,
    /// BPB_TotSec16
    pub bpb_tot_sec16: u16,
    /// BPB_Media
    pub bpb_media: u8,
    /// BPB_FATSz16
    pub bpb_fat_sz16: u16,
    /// BPB_SecPerTrk 
    pub bpb_sec_per_trk: u16,
    /// BPB_NumHeads
    pub bpb_num_heads: u16,
    /// BPB_HiddSec 
    pub bpb_hidd_sec: u32,
    /// BPB_TotSec32
    pub bpb_tot_sec32: u32,
    /// BPB_FATSz32
    pub bpb_fat_sz32: u32,
    /// BPB_ExtFlags
    pub bpb_ext_flags: u16,
    /// BPB_FSVer
    pub bpb_fs_ver: u16,
    /// BPB_RootClus
    pub bpb_root_clus: u32,
    /// BPB_FSInfo
    pub bpb_fs_info: u16,
    /// BPB_BkBootSec
    pub bpb_bk_boot_sec: u16,
    /// BPB_Reserved
    pub bpb_reserved: [u8; 12],
    /// BS_DrvNum
    pub bs_drv_num: u8,
    /// BS_Reserved1
    pub bs_reserved1: u8,
    /// BS_BootSig
    pub bs_boot_sig: u8,
    /// BS_VolID
    pub bs_vol_id: u32,
    /// BS_VolLab
    pub bs_vol_lab: [u8; 11],
    /// BS_FilSysType
    pub bs_fil_sys_type: [u8; 8],
    pub bs_boot_code32: Box<[u8; 420]>,
    pub bs_sign: u16,
}   


impl BPB {
    fn validate(&self) -> Fat32Result<()> {
        match (self.bs_jmp_boot[0], self.bs_jmp_boot[2]) {
            (0xEB, 0x90) => {},
            (0xE9, _) => {},
            _=> {
                return Err(Fat32Error::InvalidBPB("Invalid BS_jmpBoot"));
            }
        }

        match self.bpb_bytes_per_sec {
            512 | 1024 | 2048 | 4096 => {},
            _=> {
                return Err(Fat32Error::InvalidBPB("BPB_BytsPerSec can only be 512, 1024, 2048 or 4096"))
            }
        }

        if self.bpb_sec_per_clus == 0 || !self.bpb_sec_per_clus.is_power_of_two() {
            return Err(Fat32Error::InvalidBPB("BPB_SecPerClus can only be 1, 2, 4, 8, 16, 32, 64, and 128"));
        }

        let bytes_per_cluster = self.bpb_bytes_per_sec * self.bpb_sec_per_clus as u16;

        if bytes_per_cluster > 32 * 1024 {
            return Err(Fat32Error::InvalidBPB("No. of bytes per cluster should not exceed 32 * 1024"));
        }

        if ![0xF0, 0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF].contains(&self.bpb_media) {
            return Err(Fat32Error::InvalidBPB("Invalid BPB_Media value"));
        }

        if self.bpb_root_ent_cnt != 0 {
            return Err(Fat32Error::InvalidBPB("BPB_RootEntCnt must be 0 for Fat32"));
        }

        if self.bpb_tot_sec16 != 0 {
            return Err(Fat32Error::InvalidBPB("BPB_TotSec16 must be 0 for Fat32"));
        }

        if self.bpb_fat_sz16 != 0 {
            return Err(Fat32Error::InvalidBPB("BPB_FATSz16 must be 0 for Fat32"));
        }

        if self.bpb_tot_sec32 == 0 {
            return Err(Fat32Error::InvalidBPB("BPB_TotSec32 must be non zero for Fat32"));
        }

        if self.bpb_fs_ver != 0 {
            return Err(Fat32Error::InvalidBPB("BPB_FSVer higher than 0:0"));
        }

        if !self.bpb_reserved.iter().all(|x| *x == 0) {
            return Err(Fat32Error::InvalidBPB("BPB_Reserved shoulb be 0"));
        }

        if &self.bs_fil_sys_type != b"FAT32   " {
            return Err(Fat32Error::InvalidBPB("BS_FilSysType must be \"FAT32   \""));
        }

        if self.bs_sign != 0xAA55 {
            return Err(Fat32Error::InvalidBPB("BS_Sign must be 0xAA55 I think??".into()));
        }

        Ok(())
    }
    pub fn read_from(drive: &Drive) -> Fat32Result<Self> {
        let mut buf = Box::new([0u8; 512]);
        drive.read(&mut *buf, 0)?;

        let mut reader = Cursor::new(&*buf);

        let mut bs_jmp_boot = [0; 3];
        let mut bs_oem_name = [0; 8];
        reader.read_exact(&mut bs_jmp_boot).map_err(|err| Fat32Error::IOError(err))?;
        reader.read_exact(&mut bs_oem_name).map_err(|err| Fat32Error::IOError(err))?;
    
        let bpb_bytes_per_sec = read_bytes!(u16, reader)?;
        let bpb_sec_per_clus = read_bytes!(u8, reader)?;
        let bpb_rsvd_sec_cnt = read_bytes!(u16, reader)?;
        let bpb_num_fats = read_bytes!(u8, reader)?;
        let bpb_root_ent_cnt = read_bytes!(u16, reader)?;
        let bpb_tot_sec16 = read_bytes!(u16, reader)?;
        let bpb_media = read_bytes!(u8, reader)?;
        let bpb_fat_sz_16 = read_bytes!(u16, reader)?;
        let bpb_sec_per_trk = read_bytes!(u16, reader)?;
        let bpb_num_heads = read_bytes!(u16, reader)?;
        let bpb_hidd_sec = read_bytes!(u32, reader)?;
        let bpb_tot_sec_32 = read_bytes!(u32, reader)?;
        let bpb_fat_sz_32 = read_bytes!(u32, reader)?;
        let bpb_ext_flags = read_bytes!(u16, reader)?;
        let bpb_fs_ver = read_bytes!(u16, reader)?;
        let bpb_root_clus = read_bytes!(u32, reader)?;
        let bpb_fs_info = read_bytes!(u16, reader)?;
        let bpb_bk_boot_sec = read_bytes!(u16, reader)?;
        
        let mut bpb_reserved = [0; 12];
        reader.read_exact(&mut bpb_reserved).map_err(|err| Fat32Error::IOError(err))?;

        let bs_drv_num = read_bytes!(u8, reader)?;
        let bs_reserved1 = read_bytes!(u8, reader)?;
        let bs_boot_sig = read_bytes!(u8, reader)?;
        let bs_vol_id = read_bytes!(u32, reader)?;
        
        let mut bs_vol_lab = [0; 11];
        reader.read_exact(&mut bs_vol_lab).map_err(|err| Fat32Error::IOError(err))?;

        let mut bs_fil_sys_type = [0; 8];
        reader.read_exact(&mut bs_fil_sys_type).map_err(|err| Fat32Error::IOError(err))?;
    
        let mut bs_boot_code32= Box::new([0; 420]);

        reader.read(&mut *bs_boot_code32).map_err(|err| Fat32Error::IOError(err))?;

        let bs_sign = read_bytes!(u16, reader)?;

        let bpb = BPB {
            bs_jmp_boot,
            bs_oem_name,
            bpb_bytes_per_sec,
            bpb_sec_per_clus,
            bpb_rsvd_sec_cnt,
            bpb_num_fats,
            bpb_root_ent_cnt,
            bpb_tot_sec16,
            bpb_media,
            bpb_fat_sz16: bpb_fat_sz_16,
            bpb_sec_per_trk,
            bpb_num_heads,
            bpb_hidd_sec,
            bpb_tot_sec32: bpb_tot_sec_32,
            bpb_fat_sz32: bpb_fat_sz_32,
            bpb_ext_flags,
            bpb_fs_ver,
            bpb_root_clus,
            bpb_fs_info,
            bpb_bk_boot_sec,
            bpb_reserved,
            bs_drv_num,
            bs_reserved1,
            bs_boot_sig,
            bs_vol_id,
            bs_vol_lab,
            bs_fil_sys_type,
            bs_boot_code32,
            bs_sign,
        };

        bpb.validate()?;

        Ok(bpb)
    }
    pub fn fat_start_sector(&self) -> usize {
        self.bpb_rsvd_sec_cnt as usize
    } 
    pub fn fat_sectors(&self) -> usize {
        self.bpb_fat_sz32 as usize * self.bpb_num_fats as usize
    }
    pub fn root_dir_start_sector(&self) -> usize {
        self.fat_start_sector() + self.fat_sectors()
    }
    pub fn root_dir_sectors(&self) -> usize {
        (32 * self.bpb_root_ent_cnt as usize + self.bpb_bytes_per_sec as usize - 1) / self.bpb_bytes_per_sec as usize
    }
    pub fn data_start_sector(&self) -> usize {
        self.root_dir_start_sector() + self.root_dir_sectors()
    }
    pub fn data_sectors(&self) -> usize {
        self.bpb_tot_sec32 as usize - self.data_start_sector()
    }
    #[inline]
    pub fn bytes_per_sector(&self) -> usize {
        self.bpb_bytes_per_sec as usize
    }
    #[inline]
    pub fn bytes_per_cluster(&self) -> usize {
        self.bytes_per_sector() * self.sectors_per_cluster()
    }
    #[inline]
    pub fn sectors_per_cluster(&self) -> usize {
        self.bpb_sec_per_clus as usize
    }
    #[inline]
    pub fn cluster_start_sector(&self, n: usize) -> usize {
        self.data_start_sector() + (n - 2) * self.sectors_per_cluster()
    }

}

impl Debug for BPB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct BootCode;

        f.debug_struct("BPB")
        .field("bs_jmp_boot", &self.bs_jmp_boot)
        .field("bs_oem_name", &String::from_utf8_lossy(&self.bs_oem_name))
        .field("bpb_bytes_per_sec", &self.bpb_bytes_per_sec)
        .field("bpb_sec_per_clus", &self.bpb_sec_per_clus)
        .field("bpb_rsvd_sec_cnt", &self.bpb_rsvd_sec_cnt)
        .field("bpb_num_fats", &self.bpb_num_fats)
        .field("bpb_root_ent_cnt", &self.bpb_root_ent_cnt)
        .field("bpb_tot_sec16", &self.bpb_tot_sec16)
        .field("bpb_media", &self.bpb_media)
        .field("bpb_fat_sz16", &self.bpb_fat_sz16)
        .field("bpb_sec_per_trk", &self.bpb_sec_per_trk)
        .field("bpb_num_heads", &self.bpb_num_heads)
        .field("bpb_hidd_sec", &self.bpb_hidd_sec)
        .field("bpb_tot_sec32", &self.bpb_tot_sec32)
        .field("bpb_fat_sz32", &self.bpb_fat_sz32)
        .field("bpb_ext_flags", &self.bpb_ext_flags)
        .field("bpb_fs_ver", &self.bpb_fs_ver)
        .field("bpb_root_clus", &self.bpb_root_clus)
        .field("bpb_fs_info", &self.bpb_fs_info)
        .field("bpb_bk_boot_sec", &self.bpb_bk_boot_sec)
        .field("bpb_reserved", &self.bpb_reserved)
        .field("bs_drv_num", &self.bs_drv_num)
        .field("bs_reserved1", &self.bs_reserved1)
        .field("bs_boot_sig", &self.bs_boot_sig)
        .field("bs_vol_id", &self.bs_vol_id)
        .field("bs_vol_lab", &String::from_utf8_lossy(&self.bs_vol_lab))
        .field("bs_fil_sys_type", &String::from_utf8_lossy(&self.bs_fil_sys_type))
        .field("bs_boot_code32", &BootCode)
        .field("bs_sign", &self.bs_sign)
        .finish()
    }
}