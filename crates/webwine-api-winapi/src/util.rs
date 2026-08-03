//! Shared registration helpers for winapi DLL modules.

use super::{HandlerFn, WinApiRegistry};

pub type Entry = (&'static str, &'static str, HandlerFn);

pub fn register_entries(r: &mut WinApiRegistry, entries: &[Entry]) {
    for &(dll, name, handler) in entries {
        r.add(dll, name, handler);
    }
}
