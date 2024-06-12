use super::io::Drive;
use super::{boot::BPB, Fat32Result};

pub struct Driver {
    pub(crate) drive: Drive,
    pub(crate) bpb: BPB,
}

impl Driver {
    pub fn new(drive: Drive) -> Fat32Result<Self> {
        let bpb = BPB::read_from(&drive)?;

        println!("{:#?}", bpb);

        Ok(Self {
            drive,
            bpb
        })
    }
}
