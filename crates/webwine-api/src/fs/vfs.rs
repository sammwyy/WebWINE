use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VmError};
use crate::fs::driver::StorageDriver;
use crate::fs::path::GuestPath;

/// A file's metadata (name + flags), kept separate from its content. Content for
/// the in-memory disk lives in `bytes`; driver-backed disks keep content on the
/// host and fetch it lazily, so callers must go through `content()` rather than
/// reading a field directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsFile {
    pub name: String,
    /// Ghost file: appears in listings and `stat`, but has no content — reading
    /// errors, it is never persisted, and it cannot be updated. Used for the
    /// virtual System32 DLL placeholders so the FS resembles Windows.
    #[serde(default)]
    pub is_virtual: bool,
    // In-memory content. Empty for virtual ghosts. Access via `content()`.
    #[serde(default)]
    bytes: Vec<u8>,
}

impl VfsFile {
    /// A real file holding `bytes` in memory.
    pub fn real(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        VfsFile {
            name: name.into(),
            is_virtual: false,
            bytes,
        }
    }

    /// A virtual ghost: it exists (listings/stat) but has no readable content.
    pub fn ghost(name: impl Into<String>) -> Self {
        VfsFile {
            name: name.into(),
            is_virtual: true,
            bytes: Vec::new(),
        }
    }

    /// Reported size in bytes (0 for ghosts).
    pub fn size(&self) -> u64 {
        if self.is_virtual {
            0
        } else {
            self.bytes.len() as u64
        }
    }

    /// Borrow the content. Errors for a virtual ghost ("exists but no content").
    pub fn content(&self) -> Result<&[u8]> {
        if self.is_virtual {
            Err(VmError::NotFound(format!(
                "{}: virtual file has no content",
                self.name
            )))
        } else {
            Ok(&self.bytes)
        }
    }
}

/// Metadata for a path: the result of a `stat`/`fstat`, returned without reading
/// content (so driver-backed disks needn't fetch the whole file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStat {
    pub size: u64,
    pub is_dir: bool,
    pub is_virtual: bool,
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
    #[serde(default)]
    pub is_virtual: bool,
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

    /// Register a driver under the drive letter it exposes via `drives()`. Each
    /// storage driver owns one unit (C:, D:, …) and manages all of its ops.
    pub fn register_storage_driver(&mut self, driver: Box<dyn StorageDriver>) -> Option<char> {
        let drive = driver.drives().first().copied()?.to_ascii_uppercase();
        self.drivers.insert(drive, driver);
        Some(drive)
    }

    /// Initialise a disk's default Windows layout — the standard profile and
    /// system folders plus the WebWINE shell helper exes — creating only what is
    /// missing (idempotent). Writes go through normal ops, so on a driver-backed
    /// drive the driver persists them (persistence is the driver's job, not the
    /// core's). Virtual ghost files are NOT seeded here; they are core-only and
    /// never persisted. The host decides when to call this (e.g. a persistent
    /// browser disk on first run); a 1:1 debug passthrough should not, to avoid
    /// polluting the real directory.
    pub fn init_disk_defaults(&mut self, drive: char) {
        const DIRS: &[&[&str]] = &[
            &["Users", "guest", "Desktop"],
            &["Users", "guest", "Documents"],
            &["Users", "guest", "Pictures"],
            &["Users", "guest", "Music"],
            &["Users", "guest", "Videos"],
            &["Users", "guest", "AppData", "Roaming"],
            &["Users", "guest", "AppData", "Local", "Temp"],
            &["Windows", "System32"],
            &["Windows", "Temp"],
            &["Temp"],
        ];
        for chain in DIRS {
            let mut acc = format!("{drive}:");
            for seg in *chain {
                acc.push('\\');
                acc.push_str(seg);
                let _ = self.create_dir(&acc); // ignore AlreadyExists
            }
        }
        // Virtual apps (Explorer, Editor, …) are a client concern: the frontend
        // owns the list and materializes each via WebWineVm::register_app. The
        // filesystem layer only lays down the directory skeleton.
    }

    /// True if `guest_path`'s drive is backed by a host storage driver.
    fn driver_drive(guest_path: &str) -> Option<char> {
        GuestPath::parse(guest_path)
            .ok()
            .map(|p| p.drive.to_ascii_uppercase())
    }

    /// Metadata for a path (size / is_dir / is_virtual) without reading content.
    /// Backs Win32 GetFileAttributes / stat / fstat.
    pub fn stat(&self, guest_path: &str) -> Result<FileStat> {
        if let Some(d) = Self::driver_drive(guest_path).and_then(|dr| self.drivers.get(&dr)) {
            return d.stat(guest_path);
        }
        let path = GuestPath::parse(guest_path)?;
        let drive = self.get_drive(path.drive)?;
        if path.components.is_empty() {
            return Ok(FileStat {
                size: 0,
                is_dir: true,
                is_virtual: false,
            });
        }
        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();
        let name = path
            .file_name()
            .ok_or_else(|| VmError::Path("no filename".into()))?;
        let dir = Self::resolve_dir(drive, &parent_components)?;
        match dir.children.get(&name.to_ascii_uppercase()) {
            Some(VfsNode::File(f)) => Ok(FileStat {
                size: f.size(),
                is_dir: false,
                is_virtual: f.is_virtual,
            }),
            Some(VfsNode::Directory(_)) => Ok(FileStat {
                size: 0,
                is_dir: true,
                is_virtual: false,
            }),
            None => Err(VmError::NotFound(guest_path.to_string())),
        }
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

        // Ensure Places shortcuts
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
        // No virtual apps are seeded here: the client registers them via
        // WebWineVm::register_app once the runtime is ready (see app-registry.ts).
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
                VfsNode::File(VfsFile::real(parts[0].to_string(), bytes)),
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

    /// Insert a virtual ghost file (exists in listings/stat, errors on read,
    /// never persisted). No-op if a real file already occupies the path. Always
    /// targets the in-memory tree — ghosts are core-managed, not driver state.
    pub fn mount_virtual_file(&mut self, guest_path: &str) -> Result<()> {
        let path = GuestPath::parse(guest_path)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| VmError::Path("path has no filename".into()))?
            .to_string();
        let parent_components = path.parent().map(|p| p.components).unwrap_or_default();
        let drive = self.get_drive_mut(path.drive)?;
        let dir = Self::resolve_dir_mut(drive, &parent_components)?;
        let key = file_name.to_ascii_uppercase();
        if matches!(dir.children.get(&key), Some(VfsNode::File(f)) if !f.is_virtual) {
            return Ok(()); // don't shadow a real file with a ghost
        }
        dir.children
            .insert(key, VfsNode::File(VfsFile::ghost(file_name)));
        Ok(())
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
        // A virtual ghost cannot be updated or replaced through normal writes.
        if let Some(VfsNode::File(f)) = dir.children.get(&key) {
            if f.is_virtual {
                return Err(VmError::Path(format!(
                    "{file_name}: virtual file is read-only"
                )));
            }
        }
        dir.children
            .insert(key, VfsNode::File(VfsFile::real(file_name, bytes)));
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
                let (kind, size, is_virtual) = match node {
                    VfsNode::File(f) => (EntryKind::File, f.size(), f.is_virtual),
                    VfsNode::Directory(_) => (EntryKind::Directory, 0, false),
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
                    is_virtual,
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
                let bytes = f.content()?;
                if let Some(target) = Self::shortcut_target(bytes) {
                    if target.to_lowercase().starts_with("action:") {
                        return Ok(bytes.to_vec());
                    }
                    if self.node_exists(&target) {
                        return self.read_file_internal(&target, depth + 1);
                    }
                }
                Ok(bytes.to_vec())
            }
            Some(VfsNode::File(f)) => Ok(f.content()?.to_vec()),
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
            Some(VfsNode::File(f)) => f.content(),
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
            Some(VfsNode::File(f)) => f.content().map(|b| b.to_vec()),
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
    fn virtual_files_exist_but_error_on_read_and_are_not_persisted() {
        let mut fs = VirtualFileSystem::new();
        let p = "C:\\Windows\\System32\\kernel32.dll";
        fs.mount_virtual_file(p).unwrap();

        // Exists in listings and stat, with is_virtual + zero size.
        assert!(fs.node_exists(p));
        let st = fs.stat(p).unwrap();
        assert!(st.is_virtual && st.size == 0);
        assert!(fs
            .list_dir("C:\\Windows\\System32\\")
            .unwrap()
            .iter()
            .any(|e| e.name == "kernel32.dll" && e.is_virtual));

        // Accessing content errors ("exists but no content").
        assert!(fs.read_file(p).is_err());
        assert!(fs.read_range(p, 0, 16).is_err());

        // A virtual ghost cannot be overwritten by a normal write.
        assert!(fs.mount_file(p, b"real".to_vec()).is_err());
    }

    #[test]
    fn bootstrap_creates_guest_profile_folders() {
        let fs = VirtualFileSystem::new();
        for folder in ["Desktop", "Documents", "Pictures", "Music", "Videos"] {
            let entries = fs
                .list_dir(&format!("C:\\Users\\guest\\{folder}\\"))
                .unwrap();
            assert!(entries.is_empty());
        }
        // The Start Menu Programs folder exists but is empty: virtual apps are
        // registered by the client (WebWineVm::register_app), not seeded here.
        let start_menu = fs
            .list_dir(
                "C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\",
            )
            .unwrap();
        assert!(start_menu.is_empty());
        let places = fs
            .list_dir("C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Places\\")
            .unwrap();
        assert!(places.iter().any(|e| e.name == "Your PC.lnk"));
    }

    #[test]
    fn lnk_reads_follow_target_but_raw_is_preserved() {
        let mut fs = VirtualFileSystem::new();
        // Materialize a shortcut by hand (what WebWineVm::register_app does): the
        // .lnk holds the target exe path; that exe carries a `special:` marker.
        let exe = "C:\\Windows\\System32\\WWExplorer.exe";
        let lnk = "C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\WW Explorer.lnk";
        fs.mount_file(exe, b"special:explorer".to_vec()).unwrap();
        fs.mount_file(lnk, exe.as_bytes().to_vec()).unwrap();

        // read_file follows the .lnk to its target's content...
        assert_eq!(fs.read_file(lnk).unwrap(), b"special:explorer");
        // ...while read_raw_file preserves the .lnk's own bytes (the target path).
        assert_eq!(fs.read_raw_file(lnk).unwrap(), exe.as_bytes());
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
