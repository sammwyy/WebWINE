use super::{ApiContext, Handled, WinApiRegistry};
use crate::vm::handles::{
    KernelObject, CURRENT_PROCESS, CURRENT_THREAD, INVALID_HANDLE, STD_ERROR_HANDLE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};

// Default working directory for relative guest paths.
const CWD: &str = "C:\\Users\\guest\\Desktop";

// Win32 error codes used by the file APIs.
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_FILE_EXISTS: u32 = 80;

const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const INVALID_FILE_ATTRIBUTES: u32 = 0xFFFF_FFFF;

/// Resolve a guest path to an absolute `X:\...` form, applying the default
/// working directory for relative paths and stripping the `\\?\` verbatim prefix.
fn resolve_path(raw: &str) -> String {
    let r = raw.replace('/', "\\");
    let r = r.strip_prefix("\\\\?\\").unwrap_or(&r);
    if r.len() >= 2 && r.as_bytes()[1] == b':' {
        r.to_string()
    } else {
        format!("{CWD}\\{}", r.trim_start_matches('\\'))
    }
}

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("kernel32.dll", "ExitProcess", exit_process),
        ("kernel32.dll", "GetStdHandle", get_std_handle),
        ("kernel32.dll", "WriteFile", write_file),
        ("kernel32.dll", "WriteConsoleA", write_console_a),
        ("kernel32.dll", "WriteConsoleW", write_console_w),
        ("kernel32.dll", "ReadFile", read_file),
        ("kernel32.dll", "CloseHandle", close_handle),
        ("kernel32.dll", "GetLastError", get_last_error),
        ("kernel32.dll", "SetLastError", set_last_error),
        ("kernel32.dll", "GetProcessHeap", get_process_heap),
        ("kernel32.dll", "HeapAlloc", heap_alloc),
        ("kernel32.dll", "HeapFree", heap_free),
        ("kernel32.dll", "HeapReAlloc", heap_realloc),
        ("kernel32.dll", "HeapSize", heap_size),
        ("kernel32.dll", "HeapCreate", heap_create),
        ("kernel32.dll", "HeapDestroy", r1_1),
        ("kernel32.dll", "VirtualAlloc", virtual_alloc),
        ("kernel32.dll", "VirtualFree", r1_3),
        ("kernel32.dll", "VirtualProtect", virtual_protect),
        ("kernel32.dll", "VirtualQuery", r0_3),
        ("kernel32.dll", "GetModuleHandleA", get_module_handle_a),
        ("kernel32.dll", "GetModuleHandleW", get_module_handle_w),
        ("kernel32.dll", "GetModuleFileNameA", get_module_filename_a),
        ("kernel32.dll", "GetModuleFileNameW", r0_3),
        ("kernel32.dll", "GetProcAddress", r0_2),
        ("kernel32.dll", "LoadLibraryA", r0_1),
        ("kernel32.dll", "LoadLibraryW", r0_1),
        ("kernel32.dll", "LoadLibraryExW", r0_3),
        ("kernel32.dll", "FreeLibrary", r1_1),
        ("kernel32.dll", "IsDebuggerPresent", r0_0),
        ("kernel32.dll", "IsProcessorFeaturePresent", r1_1),
        ("kernel32.dll", "InitializeSListHead", r0_1),
        ("kernel32.dll", "QueryDepthSList", r0_1),
        ("kernel32.dll", "InterlockedPushEntrySList", r0_2),
        ("kernel32.dll", "InterlockedFlushSList", r0_1),
        ("kernel32.dll", "GetProcessAffinityMask", r1_3),
        ("kernel32.dll", "GetNativeSystemInfo", r0_1),
        ("kernel32.dll", "GetCurrentDirectoryW", r0_2),
        ("kernel32.dll", "SetThreadErrorMode", r1_2),
        ("kernel32.dll", "GetThreadPriority", r0_1),
        ("kernel32.dll", "AddVectoredExceptionHandler", r1_2),
        ("kernel32.dll", "RemoveVectoredExceptionHandler", r1_1),
        ("kernel32.dll", "SetThreadStackGuarantee", r1_1),
        ("kernel32.dll", "GetModuleHandleExW", get_module_handle_ex),
        ("kernel32.dll", "GetModuleHandleExA", get_module_handle_ex),
        ("kernel32.dll", "GetSystemTimePreciseAsFileTime", get_system_time),
        ("kernel32.dll", "InitOnceExecuteOnce", r1_4),
        ("kernel32.dll", "AcquireSRWLockExclusive", r0_1),
        ("kernel32.dll", "ReleaseSRWLockExclusive", r0_1),
        ("kernel32.dll", "AcquireSRWLockShared", r0_1),
        ("kernel32.dll", "ReleaseSRWLockShared", r0_1),
        ("kernel32.dll", "TryAcquireSRWLockExclusive", r1_1),
        ("kernel32.dll", "InitializeSRWLock", r0_1),
        ("kernel32.dll", "FlsAlloc", r0_1),
        ("kernel32.dll", "FlsSetValue", tls_set),
        ("kernel32.dll", "FlsGetValue", tls_get),
        ("kernel32.dll", "FlsFree", r1_1),
        ("kernel32.dll", "GetProcessHeaps", r0_2),
        ("kernel32.dll", "GetCommandLineA", get_command_line_a),
        ("kernel32.dll", "GetCommandLineW", get_command_line_w),
        ("kernel32.dll", "GetStartupInfoA", get_startup_info_a),
        ("kernel32.dll", "GetStartupInfoW", r0_1),
        (
            "kernel32.dll",
            "GetCurrentProcessId",
            get_current_process_id,
        ),
        ("kernel32.dll", "GetCurrentThreadId", get_current_thread_id),
        ("kernel32.dll", "GetCurrentProcess", get_current_process),
        ("kernel32.dll", "GetCurrentThread", get_current_thread),
        ("kernel32.dll", "GetSystemInfo", r0_1),
        ("kernel32.dll", "GetSystemTimeAsFileTime", get_system_time),
        (
            "kernel32.dll",
            "QueryPerformanceCounter",
            query_perf_counter,
        ),
        ("kernel32.dll", "QueryPerformanceFrequency", query_perf_freq),
        ("kernel32.dll", "GetTickCount", r0_0),
        ("kernel32.dll", "GetTickCount64", r0_0),
        ("kernel32.dll", "FlushFileBuffers", r1_1),
        ("kernel32.dll", "SetFilePointer", r0_4),
        ("kernel32.dll", "SetUnhandledExceptionFilter", r0_1),
        ("kernel32.dll", "UnhandledExceptionFilter", r0_1),
        ("kernel32.dll", "GetEnvironmentVariableW", r0_3),
        ("kernel32.dll", "GetEnvironmentVariableA", r0_3),
        ("kernel32.dll", "SetEnvironmentVariableW", r1_2),
        ("kernel32.dll", "SetEnvironmentVariableA", r1_2),
        ("kernel32.dll", "GetCurrentDirectoryA", r0_2),
        ("kernel32.dll", "GetFullPathNameW", r0_4),
        ("kernel32.dll", "GetFileAttributesW", get_file_attributes_w),
        ("kernel32.dll", "GetStdHandle", get_std_handle),
        ("kernel32.dll", "WriteConsoleW", write_console_w),
        ("kernel32.dll", "GetEnvironmentStringsW", r0_0),
        ("kernel32.dll", "FreeEnvironmentStringsW", r1_1),
        ("kernel32.dll", "GetEnvironmentStringsA", r0_0),
        ("kernel32.dll", "FreeEnvironmentStringsA", r1_1),
        ("kernel32.dll", "InitializeCriticalSection", r0_1),
        ("kernel32.dll", "InitializeCriticalSectionAndSpinCount", r1_2),
        ("kernel32.dll", "InitializeCriticalSectionEx", r1_3),
        ("kernel32.dll", "DeleteCriticalSection", r0_1),
        ("kernel32.dll", "EnterCriticalSection", r0_1),
        ("kernel32.dll", "LeaveCriticalSection", r0_1),
        ("kernel32.dll", "TryEnterCriticalSection", r1_1),
        ("kernel32.dll", "TlsAlloc", tls_alloc),
        ("kernel32.dll", "TlsSetValue", tls_set),
        ("kernel32.dll", "TlsGetValue", tls_get),
        ("kernel32.dll", "TlsFree", r1_1),
        ("kernel32.dll", "GetConsoleCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetConsoleOutputCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetConsoleMode", r0_2),
        ("kernel32.dll", "SetConsoleMode", r1_2),
        ("kernel32.dll", "SetConsoleCtrlHandler", r1_2),
        ("kernel32.dll", "MultiByteToWideChar", multibyte_to_widechar),
        ("kernel32.dll", "WideCharToMultiByte", widechar_to_multibyte),
        ("kernel32.dll", "GetACP", |c| {
            c.ret_stdcall(1252, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetOEMCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "IsValidCodePage", r1_1),
        ("kernel32.dll", "GetCPInfo", r0_2),
        ("kernel32.dll", "LCMapStringW", r0_6),
        ("kernel32.dll", "LCMapStringEx", r0_8),
        ("kernel32.dll", "FindFirstFileA", |c| {
            c.ret_stdcall(INVALID_HANDLE, 2);
            Handled::Ok
        }),
        ("kernel32.dll", "FindClose", r1_1),
        ("kernel32.dll", "CreateFileA", create_file_a),
        ("kernel32.dll", "CreateFileW", create_file_w),
        ("kernel32.dll", "ReadFile", read_file),
        ("kernel32.dll", "WriteFile", write_file),
        ("kernel32.dll", "GetFileSize", get_file_size),
        ("kernel32.dll", "GetFileSizeEx", get_file_size),
        ("kernel32.dll", "SetFilePointer", set_file_pointer),
        ("kernel32.dll", "CreateDirectoryA", create_directory_a),
        ("kernel32.dll", "CreateDirectoryW", create_directory_w),
        ("kernel32.dll", "DeleteFileA", delete_file_a),
        ("kernel32.dll", "DeleteFileW", delete_file_w),
        ("kernel32.dll", "GetFileAttributesA", get_file_attributes_a),
        ("kernel32.dll", "GetFileType", get_file_type),
        ("kernel32.dll", "SetHandleInformation", r1_3),
        ("kernel32.dll", "DuplicateHandle", dup_handle),
        ("kernel32.dll", "TerminateProcess", terminate_process),
        ("kernel32.dll", "RaiseException", raise_exception),
        ("kernel32.dll", "GetStringTypeW", r0_4),
        ("kernel32.dll", "GetStringTypeA", r0_5),
        ("kernel32.dll", "FormatMessageA", r0_7),
        ("kernel32.dll", "FormatMessageW", r0_7),
        ("kernel32.dll", "OutputDebugStringA", output_debug_string_a),
        ("kernel32.dll", "OutputDebugStringW", r0_1),
        ("kernel32.dll", "EncodePointer", |c| {
            let p = c.arg(0);
            c.ret_stdcall(p ^ 0xDEAD, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "DecodePointer", |c| {
            let p = c.arg(0);
            c.ret_stdcall(p ^ 0xDEAD, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "Sleep", r0_1),
        ("kernel32.dll", "WaitForSingleObject", r0_2), // WAIT_OBJECT_0
        ("kernel32.dll", "CreateEventA", r0_4),
        ("kernel32.dll", "CreateEventW", r0_4),
        ("kernel32.dll", "CreateMutexA", r0_3),
        ("kernel32.dll", "CreateMutexW", r0_3),
        ("kernel32.dll", "ReleaseMutex", r1_1),
        ("kernel32.dll", "SetEvent", r1_1),
        ("kernel32.dll", "ResetEvent", r1_1),
        ("kernel32.dll", "InterlockedIncrement", interlocked_inc),
        ("kernel32.dll", "InterlockedDecrement", interlocked_dec),
        ("kernel32.dll", "InterlockedExchange", interlocked_xchg),
        (
            "kernel32.dll",
            "InterlockedCompareExchange",
            interlocked_cmpxchg,
        ),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn exit_process(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(ctx.arg(0))
}

fn terminate_process(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(ctx.arg(1))
}

fn get_std_handle(ctx: &mut ApiContext) -> Handled {
    let id = ctx.arg(0);
    let h = match id {
        0xFFFF_FFF6 => STD_INPUT_HANDLE,
        0xFFFF_FFF5 => STD_OUTPUT_HANDLE,
        0xFFFF_FFF4 => STD_ERROR_HANDLE,
        _ => INVALID_HANDLE,
    };
    ctx.ret_stdcall(h, 1);
    Handled::Ok
}

fn write_file(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let buf = ctx.arg(1);
    let count = ctx.arg(2);
    let out = ctx.arg(3);
    let bytes = ctx
        .memory
        .read_bytes(buf, count as usize)
        .unwrap_or_default();

    // VFS-backed file handle?
    let file = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, cursor, .. }) => Some((path.clone(), *cursor)),
        _ => None,
    };

    if let Some((path, cursor)) = file {
        let mut content = ctx.fs.read_file(&path).unwrap_or_default();
        let start = cursor as usize;
        let end = start + bytes.len();
        if content.len() < end {
            content.resize(end, 0);
        }
        content[start..end].copy_from_slice(&bytes);
        let _ = ctx.fs.mount_file(&path, content);
        if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(handle) {
            *cursor += bytes.len() as u64;
        }
    } else {
        route_output(handle, &bytes, ctx);
    }

    if out != 0 {
        let _ = ctx.memory.write_u32(out, bytes.len() as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

fn write_console_a(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let buf = ctx.arg(1);
    let count = ctx.arg(2);
    let out = ctx.arg(3);
    let bytes = ctx
        .memory
        .read_bytes(buf, count as usize)
        .unwrap_or_default();
    route_output(handle, &bytes, ctx);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, bytes.len() as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

fn write_console_w(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let buf = ctx.arg(1);
    let count = ctx.arg(2);
    let out = ctx.arg(3);
    let s = ctx.memory.read_wstr(buf);
    // truncate to count chars
    let s: String = s.chars().take(count as usize).collect();
    route_output(handle, s.as_bytes(), ctx);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, count);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

fn route_output(handle: u32, bytes: &[u8], ctx: &mut ApiContext) {
    match handle {
        h if h == STD_OUTPUT_HANDLE => ctx.console.stdout.extend_from_slice(bytes),
        h if h == STD_ERROR_HANDLE => ctx.console.stderr.extend_from_slice(bytes),
        _ => ctx.console.stdout.extend_from_slice(bytes), // assume stdout for file handles
    }
}

fn read_file(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let buf = ctx.arg(1);
    let max = ctx.arg(2);
    let out = ctx.arg(3);

    let file = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, cursor, .. }) => Some((path.clone(), *cursor)),
        _ => None,
    };

    let n = if let Some((path, cursor)) = file {
        let content = ctx.fs.read_file(&path).unwrap_or_default();
        let start = (cursor as usize).min(content.len());
        let n = (max as usize).min(content.len() - start);
        if n > 0 {
            let _ = ctx.memory.write_bytes(buf, &content[start..start + n]);
            if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(handle) {
                *cursor += n as u64;
            }
        }
        n
    } else {
        // console stdin
        let n = max.min(ctx.console.stdin.len() as u32) as usize;
        let data: Vec<u8> = ctx.console.stdin.drain(..n).collect();
        if !data.is_empty() {
            let _ = ctx.memory.write_bytes(buf, &data);
        }
        n
    };

    if out != 0 {
        let _ = ctx.memory.write_u32(out, n as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

fn close_handle(ctx: &mut ApiContext) -> Handled {
    let h = ctx.arg(0);
    ctx.handles.remove(h);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// ── file APIs (Milestone 7) ──────────────────────────────────────────────────

const CREATE_NEW: u32 = 1;
const CREATE_ALWAYS: u32 = 2;
const OPEN_EXISTING: u32 = 3;
const TRUNCATE_EXISTING: u32 = 5;

fn create_file(ctx: &mut ApiContext, name: String, nargs: u32) -> Handled {
    let access = ctx.arg(1);
    let disposition = ctx.arg(4);
    let path = resolve_path(&name);
    let exists = ctx.fs.node_exists(&path);
    let writable = access & 0x4000_0000 != 0; // GENERIC_WRITE

    // disposition rules
    if disposition == OPEN_EXISTING && !exists {
        ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
        ctx.ret_stdcall(INVALID_HANDLE, nargs);
        return Handled::Ok;
    }
    if disposition == CREATE_NEW && exists {
        ctx.cpu.last_error = ERROR_FILE_EXISTS;
        ctx.ret_stdcall(INVALID_HANDLE, nargs);
        return Handled::Ok;
    }

    let truncate = disposition == CREATE_ALWAYS || disposition == TRUNCATE_EXISTING;
    if !exists || truncate {
        if ctx.fs.mount_file(&path, Vec::new()).is_err() {
            ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
            ctx.ret_stdcall(INVALID_HANDLE, nargs);
            return Handled::Ok;
        }
    }

    let h = ctx.handles.insert(KernelObject::VfsFile { path, cursor: 0, writable });
    ctx.cpu.last_error = 0;
    ctx.ret_stdcall(h, nargs);
    Handled::Ok
}

fn create_file_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    create_file(ctx, name, 7)
}

fn create_file_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    create_file(ctx, name, 7)
}

fn get_file_size(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let high = ctx.arg(1);
    let size = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, .. }) => {
            ctx.fs.read_file(path).map(|b| b.len()).unwrap_or(0) as u32
        }
        _ => 0,
    };
    if high != 0 {
        let _ = ctx.memory.write_u32(high, 0);
    }
    ctx.ret_stdcall(size, 2);
    Handled::Ok
}

fn set_file_pointer(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let dist = ctx.arg(1) as i32 as i64;
    let method = ctx.arg(3); // FILE_BEGIN=0, FILE_CURRENT=1, FILE_END=2

    let (cur, size) = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, cursor, .. }) => (
            *cursor as i64,
            ctx.fs.read_file(path).map(|b| b.len()).unwrap_or(0) as i64,
        ),
        _ => {
            ctx.ret_stdcall(INVALID_HANDLE, 4);
            return Handled::Ok;
        }
    };
    let base = match method { 1 => cur, 2 => size, _ => 0 };
    let new_pos = (base + dist).max(0) as u64;
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(handle) {
        *cursor = new_pos;
    }
    ctx.ret_stdcall(new_pos as u32, 4);
    Handled::Ok
}

fn create_directory(ctx: &mut ApiContext, name: String) -> Handled {
    let path = resolve_path(&name);
    if ctx.fs.node_exists(&path) {
        ctx.cpu.last_error = 183; // ERROR_ALREADY_EXISTS
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    }
    let ok = ctx.fs.create_dir(&path).is_ok();
    ctx.ret_stdcall(ok as u32, 2);
    Handled::Ok
}

fn create_directory_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    create_directory(ctx, name)
}

fn create_directory_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    create_directory(ctx, name)
}

fn delete_file(ctx: &mut ApiContext, name: String) -> Handled {
    let path = resolve_path(&name);
    let ok = ctx.fs.delete_node(&path).is_ok();
    ctx.ret_stdcall(ok as u32, 1);
    Handled::Ok
}

fn delete_file_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    delete_file(ctx, name)
}

fn delete_file_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    delete_file(ctx, name)
}

fn get_file_attributes(ctx: &mut ApiContext, name: String) -> Handled {
    let path = resolve_path(&name);
    let attr = if !ctx.fs.node_exists(&path) {
        ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
        INVALID_FILE_ATTRIBUTES
    } else if ctx.fs.read_file(&path).is_ok() {
        FILE_ATTRIBUTE_NORMAL
    } else {
        FILE_ATTRIBUTE_DIRECTORY
    };
    ctx.ret_stdcall(attr, 1);
    Handled::Ok
}

fn get_file_attributes_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    get_file_attributes(ctx, name)
}

fn get_file_attributes_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    get_file_attributes(ctx, name)
}

fn get_last_error(ctx: &mut ApiContext) -> Handled {
    let e = ctx.cpu.last_error;
    ctx.ret_stdcall(e, 0);
    Handled::Ok
}

fn set_last_error(ctx: &mut ApiContext) -> Handled {
    ctx.cpu.last_error = ctx.arg(0);
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn get_process_heap(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0x1000_0000, 0);
    Handled::Ok
}

fn heap_alloc(ctx: &mut ApiContext) -> Handled {
    let size = ctx.arg(2);
    let ptr = ctx.heap_alloc(size);
    ctx.ret_stdcall(ptr, 3);
    Handled::Ok
}

fn heap_free(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

fn heap_realloc(ctx: &mut ApiContext) -> Handled {
    let size = ctx.arg(3);
    let ptr = ctx.heap_alloc(size);
    ctx.ret_stdcall(ptr, 4);
    Handled::Ok
}

fn heap_size(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0, 3);
    Handled::Ok
}

fn heap_create(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0x1000_0000, 3);
    Handled::Ok
}

fn virtual_alloc(ctx: &mut ApiContext) -> Handled {
    let addr = ctx.arg(0);
    let size = ctx.arg(1);
    let ptr = if addr != 0 {
        addr
    } else {
        ctx.heap_alloc(size)
    };
    ctx.ret_stdcall(ptr, 4);
    Handled::Ok
}

fn virtual_protect(ctx: &mut ApiContext) -> Handled {
    let old_out = ctx.arg(3);
    if old_out != 0 {
        let _ = ctx.memory.write_u32(old_out, 0x40);
    }
    ctx.ret_stdcall(1, 4);
    Handled::Ok
}

fn get_module_handle_a(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(
        ctx.memory
            .regions
            .first()
            .map(|r| r.base)
            .unwrap_or(0x0040_0000),
        1,
    );
    Handled::Ok
}

fn get_module_handle_w(ctx: &mut ApiContext) -> Handled {
    get_module_handle_a(ctx)
}

// GetModuleHandleEx(flags, name, &out_module) — write image base, return TRUE
fn get_module_handle_ex(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(2);
    let base = ctx.memory.regions.first().map(|r| r.base).unwrap_or(0x0040_0000);
    if out != 0 { let _ = ctx.memory.write_u32(out, base); }
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

fn get_module_filename_a(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let cap = ctx.arg(2);
    let name = b"C:\\Users\\guest\\Desktop\\program.exe\0";
    let n = name.len().min(cap as usize);
    let _ = ctx.memory.write_bytes(buf, &name[..n]);
    ctx.ret_stdcall(n as u32, 3);
    Handled::Ok
}

fn get_command_line_a(ctx: &mut ApiContext) -> Handled {
    // Write an empty command line at a fixed PEB-area address
    let va = 0x7FFD_F100;
    let _ = ctx.memory.write_bytes(va, b"program.exe\0");
    ctx.ret_stdcall(va, 0);
    Handled::Ok
}

fn get_command_line_w(ctx: &mut ApiContext) -> Handled {
    let va = 0x7FFD_F200;
    let wide: Vec<u8> = "program.exe\0"
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let _ = ctx.memory.write_bytes(va, &wide);
    ctx.ret_stdcall(va, 0);
    Handled::Ok
}

fn get_startup_info_a(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_bytes(p, &[0u8; 68]);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn get_current_process_id(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(ctx.pid, 0);
    Handled::Ok
}

fn get_current_thread_id(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(ctx.pid * 100, 0);
    Handled::Ok
}

fn get_current_process(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(CURRENT_PROCESS, 0);
    Handled::Ok
}

fn get_current_thread(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(CURRENT_THREAD, 0);
    Handled::Ok
}

fn get_system_time(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_bytes(p, &[0u8; 8]);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn query_perf_counter(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_bytes(p, &[0u8; 8]);
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn query_perf_freq(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx
            .memory
            .write_bytes(p, &[0x40, 0x42, 0x0F, 0, 0, 0, 0, 0]);
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn get_file_type(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(2, 1);
    Handled::Ok // FILE_TYPE_CHAR
}

fn dup_handle(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(5);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, ctx.arg(2));
    }
    ctx.ret_stdcall(1, 7);
    Handled::Ok
}

fn raise_exception(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(1)
}

fn output_debug_string_a(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let s = ctx.cstr(p);
    ctx.console.stderr.extend_from_slice(s.as_bytes());
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn tls_alloc(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0, 0);
    Handled::Ok
}

fn tls_set(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

fn tls_get(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn multibyte_to_widechar(ctx: &mut ApiContext) -> Handled {
    let src = ctx.arg(2);
    let srcl = ctx.arg(3);
    let dst = ctx.arg(4);
    let dstl = ctx.arg(5);
    let s = if srcl == 0xFFFF_FFFF {
        ctx.cstr(src)
    } else {
        String::from_utf8_lossy(
            &ctx.memory
                .read_bytes(src, srcl as usize)
                .unwrap_or_default(),
        )
        .into_owned()
    };
    let wide: Vec<u16> = s.encode_utf16().collect();
    let out_len = if dstl == 0 {
        wide.len() + 1
    } else {
        if dst != 0 {
            for (i, &c) in wide.iter().take(dstl as usize).enumerate() {
                let _ = ctx.memory.write_u16(dst + (i as u32) * 2, c);
            }
            if (wide.len() as u32) < dstl {
                let _ = ctx.memory.write_u16(dst + wide.len() as u32 * 2, 0);
            }
        }
        wide.len().min(dstl as usize) + 1
    };
    ctx.ret_stdcall(out_len as u32, 6);
    Handled::Ok
}

fn widechar_to_multibyte(ctx: &mut ApiContext) -> Handled {
    let src = ctx.arg(2);
    let srcl = ctx.arg(3);
    let dst = ctx.arg(4);
    let dstl = ctx.arg(5);
    let nchars = if srcl == 0xFFFF_FFFF {
        ctx.memory.read_wstr(src).len() + 1
    } else {
        srcl as usize
    };
    let s = ctx.memory.read_wstr(src);
    let bytes = s.as_bytes();
    let n = bytes.len().min(if dstl == 0 { 0 } else { dstl as usize });
    if n > 0 && dst != 0 {
        let _ = ctx.memory.write_bytes(dst, &bytes[..n]);
    }
    ctx.ret_stdcall(if dstl == 0 { nchars as u32 } else { n as u32 }, 8);
    Handled::Ok
}

fn interlocked_inc(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let v = ctx.memory.read_u32(p).unwrap_or(0).wrapping_add(1);
    let _ = ctx.memory.write_u32(p, v);
    ctx.ret_stdcall(v, 1);
    Handled::Ok
}

fn interlocked_dec(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let v = ctx.memory.read_u32(p).unwrap_or(0).wrapping_sub(1);
    let _ = ctx.memory.write_u32(p, v);
    ctx.ret_stdcall(v, 1);
    Handled::Ok
}

fn interlocked_xchg(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let new = ctx.arg(1);
    let old = ctx.memory.read_u32(p).unwrap_or(0);
    let _ = ctx.memory.write_u32(p, new);
    ctx.ret_stdcall(old, 2);
    Handled::Ok
}

fn interlocked_cmpxchg(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let new = ctx.arg(1);
    let cmp = ctx.arg(2);
    let old = ctx.memory.read_u32(p).unwrap_or(0);
    if old == cmp {
        let _ = ctx.memory.write_u32(p, new);
    }
    ctx.ret_stdcall(old, 3);
    Handled::Ok
}

// stub helpers

// Stub family: return a constant in EAX and clean exactly `n` stdcall args.
// Naming: r{val}_{nargs}. Correct arg counts matter — a wrong count drifts
// the guest stack and eventually derails a `ret`.
macro_rules! stubs {
    ($($name:ident => ($val:expr, $n:expr)),* $(,)?) => {
        $( #[allow(dead_code)] fn $name(c: &mut ApiContext) -> Handled { c.ret_stdcall($val, $n); Handled::Ok } )*
    };
}

stubs! {
    r0_0 => (0, 0), r0_1 => (0, 1), r0_2 => (0, 2), r0_3 => (0, 3),
    r0_4 => (0, 4), r0_5 => (0, 5), r0_6 => (0, 6), r0_7 => (0, 7),
    r0_8 => (0, 8),
    r1_0 => (1, 0), r1_1 => (1, 1), r1_2 => (1, 2), r1_3 => (1, 3),
    r1_4 => (1, 4), r1_6 => (1, 6), r1_7 => (1, 7),
}

fn stub_invalid_handle(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(INVALID_HANDLE, 7);
    Handled::Ok
}
