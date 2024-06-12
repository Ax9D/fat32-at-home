use std::error::Error;

use fat32::{Directories, DirectoryType, Drive, Driver, Fat32Result};
use std::collections::VecDeque;

fn all_dirs(driver: &Driver) -> Fat32Result<()> {
    let mut queue = VecDeque::new();

    queue.push_back(Directories::new(driver, DirectoryType::Root));

    while !queue.is_empty() {
        let n = queue.len();

        for _ in 0..n {
            let mut current = queue.pop_front().unwrap();

            while let Some(directory) = current.next()? {
                let special_dir = directory.is_current_dir() || directory.is_parent_dir();
                if !special_dir {
                    println!("{:?}", directory.short_name());
                }

                if directory.is_dir() && !special_dir {
                    queue.push_back(Directories::new(driver, DirectoryType::SubDirectory(directory)));
                }
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();

    let drive_path = &args[1];

    let file = std::fs::OpenOptions::new().read(true).open(drive_path)?;

    let drive = Drive::from_file(file)?;

    let driver = Driver::new(drive)?;


    all_dirs(&driver)?;

    Ok(())
}
