use super::{ApiContext, Handled, WinApiRegistry};
use crate::vm::handles::{
    CURRENT_PROCESS, CURRENT_THREAD, INVALID_HANDLE, STD_ERROR_HANDLE, STD_INPUT_HANDLE,
    STD_OUTPUT_HANDLE,
};

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
        ("kernel32.dll", "HeapDestroy", stub_true_1),
        ("kernel32.dll", "VirtualAlloc", virtual_alloc),
        ("kernel32.dll", "VirtualFree", stub_true_1),
        ("kernel32.dll", "VirtualProtect", virtual_protect),
        ("kernel32.dll", "VirtualQuery", stub_zero_1),
        ("kernel32.dll", "GetModuleHandleA", get_module_handle_a),
        ("kernel32.dll", "GetModuleHandleW", get_module_handle_w),
        ("kernel32.dll", "GetModuleFileNameA", get_module_filename_a),
        ("kernel32.dll", "GetModuleFileNameW", stub_zero_1),
        ("kernel32.dll", "GetProcAddress", stub_zero_1),
        ("kernel32.dll", "LoadLibraryA", stub_zero_1),
        ("kernel32.dll", "LoadLibraryW", stub_zero_1),
        ("kernel32.dll", "FreeLibrary", stub_true_1),
        ("kernel32.dll", "IsDebuggerPresent", stub_zero_0),
        ("kernel32.dll", "GetCommandLineA", get_command_line_a),
        ("kernel32.dll", "GetCommandLineW", get_command_line_w),
        ("kernel32.dll", "GetStartupInfoA", get_startup_info_a),
        ("kernel32.dll", "GetStartupInfoW", stub_void_1),
        (
            "kernel32.dll",
            "GetCurrentProcessId",
            get_current_process_id,
        ),
        ("kernel32.dll", "GetCurrentThreadId", get_current_thread_id),
        ("kernel32.dll", "GetCurrentProcess", get_current_process),
        ("kernel32.dll", "GetCurrentThread", get_current_thread),
        ("kernel32.dll", "GetSystemInfo", stub_void_1),
        ("kernel32.dll", "GetSystemTimeAsFileTime", get_system_time),
        (
            "kernel32.dll",
            "QueryPerformanceCounter",
            query_perf_counter,
        ),
        ("kernel32.dll", "QueryPerformanceFrequency", query_perf_freq),
        ("kernel32.dll", "GetTickCount", stub_zero_0),
        ("kernel32.dll", "GetTickCount64", stub_zero_0),
        ("kernel32.dll", "FlushFileBuffers", stub_true_1),
        ("kernel32.dll", "SetFilePointer", stub_zero_1),
        ("kernel32.dll", "SetUnhandledExceptionFilter", stub_zero_1),
        ("kernel32.dll", "UnhandledExceptionFilter", stub_zero_1),
        ("kernel32.dll", "GetEnvironmentStringsW", stub_zero_0),
        ("kernel32.dll", "FreeEnvironmentStringsW", stub_true_1),
        ("kernel32.dll", "GetEnvironmentStringsA", stub_zero_0),
        ("kernel32.dll", "FreeEnvironmentStringsA", stub_true_1),
        ("kernel32.dll", "InitializeCriticalSection", stub_void_1),
        (
            "kernel32.dll",
            "InitializeCriticalSectionAndSpinCount",
            stub_true_2,
        ),
        ("kernel32.dll", "DeleteCriticalSection", stub_void_1),
        ("kernel32.dll", "EnterCriticalSection", stub_void_1),
        ("kernel32.dll", "LeaveCriticalSection", stub_void_1),
        ("kernel32.dll", "TryEnterCriticalSection", stub_true_1),
        ("kernel32.dll", "TlsAlloc", tls_alloc),
        ("kernel32.dll", "TlsSetValue", tls_set),
        ("kernel32.dll", "TlsGetValue", tls_get),
        ("kernel32.dll", "TlsFree", stub_true_1),
        ("kernel32.dll", "GetConsoleCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetConsoleOutputCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetConsoleMode", stub_false_2),
        ("kernel32.dll", "SetConsoleCtrlHandler", stub_true_2),
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
        ("kernel32.dll", "IsValidCodePage", stub_true_1),
        ("kernel32.dll", "GetCPInfo", stub_false_2),
        ("kernel32.dll", "LCMapStringW", stub_zero_1),
        ("kernel32.dll", "FindFirstFileA", |c| {
            c.ret_stdcall(INVALID_HANDLE, 2);
            Handled::Ok
        }),
        ("kernel32.dll", "FindClose", stub_false_1),
        ("kernel32.dll", "CreateFileA", stub_invalid_handle),
        ("kernel32.dll", "CreateFileW", stub_invalid_handle),
        ("kernel32.dll", "GetFileType", get_file_type),
        ("kernel32.dll", "SetHandleInformation", stub_true_2),
        ("kernel32.dll", "DuplicateHandle", dup_handle),
        ("kernel32.dll", "TerminateProcess", terminate_process),
        ("kernel32.dll", "RaiseException", raise_exception),
        ("kernel32.dll", "GetStringTypeW", stub_false_1),
        ("kernel32.dll", "FormatMessageA", stub_zero_1),
        ("kernel32.dll", "FormatMessageW", stub_zero_1),
        ("kernel32.dll", "OutputDebugStringA", output_debug_string_a),
        ("kernel32.dll", "OutputDebugStringW", stub_void_1),
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
        ("kernel32.dll", "Sleep", stub_void_1),
        ("kernel32.dll", "WaitForSingleObject", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }), // WAIT_OBJECT_0
        ("kernel32.dll", "CreateEventA", stub_zero_1),
        ("kernel32.dll", "CreateMutexA", stub_zero_1),
        ("kernel32.dll", "ReleaseMutex", stub_true_1),
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
    route_output(handle, &bytes, ctx);
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
    let buf = ctx.arg(1);
    let max = ctx.arg(2);
    let out = ctx.arg(3);
    // drain stdin
    let n = max.min(ctx.console.stdin.len() as u32) as usize;
    let data: Vec<u8> = ctx.console.stdin.drain(..n).collect();
    if !data.is_empty() {
        let _ = ctx.memory.write_bytes(buf, &data);
    }
    if out != 0 {
        let _ = ctx.memory.write_u32(out, n as u32);
    }
    ctx.ret_stdcall(if n > 0 { 1 } else { 0 }, 5);
    Handled::Ok
}

fn close_handle(ctx: &mut ApiContext) -> Handled {
    let h = ctx.arg(0);
    ctx.handles.remove(h);
    ctx.ret_stdcall(1, 1);
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

fn stub_zero_0(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 0);
    Handled::Ok
}
fn stub_zero_1(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1);
    Handled::Ok
}
fn stub_true_1(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}
fn stub_true_2(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 2);
    Handled::Ok
}
fn stub_false_1(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1);
    Handled::Ok
}
fn stub_false_2(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}
fn stub_void_1(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1);
    Handled::Ok
}
fn stub_void_2(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}
fn stub_invalid_handle(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(INVALID_HANDLE, 7);
    Handled::Ok
}
