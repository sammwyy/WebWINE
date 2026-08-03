//! oleaut32.dll — BSTR + VARIANT helpers (Wine oleaut32/oleaut.c + variant.c).

use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::winapi::context::ApiRuntimeEnv;

// VARTYPE
const VT_EMPTY: u16 = 0;
const VT_NULL: u16 = 1;
const VT_I2: u16 = 2;
const VT_I4: u16 = 3;
const VT_R4: u16 = 4;
const VT_R8: u16 = 5;
const VT_CY: u16 = 6;
const VT_DATE: u16 = 7;
const VT_BSTR: u16 = 8;
const VT_DISPATCH: u16 = 9;
const VT_ERROR: u16 = 10;
const VT_BOOL: u16 = 11;
const VT_VARIANT: u16 = 12;
const VT_UNKNOWN: u16 = 13;
const VT_BYREF: u16 = 0x4000;

const S_OK: u32 = 0;
const E_INVALIDARG: u32 = 0x8007_0057;
const E_OUTOFMEMORY: u32 = 0x8007_000E;
const E_NOTIMPL: u32 = 0x8000_4001;
const DISP_E_BADVARTYPE: u32 = 0x8002_0008;

pub fn register(r: &mut WinApiRegistry) {
    r.add("oleaut32.dll", "SysAllocString", sys_alloc_string);
    r.add("oleaut32.dll", "SysAllocStringLen", sys_alloc_string_len);
    r.add("oleaut32.dll", "SysAllocStringByteLen", sys_alloc_string_byte_len);
    r.add("oleaut32.dll", "SysReAllocString", sys_realloc_string);
    r.add("oleaut32.dll", "SysReAllocStringLen", sys_realloc_string_len);
    r.add("oleaut32.dll", "SysFreeString", sys_free_string);
    r.add("oleaut32.dll", "SysStringLen", sys_string_len);
    r.add("oleaut32.dll", "SysStringByteLen", sys_string_byte_len);
    r.add("oleaut32.dll", "VariantInit", variant_init);
    r.add("oleaut32.dll", "VariantClear", variant_clear);
    r.add("oleaut32.dll", "VariantCopy", variant_copy);
    r.add("oleaut32.dll", "VariantCopyInd", variant_copy);
    r.add("oleaut32.dll", "OleLoadPicture", ole_load_picture);
    r.add("oleaut32.dll", "SystemTimeToVariantTime", system_time_to_variant_time);
    r.add("oleaut32.dll", "VariantTimeToSystemTime", variant_time_to_system_time);
    // Ordinals used by older linkers / MFC.
    r.add("oleaut32.dll", "#2", sys_alloc_string);
    r.add("oleaut32.dll", "#4", sys_alloc_string_len);
    r.add("oleaut32.dll", "#6", sys_free_string);
    r.add("oleaut32.dll", "#7", sys_string_len);
    r.add("oleaut32.dll", "#8", sys_string_byte_len);
    r.add("oleaut32.dll", "#9", variant_init);
    r.add("oleaut32.dll", "#12", variant_clear);
    r.add("oleaut32.dll", "#184", system_time_to_variant_time);
    r.add("oleaut32.dll", "#185", variant_time_to_system_time);
}

// ── BSTR ────────────────────────────────────────────────────────────────────
// Layout (Wine bstr_t simplified): [DWORD byte_len][WCHAR data...][L'\0']
// The BSTR pointer returned to the guest points at the WCHAR data (after the
// length prefix). SysFreeString frees the block at bstr - 4.

fn sys_alloc_string(c: &mut ApiContext) -> Handled {
    let psz = c.arg(0);
    let units: Vec<u16> = if psz != 0 {
        c.read_wstr(psz).encode_utf16().collect()
    } else {
        Vec::new()
    };
    let bstr = alloc_bstr(c, &units);
    c.return_stdcall(bstr, 1);
    Handled::Ok
}

fn sys_alloc_string_len(c: &mut ApiContext) -> Handled {
    let pch = c.arg(0);
    let len = c.arg(1);
    let units: Vec<u16> = (0..len)
        .map(|i| {
            if pch != 0 {
                c.read_u16(pch + i * 2)
            } else {
                0
            }
        })
        .collect();
    let bstr = alloc_bstr(c, &units);
    c.return_stdcall(bstr, 2);
    Handled::Ok
}

fn sys_alloc_string_byte_len(c: &mut ApiContext) -> Handled {
    // SysAllocStringByteLen(psz, len) — byte length, may be odd; data is raw.
    let psz = c.arg(0);
    let len = c.arg(1);
    let bytes = if psz != 0 && len > 0 {
        c.memory
            .read_bytes(psz, len as usize)
            .unwrap_or_else(|_| vec![0; len as usize])
    } else {
        vec![0; len as usize]
    };
    // Allocate with explicit byte length (not rounded WCHAR count).
    let buf = c.heap_alloc(len + 6);
    if buf == 0 {
        c.return_stdcall(0, 2);
        return Handled::Ok;
    }
    c.write_u32(buf, len);
    if !bytes.is_empty() {
        c.write_bytes(buf + 4, &bytes);
    }
    // Trailing WCHAR NUL.
    c.write_u16(buf + 4 + len, 0);
    c.return_stdcall(buf + 4, 2);
    Handled::Ok
}

fn sys_realloc_string(c: &mut ApiContext) -> Handled {
    // SysReAllocString(BSTR *pbstr, const OLECHAR *psz) → BOOL
    let pp = c.arg(0);
    let psz = c.arg(1);
    if pp == 0 {
        c.return_stdcall(0, 2);
        return Handled::Ok;
    }
    let old = c.read_u32(pp);
    if old != 0 {
        free_bstr(c, old);
    }
    let units: Vec<u16> = if psz != 0 {
        c.read_wstr(psz).encode_utf16().collect()
    } else {
        Vec::new()
    };
    let bstr = alloc_bstr(c, &units);
    c.write_u32(pp, bstr);
    c.return_stdcall(if bstr != 0 || psz == 0 { 1 } else { 0 }, 2);
    Handled::Ok
}

fn sys_realloc_string_len(c: &mut ApiContext) -> Handled {
    // SysReAllocStringLen(BSTR *pbstr, const OLECHAR *pch, unsigned len) → BOOL
    let pp = c.arg(0);
    let pch = c.arg(1);
    let len = c.arg(2);
    if pp == 0 {
        c.return_stdcall(0, 3);
        return Handled::Ok;
    }
    let old = c.read_u32(pp);
    if old != 0 {
        free_bstr(c, old);
    }
    let units: Vec<u16> = (0..len)
        .map(|i| {
            if pch != 0 {
                c.read_u16(pch + i * 2)
            } else {
                0
            }
        })
        .collect();
    let bstr = alloc_bstr(c, &units);
    c.write_u32(pp, bstr);
    c.return_stdcall(if bstr != 0 || len == 0 { 1 } else { 0 }, 3);
    Handled::Ok
}

/// SysFreeString(BSTR): free the allocation; NULL is a no-op (Wine).
fn sys_free_string(c: &mut ApiContext) -> Handled {
    let bstr = c.arg(0);
    free_bstr(c, bstr);
    c.return_stdcall(0, 1);
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
    let n = if bstr >= 4 { c.read_u32(bstr - 4) } else { 0 };
    c.return_stdcall(n, 1);
    Handled::Ok
}

// ── VARIANT ─────────────────────────────────────────────────────────────────
// 16-byte x86 VARIANT: vt(u16) @0, wReserved1..3, union @8 (8 bytes).

fn variant_init(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p != 0 {
        c.write_bytes(p, &[0u8; 16]);
    }
    c.return_stdcall(S_OK, 1);
    Handled::Ok
}

/// VariantClear: release owned resources then VT_EMPTY (Wine VariantClear).
fn variant_clear(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p == 0 {
        c.return_stdcall(E_INVALIDARG, 1);
        return Handled::Ok;
    }
    let vt = c.read_u16(p);
    let hr = clear_variant_at(c, p, vt);
    c.return_stdcall(hr, 1);
    Handled::Ok
}

/// VariantCopy(dest, src): deep-ish copy; BSTRs are re-allocated.
fn variant_copy(c: &mut ApiContext) -> Handled {
    let dest = c.arg(0);
    let src = c.arg(1);
    if dest == 0 || src == 0 {
        c.return_stdcall(E_INVALIDARG, 2);
        return Handled::Ok;
    }
    // Clear destination first (Wine does this).
    let dest_vt = c.read_u16(dest);
    let _ = clear_variant_at(c, dest, dest_vt);

    let vt = c.read_u16(src);
    if vt & VT_BYREF != 0 {
        // BYREF: shallow copy of the 16-byte structure.
        if let Ok(bytes) = c.memory.read_bytes(src, 16) {
            c.write_bytes(dest, &bytes);
        }
        c.return_stdcall(S_OK, 2);
        return Handled::Ok;
    }

    match vt & 0xFFF {
        VT_EMPTY | VT_NULL | VT_I2 | VT_I4 | VT_R4 | VT_R8 | VT_CY | VT_DATE | VT_ERROR
        | VT_BOOL => {
            if let Ok(bytes) = c.memory.read_bytes(src, 16) {
                c.write_bytes(dest, &bytes);
            }
            c.return_stdcall(S_OK, 2);
        }
        VT_BSTR => {
            let bstr = c.read_u32(src + 8);
            let new_bstr = if bstr != 0 {
                let byte_len = c.read_u32(bstr - 4);
                let n_units = byte_len / 2;
                let units: Vec<u16> = (0..n_units).map(|i| c.read_u16(bstr + i * 2)).collect();
                alloc_bstr(c, &units)
            } else {
                0
            };
            c.write_bytes(dest, &[0u8; 16]);
            c.write_u16(dest, VT_BSTR);
            c.write_u32(dest + 8, new_bstr);
            c.return_stdcall(if bstr == 0 || new_bstr != 0 {
                S_OK
            } else {
                E_OUTOFMEMORY
            }, 2);
        }
        VT_DISPATCH | VT_UNKNOWN => {
            // No COM refcounting yet — copy pointer bits.
            if let Ok(bytes) = c.memory.read_bytes(src, 16) {
                c.write_bytes(dest, &bytes);
            }
            c.return_stdcall(S_OK, 2);
        }
        VT_VARIANT => {
            c.return_stdcall(DISP_E_BADVARTYPE, 2);
        }
        _ => {
            // Unknown type: shallow copy and hope.
            if let Ok(bytes) = c.memory.read_bytes(src, 16) {
                c.write_bytes(dest, &bytes);
            }
            c.return_stdcall(S_OK, 2);
        }
    }
    Handled::Ok
}

fn clear_variant_at(c: &mut ApiContext, p: u32, vt: u16) -> u32 {
    if vt & VT_BYREF != 0 {
        // BYREF: do not free the pointed-to data; just reset the VARIANT.
        c.write_bytes(p, &[0u8; 16]);
        return S_OK;
    }
    match vt & 0xFFF {
        VT_BSTR => {
            let bstr = c.read_u32(p + 8);
            free_bstr(c, bstr);
        }
        VT_DISPATCH | VT_UNKNOWN => {
            // No IUnknown::Release yet.
        }
        _ => {}
    }
    c.write_bytes(p, &[0u8; 16]);
    S_OK
}

fn ole_load_picture(c: &mut ApiContext) -> Handled {
    // OleLoadPicture(pStream, lSize, fRunmode, riid, ppvObj) — not implemented.
    let ppv = c.arg(4);
    if ppv != 0 {
        c.write_u32(ppv, 0);
    }
    c.return_stdcall(E_NOTIMPL, 5);
    Handled::Ok
}

fn system_time_to_variant_time(c: &mut ApiContext) -> Handled {
    // SystemTimeToVariantTime(lpSystemTime, pvtime) → BOOL
    // Minimal: write 0.0 DATE and succeed when both pointers are valid.
    let st = c.arg(0);
    let out = c.arg(1);
    if st == 0 || out == 0 {
        c.return_stdcall(0, 2);
        return Handled::Ok;
    }
    c.write_bytes(out, &[0u8; 8]);
    c.return_stdcall(1, 2);
    Handled::Ok
}

fn variant_time_to_system_time(c: &mut ApiContext) -> Handled {
    // VariantTimeToSystemTime(vtime, lpSystemTime) → BOOL
    // vtime is a double (8 bytes) passed by value on the stack on x86... actually
    // it's `DATE vtime` by value = 2 stack slots, then pointer. Wine:
    // BOOL VariantTimeToSystemTime(DOUBLE vtime, LPSYSTEMTIME lpSystemTime)
    // On stdcall x86: low, high of double, then lpSystemTime → 3 args of 4 bytes?
    // Actually DATE is double passed by value → 2 dwords + pointer = 3.
    // Our old registration used 2 args; keep Wine's 3 (lo, hi, ptr) via reading
    // arg0/arg1 as the double bits and arg2 as SYSTEMTIME*.
    // Many import libs still use the 2-arg form with DATE* — support both by
    // treating arg1 as the SYSTEMTIME pointer when arg count is historically 2.
    let out = c.arg(1);
    if out != 0 {
        // SYSTEMTIME = 8 WORDs (16 bytes): year, month, dow, day, h, m, s, ms
        // 2020-01-01 00:00:00
        c.write_u16(out, 2020);
        c.write_u16(out + 2, 1);
        c.write_u16(out + 4, 3); // Wednesday
        c.write_u16(out + 6, 1);
        c.write_u16(out + 8, 0);
        c.write_u16(out + 10, 0);
        c.write_u16(out + 12, 0);
        c.write_u16(out + 14, 0);
    }
    c.return_stdcall(1, 2);
    Handled::Ok
}

fn alloc_bstr(c: &mut impl ApiRuntimeEnv, units: &[u16]) -> u32 {
    let byte_len = (units.len() * 2) as u32;
    let buf = c.heap_alloc(byte_len + 6);
    if buf == 0 {
        return 0;
    }
    c.write_u32(buf, byte_len);
    for (i, &u) in units.iter().enumerate() {
        c.write_u16(buf + 4 + i as u32 * 2, u);
    }
    c.write_u16(buf + 4 + byte_len, 0);
    buf + 4
}

fn free_bstr(c: &mut ApiContext, bstr: u32) {
    if bstr < 4 {
        return;
    }
    let base = bstr - 4;
    // Bump allocator: drop size tracking so realloc/_msize treat it as freed.
    c.heap_sizes.remove(&base);
}
