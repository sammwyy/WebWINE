//! Compatibility exports for the Visual C++ 2013 C++ runtime.
//!
//! The PE loader does not need a physical DLL for these exports: it patches
//! imports to Rust trampolines.  This module handles the ABI-critical runtime
//! primitives and common STL/exception symbols; unknown template exports still
//! get a safe cdecl trampoline through the normal stub-DLL path.
use super::{ApiContext, Handled, WinApiRegistry};

fn cdecl0(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}
fn cdecl_this(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    c.ret_cdecl(this);
    Handled::Ok
}
fn operator_new(c: &mut ApiContext) -> Handled {
    let size = c.arg(0);
    let p = c.heap_alloc(size);
    c.ret_cdecl(p);
    Handled::Ok
}
fn operator_new_array(c: &mut ApiContext) -> Handled {
    operator_new(c)
}
fn operator_delete(c: &mut ApiContext) -> Handled {
    c.heap_free_block(c.arg(0));
    c.ret_cdecl(0);
    Handled::Ok
}
fn exception_what(c: &mut ApiContext) -> Handled {
    let key = "msvcp120.exception.what";
    let ptr = if let Some(&p) = c.dll_state.get(key) {
        p
    } else {
        let p = c.heap_alloc(23);
        if p != 0 {
            let _ = c.memory.write_bytes(p, b"std::exception\0");
            c.dll_state.insert(key.into(), p);
        }
        p
    };
    c.ret_cdecl(ptr);
    Handled::Ok
}
fn terminate(c: &mut ApiContext) -> Handled {
    // A real terminate aborts the process. Returning keeps the guest debuggable
    // and matches the runtime's non-returning ABI at the trampoline boundary.
    c.ret_cdecl(0);
    Handled::Ok
}

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, super::HandlerFn)] = &[
        // new/delete, including the decorated names emitted by VS2013.
        ("??2@YAPAXI@Z", operator_new),
        ("??_U@YAPAXI@Z", operator_new_array),
        ("??3@YAXPAX@Z", operator_delete),
        ("??_V@YAXPAX@Z", operator_delete),
        ("?_Znwj", operator_new),
        ("?_ZdlPv", operator_delete),
        // std::exception / std::bad_alloc constructors and destructors.
        ("??0exception@std@@QAE@ABV01@@Z", cdecl_this),
        ("??0exception@std@@QAE@XZ", cdecl_this),
        ("??1exception@std@@UAE@XZ", cdecl_this),
        ("?what@exception@std@@UBEPBDXZ", exception_what),
        ("??0bad_alloc@std@@QAE@ABV01@@Z", cdecl_this),
        ("??0bad_alloc@std@@QAE@XZ", cdecl_this),
        ("??1bad_alloc@std@@UAE@XZ", cdecl_this),
        // The compiler imports these as harmless runtime hooks in many games.
        ("_CxxThrowException", cdecl0),
        ("__CxxFrameHandler3", cdecl0),
        ("?terminate@@YAXXZ", terminate),
        ("?unexpected@@YAXXZ", terminate),
        ("?uncaught_exception@std@@YA_NXZ", cdecl0),
        // Frequently referenced stream/string locale helpers. They are no-op
        // fallbacks, but preserve cdecl stack discipline and a valid this ptr.
        ("?init@ios_base@std@@QAEXXZ", cdecl_this),
        (
            "?init@?$basic_ios@DU?$char_traits@D@std@@@std@@QAEXXZ",
            cdecl_this,
        ),
        ("?clear@ios_base@std@@QAEXH@Z", cdecl_this),
        ("?good@ios_base@std@@QBE_NXZ", cdecl0),
        ("?length@?$char_traits@D@std@@SAIPBD@Z", cdecl0),
    ];
    for &(name, f) in fns {
        r.add("msvcp120.dll", name, f);
    }
}
