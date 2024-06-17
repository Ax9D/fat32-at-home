mod filesystem;
mod inode;

use std::error::Error;

use fat32::{Drive, Driver, Fat32Result, FatDirectory, File, Files};
use filesystem::Fat32;
use fuser::MountOption;
use std::collections::VecDeque;

fn all_files(driver: &Driver) -> Fat32Result<()> {
    let mut queue = VecDeque::new();

    queue.push_back(Files::new(driver, &FatDirectory::root(driver)));

    while !queue.is_empty() {
        let n = queue.len();

        for _ in 0..n {
            let mut current = queue.pop_front().unwrap();

            while let Some(directory) = current.next()? {
                let special_dir = directory.is_current_dir() || directory.is_parent_dir();
                if !special_dir {
                    println!("{:?}", directory.short_name());
                }

                if directory.is_file() && directory.short_name().to_str().unwrap().starts_with("DORA") {
                    let mut buffer = vec![0; directory.file_size()];
                    let file = File::new(directory.clone())?;
                    file.read(driver, 0, &mut buffer)?;
                    // println!("{:?}", buffer);
                }

                if directory.is_dir() && !special_dir {
                    queue.push_back(Files::new(driver, &FatDirectory::root(&driver)));
                }
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init().unwrap();

    let args: Vec<_> = std::env::args().collect();

    let drive_path = &args[1];
    let mount_point = &args[2];

    
    let file = std::fs::OpenOptions::new().read(true).open(drive_path)?;
    
    let drive = Drive::from_file(file)?;
    
    let driver = Driver::new(drive)?;
    // all_files(&driver)?;

    // panic!();

    let mount_options = &vec![MountOption::RO, MountOption::AllowOther, MountOption::AutoUnmount];
    let filesystem = Fat32::new(driver, nix::unistd::geteuid().as_raw(), nix::unistd::getegid().as_raw(), mount_options);
    fuser::mount2(filesystem, mount_point, mount_options)?;

    Ok(())
}
