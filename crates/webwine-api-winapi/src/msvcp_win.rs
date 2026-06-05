use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("msvcp_win.dll", "_Mtx_init_in_situ", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Mtx_destroy_in_situ", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Mtx_lock", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Mtx_unlock", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Mtx_trylock", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Cnd_init_in_situ", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Cnd_destroy_in_situ", |c| { c.ret_cdecl(0); Handled::Ok }),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
