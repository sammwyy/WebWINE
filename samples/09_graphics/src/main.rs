// No-CRT Win32 GUI sample with GDI graphics + a system beep.
#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

type Handle = *mut c_void;

const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000F;
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const SW_SHOW: i32 = 5;
const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;

fn rgb(r: u32, g: u32, b: u32) -> u32 { r | (g << 8) | (b << 16) }

#[repr(C)]
struct WndClassA {
    style: u32, lpfn_wndproc: usize, cb_cls_extra: i32, cb_wnd_extra: i32,
    hinstance: usize, hicon: usize, hcursor: usize, hbr_background: usize,
    lpsz_menu_name: usize, lpsz_class_name: usize,
}

#[repr(C)]
struct Msg { hwnd: Handle, message: u32, wparam: usize, lparam: usize, time: u32, pt_x: i32, pt_y: i32 }

#[link(name = "user32")]
extern "system" {
    fn RegisterClassA(wc: *const WndClassA) -> u16;
    fn CreateWindowExA(ex: u32, class: *const u8, title: *const u8, style: u32,
                       x: i32, y: i32, w: i32, h: i32,
                       parent: Handle, menu: Handle, inst: Handle, param: *mut c_void) -> Handle;
    fn ShowWindow(hwnd: Handle, cmd: i32) -> i32;
    fn UpdateWindow(hwnd: Handle) -> i32;
    fn GetMessageA(msg: *mut Msg, hwnd: Handle, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageA(msg: *const Msg) -> isize;
    fn DefWindowProcA(hwnd: Handle, msg: u32, wp: usize, lp: usize) -> isize;
    fn PostQuitMessage(code: i32);
    fn BeginPaint(hwnd: Handle, ps: *mut u8) -> Handle;
    fn EndPaint(hwnd: Handle, ps: *const u8) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateSolidBrush(color: u32) -> Handle;
    fn SelectObject(hdc: Handle, obj: Handle) -> Handle;
    fn Rectangle(hdc: Handle, l: i32, t: i32, r: i32, b: i32) -> i32;
    fn Ellipse(hdc: Handle, l: i32, t: i32, r: i32, b: i32) -> i32;
    fn MoveToEx(hdc: Handle, x: i32, y: i32, old: *mut c_void) -> i32;
    fn LineTo(hdc: Handle, x: i32, y: i32) -> i32;
    fn TextOutA(hdc: Handle, x: i32, y: i32, s: *const u8, n: i32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn Beep(freq: u32, dur: u32) -> i32;
    fn ExitProcess(code: u32) -> !;
}

extern "system" fn wndproc(hwnd: Handle, msg: u32, wp: usize, lp: usize) -> isize {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps = [0u8; 64];
                let hdc = BeginPaint(hwnd, ps.as_mut_ptr());

                // filled rectangle
                let blue = CreateSolidBrush(rgb(60, 120, 220));
                SelectObject(hdc, blue);
                Rectangle(hdc, 20, 20, 160, 110);

                // filled ellipse
                let red = CreateSolidBrush(rgb(220, 70, 70));
                SelectObject(hdc, red);
                Ellipse(hdc, 190, 20, 320, 110);

                // a diagonal line
                MoveToEx(hdc, 20, 140, core::ptr::null_mut());
                LineTo(hdc, 320, 200);

                let label = b"GDI graphics in WebWINE";
                TextOutA(hdc, 20, 215, label.as_ptr(), label.len() as i32);

                EndPaint(hwnd, ps.as_ptr());
                0
            }
            WM_DESTROY => { PostQuitMessage(0); 0 }
            _ => DefWindowProcA(hwnd, msg, wp, lp),
        }
    }
}

#[no_mangle]
pub extern "system" fn mainCRTStartup() -> ! {
    unsafe {
        Beep(880, 150);

        let class_name = b"WebWineGfx\0";
        let wc = WndClassA {
            style: 0, lpfn_wndproc: wndproc as usize, cb_cls_extra: 0, cb_wnd_extra: 0,
            hinstance: 0, hicon: 0, hcursor: 0, hbr_background: 0,
            lpsz_menu_name: 0, lpsz_class_name: class_name.as_ptr() as usize,
        };
        RegisterClassA(&wc);

        let title = b"WebWINE Graphics Demo\0";
        let hwnd = CreateWindowExA(
            0, class_name.as_ptr(), title.as_ptr(), WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT, 360, 300,
            core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(),
        );
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg = Msg { hwnd: core::ptr::null_mut(), message: 0, wparam: 0, lparam: 0, time: 0, pt_x: 0, pt_y: 0 };
        while GetMessageA(&mut msg, core::ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
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
