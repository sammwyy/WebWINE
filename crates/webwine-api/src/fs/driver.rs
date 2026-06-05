//! Storage drivers — the seam that lets the host (browser JS, or the native
//! CLI) own a whole disk and service every file operation for it.
//!
//! The in-memory [`VirtualFileSystem`](super::vfs::VirtualFileSystem) is the
//! default backing store. A drive letter can instead be bound to a
//! `StorageDriver` via `VirtualFileSystem::register_driver`, after which every
//! read/write/list/stat on that drive is delegated to the driver. This is how:
//!   - the **browser** can back drive C: with IndexedDB / OPFS (a TS/JS driver),
//!   - the **CLI** can pass a real host directory through 1:1 for debugging
//!     (`PassthroughStorageDriver`, no copy into the VFS),
//!   - future persistent storage plugs in without touching the guest API.
//!
//! Video / shell / sound are the output-side analogues: those already flow out
//! through the `UiEvent` stream (window create/blit, console, message boxes),
//! which is the host-implemented seam for rendering and shell/sound events.

use crate::error::Result;
use crate::fs::vfs::{DirEntry, FileStat};

/// Host-provided backend for one or more mounted disks. Each driver, at
/// registration, declares the drive letters it owns via [`drives`]; the core
/// then routes every file op on those drives to it. Metadata ops (exists /
/// is_dir / list / stat / mkdir / rename / delete) are separate from content
/// (read / read_range), so a driver can answer metadata without fetching bytes.
///
/// Paths are full guest paths (e.g. `C:\\Windows\\System32\\foo.dll`); the
/// driver maps them however it likes (a real directory, an IndexedDB store, …).
pub trait StorageDriver: std::fmt::Debug + Send {
    /// Drive letters this driver exposes (e.g. `vec!['C']`). The core registers
    /// the driver for each. Empty means "register me explicitly".
    fn drives(&self) -> Vec<char> {
        Vec::new()
    }

    fn exists(&self, path: &str) -> bool;
    fn is_dir(&self, path: &str) -> bool;
    fn read(&self, path: &str) -> Result<Vec<u8>>;
    fn write(&mut self, path: &str, data: &[u8]) -> Result<()>;
    fn list(&self, path: &str) -> Result<Vec<DirEntry>>;
    fn create_dir(&mut self, path: &str) -> Result<()>;
    fn delete(&mut self, path: &str) -> Result<()>;
    fn rename(&mut self, path: &str, new_name: &str) -> Result<()>;

    /// Metadata for a path without reading content. Default composes the other
    /// queries; drivers can override with a single host stat call.
    fn stat(&self, path: &str) -> Result<FileStat> {
        if !self.exists(path) {
            return Err(crate::error::VmError::NotFound(path.to_string()));
        }
        let is_dir = self.is_dir(path);
        let size = if is_dir { 0 } else { self.len(path).unwrap_or(0) as u64 };
        Ok(FileStat { size, is_dir, is_virtual: false })
    }

    /// File length. Default reads the whole file; override for efficiency.
    fn len(&self, path: &str) -> Result<usize> {
        Ok(self.read(path)?.len())
    }

    /// Read `len` bytes from `offset`. Default slices a full read; override for
    /// large files (e.g. game archives) to avoid copying the whole file.
    fn read_range(&self, path: &str, offset: usize, len: usize) -> Result<Vec<u8>> {
        let bytes = self.read(path)?;
        let start = offset.min(bytes.len());
        let end = (start + len).min(bytes.len());
        Ok(bytes[start..end].to_vec())
    }
}
