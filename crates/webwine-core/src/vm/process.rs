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
    // GDI raster drawing on a window's client canvas. Colors are COLORREF (0x00BBGGRR).
    FillRect { hwnd: u32, x: i32, y: i32, w: i32, h: i32, color: u32 },
    Rect { hwnd: u32, x: i32, y: i32, w: i32, h: i32, fill: u32, stroke: u32 },
    Ellipse { hwnd: u32, x: i32, y: i32, w: i32, h: i32, fill: u32, stroke: u32 },
    Line { hwnd: u32, x1: i32, y1: i32, x2: i32, y2: i32, color: u32 },
    SetPixel { hwnd: u32, x: i32, y: i32, color: u32 },
    // Framebuffer blit: an RGBA8888 image copied to the window's client area at
    // (x,y). `src_w`/`src_h` are the source size; if they differ from w/h the
    // frontend scales. Produced by BitBlt/StretchDIBits from a DIB section.
    Blit { hwnd: u32, x: i32, y: i32, w: i32, h: i32, src_w: i32, src_h: i32, pixels: Vec<u8> },
    // System sounds.
    Beep { freq: u32, duration: u32 },
}

/// High byte tag marking a GDI object handle (memory DC / DIB section), distinct
/// from window HWNDs (0x0001xxxx) and the brush/pen handle tags in user32.
pub const GDI_TAG: u32 = 0x0D00_0000;

/// A GDI object addressed by an opaque handle (tagged with `GDI_TAG`).
#[derive(Clone)]
pub enum GdiObject {
    /// A memory device context, with the currently selected bitmap (0 = none).
    MemDc { bitmap: u32 },
    /// A device-independent bitmap: a pixel buffer living in guest memory.
    /// `top_down` is true when the source BITMAPINFOHEADER height was negative.
    Dib { bits: u32, width: i32, height: i32, bpp: u16, top_down: bool },
}

/// A queued window message (WM_*).
#[derive(Debug, Clone)]
pub struct GuestMsg {
    pub hwnd: u32,
    pub message: u32,
    pub wparam: u32,
    pub lparam: u32,
}

/// A request from a guest (CreateProcess) to launch a child process. The VM
/// drains these after each slice and loads them into the process table.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub path: String,
    pub pi_addr: u32, // guest PROCESS_INFORMATION to fill (0 if none)
}

/// Per-process Win32 GUI state: registered window classes, live windows, and
/// the thread message queue.
pub struct GuiState {
    pub next_hwnd: u32,
    pub classes: std::collections::HashMap<String, u32>, // class name -> WndProc VA
    pub windows: std::collections::HashMap<u32, WindowEntry>, // hwnd -> window
    pub queue: std::collections::VecDeque<GuestMsg>,
    pub quit: Option<u32>,
    // GDI objects (memory DCs, DIB sections) keyed by an opaque handle.
    pub gdi_objects: std::collections::HashMap<u32, GdiObject>,
    pub next_gdi: u32,
}

pub struct WindowEntry {
    pub wndproc: u32,
    pub needs_paint: bool,
    pub width: i32,
    pub height: i32,
    // GDI device-context state (hdc == hwnd in our model).
    pub pen_color: u32,
    pub brush_color: u32,
    pub cur_x: i32,
    pub cur_y: i32,
}

impl GuiState {
    pub fn new() -> Self {
        GuiState {
            next_hwnd: 0x0001_0010,
            classes: std::collections::HashMap::new(),
            windows: std::collections::HashMap::new(),
            queue: std::collections::VecDeque::new(),
            quit: None,
            gdi_objects: std::collections::HashMap::new(),
            next_gdi: GDI_TAG | 0x10,
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
    pub heap_sizes:  std::collections::HashMap<u32, u32>, // ptr -> size, for realloc
    pub memory:      GuestMemory,
    pub cpu:         X86Cpu,
    pub handles:     HandleTable,
    pub console:     ConsoleStreams,
    pub ui_events:   Vec<UiEvent>,
    pub gui:         GuiState,
    pub spawns:      Vec<SpawnRequest>,
    pub next_child_pid: u32, // pid the next CreateProcess child will receive
    pub state:       ProcessState,
    // Current working directory, initialized to the launched image's directory
    // (matching how Explorer launches a process). Relative guest paths resolve
    // against this, and SetCurrentDirectory updates it.
    pub cwd:         String,
    // Full command line, e.g. `"C:\...\doom.exe" -iwad doom1.wad`. Returned by
    // GetCommandLine and parsed into argv by the CRT startup.
    pub cmdline:     String,
    // Managed (.NET/CLI) image bytes, set when this is a managed process. Such a
    // process has no meaningful x86 CPU state; it runs via the CLR interpreter.
    pub managed:     Option<Vec<u8>>,
}

/// Directory portion of a guest path, e.g. `C:\a\b\foo.exe` -> `C:\a\b`.
pub fn parent_dir(path: &str) -> String {
    let p = path.replace('/', "\\");
    match p.rfind('\\') {
        Some(i) => {
            let d = &p[..i];
            if d.len() <= 2 { format!("{d}\\") } else { d.to_string() }
        }
        None => "C:\\".to_string(),
    }
}

impl GuestProcess {
    pub fn info(&self) -> ProcessInfo {
        ProcessInfo { pid: self.pid, path: self.path.clone(),
            image_base: self.image_base, entry_point: self.entry_point,
            state: self.state.clone() }
    }

    /// A process backed by a managed (.NET) assembly. The x86 fields are left
    /// empty; execution is driven by the CLR interpreter on the first slice.
    pub fn new_managed(pid: u32, path: &str, bytes: Vec<u8>) -> Self {
        GuestProcess {
            pid,
            path: path.to_string(),
            image_base: 0,
            entry_point: 0,
            heap_base: 0,
            heap_next: 0,
            heap_sizes: std::collections::HashMap::new(),
            memory: GuestMemory::new(),
            cpu: X86Cpu::new(),
            handles: HandleTable::new(pid),
            console: ConsoleStreams::new(),
            ui_events: Vec::new(),
            gui: GuiState::new(),
            spawns: Vec::new(),
            next_child_pid: 0,
            state: ProcessState::Created,
            cwd: parent_dir(path),
            cmdline: format!("\"{path}\""),
            managed: Some(bytes),
        }
    }
}

pub struct ProcessTable {
    pub processes: indexmap::IndexMap<u32, GuestProcess>,
    next_pid: u32,
}

impl ProcessTable {
    pub fn new() -> Self { ProcessTable { processes: indexmap::IndexMap::new(), next_pid: 1 } }
    pub fn alloc_pid(&mut self) -> u32 { let p = self.next_pid; self.next_pid += 1; p }
    pub fn peek_next_pid(&self) -> u32 { self.next_pid }
    pub fn insert(&mut self, p: GuestProcess) { self.processes.insert(p.pid, p); }
    pub fn get(&self, pid: u32) -> Option<&GuestProcess> { self.processes.get(&pid) }
    pub fn get_mut(&mut self, pid: u32) -> Option<&mut GuestProcess> { self.processes.get_mut(&pid) }
    pub fn list_info(&self) -> Vec<ProcessInfo> { self.processes.values().map(|p| p.info()).collect() }
}

impl Default for ProcessTable { fn default() -> Self { Self::new() } }
