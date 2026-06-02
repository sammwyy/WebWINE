// Minimal no-CRT PE: bypasses the entire UCRT startup by defining the
// console entry point (mainCRTStartup) directly. Talks to Win32 only.
// This is the cleanest target for an emulator that doesn't yet run the
// full C runtime initialization.
#![no_std]
#![no_main]

use core::panic::PanicInfo;

type Handle = *mut core::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(n: u32) -> Handle;
    fn WriteFile(h: Handle, buf: *const u8, len: u32, written: *mut u32, ovl: *mut core::ffi::c_void) -> i32;
    fn ExitProcess(code: u32) -> !;
}

const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5; // (DWORD)-11

#[no_mangle]
pub extern "system" fn mainCRTStartup() -> ! {
    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        write(stdout, b"Hello from WebWINE!\r\n");
        write(stdout, b"Counting: ");
        let mut buf = [b'0', b' '];
        let mut i = 0u8;
        while i < 5 {
            buf[0] = b'0' + i;
            write(stdout, &buf);
            i += 1;
        }
        write(stdout, b"\r\nDone!\r\n");
        ExitProcess(0);
    }
}

unsafe fn write(h: Handle, data: &[u8]) {
    let mut written: u32 = 0;
    WriteFile(h, data.as_ptr(), data.len() as u32, &mut written, core::ptr::null_mut());
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    unsafe { ExitProcess(1) }
}

// Intrinsics that `core` may reference. Tiny byte-wise implementations are
// fine for our purposes and avoid pulling the CRT.
#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n { *dst.add(i) = *src.add(i); i += 1; }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memset(dst: *mut u8, val: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n { *dst.add(i) = val as u8; i += 1; }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let x = *a.add(i);
        let y = *b.add(i);
        if x != y { return x as i32 - y as i32; }
        i += 1;
    }
    0
}

// Symbols `core` references from its u128-formatting and panic codegen units.
// None are ever reached at runtime in this program — they exist only so the
// MSVC linker resolves the (dead) references. Defined in raw asm so the exact
// symbol names (no extra underscore decoration) match what the linker asks for.
core::arch::global_asm!(
    ".globl __fltused",  "__fltused: .long 0",
    ".globl __aulldiv",  "__aulldiv: ret",
    ".globl __aullrem",  "__aullrem: ret",
    ".globl ___CxxFrameHandler3", "___CxxFrameHandler3: ret",
);
