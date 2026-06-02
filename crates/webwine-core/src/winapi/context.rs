use crate::fs::vfs::VirtualFileSystem;
use crate::logs::LogBuffer;
use crate::vm::cpu::X86Cpu;
use crate::vm::handles::HandleTable;
use crate::vm::memory::GuestMemory;
use crate::vm::process::ConsoleStreams;
pub use crate::vm::process::UiEvent;

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
}

pub struct ApiContext<'a> {
    pub cpu:       &'a mut X86Cpu,
    pub memory:    &'a mut GuestMemory,
    pub handles:   &'a mut HandleTable,
    pub console:   &'a mut ConsoleStreams,
    pub ui_events: &'a mut Vec<UiEvent>,
    pub heap_next: &'a mut u32,
    pub fs:        &'a mut VirtualFileSystem,
    pub logs:      &'a mut LogBuffer,
    pub pid:       u32,
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

    /// Simple bump allocator on the process heap.
    pub fn heap_alloc(&mut self, size: u32) -> u32 {
        if size == 0 { return 0; }
        let aligned = (size + 7) & !7;
        let ptr = *self.heap_next;
        *self.heap_next = self.heap_next.wrapping_add(aligned);
        ptr
    }
}
