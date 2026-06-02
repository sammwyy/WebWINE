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

/// UI requests emitted by guest code for the frontend to render as real
/// windows rather than console text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiEvent {
    MessageBox { title: String, text: String, style: u32 },
    CreateWindow { hwnd: u32, title: String, x: i32, y: i32, width: i32, height: i32 },
    ShowWindow { hwnd: u32, show: bool },
    SetWindowText { hwnd: u32, title: String },
    DestroyWindow { hwnd: u32 },
    ClearClient { hwnd: u32 },
    DrawText { hwnd: u32, x: i32, y: i32, text: String, color: u32 },
}

/// A queued window message (WM_*).
#[derive(Debug, Clone)]
pub struct GuestMsg {
    pub hwnd: u32,
    pub message: u32,
    pub wparam: u32,
    pub lparam: u32,
}

/// Per-process Win32 GUI state: registered window classes, live windows, and
/// the thread message queue.
pub struct GuiState {
    pub next_hwnd: u32,
    pub classes: std::collections::HashMap<String, u32>, // class name -> WndProc VA
    pub windows: std::collections::HashMap<u32, WindowEntry>, // hwnd -> window
    pub queue: std::collections::VecDeque<GuestMsg>,
    pub quit: Option<u32>,
}

pub struct WindowEntry {
    pub wndproc: u32,
    pub needs_paint: bool,
    pub width: i32,
    pub height: i32,
}

impl GuiState {
    pub fn new() -> Self {
        GuiState {
            next_hwnd: 0x0001_0010,
            classes: std::collections::HashMap::new(),
            windows: std::collections::HashMap::new(),
            queue: std::collections::VecDeque::new(),
            quit: None,
        }
    }
}

impl Default for GuiState {
    fn default() -> Self { Self::new() }
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
    pub ui_events:   Vec<UiEvent>,
    pub gui:         GuiState,
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
