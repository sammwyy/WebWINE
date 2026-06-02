# WebWINE Experimental Runtime Specification

## 1. Project Overview

**WebWINE** is an experimental browser-based Windows-like runtime designed to load, inspect, mount, and execute Windows PE `.exe` files directly inside a web application.

The project is not intended to be a full Wine replacement in its first stages. Instead, it is a progressive prototype that combines:

* A Rust-based virtual machine runtime compiled to WebAssembly.
* A browser frontend built with Vite and HTML/JavaScript or TypeScript.
* A virtual filesystem mounted around a Windows-like path model.
* A minimal virtual desktop interface.
* A process manager capable of loading and running multiple guest processes.
* A PE loader and x86 user-mode emulator.
* A Win32/NT API interception layer for future GUI and system API support.
* DOM-based stdout, stderr, runtime logs, and debug output.

The long-term vision is to emulate a small Windows-like environment inside the browser, where the user can upload `.exe` files and companion assets, place them into a virtual `C:\Users\guest\Desktop\` directory, and interact with them through a desktop-like UI.

## 2. Main Goal

The target experience is:

1. The user opens the WebWINE page.
2. The browser displays a minimal virtual desktop.
3. The user uploads one or more files.
4. Uploaded files are mounted into a virtual filesystem, initially under:

```txt
C:\Users\guest\Desktop\
```

5. The desktop UI renders icons based on the contents of the virtual Desktop folder.
6. The user double-clicks an icon.
7. Behavior depends on the file type:

```txt
.exe      -> load and execute as a guest process
folder    -> open a file explorer window
other     -> open a raw content viewer window
```

8. When a process starts, the frontend opens a floating window connected to that process.
9. The floating process window displays:

```txt
stdout
stderr
runtime logs
debug traces
process status
optional stdin input
```

10. The Rust/WASM runtime controls:

```txt
virtual memory
PE loading
x86 execution
Win32/NT API interception
virtual filesystem
process lifecycle
handles
threads, initially cooperative or simulated
```

## 3. High-Level Architecture

The application should be split into three primary layers:

```txt
Browser UI
  |
  | JavaScript / TypeScript API
  |
Web Worker Runtime Host
  |
  | wasm-bindgen bindings
  |
Rust WebWINE Runtime compiled to WASM
```

The frontend owns visual state and user interaction. The Rust/WASM runtime owns the virtual machine state.

### 3.1 Browser Frontend

The frontend is responsible for:

* Rendering the virtual desktop.
* Showing icons from the virtual Desktop directory.
* Handling drag-and-drop uploads.
* Handling file picker uploads.
* Opening floating windows.
* Rendering stdout, stderr, logs, and debug streams.
* Sending user actions to the WASM runtime.
* Displaying process state.
* Displaying raw file contents.
* Displaying directory contents in explorer windows.
* Managing DOM-level window layout.

The frontend should not implement PE loading, x86 execution, Win32 API behavior, or process emulation. Those belong inside the Rust runtime.

### 3.2 Web Worker Runtime Host

The WASM module should run inside a Web Worker.

Reasons:

* A guest process may run for a long time.
* A bug in the emulator should not freeze the UI thread.
* Runtime stepping and execution loops can be isolated.
* The main thread remains responsive.
* Future process scheduling can happen inside the Worker.

The Worker acts as the bridge between UI messages and the Rust runtime.

Example message flow:

```txt
Main Thread:
  user double-clicks "demo.exe"

Main Thread -> Worker:
  { type: "launch_process", path: "C:\\Users\\guest\\Desktop\\demo.exe" }

Worker -> WASM:
  vm.launch_process(path)

WASM -> Worker:
  process created, pid = 1

Worker -> Main Thread:
  { type: "process_started", pid: 1, path: "..." }

Main Thread:
  open process console window
```

### 3.3 Rust/WASM Runtime

The Rust runtime is the core of the project.

It should expose a clean API to JavaScript while keeping most internal logic private.

Core responsibilities:

* Virtual filesystem.
* Windows-like path normalization.
* PE parsing and loading.
* Guest virtual memory.
* x86 CPU state.
* Instruction decoding.
* Instruction interpretation.
* Import resolution.
* Win32/NT API interception.
* Process table.
* Handle table.
* Console streams.
* Runtime logs.
* Debug traces.
* Snapshots.
* Error reporting.

The runtime should be designed as a reusable Rust library first, and a WASM module second.

That means the core should be testable outside the browser:

```rust
let mut vm = WebWineVm::new();
vm.mount_file("C:\\Users\\guest\\Desktop\\hello.exe", bytes)?;
let pid = vm.launch_process("C:\\Users\\guest\\Desktop\\hello.exe")?;
vm.run_process_until_blocked(pid)?;
```

Then the same runtime can be compiled to WebAssembly and called from JavaScript.

## 4. Recommended Technology Stack

## 4.1 Rust Core

Recommended Rust crates:

```toml
[dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
serde_json = "1"
thiserror = "1"
anyhow = "1"
bitflags = "2"
indexmap = "2"
smallvec = "1"
bytes = "1"
goblin = "0.10"
iced-x86 = "1"
```

Optional later-stage crates:

```toml
[dependencies]
wasm-bindgen-futures = "0.4"
console_error_panic_hook = "0.1"
wee_alloc = "0.4"
tracing = "0.1"
tracing-subscriber = "0.3"
```

Optional native-only development crates:

```toml
[dev-dependencies]
proptest = "1"
insta = "1"
criterion = "0.5"
```

Potential filesystem design inspiration:

```toml
cap-std = "4"
```

However, `cap-std` may be more useful for native builds and architectural inspiration than for browser-only builds. The browser runtime should have its own virtual filesystem abstraction.

## 4.2 Frontend

Recommended frontend stack:

```txt
Vite
TypeScript
HTML/CSS
Web Workers
Native DOM APIs
```

A framework is optional. The first prototype can be implemented with plain TypeScript and DOM rendering.

Recommended packages:

```json
{
  "devDependencies": {
    "vite": "latest",
    "typescript": "latest"
  }
}
```

Optional later frontend packages:

```json
{
  "dependencies": {
    "xterm": "latest",
    "monaco-editor": "latest"
  }
}
```

The first version should avoid overbuilding the UI. A simple DOM-based virtual desktop is enough.

## 5. Repository Structure

Recommended monorepo layout:

```txt
webwine/
  Cargo.toml
  crates/
    webwine-core/
      Cargo.toml
      src/
        lib.rs
        vm.rs
        process.rs
        memory.rs
        cpu/
          mod.rs
          x86.rs
          flags.rs
          decoder.rs
          interpreter.rs
        pe/
          mod.rs
          loader.rs
          imports.rs
          relocations.rs
        winapi/
          mod.rs
          kernel32.rs
          ntdll.rs
          user32.rs
          gdi32.rs
          msvcrt.rs
        fs/
          mod.rs
          path.rs
          vfs.rs
          node.rs
        handles/
          mod.rs
          table.rs
        console.rs
        logs.rs
        error.rs

    webwine-wasm/
      Cargo.toml
      src/
        lib.rs
        bindings.rs

  web/
    package.json
    vite.config.ts
    index.html
    src/
      main.ts
      worker.ts
      wasm.ts
      desktop/
        desktop.ts
        icons.ts
        windows.ts
      fs/
        upload.ts
        explorer.ts
        raw-viewer.ts
      process/
        console-window.ts
        process-manager.ts
      styles/
        desktop.css
```

## 6. Rust Runtime Design

## 6.1 Main Runtime Object

The main Rust object should represent the whole VM:

```rust
pub struct WebWineVm {
    fs: VirtualFileSystem,
    processes: ProcessTable,
    memory_manager: MemoryManager,
    api: WinApiDispatcher,
    logs: LogBuffer,
    next_pid: u32,
}
```

The VM should expose methods such as:

```rust
impl WebWineVm {
    pub fn new() -> Self;

    pub fn mount_file(
        &mut self,
        guest_path: &str,
        bytes: Vec<u8>,
    ) -> Result<()>;

    pub fn create_dir(
        &mut self,
        guest_path: &str,
    ) -> Result<()>;

    pub fn list_dir(
        &self,
        guest_path: &str,
    ) -> Result<Vec<DirEntry>>;

    pub fn read_file(
        &self,
        guest_path: &str,
    ) -> Result<Vec<u8>>;

    pub fn launch_process(
        &mut self,
        guest_path: &str,
    ) -> Result<Pid>;

    pub fn step_process(
        &mut self,
        pid: Pid,
    ) -> Result<ProcessStepResult>;

    pub fn run_process_slice(
        &mut self,
        pid: Pid,
        instruction_budget: u32,
    ) -> Result<ProcessRunResult>;

    pub fn write_stdin(
        &mut self,
        pid: Pid,
        data: &[u8],
    ) -> Result<()>;

    pub fn drain_stdout(
        &mut self,
        pid: Pid,
    ) -> Result<Vec<u8>>;

    pub fn drain_stderr(
        &mut self,
        pid: Pid,
    ) -> Result<Vec<u8>>;

    pub fn drain_logs(
        &mut self,
    ) -> Vec<LogEvent>;
}
```

## 6.2 Process Model

Each guest process should be represented independently.

```rust
pub struct GuestProcess {
    pid: Pid,
    path: GuestPath,
    image_base: u32,
    entry_point: u32,
    memory: GuestMemory,
    cpu: X86Cpu,
    handles: HandleTable,
    console: ConsoleStreams,
    state: ProcessState,
}
```

Initial process states:

```rust
pub enum ProcessState {
    Created,
    Running,
    Blocked,
    WaitingForInput,
    Exited { code: u32 },
    Crashed { reason: String },
}
```

Each process should have its own:

```txt
virtual memory
CPU registers
stack
heap
handle table
stdout
stderr
stdin
loaded modules
```

The VM can later add shared kernel objects between processes.

## 6.3 Execution Model

The first version should use a cooperative execution model.

Instead of running forever, the runtime should execute a limited instruction budget:

```rust
vm.run_process_slice(pid, 10_000)?;
```

This avoids blocking the Worker and makes the runtime easier to debug.

The Worker can run a loop:

```ts
while (processIsRunning) {
  const result = wasm.runProcessSlice(pid, 10_000);
  flushProcessOutput(pid);
  postProcessEventsToMainThread(pid, result);

  if (result.state !== "running") {
    break;
  }

  await nextMicrotaskOrTimeout();
}
```

## 7. Virtual Filesystem

## 7.1 Goals

The virtual filesystem should provide a Windows-like filesystem model inside the Rust runtime.

Default root layout:

```txt
C:\
C:\Users\
C:\Users\guest\
C:\Users\guest\Desktop\
C:\Windows\
C:\Windows\System32\
C:\Temp\
```

Uploads should be mounted into:

```txt
C:\Users\guest\Desktop\
```

Example:

```txt
uploaded demo.exe
  -> C:\Users\guest\Desktop\demo.exe

uploaded readme.txt
  -> C:\Users\guest\Desktop\readme.txt

uploaded assets/logo.bmp
  -> C:\Users\guest\Desktop\assets\logo.bmp
```

## 7.2 VFS Node Types

```rust
pub enum VfsNode {
    Directory(VfsDirectory),
    File(VfsFile),
}
```

```rust
pub struct VfsFile {
    name: String,
    bytes: Vec<u8>,
    created_at: u64,
    modified_at: u64,
}
```

```rust
pub struct VfsDirectory {
    name: String,
    children: IndexMap<String, VfsNode>,
}
```

## 7.3 Path Rules

The VFS should normalize paths such as:

```txt
C:/Users/guest/Desktop/demo.exe
C:\Users\guest\Desktop\demo.exe
c:\users\guest\desktop\demo.exe
.\demo.exe
..\Desktop\demo.exe
```

into canonical internal paths.

Recommended internal representation:

```rust
pub struct GuestPath {
    drive: char,
    components: Vec<String>,
}
```

Rules:

```txt
case-insensitive lookup
preserve original display name
support backslash and slash as separators
reject paths escaping mounted roots
support C:\ drive initially
reserve support for D:\, Z:\, virtual CD-ROMs, and mounted archives later
```

## 7.4 Filesystem Persistence

Initial version:

```txt
in-memory only
```

Later versions:

```txt
IndexedDB persistence
OPFS persistence
export/import filesystem snapshot
drag-and-drop folder import
```

The Rust runtime should not directly depend on browser storage APIs. Instead, persistence should be controlled by the JS/TS host.

Recommended model:

```txt
Rust VFS:
  owns current filesystem state

JS Host:
  can export snapshot from Rust
  can save snapshot to IndexedDB or OPFS
  can reload snapshot into Rust
```

## 8. PE Loader

## 8.1 Initial Target

The first PE loader should support:

```txt
PE32
x86
console subsystem
.EXE files
little-endian memory
basic sections
entry point
image base
section mapping
import table
relocations
basic TLS stubs
```

Not required in the first version:

```txt
PE32+
x86_64
.NET assemblies
drivers
kernel-mode PE files
packed malware
self-modifying code
full SEH
full TLS callback behavior
full Windows loader compatibility
```

## 8.2 PE Loading Flow

Expected loading sequence:

```txt
read file from VFS
parse PE headers
validate machine type
validate subsystem
allocate image memory
map headers
map sections
apply relocations if needed
resolve imports
create initial stack
create fake PEB/TEB
initialize CPU registers
set EIP to entry point
mark process as Created or Running
```

## 8.3 Suggested Crate

Use `goblin` initially for PE parsing.

The runtime should wrap `goblin` behind its own abstraction:

```rust
pub struct PeImage {
    pub image_base: u32,
    pub entry_point_rva: u32,
    pub sections: Vec<PeSection>,
    pub imports: Vec<PeImport>,
    pub relocations: Vec<PeRelocation>,
}
```

This avoids leaking the parser crate into the rest of the emulator.

If `goblin` becomes limiting later, the internal `PeImage` abstraction allows replacing it with a custom parser.

## 9. x86 CPU Emulation

## 9.1 Initial CPU Scope

Initial target:

```txt
x86 32-bit protected user mode
integer instructions
basic stack operations
control flow
calls and returns
memory addressing modes
EFLAGS
basic string instructions
basic FPU/SSE stubs only when needed
```

Initial CPU state:

```rust
pub struct X86Cpu {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub eip: u32,
    pub eflags: u32,
}
```

## 9.2 Instruction Decoder

Use `iced-x86` for decoding and disassembly.

The emulator should still define its own execution layer:

```rust
pub enum DecodedOp {
    Mov,
    Push,
    Pop,
    Add,
    Sub,
    Call,
    Ret,
    Jmp,
    Jcc,
    Cmp,
    Test,
    Xor,
    And,
    Or,
    Lea,
    Int,
    SyscallLikeTrap,
    Unsupported,
}
```

The decoder layer should translate `iced-x86` instructions into internal execution operations where practical.

## 9.3 Execution Strategy

Start with a pure interpreter.

Do not start with JIT or dynamic recompilation.

Execution loop:

```rust
loop {
    let eip = cpu.eip;
    let bytes = memory.read_instruction_window(eip)?;
    let instruction = decoder.decode(bytes)?;
    let trap = interpreter.execute(&instruction, cpu, memory)?;

    if let Some(trap) = trap {
        return handle_trap(trap);
    }
}
```

The first performance improvement should be a decoded basic-block cache:

```txt
EIP -> Vec<DecodedInstruction>
```

Dynamic recompilation to WASM can be explored later, after correctness is established.

## 10. Guest Memory Model

Each process should own a virtual address space.

The memory model should support:

```txt
allocate
free
read
write
read_u8/u16/u32
write_u8/u16/u32
page permissions
image mapping
stack mapping
heap mapping
guard pages later
```

Recommended memory layout for early prototype:

```txt
0x00400000  default PE image base
0x10000000  heap
0x70000000  stack top
0x7FF00000  fake PEB/TEB/kernel structures
```

Example structure:

```rust
pub struct GuestMemory {
    regions: Vec<MemoryRegion>,
}
```

```rust
pub struct MemoryRegion {
    base: u32,
    size: u32,
    protection: PageProtection,
    bytes: Vec<u8>,
}
```

Page protection:

```rust
bitflags! {
    pub struct PageProtection: u32 {
        const READ    = 0b0001;
        const WRITE   = 0b0010;
        const EXECUTE = 0b0100;
    }
}
```

For the first prototype, memory can be region-based rather than true page-table-based.

## 11. Win32 and NT API Interception

## 11.1 Purpose

The Win32/NT API interception layer is one of the most important long-term parts of WebWINE.

Guest programs usually do not interact with the operating system through raw CPU instructions only. They call imported functions from DLLs such as:

```txt
kernel32.dll
ntdll.dll
user32.dll
gdi32.dll
advapi32.dll
msvcrt.dll
ucrtbase.dll
shell32.dll
comdlg32.dll
```

The runtime should intercept these calls and implement them in Rust.

## 11.2 Import Resolution Strategy

During PE loading, imports should be resolved to fake trampoline addresses.

Example:

```txt
guest imports:
  kernel32.dll!WriteFile

runtime creates:
  fake address 0x7FFE1000 -> ApiId::Kernel32_WriteFile

IAT entry receives:
  0x7FFE1000
```

When guest code calls that address, the emulator traps into Rust:

```txt
CALL DWORD PTR [WriteFile]
  -> EIP = 0x7FFE1000
  -> emulator detects API trampoline
  -> WinApiDispatcher handles Kernel32_WriteFile
```

## 11.3 API Dispatcher

```rust
pub struct WinApiDispatcher {
    trampolines: HashMap<u32, ApiId>,
}
```

```rust
pub enum ApiId {
    Kernel32_GetStdHandle,
    Kernel32_WriteFile,
    Kernel32_ReadFile,
    Kernel32_ExitProcess,
    Kernel32_CreateFileA,
    Kernel32_CreateFileW,
    Kernel32_CloseHandle,
    Kernel32_GetLastError,
    Kernel32_SetLastError,
    Kernel32_HeapAlloc,
    Kernel32_HeapFree,
    Kernel32_GetProcessHeap,

    Ntdll_RtlAllocateHeap,
    Ntdll_RtlFreeHeap,
    Ntdll_NtClose,

    Msvcrt_Printf,
    Msvcrt_Puts,
    Msvcrt_Exit,

    User32_MessageBoxA,
    User32_MessageBoxW,

    Unsupported {
        dll: String,
        name: String,
    },
}
```

## 11.4 Initial API Support

Minimum APIs for console programs:

```txt
kernel32.dll:
  GetStdHandle
  WriteFile
  ReadFile
  ExitProcess
  GetLastError
  SetLastError
  CreateFileA
  CreateFileW
  CloseHandle
  GetProcessHeap
  HeapAlloc
  HeapFree
  VirtualAlloc
  VirtualFree
  VirtualProtect
  GetModuleHandleA
  GetProcAddress

msvcrt.dll:
  printf
  puts
  putchar
  getchar
  exit
  malloc
  free
  memcpy
  memset
  strlen
  strcmp

ntdll.dll:
  RtlAllocateHeap
  RtlFreeHeap
  RtlZeroMemory
  NtClose
```

## 11.5 Future GUI API Interception

The project should explicitly reserve a future Win32 UI layer.

Future DLL targets:

```txt
user32.dll
gdi32.dll
comctl32.dll
shell32.dll
ole32.dll
comdlg32.dll
```

Initial GUI interception can be high-level and DOM-backed.

Example:

```txt
MessageBoxA
  -> open browser modal or WebWINE floating dialog

CreateWindowExA
  -> create virtual Win32 window object
  -> create matching DOM floating window

SetWindowTextA
  -> update DOM title

ShowWindow
  -> show/hide DOM window

DestroyWindow
  -> close DOM window

GetMessageA / PeekMessageA
  -> read from WebWINE message queue

DispatchMessageA
  -> call guest window procedure if supported

BeginPaint / EndPaint
  -> provide drawing context abstraction

TextOutA
  -> draw text into canvas

BitBlt
  -> future canvas-backed implementation
```

The first GUI milestone should not attempt full Windows compatibility.

Instead, it should expose a minimal mapping:

```txt
Win32 window handle HWND
  -> Rust window object
    -> frontend DOM window id
```

Potential GUI object model:

```rust
pub struct GuestWindow {
    hwnd: u32,
    owner_pid: Pid,
    title: String,
    class_name: String,
    rect: Rect,
    visible: bool,
    dom_window_id: String,
}
```

The Rust runtime should emit UI events:

```rust
pub enum UiEvent {
    CreateWindow {
        hwnd: u32,
        title: String,
        width: u32,
        height: u32,
    },
    SetWindowTitle {
        hwnd: u32,
        title: String,
    },
    ShowWindow {
        hwnd: u32,
    },
    HideWindow {
        hwnd: u32,
    },
    DestroyWindow {
        hwnd: u32,
    },
    MessageBox {
        owner: Option<u32>,
        title: String,
        text: String,
    },
}
```

The frontend then renders these events into DOM windows.

## 12. Handles

The runtime needs a Windows-like handle table.

Handles should be used for:

```txt
files
directories
console stdin
console stdout
console stderr
processes
threads
future windows
future events
future mutexes
future registry keys
```

Example:

```rust
pub enum KernelObject {
    File(VfsFileHandle),
    Directory(VfsDirectoryHandle),
    ConsoleInput(Pid),
    ConsoleOutput(Pid),
    Process(Pid),
    Thread(ThreadId),
    Window(Hwnd),
}
```

```rust
pub struct HandleTable {
    next_handle: u32,
    objects: HashMap<u32, KernelObject>,
}
```

Reserved handles:

```txt
STD_INPUT_HANDLE   = -10
STD_OUTPUT_HANDLE  = -11
STD_ERROR_HANDLE   = -12
```

`GetStdHandle` should return guest handles that map to process console streams.

## 13. Frontend UI Specification

## 13.1 Visual Style

The UI should be minimalistic.

Initial layout:

```txt
full browser viewport
desktop background
desktop icon grid
floating windows
simple taskbar optional
DOM log/debug panel optional
```

No complex CSS framework is required.

## 13.2 Desktop Grid

The desktop should render the contents of:

```txt
C:\Users\guest\Desktop\
```

Each VFS entry becomes an icon.

Icon rules:

```txt
.exe       -> executable icon
folder     -> folder icon
.txt/.log  -> text file icon
other      -> generic file icon
```

Double-click behavior:

```txt
.exe:
  launch process
  open process console window

folder:
  open explorer window

other:
  open raw viewer window
```

## 13.3 File Upload

The frontend should support:

```txt
file picker
drag and drop
multi-file upload
future folder upload
```

Default mount target:

```txt
C:\Users\guest\Desktop\
```

For every uploaded file:

```ts
await runtime.mountFile(
  `C:\\Users\\guest\\Desktop\\${file.name}`,
  await file.arrayBuffer()
);
```

After upload, refresh desktop:

```ts
const entries = await runtime.listDir("C:\\Users\\guest\\Desktop\\");
renderDesktop(entries);
```

## 13.4 Process Console Window

When a `.exe` is launched, the frontend should create a floating window.

Window contents:

```txt
title bar:
  process name + pid + status

body:
  stdout stream
  stderr stream
  runtime logs
  debug events

footer:
  stdin input box
  send button
  stop button
```

Example title:

```txt
demo.exe - PID 1 - Running
```

The first version can simply append text logs into a `<pre>` element.

## 13.5 Raw File Viewer

For non-executable files, open a raw viewer window.

Initial behavior:

```txt
try decode as UTF-8
if valid text:
  show text
else:
  show hex preview
```

Future behavior:

```txt
image preview
audio preview
structured PE viewer
JSON viewer
binary hex editor
```

## 13.6 Explorer Window

For folders, open an explorer window showing directory contents.

Required features:

```txt
path bar
file/folder list
double-click navigation
double-click executable launch
double-click raw viewer
back button optional
```

## 14. JavaScript/WASM API

The WASM package should expose a class-like API.

Example TypeScript-facing API:

```ts
export class WebWineRuntime {
  constructor();

  initDefaultFilesystem(): void;

  mountFile(path: string, bytes: Uint8Array): void;

  createDirectory(path: string): void;

  listDirectory(path: string): DirectoryEntry[];

  readFile(path: string): Uint8Array;

  launchProcess(path: string): number;

  runProcessSlice(pid: number, instructionBudget: number): ProcessRunResult;

  writeProcessStdin(pid: number, bytes: Uint8Array): void;

  drainProcessStdout(pid: number): Uint8Array;

  drainProcessStderr(pid: number): Uint8Array;

  drainLogs(): LogEvent[];

  getProcessList(): ProcessInfo[];

  killProcess(pid: number): void;
}
```

Data returned to JavaScript should use serializable structures.

Example:

```ts
export interface DirectoryEntry {
  name: string;
  path: string;
  kind: "file" | "directory";
  size: number;
  executable: boolean;
}
```

```ts
export interface ProcessInfo {
  pid: number;
  path: string;
  state: "created" | "running" | "blocked" | "waiting_for_input" | "exited" | "crashed";
  exitCode?: number;
  crashReason?: string;
}
```

```ts
export interface LogEvent {
  level: "trace" | "debug" | "info" | "warn" | "error";
  target: string;
  message: string;
  pid?: number;
}
```

## 15. Worker Protocol

The Worker should expose an asynchronous message protocol.

Example messages from main thread to Worker:

```ts
type RuntimeRequest =
  | { type: "init" }
  | { type: "mount_file"; path: string; bytes: ArrayBuffer }
  | { type: "list_dir"; path: string; requestId: string }
  | { type: "read_file"; path: string; requestId: string }
  | { type: "launch_process"; path: string; requestId: string }
  | { type: "write_stdin"; pid: number; text: string }
  | { type: "kill_process"; pid: number };
```

Example messages from Worker to main thread:

```ts
type RuntimeEvent =
  | { type: "ready" }
  | { type: "dir_list"; requestId: string; entries: DirectoryEntry[] }
  | { type: "file_data"; requestId: string; path: string; bytes: ArrayBuffer }
  | { type: "process_started"; pid: number; path: string }
  | { type: "process_stdout"; pid: number; text: string }
  | { type: "process_stderr"; pid: number; text: string }
  | { type: "process_log"; pid?: number; level: string; message: string }
  | { type: "process_exited"; pid: number; exitCode: number }
  | { type: "process_crashed"; pid: number; reason: string }
  | { type: "ui_event"; event: GuestUiEvent };
```

The Worker should run process slices and periodically flush output.

## 16. Logging and Debugging

The first UI should show logs in the DOM.

Log categories:

```txt
loader
pe
memory
cpu
api
fs
process
frontend
worker
```

Example logs:

```txt
[loader] loaded C:\Users\guest\Desktop\demo.exe
[pe] image_base=0x00400000 entry=0x00401230
[api] kernel32.WriteFile(handle=stdout, len=13)
[stdout] Hello world!
[process] pid=1 exited with code 0
```

The Rust runtime should keep a ring buffer of log events.

```rust
pub struct LogBuffer {
    events: VecDeque<LogEvent>,
    max_events: usize,
}
```

The JS side can call:

```ts
const logs = runtime.drainLogs();
appendLogsToDom(logs);
```

## 17. Security Model

WebWINE must treat all guest executables as untrusted.

Security principles:

```txt
guest code never executes natively
guest code runs only inside the emulator
guest filesystem is virtual
guest network access is disabled by default
guest browser DOM access is impossible
guest JavaScript execution is impossible
guest process cannot access host files
all host capabilities must be explicit
```

The browser sandbox already helps, but the runtime should not rely only on the browser.

The Rust VM should enforce:

```txt
no direct host filesystem access
no direct browser API access from guest
no raw JS eval
no native plugins
bounded execution slices
bounded memory
bounded file size
bounded process count
bounded log size
```

Recommended default limits:

```txt
max uploaded file size: configurable, e.g. 64 MB
max VFS size: configurable, e.g. 256 MB
max process memory: configurable, e.g. 128 MB
max processes: configurable, e.g. 16
max instruction budget per slice: configurable
```

## 18. Build System

## 18.1 Rust WASM Build

Use:

```txt
wasm-pack
wasm-bindgen
wasm32-unknown-unknown target
```

Example commands:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build crates/webwine-wasm --target web --out-dir ../../web/src/pkg
```

## 18.2 Frontend Build

Use Vite:

```bash
cd web
npm install
npm run dev
```

## 18.3 Suggested package.json

```json
{
  "scripts": {
    "dev": "vite",
    "build:wasm": "wasm-pack build ../crates/webwine-wasm --target web --out-dir ../web/src/pkg",
    "build": "npm run build:wasm && vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "typescript": "latest",
    "vite": "latest"
  }
}
```

## 19. MVP Scope

## 19.1 MVP Frontend

The MVP should include:

```txt
Vite app
virtual desktop
desktop icon grid
file upload
mount uploaded files into C:\Users\guest\Desktop\
double-click behavior
floating windows
process console window
raw file viewer
folder explorer
DOM logs
Worker-based WASM runtime
```

## 19.2 MVP Rust Runtime

The MVP runtime should include:

```txt
in-memory VFS
Windows-like path handling
file mounting
directory listing
raw file reading
process table
basic PE parser wrapper
PE32 validation
initial process object
console streams
log buffer
stub x86 CPU object
stub execution loop
API dispatcher skeleton
```

The first MVP does not need to successfully run complex Windows executables.

A valid first execution target could be a tiny hand-selected PE32 console program that calls only:

```txt
GetStdHandle
WriteFile
ExitProcess
```

## 20. Milestones

## Milestone 1: Browser Desktop and VFS

Deliverables:

```txt
Vite frontend
virtual desktop UI
file upload
in-memory VFS in Rust
WASM bridge
list Desktop directory
render icons
raw file viewer
folder explorer
DOM logs
```

Goal:

```txt
Users can upload files and browse them inside a Windows-like virtual desktop.
```

## Milestone 2: PE Inspection

Deliverables:

```txt
PE parser integration
detect PE32 executables
show PE metadata
sections
imports
entry point
image base
subsystem
machine type
```

Goal:

```txt
Double-clicking an .exe opens a process/debug window and shows PE loading information.
```

## Milestone 3: Minimal PE Loader

Deliverables:

```txt
map PE image into guest memory
map sections
apply relocations
resolve imports to fake trampolines
create stack
create fake PEB/TEB placeholders
initialize EIP/ESP
```

Goal:

```txt
The runtime can prepare a PE32 executable for interpretation.
```

## Milestone 4: x86 Interpreter

Deliverables:

```txt
iced-x86 decoder
basic instruction interpreter
register state
EFLAGS
memory reads/writes
call/ret/jmp/jcc
stack operations
debug stepping
```

Goal:

```txt
The runtime can step through simple PE32 user-mode code.
```

## Milestone 5: Console API Support

Deliverables:

```txt
kernel32.GetStdHandle
kernel32.WriteFile
kernel32.ReadFile
kernel32.ExitProcess
basic msvcrt output helpers
stdout/stderr connected to DOM
stdin connected to process window
```

Goal:

```txt
A simple console .exe can print text into a floating DOM window.
```

## Milestone 6: Process Management

Deliverables:

```txt
multiple process table
process states
kill process
run slices
stdout/stderr per process
logs per process
basic handle table
```

Goal:

```txt
Multiple guest processes can exist and be controlled independently.
```

## Milestone 7: Filesystem APIs

Deliverables:

```txt
CreateFileA/W
ReadFile
WriteFile
CloseHandle
GetFileSize
SetFilePointer
DeleteFile optional
FindFirstFile/FindNextFile optional
```

Goal:

```txt
Guest programs can read and write files inside the virtual C:\ filesystem.
```

## Milestone 8: Early Win32 UI Interception

Deliverables:

```txt
MessageBoxA/W
CreateWindowExA/W skeleton
ShowWindow
SetWindowTextA/W
DestroyWindow
basic HWND table
runtime-to-frontend UI events
DOM-backed guest windows
```

Goal:

```txt
Simple GUI API calls can create browser-rendered windows or dialogs.
```

## Milestone 9: Persistent Virtual Disk

Deliverables:

```txt
VFS snapshot export
VFS snapshot import
IndexedDB or OPFS storage
restore desktop state after reload
```

Goal:

```txt
The virtual C:\ drive can persist between browser sessions.
```

## 21. Non-Goals for Early Versions

Early versions should not attempt:

```txt
full Wine compatibility
kernel-mode drivers
real Windows syscalls
x86_64 support
DirectX
OpenGL passthrough
COM/OLE compatibility
networking
registry compatibility
anti-debug bypasses
malware-focused behavior
full GDI rendering
full user32 message pump compatibility
```

These can be explored later, after the core runtime is reliable.

## 22. Testing Strategy

## 22.1 Rust Unit Tests

Test:

```txt
path normalization
VFS operations
PE parsing wrappers
memory read/write
page protection
instruction decoding
individual instruction behavior
API dispatcher behavior
handle table behavior
```

## 22.2 Golden Tests

Use tiny PE samples with known behavior.

Example expected output:

```txt
hello_writefile.exe
  stdout: "hello from WebWINE\n"
  exit code: 0
```

## 22.3 Browser Tests

Test:

```txt
upload file
desktop refresh
open raw viewer
open explorer
launch fake process
receive stdout
receive exit event
```

## 23. Suggested First Demo

The first public demo should not claim broad compatibility.

Recommended demo:

```txt
1. Open WebWINE in the browser.
2. Upload hello.exe.
3. The file appears on C:\Users\guest\Desktop\.
4. Double-click hello.exe.
5. A process window opens.
6. Logs show PE loading.
7. Runtime intercepts kernel32.WriteFile.
8. The process window prints:
   Hello from WebWINE!
9. Runtime intercepts ExitProcess.
10. The process exits with code 0.
```

This demonstrates the full vertical slice:

```txt
frontend upload
VFS mount
desktop rendering
double-click launch
PE loading
x86 execution
Win32 API interception
stdout to DOM
process lifecycle
```

## 24. Design Philosophy

WebWINE should be built progressively.

Correctness and observability are more important than speed in the early stages.

The first versions should favor:

```txt
simple interpreter over JIT
clear logs over silent behavior
small supported API surface over broad fake compatibility
deterministic process slices over uncontrolled execution
well-defined VFS over direct browser storage coupling
explicit frontend events over hidden DOM access
```

The long-term architecture should leave room for:

```txt
faster instruction execution
basic block caching
dynamic recompilation
more complete Win32 APIs
DOM-backed GUI windows
Canvas-backed GDI rendering
persistent virtual disks
snapshots
debugger UI
registry emulation
multi-process coordination
```

## 25. Final Target

The final target is a browser-based experimental Windows-like environment where:

```txt
the user sees a desktop
files live under C:\Users\guest\Desktop\
executables can be launched by double-clicking
each process has its own floating console/debug window
non-executable files can be inspected
folders can be browsed
the Rust runtime owns the VM
the JS/TS frontend owns the UI
the VFS is controlled across JS <-> WASM <-> Rust
Win32 APIs are intercepted and progressively implemented
future GUI APIs can map to DOM-backed floating windows
```

WebWINE should be treated as a VM-like runtime library exposed to the browser, not merely as a frontend demo.

The core product is the Rust runtime.

The browser desktop is the first user-facing shell around it.
