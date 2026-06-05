//! Stubs for common app DLLs we don't fully implement: version, gdiplus,
//! wininet, comdlg32, oleaut32, dbghelp, and a few shell32/advapi helpers.
//!
//! The point is correct **stdcall arg counts**: an imported-but-unimplemented
//! function falls back to a guessed arg count (often wrong), which leaves the
//! guest stack unbalanced and corrupts a later `ret`. Registering each function
//! with its real arity — even as a no-op returning a sensible value — keeps the
//! stack balanced so apps degrade gracefully instead of crashing.

use super::{ApiContext, Handled, WinApiRegistry};

// Handlers are plain `fn`s (the registry wants `fn` pointers, not closures), so
// we declare one per (return value, arg count) shape we need and reuse them.
macro_rules! retn {
    ($name:ident, $ret:expr, $argc:expr) => {
        fn $name(c: &mut ApiContext) -> Handled { c.ret_stdcall($ret, $argc); Handled::Ok }
    };
}
retn!(r0_0, 0, 0);
retn!(r0_1, 0, 1);
retn!(r0_2, 0, 2);
retn!(r0_3, 0, 3);
retn!(r0_4, 0, 4);
retn!(r0_5, 0, 5);
retn!(r0_6, 0, 6);
retn!(r0_7, 0, 7);
retn!(r0_9, 0, 9);
retn!(r1_1, 1, 1);
retn!(r1_3, 1, 3);

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        // ---- version.dll: report "no version info" (apps fall back to defaults).
        ("version.dll", "GetFileVersionInfoSizeA", r0_2),       // (name, &handle)
        ("version.dll", "GetFileVersionInfoSizeW", r0_2),
        ("version.dll", "GetFileVersionInfoSizeExW", r0_3),     // (flags, name, &handle)
        ("version.dll", "GetFileVersionInfoSizeExA", r0_3),
        ("version.dll", "GetFileVersionInfoA", r0_4),           // (name, handle, len, data)
        ("version.dll", "GetFileVersionInfoW", r0_4),
        ("version.dll", "GetFileVersionInfoExW", r0_5),
        ("version.dll", "GetFileVersionInfoExA", r0_5),
        ("version.dll", "VerQueryValueA", r0_4),                // -> 0 (not found)
        ("version.dll", "VerQueryValueW", r0_4),
        ("version.dll", "VerLanguageNameA", r0_3),
        ("version.dll", "VerLanguageNameW", r0_3),

        // ---- gdiplus.dll: accept init, no-op the rest (Ok = 0 = Gdiplus::Ok).
        ("gdiplus.dll", "GdiplusStartup", gdiplus_startup),     // (&token, &input, &output)
        ("gdiplus.dll", "GdiplusShutdown", r0_1),
        ("gdiplus.dll", "GdipAlloc", gdip_alloc),               // (size) -> ptr
        ("gdiplus.dll", "GdipFree", r0_1),
        ("gdiplus.dll", "GdipCreateFromHDC", r0_2),
        ("gdiplus.dll", "GdipDeleteGraphics", r0_1),
        ("gdiplus.dll", "GdipCreateBitmapFromScan0", r0_6),
        ("gdiplus.dll", "GdipCreateBitmapFromHBITMAP", r0_3),
        ("gdiplus.dll", "GdipDisposeImage", r0_1),
        ("gdiplus.dll", "GdipGetImageWidth", r0_2),
        ("gdiplus.dll", "GdipGetImageHeight", r0_2),
        ("gdiplus.dll", "GdipDrawImageRectI", r0_6),
        ("gdiplus.dll", "GdipDrawImageI", r0_4),
        ("gdiplus.dll", "GdipSetSmoothingMode", r0_2),
        ("gdiplus.dll", "GdipSetInterpolationMode", r0_2),

        // ---- wininet.dll: no network — report failure cleanly.
        ("wininet.dll", "InternetOpenA", r0_5),                 // -> NULL handle
        ("wininet.dll", "InternetOpenW", r0_5),
        ("wininet.dll", "InternetCloseHandle", r1_1),
        ("wininet.dll", "InternetOpenUrlA", r0_6),
        ("wininet.dll", "InternetOpenUrlW", r0_6),
        ("wininet.dll", "InternetConnectA", r0_8),
        ("wininet.dll", "InternetConnectW", r0_8),
        ("wininet.dll", "InternetReadFile", r0_4),
        ("wininet.dll", "InternetSetOptionA", r1_4),
        ("wininet.dll", "HttpOpenRequestA", r0_8),
        ("wininet.dll", "HttpSendRequestA", r0_5),
        ("wininet.dll", "HttpQueryInfoA", r0_5),
        ("wininet.dll", "InternetGetConnectedState", r0_2),     // -> 0 (offline)

        // ---- comdlg32.dll: dialogs report "cancelled" (0).
        ("comdlg32.dll", "GetOpenFileNameA", r0_1),
        ("comdlg32.dll", "GetOpenFileNameW", r0_1),
        ("comdlg32.dll", "GetSaveFileNameA", r0_1),
        ("comdlg32.dll", "GetSaveFileNameW", r0_1),
        ("comdlg32.dll", "ChooseColorA", r0_1),
        ("comdlg32.dll", "ChooseColorW", r0_1),
        ("comdlg32.dll", "ChooseFontA", r0_1),
        ("comdlg32.dll", "ChooseFontW", r0_1),
        ("comdlg32.dll", "PrintDlgA", r0_1),
        ("comdlg32.dll", "PrintDlgW", r0_1),
        ("comdlg32.dll", "CommDlgExtendedError", r0_0),

        // ---- oleaut32.dll: BSTR/VARIANT helpers (real where cheap).
        ("oleaut32.dll", "SysAllocString", sys_alloc_string),   // (wsz) -> BSTR
        ("oleaut32.dll", "SysAllocStringLen", sys_alloc_string_len),
        ("oleaut32.dll", "SysFreeString", r0_1),
        ("oleaut32.dll", "SysStringLen", sys_string_len),
        ("oleaut32.dll", "SysStringByteLen", sys_string_byte_len),
        ("oleaut32.dll", "VariantInit", variant_init),          // zero 16 bytes
        ("oleaut32.dll", "VariantClear", variant_init),
        ("oleaut32.dll", "VariantCopy", r0_2),
        ("oleaut32.dll", "OleLoadPicture", r1_5),
        ("oleaut32.dll", "#2", sys_alloc_string),               // SysAllocString ordinal
        ("oleaut32.dll", "#6", r0_1),                           // SysFreeString ordinal
        ("oleaut32.dll", "#7", sys_string_len),                 // SysStringLen ordinal

        // ---- dbghelp.dll: crash/symbol tooling — report "unavailable".
        ("dbghelp.dll", "SymInitialize", r1_3),
        ("dbghelp.dll", "SymCleanup", r1_1),
        ("dbghelp.dll", "SymGetOptions", r0_0),
        ("dbghelp.dll", "SymSetOptions", r0_1),
        ("dbghelp.dll", "SymGetSymFromAddr", r0_4),
        ("dbghelp.dll", "SymFromAddr", r0_4),
        ("dbghelp.dll", "SymGetLineFromAddr64", r0_4),
        ("dbghelp.dll", "StackWalk64", r0_9),
        ("dbghelp.dll", "SymFunctionTableAccess64", r0_2),
        ("dbghelp.dll", "SymGetModuleBase64", r0_2),
        ("dbghelp.dll", "MiniDumpWriteDump", r0_7),
        ("dbghelp.dll", "ImageNtHeader", r0_1),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

// 8/4-arg shapes used above but not in the small set.
retn!(r0_8, 0, 8);
retn!(r1_4, 1, 4);
retn!(r1_5, 1, 5);

// GdiplusStartup(&token, &input, &output): write a non-zero token, return Ok(0).
fn gdiplus_startup(c: &mut ApiContext) -> Handled {
    let token_ptr = c.arg(0);
    if token_ptr != 0 {
        let _ = c.memory.write_u32(token_ptr, 1);
    }
    c.ret_stdcall(0, 3);
    Handled::Ok
}

// GdipAlloc(size) -> heap pointer (gdiplus' internal allocator).
fn gdip_alloc(c: &mut ApiContext) -> Handled {
    let size = c.arg(0).max(1);
    let p = c.heap_alloc(size);
    c.ret_stdcall(p, 1);
    Handled::Ok
}

// SysAllocString(OLECHAR* psz): allocate a BSTR (4-byte byte-length prefix +
// wide string + null). Returns a pointer to the string (after the prefix).
fn sys_alloc_string(c: &mut ApiContext) -> Handled {
    let psz = c.arg(0);
    let s = if psz != 0 { c.wstr(psz) } else { String::new() };
    let units: Vec<u16> = s.encode_utf16().collect();
    let byte_len = (units.len() * 2) as u32;
    let buf = c.heap_alloc(byte_len + 6);
    let _ = c.memory.write_u32(buf, byte_len);
    for (i, &u) in units.iter().enumerate() {
        let _ = c.memory.write_u16(buf + 4 + i as u32 * 2, u);
    }
    let _ = c.memory.write_u16(buf + 4 + byte_len, 0);
    c.ret_stdcall(buf + 4, 1);
    Handled::Ok
}

// SysAllocStringLen(pch, len): BSTR of `len` wide chars from pch (or zeroed).
fn sys_alloc_string_len(c: &mut ApiContext) -> Handled {
    let pch = c.arg(0);
    let len = c.arg(1);
    let byte_len = len * 2;
    let buf = c.heap_alloc(byte_len + 6);
    let _ = c.memory.write_u32(buf, byte_len);
    for i in 0..len {
        let v = if pch != 0 { c.memory.read_u16(pch + i * 2).unwrap_or(0) } else { 0 };
        let _ = c.memory.write_u16(buf + 4 + i * 2, v);
    }
    let _ = c.memory.write_u16(buf + 4 + byte_len, 0);
    c.ret_stdcall(buf + 4, 2);
    Handled::Ok
}

// SysStringLen(bstr): length in wide chars, read from the 4-byte prefix.
fn sys_string_len(c: &mut ApiContext) -> Handled {
    let bstr = c.arg(0);
    let n = if bstr >= 4 { c.memory.read_u32(bstr - 4).unwrap_or(0) / 2 } else { 0 };
    c.ret_stdcall(n, 1);
    Handled::Ok
}

fn sys_string_byte_len(c: &mut ApiContext) -> Handled {
    let bstr = c.arg(0);
    let n = if bstr >= 4 { c.memory.read_u32(bstr - 4).unwrap_or(0) } else { 0 };
    c.ret_stdcall(n, 1);
    Handled::Ok
}

// VariantInit / VariantClear(pvarg): zero the 16-byte VARIANT.
fn variant_init(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p != 0 {
        let _ = c.memory.write_bytes(p, &[0u8; 16]);
    }
    c.ret_stdcall(0, 1);
    Handled::Ok
}
