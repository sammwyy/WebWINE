// No-CRT sample (Milestone 6): spawns a child process via CreateProcessA and
// waits for it. The child (minimal.exe) must sit next to this exe on the desktop.
#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

type Handle = *mut c_void;
const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
const INFINITE: u32 = 0xFFFF_FFFF;

#[repr(C)]
struct StartupInfoA {
    cb: u32,
    reserved: usize, desktop: usize, title: usize,
    x: u32, y: u32, xsize: u32, ysize: u32,
    xchars: u32, ychars: u32, fill_attr: u32, flags: u32,
    show: u16, cb_reserved2: u16, reserved2: usize,
    std_input: usize, std_output: usize, std_error: usize,
}

#[repr(C)]
struct ProcessInformation {
    h_process: Handle,
    h_thread: Handle,
    dw_process_id: u32,
    dw_thread_id: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(n: u32) -> Handle;
    fn WriteFile(h: Handle, buf: *const u8, len: u32, written: *mut u32, ovl: *mut c_void) -> i32;
    fn CreateProcessA(app: *const u8, cmd: *mut u8, pa: *mut c_void, ta: *mut c_void,
                      inherit: i32, flags: u32, env: *mut c_void, cwd: *const u8,
                      si: *const StartupInfoA, pi: *mut ProcessInformation) -> i32;
    fn WaitForSingleObject(h: Handle, ms: u32) -> u32;
    fn CloseHandle(h: Handle) -> i32;
    fn ExitProcess(code: u32) -> !;
}

unsafe fn print(s: &[u8]) {
    let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
    let mut w = 0u32;
    WriteFile(stdout, s.as_ptr(), s.len() as u32, &mut w, core::ptr::null_mut());
}

#[no_mangle]
pub extern "system" fn mainCRTStartup() -> ! {
    unsafe {
        print(b"parent: launching child...\r\n");

        let si = StartupInfoA {
            cb: core::mem::size_of::<StartupInfoA>() as u32,
            reserved: 0, desktop: 0, title: 0, x: 0, y: 0, xsize: 0, ysize: 0,
            xchars: 0, ychars: 0, fill_attr: 0, flags: 0, show: 0, cb_reserved2: 0,
            reserved2: 0, std_input: 0, std_output: 0, std_error: 0,
        };
        let mut pi = ProcessInformation {
            h_process: core::ptr::null_mut(), h_thread: core::ptr::null_mut(),
            dw_process_id: 0, dw_thread_id: 0,
        };

        let ok = CreateProcessA(
            b"minimal.exe\0".as_ptr(), core::ptr::null_mut(),
            core::ptr::null_mut(), core::ptr::null_mut(), 0, 0,
            core::ptr::null_mut(), core::ptr::null(), &si, &mut pi,
        );

        if ok != 0 {
            print(b"parent: child created\r\n");
            WaitForSingleObject(pi.h_process, INFINITE);
            CloseHandle(pi.h_process);
            CloseHandle(pi.h_thread);
            print(b"parent: child finished, exiting\r\n");
        } else {
            print(b"parent: CreateProcess failed\r\n");
        }
        ExitProcess(0);
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! { unsafe { ExitProcess(1) } }

#[no_mangle]
pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    let mut i = 0; while i < n { *d.add(i) = *s.add(i); i += 1; } d
}
#[no_mangle]
pub unsafe extern "C" fn memset(d: *mut u8, v: i32, n: usize) -> *mut u8 {
    let mut i = 0; while i < n { *d.add(i) = v as u8; i += 1; } d
}
#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n { let (x, y) = (*a.add(i), *b.add(i)); if x != y { return x as i32 - y as i32; } i += 1; }
    0
}

core::arch::global_asm!(
    ".globl __fltused",  "__fltused: .long 0",
    ".globl __aulldiv",  "__aulldiv: ret",
    ".globl __aullrem",  "__aullrem: ret",
    ".globl ___CxxFrameHandler3", "___CxxFrameHandler3: ret",
);
