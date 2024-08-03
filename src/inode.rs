use std::{collections::HashMap, ffi::OsStr, path::{Path, PathBuf}};

pub const ROOT_INODE: u64 = 1;
pub struct InodeResolver {
    path_to_inode: HashMap<PathBuf, u64>,
    inode_to_path: HashMap<u64, PathBuf>,
    parent_inode: HashMap<u64, u64>,
    next_inode: u64,
}

impl InodeResolver {
    pub fn new() -> Self {
        let parent_inode =HashMap::new();
        
        let mut path_to_inode = HashMap::new();
        let mut inode_to_path = HashMap::new();

        path_to_inode.insert(Path::new("/").to_owned(), ROOT_INODE);
        inode_to_path.insert(ROOT_INODE, Path::new("/").to_owned());
        
        let next_inode = ROOT_INODE;
        Self {
            path_to_inode,
            parent_inode,
            inode_to_path,
            next_inode
        }
    }
    fn register_path_inode(&mut self, path: PathBuf, inode: u64) {
        self.inode_to_path.insert(inode, path.clone());
        self.path_to_inode.insert(path, inode);
    }
    pub fn path(&self, inode: u64) -> &Path {
        &self.inode_to_path[&inode]
    }
    pub fn get_or_assign_inode(&mut self, parent: u64, name: &OsStr) -> u64 {
        if name == "." {
            parent
        } else if name == ".." {
            let parent_inode = self.parent_inode[&parent];
            parent_inode
        }  else {
            let parent_path = self.inode_to_path.get(&parent).expect("Parent path lookup must have happened before child");
            let current_path = parent_path.join(name);

            if let Some(existing_inode) = self.path_to_inode.get(&current_path) {
                return *existing_inode;
            }

            self.next_inode += 1;
            let inode = self.next_inode;
            self.register_path_inode(current_path, inode);
            self.parent_inode.insert(inode, parent);

            inode
        }
    }
}