pub mod advapi32;
pub mod comctl32;
pub mod comdlg32;
pub mod dbghelp;
pub mod extra_dlls;
pub mod gdi32;
pub mod gdiplus;
pub mod kernel32;
pub mod mfc42u;
pub mod msvcrt;
pub mod msvcp_win;
pub mod ntdll;
pub mod ole32;
pub mod oleaut32;
pub mod shdocvw;
pub mod shell32;
pub mod shlwapi;
pub mod util;
pub mod user32;
pub mod ucrtbase;
pub mod version;
pub mod vcruntime140;
pub mod wininet;
pub mod winmm;
pub mod ws2_32;

pub use webwine_api::winapi::{ApiContext, Handled, HandlerFn, WinApiRegistry};

/// Register the base Win32/CRT API set.
pub fn register(r: &mut WinApiRegistry) {
    advapi32::register(r);
    shlwapi::register(r);
    comctl32::register(r);
    ws2_32::register(r);
    shell32::register(r);
    gdi32::register(r);
    shdocvw::register(r);
    ole32::register(r);
    mfc42u::register(r);
    kernel32::register(r);
    msvcrt::register(r);
    msvcp_win::register(r);
    ucrtbase::register(r);
    vcruntime140::register(r);
    ntdll::register(r);
    user32::register(r);
    winmm::register(r);
    extra_dlls::register(r);
}
