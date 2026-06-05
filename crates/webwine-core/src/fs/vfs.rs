use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VmError};
use crate::fs::driver::StorageDriver;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualFileSystem {
    root: IndexMap<char, VfsDirectory>,
    // Drive letters bound to a host storage driver. Every op on such a drive is
    // delegated to its driver instead of the in-memory `root` tree. Runtime-only:
    // not serialized (the host re-registers drivers after import).
    #[serde(skip)]
    drivers: HashMap<char, Box<dyn StorageDriver>>,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        let mut fs = VirtualFileSystem {
            root: IndexMap::new(),
            drivers: HashMap::new(),
        };
        fs.bootstrap();
        fs
    }

    /// Bind a drive letter to a host storage driver (e.g. a real-disk
    /// passthrough in the CLI, or an IndexedDB/OPFS store in the browser). After
    /// this, all file operations on that drive route to the driver.
    pub fn register_driver(&mut self, drive: char, driver: Box<dyn StorageDriver>) {
        self.drivers.insert(drive.to_ascii_uppercase(), driver);
    }

    /// True if `guest_path`'s drive is backed by a host storage driver.
    fn driver_drive(guest_path: &str) -> Option<char> {
        GuestPath::parse(guest_path).ok().map(|p| p.drive.to_ascii_uppercase())
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
        Self::ensure_dir_chain(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Start Menu",
                "Programs",
            ],
        );
        Self::ensure_dir_chain(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Places",
            ],
        );
        Self::ensure_dir_chain(&mut c, &["Windows", "System32"]);
        Self::ensure_file(
            &mut c,
            &["Windows", "System32", "explorer.exe"],
            b"special:explorer".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &["Windows", "System32", "uploadfile.exe"],
            b"special:upload-file".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &["Windows", "System32", "uploadfolder.exe"],
            b"special:upload-folder".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Start Menu",
                "Programs",
                "Your PC.lnk",
            ],
            b"action:this-pc".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Start Menu",
                "Programs",
                "File Explorer.lnk",
            ],
            b"C:\\Windows\\System32\\explorer.exe".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Start Menu",
                "Programs",
                "Upload File.lnk",
            ],
            b"C:\\Windows\\System32\\uploadfile.exe".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Start Menu",
                "Programs",
                "Upload Folder.lnk",
            ],
            b"C:\\Windows\\System32\\uploadfolder.exe".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Places",
                "Your PC.lnk",
            ],
            b"action:this-pc".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Places",
                "Documents.lnk",
            ],
            b"C:\\Users\\guest\\Documents".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Places",
                "Pictures.lnk",
            ],
            b"C:\\Users\\guest\\Pictures".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Places",
                "Music.lnk",
            ],
            b"C:\\Users\\guest\\Music".to_vec(),
        );
        Self::ensure_file(
            &mut c,
            &[
                "Users",
                "guest",
                "AppData",
                "Roaming",
                "Microsoft",
                "Windows",
                "Places",
                "Videos.lnk",
            ],
            b"C:\\Users\\guest\\Videos".to_vec(),
        );
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
            child.name = parts[0].to_string();
            Self::ensure_dir_chain(child, &parts[1..]);
        }
    }

    fn ensure_file(dir: &mut VfsDirectory, parts: &[&str], bytes: Vec<u8>) {
        if parts.is_empty() {
            return;
        }
        if parts.len() == 1 {
            let key = parts[0].to_ascii_uppercase();
            dir.children.insert(
                key,
                VfsNode::File(VfsFile {
                    name: parts[0].to_string(),
                    bytes,
                }),
            );
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
            child.name = parts[0].to_string();
            Self::ensure_file(child, &parts[1..], bytes);
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

    fn resolve_dir<'a>(dir: &'a VfsDirectory, components: &[String]) -> Result<&'a VfsDirectory> {
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
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get_mut(&dr)) {
            return d.write(guest_path, &bytes);
        }
        let path = GuestPath::parse(guest_path)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no filename".into()))?
            .to_string();

        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();

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
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get_mut(&dr)) {
            return d.create_dir(guest_path);
        }
        let path = GuestPath::parse(guest_path)?;
        let dir_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no name".into()))?
            .to_string();

        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();

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
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get(&dr)) {
            return d.list(guest_path);
        }
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
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get(&dr)) {
            return d.read(guest_path);
        }
        self.read_file_internal(guest_path, 0)
    }

    fn read_file_internal(&self, guest_path: &str, depth: usize) -> Result<Vec<u8>> {
        if depth > 10 {
            return Err(VmError::Path("symlink loop detected".into()));
        }

        let path = GuestPath::parse(guest_path)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no filename".into()))?;

        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();

        let drive = self.get_drive(path.drive)?;
        let dir = Self::resolve_dir(drive, &parent_components)?;

        let key = file_name.to_ascii_uppercase();
        match dir.children.get(&key) {
            Some(VfsNode::File(f)) if file_name.to_lowercase().ends_with(".lnk") => {
                if let Some(target) = Self::shortcut_target(&f.bytes) {
                    if target.to_lowercase().starts_with("action:") {
                        return Ok(f.bytes.clone());
                    }
                    if self.node_exists(&target) {
                        return self.read_file_internal(&target, depth + 1);
                    }
                }
                Ok(f.bytes.clone())
            }
            Some(VfsNode::File(f)) => Ok(f.bytes.clone()),
            Some(VfsNode::Directory(_)) => Err(VmError::NotAFile(guest_path.to_string())),
            None => Err(VmError::NotFound(guest_path.to_string())),
        }
    }

    /// Borrow a file's raw bytes without cloning. Used by range reads so large
    /// files (e.g. a 4 MB game wad) aren't copied in full on every `fread`.
    fn file_bytes(&self, guest_path: &str) -> Result<&[u8]> {
        let path = GuestPath::parse(guest_path)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no filename".into()))?;
        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();
        let drive = self.get_drive(path.drive)?;
        let dir = Self::resolve_dir(drive, &parent_components)?;
        match dir.children.get(&file_name.to_ascii_uppercase()) {
            Some(VfsNode::File(f)) => Ok(&f.bytes),
            Some(VfsNode::Directory(_)) => Err(VmError::NotAFile(guest_path.to_string())),
            None => Err(VmError::NotFound(guest_path.to_string())),
        }
    }

    /// Length of a file in bytes.
    pub fn file_len(&self, guest_path: &str) -> Result<usize> {
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get(&dr)) {
            return d.len(guest_path);
        }
        Ok(self.file_bytes(guest_path)?.len())
    }

    /// Read at most `len` bytes starting at `offset`, cloning only that range.
    pub fn read_range(&self, guest_path: &str, offset: usize, len: usize) -> Result<Vec<u8>> {
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get(&dr)) {
            return d.read_range(guest_path, offset, len);
        }
        let bytes = self.file_bytes(guest_path)?;
        let start = offset.min(bytes.len());
        let end = (start + len).min(bytes.len());
        Ok(bytes[start..end].to_vec())
    }

    pub fn read_raw_file(&self, guest_path: &str) -> Result<Vec<u8>> {
        let path = GuestPath::parse(guest_path)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no filename".into()))?;

        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();

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
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get_mut(&dr)) {
            return d.delete(guest_path);
        }
        let path = GuestPath::parse(guest_path)?;
        let node_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no name".into()))?
            .to_string();

        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();

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
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get_mut(&dr)) {
            return d.rename(guest_path, new_name);
        }

        let path = GuestPath::parse(guest_path)?;
        let old_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no name".into()))?
            .to_string();

        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();

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
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get(&dr)) {
            return d.exists(guest_path);
        }
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

    /// Parses a custom text-based `.lnk` format used internally by WebWINE.
    /// This only reads the first line of the file and does NOT parse real Windows Shell Link binaries.
    fn shortcut_target(bytes: &[u8]) -> Option<String> {
        let text = String::from_utf8_lossy(bytes);
        let raw = text.lines().next()?.trim();
        if raw.is_empty() {
            None
        } else {
            Some(raw.to_string())
        }
    }

    pub fn export_snapshot(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| VmError::Internal(format!("VFS serialize failed: {e}")))
    }

    pub fn import_snapshot(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(|e| VmError::Internal(format!("VFS deserialize failed: {e}")))
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
            let entries = fs
                .list_dir(&format!("C:\\Users\\guest\\{folder}\\"))
                .unwrap();
            assert!(entries.is_empty());
        }
        let start_menu = fs
            .list_dir(
                "C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\",
            )
            .unwrap();
        assert!(start_menu.iter().any(|e| e.name == "File Explorer.lnk"));
        let places = fs
            .list_dir("C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Places\\")
            .unwrap();
        assert!(places.iter().any(|e| e.name == "Your PC.lnk"));
        let system32 = fs.list_dir("C:\\Windows\\System32\\").unwrap();
        assert!(system32.iter().any(|e| e.name == "explorer.exe"));
    }

    #[test]
    fn lnk_reads_follow_target_but_raw_is_preserved() {
        let fs = VirtualFileSystem::new();
        let resolved = fs
            .read_file("C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\File Explorer.lnk")
            .unwrap();
        assert_eq!(resolved, b"special:explorer");

        let raw = fs
            .read_raw_file("C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\File Explorer.lnk")
            .unwrap();
        assert_eq!(raw, b"C:\\Windows\\System32\\explorer.exe");
    }

    #[test]
    fn mount_and_read_file() {
        let mut fs = VirtualFileSystem::new();
        fs.mount_file("C:\\Users\\guest\\Desktop\\hello.txt", b"hi".to_vec())
            .unwrap();
        let bytes = fs
            .read_file("C:\\Users\\guest\\Desktop\\hello.txt")
            .unwrap();
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
