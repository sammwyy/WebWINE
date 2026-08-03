use super::WinApiRegistry;

pub fn register(r: &mut WinApiRegistry) {
    r.add("msdxm.ocx", "RunDllW", |c| {
        c.ret_stdcall(0, 4);
        webwine_api::winapi::Handled::Ok
    });
    crate::version::register(r);
    crate::gdiplus::register(r);
    crate::wininet::register(r);
    crate::comdlg32::register(r);
    crate::oleaut32::register(r);
    crate::dbghelp::register(r);
}
