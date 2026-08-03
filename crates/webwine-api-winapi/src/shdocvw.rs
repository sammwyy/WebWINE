//! shdocvw.dll — shell document / explorer bootstrap ordinals.

use super::{ApiContext, Handled, WinApiRegistry};

const S_OK: u32 = 0;

pub fn register(r: &mut WinApiRegistry) {
    r.add("shdocvw.dll", "#110", winlist_init);
    r.add("shdocvw.dll", "#111", winlist_terminate);
    r.add("shdocvw.dll", "#125", sh_create_from_desktop);
    r.add("shdocvw.dll", "DllInstall", dll_install);
    r.add("shdocvw.dll", "DllGetClassObject", dll_get_class_object);
    r.add("shdocvw.dll", "DllCanUnloadNow", dll_can_unload_now);
    r.add("shdocvw.dll", "DllRegisterServer", dll_register_server);
    r.add("shdocvw.dll", "DllUnregisterServer", dll_unregister_server);
}

/// WinList_Init — explorer startup hook; returns S_OK.
fn winlist_init(c: &mut ApiContext) -> Handled {
    c.dll_state.insert("shdocvw.winlist".into(), 1);
    c.ret_stdcall(S_OK, 0);
    Handled::Ok
}

/// WinList_Terminate.
fn winlist_terminate(c: &mut ApiContext) -> Handled {
    c.dll_state.remove("shdocvw.winlist");
    c.ret_stdcall(S_OK, 0);
    Handled::Ok
}

/// SHCreateFromDesktop — desktop shell bootstrap.
fn sh_create_from_desktop(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 0);
    Handled::Ok
}

fn dll_install(c: &mut ApiContext) -> Handled {
    // DllInstall(bInstall, pszCmdLine) → HRESULT
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dll_get_class_object(c: &mut ApiContext) -> Handled {
    let out = c.arg(2);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(0x8004_0154, 3); // REGDB_E_CLASSNOTREG
    Handled::Ok
}

fn dll_can_unload_now(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 0); // S_OK = can unload
    Handled::Ok
}

fn dll_register_server(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 0);
    Handled::Ok
}

fn dll_unregister_server(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 0);
    Handled::Ok
}
