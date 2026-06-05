use super::WinApiRegistry;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("ucrtbase.dll", "exit", crate::msvcrt::exit),
        ("ucrtbase.dll", "_exit", crate::msvcrt::exit),
        ("ucrtbase.dll", "malloc", crate::msvcrt::malloc),
        ("ucrtbase.dll", "free", crate::msvcrt::stub_void_1),
        ("ucrtbase.dll", "printf", crate::msvcrt::printf),
        ("ucrtbase.dll", "puts", crate::msvcrt::puts),
        ("ucrtbase.dll", "__stdio_common_vfprintf", crate::msvcrt::stdio_vfprintf),
        ("ucrtbase.dll", "_initterm", crate::msvcrt::initterm),
        ("ucrtbase.dll", "_initterm_e", crate::msvcrt::initterm_e),
        ("ucrtbase.dll", "__acrt_iob_func", crate::msvcrt::acrt_iob),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
