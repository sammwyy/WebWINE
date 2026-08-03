use super::WinApiRegistry;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("vcruntime140.dll", "memcpy", crate::msvcrt::memcpy),
        ("vcruntime140.dll", "memset", crate::msvcrt::memset),
        ("vcruntime140.dll", "memmove", crate::msvcrt::memcpy),
        ("vcruntime140.dll", "__C_specific_handler", crate::msvcrt::except_handler_cdecl_1),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
