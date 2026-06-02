// No-CRT sample exercising the Win32 file APIs (Milestone 7):
// creates hello.txt with "world", a folder "foo", and an empty foo\bar.txt.
#![no_std]
#![no_main]

use core::panic::PanicInfo;

type Handle = *mut core::ffi::c_void;
const INVALID_HANDLE: Handle = !0usize as Handle;

const GENERIC_WRITE: u32 = 0x4000_0000;
const CREATE_ALWAYS: u32 = 2;
const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;

#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(n: u32) -> Handle;
    fn WriteFile(h: Handle, buf: *const u8, len: u32, written: *mut u32, ovl: *mut core::ffi::c_void) -> i32;
    fn CreateFileA(name: *const u8, access: u32, share: u32, sec: *mut core::ffi::c_void,
                   disp: u32, flags: u32, template: Handle) -> Handle;
    fn CreateDirectoryA(name: *const u8, sec: *mut core::ffi::c_void) -> i32;
    fn CloseHandle(h: Handle) -> i32;
    fn ExitProcess(code: u32) -> !;
}

unsafe fn print(s: &[u8]) {
    let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
    let mut written = 0u32;
    WriteFile(stdout, s.as_ptr(), s.len() as u32, &mut written, core::ptr::null_mut());
}

unsafe fn create_write(name: &[u8], data: &[u8]) -> bool {
    let h = CreateFileA(name.as_ptr(), GENERIC_WRITE, 0, core::ptr::null_mut(),
                        CREATE_ALWAYS, 0, core::ptr::null_mut());
    if h == INVALID_HANDLE { return false; }
    if !data.is_empty() {
        let mut written = 0u32;
        WriteFile(h, data.as_ptr(), data.len() as u32, &mut written, core::ptr::null_mut());
    }
    CloseHandle(h);
    true
}

#[no_mangle]
pub extern "system" fn mainCRTStartup() -> ! {
    unsafe {
        if create_write(b"hello.txt\0", b"world") {
            print(b"created hello.txt\r\n");
        }
        if CreateDirectoryA(b"foo\0".as_ptr(), core::ptr::null_mut()) != 0 {
            print(b"created foo\\\r\n");
        }
        if create_write(b"foo\\bar.txt\0", b"") {
            print(b"created foo\\bar.txt\r\n");
        }
        print(b"done\r\n");
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
