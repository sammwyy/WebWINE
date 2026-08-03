//! version.dll — file version resource queries (Wine kernelbase/version.c semantics).
//!
//! When a PE on the VFS has an RT_VERSION resource we surface it; otherwise a
//! synthetic VS_VERSION_INFO block is generated so callers that only need
//! VerQueryValue("\\") / a non-zero size keep working.

use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::winapi::context::ApiRuntimeEnv;

const ERROR_INVALID_PARAMETER: u32 = 87;
const ERROR_BAD_PATHNAME: u32 = 161;
const ERROR_FILE_NOT_FOUND: u32 = 2;

/// Size of the synthetic VS_VERSION_INFO we emit (wLength field + payload).
const SYNTH_LENGTH: u32 = 92;
/// Buffer size reported by GetFileVersionInfoSize (2× resource + 4 "FE2X", Wine-style).
const SYNTH_BUF_SIZE: u32 = SYNTH_LENGTH * 2 + 4;

pub fn register(r: &mut WinApiRegistry) {
    r.add("version.dll", "GetFileVersionInfoSizeA", |c| get_file_version_info_size(c, false, false));
    r.add("version.dll", "GetFileVersionInfoSizeW", |c| get_file_version_info_size(c, true, false));
    r.add("version.dll", "GetFileVersionInfoSizeExA", |c| get_file_version_info_size(c, false, true));
    r.add("version.dll", "GetFileVersionInfoSizeExW", |c| get_file_version_info_size(c, true, true));
    r.add("version.dll", "GetFileVersionInfoA", |c| get_file_version_info(c, false, false));
    r.add("version.dll", "GetFileVersionInfoW", |c| get_file_version_info(c, true, false));
    r.add("version.dll", "GetFileVersionInfoExA", |c| get_file_version_info(c, false, true));
    r.add("version.dll", "GetFileVersionInfoExW", |c| get_file_version_info(c, true, true));
    r.add("version.dll", "VerQueryValueA", |c| ver_query_value(c, false));
    r.add("version.dll", "VerQueryValueW", |c| ver_query_value(c, true));
    r.add("version.dll", "VerLanguageNameA", |c| ver_language_name(c, false));
    r.add("version.dll", "VerLanguageNameW", |c| ver_language_name(c, true));
}

fn read_path(c: &ApiContext, ptr: u32, wide: bool) -> String {
    if ptr == 0 {
        String::new()
    } else if wide {
        c.wstr(ptr)
    } else {
        c.cstr(ptr)
    }
}

/// GetFileVersionInfoSize[Ex](A|W): size needed for GetFileVersionInfo buffer.
fn get_file_version_info_size(c: &mut ApiContext, wide: bool, ex: bool) -> Handled {
    let (flags_off, path_off, handle_off, nargs) = if ex {
        (0u32, 1u32, 2u32, 3u32)
    } else {
        (0u32, 0u32, 1u32, 2u32)
    };
    let _flags = if ex { c.arg(flags_off) } else { 0 };
    let path = read_path(c, c.arg(path_off), wide);
    let handle = c.arg(handle_off);
    if handle != 0 {
        c.write_u32(handle, 0);
    }

    if path.is_empty() {
        c.cpu.last_error = if path_off == 0 || c.arg(path_off) == 0 {
            ERROR_INVALID_PARAMETER
        } else {
            ERROR_BAD_PATHNAME
        };
        c.return_stdcall(0, nargs);
        return Handled::Ok;
    }

    let resolved = c.resolve_path(&path);
    // Prefer a real RT_VERSION blob from the PE when present on the VFS.
    if let Some(size) = pe_version_resource_size(c, &resolved) {
        c.cpu.last_error = 0;
        // Wine: (len * 2) + 4 for 32-bit resources.
        c.return_stdcall(size.saturating_mul(2).saturating_add(4), nargs);
        return Handled::Ok;
    }

    // File exists but no version resource (or not a PE): still offer a synthetic block.
    if c.fs.node_exists(&resolved) {
        c.cpu.last_error = 0;
        c.return_stdcall(SYNTH_BUF_SIZE, nargs);
        return Handled::Ok;
    }

    c.cpu.last_error = ERROR_FILE_NOT_FOUND;
    c.return_stdcall(0, nargs);
    Handled::Ok
}

/// GetFileVersionInfo[Ex](A|W): fill caller buffer with version info.
fn get_file_version_info(c: &mut ApiContext, wide: bool, ex: bool) -> Handled {
    let (path_off, size_off, data_off, nargs) = if ex {
        // flags, filename, handle, size, data
        (1u32, 3u32, 4u32, 5u32)
    } else {
        // filename, handle, size, data
        (0u32, 2u32, 3u32, 4u32)
    };
    let path = read_path(c, c.arg(path_off), wide);
    let size = c.arg(size_off);
    let data = c.arg(data_off);

    if data == 0 || size == 0 {
        c.cpu.last_error = ERROR_INVALID_PARAMETER;
        c.return_stdcall(0, nargs);
        return Handled::Ok;
    }

    let resolved = c.resolve_path(&path);
    if let Some(blob) = pe_version_resource_bytes(c, &resolved) {
        let n = (blob.len() as u32).min(size) as usize;
        c.write_bytes(data, &blob[..n]);
        // FE2X signature after resource (Wine/Windows layout for A conversions).
        if size >= n as u32 + 4 && n >= 2 {
            let wlen = u16::from_le_bytes([blob[0], blob[1]]) as usize;
            if size as usize >= wlen + 4 {
                c.write_bytes(data + wlen as u32, b"FE2X");
            }
        }
        c.cpu.last_error = 0;
        c.return_stdcall(1, nargs);
        return Handled::Ok;
    }

    if !c.fs.node_exists(&resolved) {
        c.cpu.last_error = ERROR_FILE_NOT_FOUND;
        c.return_stdcall(0, nargs);
        return Handled::Ok;
    }

    let block = build_synthetic_version_info();
    let n = (block.len() as u32).min(size) as usize;
    c.write_bytes(data, &block[..n]);
    if size >= SYNTH_LENGTH + 4 {
        c.write_bytes(data + SYNTH_LENGTH, b"FE2X");
    }
    c.cpu.last_error = 0;
    c.return_stdcall(1, nargs);
    Handled::Ok
}

/// VerQueryValue(A|W): resolve a sub-block path inside the version info blob.
fn ver_query_value(c: &mut ApiContext, wide: bool) -> Handled {
    let block = c.arg(0);
    let sub = c.arg(1);
    let out_buf = c.arg(2);
    let out_len = c.arg(3);

    if block == 0 {
        c.return_stdcall(0, 4);
        return Handled::Ok;
    }

    let path = if sub == 0 {
        String::new()
    } else if wide {
        c.wstr(sub)
    } else {
        c.cstr(sub)
    };

    // Root "\\" or empty → VS_FIXEDFILEINFO at the Value offset of the root node.
    let is_root = path.is_empty() || path == "\\" || path == "/";
    if is_root {
        // VS_VERSION_INFO_STRUCT32: wLength, wValueLength, wType, szKey(L"VS_VERSION_INFO\0")
        // Value starts DWORD-aligned after the key.
        let value_off = fixed_file_info_offset(c, block);
        if out_buf != 0 {
            c.write_u32(out_buf, block + value_off);
        }
        if out_len != 0 {
            c.write_u32(out_len, 52); // sizeof(VS_FIXEDFILEINFO)
        }
        c.return_stdcall(1, 4);
        return Handled::Ok;
    }

    // Walk children for StringFileInfo / VarFileInfo style paths — best-effort.
    if let Some((ptr, len)) = find_child_value(c, block, &path) {
        if out_buf != 0 {
            c.write_u32(out_buf, ptr);
        }
        if out_len != 0 {
            c.write_u32(out_len, len);
        }
        c.return_stdcall(1, 4);
        return Handled::Ok;
    }

    if out_len != 0 {
        c.write_u32(out_len, 0);
    }
    c.return_stdcall(0, 4);
    Handled::Ok
}

fn ver_language_name(c: &mut ApiContext, wide: bool) -> Handled {
    // VerLanguageName(wLang, szLang, cchLang) → chars written excluding NUL.
    let _lang = c.arg(0);
    let buf = c.arg(1);
    let cch = c.arg(2) as usize;
    let name = "Language Neutral";
    if buf != 0 && cch > 0 {
        if wide {
            let units: Vec<u16> = name.encode_utf16().take(cch.saturating_sub(1)).collect();
            for (i, u) in units.iter().enumerate() {
                c.write_u16(buf + i as u32 * 2, *u);
            }
            c.write_u16(buf + units.len() as u32 * 2, 0);
            c.return_stdcall(units.len() as u32, 3);
        } else {
            let n = name.len().min(cch.saturating_sub(1));
            c.write_bytes(buf, name.as_bytes()[..n].as_ref());
            c.write_bytes(buf + n as u32, &[0]);
            c.return_stdcall(n as u32, 3);
        }
    } else {
        c.return_stdcall(0, 3);
    }
    Handled::Ok
}

/// Build a minimal VS_VERSION_INFO with VS_FIXEDFILEINFO (Wine/Windows layout).
fn build_synthetic_version_info() -> Vec<u8> {
    // Key: "VS_VERSION_INFO" as UTF-16 + NUL, then pad to DWORD, then FIXEDFILEINFO.
    let key: Vec<u16> = "VS_VERSION_INFO"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // Header: wLength(2) wValueLength(2) wType(2) + key
    let header_key_len = 6 + key.len() * 2;
    let pad = (4 - (header_key_len % 4)) % 4;
    let value_off = header_key_len + pad;
    let total = value_off + 52; // + VS_FIXEDFILEINFO

    let mut buf = vec![0u8; total];
    buf[0..2].copy_from_slice(&(total as u16).to_le_bytes());
    buf[2..4].copy_from_slice(&52u16.to_le_bytes()); // wValueLength = sizeof FIXEDFILEINFO
    buf[4..6].copy_from_slice(&0u16.to_le_bytes()); // wType = binary
    for (i, u) in key.iter().enumerate() {
        let o = 6 + i * 2;
        buf[o..o + 2].copy_from_slice(&u.to_le_bytes());
    }

    // VS_FIXEDFILEINFO
    let ffi = value_off;
    write_u32(&mut buf, ffi, 0xFEEF_04BD); // dwSignature
    write_u32(&mut buf, ffi + 4, 0x0001_0000); // dwStrucVersion
    write_u32(&mut buf, ffi + 8, 0x0001_0000); // dwFileVersionMS 1.0
    write_u32(&mut buf, ffi + 12, 0x0000_0000); // dwFileVersionLS
    write_u32(&mut buf, ffi + 16, 0x0001_0000); // dwProductVersionMS
    write_u32(&mut buf, ffi + 20, 0x0000_0000); // dwProductVersionLS
    write_u32(&mut buf, ffi + 24, 0x0000_003F); // dwFileFlagsMask
    write_u32(&mut buf, ffi + 28, 0); // dwFileFlags
    write_u32(&mut buf, ffi + 32, 0x0000_0004); // VOS__WINDOWS32
    write_u32(&mut buf, ffi + 36, 0x0000_0001); // VFT_APP
    write_u32(&mut buf, ffi + 40, 0); // dwFileSubtype
    write_u32(&mut buf, ffi + 44, 0); // dwFileDateMS
    write_u32(&mut buf, ffi + 48, 0); // dwFileDateLS
    buf
}

fn write_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

fn fixed_file_info_offset(c: &ApiContext, block: u32) -> u32 {
    // Prefer walking the real structure; fall back to synthetic layout offset.
    let w_value_len = c.read_u16(block + 2) as u32;
    let w_type = c.read_u16(block + 4);
    // Skip key (UTF-16, null-terminated) starting at block+6.
    let mut p = block + 6;
    for _ in 0..64 {
        if c.read_u16(p) == 0 {
            p += 2;
            break;
        }
        p += 2;
    }
    // DWORD align.
    p = (p + 3) & !3;
    if w_value_len >= 52 && w_type == 0 {
        return p - block;
    }
    // Synthetic layout constant.
    40
}

fn find_child_value(c: &ApiContext, block: u32, path: &str) -> Option<(u32, u32)> {
    // Minimal walker: only handles a single child key match at the root level
    // (e.g. "\\StringFileInfo" is not fully walked; StringFileInfo needs nested
    // children). Good enough for apps that only touch FIXEDFILEINFO.
    let _ = (c, block, path);
    None
}

/// Scan a PE image on the VFS for the first RT_VERSION (type 16) resource size.
fn pe_version_resource_size(c: &ApiContext, path: &str) -> Option<u32> {
    pe_version_resource_bytes(c, path).map(|b| b.len() as u32)
}

fn pe_version_resource_bytes(c: &ApiContext, path: &str) -> Option<Vec<u8>> {
    let data = c.fs.read_file(path).ok()?;
    extract_version_resource(&data)
}

/// Locate RT_VERSION resource data in a PE32 image.
fn extract_version_resource(pe: &[u8]) -> Option<Vec<u8>> {
    if pe.len() < 0x40 || &pe[0..2] != b"MZ" {
        return None;
    }
    let e_lfanew = u32::from_le_bytes(pe[0x3C..0x40].try_into().ok()?) as usize;
    if pe.len() < e_lfanew + 24 || &pe[e_lfanew..e_lfanew + 4] != b"PE\0\0" {
        return None;
    }
    let coff = e_lfanew + 4;
    let num_sections = u16::from_le_bytes(pe[coff + 2..coff + 4].try_into().ok()?) as usize;
    let opt_size = u16::from_le_bytes(pe[coff + 16..coff + 18].try_into().ok()?) as usize;
    let opt = coff + 20;
    if pe.len() < opt + 2 {
        return None;
    }
    let magic = u16::from_le_bytes(pe[opt..opt + 2].try_into().ok()?);
    // DataDirectory[2] = resource table. PE32: optional header + 96 + 2*8.
    let dd_off = if magic == 0x10B {
        opt + 96
    } else if magic == 0x20B {
        opt + 112
    } else {
        return None;
    };
    let res_dd = dd_off + 2 * 8; // entry index 2
    if pe.len() < res_dd + 8 {
        return None;
    }
    let res_rva = u32::from_le_bytes(pe[res_dd..res_dd + 4].try_into().ok()?);
    let res_size = u32::from_le_bytes(pe[res_dd + 4..res_dd + 8].try_into().ok()?);
    if res_rva == 0 || res_size == 0 {
        return None;
    }

    let sections_off = opt + opt_size;
    let file_off = rva_to_offset(pe, sections_off, num_sections, res_rva)?;
    let res_end = file_off.saturating_add(res_size as usize).min(pe.len());
    if file_off >= pe.len() {
        return None;
    }
    let res = &pe[file_off..res_end];

    // Resource directory: type → name → language → data entry.
    // Type ID 16 = RT_VERSION.
    let type_entry = find_id_entry(res, 0, 16)?;
    let name_dir = (type_entry & 0x7FFF_FFFF) as usize;
    // First name/id entry under RT_VERSION.
    let name_entry = first_entry(res, name_dir)?;
    let lang_dir = (name_entry & 0x7FFF_FFFF) as usize;
    let lang_entry = first_entry(res, lang_dir)?;
    let data_entry_off = if lang_entry & 0x8000_0000 != 0 {
        return None; // unexpected another directory
    } else {
        lang_entry as usize
    };
    if res.len() < data_entry_off + 16 {
        return None;
    }
    let data_rva = u32::from_le_bytes(res[data_entry_off..data_entry_off + 4].try_into().ok()?);
    let data_size =
        u32::from_le_bytes(res[data_entry_off + 4..data_entry_off + 8].try_into().ok()?) as usize;
    let data_file = rva_to_offset(pe, sections_off, num_sections, data_rva)?;
    if data_file + data_size > pe.len() {
        return None;
    }
    Some(pe[data_file..data_file + data_size].to_vec())
}

fn rva_to_offset(pe: &[u8], sections_off: usize, num_sections: usize, rva: u32) -> Option<usize> {
    for i in 0..num_sections {
        let s = sections_off + i * 40;
        if pe.len() < s + 40 {
            return None;
        }
        let virt_size = u32::from_le_bytes(pe[s + 8..s + 12].try_into().ok()?);
        let virt_addr = u32::from_le_bytes(pe[s + 12..s + 16].try_into().ok()?);
        let raw_size = u32::from_le_bytes(pe[s + 16..s + 20].try_into().ok()?);
        let raw_ptr = u32::from_le_bytes(pe[s + 20..s + 24].try_into().ok()?);
        let size = virt_size.max(raw_size);
        if rva >= virt_addr && rva < virt_addr.saturating_add(size) {
            return Some((raw_ptr + (rva - virt_addr)) as usize);
        }
    }
    None
}

fn find_id_entry(res: &[u8], dir_off: usize, id: u32) -> Option<u32> {
    if res.len() < dir_off + 16 {
        return None;
    }
    let num_named = u16::from_le_bytes(res[dir_off + 12..dir_off + 14].try_into().ok()?) as usize;
    let num_id = u16::from_le_bytes(res[dir_off + 14..dir_off + 16].try_into().ok()?) as usize;
    let entries = dir_off + 16 + num_named * 8; // skip named
    for i in 0..num_id {
        let e = entries + i * 8;
        if res.len() < e + 8 {
            return None;
        }
        let entry_id = u32::from_le_bytes(res[e..e + 4].try_into().ok()?);
        let offset = u32::from_le_bytes(res[e + 4..e + 8].try_into().ok()?);
        if entry_id == id {
            return Some(offset);
        }
    }
    None
}

fn first_entry(res: &[u8], dir_off: usize) -> Option<u32> {
    if res.len() < dir_off + 16 {
        return None;
    }
    let num_named = u16::from_le_bytes(res[dir_off + 12..dir_off + 14].try_into().ok()?) as usize;
    let num_id = u16::from_le_bytes(res[dir_off + 14..dir_off + 16].try_into().ok()?) as usize;
    if num_named + num_id == 0 {
        return None;
    }
    let e = dir_off + 16;
    if res.len() < e + 8 {
        return None;
    }
    Some(u32::from_le_bytes(res[e + 4..e + 8].try_into().ok()?))
}
