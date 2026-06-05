use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        // clean the exact arg counts (a wrong count corrupts the guest stack).
        // ERROR_FILE_NOT_FOUND (2) makes apps fall back to defaults.
        ("advapi32.dll", "RegOpenKeyExA", crate::kernel32::reg_open_key),
        ("advapi32.dll", "RegOpenKeyExW", crate::kernel32::reg_open_key),
        ("advapi32.dll", "RegOpenKeyA", |c| {
            let o = c.arg(2);
            if o != 0 {
                let _ = c.memory.write_u32(o, 0);
            }
            c.ret_stdcall(2, 3);
            Handled::Ok
        }),
        ("advapi32.dll", "RegQueryValueExA", crate::kernel32::reg_query_value),
        ("advapi32.dll", "RegQueryValueExW", |c| {
            c.ret_stdcall(2, 6);
            Handled::Ok
        }),
        ("advapi32.dll", "RegCreateKeyA", |c| {
            let o = c.arg(2);
            if o != 0 {
                let _ = c.memory.write_u32(o, 0);
            }
            c.ret_stdcall(2, 3);
            Handled::Ok
        }),
        ("advapi32.dll", "RegCreateKeyW", |c| {
            let o = c.arg(2);
            if o != 0 {
                let _ = c.memory.write_u32(o, 0);
            }
            c.ret_stdcall(2, 3);
            Handled::Ok
        }),
        ("advapi32.dll", "RegCreateKeyExA", |c| {
            let o = c.arg(7);
            if o != 0 {
                let _ = c.memory.write_u32(o, 0);
            }
            c.ret_stdcall(2, 9);
            Handled::Ok
        }),
        ("advapi32.dll", "RegSetValueExA", |c| {
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        ("advapi32.dll", "RegSetValueExW", |c| {
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        ("advapi32.dll", "RegCloseKey", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("advapi32.dll", "RegOpenKeyW", |c| {
            let o = c.arg(2);
            if o != 0 {
                let _ = c.memory.write_u32(o, 0);
            }
            c.ret_stdcall(2, 3);
            Handled::Ok
        }),
        ("advapi32.dll", "RegCreateKeyExW", |c| {
            let o = c.arg(7);
            if o != 0 {
                let _ = c.memory.write_u32(o, 0);
            }
            c.ret_stdcall(2, 9);
            Handled::Ok
        }),
        ("advapi32.dll", "RegDeleteValueW", |c| {
            c.ret_stdcall(2, 2);
            Handled::Ok
        }),
        ("advapi32.dll", "RegDeleteKeyW", |c| {
            c.ret_stdcall(2, 2);
            Handled::Ok
        }),
        ("advapi32.dll", "RegEnumValueW", |c| {
            c.ret_stdcall(0x103, 8);
            Handled::Ok
        }), // ERROR_NO_MORE_ITEMS
        ("advapi32.dll", "RegEnumKeyExW", |c| {
            c.ret_stdcall(0x103, 8);
            Handled::Ok
        }),
        ("advapi32.dll", "RegQueryInfoKeyW", |c| {
            c.ret_stdcall(0, 12);
            Handled::Ok
        }),
        ("advapi32.dll", "RegQueryValueW", |c| {
            c.ret_stdcall(2, 4);
            Handled::Ok
        }),
        ("advapi32.dll", "RegNotifyChangeKeyValue", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("advapi32.dll", "RegFlushKey", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("advapi32.dll", "GetUserNameA", crate::kernel32::get_user_name_a),
        ("advapi32.dll", "GetUserNameW", crate::kernel32::get_user_name_w),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
