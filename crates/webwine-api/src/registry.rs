//! In-memory Windows registry.
//!
//! The guest reaches this through the `Reg*` APIs in advapi32; it is authoritative
//! and **synchronous** (the guest queries it mid-execution and needs an answer
//! immediately). Persistence is therefore done as a snapshot at the host boundary
//! (`export`/`import`) — exactly how a real registry hive is loaded into memory and
//! flushed to disk — rather than a per-operation host driver, which could not be
//! serviced synchronously from inside a Web Worker.
//!
//! Keys are stored in a flat map keyed by the lowercased full path (e.g.
//! `hkey_local_machine\software\...`); the original-case path is preserved for
//! display. Subkey enumeration scans for immediate children of a prefix — fine for
//! the small hives we keep. Value names are case-insensitive; `""` is the key's
//! default value.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Predefined root HKEYs (winreg.h).
pub const HKEY_CLASSES_ROOT: u32 = 0x8000_0000;
pub const HKEY_CURRENT_USER: u32 = 0x8000_0001;
pub const HKEY_LOCAL_MACHINE: u32 = 0x8000_0002;
pub const HKEY_USERS: u32 = 0x8000_0003;
pub const HKEY_CURRENT_CONFIG: u32 = 0x8000_0005;

// REG_* value type ids.
pub const REG_NONE: u32 = 0;
pub const REG_SZ: u32 = 1;
pub const REG_EXPAND_SZ: u32 = 2;
pub const REG_BINARY: u32 = 3;
pub const REG_DWORD: u32 = 4;
pub const REG_MULTI_SZ: u32 = 7;
pub const REG_QWORD: u32 = 11;

/// First handle handed out for an opened key; predefined roots use their HKEY
/// constant directly so they never collide with this range.
const HANDLE_BASE: u32 = 0x4B00_0000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RegValue {
    Sz(String),
    ExpandSz(String),
    Dword(u32),
    Qword(u64),
    Binary(Vec<u8>),
    MultiSz(Vec<String>),
    None,
}

impl RegValue {
    pub fn type_id(&self) -> u32 {
        match self {
            RegValue::None => REG_NONE,
            RegValue::Sz(_) => REG_SZ,
            RegValue::ExpandSz(_) => REG_EXPAND_SZ,
            RegValue::Binary(_) => REG_BINARY,
            RegValue::Dword(_) => REG_DWORD,
            RegValue::MultiSz(_) => REG_MULTI_SZ,
            RegValue::Qword(_) => REG_QWORD,
        }
    }

    /// Encode the value as the raw bytes `RegQueryValueExW` returns to the guest.
    /// Strings are UTF-16LE with a null terminator; MULTI_SZ gets a trailing extra
    /// null; numbers are little-endian.
    pub fn to_bytes(&self) -> Vec<u8> {
        fn wide(s: &str) -> Vec<u8> {
            let mut v: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            v.extend_from_slice(&[0, 0]);
            v
        }
        match self {
            RegValue::Sz(s) | RegValue::ExpandSz(s) => wide(s),
            RegValue::Dword(d) => d.to_le_bytes().to_vec(),
            RegValue::Qword(q) => q.to_le_bytes().to_vec(),
            RegValue::Binary(b) => b.clone(),
            RegValue::MultiSz(list) => {
                let mut v = Vec::new();
                for s in list {
                    v.extend(s.encode_utf16().flat_map(|u| u.to_le_bytes()));
                    v.extend_from_slice(&[0, 0]);
                }
                v.extend_from_slice(&[0, 0]); // final terminator
                v
            }
            RegValue::None => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistryKey {
    /// Original-case full path, for display/enumeration.
    pub path: String,
    /// Value name (original case) -> value. `""` is the default value.
    pub values: BTreeMap<String, RegValue>,
}

/// Serializable snapshot of the whole registry (just the key map; open handles are
/// runtime-only and not persisted).
pub type RegistrySnapshot = BTreeMap<String, RegistryKey>;

#[derive(Debug)]
pub struct Registry {
    /// lowercased full path -> key.
    keys: BTreeMap<String, RegistryKey>,
    /// open HKEY handle -> lowercased path.
    handles: BTreeMap<u32, String>,
    next_handle: u32,
}

fn root_name(hkey: u32) -> Option<&'static str> {
    match hkey {
        HKEY_CLASSES_ROOT => Some("HKEY_CLASSES_ROOT"),
        HKEY_CURRENT_USER => Some("HKEY_CURRENT_USER"),
        HKEY_LOCAL_MACHINE => Some("HKEY_LOCAL_MACHINE"),
        HKEY_USERS => Some("HKEY_USERS"),
        HKEY_CURRENT_CONFIG => Some("HKEY_CURRENT_CONFIG"),
        _ => None,
    }
}

fn norm(path: &str) -> String {
    path.trim_matches('\\').to_string()
}
fn key_of(path: &str) -> String {
    norm(path).to_lowercase()
}

impl Registry {
    pub fn new() -> Self {
        let mut r = Registry {
            keys: BTreeMap::new(),
            handles: BTreeMap::new(),
            next_handle: HANDLE_BASE,
        };
        for hk in [
            HKEY_CLASSES_ROOT,
            HKEY_CURRENT_USER,
            HKEY_LOCAL_MACHINE,
            HKEY_USERS,
            HKEY_CURRENT_CONFIG,
        ] {
            let name = root_name(hk).unwrap();
            r.ensure_key(name);
        }
        r.seed_defaults();
        r
    }

    /// Minimal skeleton many apps expect to exist. Kept tiny on purpose; the host
    /// can layer more via `import`.
    fn seed_defaults(&mut self) {
        self.ensure_key("HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
        self.set_value_path(
            "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
            "ProductName",
            RegValue::Sz("Windows".into()),
        );
        self.ensure_key("HKEY_CURRENT_USER\\SOFTWARE");
        self.ensure_key("HKEY_LOCAL_MACHINE\\SOFTWARE");
        self.ensure_key("HKEY_LOCAL_MACHINE\\SYSTEM");

        // Windows Media Player: present it as already installed/first-run-complete
        // so wmplayer.exe proceeds past its setup gate instead of bailing out.
        self.set_value_path(
            "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\MediaPlayer\\Setup",
            "InstallResult",
            RegValue::Dword(0),
        );
        self.set_value_path(
            "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\MediaPlayer\\Setup",
            "Installation Directory",
            RegValue::Sz("C:\\Program Files\\Windows Media Player".into()),
        );
        self.set_value_path(
            "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\wmplayer.exe",
            "",
            RegValue::Sz("C:\\Program Files\\Windows Media Player\\wmplayer.exe".into()),
        );
        // wmplayer.exe reads "Path" here, expands it and SetCurrentDirectory()s to
        // it so it can load wmp.dll from its install dir. Missing => it bails early.
        self.set_value_path(
            "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\wmplayer.exe",
            "Path",
            RegValue::Sz("C:\\Program Files\\Windows Media Player".into()),
        );
        self.set_value_path(
            "HKEY_CURRENT_USER\\SOFTWARE\\Microsoft\\MediaPlayer\\Preferences",
            "AcceptedPrivacyStatement",
            RegValue::Dword(1),
        );
        self.set_value_path(
            "HKEY_CURRENT_USER\\SOFTWARE\\Microsoft\\MediaPlayer\\Preferences",
            "FirstRun",
            RegValue::Dword(0),
        );
        // Gate that wmplayer.exe checks to decide whether per-user first-logon
        // setup (unregmp2.exe /AsyncFirstLogon) still needs to run. Mark it done.
        self.set_value_path(
            "HKEY_CURRENT_USER\\SOFTWARE\\Microsoft\\MediaPlayer\\Preferences",
            "PlaylistImportComplete",
            RegValue::Dword(1),
        );
        self.ensure_key("HKEY_LOCAL_MACHINE\\SOFTWARE\\Policies\\Microsoft\\WindowsMediaPlayer");
    }

    // ---- path-based access (used by the regedit host bridge and seeding) ----

    /// Create `path` and all its ancestors if missing.
    pub fn ensure_key(&mut self, path: &str) {
        let np = norm(path);
        if np.is_empty() {
            return;
        }
        let mut acc = String::new();
        for seg in np.split('\\') {
            if seg.is_empty() {
                continue;
            }
            if acc.is_empty() {
                acc = seg.to_string();
            } else {
                acc = format!("{acc}\\{seg}");
            }
            let lk = acc.to_lowercase();
            self.keys.entry(lk).or_insert_with(|| RegistryKey {
                path: acc.clone(),
                values: BTreeMap::new(),
            });
        }
    }

    pub fn key_exists(&self, path: &str) -> bool {
        self.keys.contains_key(&key_of(path))
    }

    pub fn delete_key_path(&mut self, path: &str) -> bool {
        let lk = key_of(path);
        let prefix = format!("{lk}\\");
        let victims: Vec<String> = self
            .keys
            .keys()
            .filter(|k| **k == lk || k.starts_with(&prefix))
            .cloned()
            .collect();
        let found = !victims.is_empty();
        for v in victims {
            self.keys.remove(&v);
        }
        found
    }

    /// Immediate child key names (display case) of `path`.
    pub fn subkeys(&self, path: &str) -> Vec<String> {
        let lk = key_of(path);
        let prefix = if lk.is_empty() {
            String::new()
        } else {
            format!("{lk}\\")
        };
        let mut out = Vec::new();
        for (k, v) in &self.keys {
            if !k.starts_with(&prefix) || k == &lk {
                continue;
            }
            let rest = &k[prefix.len()..];
            if rest.is_empty() || rest.contains('\\') {
                continue; // not an immediate child
            }
            // last segment of the display path
            let name = v.path.rsplit('\\').next().unwrap_or(&v.path).to_string();
            out.push(name);
        }
        out
    }

    pub fn values_of(&self, path: &str) -> Option<&BTreeMap<String, RegValue>> {
        self.keys.get(&key_of(path)).map(|k| &k.values)
    }

    pub fn get_value_path(&self, path: &str, name: &str) -> Option<&RegValue> {
        let k = self.keys.get(&key_of(path))?;
        let nl = name.to_lowercase();
        k.values
            .iter()
            .find(|(n, _)| n.to_lowercase() == nl)
            .map(|(_, v)| v)
    }

    pub fn set_value_path(&mut self, path: &str, name: &str, value: RegValue) {
        self.ensure_key(path);
        let k = self.keys.get_mut(&key_of(path)).expect("ensured");
        // Replace an existing name case-insensitively, else insert.
        let nl = name.to_lowercase();
        if let Some(existing) = k.values.keys().find(|n| n.to_lowercase() == nl).cloned() {
            k.values.insert(existing, value);
        } else {
            k.values.insert(name.to_string(), value);
        }
    }

    pub fn delete_value_path(&mut self, path: &str, name: &str) -> bool {
        if let Some(k) = self.keys.get_mut(&key_of(path)) {
            let nl = name.to_lowercase();
            if let Some(existing) = k.values.keys().find(|n| n.to_lowercase() == nl).cloned() {
                k.values.remove(&existing);
                return true;
            }
        }
        false
    }

    // ---- handle-based access (used by the guest Reg* APIs) ----

    /// Resolve an hKey (predefined root or open handle) to its full path.
    pub fn path_of_handle(&self, hkey: u32) -> Option<String> {
        if let Some(n) = root_name(hkey) {
            return Some(n.to_string());
        }
        self.handles.get(&hkey).map(|lk| {
            self.keys
                .get(lk)
                .map(|k| k.path.clone())
                .unwrap_or_else(|| lk.clone())
        })
    }

    fn full_path(&self, hkey: u32, subkey: &str) -> Option<String> {
        let base = self.path_of_handle(hkey)?;
        let sub = norm(subkey);
        Some(if sub.is_empty() {
            base
        } else {
            format!("{base}\\{sub}")
        })
    }

    fn alloc_handle(&mut self, path: &str) -> u32 {
        let h = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        self.handles.insert(h, key_of(path));
        h
    }

    /// RegOpenKeyEx: returns a handle if the key exists, else None.
    pub fn open(&mut self, hkey: u32, subkey: &str) -> Option<u32> {
        let path = self.full_path(hkey, subkey)?;
        if self.key_exists(&path) {
            Some(self.alloc_handle(&path))
        } else {
            None
        }
    }

    /// RegCreateKeyEx: creates the key (and ancestors) then returns a handle.
    pub fn create(&mut self, hkey: u32, subkey: &str) -> Option<u32> {
        let path = self.full_path(hkey, subkey)?;
        self.ensure_key(&path);
        Some(self.alloc_handle(&path))
    }

    pub fn close(&mut self, handle: u32) {
        self.handles.remove(&handle);
    }

    pub fn query(&self, hkey: u32, name: &str) -> Option<&RegValue> {
        let path = self.path_of_handle(hkey)?;
        self.get_value_path(&path, name)
    }

    pub fn set(&mut self, hkey: u32, name: &str, value: RegValue) -> bool {
        let Some(path) = self.path_of_handle(hkey) else {
            return false;
        };
        self.set_value_path(&path, name, value);
        true
    }

    pub fn delete_value(&mut self, hkey: u32, name: &str) -> bool {
        let Some(path) = self.path_of_handle(hkey) else {
            return false;
        };
        self.delete_value_path(&path, name)
    }

    pub fn delete_subkey(&mut self, hkey: u32, subkey: &str) -> bool {
        let Some(path) = self.full_path(hkey, subkey) else {
            return false;
        };
        self.delete_key_path(&path)
    }

    /// nth immediate subkey of an open key (RegEnumKeyEx).
    pub fn enum_key(&self, hkey: u32, index: u32) -> Option<String> {
        let path = self.path_of_handle(hkey)?;
        self.subkeys(&path).into_iter().nth(index as usize)
    }

    /// nth value of an open key (RegEnumValue).
    pub fn enum_value(&self, hkey: u32, index: u32) -> Option<(String, RegValue)> {
        let path = self.path_of_handle(hkey)?;
        let k = self.keys.get(&key_of(&path))?;
        k.values
            .iter()
            .nth(index as usize)
            .map(|(n, v)| (n.clone(), v.clone()))
    }

    // ---- persistence ----

    pub fn export(&self) -> RegistrySnapshot {
        self.keys.clone()
    }

    /// Replace the contents from a snapshot (open handles are reset).
    pub fn import(&mut self, snapshot: RegistrySnapshot) {
        self.keys = snapshot;
        self.handles.clear();
        self.next_handle = HANDLE_BASE;
        // Guarantee the roots always exist after a load.
        for hk in [
            HKEY_CLASSES_ROOT,
            HKEY_CURRENT_USER,
            HKEY_LOCAL_MACHINE,
            HKEY_USERS,
            HKEY_CURRENT_CONFIG,
        ] {
            self.ensure_key(root_name(hk).unwrap());
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Registry::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_create_set_query() {
        let mut r = Registry::new();
        // Non-existent key: open fails.
        assert!(r.open(HKEY_CURRENT_USER, "Software\\Acme\\App").is_none());
        // Create it, set and read back a value (case-insensitive name).
        let h = r.create(HKEY_CURRENT_USER, "Software\\Acme\\App").unwrap();
        assert!(r.set(h, "Count", RegValue::Dword(7)));
        assert_eq!(r.query(h, "count"), Some(&RegValue::Dword(7)));
        // A fresh open now succeeds (handle differs but resolves same path).
        let h2 = r.open(HKEY_CURRENT_USER, "SOFTWARE\\acme\\app").unwrap();
        assert_eq!(r.query(h2, "Count"), Some(&RegValue::Dword(7)));
    }

    #[test]
    fn enum_and_delete() {
        let mut r = Registry::new();
        r.create(HKEY_LOCAL_MACHINE, "Software\\T\\A");
        r.create(HKEY_LOCAL_MACHINE, "Software\\T\\B");
        let h = r.open(HKEY_LOCAL_MACHINE, "Software\\T").unwrap();
        let mut names: Vec<String> = (0..).map_while(|i| r.enum_key(h, i)).collect();
        names.sort();
        assert_eq!(names, vec!["A".to_string(), "B".to_string()]);
        assert!(r.delete_subkey(HKEY_LOCAL_MACHINE, "Software\\T\\A"));
        assert!(!r.key_exists("HKEY_LOCAL_MACHINE\\Software\\T\\A"));
        assert!(r.key_exists("HKEY_LOCAL_MACHINE\\Software\\T\\B"));
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut r = Registry::new();
        r.set_value_path(
            "HKEY_CURRENT_USER\\Software\\X",
            "Name",
            RegValue::Sz("hi".into()),
        );
        let snap = r.export();
        let mut r2 = Registry::new();
        r2.import(snap);
        assert_eq!(
            r2.get_value_path("HKEY_CURRENT_USER\\Software\\X", "Name"),
            Some(&RegValue::Sz("hi".into()))
        );
    }

    #[test]
    fn sz_to_bytes_is_utf16_null_terminated() {
        let b = RegValue::Sz("Hi".into()).to_bytes();
        assert_eq!(b, vec![b'H', 0, b'i', 0, 0, 0]);
    }
}
