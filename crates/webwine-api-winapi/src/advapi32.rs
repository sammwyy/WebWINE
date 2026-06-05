use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::registry::{RegValue, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_QWORD, REG_SZ};

// Win32 error codes.
const ERROR_SUCCESS: u32 = 0;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_MORE_DATA: u32 = 234;
const ERROR_NO_MORE_ITEMS: u32 = 259;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("advapi32.dll", "RegOpenKeyExA", |c| reg_open_ex(c, false)),
        ("advapi32.dll", "RegOpenKeyExW", |c| reg_open_ex(c, true)),
        ("advapi32.dll", "RegOpenKeyA", |c| reg_open(c, false)),
        ("advapi32.dll", "RegOpenKeyW", |c| reg_open(c, true)),
        ("advapi32.dll", "RegCreateKeyExA", |c| reg_create_ex(c, false)),
        ("advapi32.dll", "RegCreateKeyExW", |c| reg_create_ex(c, true)),
        ("advapi32.dll", "RegCreateKeyA", |c| reg_create(c, false)),
        ("advapi32.dll", "RegCreateKeyW", |c| reg_create(c, true)),
        ("advapi32.dll", "RegQueryValueExA", |c| reg_query_value_ex(c, false)),
        ("advapi32.dll", "RegQueryValueExW", |c| reg_query_value_ex(c, true)),
        ("advapi32.dll", "RegSetValueExA", |c| reg_set_value_ex(c, false)),
        ("advapi32.dll", "RegSetValueExW", |c| reg_set_value_ex(c, true)),
        ("advapi32.dll", "RegDeleteValueA", |c| reg_delete_value(c, false)),
        ("advapi32.dll", "RegDeleteValueW", |c| reg_delete_value(c, true)),
        ("advapi32.dll", "RegDeleteKeyA", |c| reg_delete_key(c, false)),
        ("advapi32.dll", "RegDeleteKeyW", |c| reg_delete_key(c, true)),
        ("advapi32.dll", "RegEnumKeyExA", |c| reg_enum_key_ex(c, false)),
        ("advapi32.dll", "RegEnumKeyExW", |c| reg_enum_key_ex(c, true)),
        ("advapi32.dll", "RegEnumValueA", |c| reg_enum_value(c, false)),
        ("advapi32.dll", "RegEnumValueW", |c| reg_enum_value(c, true)),
        ("advapi32.dll", "RegQueryInfoKeyA", reg_query_info_key),
        ("advapi32.dll", "RegQueryInfoKeyW", reg_query_info_key),
        ("advapi32.dll", "RegCloseKey", |c| {
            let h = c.arg(0);
            c.registry.close(h);
            c.ret_stdcall(ERROR_SUCCESS, 1);
            Handled::Ok
        }),
        ("advapi32.dll", "RegNotifyChangeKeyValue", |c| {
            c.ret_stdcall(ERROR_SUCCESS, 5);
            Handled::Ok
        }),
        ("advapi32.dll", "RegFlushKey", |c| {
            c.ret_stdcall(ERROR_SUCCESS, 1);
            Handled::Ok
        }),
        ("advapi32.dll", "GetUserNameA", crate::kernel32::get_user_name_a),
        ("advapi32.dll", "GetUserNameW", crate::kernel32::get_user_name_w),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

// ---- string helpers ----

fn read_str(c: &ApiContext, ptr: u32, wide: bool) -> String {
    if ptr == 0 {
        String::new()
    } else if wide {
        c.wstr(ptr)
    } else {
        c.cstr(ptr)
    }
}

/// Write a string + null terminator to `ptr`, capped at `cap` *characters*
/// (including the terminator). Returns the character count written (excluding
/// the terminator).
fn write_str_capped(c: &mut ApiContext, ptr: u32, s: &str, wide: bool, cap: usize) -> usize {
    if wide {
        let mut units: Vec<u16> = s.encode_utf16().collect();
        if cap > 0 && units.len() + 1 > cap {
            units.truncate(cap.saturating_sub(1));
        }
        let written = units.len();
        let mut bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
        bytes.extend_from_slice(&[0, 0]);
        let _ = c.memory.write_bytes(ptr, &bytes);
        written
    } else {
        let mut bytes: Vec<u8> = s.bytes().collect();
        if cap > 0 && bytes.len() + 1 > cap {
            bytes.truncate(cap.saturating_sub(1));
        }
        let written = bytes.len();
        bytes.push(0);
        let _ = c.memory.write_bytes(ptr, &bytes);
        written
    }
}

fn decode_wide(b: &[u8]) -> String {
    let u: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&x| x != 0)
        .collect();
    String::from_utf16_lossy(&u)
}

/// Encode a value into the raw bytes a query returns, honoring ANSI vs wide for
/// string types.
fn value_out_bytes(v: &RegValue, wide: bool) -> Vec<u8> {
    if wide {
        return v.to_bytes();
    }
    match v {
        RegValue::Sz(s) | RegValue::ExpandSz(s) => {
            let mut b = s.clone().into_bytes();
            b.push(0);
            b
        }
        RegValue::MultiSz(list) => {
            let mut b = Vec::new();
            for s in list {
                b.extend_from_slice(s.as_bytes());
                b.push(0);
            }
            b.push(0);
            b
        }
        _ => v.to_bytes(),
    }
}

// ---- handlers ----

fn reg_open_ex(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let sub = read_str(c, c.arg(1), wide);
    let phk = c.arg(4);
    match c.registry.open(hkey, &sub) {
        Some(h) => {
            if phk != 0 {
                let _ = c.memory.write_u32(phk, h);
            }
            c.ret_stdcall(ERROR_SUCCESS, 5);
        }
        None => {
            if phk != 0 {
                let _ = c.memory.write_u32(phk, 0);
            }
            c.ret_stdcall(ERROR_FILE_NOT_FOUND, 5);
        }
    }
    Handled::Ok
}

fn reg_open(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let sub = read_str(c, c.arg(1), wide);
    let phk = c.arg(2);
    let code = match c.registry.open(hkey, &sub) {
        Some(h) => {
            if phk != 0 {
                let _ = c.memory.write_u32(phk, h);
            }
            ERROR_SUCCESS
        }
        None => {
            if phk != 0 {
                let _ = c.memory.write_u32(phk, 0);
            }
            ERROR_FILE_NOT_FOUND
        }
    };
    c.ret_stdcall(code, 3);
    Handled::Ok
}

fn reg_create_ex(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let sub = read_str(c, c.arg(1), wide);
    let existed = c.registry.path_of_handle(hkey).map(|base| {
        let full = if sub.is_empty() { base } else { format!("{base}\\{sub}") };
        c.registry.key_exists(&full)
    });
    let phk = c.arg(7);
    let disp = c.arg(8);
    match c.registry.create(hkey, &sub) {
        Some(h) => {
            if phk != 0 {
                let _ = c.memory.write_u32(phk, h);
            }
            if disp != 0 {
                // REG_CREATED_NEW_KEY=1, REG_OPENED_EXISTING_KEY=2
                let d = if existed == Some(true) { 2 } else { 1 };
                let _ = c.memory.write_u32(disp, d);
            }
            c.ret_stdcall(ERROR_SUCCESS, 9);
        }
        None => {
            if phk != 0 {
                let _ = c.memory.write_u32(phk, 0);
            }
            c.ret_stdcall(ERROR_FILE_NOT_FOUND, 9);
        }
    }
    Handled::Ok
}

fn reg_create(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let sub = read_str(c, c.arg(1), wide);
    let phk = c.arg(2);
    let code = match c.registry.create(hkey, &sub) {
        Some(h) => {
            if phk != 0 {
                let _ = c.memory.write_u32(phk, h);
            }
            ERROR_SUCCESS
        }
        None => ERROR_FILE_NOT_FOUND,
    };
    c.ret_stdcall(code, 3);
    Handled::Ok
}

fn reg_query_value_ex(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let name = read_str(c, c.arg(1), wide);
    let type_ptr = c.arg(3);
    let data_ptr = c.arg(4);
    let cb_ptr = c.arg(5);

    let found = c.registry.query(hkey, &name).map(|v| (v.type_id(), value_out_bytes(v, wide)));
    let Some((tid, bytes)) = found else {
        c.ret_stdcall(ERROR_FILE_NOT_FOUND, 6);
        return Handled::Ok;
    };

    if type_ptr != 0 {
        let _ = c.memory.write_u32(type_ptr, tid);
    }
    let avail = if cb_ptr != 0 { c.memory.read_u32(cb_ptr).unwrap_or(0) } else { 0 };
    let need = bytes.len() as u32;
    let code = if data_ptr == 0 {
        // Size query only.
        ERROR_SUCCESS
    } else if cb_ptr != 0 && avail < need {
        ERROR_MORE_DATA
    } else {
        let _ = c.memory.write_bytes(data_ptr, &bytes);
        ERROR_SUCCESS
    };
    if cb_ptr != 0 {
        let _ = c.memory.write_u32(cb_ptr, need);
    }
    c.ret_stdcall(code, 6);
    Handled::Ok
}

fn reg_set_value_ex(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let name = read_str(c, c.arg(1), wide);
    let dtype = c.arg(3);
    let data_ptr = c.arg(4);
    let cb = c.arg(5);
    let raw = if data_ptr != 0 && cb > 0 {
        c.memory.read_bytes(data_ptr, cb as usize).unwrap_or_default()
    } else {
        Vec::new()
    };

    let value = match dtype {
        REG_DWORD => RegValue::Dword(u32::from_le_bytes([
            *raw.first().unwrap_or(&0),
            *raw.get(1).unwrap_or(&0),
            *raw.get(2).unwrap_or(&0),
            *raw.get(3).unwrap_or(&0),
        ])),
        REG_QWORD => {
            let mut b = [0u8; 8];
            for (i, v) in raw.iter().take(8).enumerate() {
                b[i] = *v;
            }
            RegValue::Qword(u64::from_le_bytes(b))
        }
        REG_SZ | REG_EXPAND_SZ => {
            let s = if wide { decode_wide(&raw) } else {
                let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
                String::from_utf8_lossy(&raw[..end]).into_owned()
            };
            if dtype == REG_EXPAND_SZ { RegValue::ExpandSz(s) } else { RegValue::Sz(s) }
        }
        REG_MULTI_SZ => {
            let parts: Vec<String> = if wide {
                decode_multi_wide(&raw)
            } else {
                raw.split(|&b| b == 0)
                    .filter(|s| !s.is_empty())
                    .map(|s| String::from_utf8_lossy(s).into_owned())
                    .collect()
            };
            RegValue::MultiSz(parts)
        }
        _ => RegValue::Binary(raw),
    };

    c.registry.set(hkey, &name, value);
    c.ret_stdcall(ERROR_SUCCESS, 6);
    Handled::Ok
}

fn decode_multi_wide(b: &[u8]) -> Vec<String> {
    let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for u in units {
        if u == 0 {
            if cur.is_empty() {
                break; // double-null terminator
            }
            out.push(String::from_utf16_lossy(&cur));
            cur.clear();
        } else {
            cur.push(u);
        }
    }
    out
}

fn reg_delete_value(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let name = read_str(c, c.arg(1), wide);
    let code = if c.registry.delete_value(hkey, &name) { ERROR_SUCCESS } else { ERROR_FILE_NOT_FOUND };
    c.ret_stdcall(code, 2);
    Handled::Ok
}

fn reg_delete_key(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let sub = read_str(c, c.arg(1), wide);
    let code = if c.registry.delete_subkey(hkey, &sub) { ERROR_SUCCESS } else { ERROR_FILE_NOT_FOUND };
    c.ret_stdcall(code, 2);
    Handled::Ok
}

// RegEnumKeyEx(hKey, dwIndex, lpName, lpcchName, lpReserved, lpClass, lpcchClass, lpftLastWriteTime)
fn reg_enum_key_ex(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let index = c.arg(1);
    let name_ptr = c.arg(2);
    let cch_ptr = c.arg(3);
    match c.registry.enum_key(hkey, index) {
        Some(name) => {
            let cap = if cch_ptr != 0 { c.memory.read_u32(cch_ptr).unwrap_or(0) as usize } else { 0 };
            let written = write_str_capped(c, name_ptr, &name, wide, cap);
            if cch_ptr != 0 {
                let _ = c.memory.write_u32(cch_ptr, written as u32);
            }
            c.ret_stdcall(ERROR_SUCCESS, 8);
        }
        None => c.ret_stdcall(ERROR_NO_MORE_ITEMS, 8),
    }
    Handled::Ok
}

// RegEnumValue(hKey, dwIndex, lpValueName, lpcchValueName, lpReserved, lpType, lpData, lpcbData)
fn reg_enum_value(c: &mut ApiContext, wide: bool) -> Handled {
    let hkey = c.arg(0);
    let index = c.arg(1);
    let name_ptr = c.arg(2);
    let cch_ptr = c.arg(3);
    let type_ptr = c.arg(5);
    let data_ptr = c.arg(6);
    let cb_ptr = c.arg(7);

    let Some((name, value)) = c.registry.enum_value(hkey, index) else {
        c.ret_stdcall(ERROR_NO_MORE_ITEMS, 8);
        return Handled::Ok;
    };
    let tid = value.type_id();
    let bytes = value_out_bytes(&value, wide);

    let cap = if cch_ptr != 0 { c.memory.read_u32(cch_ptr).unwrap_or(0) as usize } else { 0 };
    let written = write_str_capped(c, name_ptr, &name, wide, cap);
    if cch_ptr != 0 {
        let _ = c.memory.write_u32(cch_ptr, written as u32);
    }
    if type_ptr != 0 {
        let _ = c.memory.write_u32(type_ptr, tid);
    }
    let need = bytes.len() as u32;
    if data_ptr != 0 {
        let avail = if cb_ptr != 0 { c.memory.read_u32(cb_ptr).unwrap_or(0) } else { 0 };
        if cb_ptr != 0 && avail < need {
            if cb_ptr != 0 {
                let _ = c.memory.write_u32(cb_ptr, need);
            }
            c.ret_stdcall(ERROR_MORE_DATA, 8);
            return Handled::Ok;
        }
        let _ = c.memory.write_bytes(data_ptr, &bytes);
    }
    if cb_ptr != 0 {
        let _ = c.memory.write_u32(cb_ptr, need);
    }
    c.ret_stdcall(ERROR_SUCCESS, 8);
    Handled::Ok
}

// RegQueryInfoKey(hKey, lpClass, lpcchClass, lpReserved, lpcSubKeys,
//   lpcbMaxSubKeyLen, lpcbMaxClassLen, lpcValues, lpcbMaxValueNameLen,
//   lpcbMaxValueLen, lpcbSecurityDescriptor, lpftLastWriteTime) — 12 args.
fn reg_query_info_key(c: &mut ApiContext) -> Handled {
    let hkey = c.arg(0);
    let (subkeys, values) = c
        .registry
        .path_of_handle(hkey)
        .map(|p| (c.registry.subkeys(&p).len() as u32, c.registry.values_of(&p).map(|v| v.len()).unwrap_or(0) as u32))
        .unwrap_or((0, 0));
    let sub_ptr = c.arg(4);
    let val_ptr = c.arg(7);
    if sub_ptr != 0 {
        let _ = c.memory.write_u32(sub_ptr, subkeys);
    }
    if val_ptr != 0 {
        let _ = c.memory.write_u32(val_ptr, values);
    }
    c.ret_stdcall(ERROR_SUCCESS, 12);
    Handled::Ok
}
