//! dbghelp.dll — symbol/debug helpers (Wine-aligned success/failure paths).
//!
//! WebWINE has no PDB/symbol server; most lookups fail cleanly. ImageNtHeader
//! walks a real PE image in guest memory so crash-dump style callers work.

use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::winapi::context::ApiRuntimeEnv;

pub fn register(r: &mut WinApiRegistry) {
    r.add("dbghelp.dll", "SymInitialize", sym_initialize);
    r.add("dbghelp.dll", "SymInitializeW", sym_initialize);
    r.add("dbghelp.dll", "SymCleanup", sym_cleanup);
    r.add("dbghelp.dll", "SymGetOptions", sym_get_options);
    r.add("dbghelp.dll", "SymSetOptions", sym_set_options);
    r.add("dbghelp.dll", "SymGetSymFromAddr", sym_get_sym_from_addr);
    r.add("dbghelp.dll", "SymGetSymFromAddr64", sym_get_sym_from_addr);
    r.add("dbghelp.dll", "SymFromAddr", sym_from_addr);
    r.add("dbghelp.dll", "SymFromAddrW", sym_from_addr);
    r.add("dbghelp.dll", "SymGetLineFromAddr64", sym_get_line_from_addr64);
    r.add("dbghelp.dll", "SymGetLineFromAddr", sym_get_line_from_addr64);
    r.add("dbghelp.dll", "StackWalk64", stack_walk64);
    r.add("dbghelp.dll", "StackWalk", stack_walk64);
    r.add(
        "dbghelp.dll",
        "SymFunctionTableAccess64",
        sym_function_table_access64,
    );
    r.add("dbghelp.dll", "SymGetModuleBase64", sym_get_module_base64);
    r.add("dbghelp.dll", "SymGetModuleBase", sym_get_module_base64);
    r.add("dbghelp.dll", "MiniDumpWriteDump", mini_dump_write_dump);
    r.add("dbghelp.dll", "ImageNtHeader", image_nt_header);
    r.add("dbghelp.dll", "ImageDirectoryEntryToData", image_directory_entry_to_data);
    r.add("dbghelp.dll", "SymLoadModule64", sym_load_module64);
    r.add("dbghelp.dll", "SymLoadModuleEx", sym_load_module_ex);
    r.add("dbghelp.dll", "SymUnloadModule64", sym_unload_module64);
    r.add("dbghelp.dll", "SymRefreshModuleList", sym_refresh_module_list);
    r.add("dbghelp.dll", "SymSetSearchPath", sym_set_search_path);
    r.add("dbghelp.dll", "SymSetSearchPathW", sym_set_search_path);
    r.add("dbghelp.dll", "SymGetSearchPath", sym_get_search_path);
}

fn sym_initialize(c: &mut ApiContext) -> Handled {
    // BOOL SymInitialize(hProcess, UserSearchPath, fInvadeProcess)
    let hprocess = c.arg(0);
    c.dll_state.insert(format!("dbghelp.init.{hprocess}"), 1);
    c.dll_state
        .entry("dbghelp.options".into())
        .or_insert(0x0000_0004); // SYMOPT_DEFERRED_LOADS
    c.return_stdcall(1, 3);
    Handled::Ok
}

fn sym_cleanup(c: &mut ApiContext) -> Handled {
    let hprocess = c.arg(0);
    c.dll_state.remove(&format!("dbghelp.init.{hprocess}"));
    c.return_stdcall(1, 1);
    Handled::Ok
}

fn sym_get_options(c: &mut ApiContext) -> Handled {
    let opts = c
        .dll_state
        .get("dbghelp.options")
        .copied()
        .unwrap_or(0x0000_0004);
    c.return_stdcall(opts, 0);
    Handled::Ok
}

fn sym_set_options(c: &mut ApiContext) -> Handled {
    let opts = c.arg(0);
    let prev = c
        .dll_state
        .get("dbghelp.options")
        .copied()
        .unwrap_or(0x0000_0004);
    c.dll_state.insert("dbghelp.options".into(), opts);
    c.return_stdcall(prev, 1);
    Handled::Ok
}

fn sym_get_sym_from_addr(c: &mut ApiContext) -> Handled {
    // BOOL SymGetSymFromAddr(hProcess, dwAddr, pdwDisplacement, pSymbol)
    // No symbols — fail with zeroed displacement.
    let disp = c.arg(2);
    if disp != 0 {
        c.write_u32(disp, 0);
    }
    c.cpu.last_error = 487; // ERROR_INVALID_ADDRESS
    c.return_stdcall(0, 4);
    Handled::Ok
}

fn sym_from_addr(c: &mut ApiContext) -> Handled {
    // BOOL SymFromAddr(hProcess, Address, Displacement, Symbol)
    let disp = c.arg(2);
    if disp != 0 {
        // Displacement is ULONG64* on 64-bit APIs; we write 8 zero bytes when possible.
        c.write_u32(disp, 0);
        c.write_u32(disp + 4, 0);
    }
    c.cpu.last_error = 487;
    c.return_stdcall(0, 4);
    Handled::Ok
}

fn sym_get_line_from_addr64(c: &mut ApiContext) -> Handled {
    let disp = c.arg(2);
    if disp != 0 {
        c.write_u32(disp, 0);
    }
    c.cpu.last_error = 487;
    c.return_stdcall(0, 4);
    Handled::Ok
}

fn stack_walk64(c: &mut ApiContext) -> Handled {
    // BOOL StackWalk64(... 9 args). Without a real stack walker, stop immediately.
    c.return_stdcall(0, 9);
    Handled::Ok
}

fn sym_function_table_access64(c: &mut ApiContext) -> Handled {
    // PVOID SymFunctionTableAccess64(hProcess, AddrBase) → NULL
    c.return_stdcall(0, 2);
    Handled::Ok
}

fn sym_get_module_base64(c: &mut ApiContext) -> Handled {
    // DWORD64 SymGetModuleBase64(hProcess, dwAddr) — return image base if addr
    // looks like it's inside the loaded PE, else 0.
    let addr = c.arg(1);
    // Guest images typically load at 0x0040_0000.
    let base = if addr >= 0x0040_0000 && addr < 0x8000_0000 {
        0x0040_0000
    } else {
        0
    };
    c.return_stdcall(base, 2);
    Handled::Ok
}

fn mini_dump_write_dump(c: &mut ApiContext) -> Handled {
    // BOOL MiniDumpWriteDump(hProcess, ProcessId, hFile, DumpType, ExceptionParam,
    //                        UserStreamParam, CallbackParam) — 7 args.
    // No dump support.
    c.cpu.last_error = 50; // ERROR_NOT_SUPPORTED
    c.return_stdcall(0, 7);
    Handled::Ok
}

/// ImageNtHeader(Base) → PIMAGE_NT_HEADERS
fn image_nt_header(c: &mut ApiContext) -> Handled {
    let base = c.arg(0);
    if base == 0 {
        c.return_stdcall(0, 1);
        return Handled::Ok;
    }
    // IMAGE_DOS_HEADER.e_magic
    let mz = c.read_u16(base);
    if mz != 0x5A4D {
        c.return_stdcall(0, 1);
        return Handled::Ok;
    }
    let e_lfanew = c.read_u32(base + 0x3C);
    let nt = base.wrapping_add(e_lfanew);
    let sig = c.read_u32(nt);
    if sig != 0x0000_4550 {
        // "PE\0\0"
        c.return_stdcall(0, 1);
        return Handled::Ok;
    }
    c.return_stdcall(nt, 1);
    Handled::Ok
}

fn image_directory_entry_to_data(c: &mut ApiContext) -> Handled {
    // PVOID ImageDirectoryEntryToData(Base, MappedAsImage, DirectoryEntry, Size)
    let size_ptr = c.arg(3);
    if size_ptr != 0 {
        c.write_u32(size_ptr, 0);
    }
    c.return_stdcall(0, 4);
    Handled::Ok
}

fn sym_load_module64(c: &mut ApiContext) -> Handled {
    // DWORD64 SymLoadModule64(...) — 6 args; return BaseOfDll arg or 0.
    let base = c.arg(2);
    c.return_stdcall(base, 6);
    Handled::Ok
}

fn sym_load_module_ex(c: &mut ApiContext) -> Handled {
    // DWORD64 SymLoadModuleEx(hProcess, hFile, ImageName, ModuleName, BaseOfDll,
    //                         DllSize, Data, Flags) — 8 args
    let base = c.arg(4);
    c.return_stdcall(base, 8);
    Handled::Ok
}

fn sym_unload_module64(c: &mut ApiContext) -> Handled {
    c.return_stdcall(1, 2);
    Handled::Ok
}

fn sym_refresh_module_list(c: &mut ApiContext) -> Handled {
    c.return_stdcall(1, 1);
    Handled::Ok
}

fn sym_set_search_path(c: &mut ApiContext) -> Handled {
    c.return_stdcall(1, 2);
    Handled::Ok
}

fn sym_get_search_path(c: &mut ApiContext) -> Handled {
    // BOOL SymGetSearchPath(hProcess, SearchPath, SearchPathLength)
    let buf = c.arg(1);
    let len = c.arg(2) as usize;
    if buf != 0 && len > 0 {
        let path = b".\0";
        let n = path.len().min(len);
        c.write_bytes(buf, &path[..n]);
    }
    c.return_stdcall(1, 3);
    Handled::Ok
}
