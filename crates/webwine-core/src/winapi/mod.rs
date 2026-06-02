pub mod context;
pub mod kernel32;
pub mod msvcrt;
pub mod ntdll;
pub mod user32;

pub use context::{ApiContext, Handled};

use std::collections::HashMap;

const TRAMPOLINE_BASE: u32 = 0x7FFE_0000;

pub type HandlerFn = fn(&mut ApiContext) -> Handled;

pub struct WinApiRegistry {
    handlers: HashMap<(String, String), HandlerFn>,
    // function-name -> handler, used as a fallback when the exact (dll, name)
    // pair isn't registered. CRT/Win32 names are effectively globally unique,
    // and Windows routes the same function through many apiset DLLs
    // (api-ms-win-crt-runtime-l1-1-0.dll, ucrtbase.dll, msvcrt.dll, …).
    by_func:  HashMap<String, HandlerFn>,
    by_va:    HashMap<u32, (String, String)>,
    by_name:  HashMap<(String, String), u32>,
    next:     u32,
}

impl WinApiRegistry {
    pub fn new() -> Self {
        WinApiRegistry {
            handlers: HashMap::new(),
            by_func:  HashMap::new(),
            by_va:    HashMap::new(),
            by_name:  HashMap::new(),
            next:     TRAMPOLINE_BASE,
        }
    }

    /// Register a handler for a known function. Called during VM init.
    pub fn add(&mut self, dll: &str, name: &str, f: HandlerFn) {
        let key = (dll.to_ascii_uppercase(), name.to_string());
        self.handlers.insert(key, f);
        // First registration of a name wins for the fallback map; explicit
        // (dll, name) matches always take precedence in dispatch anyway.
        self.by_func.entry(name.to_string()).or_insert(f);
    }

    /// Allocate (or return existing) trampoline VA for a (dll, name) pair.
    /// Called during PE loading when patching the IAT.
    pub fn resolve_trampoline(&mut self, dll: &str, name: &str) -> u32 {
        let key = (dll.to_ascii_uppercase(), name.to_string());
        if let Some(&va) = self.by_name.get(&key) { return va; }
        let va = self.next; self.next += 4;
        self.by_va.insert(va, key.clone());
        self.by_name.insert(key, va);
        va
    }

    pub fn is_trampoline(&self, va: u32) -> bool {
        self.by_va.contains_key(&va)
    }

    pub fn dispatch(&self, va: u32, ctx: &mut ApiContext) -> Option<Handled> {
        let key = self.by_va.get(&va)?;
        // Exact (dll, name) first, then fall back to name-only.
        let f = self.handlers.get(key).or_else(|| self.by_func.get(&key.1));
        Some(match f {
            Some(handler) => handler(ctx),
            None => Handled::Unimplemented,
        })
    }

    pub fn lookup_name(&self, va: u32) -> Option<&(String, String)> {
        self.by_va.get(&va)
    }
}

impl Default for WinApiRegistry {
    fn default() -> Self { Self::new() }
}

/// Register all known handlers. Called once from WebWineVm::new().
pub fn register_all(r: &mut WinApiRegistry) {
    kernel32::register(r);
    msvcrt::register(r);
    ntdll::register(r);
    user32::register(r);
}
