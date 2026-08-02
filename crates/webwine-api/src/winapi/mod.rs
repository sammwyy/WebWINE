pub mod context;

pub use context::{ApiContext, Handled};

use std::collections::{HashMap, HashSet};

const TRAMPOLINE_BASE: u32 = 0x7FFE_0000;

pub type HandlerFn = fn(&mut ApiContext) -> Handled;

pub trait ApiDispatcher {
    fn handler(&self, dll: &str, name: &str) -> Option<HandlerFn>;
}

pub struct WinApiRegistry {
    handlers: HashMap<(String, String), HandlerFn>,
    // function-name -> handler, used as a fallback when the exact (dll, name)
    // pair isn't registered. CRT/Win32 names are effectively globally unique,
    // and Windows routes the same function through many apiset DLLs
    // (api-ms-win-crt-runtime-l1-1-0.dll, ucrtbase.dll, msvcrt.dll, ...).
    by_func: HashMap<String, HandlerFn>,
    by_va: HashMap<u32, (String, String)>,
    by_name: HashMap<(String, String), u32>,
    // function name -> trampoline VA, for GetProcAddress (dynamic linking).
    proc_addr: HashMap<String, u32>,
    // Imported variables (CRT globals such as `_wcmdln`) are IAT entries too,
    // but they must resolve to guest data, never executable trampolines.
    data_symbols: HashMap<(String, String), u32>,
    // Upper-cased names of every DLL we provide built-in stubs for. The loader
    // uses this (not a name heuristic) to tell "we implement this DLL" from
    // "this is an app/third-party DLL that must come from a file".
    dll_names: HashSet<String>,
    next: u32,
}

impl WinApiRegistry {
    pub fn new() -> Self {
        WinApiRegistry {
            handlers: HashMap::new(),
            by_func: HashMap::new(),
            by_va: HashMap::new(),
            by_name: HashMap::new(),
            proc_addr: HashMap::new(),
            data_symbols: HashMap::new(),
            dll_names: HashSet::new(),
            next: TRAMPOLINE_BASE,
        }
    }

    /// Allocate a trampoline for every registered function name so GetProcAddress
    /// can hand back a real, callable address. Call once after all `add`s.
    pub fn finalize(&mut self) {
        let names: Vec<String> = self.by_func.keys().cloned().collect();
        for name in names {
            let va = self.resolve_trampoline("PROC", &name);
            self.proc_addr.insert(name, va);
        }
    }

    /// Trampoline VA for a dynamically-resolved function (GetProcAddress), or 0
    /// if we have no implementation, in which case the caller uses its fallback.
    pub fn proc_address(&self, name: &str) -> u32 {
        self.proc_addr.get(name).copied().unwrap_or(0)
    }

    pub fn proc_addr_map(&self) -> &HashMap<String, u32> {
        &self.proc_addr
    }

    /// Register a handler for a known function. Called during VM init.
    pub fn add(&mut self, dll: &str, name: &str, f: HandlerFn) {
        let dll_upper = dll.to_ascii_uppercase();
        let key = (dll_upper.clone(), name.to_string());
        self.handlers.insert(key, f);
        // First registration of a name wins for the fallback map; explicit
        // (dll, name) matches always take precedence in dispatch anyway.
        self.by_func.entry(name.to_string()).or_insert(f);
        // Record the DLL as "provided", except the synthetic buckets used for COM
        // vtable methods (`*.vtbl`) and GetProcAddress trampolines (`PROC`), which
        // aren't real importable DLL names.
        if !dll_upper.ends_with(".VTBL") && dll_upper != "PROC" {
            self.dll_names.insert(dll_upper);
        }
    }

    /// Register a guest address for an imported data symbol.
    pub fn add_data(&mut self, dll: &str, name: &str, address: u32) {
        self.data_symbols
            .insert((dll.to_ascii_uppercase(), name.to_string()), address);
        self.dll_names.insert(dll.to_ascii_uppercase());
    }

    pub fn data_address(&self, dll: &str, name: &str) -> Option<u32> {
        self.data_symbols
            .get(&(dll.to_ascii_uppercase(), name.to_string()))
            .copied()
    }

    /// True if we provide built-in stubs for this DLL (any function registered
    /// under its name). Replaces name-prefix heuristics in the loader.
    pub fn has_stub_dll(&self, dll: &str) -> bool {
        self.dll_names.contains(&dll.to_ascii_uppercase())
    }

    /// Every DLL name we stub, e.g. for seeding virtual files into System32.
    pub fn stub_dll_names(&self) -> impl Iterator<Item = &str> {
        self.dll_names.iter().map(|s| s.as_str())
    }

    /// Allocate (or return existing) trampoline VA for a (dll, name) pair.
    /// Called during PE loading when patching the IAT.
    pub fn resolve_trampoline(&mut self, dll: &str, name: &str) -> u32 {
        let key = (dll.to_ascii_uppercase(), name.to_string());
        if let Some(&va) = self.by_name.get(&key) {
            return va;
        }
        let va = self.next;
        self.next += 4;
        self.by_va.insert(va, key.clone());
        self.by_name.insert(key, va);
        va
    }

    pub fn is_trampoline(&self, va: u32) -> bool {
        self.by_va.contains_key(&va)
    }

    pub fn dispatch(&self, va: u32, ctx: &mut ApiContext) -> Option<Handled> {
        let key = self.by_va.get(&va)?;
        // Exact (dll, name) first, then name-only, then the UCRT "_o_" downlevel
        // alias (e.g. _o__set_app_type -> _set_app_type).
        let f = self
            .handlers
            .get(key)
            .or_else(|| self.by_func.get(&key.1))
            .or_else(|| key.1.strip_prefix("_o_").and_then(|n| self.by_func.get(n)));
        Some(match f {
            Some(handler) => handler(ctx),
            None => Handled::Unimplemented,
        })
    }

    /// Best-effort stdcall arg count for an unimplemented Win32 function, so the
    /// dispatcher can clean the stack instead of leaking args (which corrupts a
    /// later `ret`). CRT functions are cdecl (caller cleans) -> 0.
    pub fn unimpl_stdcall_args(&self, va: u32) -> u32 {
        let Some((dll, name)) = self.by_va.get(&va) else {
            return 0;
        };
        // CRT DLLs are cdecl, caller cleans, so the callee pops nothing.
        let d = dll.as_str();
        if d.starts_with("MSVCRT")
            || d.starts_with("UCRTBASE")
            || d.starts_with("VCRUNTIME")
            || d.starts_with("API-MS-WIN-CRT")
        {
            return 0;
        }
        default_stdcall_args(name)
    }

    pub fn lookup_name(&self, va: u32) -> Option<&(String, String)> {
        self.by_va.get(&va)
    }

    /// Whether we have a real handler for this import (exact (dll,name), the
    /// global name-only fallback, or the UCRT `_o_` downlevel alias). Used by the
    /// loader to tell a stubbed system DLL apart from a genuinely missing one.
    pub fn is_implemented(&self, dll: &str, name: &str) -> bool {
        let key = (dll.to_ascii_uppercase(), name.to_string());
        self.handlers.contains_key(&key)
            || self.by_func.contains_key(name)
            || name
                .strip_prefix("_o_")
                .is_some_and(|n| self.by_func.contains_key(n))
    }
}

impl ApiDispatcher for WinApiRegistry {
    fn handler(&self, dll: &str, name: &str) -> Option<HandlerFn> {
        let key = (dll.to_ascii_uppercase(), name.to_string());
        self.handlers
            .get(&key)
            .or_else(|| self.by_func.get(name))
            .or_else(|| name.strip_prefix("_o_").and_then(|n| self.by_func.get(n)))
            .copied()
    }
}

impl Default for WinApiRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Arg count for common Win32 stdcall functions, used to clean the stack when a
/// function is imported but not implemented. Keeps the guest stack balanced so
/// execution survives to the next real issue instead of derailing on a `ret`.
/// Unknown -> 1 (the common case; better than 0 which leaks for most APIs).
fn default_stdcall_args(name: &str) -> u32 {
    match name {
        // 0 args
        "GetLastError"
        | "GetCurrentThread"
        | "GetCurrentProcess"
        | "GetCurrentThreadId"
        | "GetCurrentProcessId"
        | "GetCommandLineA"
        | "GetCommandLineW"
        | "GetTickCount"
        | "GetTickCount64"
        | "GetCursor"
        | "ReleaseCapture"
        | "GetDesktopWindow"
        | "GetActiveWindow"
        | "GetForegroundWindow" => 0,
        // 1 arg
        "CloseHandle"
        | "SetLastError"
        | "ExitProcess"
        | "Sleep"
        | "LocalFree"
        | "GlobalFree"
        | "SetThreadLocale"
        | "GetThreadLocale"
        | "IsWindow"
        | "DestroyWindow"
        | "FreeLibrary"
        | "DeleteObject"
        | "GetDC"
        | "SetThreadStackGuarantee"
        | "RtlDeleteCriticalSection"
        | "ClipCursor"
        | "DestroyCursor"
        | "DestroyIcon"
        | "GetCursorPos"
        | "GetKeyState"
        | "GetAsyncKeyState"
        | "GetKeyboardLayout"
        | "GetKeyboardLayoutNameA"
        | "GetKeyboardState"
        | "GetMenu"
        | "GetParent"
        | "IsZoomed"
        | "IsIconic"
        | "SetCapture"
        | "SetCursor"
        | "SetFocus"
        | "SetForegroundWindow"
        | "PostQuitMessage"
        | "GetSystemMetrics"
        | "DeleteDC"
        | "RealizePalette"
        | "GetSystemPaletteUse"
        | "UnrealizeObject"
        | "CreatePalette"
        | "UpdateWindow"
        | "GetWindowDC"
        | "SwapBuffers" => 1,
        // 2 args
        "GetProcAddress"
        | "SetEvent"
        | "WaitForSingleObject"
        | "ReleaseMutex"
        | "EnableWindow"
        | "ShowWindow"
        | "GetWindowRect"
        | "ReleaseDC"
        | "ClientToScreen"
        | "ScreenToClient"
        | "ChangeDisplaySettingsA"
        | "GetWindowLongA"
        | "GetWindowLongW"
        | "KillTimer"
        | "LoadKeyboardLayoutA"
        | "SetCursorPos"
        | "UnregisterClassA"
        | "UnregisterClassW"
        | "WindowFromPoint"
        | "SetWindowTextA"
        | "SetWindowTextW"
        | "GetClientRect"
        | "BeginPaint"
        | "EndPaint"
        | "SetDeviceGammaRamp"
        | "GetDeviceGammaRamp"
        | "GetDeviceCaps"
        | "SetSystemPaletteUse"
        | "ChoosePixelFormat"
        | "GetStockObject" => 2,
        // 3 args
        "OpenThread"
        | "VirtualFree"
        | "TlsSetValue"
        | "HeapDestroy"
        | "VirtualLock"
        | "EnumDisplaySettingsA"
        | "GetClassInfoA"
        | "InvalidateRect"
        | "MapVirtualKeyExA"
        | "SetClassLongA"
        | "SetWindowLongA"
        | "SetWindowLongW"
        | "AdjustWindowRect"
        | "PtInRect"
        | "SetPixelFormat"
        | "SelectPalette" => 3,
        // 4 args
        "VirtualAlloc"
        | "MessageBoxA"
        | "MessageBoxW"
        | "CreateThread"
        | "VirtualProtect"
        | "AdjustWindowRectEx"
        | "MapWindowPoints"
        | "PostMessageA"
        | "PostMessageW"
        | "SetTimer"
        | "GetMessageA"
        | "GetMessageW"
        | "DefWindowProcA"
        | "DefWindowProcW"
        | "DescribePixelFormat"
        | "SetDIBColorTable"
        | "SetPaletteEntries"
        | "GetSystemPaletteEntries" => 4,
        // 5 args
        "CallWindowProcA" | "PeekMessageA" | "PeekMessageW" | "ToAsciiEx" | "ToUnicode" => 5,
        // 6 args
        "CreateFileMappingW" | "RegOpenKeyExW" | "RegQueryValueExW" | "RegOpenKeyExA"
        | "RegQueryValueExA" | "MoveWindow" | "LoadImageA" | "LoadImageW" => 6,
        // 7 args
        "CreateCursor" | "CreateIconFromResourceEx" | "SetWindowPos" | "GetDIBits" => 7,
        // 9+ args
        "BitBlt" => 9,
        "StretchBlt" => 11,
        "SetDIBitsToDevice" => 12,
        "StretchDIBits" => 13,
        "CreateProcessW" | "CreateProcessA" => 10,
        "CreateWindowExA" | "CreateWindowExW" => 12,
        // Unknown stdcall: assume 1 arg (most common). Better than leaking on a
        // multi-arg call, and a wrong guess here is rare and contained.
        _ => 1,
    }
}
