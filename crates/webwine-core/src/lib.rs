pub mod clr;
pub mod pe;
pub mod vm;
pub mod winapi;

pub use webwine_api::{error, fs, logs, registry};
pub use vm::{ProcessInfo, ProcessState, SliceResult};
pub use vm::executor::SliceResult as RunResult;
pub use fs::vfs::{DirEntry, EntryKind};
pub use logs::{LogEvent, LogLevel};
pub use pe::inspector::{PeImportModule, PeInfo, PeSection};
pub use error::{VmError, Result};

// top-level VM
mod top_vm;
pub use top_vm::{AppRegistration, WebWineVm};
