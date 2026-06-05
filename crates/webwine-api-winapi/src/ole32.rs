use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[

        ("ole32.dll", "CoCreateInstance", |c| {
            let out = c.arg(4);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0x8004_0154, 5);
            Handled::Ok
        }),
        ("ole32.dll", "OleUninitialize", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("ole32.dll", "CoUninitialize", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        // COM task allocator â€” must return real memory or C++ code throws bad_alloc.
        ("ole32.dll", "CoTaskMemAlloc", |c| {
            let n = c.arg(0);
            let p = c.heap_alloc(n);
            c.ret_stdcall(p, 1);
            Handled::Ok
        }),
        ("ole32.dll", "CoTaskMemFree", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ole32.dll", "CoTaskMemRealloc", |c| {
            let p = c.arg(0);
            let n = c.arg(1);
            let r = c.heap_realloc(p, n);
            c.ret_stdcall(r, 2);
            Handled::Ok
        }),
        ("ole32.dll", "CoInitialize", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }), // S_OK
        ("ole32.dll", "CoInitializeEx", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("ole32.dll", "OleInitialize", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ole32.dll", "CoCreateGuid", |c| {
            let p = c.arg(0);
            if p != 0 {
                let _ = c.memory.write_bytes(p, &[0u8; 16]);
            }
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ole32.dll", "CoGetMalloc", |c| {
            let o = c.arg(1);
            if o != 0 {
                let _ = c.memory.write_u32(o, 0);
            }
            c.ret_stdcall(0x8000_4001u32, 2);
            Handled::Ok
        }),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
