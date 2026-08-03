//! ole32.dll — COM apartment init, task allocator, GUID helpers.

use super::{ApiContext, Handled, WinApiRegistry};

const S_OK: u32 = 0;
const S_FALSE: u32 = 1;
const E_NOINTERFACE: u32 = 0x8000_4002;
const E_POINTER: u32 = 0x8000_4003;
const E_NOTIMPL: u32 = 0x8000_4001;
const CO_E_NOTINITIALIZED: u32 = 0x8004_01F0;
const REGDB_E_CLASSNOTREG: u32 = 0x8004_0154;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("ole32.dll", "CoCreateInstance", co_create_instance),
        ("ole32.dll", "CoCreateInstanceEx", co_create_instance_ex),
        ("ole32.dll", "OleUninitialize", ole_uninitialize),
        ("ole32.dll", "CoUninitialize", co_uninitialize),
        ("ole32.dll", "CoTaskMemAlloc", co_task_mem_alloc),
        ("ole32.dll", "CoTaskMemFree", co_task_mem_free),
        ("ole32.dll", "CoTaskMemRealloc", co_task_mem_realloc),
        ("ole32.dll", "CoInitialize", co_initialize),
        ("ole32.dll", "CoInitializeEx", co_initialize_ex),
        ("ole32.dll", "OleInitialize", ole_initialize),
        ("ole32.dll", "CoRegisterClassObject", co_register_class_object),
        ("ole32.dll", "CoRevokeClassObject", co_revoke_class_object),
        ("ole32.dll", "CoCreateGuid", co_create_guid),
        ("ole32.dll", "CoGetMalloc", co_get_malloc),
        ("ole32.dll", "CoGetClassObject", co_get_class_object),
        ("ole32.dll", "CLSIDFromString", clsid_from_string),
        ("ole32.dll", "StringFromGUID2", string_from_guid2),
        ("ole32.dll", "StringFromCLSID", string_from_clsid),
        ("ole32.dll", "ProgIDFromCLSID", prog_id_from_clsid),
        ("ole32.dll", "CLSIDFromProgID", clsid_from_prog_id),
        ("ole32.dll", "CoFreeUnusedLibraries", co_free_unused_libraries),
        ("ole32.dll", "CoFreeUnusedLibrariesEx", co_free_unused_libraries_ex),
        ("ole32.dll", "OleFlushClipboard", ole_flush_clipboard),
        ("ole32.dll", "OleIsCurrentClipboard", ole_is_current_clipboard),
        ("ole32.dll", "CoInitializeSecurity", co_initialize_security),
        ("ole32.dll", "CoSetProxyBlanket", co_set_proxy_blanket),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn co_init_count(c: &mut ApiContext) -> u32 {
    c.dll_state.get("ole32.co_init").copied().unwrap_or(0)
}

fn co_create_instance(c: &mut ApiContext) -> Handled {
    // CoCreateInstance(rclsid, pUnkOuter, dwClsContext, riid, ppv)
    let out = c.arg(4);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    if co_init_count(c) == 0 {
        c.ret_stdcall(CO_E_NOTINITIALIZED, 5);
    } else {
        c.ret_stdcall(REGDB_E_CLASSNOTREG, 5);
    }
    Handled::Ok
}

fn co_create_instance_ex(c: &mut ApiContext) -> Handled {
    // 6 args
    c.ret_stdcall(REGDB_E_CLASSNOTREG, 6);
    Handled::Ok
}

fn ole_uninitialize(c: &mut ApiContext) -> Handled {
    let n = co_init_count(c);
    if n > 0 {
        c.dll_state.insert("ole32.co_init".into(), n - 1);
    }
    c.ret_stdcall(0, 0);
    Handled::Ok
}

fn co_uninitialize(c: &mut ApiContext) -> Handled {
    let n = co_init_count(c);
    if n > 0 {
        c.dll_state.insert("ole32.co_init".into(), n - 1);
    }
    c.ret_stdcall(0, 0);
    Handled::Ok
}

fn co_task_mem_alloc(c: &mut ApiContext) -> Handled {
    let n = c.arg(0);
    let p = if n == 0 { 0 } else { c.heap_alloc(n) };
    c.ret_stdcall(p, 1);
    Handled::Ok
}

fn co_task_mem_free(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p != 0 {
        c.heap_sizes.remove(&p);
    }
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn co_task_mem_realloc(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    let n = c.arg(1);
    let r = c.heap_realloc(p, n);
    c.ret_stdcall(r, 2);
    Handled::Ok
}

fn co_initialize(c: &mut ApiContext) -> Handled {
    // CoInitialize(pvReserved)
    let n = co_init_count(c);
    c.dll_state.insert("ole32.co_init".into(), n + 1);
    c.ret_stdcall(if n == 0 { S_OK } else { S_FALSE }, 1);
    Handled::Ok
}

fn co_initialize_ex(c: &mut ApiContext) -> Handled {
    // CoInitializeEx(pvReserved, dwCoInit)
    let n = co_init_count(c);
    c.dll_state.insert("ole32.co_init".into(), n + 1);
    c.ret_stdcall(if n == 0 { S_OK } else { S_FALSE }, 2);
    Handled::Ok
}

fn ole_initialize(c: &mut ApiContext) -> Handled {
    let n = co_init_count(c);
    c.dll_state.insert("ole32.co_init".into(), n + 1);
    c.ret_stdcall(if n == 0 { S_OK } else { S_FALSE }, 1);
    Handled::Ok
}

fn co_register_class_object(c: &mut ApiContext) -> Handled {
    let cookie = c.arg(4);
    if cookie != 0 {
        let id = c
            .dll_state
            .entry("ole32.class_cookie".into())
            .or_insert(1);
        let v = *id;
        *id = id.wrapping_add(1);
        let _ = c.memory.write_u32(cookie, v);
    }
    c.ret_stdcall(S_OK, 5);
    Handled::Ok
}

fn co_revoke_class_object(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn co_create_guid(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p == 0 {
        c.ret_stdcall(E_POINTER, 1);
        return Handled::Ok;
    }
    // Deterministic pseudo-GUID from pid + sequence (not RFC4122 random, but unique enough).
    let seq = c
        .dll_state
        .entry("ole32.guid_seq".into())
        .or_insert(1);
    let s = *seq;
    *seq = seq.wrapping_add(1);
    let pid = c.pid;
    let _ = c.memory.write_u32(p, 0xA11C_E000u32.wrapping_add(s));
    let _ = c.memory.write_u16(p + 4, (pid & 0xFFFF) as u16);
    let _ = c.memory.write_u16(p + 6, 0x4000 | ((s >> 16) as u16 & 0x0FFF)); // version 4-ish
    let _ = c.memory.write_u8(p + 8, 0x80); // variant
    let _ = c.memory.write_u8(p + 9, (s & 0xFF) as u8);
    let _ = c.memory.write_u16(p + 10, 0x5745); // "WE"
    let _ = c.memory.write_u32(p + 12, 0x424C_494E); // "NIBW" marker
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn co_get_malloc(c: &mut ApiContext) -> Handled {
    let o = c.arg(1);
    if o != 0 {
        let _ = c.memory.write_u32(o, 0);
    }
    c.ret_stdcall(E_NOTIMPL, 2);
    Handled::Ok
}

fn co_get_class_object(c: &mut ApiContext) -> Handled {
    let out = c.arg(4);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(REGDB_E_CLASSNOTREG, 5);
    Handled::Ok
}

fn clsid_from_string(c: &mut ApiContext) -> Handled {
    // CLSIDFromString(lpsz, pclsid)
    let out = c.arg(1);
    if out != 0 {
        let _ = c.memory.write_bytes(out, &[0u8; 16]);
    }
    c.ret_stdcall(E_NOTIMPL, 2);
    Handled::Ok
}

fn string_from_guid2(c: &mut ApiContext) -> Handled {
    // int StringFromGUID2(rguid, lpsz, cchMax)
    let guid = c.arg(0);
    let buf = c.arg(1);
    let cch = c.arg(2) as usize;
    if guid == 0 || buf == 0 || cch < 39 {
        c.ret_stdcall(0, 3);
        return Handled::Ok;
    }
    let d1 = c.memory.read_u32(guid).unwrap_or(0);
    let d2 = c.memory.read_u16(guid + 4).unwrap_or(0);
    let d3 = c.memory.read_u16(guid + 6).unwrap_or(0);
    let mut b = [0u8; 8];
    for i in 0..8 {
        b[i] = c.memory.read_u8(guid + 8 + i as u32).unwrap_or(0);
    }
    let s = format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        d1, d2, d3, b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]
    );
    let units: Vec<u16> = s.encode_utf16().collect();
    for (i, u) in units.iter().enumerate() {
        if i + 1 >= cch {
            break;
        }
        let _ = c.memory.write_u16(buf + i as u32 * 2, *u);
    }
    let n = units.len().min(cch - 1);
    let _ = c.memory.write_u16(buf + n as u32 * 2, 0);
    c.ret_stdcall((n + 1) as u32, 3);
    Handled::Ok
}

fn string_from_clsid(c: &mut ApiContext) -> Handled {
    // HRESULT StringFromCLSID(rclsid, lplpsz) — allocate OLESTR via task mem.
    let guid = c.arg(0);
    let out = c.arg(1);
    if out == 0 {
        c.ret_stdcall(E_POINTER, 2);
        return Handled::Ok;
    }
    // Build the same string as StringFromGUID2, allocate as wide.
    let d1 = c.memory.read_u32(guid).unwrap_or(0);
    let d2 = c.memory.read_u16(guid + 4).unwrap_or(0);
    let d3 = c.memory.read_u16(guid + 6).unwrap_or(0);
    let mut b = [0u8; 8];
    for i in 0..8 {
        b[i] = c.memory.read_u8(guid + 8 + i as u32).unwrap_or(0);
    }
    let s = format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        d1, d2, d3, b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]
    );
    let units: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
    let p = c.heap_alloc(bytes.len() as u32);
    let _ = c.memory.write_bytes(p, &bytes);
    let _ = c.memory.write_u32(out, p);
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn prog_id_from_clsid(c: &mut ApiContext) -> Handled {
    let out = c.arg(1);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(REGDB_E_CLASSNOTREG, 2);
    Handled::Ok
}

fn clsid_from_prog_id(c: &mut ApiContext) -> Handled {
    let out = c.arg(1);
    if out != 0 {
        let _ = c.memory.write_bytes(out, &[0u8; 16]);
    }
    c.ret_stdcall(REGDB_E_CLASSNOTREG, 2);
    Handled::Ok
}

fn co_free_unused_libraries(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 0);
    Handled::Ok
}

fn co_free_unused_libraries_ex(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn ole_flush_clipboard(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 0);
    Handled::Ok
}

fn ole_is_current_clipboard(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_FALSE, 1);
    Handled::Ok
}

fn co_initialize_security(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 9);
    Handled::Ok
}

fn co_set_proxy_blanket(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(E_NOINTERFACE, 8);
    Handled::Ok
}
