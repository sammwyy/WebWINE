use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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
    Exited { exit_code: u32 },
    Crashed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub path: String,
    pub image_base: u32,
    pub entry_point: u32,
    pub state: ProcessState,
    /// Sum of live heap block sizes (bytes currently allocated via HeapAlloc/malloc).
    pub heap_used: u64,
    /// Number of live heap blocks.
    pub heap_blocks: u32,
    /// Bytes sitting on the free list (reusable without bumping).
    pub heap_free: u64,
    /// Free-list block count.
    pub heap_free_blocks: u32,
    /// High-water mark: heap_next - heap_base (bump cursor extent).
    pub heap_committed: u64,
    /// Max guest VA the heap can grow into (toward the DLL region).
    pub heap_limit: u64,
    /// Sum of all mapped GuestMemory region sizes for this process.
    pub mapped_bytes: u64,
    /// Number of mapped regions (image, heap, stack, DLLs, …).
    pub region_count: u32,
}

/// Aggregate memory snapshot for Task Manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMemoryInfo {
    pub processes: Vec<ProcessInfo>,
    pub total_heap_used: u64,
    pub total_heap_free: u64,
    pub total_heap_committed: u64,
    pub total_mapped: u64,
    /// Soft guest heap capacity per process (~1 GiB layout).
    pub heap_limit_per_process: u64,
}

pub struct ConsoleStreams {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdin: VecDeque<u8>,
}

impl ConsoleStreams {
    pub fn new() -> Self {
        ConsoleStreams {
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdin: VecDeque::new(),
        }
    }
    pub fn drain_stdout(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.stdout)
    }
    pub fn drain_stderr(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.stderr)
    }
}

/// UI requests emitted by guest code for the frontend to render as real
/// windows rather than console text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiEvent {
    MessageBox {
        title: String,
        text: String,
        style: u32,
    },
    CreateWindow {
        hwnd: u32,
        title: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    ShowWindow {
        hwnd: u32,
        show: bool,
    },
    SetWindowText {
        hwnd: u32,
        title: String,
    },
    DestroyWindow {
        hwnd: u32,
    },
    ClearClient {
        hwnd: u32,
    },
    DrawText {
        hwnd: u32,
        x: i32,
        y: i32,
        text: String,
        color: u32,
    },
    // GDI raster drawing on a window's client canvas. Colors are COLORREF (0x00BBGGRR).
    FillRect {
        hwnd: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: u32,
    },
    Rect {
        hwnd: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        fill: u32,
        stroke: u32,
    },
    Ellipse {
        hwnd: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        fill: u32,
        stroke: u32,
    },
    Line {
        hwnd: u32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: u32,
    },
    SetPixel {
        hwnd: u32,
        x: i32,
        y: i32,
        color: u32,
    },
    // Framebuffer blit: an RGBA8888 image copied to the window's client area at
    // (x,y). `src_w`/`src_h` are the source size; if they differ from w/h the
    // frontend scales. Produced by BitBlt/StretchDIBits from a DIB section.
    Blit {
        hwnd: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        src_w: i32,
        src_h: i32,
        pixels: Vec<u8>,
    },
    // System sounds.
    Beep {
        freq: u32,
        duration: u32,
    },

    // The window's menu bar (SetMenu). `items` is the top-level bar; each may have
    // `children` (dropdown). Clicking a leaf posts WM_COMMAND(id) to the window.
    SetMenu {
        hwnd: u32,
        items: Vec<MenuItemData>,
    },

    /// Full layout of a dialog and its child controls (Wine dialog manager).
    /// Frontend paints classic Win32 chrome and maps clicks → WM_COMMAND.
    DialogLayout {
        hwnd: u32,
        title: String,
        width: i32,
        height: i32,
        controls: Vec<DialogControlData>,
    },

    /// Update one control's text after SetDlgItemText / SetWindowText on a child.
    ControlText {
        hwnd: u32,
        control_hwnd: u32,
        text: String,
    },

    // A modal file picker the guest is blocked on (GetOpenFileName/GetSaveFileName).
    // The host shows a picker and replies via `post_dialog_reply`. `filter` is the
    // raw Win32 double-null filter string flattened to "Label|pattern|..." pairs.
    FileDialog {
        save: bool,
        title: String,
        filter: String,
        initial_dir: String,
        default_name: String,
    },

    // Direct3D8 GPU command stream
    // Emitted by the directx crate's D3D8 state tracker; consumed by the host
    // VideoDriver (WebGL/WebGPU). The guest-side D3D8 COM layer translates draw
    // calls into these backend-agnostic commands. `hwnd` is the device's window.
    //
    // Clear the backbuffer to `color` (D3DCOLOR = 0x00AARRGGBB... actually ARGB).
    GpuClear {
        hwnd: u32,
        color: u32,
    },
    // Define/upload a texture `id` with RGBA8888 `pixels` (w*h*4 bytes).
    GpuTexture {
        hwnd: u32,
        id: u32,
        w: u32,
        h: u32,
        pixels: Vec<u8>,
    },
    // Draw textured/colored triangles. `verts` is a flat list of
    // [x, y, u, v, r, g, b, a] per vertex (screen-space px, 0..1 uv, 0..1 rgba);
    // every 3 verts = a triangle. `texture` 0 = untextured. `blend` = a small
    // blend-mode id (0 none, 1 alpha, 2 additive).
    GpuDrawTris {
        hwnd: u32,
        texture: u32,
        blend: u32,
        verts: Vec<f32>,
    },
    // End of frame: flush/swap.
    GpuPresent {
        hwnd: u32,
    },
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
    Dib {
        bits: u32,
        width: i32,
        height: i32,
        bpp: u16,
        top_down: bool,
    },
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

    pub ddraw_display_w: u32,
    pub ddraw_display_h: u32,
    pub ddraw_display_bpp: u32,
    pub ddraw_surfaces: std::collections::HashMap<u32, DDrawSurface>,
    pub next_ddraw_surface: u32,

    // A modal dialog (MessageBox / file picker) the guest is blocked on, waiting
    // for the user's choice. `pending` is set while the dialog is on screen;
    // `reply` is filled by the host when the user answers, which the blocked API
    // handler reads on resume. See `WebWineVm::post_dialog_reply`.
    pub dialog_pending: bool,
    pub dialog_reply: Option<DialogReply>,

    // Menus, keyed by an opaque HMENU (tagged with `MENU_TAG`). A menu is a flat
    // list of items; a popup item points at a child menu by handle. `hwnd_menu`
    // maps a window to its attached menu bar (SetMenu).
    pub menus: std::collections::HashMap<u32, Vec<MenuItem>>,
    pub next_menu: u32,
    pub hwnd_menu: std::collections::HashMap<u32, u32>,

    /// Fast GetDlgItem: (dialog_hwnd, control_id) → child HWND.
    pub dlg_ctrl: std::collections::HashMap<(u32, i32), u32>,
}

/// High byte tag marking an HMENU, distinct from HWNDs and GDI handles.
pub const MENU_TAG: u32 = 0x0E00_0000;

/// One entry in a menu (internal form). A popup item carries `submenu` (a child
/// HMENU); a leaf carries `id` (the WM_COMMAND id sent when clicked).
#[derive(Clone)]
pub struct MenuItem {
    pub text: String,
    pub id: u32,
    pub submenu: Option<u32>,
    pub separator: bool,
    pub disabled: bool,
}

/// One dialog child control for the frontend layout pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogControlData {
    pub hwnd: u32,
    pub id: i32,
    pub class_name: String,
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub style: u32,
    pub enabled: bool,
    pub checked: bool,
    pub visible: bool,
}

/// A resolved menu node for the frontend (submenus expanded into `children`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItemData {
    pub text: String,
    pub id: u32,
    pub separator: bool,
    pub disabled: bool,
    pub children: Vec<MenuItemData>,
}

/// The user's answer to a modal dialog. `button` is a Win32 ID (IDOK=1,
/// IDCANCEL=2, IDABORT=3, IDRETRY=4, IDIGNORE=5, IDYES=6, IDNO=7); for a file
/// dialog `file` is the chosen path (None = cancelled).
#[derive(Debug, Clone)]
pub struct DialogReply {
    pub button: u32,
    pub file: Option<String>,
}

pub enum DDrawSurfaceKind {
    Primary,
    Offscreen,
}

pub struct DDrawSurface {
    pub kind: DDrawSurfaceKind,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixels_va: u32,
    pub color_key: Option<u32>,
    pub back_id: Option<u32>,
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
    /// Parent HWND for child controls (`None` = top-level).
    pub parent: Option<u32>,
    /// Control id (`GWLP_ID`); 0 for plain top-level windows.
    pub id: i32,
    pub class_name: String,
    pub title: String,
    pub style: u32,
    pub ex_style: u32,
    pub x: i32,
    pub y: i32,
    pub visible: bool,
    pub enabled: bool,
    pub children: Vec<u32>,
    /// Dialog procedure (`DWLP_DLGPROC`); only for dialogs.
    pub dlg_proc: Option<u32>,
    pub x_base_unit: u32,
    pub y_base_unit: u32,
    pub is_dialog: bool,
    /// BS_CHECKBOX / BS_RADIOBUTTON state.
    pub checked: bool,
    /// Result code from EndDialog (DialogBox* path).
    pub dlg_result: i32,
}

impl WindowEntry {
    pub fn new_toplevel(wndproc: u32, width: i32, height: i32, class_name: &str, title: &str) -> Self {
        WindowEntry {
            wndproc,
            needs_paint: true,
            width,
            height,
            pen_color: 0x00_0000,
            brush_color: 0xFF_FFFF,
            cur_x: 0,
            cur_y: 0,
            parent: None,
            id: 0,
            class_name: class_name.to_string(),
            title: title.to_string(),
            style: 0x00CF_0000, // WS_OVERLAPPEDWINDOW-ish
            ex_style: 0,
            x: 0,
            y: 0,
            visible: true,
            enabled: true,
            children: Vec::new(),
            dlg_proc: None,
            x_base_unit: 8,
            y_base_unit: 16,
            is_dialog: false,
            checked: false,
            dlg_result: 0,
        }
    }
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
            ddraw_display_w: 640,
            ddraw_display_h: 480,
            ddraw_display_bpp: 32,
            ddraw_surfaces: std::collections::HashMap::new(),
            next_ddraw_surface: 1,
            dialog_pending: false,
            dialog_reply: None,
            menus: std::collections::HashMap::new(),
            next_menu: MENU_TAG | 0x10,
            hwnd_menu: std::collections::HashMap::new(),
            dlg_ctrl: std::collections::HashMap::new(),
        }
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GuestProcess {
    pub pid: u32,
    pub path: String,
    pub image_base: u32,
    pub entry_point: u32,
    pub heap_base: u32,
    pub heap_next: u32,                                  // bump allocator pointer
    pub heap_sizes: std::collections::HashMap<u32, u32>, // ptr -> size, for realloc
    /// Coalesced free blocks `(ptr, size)` sorted by address — HeapFree reclaims
    /// here so games that alloc/read/free in a loop do not exhaust the bump heap.
    pub heap_free_list: Vec<(u32, u32)>,
    /// Exclusive upper bound the bump heap may grow to (DLL region base).
    pub heap_limit: u32,
    pub memory: GuestMemory,
    pub cpu: X86Cpu,
    pub handles: HandleTable,
    pub console: ConsoleStreams,
    pub ui_events: Vec<UiEvent>,
    pub gui: GuiState,
    pub spawns: Vec<SpawnRequest>,
    pub next_child_pid: u32, // pid the next CreateProcess child will receive
    pub state: ProcessState,
    // Current working directory, initialized to the launched image's directory
    // (matching how Explorer launches a process). Relative guest paths resolve
    // against this, and SetCurrentDirectory updates it.
    pub cwd: String,
    // Full command line, e.g. `"C:\...\doom.exe" -iwad doom1.wad`. Returned by
    // GetCommandLine and parsed into argv by the CRT startup.
    pub cmdline: String,
    // Message-table resource (id -> text) for FormatMessage(FROM_HMODULE), used by
    // cmd.exe and other system apps for their banner/messages.
    pub messages: std::collections::HashMap<u32, String>,
    // RT_STRING resources used by LoadStringA/W.
    pub strings: std::collections::HashMap<u32, String>,
    /// RT_DIALOG templates (resource id → raw template bytes).
    pub dialogs: std::collections::HashMap<u32, Vec<u8>>,
    /// RT_DIALOG by lowercase resource name (when named).
    pub dialogs_by_name: std::collections::HashMap<String, Vec<u8>>,
    // Managed (.NET/CLI) image bytes, set when this is a managed process. Such a
    // process has no meaningful x86 CPU state; it runs via the CLR interpreter.
    pub managed: Option<Vec<u8>>,
    // TLS slots: slot index -> value
    pub tls_slots: std::collections::HashMap<u32, u32>,
    pub next_tls: u32,
    // PRNG state for msvcrt rand()
    pub rand_seed: u32,
    /// Per-process storage for stateful built-in DLL shims.  Real DLL globals
    /// have process lifetime; keeping them here lets compatibility handlers
    /// return stable guest pointers across calls without hard-coded addresses.
    pub dll_state: std::collections::HashMap<String, u32>,
}

/// Directory portion of a guest path, e.g. `C:\a\b\foo.exe` -> `C:\a\b`.
pub fn parent_dir(path: &str) -> String {
    let p = path.replace('/', "\\");
    match p.rfind('\\') {
        Some(i) => {
            let d = &p[..i];
            if d.len() <= 2 {
                format!("{d}\\")
            } else {
                d.to_string()
            }
        }
        None => "C:\\".to_string(),
    }
}

impl GuestProcess {
    pub fn info(&self) -> ProcessInfo {
        let heap_used: u64 = self.heap_sizes.values().map(|&s| s as u64).sum();
        let heap_free: u64 = self.heap_free_list.iter().map(|&(_, s)| s as u64).sum();
        let mapped_bytes: u64 = self.memory.regions.values().map(|r| r.size as u64).sum();
        let heap_committed = self
            .heap_next
            .saturating_sub(self.heap_base) as u64;
        ProcessInfo {
            pid: self.pid,
            path: self.path.clone(),
            image_base: self.image_base,
            entry_point: self.entry_point,
            state: self.state.clone(),
            heap_used,
            heap_blocks: self.heap_sizes.len() as u32,
            heap_free,
            heap_free_blocks: self.heap_free_list.len() as u32,
            heap_committed,
            heap_limit: self.heap_limit.saturating_sub(self.heap_base) as u64,
            mapped_bytes,
            region_count: self.memory.regions.len() as u32,
        }
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
            heap_free_list: Vec::new(),
            heap_limit: 0,
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
            messages: std::collections::HashMap::new(),
            strings: std::collections::HashMap::new(),
            dialogs: std::collections::HashMap::new(),
            dialogs_by_name: std::collections::HashMap::new(),
            managed: Some(bytes),
            tls_slots: std::collections::HashMap::new(),
            next_tls: 1,
            rand_seed: 1,
            dll_state: std::collections::HashMap::new(),
        }
    }
}

pub struct ProcessTable {
    pub processes: indexmap::IndexMap<u32, GuestProcess>,
    next_pid: u32,
}

impl ProcessTable {
    pub fn new() -> Self {
        ProcessTable {
            processes: indexmap::IndexMap::new(),
            next_pid: 1,
        }
    }
    pub fn alloc_pid(&mut self) -> u32 {
        let p = self.next_pid;
        self.next_pid += 1;
        p
    }
    pub fn peek_next_pid(&self) -> u32 {
        self.next_pid
    }
    pub fn insert(&mut self, p: GuestProcess) {
        self.processes.insert(p.pid, p);
    }
    pub fn get(&self, pid: u32) -> Option<&GuestProcess> {
        self.processes.get(&pid)
    }
    pub fn get_mut(&mut self, pid: u32) -> Option<&mut GuestProcess> {
        self.processes.get_mut(&pid)
    }
    pub fn list_info(&self) -> Vec<ProcessInfo> {
        self.processes.values().map(|p| p.info()).collect()
    }

    /// Aggregate heap/mapped stats across all guest processes (Task Manager).
    pub fn system_memory(&self) -> SystemMemoryInfo {
        let processes = self.list_info();
        let total_heap_used = processes.iter().map(|p| p.heap_used).sum();
        let total_heap_free = processes.iter().map(|p| p.heap_free).sum();
        let total_heap_committed = processes.iter().map(|p| p.heap_committed).sum();
        let total_mapped = processes.iter().map(|p| p.mapped_bytes).sum();
        let heap_limit_per_process = processes
            .iter()
            .map(|p| p.heap_limit)
            .max()
            .unwrap_or(0x4000_0000); // ~1 GiB default layout
        SystemMemoryInfo {
            processes,
            total_heap_used,
            total_heap_free,
            total_heap_committed,
            total_mapped,
            heap_limit_per_process,
        }
    }
}

impl Default for ProcessTable {
    fn default() -> Self {
        Self::new()
    }
}
