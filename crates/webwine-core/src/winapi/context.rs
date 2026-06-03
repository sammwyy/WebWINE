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
    /// The call blocks (e.g. GetMessage with an empty queue). The executor
    /// suspends the process (WaitingForInput) WITHOUT advancing past the call,
    /// so it re-dispatches when the process is resumed.
    Block,
    /// Call a guest function `func(args...)` (stdcall, callee cleans its args),
    /// then return from the current API with the function's result in EAX,
    /// cleaning `ret_args` stdcall args. Used by DispatchMessage → WndProc.
    Invoke { func: u32, args: Vec<u32>, ret_args: u32 },
}

pub struct ApiContext<'a> {
    pub cpu:       &'a mut X86Cpu,
    pub memory:    &'a mut GuestMemory,
    pub handles:   &'a mut HandleTable,
    pub console:   &'a mut ConsoleStreams,
    pub ui_events: &'a mut Vec<UiEvent>,
    pub gui:       &'a mut GuiState,
    pub spawns:    &'a mut Vec<SpawnRequest>,
    /// pid the next CreateProcess child will receive (lets the handler fill
    /// PROCESS_INFORMATION synchronously).
    pub next_child_pid: u32,
    pub heap_next: &'a mut u32,
    pub heap_sizes: &'a mut std::collections::HashMap<u32, u32>,
    pub fs:        &'a mut VirtualFileSystem,
    pub logs:      &'a mut LogBuffer,
    pub pid:       u32,
    /// Guest path of the running image (e.g. C:\Users\guest\Desktop\calc.exe).
    pub exe_path:  &'a str,
    /// Function-name → trampoline VA, for GetProcAddress (0 = not available).
    pub proc_addr: &'a std::collections::HashMap<String, u32>,
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

    /// Read a null-terminated ASCII string from guest memory.
    pub fn cstr(&self, va: u32) -> String {
        self.memory.read_cstr(va)
    }

    /// Read a null-terminated wide string from guest memory.
    pub fn wstr(&self, va: u32) -> String {
        self.memory.read_wstr(va)
    }

    /// Bump allocator on the process heap, tracking sizes so realloc can copy.
    pub fn heap_alloc(&mut self, size: u32) -> u32 {
        if size == 0 { return 0; }
        let aligned = (size + 15) & !15;
        let ptr = *self.heap_next;
        *self.heap_next = self.heap_next.wrapping_add(aligned);
        self.heap_sizes.insert(ptr, size);
        ptr
    }

    /// Reallocate `old` to `new_size`, preserving contents. Grows the last
    /// block in place; otherwise allocates a fresh block and copies.
    pub fn heap_realloc(&mut self, old: u32, new_size: u32) -> u32 {
        if old == 0 { return self.heap_alloc(new_size); }
        if new_size == 0 { return 0; }
        let old_size = self.heap_sizes.get(&old).copied().unwrap_or(0);
        let aligned_old = (old_size + 15) & !15;

        // Most-recent allocation? Extend in place.
        if old.wrapping_add(aligned_old) == *self.heap_next {
            *self.heap_next = old.wrapping_add((new_size + 15) & !15);
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
