use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VmError};
use crate::fs::path::GuestPath;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsDirectory {
    pub name: String,
    pub children: IndexMap<String, VfsNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VfsNode {
    File(VfsFile),
    Directory(VfsDirectory),
}

impl VfsNode {
    pub fn name(&self) -> &str {
        match self {
            VfsNode::File(f) => &f.name,
            VfsNode::Directory(d) => &d.name,
        }
    }

    pub fn is_file(&self) -> bool {
        matches!(self, VfsNode::File(_))
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, VfsNode::Directory(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub kind: EntryKind,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Directory,
}

pub struct VirtualFileSystem {
    root: IndexMap<char, VfsDirectory>,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        let mut fs = VirtualFileSystem {
            root: IndexMap::new(),
        };
        fs.bootstrap();
        fs
    }

    fn bootstrap(&mut self) {
        let mut c = VfsDirectory {
            name: "C:".to_string(),
            children: IndexMap::new(),
        };
        Self::ensure_dir_chain(&mut c, &["Users", "guest", "Desktop"]);
        Self::ensure_dir_chain(&mut c, &["Users", "guest", "Documents"]);
        Self::ensure_dir_chain(&mut c, &["Users", "guest", "Pictures"]);
        Self::ensure_dir_chain(&mut c, &["Users", "guest", "Music"]);
        Self::ensure_dir_chain(&mut c, &["Users", "guest", "Videos"]);
        Self::ensure_dir_chain(&mut c, &["Windows", "System32"]);
        Self::ensure_dir_chain(&mut c, &["Temp"]);
        self.root.insert('C', c);
    }

    fn ensure_dir_chain(dir: &mut VfsDirectory, parts: &[&str]) {
        if parts.is_empty() {
            return;
        }
        let key = parts[0].to_ascii_uppercase();
        let entry = dir.children.entry(key).or_insert_with(|| {
            VfsNode::Directory(VfsDirectory {
                name: parts[0].to_string(),
                children: IndexMap::new(),
            })
        });
        if let VfsNode::Directory(child) = entry {
            Self::ensure_dir_chain(child, &parts[1..]);
        }
    }

    fn get_drive_mut(&mut self, drive: char) -> Result<&mut VfsDirectory> {
        self.root
            .get_mut(&drive)
            .ok_or_else(|| VmError::NotFound(format!("drive {drive}:")))
    }

    fn get_drive(&self, drive: char) -> Result<&VfsDirectory> {
        self.root
            .get(&drive)
            .ok_or_else(|| VmError::NotFound(format!("drive {drive}:")))
    }

    fn resolve_dir_mut<'a>(
        dir: &'a mut VfsDirectory,
        components: &[String],
    ) -> Result<&'a mut VfsDirectory> {
        if components.is_empty() {
            return Ok(dir);
        }
        let key = components[0].to_ascii_uppercase();
        match dir.children.get_mut(&key) {
            Some(VfsNode::Directory(child)) => Self::resolve_dir_mut(child, &components[1..]),
            Some(VfsNode::File(_)) => Err(VmError::NotADirectory(components[0].clone())),
            None => Err(VmError::NotFound(components[0].clone())),
        }
    }

    fn resolve_dir<'a>(
        dir: &'a VfsDirectory,
        components: &[String],
    ) -> Result<&'a VfsDirectory> {
        if components.is_empty() {
            return Ok(dir);
        }
        let key = components[0].to_ascii_uppercase();
        match dir.children.get(&key) {
            Some(VfsNode::Directory(child)) => Self::resolve_dir(child, &components[1..]),
            Some(VfsNode::File(_)) => Err(VmError::NotADirectory(components[0].clone())),
            None => Err(VmError::NotFound(components[0].clone())),
        }
    }

    pub fn mount_file(&mut self, guest_path: &str, bytes: Vec<u8>) -> Result<()> {
        let path = GuestPath::parse(guest_path)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no filename".into()))?
            .to_string();

        let parent_components = path
            .parent()
            .map(|p| p.components)
            .unwrap_or_default();

        let drive = self.get_drive_mut(path.drive)?;
        let dir = Self::resolve_dir_mut(drive, &parent_components)?;

        let key = file_name.to_ascii_uppercase();
        dir.children.insert(
            key,
            VfsNode::File(VfsFile {
                name: file_name,
                bytes,
            }),
        );
        Ok(())
    }

    pub fn create_dir(&mut self, guest_path: &str) -> Result<()> {
        let path = GuestPath::parse(guest_path)?;
        let dir_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no name".into()))?
            .to_string();

        let parent_components = path
            .parent()
            .map(|p| p.components)
            .unwrap_or_default();

        let drive = self.get_drive_mut(path.drive)?;
        let parent = Self::resolve_dir_mut(drive, &parent_components)?;

        let key = dir_name.to_ascii_uppercase();
        if parent.children.contains_key(&key) {
            return Err(VmError::AlreadyExists(guest_path.to_string()));
        }
        parent.children.insert(
            key,
            VfsNode::Directory(VfsDirectory {
                name: dir_name,
                children: IndexMap::new(),
            }),
        );
        Ok(())
    }

    pub fn list_dir(&self, guest_path: &str) -> Result<Vec<DirEntry>> {
        let path = GuestPath::parse(guest_path)?;
        let drive = self.get_drive(path.drive)?;
        let dir = Self::resolve_dir(drive, &path.components)?;

        let entries = dir
            .children
            .values()
            .map(|node| {
                let (kind, size) = match node {
                    VfsNode::File(f) => (EntryKind::File, f.bytes.len() as u64),
                    VfsNode::Directory(_) => (EntryKind::Directory, 0),
                };
                let entry_path = if path.components.is_empty() {
                    format!("{}:\\{}", path.drive, node.name())
                } else {
                    format!(
                        "{}:\\{}\\{}",
                        path.drive,
                        path.components.join("\\"),
                        node.name()
                    )
                };
                DirEntry {
                    name: node.name().to_string(),
                    path: entry_path,
                    kind,
                    size,
                }
            })
            .collect();

        Ok(entries)
    }

    pub fn read_file(&self, guest_path: &str) -> Result<Vec<u8>> {
        let path = GuestPath::parse(guest_path)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no filename".into()))?;

        let parent_components = path
            .parent()
            .map(|p| p.components)
            .unwrap_or_default();

        let drive = self.get_drive(path.drive)?;
        let dir = Self::resolve_dir(drive, &parent_components)?;

        let key = file_name.to_ascii_uppercase();
        match dir.children.get(&key) {
            Some(VfsNode::File(f)) => Ok(f.bytes.clone()),
            Some(VfsNode::Directory(_)) => Err(VmError::NotAFile(guest_path.to_string())),
            None => Err(VmError::NotFound(guest_path.to_string())),
        }
    }

    pub fn delete_node(&mut self, guest_path: &str) -> Result<()> {
        let path = GuestPath::parse(guest_path)?;
        let node_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no name".into()))?
            .to_string();

        let parent_components = path
            .parent()
            .map(|p| p.components)
            .unwrap_or_default();

        let drive = self.get_drive_mut(path.drive)?;
        let parent = Self::resolve_dir_mut(drive, &parent_components)?;

        let key = node_name.to_ascii_uppercase();
        if parent.children.shift_remove(&key).is_none() {
            return Err(VmError::NotFound(guest_path.to_string()));
        }
        Ok(())
    }

    pub fn rename_node(&mut self, guest_path: &str, new_name: &str) -> Result<()> {
        if new_name.is_empty() || new_name.contains('\\') || new_name.contains('/') {
            return Err(VmError::Path(format!("invalid name: '{new_name}'")));
        }

        let path = GuestPath::parse(guest_path)?;
        let old_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no name".into()))?
            .to_string();

        let parent_components = path
            .parent()
            .map(|p| p.components)
            .unwrap_or_default();

        let drive = self.get_drive_mut(path.drive)?;
        let parent = Self::resolve_dir_mut(drive, &parent_components)?;

        let old_key = old_name.to_ascii_uppercase();
        let new_key = new_name.to_ascii_uppercase();

        if parent.children.contains_key(&new_key) {
            return Err(VmError::AlreadyExists(new_name.to_string()));
        }

        let mut node = parent
            .children
            .shift_remove(&old_key)
            .ok_or_else(|| VmError::NotFound(guest_path.to_string()))?;

        // update the stored display name inside the node
        match &mut node {
            VfsNode::File(f) => f.name = new_name.to_string(),
            VfsNode::Directory(d) => d.name = new_name.to_string(),
        }

        parent.children.insert(new_key, node);
        Ok(())
    }

    pub fn node_exists(&self, guest_path: &str) -> bool {
        let Ok(path) = GuestPath::parse(guest_path) else {
            return false;
        };
        let Ok(drive) = self.get_drive(path.drive) else {
            return false;
        };
        if path.components.is_empty() {
            return true;
        }
        let parent: Vec<String> = path.components[..path.components.len() - 1].to_vec();
        let Ok(dir) = Self::resolve_dir(drive, &parent) else {
            return false;
        };
        let key = path.components.last().unwrap().to_ascii_uppercase();
        dir.children.contains_key(&key)
    }
}

impl Default for VirtualFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_creates_guest_profile_folders() {
        let fs = VirtualFileSystem::new();
        for folder in ["Desktop", "Documents", "Pictures", "Music", "Videos"] {
            let entries = fs.list_dir(&format!("C:\\Users\\guest\\{folder}\\")).unwrap();
            assert!(entries.is_empty());
        }
    }

    #[test]
    fn mount_and_read_file() {
        let mut fs = VirtualFileSystem::new();
        fs.mount_file("C:\\Users\\guest\\Desktop\\hello.txt", b"hi".to_vec())
            .unwrap();
        let bytes = fs.read_file("C:\\Users\\guest\\Desktop\\hello.txt").unwrap();
        assert_eq!(bytes, b"hi");
    }

    #[test]
    fn list_dir_shows_mounted_file() {
        let mut fs = VirtualFileSystem::new();
        fs.mount_file("C:\\Users\\guest\\Desktop\\test.exe", vec![0u8; 100])
            .unwrap();
        let entries = fs.list_dir("C:\\Users\\guest\\Desktop").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "test.exe");
        assert_eq!(entries[0].size, 100);
    }
}
