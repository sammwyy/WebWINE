use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

use crate::vm::cpu::X86Cpu;
use crate::vm::handles::HandleTable;
use crate::vm::memory::GuestMemory;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProcessState {
    Created,
    Running,
    Blocked,
    WaitingForInput,
    Exited  { exit_code: u32 },
    Crashed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid:         u32,
    pub path:        String,
    pub image_base:  u32,
    pub entry_point: u32,
    pub state:       ProcessState,
}

pub struct ConsoleStreams {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdin:  VecDeque<u8>,
}

impl ConsoleStreams {
    pub fn new() -> Self {
        ConsoleStreams { stdout: Vec::new(), stderr: Vec::new(), stdin: VecDeque::new() }
    }
    pub fn drain_stdout(&mut self) -> Vec<u8> { std::mem::take(&mut self.stdout) }
    pub fn drain_stderr(&mut self) -> Vec<u8> { std::mem::take(&mut self.stderr) }
}

pub struct GuestProcess {
    pub pid:         u32,
    pub path:        String,
    pub image_base:  u32,
    pub entry_point: u32,
    pub heap_base:   u32,
    pub heap_next:   u32,   // bump allocator pointer
    pub memory:      GuestMemory,
    pub cpu:         X86Cpu,
    pub handles:     HandleTable,
    pub console:     ConsoleStreams,
    pub state:       ProcessState,
}

impl GuestProcess {
    pub fn info(&self) -> ProcessInfo {
        ProcessInfo { pid: self.pid, path: self.path.clone(),
            image_base: self.image_base, entry_point: self.entry_point,
            state: self.state.clone() }
    }
}

pub struct ProcessTable {
    pub processes: indexmap::IndexMap<u32, GuestProcess>,
    next_pid: u32,
}

impl ProcessTable {
    pub fn new() -> Self { ProcessTable { processes: indexmap::IndexMap::new(), next_pid: 1 } }
    pub fn alloc_pid(&mut self) -> u32 { let p = self.next_pid; self.next_pid += 1; p }
    pub fn insert(&mut self, p: GuestProcess) { self.processes.insert(p.pid, p); }
    pub fn get(&self, pid: u32) -> Option<&GuestProcess> { self.processes.get(&pid) }
    pub fn get_mut(&mut self, pid: u32) -> Option<&mut GuestProcess> { self.processes.get_mut(&pid) }
    pub fn list_info(&self) -> Vec<ProcessInfo> { self.processes.values().map(|p| p.info()).collect() }
}

impl Default for ProcessTable { fn default() -> Self { Self::new() } }
