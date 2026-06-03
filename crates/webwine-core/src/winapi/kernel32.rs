use super::{ApiContext, Handled, WinApiRegistry};
use crate::vm::handles::{
    KernelObject, CURRENT_PROCESS, CURRENT_THREAD, INVALID_HANDLE, STD_ERROR_HANDLE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};

// Win32 error codes used by the file APIs.
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_FILE_EXISTS: u32 = 80;

const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const INVALID_FILE_ATTRIBUTES: u32 = 0xFFFF_FFFF;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        // advapi32 registry: we have no registry, so report "key not found" and
        // clean the exact arg counts (a wrong count corrupts the guest stack).
        // ERROR_FILE_NOT_FOUND (2) makes apps fall back to defaults.
        ("advapi32.dll", "RegOpenKeyExA", reg_open_key),
        ("advapi32.dll", "RegOpenKeyExW", reg_open_key),
        ("advapi32.dll", "RegOpenKeyA", |c| { let o = c.arg(2); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 3); Handled::Ok }),
        ("advapi32.dll", "RegQueryValueExA", reg_query_value),
        ("advapi32.dll", "RegQueryValueExW", |c| { c.ret_stdcall(2, 6); Handled::Ok }),
        ("advapi32.dll", "RegCreateKeyExA", |c| { let o = c.arg(7); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 9); Handled::Ok }),
        ("advapi32.dll", "RegSetValueExA", |c| { c.ret_stdcall(0, 6); Handled::Ok }),
        ("advapi32.dll", "RegSetValueExW", |c| { c.ret_stdcall(0, 6); Handled::Ok }),
        ("advapi32.dll", "RegCloseKey", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("advapi32.dll", "RegOpenKeyW", |c| { let o = c.arg(2); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 3); Handled::Ok }),
        ("advapi32.dll", "RegCreateKeyExW", |c| { let o = c.arg(7); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 9); Handled::Ok }),
        ("advapi32.dll", "RegDeleteValueW", |c| { c.ret_stdcall(2, 2); Handled::Ok }),
        ("advapi32.dll", "RegDeleteKeyW", |c| { c.ret_stdcall(2, 2); Handled::Ok }),
        ("advapi32.dll", "RegEnumValueW", |c| { c.ret_stdcall(0x103, 8); Handled::Ok }),  // ERROR_NO_MORE_ITEMS
        ("advapi32.dll", "RegEnumKeyExW", |c| { c.ret_stdcall(0x103, 8); Handled::Ok }),
        ("advapi32.dll", "RegQueryInfoKeyW", |c| { c.ret_stdcall(0, 12); Handled::Ok }),
        ("advapi32.dll", "RegQueryValueW", |c| { c.ret_stdcall(2, 4); Handled::Ok }),
        ("advapi32.dll", "RegNotifyChangeKeyValue", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("advapi32.dll", "RegFlushKey", |c| { c.ret_stdcall(0, 1); Handled::Ok }),

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
        ("kernel32.dll", "GetModuleFileNameW", get_module_filename_w),
        ("kernel32.dll", "SetCurrentDirectoryA", set_current_directory_a),
        ("kernel32.dll", "SetCurrentDirectoryW", set_current_directory_w),
        ("kernel32.dll", "GetProcAddress", get_proc_address),
        // UI language / locale (cmd.exe resolves these via GetProcAddress). 0x409 = en-US.
        ("kernel32.dll", "SetThreadUILanguage", |c| { let l = c.arg(0); c.ret_stdcall(if l == 0 { 0x409 } else { l }, 1); Handled::Ok }),
        ("kernel32.dll", "GetThreadUILanguage", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetUserDefaultUILanguage", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetSystemDefaultUILanguage", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetUserDefaultLangID", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetSystemDefaultLangID", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetUserDefaultLCID", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetSystemDefaultLCID", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetLocaleInfoA", r0_4),
        ("kernel32.dll", "GetLocaleInfoW", r0_4),
        ("kernel32.dll", "GetThreadLocale", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "SetThreadLocale", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        // Report Windows XP (5.1 build 2600). GetVersion: (build<<16)|(minor<<8)|major.
        ("kernel32.dll", "GetVersion", |c| { c.ret_stdcall(0x0A28_0105, 0); Handled::Ok }),
        ("kernel32.dll", "GetVersionExA", get_version_ex),
        ("kernel32.dll", "GetVersionExW", get_version_ex),
        ("kernel32.dll", "LoadLibraryA", load_library_a),
        ("kernel32.dll", "LoadLibraryW", load_library_w),
        ("kernel32.dll", "LoadLibraryExA", |c| { c.ret_stdcall(FAKE_MODULE, 3); Handled::Ok }),
        ("kernel32.dll", "LoadLibraryExW", |c| { c.ret_stdcall(FAKE_MODULE, 3); Handled::Ok }),
        ("kernel32.dll", "FreeLibrary", r1_1),
        ("kernel32.dll", "IsDebuggerPresent", r0_0),
        ("kernel32.dll", "IsProcessorFeaturePresent", r1_1),
        ("kernel32.dll", "InitializeSListHead", r0_1),
        ("kernel32.dll", "QueryDepthSList", r0_1),
        ("kernel32.dll", "InterlockedPushEntrySList", r0_2),
        ("kernel32.dll", "InterlockedFlushSList", r0_1),
        ("kernel32.dll", "GetProcessAffinityMask", r1_3),
        ("kernel32.dll", "GetNativeSystemInfo", r0_1),
        (
            "kernel32.dll",
            "GetCurrentDirectoryW",
            get_current_directory_w,
        ),
        ("kernel32.dll", "SetThreadErrorMode", r1_2),
        ("kernel32.dll", "GetThreadPriority", r0_1),
        ("kernel32.dll", "AddVectoredExceptionHandler", r1_2),
        ("kernel32.dll", "RemoveVectoredExceptionHandler", r1_1),
        ("kernel32.dll", "SetThreadStackGuarantee", r1_1),
        ("kernel32.dll", "GetModuleHandleExW", get_module_handle_ex),
        ("kernel32.dll", "GetModuleHandleExA", get_module_handle_ex),
        (
            "kernel32.dll",
            "GetSystemTimePreciseAsFileTime",
            get_system_time,
        ),
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
        ("kernel32.dll", "GetStartupInfoA", get_startup_info),
        ("kernel32.dll", "GetStartupInfoW", get_startup_info),
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
        ("kernel32.dll", "GetTickCount", get_tick_count),
        ("kernel32.dll", "GetTickCount64", get_tick_count),
        ("kernel32.dll", "FlushFileBuffers", r1_1),
        ("kernel32.dll", "SetFilePointer", r0_4),
        ("kernel32.dll", "SetUnhandledExceptionFilter", r0_1),
        ("kernel32.dll", "UnhandledExceptionFilter", r0_1),
        ("kernel32.dll", "GetEnvironmentVariableW", get_env_var_w),
        ("kernel32.dll", "GetEnvironmentVariableA", get_env_var_a),
        ("kernel32.dll", "SetEnvironmentVariableW", r1_2),
        ("kernel32.dll", "SetEnvironmentVariableA", r1_2),
        (
            "kernel32.dll",
            "GetCurrentDirectoryA",
            get_current_directory_a,
        ),
        ("kernel32.dll", "GetFullPathNameW", get_full_path_name_w),
        ("kernel32.dll", "GetFullPathNameA", get_full_path_name_a),
        ("kernel32.dll", "GetFileAttributesW", get_file_attributes_w),
        ("kernel32.dll", "GetFileAttributesA", get_file_attributes_a),
        ("kernel32.dll", "FindFirstFileW", find_first_file_w),
        ("kernel32.dll", "FindFirstFileA", find_first_file_a),
        ("kernel32.dll", "FindNextFileW", find_next_file_w),
        ("kernel32.dll", "FindNextFileA", find_next_file_a),
        ("kernel32.dll", "FindClose", find_close),
        ("kernel32.dll", "GetStdHandle", get_std_handle),
        ("kernel32.dll", "WriteConsoleW", write_console_w),
        ("kernel32.dll", "GetEnvironmentStringsW", |c| { let p = env_block(c, true); c.ret_stdcall(p, 0); Handled::Ok }),
        ("kernel32.dll", "GetEnvironmentStrings", |c| { let p = env_block(c, false); c.ret_stdcall(p, 0); Handled::Ok }),
        ("kernel32.dll", "FreeEnvironmentStringsW", r1_1),
        ("kernel32.dll", "GetEnvironmentStringsA", |c| { let p = env_block(c, false); c.ret_stdcall(p, 0); Handled::Ok }),
        ("kernel32.dll", "FreeEnvironmentStringsA", r1_1),
        ("kernel32.dll", "InitializeCriticalSection", r0_1),
        (
            "kernel32.dll",
            "InitializeCriticalSectionAndSpinCount",
            r1_2,
        ),
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
        ("kernel32.dll", "GetConsoleMode", get_console_mode),
        ("kernel32.dll", "SetConsoleMode", r1_2),
        ("kernel32.dll", "GetConsoleScreenBufferInfo", get_console_screen_buffer_info),
        ("kernel32.dll", "SetConsoleTextAttribute", r1_2),
        ("kernel32.dll", "SetConsoleCursorPosition", r1_2),
        ("kernel32.dll", "SetConsoleTitleA", r1_1),
        ("kernel32.dll", "SetConsoleTitleW", r1_1),
        ("kernel32.dll", "GetConsoleTitleA", r0_2),
        ("kernel32.dll", "GetConsoleTitleW", r0_2),
        ("kernel32.dll", "SetConsoleScreenBufferSize", r1_2),
        ("kernel32.dll", "FillConsoleOutputCharacterA", r1_5),
        ("kernel32.dll", "FillConsoleOutputAttribute", r1_5),
        ("kernel32.dll", "ScrollConsoleScreenBufferA", r1_5),
        ("kernel32.dll", "SetConsoleWindowInfo", r1_3),
        ("kernel32.dll", "GetConsoleCursorInfo", r1_2),
        ("kernel32.dll", "SetConsoleCursorInfo", r1_2),
        ("kernel32.dll", "ReadConsoleA", read_console_a),
        ("kernel32.dll", "ReadConsoleW", read_console_w),
        ("kernel32.dll", "SetConsoleInputExeNameW", r1_1),
        ("kernel32.dll", "SetConsoleInputExeNameA", r1_1),
        ("kernel32.dll", "GetConsoleInputExeNameW", r1_2),
        ("kernel32.dll", "GetConsoleInputExeNameA", r1_2),
        ("kernel32.dll", "CopyFileExW", r1_6),
        ("kernel32.dll", "CopyFileExA", r1_6),
        ("kernel32.dll", "WriteConsoleInputW", r1_4),
        ("kernel32.dll", "ReadConsoleInputW", r0_4),
        ("kernel32.dll", "ReadConsoleInputA", r0_4),
        ("kernel32.dll", "PeekConsoleInputW", r0_4),
        ("kernel32.dll", "GetNumberOfConsoleInputEvents", r0_2),
        ("kernel32.dll", "FlushConsoleInputBuffer", r1_1),
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
        ("kernel32.dll", "CreateProcessA", create_process_a),
        ("kernel32.dll", "CreateProcessW", create_process_w),
        ("kernel32.dll", "GetExitCodeProcess", get_exit_code_process),
        ("kernel32.dll", "Beep", beep),
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
        // console stdin: block until input is available (interactive prompt).
        if ctx.console.stdin.is_empty() {
            return Handled::Block;
        }
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

// Read one line of console input (up to a newline or the char limit) from the
// process stdin buffer. Blocks (WaitingForInput) when no input is available, so
// the frontend can supply typed text. Returns the raw bytes consumed.
fn read_console_line(ctx: &mut ApiContext, limit: usize) -> Option<Vec<u8>> {
    if ctx.console.stdin.is_empty() {
        return None; // caller turns this into Handled::Block
    }
    let mut line = Vec::new();
    while line.len() < limit {
        match ctx.console.stdin.pop_front() {
            Some(b) => {
                line.push(b);
                if b == b'\n' {
                    break;
                }
            }
            None => break,
        }
    }
    Some(line)
}

// ReadConsoleW(hInput, lpBuffer, nChars, lpNumRead, pInputControl) — wide.
fn read_console_w(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let n_chars = ctx.arg(2) as usize;
    let out = ctx.arg(3);
    let Some(line) = read_console_line(ctx, n_chars) else {
        return Handled::Block;
    };
    let wide: Vec<u8> = line.iter().flat_map(|&b| (b as u16).to_le_bytes()).collect();
    let _ = ctx.memory.write_bytes(buf, &wide);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, line.len() as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

// ReadConsoleA(hInput, lpBuffer, nChars, lpNumRead, pInputControl) — narrow.
fn read_console_a(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let n_chars = ctx.arg(2) as usize;
    let out = ctx.arg(3);
    let Some(line) = read_console_line(ctx, n_chars) else {
        return Handled::Block;
    };
    let _ = ctx.memory.write_bytes(buf, &line);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, line.len() as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

// file APIs (Milestone 7)

const CREATE_NEW: u32 = 1;
const CREATE_ALWAYS: u32 = 2;
const OPEN_EXISTING: u32 = 3;
const TRUNCATE_EXISTING: u32 = 5;

fn create_file(ctx: &mut ApiContext, name: String, nargs: u32) -> Handled {
    let access = ctx.arg(1);
    let disposition = ctx.arg(4);
    let path = ctx.resolve_path(&name);
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

    let h = ctx.handles.insert(KernelObject::VfsFile {
        path,
        cursor: 0,
        writable,
    });
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
    let base = match method {
        1 => cur,
        2 => size,
        _ => 0,
    };
    let new_pos = (base + dist).max(0) as u64;
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(handle) {
        *cursor = new_pos;
    }
    ctx.ret_stdcall(new_pos as u32, 4);
    Handled::Ok
}

fn create_directory(ctx: &mut ApiContext, name: String) -> Handled {
    let path = ctx.resolve_path(&name);
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
    let path = ctx.resolve_path(&name);
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
    let path = ctx.resolve_path(&name);
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

// Match a filename against a DOS wildcard pattern (* and ?), case-insensitive.
fn wildcard_match(pattern: &str, name: &str) -> bool {
    fn m(p: &[u8], n: &[u8]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(b'*'), _) => m(&p[1..], n) || (!n.is_empty() && m(p, &n[1..])),
            (Some(b'?'), Some(_)) => m(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => m(&p[1..], &n[1..]),
            _ => false,
        }
    }
    m(pattern.as_bytes(), name.as_bytes())
}

// Split a search path "C:\dir\pattern" into (dir, pattern). A bare "*.*" or "*"
// matches everything.
fn split_search(path: &str) -> (String, String) {
    let p = path.replace('/', "\\");
    match p.rfind('\\') {
        Some(i) => (p[..i].to_string(), p[i + 1..].to_string()),
        None => (String::new(), p),
    }
}

// Build the match list for a FindFirstFile pattern against the VFS.
fn find_matches(ctx: &ApiContext, raw: &str) -> Vec<(String, bool, u64)> {
    let resolved = ctx.resolve_path(raw);
    let (dir, pat) = split_search(&resolved);
    let pat = if pat.is_empty() { "*".to_string() } else { pat };
    let mut out = Vec::new();
    if let Ok(entries) = ctx.fs.list_dir(&dir) {
        for e in entries {
            if wildcard_match(&pat, &e.name) {
                let is_dir = matches!(e.kind, crate::fs::vfs::EntryKind::Directory);
                out.push((e.name, is_dir, e.size));
            }
        }
    }
    out
}

// Fill a WIN32_FIND_DATAW at `p` for one entry (wide cFileName at +44).
fn fill_find_data_w(ctx: &mut ApiContext, p: u32, name: &str, is_dir: bool, size: u64) {
    let attrs = if is_dir { FILE_ATTRIBUTE_DIRECTORY } else { FILE_ATTRIBUTE_NORMAL };
    let _ = ctx.memory.write_bytes(p, &[0u8; 44]); // attrs + 3 FILETIMEs + sizes + reserved
    let _ = ctx.memory.write_u32(p, attrs);
    let _ = ctx.memory.write_u32(p + 28, (size >> 32) as u32);
    let _ = ctx.memory.write_u32(p + 32, size as u32);
    let mut wide: Vec<u8> = name.encode_utf16().take(259).flat_map(|c| c.to_le_bytes()).collect();
    wide.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(p + 44, &wide);
}

// Same, ANSI cFileName for WIN32_FIND_DATAA (cFileName at +44, bytes).
fn fill_find_data_a(ctx: &mut ApiContext, p: u32, name: &str, is_dir: bool, size: u64) {
    let attrs = if is_dir { FILE_ATTRIBUTE_DIRECTORY } else { FILE_ATTRIBUTE_NORMAL };
    let _ = ctx.memory.write_bytes(p, &[0u8; 44]);
    let _ = ctx.memory.write_u32(p, attrs);
    let _ = ctx.memory.write_u32(p + 28, (size >> 32) as u32);
    let _ = ctx.memory.write_u32(p + 32, size as u32);
    let mut bytes = name.as_bytes().to_vec();
    bytes.truncate(259);
    bytes.push(0);
    let _ = ctx.memory.write_bytes(p + 44, &bytes);
}

const INVALID_HANDLE_VALUE: u32 = 0xFFFF_FFFF;
const ERROR_NO_MORE_FILES: u32 = 18;

fn find_first_common(ctx: &mut ApiContext, raw: &str, data: u32, wide: bool) -> u32 {
    let matches = find_matches(ctx, raw);
    if matches.is_empty() {
        ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
        return INVALID_HANDLE_VALUE;
    }
    let (name, is_dir, size) = matches[0].clone();
    if wide { fill_find_data_w(ctx, data, &name, is_dir, size); }
    else { fill_find_data_a(ctx, data, &name, is_dir, size); }
    ctx.handles.insert(KernelObject::FindHandle { matches, cursor: 1 })
}

fn find_first_file_w(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.wstr(ctx.arg(0));
    let data = ctx.arg(1);
    let h = find_first_common(ctx, &raw, data, true);
    ctx.ret_stdcall(h, 2);
    Handled::Ok
}

fn find_first_file_a(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.cstr(ctx.arg(0));
    let data = ctx.arg(1);
    let h = find_first_common(ctx, &raw, data, false);
    ctx.ret_stdcall(h, 2);
    Handled::Ok
}

fn find_next_common(ctx: &mut ApiContext, handle: u32, data: u32, wide: bool) -> u32 {
    let next = match ctx.handles.get(handle) {
        Some(KernelObject::FindHandle { matches, cursor }) if *cursor < matches.len() => {
            Some(matches[*cursor].clone())
        }
        _ => None,
    };
    match next {
        Some((name, is_dir, size)) => {
            if wide { fill_find_data_w(ctx, data, &name, is_dir, size); }
            else { fill_find_data_a(ctx, data, &name, is_dir, size); }
            if let Some(KernelObject::FindHandle { cursor, .. }) = ctx.handles.get_mut(handle) {
                *cursor += 1;
            }
            1
        }
        None => {
            ctx.cpu.last_error = ERROR_NO_MORE_FILES;
            0
        }
    }
}

fn find_next_file_w(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let data = ctx.arg(1);
    let r = find_next_common(ctx, handle, data, true);
    ctx.ret_stdcall(r, 2);
    Handled::Ok
}

fn find_next_file_a(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let data = ctx.arg(1);
    let r = find_next_common(ctx, handle, data, false);
    ctx.ret_stdcall(r, 2);
    Handled::Ok
}

fn find_close(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    ctx.handles.remove(handle);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// CreateProcessA(appName, cmdLine, procAttr, threadAttr, inherit, flags, env,
//                cwd, startupInfo, processInfo) — 10 args.
fn create_process(ctx: &mut ApiContext, app: String, cmd: String) -> Handled {
    let pi = ctx.arg(9); // lpProcessInformation

    // Prefer lpApplicationName; otherwise take the first token of the command line.
    let target = if !app.is_empty() {
        app
    } else {
        first_token(&cmd)
    };
    let path = ctx.resolve_path(&target);

    if !ctx.fs.node_exists(&path) {
        ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
        ctx.ret_stdcall(0, 10); // FALSE
        return Handled::Ok;
    }

    let child = ctx.next_child_pid;
    // Fill PROCESS_INFORMATION { hProcess, hThread, dwProcessId, dwThreadId }.
    if pi != 0 {
        let fake_proc = 0x0000_0F00 | child;
        let _ = ctx.memory.write_u32(pi, fake_proc);
        let _ = ctx.memory.write_u32(pi + 4, fake_proc | 0x1_0000);
        let _ = ctx.memory.write_u32(pi + 8, child);
        let _ = ctx.memory.write_u32(pi + 12, child * 100);
    }
    ctx.spawns
        .push(crate::vm::process::SpawnRequest { path, pi_addr: pi });
    ctx.cpu.last_error = 0;
    ctx.ret_stdcall(1, 10); // TRUE
    Handled::Ok
}

fn create_process_a(ctx: &mut ApiContext) -> Handled {
    let app = if ctx.arg(0) != 0 {
        ctx.cstr(ctx.arg(0))
    } else {
        String::new()
    };
    let cmd = if ctx.arg(1) != 0 {
        ctx.cstr(ctx.arg(1))
    } else {
        String::new()
    };
    create_process(ctx, app, cmd)
}

fn create_process_w(ctx: &mut ApiContext) -> Handled {
    let app = if ctx.arg(0) != 0 {
        ctx.wstr(ctx.arg(0))
    } else {
        String::new()
    };
    let cmd = if ctx.arg(1) != 0 {
        ctx.wstr(ctx.arg(1))
    } else {
        String::new()
    };
    create_process(ctx, app, cmd)
}

fn first_token(cmd: &str) -> String {
    let cmd = cmd.trim();
    if cmd.starts_with('"') {
        cmd[1..].split('"').next().unwrap_or("").to_string()
    } else {
        cmd.split_whitespace().next().unwrap_or("").to_string()
    }
}

fn get_exit_code_process(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0);
    } // assume exited 0
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// GetProcAddress(hModule, lpProcName) — return a real trampoline for a function
// we implement (so the guest can call it), else 0 (caller uses its fallback).
fn get_proc_address(ctx: &mut ApiContext) -> Handled {
    let name_arg = ctx.arg(1);
    let va = if name_arg < 0x1_0000 {
        0 // imported by ordinal — not supported
    } else {
        let name = ctx.cstr(name_arg);
        let v = ctx.proc_addr.get(&name).copied().unwrap_or(0);
        if v == 0 {
            // A miss means a dynamically-resolved import we don't provide; the
            // guest may call NULL. Logged at trace for diagnosis ("Run as debug").
            ctx.logs.log(crate::logs::LogLevel::Trace, "api",
                &format!("GetProcAddress miss: {name}"), Some(ctx.pid));
        }
        v
    };
    ctx.ret_stdcall(va, 2);
    Handled::Ok
}

// Beep(dwFreq, dwDuration) — emit a UI beep for the frontend (Web Audio).
fn beep(ctx: &mut ApiContext) -> Handled {
    let freq = ctx.arg(0);
    let duration = ctx.arg(1);
    ctx.ui_events.push(crate::vm::process::UiEvent::Beep { freq, duration });
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// GetFullPathNameW(lpFileName, nBufferLength, lpBuffer, lpFilePart) — resolve
// to an absolute path. Rust std calls this to canonicalize before CreateFile.
fn get_full_path_name_w(ctx: &mut ApiContext) -> Handled {
    let input = ctx.wstr(ctx.arg(0));
    let buf_len = ctx.arg(1); // WCHARs
    let out = ctx.arg(2);
    let file_part = ctx.arg(3);

    let full = ctx.resolve_path(&input);
    let wide: Vec<u16> = full.encode_utf16().collect();
    let needed = wide.len() as u32;

    if out != 0 && buf_len > needed {
        for (i, &c) in wide.iter().enumerate() {
            let _ = ctx.memory.write_u16(out + (i as u32) * 2, c);
        }
        let _ = ctx.memory.write_u16(out + needed * 2, 0);
        if file_part != 0 {
            let last = full.rfind('\\').map(|i| i + 1).unwrap_or(0) as u32;
            let _ = ctx.memory.write_u32(file_part, out + last * 2);
        }
        ctx.ret_stdcall(needed, 4);
    } else {
        ctx.ret_stdcall(needed + 1, 4); // required size incl. null
    }
    Handled::Ok
}

fn get_full_path_name_a(ctx: &mut ApiContext) -> Handled {
    let input = ctx.cstr(ctx.arg(0));
    let buf_len = ctx.arg(1);
    let out = ctx.arg(2);
    let file_part = ctx.arg(3);

    let full = ctx.resolve_path(&input);
    let bytes = full.as_bytes();
    let needed = bytes.len() as u32;

    if out != 0 && buf_len > needed {
        let _ = ctx.memory.write_bytes(out, bytes);
        let _ = ctx.memory.write_u8(out + needed, 0);
        if file_part != 0 {
            let last = full.rfind('\\').map(|i| i + 1).unwrap_or(0) as u32;
            let _ = ctx.memory.write_u32(file_part, out + last);
        }
        ctx.ret_stdcall(needed, 4);
    } else {
        ctx.ret_stdcall(needed + 1, 4);
    }
    Handled::Ok
}

fn get_current_directory_w(ctx: &mut ApiContext) -> Handled {
    let buf_len = ctx.arg(0);
    let out = ctx.arg(1);
    let wide: Vec<u16> = ctx.cwd.encode_utf16().collect();
    let needed = wide.len() as u32;
    if out != 0 && buf_len > needed {
        for (i, &c) in wide.iter().enumerate() {
            let _ = ctx.memory.write_u16(out + (i as u32) * 2, c);
        }
        let _ = ctx.memory.write_u16(out + needed * 2, 0);
        ctx.ret_stdcall(needed, 2);
    } else {
        ctx.ret_stdcall(needed + 1, 2);
    }
    Handled::Ok
}

fn get_current_directory_a(ctx: &mut ApiContext) -> Handled {
    let buf_len = ctx.arg(0);
    let out = ctx.arg(1);
    let bytes = ctx.cwd.clone().into_bytes();
    let needed = bytes.len() as u32;
    if out != 0 && buf_len > needed {
        let _ = ctx.memory.write_bytes(out, &bytes);
        let _ = ctx.memory.write_u8(out + needed, 0);
        ctx.ret_stdcall(needed, 2);
    } else {
        ctx.ret_stdcall(needed + 1, 2);
    }
    Handled::Ok
}

// SetCurrentDirectory(path): update the process working directory. Returns
// nonzero on success (we always accept; the path need not exist in our VFS).
fn set_current_directory_a(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.cstr(ctx.arg(0));
    *ctx.cwd = ctx.resolve_path(&raw);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn set_current_directory_w(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.wstr(ctx.arg(0));
    *ctx.cwd = ctx.resolve_path(&raw);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// We have no registry. Report "key not found" so apps fall back to defaults
// (ERROR_FILE_NOT_FOUND). Returning success with bogus data breaks apps that
// read real settings from the registry (e.g. cmd.exe's Command Processor key).
fn reg_open_key(ctx: &mut ApiContext) -> Handled {
    let phk = ctx.arg(4);
    if phk != 0 {
        let _ = ctx.memory.write_u32(phk, 0);
    }
    ctx.ret_stdcall(2, 5); // ERROR_FILE_NOT_FOUND
    Handled::Ok
}

fn reg_query_value(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(2, 6); // ERROR_FILE_NOT_FOUND
    Handled::Ok
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
    // HeapReAlloc(hHeap, dwFlags, lpMem, dwBytes)
    let old = ctx.arg(2);
    let size = ctx.arg(3);
    let ptr = ctx.heap_realloc(old, size);
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

// A non-zero fake HMODULE for dynamically loaded DLLs. GetProcAddress resolves
// functions by name regardless of the module handle, so a single sentinel is
// enough — and returning non-NULL lets delay-load helpers succeed instead of
// raising ERROR_MOD_NOT_FOUND (0xC06D007E).
const FAKE_MODULE: u32 = 0x5AD0_0000;

fn load_library_a(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(FAKE_MODULE, 1);
    Handled::Ok
}

fn load_library_w(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(FAKE_MODULE, 1);
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
    let base = ctx
        .memory
        .regions
        .first()
        .map(|r| r.base)
        .unwrap_or(0x0040_0000);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, base);
    }
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

fn get_module_filename_a(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let cap = ctx.arg(2);
    let mut name = ctx.exe_path.as_bytes().to_vec();
    name.push(0);
    let n = name.len().min(cap as usize);
    let _ = ctx.memory.write_bytes(buf, &name[..n]);
    ctx.ret_stdcall(n.saturating_sub(1) as u32, 3); // excl. null
    Handled::Ok
}

fn get_module_filename_w(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let cap = ctx.arg(2);
    let wide: Vec<u16> = ctx.exe_path.encode_utf16().collect();
    let n = wide.len().min(cap.saturating_sub(1) as usize);
    for (i, &c) in wide[..n].iter().enumerate() {
        let _ = ctx.memory.write_u16(buf + (i as u32) * 2, c);
    }
    let _ = ctx.memory.write_u16(buf + (n as u32) * 2, 0);
    ctx.ret_stdcall(n as u32, 3);
    Handled::Ok
}

// Default process environment. cmd.exe aborts with "Null environment" if this
// is empty, and apps expect the usual variables. Block format: consecutive
// "NAME=VALUE\0" entries terminated by a final empty string (extra \0).
const ENV_VARS: &[&str] = &[
    "ALLUSERSPROFILE=C:\\Users\\guest",
    "APPDATA=C:\\Users\\guest\\AppData\\Roaming",
    "ComSpec=C:\\Windows\\System32\\cmd.exe",
    "COMPUTERNAME=WEBWINE",
    "HOMEDRIVE=C:",
    "HOMEPATH=\\Users\\guest",
    "NUMBER_OF_PROCESSORS=1",
    "OS=Windows_NT",
    "Path=C:\\Windows\\System32;C:\\Windows",
    "PATHEXT=.COM;.EXE;.BAT;.CMD",
    "PROMPT=$P$G",
    "SystemDrive=C:",
    "SystemRoot=C:\\Windows",
    "TEMP=C:\\Users\\guest\\AppData\\Local\\Temp",
    "TMP=C:\\Users\\guest\\AppData\\Local\\Temp",
    "USERNAME=guest",
    "USERPROFILE=C:\\Users\\guest",
    "windir=C:\\Windows",
];

// Look up an environment variable's value by name (case-insensitive).
fn env_lookup(name: &str) -> Option<&'static str> {
    ENV_VARS.iter().find_map(|v| {
        let (k, val) = v.split_once('=')?;
        if k.eq_ignore_ascii_case(name) { Some(val) } else { None }
    })
}

// ERROR_ENVVAR_NOT_FOUND
const ERROR_ENVVAR_NOT_FOUND: u32 = 203;

fn get_env_var_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    let buf = ctx.arg(1);
    let size = ctx.arg(2);
    match env_lookup(&name) {
        Some(val) => {
            let mut bytes = val.as_bytes().to_vec();
            let needed = bytes.len() as u32 + 1;
            if buf != 0 && size >= needed {
                bytes.push(0);
                let _ = ctx.memory.write_bytes(buf, &bytes);
                ctx.ret_stdcall(needed - 1, 3);
            } else {
                ctx.ret_stdcall(needed, 3); // required size incl. NUL
            }
        }
        None => { ctx.cpu.last_error = ERROR_ENVVAR_NOT_FOUND; ctx.ret_stdcall(0, 3); }
    }
    Handled::Ok
}

fn get_env_var_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    let buf = ctx.arg(1);
    let size = ctx.arg(2); // in WCHARs
    match env_lookup(&name) {
        Some(val) => {
            let units: Vec<u16> = val.encode_utf16().collect();
            let needed = units.len() as u32 + 1;
            if buf != 0 && size >= needed {
                let mut bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
                bytes.extend_from_slice(&[0, 0]);
                let _ = ctx.memory.write_bytes(buf, &bytes);
                ctx.ret_stdcall(needed - 1, 3);
            } else {
                ctx.ret_stdcall(needed, 3);
            }
        }
        None => { ctx.cpu.last_error = ERROR_ENVVAR_NOT_FOUND; ctx.ret_stdcall(0, 3); }
    }
    Handled::Ok
}

// Build the environment block in guest memory and return its pointer.
fn env_block(ctx: &mut ApiContext, wide: bool) -> u32 {
    if wide {
        let mut units: Vec<u16> = Vec::new();
        for v in ENV_VARS {
            units.extend(v.encode_utf16());
            units.push(0);
        }
        units.push(0); // final terminator
        let bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
        let p = ctx.heap_alloc(bytes.len() as u32);
        let _ = ctx.memory.write_bytes(p, &bytes);
        p
    } else {
        let mut bytes: Vec<u8> = Vec::new();
        for v in ENV_VARS {
            bytes.extend_from_slice(v.as_bytes());
            bytes.push(0);
        }
        bytes.push(0);
        let p = ctx.heap_alloc(bytes.len() as u32);
        let _ = ctx.memory.write_bytes(p, &bytes);
        p
    }
}

fn get_command_line_a(ctx: &mut ApiContext) -> Handled {
    // The full command line (argv[0] is the quoted image path). PEB-area scratch.
    let va = 0x7FFD_F100;
    let line = format!("{}\0", ctx.cmdline);
    let _ = ctx.memory.write_bytes(va, line.as_bytes());
    ctx.ret_stdcall(va, 0);
    Handled::Ok
}

fn get_command_line_w(ctx: &mut ApiContext) -> Handled {
    let va = 0x7FFD_F200;
    let line = format!("{}\0", ctx.cmdline);
    let wide: Vec<u8> = line.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    let _ = ctx.memory.write_bytes(va, &wide);
    ctx.ret_stdcall(va, 0);
    Handled::Ok
}

// STARTUPINFO is 68 bytes; cb at +0. Zero everything (so dwFlags has no
// STARTF_USESTDHANDLES → the CRT falls back to GetStdHandle) and set cb.
fn get_startup_info(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_bytes(p, &[0u8; 68]);
        let _ = ctx.memory.write_u32(p, 68); // cb = sizeof(STARTUPINFO)
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
    // Frequency is reported as 1 MHz, so the counter is in microseconds.
    let p = ctx.arg(0);
    if p != 0 {
        let micros = crate::winapi::winmm::tick_ms() as u64 * 1000;
        let _ = ctx.memory.write_u32(p, micros as u32);
        let _ = ctx.memory.write_u32(p + 4, (micros >> 32) as u32);
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn get_tick_count(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(crate::winapi::winmm::tick_ms(), 0);
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

// GetConsoleScreenBufferInfo(hConsole, lpInfo): fill a CONSOLE_SCREEN_BUFFER_INFO
// so console apps that query the buffer geometry (cmd.exe) proceed.
// Layout: dwSize(COORD)@0, dwCursorPosition(COORD)@4, wAttributes(WORD)@8,
//         srWindow(SMALL_RECT)@10, dwMaximumWindowSize(COORD)@18.
// GetVersionEx(lpVersionInfo): fill OSVERSIONINFO for Windows XP (5.1.2600).
// dwOSVersionInfoSize@0, dwMajorVersion@4, dwMinorVersion@8, dwBuildNumber@12,
// dwPlatformId@16 (VER_PLATFORM_WIN32_NT=2), szCSDVersion@20.
fn get_version_ex(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_u32(p + 4, 5);    // major
        let _ = ctx.memory.write_u32(p + 8, 1);    // minor
        let _ = ctx.memory.write_u32(p + 12, 2600); // build
        let _ = ctx.memory.write_u32(p + 16, 2);   // VER_PLATFORM_WIN32_NT
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn get_console_screen_buffer_info(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(1);
    if p != 0 {
        let cols: u16 = 80;
        let rows: u16 = 25;
        let coord = |x: u16, y: u16| (x as u32) | ((y as u32) << 16);
        let _ = ctx.memory.write_u32(p, coord(cols, rows));      // dwSize
        let _ = ctx.memory.write_u32(p + 4, coord(0, 0));        // dwCursorPosition
        let _ = ctx.memory.write_u16(p + 8, 0x07);               // wAttributes (gray on black)
        // srWindow: Left=0, Top=0, Right=cols-1, Bottom=rows-1 (i16 each)
        let _ = ctx.memory.write_u16(p + 10, 0);
        let _ = ctx.memory.write_u16(p + 12, 0);
        let _ = ctx.memory.write_u16(p + 14, cols - 1);
        let _ = ctx.memory.write_u16(p + 16, rows - 1);
        let _ = ctx.memory.write_u32(p + 18, coord(cols, rows)); // dwMaximumWindowSize
    }
    ctx.ret_stdcall(1, 2); // success
    Handled::Ok
}

// GetConsoleMode(hConsole, lpMode): succeed with a standard console mode so
// console apps (cmd.exe) believe they are attached to a real console.
fn get_console_mode(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let lp_mode = ctx.arg(1);
    // Input handle gets input flags; output handles get output flags.
    let mode = if handle == STD_INPUT_HANDLE {
        0x1F7 // ENABLE_PROCESSED/LINE/ECHO/MOUSE/INSERT/QUICKEDIT/EXTENDED
    } else {
        0x3 // ENABLE_PROCESSED_OUTPUT | ENABLE_WRAP_AT_EOL_OUTPUT
    };
    if lp_mode != 0 {
        let _ = ctx.memory.write_u32(lp_mode, mode);
    }
    ctx.ret_stdcall(1, 2); // success
    Handled::Ok
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
    let code = ctx.arg(0);
    ctx.logs.log(crate::logs::LogLevel::Warn, "api",
        &format!("RaiseException code=0x{code:08X}"), Some(ctx.pid));
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
    r1_4 => (1, 4), r1_5 => (1, 5), r1_6 => (1, 6), r1_7 => (1, 7),
}

fn stub_invalid_handle(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(INVALID_HANDLE, 7);
    Handled::Ok
}
