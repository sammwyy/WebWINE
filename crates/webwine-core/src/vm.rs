use crate::error::Result;
use crate::fs::vfs::{DirEntry, VirtualFileSystem};
use crate::logs::{LogBuffer, LogEvent, LogLevel};
use crate::pe::inspector::{inspect_bytes, PeInfo};

pub struct WebWineVm {
    pub fs: VirtualFileSystem,
    pub logs: LogBuffer,
}

impl WebWineVm {
    pub fn new() -> Self {
        let mut vm = WebWineVm {
            fs: VirtualFileSystem::new(),
            logs: LogBuffer::default(),
        };
        vm.logs.log(LogLevel::Info, "vm", "WebWINE VM initialized", None);
        vm
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
        self.logs
            .log(LogLevel::Info, "fs", &format!("created dir {guest_path}"), None);
        Ok(())
    }

    pub fn list_dir(&self, guest_path: &str) -> Result<Vec<DirEntry>> {
        self.fs.list_dir(guest_path)
    }

    pub fn read_file(&self, guest_path: &str) -> Result<Vec<u8>> {
        self.fs.read_file(guest_path)
    }

    pub fn inspect_pe(&mut self, guest_path: &str) -> Result<PeInfo> {
        let bytes = self.fs.read_file(guest_path)?;
        let info = inspect_bytes(&bytes)?;
        self.logs.log(
            LogLevel::Info,
            "pe",
            &format!(
                "parsed {} — {} {} image_base=0x{:08X} entry=0x{:08X} sections={} imports={}",
                guest_path,
                info.machine,
                info.subsystem,
                info.image_base,
                info.entry_point_rva,
                info.sections.len(),
                info.imports.iter().map(|m| m.functions.len()).sum::<usize>(),
            ),
            None,
        );
        Ok(info)
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
