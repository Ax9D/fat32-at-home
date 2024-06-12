use crate::{Driver, Fat32Result};

use std::fmt::Debug;

// pub struct Sector {
//     buffer: SmallVec<[u8; 512]>,
// }

// impl Sector {
//     pub fn read(driver: &Driver, n: usize) -> Fat32Result<Self> {
//         let offset = driver.bpb.bytes_per_sector() * n;

//         let mut buffer = SmallVec::from([0; 512]);
//         driver.drive.read(&mut buffer, offset as i64)?;
        
//         Ok(Self {
//             buffer
//         })
//     }
// }

// impl<Idx> Index<Idx> for Sector where Idx: SliceIndex<[u8]> {
//     type Output = Idx::Output;

//     fn index(&self, index: Idx) -> &Self::Output {
//         &self.buffer[index]
//     }
// }


pub fn read_sector(driver: &Driver, n: usize, byte_offset: usize, buffer: &mut [u8]) -> Fat32Result<()>{
    let sector_byte_offset = driver.bpb.bytes_per_sector() * n;
    let offset = sector_byte_offset + byte_offset;

    driver.drive.read(buffer, offset as i64)?;

    Ok(())
}
pub fn read_cluster(driver: &Driver, n: usize, byte_offset: usize, buffer: &mut [u8]) -> Fat32Result<()> {
    let start_sector = cluster_start_sector(driver, n);
    read_sector(driver, start_sector, byte_offset, buffer)?;

    Ok(())
}

pub fn cluster_start_sector(driver: &Driver, n: usize) -> usize {
    driver.bpb.data_start_sector() + (n - 2) * driver.bpb.sectors_per_cluster()
}



pub struct Fat(u32);

impl Debug for Fat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Fat")
        .field(&format_args!("0x{0:X}", self.0)).finish()
    }
}

impl Fat {
    pub fn read(driver: &Driver, n: usize) -> Fat32Result<Self> {
        let bpb = &driver.bpb;
        assert!(n <  bpb.fat_sectors());

        let sector = bpb.bpb_rsvd_sec_cnt as usize + (n * 4 / bpb.bytes_per_sector());
        let entry_offset = (n * 4) % bpb.bytes_per_sector();

        let mut bytes = [0; 4];

        read_sector(driver, sector, entry_offset, &mut bytes)?;

        let cluster_val = u32::from_le_bytes(bytes) & 0x0FFFFFFF;
        Ok(Self(cluster_val))
    }
    pub fn is_eoc(&self) -> bool {
        self.0 >= 0x0FFFFFF8 && self.0 <= 0x0FFFFFFF
    }
    pub fn is_free(&self) -> bool {
        self.0 == 0
    }
    pub fn is_bad(&self) -> bool {
        self.0 == 0x0FFFFFF7
    }
    pub fn is_reserved(&self) -> bool {
        self.0 == 1
    }
    pub fn value(&self) -> u32 {
        self.0
    }
    pub fn next(&self, driver: &Driver) -> Option<Fat32Result<Self>> {
        assert!(!(self.is_free() || self.is_reserved()));

        if self.is_eoc() || self.is_bad() {
            return None;
        }

        Some(Self::read(driver, self.0 as usize))
    }
}

// pub struct Cluster {
//     start_sector: usize
// }

// impl Cluster {
//     pub fn new(driver: &Driver, n: usize) -> Self {

//         Self {
//             start_sector
//         }
//     }
//     pub fn read(&self, driver: &Driver, sector: usize, byte_offset: usize, buffer: &mut[u8]) -> Fat32Result<()>{


//         Ok(())
//     }
// }