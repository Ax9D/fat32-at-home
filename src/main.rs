use std::error::Error;

mod fat32;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();

    let drive_path = &args[1];

    let mut file = std::fs::OpenOptions::new().read(true).open(drive_path)?;
    fat32::boot::BPB::read_from(&mut file)?;

    Ok(())
}
