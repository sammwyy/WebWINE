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
        ("shlwapi.dll", "PathAddBackslashA", path_add_backslash_a),
        ("shlwapi.dll", "PathAddBackslashW", path_add_backslash_w),
        ("shlwapi.dll", "PathGetArgsA", |c| path_get_args(c, false)),
        ("shlwapi.dll", "PathGetArgsW", |c| path_get_args(c, true)),
        ("shlwapi.dll", "SHRegGetValueA", sh_reg_get_value),
        ("shlwapi.dll", "SHRegGetValueW", sh_reg_get_value),
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
        ("shlwapi.dll", "#154", crate::kernel32::strcmp_ni_w),
        ("shlwapi.dll", "#158", |c| {
            let result = c.wstr(c.arg(0)).to_lowercase().cmp(&c.wstr(c.arg(1)).to_lowercase()) as i32;
            c.ret_stdcall(result as u32, 2);
            Handled::Ok
        }),
        ("shlwapi.dll", "#460", crate::kernel32::expand_env_strings_w),
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

fn path_add_backslash_a(c: &mut super::ApiContext) -> Handled {
    let start = c.arg(0);
    let value = c.cstr(start);
    let end = start + value.len() as u32;
    let result = if value.ends_with(['\\', '/']) {
        end
    } else {
        let _ = c.memory.write_bytes(end, &[b'\\', 0]);
        end + 1
    };
    c.ret_stdcall(result, 1);
    Handled::Ok
}

fn path_add_backslash_w(c: &mut super::ApiContext) -> Handled {
    let start = c.arg(0);
    let value = c.wstr(start);
    let end = start + value.encode_utf16().count() as u32 * 2;
    let result = if value.ends_with(['\\', '/']) {
        end
    } else {
        let _ = c.memory.write_u16(end, '\\' as u16);
        let _ = c.memory.write_u16(end + 2, 0);
        end + 2
    };
    c.ret_stdcall(result, 1);
    Handled::Ok
}

fn path_get_args(c: &mut super::ApiContext, wide: bool) -> Handled {
    let start = c.arg(0);
    let value = if wide { c.wstr(start) } else { c.cstr(start) };
    let offset = value.find(char::is_whitespace).unwrap_or(value.len());
    let unit = if wide { 2 } else { 1 };
    let mut pointer = start + offset as u32 * unit;
    while pointer != 0 {
        let blank = if wide { c.memory.read_u16(pointer).unwrap_or(0) == b' ' as u16 }
            else { c.memory.read_u8(pointer).unwrap_or(0) == b' ' };
        if !blank { break }
        pointer += unit;
    }
    c.ret_stdcall(pointer, 1);
    Handled::Ok
}

fn sh_reg_get_value(c: &mut super::ApiContext) -> Handled {
    if c.arg(4) != 0 { let _ = c.memory.write_u32(c.arg(4), 0); }
    if c.arg(6) != 0 { let _ = c.memory.write_u32(c.arg(6), 0); }
    c.ret_stdcall(2, 7);
    Handled::Ok
}
