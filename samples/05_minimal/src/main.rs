// Minimal sample: direct Win32 only, no Rust IO.
// Tests: GetStdHandle, WriteFile, ExitProcess.

use windows_sys::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE};
use windows_sys::Win32::System::Threading::ExitProcess;
use windows_sys::Win32::Foundation::HANDLE;

fn main() {
    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        write(stdout, b"Hello from WebWINE!\r\n");
        write(stdout, b"Counting: ");
        for i in 0u8..5 {
            write(stdout, &[b'0' + i, b' ']);
        }
        write(stdout, b"\r\nDone!\r\n");
        ExitProcess(0);
    }
}

unsafe fn write(handle: HANDLE, data: &[u8]) {
    windows_sys::Win32::Storage::FileSystem::WriteFile(
        handle,
        data.as_ptr(),
        data.len() as u32,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
}
