use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[

        ("shdocvw.dll", "#110", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }), // WinList_Init â€” S_OK
        ("shdocvw.dll", "#111", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }), // WinList_Terminate
        ("shdocvw.dll", "#125", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }), // SHCreateFromDesktop
        ("shdocvw.dll", "DllInstall", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
