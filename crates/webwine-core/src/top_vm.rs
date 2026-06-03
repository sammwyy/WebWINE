use crate::error::{Result, VmError};
use crate::fs::vfs::{DirEntry, VirtualFileSystem};
use crate::logs::{LogBuffer, LogEvent, LogLevel};
use crate::pe::inspector::{inspect_bytes, PeInfo};
use crate::pe::loader::load_pe;
use crate::vm::executor::{run_slice, SliceResult};
use crate::vm::process::{GuestProcess, ProcessInfo, ProcessState, ProcessTable};
use crate::clr::{is_managed, ClrImage, ClrRuntime};
use crate::winapi::{register_all, WinApiRegistry};

pub struct WebWineVm {
    pub fs:        VirtualFileSystem,
    pub logs:      LogBuffer,
    pub api:       WinApiRegistry,
    pub processes: ProcessTable,
}

impl WebWineVm {
    pub fn new() -> Self {
        let mut api = WinApiRegistry::new();
        register_all(&mut api);

        let mut vm = WebWineVm {
            fs: VirtualFileSystem::new(),
            logs: LogBuffer::default(),
            api,
            processes: ProcessTable::new(),
        };
        vm.logs.log(LogLevel::Info, "vm", "WebWINE VM initialized", None);
        vm
    }

    pub fn mount_file(&mut self, guest_path: &str, bytes: Vec<u8>) -> Result<()> {
        let size = bytes.len();
        self.fs.mount_file(guest_path, bytes)?;
        self.logs.log(LogLevel::Info, "fs", &format!("mounted {guest_path} ({size} bytes)"), None);
        Ok(())
    }

    pub fn create_dir(&mut self, guest_path: &str) -> Result<()> {
        self.fs.create_dir(guest_path)?;
        self.logs.log(LogLevel::Info, "fs", &format!("created dir {guest_path}"), None);
        Ok(())
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
        self.logs.log(LogLevel::Info, "fs", &format!("deleted {guest_path}"), None);
        Ok(())
    }

    pub fn rename_node(&mut self, guest_path: &str, new_name: &str) -> Result<()> {
        self.fs.rename_node(guest_path, new_name)?;
        self.logs.log(LogLevel::Info, "fs", &format!("renamed {guest_path} -> {new_name}"), None);
        Ok(())
    }

    pub fn inspect_pe(&mut self, guest_path: &str) -> Result<PeInfo> {
        let bytes = self.fs.read_file(guest_path)?;
        let info = inspect_bytes(&bytes)?;
        self.logs.log(LogLevel::Info, "pe", &format!(
            "parsed {} — {} {} base=0x{:08X} entry=0x{:08X} sections={} imports={}",
            guest_path, info.machine, info.subsystem,
            info.image_base, info.entry_point_rva,
            info.sections.len(),
            info.imports.iter().map(|m| m.functions.len()).sum::<usize>(),
        ), None);
        Ok(info)
    }

    /// Inspect a managed (.NET) assembly's CLI metadata for the Inspect panel.
    pub fn inspect_clr(&mut self, guest_path: &str) -> Result<crate::clr::ClrInfo> {
        let bytes = self.fs.read_file(guest_path)?;
        let info = crate::clr::inspect_clr(&bytes)?;
        self.logs.log(LogLevel::Info, "clr", &format!(
            "parsed managed {} — runtime {} entry {} types={} methods={}",
            guest_path, info.runtime_version, info.entry_point_method,
            info.types.len(), info.methods.len(),
        ), None);
        Ok(info)
    }

    /// True if the file at `guest_path` is a managed (.NET) assembly.
    pub fn is_managed_file(&self, guest_path: &str) -> bool {
        self.fs.read_file(guest_path).map(|b| is_managed(&b)).unwrap_or(false)
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
            self.logs.log(LogLevel::Info, "process",
                &format!("launched managed (.NET) process pid={pid} {guest_path}"), None);
            return Ok(pid);
        }

        let mut proc = load_pe(&bytes, guest_path, pid, &mut self.api, &mut self.logs)?;
        proc.cmdline = cmdline;
        self.processes.insert(proc);
        Ok(pid)
    }

    /// Run a managed process's entry point to completion via the CLR interpreter,
    /// buffering its output into the process console.
    fn run_managed(&mut self, pid: u32) {
        let Some(proc) = self.processes.get_mut(pid) else { return };
        let Some(bytes) = proc.managed.take() else { return };

        let (stdout, state) = match ClrImage::parse(&bytes) {
            Ok(img) => {
                let mut rt = ClrRuntime::new(&img);
                match rt.run_entry() {
                    Ok(code) => (rt.stdout, ProcessState::Exited { exit_code: code as u32 }),
                    // Keep whatever was printed before the fault for debugging.
                    Err(e) => (rt.stdout, ProcessState::Crashed { reason: e.to_string() }),
                }
            }
            Err(e) => (String::new(), ProcessState::Crashed { reason: e.to_string() }),
        };

        proc.console.stdout.extend_from_slice(stdout.as_bytes());
        proc.state = state.clone();
        if let ProcessState::Crashed { reason } = &state {
            self.logs.log(LogLevel::Warn, "clr",
                &format!("managed pid={pid} crashed: {reason}"), Some(pid));
        }
    }

    pub fn run_process_slice(&mut self, pid: u32, budget: u32) -> Result<SliceResult> {
        // Managed processes execute via the CLR runner; once finished, the normal
        // slice path below drains their buffered console output and exit state.
        if self.processes.get(pid).map(|p| p.managed.is_some()).unwrap_or(false) {
            self.run_managed(pid);
        }

        let next_pid = self.processes.peek_next_pid();
        let (mut result, spawns) = {
            let proc = self.processes.get_mut(pid)
                .ok_or(VmError::ProcessNotFound(pid))?;
            proc.next_child_pid = next_pid;
            let r = run_slice(proc, budget, &self.api, &mut self.fs, &mut self.logs)?;
            let spawns = std::mem::take(&mut proc.spawns);
            (r, spawns)
        };

        // Launch any child processes the guest requested via CreateProcess.
        for sp in spawns {
            match self.launch_process(&sp.path) {
                Ok(child) => {
                    result.spawned.push((child, sp.path.clone()));
                    self.logs.log(LogLevel::Info, "process",
                        &format!("spawned pid={child} from {}", sp.path), Some(pid));
                }
                Err(e) => {
                    self.logs.log(LogLevel::Warn, "process",
                        &format!("CreateProcess failed for {}: {e}", sp.path), Some(pid));
                }
            }
        }
        Ok(result)
    }

    pub fn write_stdin(&mut self, pid: u32, text: &str) -> Result<()> {
        let proc = self.processes.get_mut(pid)
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
        &mut self, pid: u32, hwnd: u32, message: u32, wparam: u32, lparam: u32,
    ) -> Result<()> {
        use crate::vm::process::{GuestMsg, ProcessState};
        let proc = self.processes.get_mut(pid)
            .ok_or(VmError::ProcessNotFound(pid))?;
        proc.gui.queue.push_back(GuestMsg { hwnd, message, wparam, lparam });
        if matches!(proc.state, ProcessState::WaitingForInput) {
            proc.state = ProcessState::Running;
        }
        Ok(())
    }

    pub fn get_process_info(&self, pid: u32) -> Result<ProcessInfo> {
        self.processes.get(pid).map(|p| p.info())
            .ok_or(VmError::ProcessNotFound(pid))
    }

    pub fn list_processes(&self) -> Vec<ProcessInfo> {
        self.processes.list_info()
    }

    pub fn kill_process(&mut self, pid: u32) -> Result<()> {
        use crate::vm::process::ProcessState;
        let proc = self.processes.get_mut(pid)
            .ok_or(VmError::ProcessNotFound(pid))?;
        proc.state = ProcessState::Exited { exit_code: 1 };
        self.logs.log(LogLevel::Info, "process", &format!("pid={pid} killed"), None);
        Ok(())
    }

    pub fn drain_logs(&mut self) -> Vec<LogEvent> {
        self.logs.drain()
    }
}

impl Default for WebWineVm {
    fn default() -> Self { Self::new() }
}
