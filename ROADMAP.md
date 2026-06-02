# WebWINE Roadmap

A browser-based Windows-like runtime. Rust/WASM owns the VM. TypeScript owns the UI.

## Milestone 1 — Browser Desktop and VFS
**Goal:** Upload files, browse them inside a virtual desktop.

- Cargo workspace with `webwine-core` and `webwine-wasm` crates
- In-memory VFS with Windows path normalization
- WASM bindings exposing VFS to JavaScript
- Vite + TypeScript frontend
- Virtual desktop with icon grid
- File upload via picker and drag-and-drop
- Floating windows (raw file viewer, folder explorer)
- DOM log panel
- Web Worker runtime host

## Milestone 2 — PE Inspection
**Goal:** Double-clicking an .exe shows PE metadata in the process window.

- Integrate `goblin` for PE parsing
- Detect PE32 x86 executables vs other files
- Extract: image base, entry point, sections, import table, machine type, subsystem
- Display PE info in a dedicated inspector window
- Log PE loading events to DOM log panel

## Milestone 3 — Minimal PE Loader
**Goal:** Runtime can prepare a PE32 executable for interpretation.

- Map PE image into `GuestMemory`
- Map individual sections with correct RVA offsets
- Apply base relocations
- Resolve imports to fake trampoline addresses via `WinApiDispatcher`
- Allocate stack and heap regions
- Write fake PEB/TEB placeholder bytes
- Initialize CPU register state (EIP, ESP)
- Mark process as `Created`

## Milestone 4 — x86 Interpreter
**Goal:** Runtime can step through simple PE32 user-mode code.

- Integrate `iced-x86` decoder
- `X86Cpu` register file and EFLAGS
- Interpreter for: MOV, PUSH, POP, ADD, SUB, AND, OR, XOR, CMP, TEST, LEA
- Control flow: JMP, Jcc, CALL, RET
- String ops: MOVS, STOS, SCAS, REP prefix
- Memory addressing modes
- API trampoline trap detection
- Instruction budget / cooperative slice execution
- Debug step output to log buffer

## Milestone 5 — Console API Support
**Goal:** A simple console .exe can print text to a floating DOM window.

- `kernel32.GetStdHandle` returning process console handles
- `kernel32.WriteFile` routing to stdout/stderr console streams
- `kernel32.ReadFile` blocking on stdin queue
- `kernel32.ExitProcess` terminating process cleanly
- `msvcrt.printf`, `puts`, `putchar` — call through to WriteFile
- `msvcrt.malloc`/`free` backed by guest heap
- `msvcrt.memcpy`, `memset`, `strlen`, `strcmp`
- Stdin input box in process console window wired to worker
- Stdout/stderr streamed to DOM in real time

## Milestone 6 — Process Management
**Goal:** Multiple guest processes exist and are controlled independently.

- Process table supporting multiple concurrent `GuestProcess` entries
- Per-process stdout, stderr, stdin, log streams
- Process state machine: Created → Running → Blocked / Exited / Crashed
- `killProcess` from frontend
- Worker runs slices across all running processes in round-robin
- Per-process console window with title showing name + PID + state
- Handle table with STD_INPUT/OUTPUT/ERROR reserved handles
- `kernel32.GetLastError` / `SetLastError` per-process

## Milestone 7 — Filesystem APIs
**Goal:** Guest programs can read and write files inside virtual C:\.

- `kernel32.CreateFileA` / `CreateFileW`
- `kernel32.ReadFile` on VFS file handles
- `kernel32.WriteFile` on VFS file handles
- `kernel32.CloseHandle` for file handles
- `kernel32.GetFileSize`
- `kernel32.SetFilePointer`
- File handle objects in handle table tracking VFS path + cursor
- Files created by guest visible in desktop explorer

## Milestone 8 — Early Win32 UI Interception
**Goal:** Simple GUI calls produce browser-rendered windows or dialogs.

- `user32.MessageBoxA` / `MessageBoxW` → browser modal / WebWINE floating dialog
- `user32.CreateWindowExA` / `CreateWindowExW` → virtual HWND + DOM window
- `user32.ShowWindow` / `DestroyWindow`
- `user32.SetWindowTextA` / `SetWindowTextW`
- HWND table in runtime
- `UiEvent` enum emitted from Rust → forwarded via Worker → rendered by frontend
- Basic message loop stubs: `GetMessageA`, `PeekMessageA`, `DispatchMessageA`
- DOM-backed guest window with title bar and client area canvas

## Milestone 9 — Persistent Virtual Disk
**Goal:** Virtual C:\ survives a browser reload.

- VFS snapshot: serialize to `Vec<u8>` (bincode or custom format)
- Expose `exportSnapshot` / `importSnapshot` on WASM API
- JS host saves/loads snapshot via OPFS (`navigator.storage.getDirectory`)
- Auto-save on file mount and on process exit
- Auto-restore on page load
- Export-to-file and import-from-file buttons in UI
