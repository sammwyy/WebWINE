use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[

        ("shlwapi.dll", "SHGetValueA", |c| {
            c.ret_stdcall(2, 6);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHGetValueW", |c| {
            c.ret_stdcall(2, 6);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHSetValueA", |c| {
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHSetValueW", |c| {
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        (
            "shlwapi.dll",
            "SHRegGetBoolUSValueA",
            crate::kernel32::sh_reg_get_bool_us_value,
        ),
        (
            "shlwapi.dll",
            "SHRegGetBoolUSValueW",
            crate::kernel32::sh_reg_get_bool_us_value,
        ),
        ("shlwapi.dll", "SHRegGetUSValueA", |c| {
            c.ret_stdcall(2, 8);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHRegGetUSValueW", |c| {
            c.ret_stdcall(2, 8);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHRegCreateUSKeyA", crate::kernel32::sh_reg_create_us_key),
        ("shlwapi.dll", "SHRegCreateUSKeyW", crate::kernel32::sh_reg_create_us_key),
        ("shlwapi.dll", "SHRegWriteUSValueA", |c| {
            c.ret_stdcall(0, 7);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHRegWriteUSValueW", |c| {
            c.ret_stdcall(0, 7);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHRegCloseUSKey", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "PathFindFileNameA", crate::kernel32::path_find_file_name_a),
        ("shlwapi.dll", "PathFindFileNameW", crate::kernel32::path_find_file_name_w),
        ("shlwapi.dll", "PathRemoveArgsA", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "PathRemoveArgsW", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "PathRemoveBlanksA", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "PathRemoveBlanksW", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "StrCmpNIA", crate::kernel32::strcmp_ni_a),
        ("shlwapi.dll", "StrCmpNIW", crate::kernel32::strcmp_ni_w),
        ("shlwapi.dll", "#241", |c| {
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        ("shlwapi.dll", "#433", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("shlwapi.dll", "#437", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "#563", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("shlwapi.dll", "#618", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "#16", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHCreateThreadRef", crate::kernel32::sh_create_thread_ref),
        ("shlwapi.dll", "SHSetThreadRef", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHGetThreadRef", |c| {
            let out = c.arg(0);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0x8000_4005, 1);
            Handled::Ok
        }),
        ("shlwapi.dll", "SHReleaseThreadRef", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
