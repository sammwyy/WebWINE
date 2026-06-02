use std::collections::HashMap;

const TRAMPOLINE_BASE: u32 = 0x7FFE_0000;

#[derive(Debug, Clone)]
pub enum ApiId {
    // kernel32
    GetStdHandle,
    WriteFile,
    ReadFile,
    ExitProcess,
    GetLastError,
    SetLastError,
    CloseHandle,
    GetProcessHeap,
    HeapAlloc,
    HeapFree,
    VirtualAlloc,
    VirtualFree,
    GetModuleHandleA,
    GetModuleHandleW,
    GetProcAddress,
    CreateFileA,
    CreateFileW,
    // msvcrt / ucrtbase
    Printf,
    Fprintf,
    Sprintf,
    Puts,
    Putchar,
    Getchar,
    Exit,
    Malloc,
    Free,
    Realloc,
    Memcpy,
    Memmove,
    Memset,
    Strlen,
    Strcmp,
    Strcpy,
    Strcat,
    // ntdll
    RtlAllocateHeap,
    RtlFreeHeap,
    RtlZeroMemory,
    NtClose,
    // user32
    MessageBoxA,
    MessageBoxW,
    // unknown — will trap and log
    Unknown { dll: String, name: String },
}

pub struct WinApiDispatcher {
    // trampoline VA -> ApiId
    trampolines: HashMap<u32, ApiId>,
    // (DLL_UPPER, name) -> trampoline VA  (dedup cache)
    cache: HashMap<(String, String), u32>,
    next: u32,
}

impl WinApiDispatcher {
    pub fn new() -> Self {
        WinApiDispatcher {
            trampolines: HashMap::new(),
            cache: HashMap::new(),
            next: TRAMPOLINE_BASE,
        }
    }

    /// Returns the trampoline VA for the given import; allocates one if needed.
    pub fn resolve(&mut self, dll: &str, name: &str) -> u32 {
        let key = (dll.to_ascii_uppercase(), name.to_string());
        if let Some(&va) = self.cache.get(&key) {
            return va;
        }
        let va = self.next;
        self.next += 4;
        let id = api_id(&key.0, name);
        self.trampolines.insert(va, id);
        self.cache.insert(key, va);
        va
    }

    pub fn lookup(&self, va: u32) -> Option<&ApiId> {
        self.trampolines.get(&va)
    }

    pub fn is_trampoline(&self, va: u32) -> bool {
        va >= TRAMPOLINE_BASE && self.trampolines.contains_key(&va)
    }
}

impl Default for WinApiDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn api_id(dll_upper: &str, name: &str) -> ApiId {
    match (dll_upper, name) {
        ("KERNEL32.DLL", "GetStdHandle")      => ApiId::GetStdHandle,
        ("KERNEL32.DLL", "WriteFile")         => ApiId::WriteFile,
        ("KERNEL32.DLL", "ReadFile")          => ApiId::ReadFile,
        ("KERNEL32.DLL", "ExitProcess")       => ApiId::ExitProcess,
        ("KERNEL32.DLL", "GetLastError")      => ApiId::GetLastError,
        ("KERNEL32.DLL", "SetLastError")      => ApiId::SetLastError,
        ("KERNEL32.DLL", "CloseHandle")       => ApiId::CloseHandle,
        ("KERNEL32.DLL", "GetProcessHeap")    => ApiId::GetProcessHeap,
        ("KERNEL32.DLL", "HeapAlloc")         => ApiId::HeapAlloc,
        ("KERNEL32.DLL", "HeapFree")          => ApiId::HeapFree,
        ("KERNEL32.DLL", "VirtualAlloc")      => ApiId::VirtualAlloc,
        ("KERNEL32.DLL", "VirtualFree")       => ApiId::VirtualFree,
        ("KERNEL32.DLL", "GetModuleHandleA")  => ApiId::GetModuleHandleA,
        ("KERNEL32.DLL", "GetModuleHandleW")  => ApiId::GetModuleHandleW,
        ("KERNEL32.DLL", "GetProcAddress")    => ApiId::GetProcAddress,
        ("KERNEL32.DLL", "CreateFileA")       => ApiId::CreateFileA,
        ("KERNEL32.DLL", "CreateFileW")       => ApiId::CreateFileW,
        (d, _n) if d.starts_with("MSVCRT") || d.starts_with("UCRTBASE") || d.starts_with("VCRUNTIME") => {
            msvcrt_id(name)
        }
        ("NTDLL.DLL", "RtlAllocateHeap")  => ApiId::RtlAllocateHeap,
        ("NTDLL.DLL", "RtlFreeHeap")      => ApiId::RtlFreeHeap,
        ("NTDLL.DLL", "RtlZeroMemory")    => ApiId::RtlZeroMemory,
        ("NTDLL.DLL", "NtClose")          => ApiId::NtClose,
        ("USER32.DLL", "MessageBoxA")     => ApiId::MessageBoxA,
        ("USER32.DLL", "MessageBoxW")     => ApiId::MessageBoxW,
        (dll, name) => ApiId::Unknown {
            dll: dll.to_string(),
            name: name.to_string(),
        },
    }
}

fn msvcrt_id(name: &str) -> ApiId {
    match name {
        "printf" | "_printf" | "__stdio_common_vfprintf" => ApiId::Printf,
        "fprintf"                                         => ApiId::Fprintf,
        "sprintf" | "_sprintf"                            => ApiId::Sprintf,
        "puts"                                            => ApiId::Puts,
        "putchar"                                         => ApiId::Putchar,
        "getchar"                                         => ApiId::Getchar,
        "exit" | "_exit"                                  => ApiId::Exit,
        "malloc"                                          => ApiId::Malloc,
        "free"                                            => ApiId::Free,
        "realloc"                                         => ApiId::Realloc,
        "memcpy" | "memmove" | "__movsb"                  => ApiId::Memcpy,
        "memset" | "__stosb"                              => ApiId::Memset,
        "strlen"                                          => ApiId::Strlen,
        "strcmp"                                          => ApiId::Strcmp,
        "strcpy"                                          => ApiId::Strcpy,
        "strcat"                                          => ApiId::Strcat,
        n => ApiId::Unknown { dll: "msvcrt.dll".into(), name: n.to_string() },
    }
}
