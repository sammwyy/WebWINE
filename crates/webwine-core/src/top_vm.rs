use crate::clr::{is_managed, ClrImage, ClrRuntime};
use crate::error::{Result, VmError};
use crate::fs::vfs::{DirEntry, VirtualFileSystem};
use crate::logs::{LogBuffer, LogEvent, LogLevel};
use crate::pe::inspector::{inspect_bytes, PeInfo};
use crate::pe::loader::load_pe;
use crate::registry::Registry;
use crate::vm::executor::SliceResult;
use crate::vm::process::{GuestProcess, ProcessInfo, ProcessState, ProcessTable, UiEvent};
use crate::winapi::{register_all, WinApiRegistry};
use serde::{Deserialize, Serialize};

/// A client-defined virtual app / shell "quick action". The frontend owns the
/// list and maps each `action` to a UI component; the core only uses these
/// fields to lay down the placeholder exe + Start Menu shortcut. UI-only metadata
/// (e.g. icon) is intentionally not part of this — it stays in the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRegistration {
    pub name: String,
    pub exe_path: String,
    pub action: String,
}

pub struct WebWineVm {
    pub fs: VirtualFileSystem,
    pub registry: Registry,
    pub logs: LogBuffer,
    pub api: WinApiRegistry,
    pub processes: ProcessTable,
}

impl WebWineVm {
    pub fn new() -> Self {
        let mut api = WinApiRegistry::new();
        register_all(&mut api);

        let mut vm = WebWineVm {
            fs: VirtualFileSystem::new(),
            registry: Registry::new(),
            logs: LogBuffer::default(),
            api,
            processes: ProcessTable::new(),
        };
        vm.seed_system_dlls();
        vm.logs
            .log(LogLevel::Info, "vm", "WebWINE VM initialized", None);
        vm
    }

    /// Place a virtual placeholder file in C:\Windows\System32 for every DLL we
    /// provide built-in stubs for, so the guest filesystem looks like Windows
    /// (apps that probe for a system DLL's existence find it). These are virtual
    /// markers, never mapped by the loader — imports of a stubbed DLL route to
    /// trampolines regardless of this file.
    fn seed_system_dlls(&mut self) {
        let names: Vec<String> = self
            .api
            .stub_dll_names()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        for name in names {
            if !name.ends_with(".dll") {
                continue;
            }
            let path = format!("C:\\Windows\\System32\\{name}");
            // Virtual ghost: exists for FS probes, no content, never persisted.
            let _ = self.fs.mount_virtual_file(&path);
        }
    }

    pub fn mount_file(&mut self, guest_path: &str, bytes: Vec<u8>) -> Result<()> {
        let size = bytes.len();
        self.fs.mount_file(guest_path, bytes)?;
        self.logs.log(
            LogLevel::Info,
            "fs",
            &format!("mounted {guest_path} ({size} bytes)"),
            None,
        );
        Ok(())
    }

    pub fn create_dir(&mut self, guest_path: &str) -> Result<()> {
        self.fs.create_dir(guest_path)?;
        self.logs.log(
            LogLevel::Info,
            "fs",
            &format!("created dir {guest_path}"),
            None,
        );
        Ok(())
    }

    /// Seed a disk's default Windows folders/files (idempotent). The host calls
    /// this when it wants the skeleton on a (possibly driver-persisted) drive;
    /// persistence itself is the driver's responsibility, not the core's.
    pub fn init_disk_defaults(&mut self, drive: char) {
        self.fs.init_disk_defaults(drive);
    }

    pub fn list_dir(&self, guest_path: &str) -> Result<Vec<DirEntry>> {
        self.fs.list_dir(guest_path)
    }

    pub fn read_file(&self, guest_path: &str) -> Result<Vec<u8>> {
        self.fs.read_file(guest_path)
    }

    pub fn read_raw_file(&self, guest_path: &str) -> Result<Vec<u8>> {
        self.fs.read_raw_file(guest_path)
    }

    pub fn delete_node(&mut self, guest_path: &str) -> Result<()> {
        self.fs.delete_node(guest_path)?;
        self.logs
            .log(LogLevel::Info, "fs", &format!("deleted {guest_path}"), None);
        Ok(())
    }

    pub fn rename_node(&mut self, guest_path: &str, new_name: &str) -> Result<()> {
        self.fs.rename_node(guest_path, new_name)?;
        self.logs.log(
            LogLevel::Info,
            "fs",
            &format!("renamed {guest_path} -> {new_name}"),
            None,
        );
        Ok(())
    }

    /// Materialize a client-defined virtual app ("quick action") in the guest
    /// filesystem: a placeholder exe carrying a `special:<action>` marker plus a
    /// Start Menu shortcut pointing at it. The *list* of apps lives in the client
    /// (it maps each action to a UI component); the core only reserves the slot.
    pub fn register_app(&mut self, app: &AppRegistration) -> Result<()> {
        let marker = format!("special:{}", app.action);
        let _ = self.fs.mount_file(&app.exe_path, marker.into_bytes());

        let lnk_path = format!(
            "C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\{}.lnk",
            app.name
        );
        let _ = self
            .fs
            .mount_file(&lnk_path, app.exe_path.as_bytes().to_vec());

        self.logs.log(
            LogLevel::Info,
            "shell",
            &format!("registered app {}", app.name),
            None,
        );
        Ok(())
    }

    pub fn inspect_pe(&mut self, guest_path: &str) -> Result<PeInfo> {
        let bytes = self.fs.read_file(guest_path)?;
        let info = inspect_bytes(&bytes)?;
        self.logs.log(
            LogLevel::Info,
            "pe",
            &format!(
                "parsed {} — {} {} base=0x{:08X} entry=0x{:08X} sections={} imports={}",
                guest_path,
                info.machine,
                info.subsystem,
                info.image_base,
                info.entry_point_rva,
                info.sections.len(),
                info.imports
                    .iter()
                    .map(|m| m.functions.len())
                    .sum::<usize>(),
            ),
            None,
        );
        Ok(info)
    }

    /// Inspect a managed (.NET) assembly's CLI metadata for the Inspect panel.
    pub fn inspect_clr(&mut self, guest_path: &str) -> Result<crate::clr::ClrInfo> {
        let bytes = self.fs.read_file(guest_path)?;
        let info = crate::clr::inspect_clr(&bytes)?;
        self.logs.log(
            LogLevel::Info,
            "clr",
            &format!(
                "parsed managed {} — runtime {} entry {} types={} methods={}",
                guest_path,
                info.runtime_version,
                info.entry_point_method,
                info.types.len(),
                info.methods.len(),
            ),
            None,
        );
        Ok(info)
    }

    /// True if the file at `guest_path` is a managed (.NET) assembly.
    pub fn is_managed_file(&self, guest_path: &str) -> bool {
        self.fs
            .read_file(guest_path)
            .map(|b| is_managed(&b))
            .unwrap_or(false)
    }

    pub fn launch_process(&mut self, guest_path: &str) -> Result<u32> {
        self.launch_process_with_args(guest_path, "")
    }

    /// Launch with command-line arguments (e.g. `-iwad doom1.wad`). argv[0] is the
    /// quoted image path; `args` is appended as the rest of the command line.
    pub fn launch_process_with_args(&mut self, guest_path: &str, args: &str) -> Result<u32> {
        let bytes = self.fs.read_file(guest_path)?;
        let pid = self.processes.alloc_pid();
        let cmdline = if args.trim().is_empty() {
            format!("\"{guest_path}\"")
        } else {
            format!("\"{guest_path}\" {}", args.trim())
        };

        // Managed (.NET) assemblies run on the CLR interpreter, not the x86 loader.
        if is_managed(&bytes) {
            let mut proc = GuestProcess::new_managed(pid, guest_path, bytes);
            proc.cmdline = cmdline;
            self.processes.insert(proc);
            self.logs.log(
                LogLevel::Info,
                "process",
                &format!("launched managed (.NET) process pid={pid} {guest_path}"),
                None,
            );
            return Ok(pid);
        }

        let proc = load_pe(
            &bytes,
            guest_path,
            &cmdline,
            pid,
            &mut self.api,
            &self.fs,
            &mut self.logs,
        )?;
        self.processes.insert(proc);
        Ok(pid)
    }

    /// Run a managed process's entry point to completion via the CLR interpreter,
    /// buffering its output into the process console.
    fn run_managed(&mut self, pid: u32) {
        let Some(proc) = self.processes.get_mut(pid) else {
            return;
        };
        let Some(bytes) = proc.managed.take() else {
            return;
        };

        let (stdout, ui_events, state) = match ClrImage::parse(&bytes) {
            Ok(img) => {
                let mut rt = ClrRuntime::new(&img);
                match rt.run_entry() {
                    Ok(code) => {
                        let state = if rt.is_waiting_for_input() {
                            ProcessState::WaitingForInput
                        } else {
                            ProcessState::Exited { exit_code: code as u32 }
                        };
                        (rt.stdout, rt.ui_events, state)
                    }
                    // Keep whatever was printed before the fault for debugging.
                    Err(e) => (
                        rt.stdout,
                        rt.ui_events,
                        ProcessState::Crashed {
                            reason: e.to_string(),
                        },
                    ),
                }
            }
            Err(e) => (
                String::new(),
                Vec::new(),
                ProcessState::Crashed {
                    reason: e.to_string(),
                },
            ),
        };

        proc.console.stdout.extend_from_slice(stdout.as_bytes());
        proc.ui_events.extend(ui_events);
        proc.state = state.clone();
        if let ProcessState::Crashed { reason } = &state {
            self.logs.log(
                LogLevel::Warn,
                "clr",
                &format!("managed pid={pid} crashed: {reason}"),
                Some(pid),
            );
        }
    }

    pub fn run_process_slice(&mut self, pid: u32, budget: u32) -> Result<SliceResult> {
        // Managed processes execute via the CLR runner; once finished, the normal
        // slice path below drains their buffered console output and exit state.
        if self
            .processes
            .get(pid)
            .map(|p| p.managed.is_some())
            .unwrap_or(false)
        {
            self.run_managed(pid);
        }

        let next_pid = self.processes.peek_next_pid();
        let (mut result, spawns) = {
            let proc = self
                .processes
                .get_mut(pid)
                .ok_or(VmError::ProcessNotFound(pid))?;
            proc.next_child_pid = next_pid;
            let r = if proc.cpu.eip == 0 {
                crate::vm::executor::SliceResult::done(proc, 0)
            } else {
                crate::vm::executor::run_slice(
                    proc,
                    budget,
                    &self.api,
                    &mut self.fs,
                    &mut self.registry,
                    &mut self.logs,
                )?
            };
            let spawns = std::mem::take(&mut proc.spawns);
            (r, spawns)
        };

        // Launch any child processes the guest requested via CreateProcess.
        for sp in spawns {
            match self.launch_process(&sp.path) {
                Ok(child) => {
                    result.spawned.push((child, sp.path.clone()));
                    self.logs.log(
                        LogLevel::Info,
                        "process",
                        &format!("spawned pid={child} from {}", sp.path),
                        Some(pid),
                    );
                }
                Err(e) => {
                    self.logs.log(
                        LogLevel::Warn,
                        "process",
                        &format!("CreateProcess failed for {}: {e}", sp.path),
                        Some(pid),
                    );
                }
            }
        }

        // Window teardown on process death. A CLEAN exit (the app called
        // ExitProcess normally) closes its windows. An ABNORMAL death — a crash,
        // or the graceful halt on control-flow corruption (exit code
        // STATUS_ACCESS_VIOLATION) — instead leaves the last frame frozen on
        // screen rather than making the window vanish, which is friendlier for
        // debugging and for apps that die mid-render (e.g. a game still bringing
        // up its UI). The frontend just stops feeding it; the window persists.
        const STATUS_ACCESS_VIOLATION: u32 = 0xC000_0005;
        let clean_exit = matches!(
            result.state,
            ProcessState::Exited { exit_code } if exit_code != STATUS_ACCESS_VIOLATION
        );
        if clean_exit {
            if let Some(proc) = self.processes.get_mut(pid) {
                if !proc.gui.windows.is_empty() {
                    let mut hwnds: Vec<u32> = proc.gui.windows.keys().copied().collect();
                    hwnds.sort_unstable();
                    for hwnd in hwnds {
                        result.ui_events.push(UiEvent::DestroyWindow { hwnd });
                    }
                    proc.gui.windows.clear();
                    self.logs.log(
                        LogLevel::Info,
                        "process",
                        &format!("pid={pid} exited — closed its windows"),
                        Some(pid),
                    );
                }
            }
        }
        Ok(result)
    }

    pub fn write_stdin(&mut self, pid: u32, text: &str) -> Result<()> {
        let proc = self
            .processes
            .get_mut(pid)
            .ok_or(VmError::ProcessNotFound(pid))?;
        let mut prev_was_cr = proc.console.stdin.back().copied() == Some(b'\r');
        for b in text.bytes() {
            match b {
                b'\x08' | b'\x7f' => {
                    proc.console.stdin.pop_back();
                    prev_was_cr = proc.console.stdin.back().copied() == Some(b'\r');
                }
                b'\n' => {
                    if !prev_was_cr {
                        proc.console.stdin.push_back(b'\r');
                    }
                    proc.console.stdin.push_back(b'\n');
                    prev_was_cr = false;
                }
                _ => {
                    proc.console.stdin.push_back(b);
                    prev_was_cr = b == b'\r';
                }
            }
        }
        if matches!(proc.state, ProcessState::WaitingForInput) {
            proc.state = ProcessState::Running;
        }
        Ok(())
    }

    /// Post a window message to a process (e.g. WM_CLOSE when the user closes a
    /// guest window). Wakes the process if it was blocked in GetMessage.
    pub fn post_window_message(
        &mut self,
        pid: u32,
        hwnd: u32,
        message: u32,
        wparam: u32,
        lparam: u32,
    ) -> Result<()> {
        use crate::vm::process::{GuestMsg, ProcessState};
        let proc = self
            .processes
            .get_mut(pid)
            .ok_or(VmError::ProcessNotFound(pid))?;
        proc.gui.queue.push_back(GuestMsg {
            hwnd,
            message,
            wparam,
            lparam,
        });
        if matches!(proc.state, ProcessState::WaitingForInput) {
            proc.state = ProcessState::Running;
        }
        Ok(())
    }

    /// Deliver the user's answer to a modal dialog (MessageBox button, or a file
    /// picker's chosen path / cancel) the process is blocked on, and wake it.
    pub fn post_dialog_reply(&mut self, pid: u32, button: u32, file: Option<String>) -> Result<()> {
        use crate::vm::process::{DialogReply, ProcessState};
        let proc = self
            .processes
            .get_mut(pid)
            .ok_or(VmError::ProcessNotFound(pid))?;
        proc.gui.dialog_reply = Some(DialogReply { button, file });
        if matches!(proc.state, ProcessState::WaitingForInput) {
            proc.state = ProcessState::Running;
        }
        Ok(())
    }

    pub fn get_process_info(&self, pid: u32) -> Result<ProcessInfo> {
        self.processes
            .get(pid)
            .map(|p| p.info())
            .ok_or(VmError::ProcessNotFound(pid))
    }

    pub fn list_processes(&self) -> Vec<ProcessInfo> {
        self.processes.list_info()
    }

    pub fn system_memory(&self) -> crate::SystemMemoryInfo {
        self.processes.system_memory()
    }

    pub fn kill_process(&mut self, pid: u32) -> Result<()> {
        use crate::vm::process::ProcessState;
        let proc = self
            .processes
            .get_mut(pid)
            .ok_or(VmError::ProcessNotFound(pid))?;
        proc.state = ProcessState::Exited { exit_code: 1 };
        self.logs.log(
            LogLevel::Info,
            "process",
            &format!("pid={pid} killed"),
            None,
        );
        Ok(())
    }

    pub fn drain_logs(&mut self) -> Vec<LogEvent> {
        self.logs.drain()
    }
}

impl Default for WebWineVm {
    fn default() -> Self {
        Self::new()
    }
}
