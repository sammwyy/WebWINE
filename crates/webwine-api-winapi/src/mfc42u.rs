use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[

        ("mfc42u.dll", "#1165", |c| {
            let ptr = c.heap_alloc(256); // give it a nice 256 byte chunk to write to
            c.ret_stdcall(ptr, 1); // guess: 1 arg? The log said "cleaned 1 args"
            Handled::Ok
        }),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
