//! A `StorageDriver` that passes a guest drive straight through to a real host
//! directory, 1:1, with no copying into the VFS. This is the native analogue of
//! a browser-side storage driver: it lets you debug against the actual files on
//! disk. Guest path `C:\a\b\c` maps to `<root>/a/b/c`.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use webwine_core::error::{Result, VmError};
use webwine_core::fs::driver::StorageDriver;
use webwine_core::fs::vfs::{DirEntry, EntryKind};

#[derive(Debug)]
pub struct PassthroughStorageDriver {
    root: PathBuf,
}

impl PassthroughStorageDriver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        PassthroughStorageDriver { root: root.into() }
    }

    /// Map a guest path (`X:\a\b`) to a host path under `root`. The drive letter
    /// and leading separator are stripped; backslashes become the host separator.
    fn host(&self, guest: &str) -> PathBuf {
        // Drop the "X:" drive prefix if present.
        let rel = match guest.find(':') {
            Some(i) => &guest[i + 1..],
            None => guest,
        };
        let rel = rel.trim_start_matches(['\\', '/']);
        let mut p = self.root.clone();
        for seg in rel.split(['\\', '/']).filter(|s| !s.is_empty()) {
            p.push(seg);
        }
        p
    }

    /// Guest directory path that a listed entry name belongs to.
    fn child_guest_path(parent_guest: &str, name: &str) -> String {
        let trimmed = parent_guest.trim_end_matches(['\\', '/']);
        format!("{trimmed}\\{name}")
    }
}

impl StorageDriver for PassthroughStorageDriver {
    fn exists(&self, path: &str) -> bool {
        self.host(path).exists()
    }

    fn is_dir(&self, path: &str) -> bool {
        self.host(path).is_dir()
    }

    fn read(&self, path: &str) -> Result<Vec<u8>> {
        let h = self.host(path);
        fs::read(&h).map_err(|e| VmError::NotFound(format!("{}: {e}", h.display())))
    }

    fn write(&mut self, path: &str, data: &[u8]) -> Result<()> {
        let h = self.host(path);
        if let Some(parent) = h.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&h, data).map_err(|e| VmError::Internal(format!("write {}: {e}", h.display())))
    }

    fn list(&self, path: &str) -> Result<Vec<DirEntry>> {
        let h = self.host(path);
        let mut out = Vec::new();
        let rd = fs::read_dir(&h)
            .map_err(|e| VmError::NotFound(format!("{}: {e}", h.display())))?;
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let md = entry.metadata().ok();
            let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            out.push(DirEntry {
                name: name.clone(),
                path: Self::child_guest_path(path, &name),
                kind: if is_dir { EntryKind::Directory } else { EntryKind::File },
                size: md.map(|m| m.len()).unwrap_or(0),
            });
        }
        Ok(out)
    }

    fn create_dir(&mut self, path: &str) -> Result<()> {
        let h = self.host(path);
        fs::create_dir_all(&h).map_err(|e| VmError::Internal(format!("mkdir {}: {e}", h.display())))
    }

    fn delete(&mut self, path: &str) -> Result<()> {
        let h = self.host(path);
        let r = if h.is_dir() { fs::remove_dir_all(&h) } else { fs::remove_file(&h) };
        r.map_err(|e| VmError::Internal(format!("delete {}: {e}", h.display())))
    }

    fn rename(&mut self, path: &str, new_name: &str) -> Result<()> {
        let h = self.host(path);
        let dst = h.parent().unwrap_or(Path::new("")).join(new_name);
        fs::rename(&h, &dst).map_err(|e| VmError::Internal(format!("rename {}: {e}", h.display())))
    }

    fn len(&self, path: &str) -> Result<usize> {
        let h = self.host(path);
        fs::metadata(&h)
            .map(|m| m.len() as usize)
            .map_err(|e| VmError::NotFound(format!("{}: {e}", h.display())))
    }

    fn read_range(&self, path: &str, offset: usize, len: usize) -> Result<Vec<u8>> {
        let h = self.host(path);
        let mut f = fs::File::open(&h)
            .map_err(|e| VmError::NotFound(format!("{}: {e}", h.display())))?;
        f.seek(SeekFrom::Start(offset as u64))
            .map_err(|e| VmError::Internal(format!("seek {}: {e}", h.display())))?;
        let mut buf = vec![0u8; len];
        let n = f.read(&mut buf).map_err(|e| VmError::Internal(format!("read {}: {e}", h.display())))?;
        buf.truncate(n);
        Ok(buf)
    }
}
