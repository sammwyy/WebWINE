use super::{ApiContext, Handled, WinApiRegistry};
use crate::util::{register_entries, ret_0_1, ret_0_2, ret_1_5, Entry};
use webwine_api::winapi::context::ApiRuntimeEnv;

pub fn register(r: &mut WinApiRegistry) {
    register_entries(r, ENTRIES);
}

const ENTRIES: &[Entry] = &[
    ("oleaut32.dll", "SysAllocString", sys_alloc_string),
    ("oleaut32.dll", "SysAllocStringLen", sys_alloc_string_len),
    ("oleaut32.dll", "SysFreeString", ret_0_1),
    ("oleaut32.dll", "SysStringLen", sys_string_len),
    ("oleaut32.dll", "SysStringByteLen", sys_string_byte_len),
    ("oleaut32.dll", "VariantInit", variant_init),
    ("oleaut32.dll", "VariantClear", variant_init),
    ("oleaut32.dll", "VariantCopy", ret_0_2),
    ("oleaut32.dll", "OleLoadPicture", ret_1_5),
    ("oleaut32.dll", "#2", sys_alloc_string),
    ("oleaut32.dll", "#6", ret_0_1),
    ("oleaut32.dll", "#7", sys_string_len),
    ("oleaut32.dll", "#184", system_time_to_variant_time),
    ("oleaut32.dll", "#185", variant_time_to_system_time),
];

fn system_time_to_variant_time(c: &mut ApiContext) -> Handled {
    if c.arg(1) != 0 { c.write_bytes(c.arg(1), &[0; 8]); }
    c.return_stdcall(1, 2);
    Handled::Ok
}

fn variant_time_to_system_time(c: &mut ApiContext) -> Handled {
    if c.arg(1) != 0 { c.write_bytes(c.arg(1), &[0; 16]); }
    c.return_stdcall(1, 2);
    Handled::Ok
}

fn sys_alloc_string(c: &mut ApiContext) -> Handled {
    let psz = c.arg(0);
    let s = if psz != 0 { c.read_wstr(psz) } else { String::new() };
    let bstr = alloc_bstr(c, s.encode_utf16());
    c.return_stdcall(bstr, 1);
    Handled::Ok
}

fn sys_alloc_string_len(c: &mut ApiContext) -> Handled {
    let pch = c.arg(0);
    let len = c.arg(1);
    let units: Vec<u16> = (0..len).map(|i| {
        let v = if pch != 0 {
            c.read_u16(pch + i * 2)
        } else {
            0
        };
        v
    }).collect();
    let bstr = alloc_bstr(c, units);
    c.return_stdcall(bstr, 2);
    Handled::Ok
}

fn sys_string_len(c: &mut ApiContext) -> Handled {
    let bstr = c.arg(0);
    let n = if bstr >= 4 {
        c.read_u32(bstr - 4) / 2
    } else {
        0
    };
    c.return_stdcall(n, 1);
    Handled::Ok
}

fn sys_string_byte_len(c: &mut ApiContext) -> Handled {
    let bstr = c.arg(0);
    let n = if bstr >= 4 {
        c.read_u32(bstr - 4)
    } else {
        0
    };
    c.return_stdcall(n, 1);
    Handled::Ok
}

fn variant_init(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p != 0 {
        c.write_bytes(p, &[0u8; 16]);
    }
    c.return_stdcall(0, 1);
    Handled::Ok
}

fn alloc_bstr(c: &mut impl ApiRuntimeEnv, units: impl IntoIterator<Item = u16>) -> u32 {
    let units: Vec<u16> = units.into_iter().collect();
    let byte_len = (units.len() * 2) as u32;
    let buf = c.heap_alloc(byte_len + 6);
    c.write_u32(buf, byte_len);
    for (i, &u) in units.iter().enumerate() {
        c.write_u16(buf + 4 + i as u32 * 2, u);
    }
    c.write_u16(buf + 4 + byte_len, 0);
    buf + 4
}
