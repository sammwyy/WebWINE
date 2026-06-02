use crate::error::{Result, VmError};
use crate::fs::vfs::{DirEntry, VirtualFileSystem};
use crate::logs::{LogBuffer, LogEvent, LogLevel};
use crate::pe::inspector::{inspect_bytes, PeInfo};
use crate::pe::loader::load_pe;
use crate::process::{ProcessInfo, ProcessTable};
use crate::winapi::WinApiDispatcher;

pub struct WebWineVm {
    pub fs:        VirtualFileSystem,
    pub logs:      LogBuffer,
    pub api:       WinApiDispatcher,
    pub processes: ProcessTable,
}

impl WebWineVm {
    pub fn new() -> Self {
        let mut vm = WebWineVm {
            fs:        VirtualFileSystem::new(),
            logs:      LogBuffer::default(),
            api:       WinApiDispatcher::new(),
            processes: ProcessTable::new(),
        };
        vm.logs.log(LogLevel::Info, "vm", "WebWINE VM initialized", None);
        vm
    }

    pub fn mount_file(&mut self, guest_path: &str, bytes: Vec<u8>) -> Result<()> {
        let size = bytes.len();
        self.fs.mount_file(guest_path, bytes)?;
        self.logs.log(LogLevel::Info, "fs",
            &format!("mounted {guest_path} ({size} bytes)"), None);
        Ok(())
    }

    pub fn create_dir(&mut self, guest_path: &str) -> Result<()> {
        self.fs.create_dir(guest_path)?;
        self.logs.log(LogLevel::Info, "fs",
            &format!("created dir {guest_path}"), None);
        Ok(())
    }

    pub fn list_dir(&self, guest_path: &str) -> Result<Vec<DirEntry>> {
        self.fs.list_dir(guest_path)
    }

    pub fn read_file(&self, guest_path: &str) -> Result<Vec<u8>> {
        self.fs.read_file(guest_path)
    }

    pub fn delete_node(&mut self, guest_path: &str) -> Result<()> {
        self.fs.delete_node(guest_path)?;
        self.logs.log(LogLevel::Info, "fs",
            &format!("deleted {guest_path}"), None);
        Ok(())
    }

    pub fn rename_node(&mut self, guest_path: &str, new_name: &str) -> Result<()> {
        self.fs.rename_node(guest_path, new_name)?;
        self.logs.log(LogLevel::Info, "fs",
            &format!("renamed {guest_path} -> {new_name}"), None);
        Ok(())
    }

    pub fn inspect_pe(&mut self, guest_path: &str) -> Result<PeInfo> {
        let bytes = self.fs.read_file(guest_path)?;
        let info = inspect_bytes(&bytes)?;
        self.logs.log(LogLevel::Info, "pe", &format!(
            "parsed {} — {} {} image_base=0x{:08X} entry=0x{:08X} sections={} imports={}",
            guest_path, info.machine, info.subsystem,
            info.image_base, info.entry_point_rva,
            info.sections.len(),
            info.imports.iter().map(|m| m.functions.len()).sum::<usize>(),
        ), None);
        Ok(info)
    }

    pub fn launch_process(&mut self, guest_path: &str) -> Result<u32> {
        let bytes = self.fs.read_file(guest_path)?;
        let pid = self.processes.alloc_pid();
        let proc = load_pe(&bytes, guest_path, pid, &mut self.api, &mut self.logs)?;
        self.processes.insert(proc);
        Ok(pid)
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

    pub fn kill_process(&mut self, pid: u32) -> Result<()> {
        use crate::process::ProcessState;
        let proc = self.processes.get_mut(pid)
            .ok_or(VmError::ProcessNotFound(pid))?;
        proc.state = ProcessState::Exited { exit_code: 1 };
        self.logs.log(LogLevel::Info, "process",
            &format!("pid={pid} killed"), None);
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
