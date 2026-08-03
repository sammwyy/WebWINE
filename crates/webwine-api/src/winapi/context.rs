use super::{ApiDispatcher, HandlerFn};
use crate::fs::vfs::VirtualFileSystem;
use crate::logs::LogBuffer;
use crate::vm::cpu::X86Cpu;
use crate::vm::handles::HandleTable;
use crate::vm::memory::GuestMemory;
use crate::vm::process::{ConsoleStreams, GuiState, SpawnRequest};
pub use crate::vm::process::{GuestMsg, UiEvent};

pub enum Handled {
    Ok,
    ExitProcess(u32),
    Unimplemented,
    /// The handler has NOT returned to the caller yet. The executor should call
    /// each listed guest function (cdecl, no args) in order, then perform a
    /// cdecl return from the current API call. Used by `_initterm` to run the
    /// CRT's C++ initializer tables, which our synchronous handlers can't call
    /// directly.
    CallChain(Vec<u32>),
    /// Same as CallChain, but aborts and returns the error code if any function
    /// returns non-zero. Used by `_initterm_e`.
    CallChainE(Vec<u32>),
    /// The call blocks (e.g. GetMessage with an empty queue). The executor
    /// suspends the process (WaitingForInput) WITHOUT advancing past the call,
    /// so it re-dispatches when the process is resumed.
    Block,
    /// Call a guest function `func(args...)` (stdcall, callee cleans its args),
    /// then return from the current API with the function's result in EAX,
    /// cleaning `ret_args` stdcall args. Used by DispatchMessage → WndProc.
    Invoke {
        func: u32,
        args: Vec<u32>,
        ret_args: u32,
    },
}

pub struct ApiContext<'a> {
    pub cpu: &'a mut X86Cpu,
    pub memory: &'a mut GuestMemory,
    pub handles: &'a mut HandleTable,
    pub console: &'a mut ConsoleStreams,
    pub ui_events: &'a mut Vec<UiEvent>,
    pub gui: &'a mut GuiState,
    pub spawns: &'a mut Vec<SpawnRequest>,
    /// pid the next CreateProcess child will receive (lets the handler fill
    /// PROCESS_INFORMATION synchronously).
    pub next_child_pid: u32,
    pub heap_next: &'a mut u32,
    pub heap_sizes: &'a mut std::collections::HashMap<u32, u32>,
    pub fs: &'a mut VirtualFileSystem,
    pub registry: &'a mut crate::registry::Registry,
    pub logs: &'a mut LogBuffer,
    pub pid: u32,
    /// Guest path of the running image (e.g. C:\Users\guest\Desktop\calc.exe).
    pub exe_path: &'a str,
    /// Current working directory; relative paths resolve against it.
    pub cwd: &'a mut String,
    /// Full command line (argv[0] is the quoted image path).
    pub cmdline: &'a str,
    /// Message-table resource (id -> text) for FormatMessage(FROM_HMODULE).
    pub messages: &'a std::collections::HashMap<u32, String>,
    pub strings: &'a std::collections::HashMap<u32, String>,
    pub proc_addr: &'a std::collections::HashMap<String, u32>,
    pub api_dispatcher: Option<&'a dyn ApiDispatcher>,
    pub tls_slots: &'a mut std::collections::HashMap<u32, u32>,
    pub next_tls: &'a mut u32,
    pub rand_seed: &'a mut u32,
    pub dll_state: &'a mut std::collections::HashMap<String, u32>,
}

pub trait ApiRuntimeEnv {
    fn arg(&self, n: u32) -> u32;
    fn return_stdcall(&mut self, retval: u32, nargs: u32);
    fn return_cdecl(&mut self, retval: u32);
    fn heap_alloc(&mut self, size: u32) -> u32;
    fn heap_realloc(&mut self, old: u32, new_size: u32) -> u32;
    fn read_wstr(&self, va: u32) -> String;
    fn read_u16(&self, va: u32) -> u16;
    fn read_u32(&self, va: u32) -> u32;
    fn write_u16(&mut self, va: u32, value: u16);
    fn write_u32(&mut self, va: u32, value: u32);
    fn write_bytes(&mut self, va: u32, bytes: &[u8]);
    fn proc_address(&self, dll: &str, name: &str) -> u32;
    fn api_handler(&self, dll: &str, name: &str) -> Option<HandlerFn>;
    fn call_api_stdcall(&mut self, dll: &str, name: &str, args: &[u32]) -> Option<(Handled, u32)>;
    fn last_error(&self) -> u32;
    fn set_last_error(&mut self, value: u32);
}

impl<'a> ApiContext<'a> {
    /// Read the nth argument (0-indexed) from the stack.
    /// At the time of the call: ESP → return_addr, ESP+4 → arg0, ESP+8 → arg1, …
    pub fn arg(&self, n: u32) -> u32 {
        self.memory.read_u32(self.cpu.esp + 4 + 4 * n).unwrap_or(0)
    }

    /// Stdcall return: callee cleans stack (ret_addr + nargs * 4).
    pub fn ret_stdcall(&mut self, retval: u32, nargs: u32) {
        let ret = self.memory.read_u32(self.cpu.esp).unwrap_or(0);
        self.cpu.esp = self.cpu.esp.wrapping_add(4 + 4 * nargs);
        self.cpu.eax = retval;
        self.cpu.eip = ret;
    }

    /// Cdecl return: caller cleans stack (only pop ret_addr).
    pub fn ret_cdecl(&mut self, retval: u32) {
        let ret = self.memory.read_u32(self.cpu.esp).unwrap_or(0);
        self.cpu.esp = self.cpu.esp.wrapping_add(4);
        self.cpu.eax = retval;
        self.cpu.eip = ret;
    }

    /// Resolve a guest path against this process's current working directory,
    /// normalizing `.`/`..` and `/` separators to an absolute `X:\...` form.
    pub fn resolve_path(&self, raw: &str) -> String {
        normalize_guest_path(self.cwd, raw)
    }

    /// Read a null-terminated ASCII string from guest memory.
    pub fn cstr(&self, va: u32) -> String {
        self.memory.read_cstr(va)
    }

    /// Read a null-terminated wide string from guest memory.
    pub fn wstr(&self, va: u32) -> String {
        self.memory.read_wstr(va)
    }

    /// Raw bytes of a null-terminated string. Use this instead of `cstr` when
    /// the result is copied back into guest memory or measured — `cstr`
    /// decodes lossily and mangles anything above ASCII.
    pub fn cstr_bytes(&self, va: u32) -> Vec<u8> {
        self.memory.read_cstr_bytes(va)
    }

    /// Raw UTF-16 units of a null-terminated wide string.
    pub fn wstr_units(&self, va: u32) -> Vec<u16> {
        self.memory.read_wstr_units(va)
    }

    pub fn current_trampoline_va(&self) -> u32 {
        self.cpu.eip
    }

    pub fn api_trampoline_va(&self, _dll: &str, name: &str) -> u32 {
        *self.proc_addr.get(name).unwrap_or(&0)
    }

    pub fn api_resolve_trampoline(&mut self, dll: &str, name: &str) -> u32 {
        self.api_trampoline_va(dll, name)
    }

    /// Call another registered host API through the runtime dispatcher using a
    /// temporary stdcall frame. This is intended for API-to-API delegation inside
    /// stubs; complex blocking/exit flows should be returned to the executor.
    pub fn call_api_stdcall(
        &mut self,
        dll: &str,
        name: &str,
        args: &[u32],
    ) -> Option<(Handled, u32)> {
        let handler = self.api_dispatcher?.handler(dll, name)?;
        let saved_esp = self.cpu.esp;
        let saved_eip = self.cpu.eip;
        let saved_eax = self.cpu.eax;
        let frame = saved_esp.wrapping_sub(4 + args.len() as u32 * 4);
        self.memory.ensure_mapped(frame, saved_esp);
        let _ = self.memory.write_u32(frame, saved_eip);
        for (i, &arg) in args.iter().enumerate() {
            let _ = self.memory.write_u32(frame + 4 + i as u32 * 4, arg);
        }
        self.cpu.esp = frame;
        let handled = handler(self);
        let retval = self.cpu.eax;
        self.cpu.esp = saved_esp;
        self.cpu.eip = saved_eip;
        if !matches!(handled, Handled::Ok) {
            self.cpu.eax = saved_eax;
        }
        Some((handled, retval))
    }

    /// Bump allocator on the process heap, tracking sizes so realloc can copy.
    pub fn heap_alloc(&mut self, size: u32) -> u32 {
        if size == 0 {
            return 0;
        }
        let aligned = (size + 15) & !15;
        let ptr = *self.heap_next;
        let new_next = self.heap_next.wrapping_add(aligned);
        // Back the new range with real memory (grows the heap region on demand).
        self.memory.ensure_mapped(ptr, new_next);
        *self.heap_next = new_next;
        self.heap_sizes.insert(ptr, size);
        ptr
    }

    /// Reallocate `old` to `new_size`, preserving contents. Grows the last
    /// block in place; otherwise allocates a fresh block and copies.
    pub fn heap_realloc(&mut self, old: u32, new_size: u32) -> u32 {
        if old == 0 {
            return self.heap_alloc(new_size);
        }
        if new_size == 0 {
            return 0;
        }
        let old_size = self.heap_sizes.get(&old).copied().unwrap_or(0);
        let aligned_old = (old_size + 15) & !15;

        // Most-recent allocation? Extend in place.
        if old.wrapping_add(aligned_old) == *self.heap_next {
            let new_next = old.wrapping_add((new_size + 15) & !15);
            self.memory.ensure_mapped(old, new_next);
            *self.heap_next = new_next;
            self.heap_sizes.insert(old, new_size);
            return old;
        }

        // Otherwise allocate and copy the overlap.
        let new_ptr = self.heap_alloc(new_size);
        let copy = old_size.min(new_size) as usize;
        if copy > 0 {
            if let Ok(bytes) = self.memory.read_bytes(old, copy) {
                let _ = self.memory.write_bytes(new_ptr, &bytes);
            }
        }
        new_ptr
    }
}

impl ApiRuntimeEnv for ApiContext<'_> {
    fn arg(&self, n: u32) -> u32 {
        self.arg(n)
    }

    fn return_stdcall(&mut self, retval: u32, nargs: u32) {
        self.ret_stdcall(retval, nargs);
    }

    fn return_cdecl(&mut self, retval: u32) {
        self.ret_cdecl(retval);
    }

    fn heap_alloc(&mut self, size: u32) -> u32 {
        self.heap_alloc(size)
    }

    fn heap_realloc(&mut self, old: u32, new_size: u32) -> u32 {
        self.heap_realloc(old, new_size)
    }

    fn read_wstr(&self, va: u32) -> String {
        self.wstr(va)
    }

    fn read_u16(&self, va: u32) -> u16 {
        self.memory.read_u16(va).unwrap_or(0)
    }

    fn read_u32(&self, va: u32) -> u32 {
        self.memory.read_u32(va).unwrap_or(0)
    }

    fn write_u16(&mut self, va: u32, value: u16) {
        let _ = self.memory.write_u16(va, value);
    }

    fn write_u32(&mut self, va: u32, value: u32) {
        let _ = self.memory.write_u32(va, value);
    }

    fn write_bytes(&mut self, va: u32, bytes: &[u8]) {
        let _ = self.memory.write_bytes(va, bytes);
    }

    fn proc_address(&self, dll: &str, name: &str) -> u32 {
        self.api_trampoline_va(dll, name)
    }

    fn api_handler(&self, dll: &str, name: &str) -> Option<HandlerFn> {
        self.api_dispatcher?.handler(dll, name)
    }

    fn call_api_stdcall(&mut self, dll: &str, name: &str, args: &[u32]) -> Option<(Handled, u32)> {
        self.call_api_stdcall(dll, name, args)
    }

    fn last_error(&self) -> u32 {
        self.cpu.last_error
    }

    fn set_last_error(&mut self, value: u32) {
        self.cpu.last_error = value;
    }
}

/// Resolve `raw` against `cwd` and normalize it to an absolute `X:\...` path.
/// Handles `/` separators, the `\\?\` verbatim prefix, drive-relative and
/// root-relative paths, and `.`/`..` components.
pub fn normalize_guest_path(cwd: &str, raw: &str) -> String {
    let r = raw.replace('/', "\\");
    let r = r.strip_prefix("\\\\?\\").unwrap_or(&r);

    let combined = if r.len() >= 2 && r.as_bytes()[1] == b':' {
        // Already absolute with a drive letter.
        r.to_string()
    } else if r.starts_with('\\') {
        // Rooted on the current drive.
        let drive = cwd.get(0..2).unwrap_or("C:");
        format!("{drive}{r}")
    } else {
        // Relative to the working directory.
        format!("{}\\{}", cwd.trim_end_matches('\\'), r)
    };

    let (drive, rest) = if combined.len() >= 2 && combined.as_bytes()[1] == b':' {
        (&combined[0..2], &combined[2..])
    } else {
        ("C:", combined.as_str())
    };

    let mut parts: Vec<&str> = Vec::new();
    for comp in rest.split('\\') {
        match comp {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    format!("{}\\{}", drive, parts.join("\\"))
}

#[cfg(test)]
mod tests {
    use super::normalize_guest_path;

    #[test]
    fn resolves_relative_against_cwd() {
        let cwd = "C:\\Games\\Doom";
        assert_eq!(
            normalize_guest_path(cwd, "doom1.wad"),
            "C:\\Games\\Doom\\doom1.wad"
        );
        // `.` and `/` separators, the shapes an app's IWAD search uses.
        assert_eq!(
            normalize_guest_path(cwd, ".\\doom1.wad"),
            "C:\\Games\\Doom\\doom1.wad"
        );
        assert_eq!(
            normalize_guest_path(cwd, "./doom1.wad"),
            "C:\\Games\\Doom\\doom1.wad"
        );
        assert_eq!(
            normalize_guest_path(cwd, "wads/e1m1.lmp"),
            "C:\\Games\\Doom\\wads\\e1m1.lmp"
        );
    }

    #[test]
    fn resolves_parent_and_absolute() {
        let cwd = "C:\\Games\\Doom";
        assert_eq!(
            normalize_guest_path(cwd, "..\\config.cfg"),
            "C:\\Games\\config.cfg"
        );
        assert_eq!(
            normalize_guest_path(cwd, "C:\\abs\\file.txt"),
            "C:\\abs\\file.txt"
        );
        // Drive-rooted path keeps the cwd's drive.
        assert_eq!(normalize_guest_path(cwd, "\\rooted\\x"), "C:\\rooted\\x");
        // Verbatim prefix is stripped.
        assert_eq!(normalize_guest_path(cwd, "\\\\?\\C:\\v\\y"), "C:\\v\\y");
    }
}
