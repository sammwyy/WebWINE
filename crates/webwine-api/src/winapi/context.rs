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
    /// Coalesced free list for HeapFree reuse (sorted by address).
    pub heap_free_list: &'a mut Vec<(u32, u32)>,
    /// Exclusive upper bound for bump growth (DLL region base).
    pub heap_limit: u32,
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

    /// Process heap allocator with free-list reuse + bump growth.
    ///
    /// - `size == 0` still returns a unique non-NULL block (Windows HeapAlloc /
    ///   CRT `malloc(0)` behaviour).
    /// - Prefer a free-list block (first-fit) so alloc/free loops in games do
    ///   not exhaust the ~1 GiB guest heap high-water mark.
    /// - Returns `0` only on real OOM.
    pub fn heap_alloc(&mut self, size: u32) -> u32 {
        let size = size.max(1);
        let aligned = (size + 15) & !15;

        // 1) First-fit from the free list.
        if let Some(idx) = self
            .heap_free_list
            .iter()
            .position(|&(_, s)| s >= aligned)
        {
            let (ptr, block) = self.heap_free_list.remove(idx);
            let rem = block - aligned;
            if rem >= 16 {
                // Split: keep the tail free.
                self.heap_free_insert(ptr.wrapping_add(aligned), rem);
            }
            self.heap_sizes.insert(ptr, size);
            return ptr;
        }

        // 2) Bump allocate (must stay below heap_limit / DLL region).
        let ptr = *self.heap_next;
        let new_next = match ptr.checked_add(aligned) {
            Some(n) => n,
            None => return 0,
        };
        if new_next > self.heap_limit {
            return 0;
        }
        if !self.memory.ensure_mapped(ptr, new_next) {
            return 0;
        }
        if !self.memory.is_range_mapped(ptr, aligned) {
            return 0;
        }
        *self.heap_next = new_next;
        self.heap_sizes.insert(ptr, size);
        ptr
    }

    /// Allocate and force-zero the block (HeapAlloc HEAP_ZERO_MEMORY / calloc).
    pub fn heap_alloc_zeroed(&mut self, size: u32) -> u32 {
        let ptr = self.heap_alloc(size);
        if ptr != 0 {
            let n = size.max(1) as usize;
            let _ = self.memory.write_bytes(ptr, &vec![0u8; n]);
        }
        ptr
    }

    /// Return a block to the free list (coalescing neighbours).
    pub fn heap_free_block(&mut self, ptr: u32) {
        if ptr == 0 {
            return;
        }
        let size = match self.heap_sizes.remove(&ptr) {
            Some(s) => (s + 15) & !15,
            None => return, // unknown / double-free: ignore
        };
        // If this was the most recent bump allocation, rewind the cursor.
        if ptr.wrapping_add(size) == *self.heap_next {
            *self.heap_next = ptr;
            // Also absorb any free blocks that now sit at the new tip.
            self.heap_free_list
                .retain(|&(p, s)| {
                    if p.wrapping_add(s) == *self.heap_next {
                        // shouldn't happen after rewind
                        true
                    } else if p == *self.heap_next {
                        false
                    } else {
                        true
                    }
                });
            // Pull free blocks that abut the tip into the rewind.
            loop {
                if let Some(i) = self
                    .heap_free_list
                    .iter()
                    .position(|&(p, s)| p.wrapping_add(s) == *self.heap_next)
                {
                    let (p, s) = self.heap_free_list.remove(i);
                    *self.heap_next = p;
                    let _ = s;
                } else {
                    break;
                }
            }
            return;
        }
        self.heap_free_insert(ptr, size);
    }

    fn heap_free_insert(&mut self, mut ptr: u32, mut size: u32) {
        // Insert sorted by address, coalescing with immediate neighbours.
        let mut i = 0;
        while i < self.heap_free_list.len() && self.heap_free_list[i].0 < ptr {
            i += 1;
        }
        // Merge with previous?
        if i > 0 {
            let (pp, ps) = self.heap_free_list[i - 1];
            if pp.wrapping_add(ps) == ptr {
                ptr = pp;
                size += ps;
                self.heap_free_list.remove(i - 1);
                i -= 1;
            }
        }
        // Merge with next?
        if i < self.heap_free_list.len() {
            let (np, ns) = self.heap_free_list[i];
            if ptr.wrapping_add(size) == np {
                size += ns;
                self.heap_free_list.remove(i);
            }
        }
        self.heap_free_list.insert(i, (ptr, size));
    }

    /// Reallocate `old` to `new_size`, preserving contents.
    pub fn heap_realloc(&mut self, old: u32, new_size: u32) -> u32 {
        if old == 0 {
            return self.heap_alloc(new_size);
        }
        if new_size == 0 {
            return self.heap_alloc(0);
        }
        let old_size = self.heap_sizes.get(&old).copied().unwrap_or(0);
        let aligned_old = (old_size + 15) & !15;
        let aligned_new = (new_size + 15) & !15;

        // In-place shrink: keep block, free the tail if large enough.
        if aligned_new <= aligned_old {
            self.heap_sizes.insert(old, new_size);
            let rem = aligned_old - aligned_new;
            if rem >= 16 {
                // Temporarily park tail as free without removing `old` from sizes.
                self.heap_free_insert(old.wrapping_add(aligned_new), rem);
                // If this was the tip allocation, rewind the bump cursor.
                if old.wrapping_add(aligned_old) == *self.heap_next {
                    *self.heap_next = old.wrapping_add(aligned_new);
                }
            }
            return old;
        }

        // Most-recent allocation? Extend in place.
        if old.wrapping_add(aligned_old) == *self.heap_next {
            let new_next = match old.checked_add(aligned_new) {
                Some(n) if n <= self.heap_limit => n,
                _ => return 0,
            };
            if !self.memory.ensure_mapped(old, new_next) {
                return 0;
            }
            let ext = (aligned_new - aligned_old) as usize;
            let _ = self
                .memory
                .write_bytes(old.wrapping_add(aligned_old), &vec![0u8; ext]);
            *self.heap_next = new_next;
            self.heap_sizes.insert(old, new_size);
            return old;
        }

        // Allocate fresh, copy, free old.
        let new_ptr = self.heap_alloc(new_size);
        if new_ptr == 0 {
            return 0;
        }
        let copy = old_size.min(new_size) as usize;
        if copy > 0 {
            if let Ok(bytes) = self.memory.read_bytes(old, copy) {
                let _ = self.memory.write_bytes(new_ptr, &bytes);
            }
        }
        self.heap_free_block(old);
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
