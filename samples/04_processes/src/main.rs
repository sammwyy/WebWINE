use std::mem;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{
    CreateProcessA, WaitForSingleObject, INFINITE,
    PROCESS_INFORMATION, STARTUPINFOA,
};

fn main() {
    let path = b"C:\\Windows\\System32\\calc.exe\0";

    let mut si: STARTUPINFOA = unsafe { mem::zeroed() };
    si.cb = mem::size_of::<STARTUPINFOA>() as u32;

    let mut pi: PROCESS_INFORMATION = unsafe { mem::zeroed() };

    let ok = unsafe {
        CreateProcessA(
            path.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut si,
            &mut pi,
        )
    };

    if ok == 0 {
        eprintln!("CreateProcessA failed");
        std::process::exit(1);
    }

    println!("launched calc.exe — pid {}", pi.dwProcessId);

    unsafe {
        WaitForSingleObject(pi.hProcess, INFINITE);
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
    }
}
