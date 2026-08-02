# WebWINE Roadmap

A browser-based Windows-like runtime. Rust/WASM owns the VM. TypeScript owns the UI.

## Status

- M1 Browser Desktop and VFS — **done**
- M2 PE Inspection — **done**
- M3 Minimal PE Loader — **done**
- M4 x86 Interpreter — **done** (integer + SSE/x87 stubs, CMPXCHG/XADD, atomics)
- M5 Console API Support — **done** (real MSVC UCRT binaries run: `_initterm`, TLS, NtWriteFile)
- M6 Process Management — **done** (per-process state/streams/handles, kill; CreateProcessA/W spawns concurrent child guest processes, each with its own console; GetExitCodeProcess/WaitForSingleObject)
- M7 Filesystem APIs — **done** (CreateFile/Read/Write/CloseHandle/GetFileSize/SetFilePointer/CreateDirectory/DeleteFile on the VFS; guest-created files appear on the desktop)
- M8 Early Win32 UI — **done** (MessageBox dialogs; RegisterClass/CreateWindowEx → real windows; working message loop GetMessage/DispatchMessage with WndProc callbacks; WM_PAINT → TextOut rendering; close → WM_CLOSE/WM_DESTROY/WM_QUIT)
- M9 Persistent Virtual Disk — pending

## Compatibility smoke runner

The native CLI can sweep one executable or an entire directory tree. Every EXE
runs in a fresh VM and the report includes architecture, terminal state,
instruction/UI counts, elapsed time, failure detail, and the most frequent
unimplemented APIs:

```powershell
cargo run -p webwine-cli -- --smoke=samples/build --max=100
cargo run -p webwine-cli -- --smoke=samples/vendored/notepad.exe --max=100
```

Current priority-app baseline:

- `samples/vendored/notepad.exe`: managed .NET/WinForms entry point reaches an
  interactive browser window.
- `samples/vendored/Notepad/notepad-nt.exe`: native Win32 Notepad loads its
  `RT_STRING` resources, creates its windows, and reaches `GetMessage` with no
  unimplemented API calls on the startup path.
- `samples/vendored/mspaint.exe`: MFC/CRT initialization is stack-safe and an
  MFC host-window bridge keeps Paint interactive. The full MFC CWnd/CFrameWnd
  object model and actual Paint command/canvas behavior remain future work.
- All executables in `samples/build` reach an interactive state with zero
  unimplemented calls in the smoke startup path.

Real-binary findings (32-bit i386):
- 32-bit MSVCRT console stubs (e.g. a Win10 `calc.exe` launcher) now get through CRT
  init (`__wgetmainargs`, `_onexit`, SHELL32 stubs implemented).
- `filesystem.exe` (Rust std fs) reaches its path-canonicalization (`maybe_verbatim`,
  prepending `\\?\`) then writes through a `Vec<u16>` whose data pointer is the
  empty-vec dangling value (2) — a capacity/reserve consistency bug in the emulated
  std internals. Not yet root-caused; needs instruction-level comparison vs hardware.
- `mspaint.exe` imports hundreds of ordinal-only **MFC42u.dll** functions. Its
  startup state, constructors, and `AfxWinMain` boundary are now modeled enough
  to present a host window; complete MFC widget behavior is still a large layer.

This remains the Wine-class long road. No-CRT, `println!`-style CRT console, Win32
GUI, GDI graphics, and multi-process binaries run end to end.

Toward real apps: the realistic path is incremental fidelity (CPU edge cases, broader
+ more accurate Win32/CRT surface, GDI→canvas), driven by specific target binaries.

Architecture limit: WebWINE is a 32-bit x86 (i386) interpreter. **Modern Windows
system binaries (calc/cmd/mspaint on Win10) are x86-64** and are now rejected with a
clear message instead of crashing. 32-bit (XP-era / SysWOW64 / 32-bit-built) PE32
images are the supported target. True x64 support would require a separate 64-bit
interpreter — a large future effort.

Extras now working: GDI raster drawing (Rectangle/Ellipse/LineTo/SetPixel/FillRect)
rendered to a per-window canvas, system beeps (Beep/MessageBeep) via Web Audio,
DirectDraw window/framebuffer paths, and a D3D8 command-stream foundation.
Broad modern DirectX and OpenGL coverage remains incomplete.

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

## Milestone 10 — CLI Metadata Reader
**Goal:** Parse a managed assembly's structure instead of rejecting it.

- Parse the COR20 (CLI) header from data directory 14
- Parse the metadata root (`BSJB`), version string, and stream headers
- Decode the `#~` tables stream header: heap-size flags, valid/sorted bitmasks, row counts
- Heaps: `#Strings`, `#US` (user strings), `#Blob`, `#GUID`
- Decode the metadata tables needed for execution: Module, TypeRef, TypeDef,
  Field, MethodDef, Param, MemberRef, MemberRef parent coded indices, AssemblyRef
- Resolve the managed entry-point token to a MethodDef row + IL RVA
- `ClrImage` inspector exposed through the existing Inspect UI for `.NET` exes
- Tests against a real `csc`-built .NET 2.0 assembly

## Milestone 11 — CIL Interpreter Core
**Goal:** Execute the IL of a method body on a managed evaluation stack.

- Parse method headers (tiny/fat), code size, local var sig token, max stack
- Evaluation stack + local/argument slots with a minimal managed value model
- Core opcodes: `nop`, `ldc.i4*`, `ldstr`, `ldloc*`/`stloc*`, `ldarg*`/`starg*`,
  `add`/`sub`/`mul`/`div`, comparison + `br*`/`brtrue`/`brfalse`, `call`, `ret`,
  `dup`, `pop`, `ldnull`, `box`/`unbox` (minimal)
- Managed call frames; `call` dispatch into other MethodDefs
- `callvirt`/`call` to BCL methods routed to internal-call intrinsics

## Milestone 12 — Minimal BCL (mscorlib intrinsics)
**Goal:** A managed "Hello, World" and basic console apps run end to end.

- Internal-call table keyed by `Namespace.Type::Method` (mirrors the Win32 registry)
- `System.Console.WriteLine`/`Write` (string, int, object) → process console
- `System.String` essentials: concat, length, `ToString`
- `System.Int32`/`System.Object.ToString`, `System.Environment.Exit`
- Managed process integrated into the scheduler + console window (PID/title)
- Loader dispatches managed images to the CLR path instead of rejecting them

## Milestone 13 — Native Graphics Foundation (framebuffer/blit)
**Goal:** Pixel-pushing apps (SDL games, DirectDraw, GDI bitmaps) render to a
window canvas. Shared base for native games (DOOM) and managed System.Drawing.

- GDI device-context + bitmap object model: `CreateCompatibleDC`,
  `CreateDIBSection` (pixel buffer in guest memory), `CreateCompatibleBitmap`,
  `SelectObject`(bitmap), `DeleteDC`
- `BitBlt` / `StretchBlt` / `SetDIBitsToDevice` / `StretchDIBits` → `UiEvent::Blit`
  carrying an RGBA framebuffer to the target window
- Frontend renders the blit to the guest window's canvas (with scaling)
- CRT/WINMM coverage games need: `timeGetTime`/`timeBeginPeriod`, `fopen`/
  `freopen`/`fwrite`/`fread` backed by the VFS, char classification (`isspace`…)
- Mount a shareware IWAD so DOOM finds its data

## Milestone 14 — Input + Real-Time Loop
**Goal:** Interactive graphical apps (keyboard, mouse, timers).

- `WM_KEYDOWN`/`WM_KEYUP`/`WM_CHAR`, `WM_MOUSEMOVE`/`WM_*BUTTON*` from the
  frontend into the guest message queue
- `PeekMessage` non-blocking path for game loops; `QueryPerformanceCounter`/
  `timeGetTime` advancing real time across slices
- `GetAsyncKeyState`/`GetKeyState`
- DOOM runs interactively in a window

## Milestone 15 — Managed Graphics (System.Drawing via P/Invoke)
**Goal:** A graphical .NET 2.0 app renders through the same GDI foundation.

- CIL `pinvokeimpl` / ImplMap table → managed `extern` calls bridged to the
  Win32 registry (the link between the CLR and native GDI/user32)
- Marshalling for the common P/Invoke shapes (ints, strings, struct pointers)
- Sample C# app that P/Invokes user32/gdi32 to open a window and draw
- Path toward System.Drawing.Graphics primitives
