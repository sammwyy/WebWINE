use super::WinApiRegistry;

pub fn register(r: &mut WinApiRegistry) {
    crate::version::register(r);
    crate::gdiplus::register(r);
    crate::wininet::register(r);
    crate::comdlg32::register(r);
    crate::oleaut32::register(r);
    crate::dbghelp::register(r);
}
