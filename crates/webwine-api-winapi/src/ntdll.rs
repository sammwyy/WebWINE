//! ntdll.dll — NT runtime helpers (heap, process info, critical sections).

use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::vm::handles::KernelObject;

// RTL_CRITICAL_SECTION (x86) field offsets.
const CS_DEBUG_INFO: u32 = 0;
const CS_LOCK_COUNT: u32 = 4;
const CS_RECURSION: u32 = 8;
const CS_OWNING_THREAD: u32 = 12;
const CS_LOCK_SEMAPHORE: u32 = 16;
const CS_SPIN_COUNT: u32 = 20;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("ntdll.dll", "RtlAllocateHeap", rtl_alloc),
        ("ntdll.dll", "RtlFreeHeap", rtl_free),
        ("ntdll.dll", "RtlReAllocateHeap", rtl_realloc),
        ("ntdll.dll", "RtlSizeHeap", rtl_size),
        ("ntdll.dll", "RtlZeroMemory", rtl_zero),
        ("ntdll.dll", "RtlFillMemory", rtl_fill),
        ("ntdll.dll", "RtlMoveMemory", rtl_move),
        ("ntdll.dll", "RtlCopyMemory", rtl_move),
        ("ntdll.dll", "NtClose", nt_close),
        ("ntdll.dll", "NtTerminateProcess", nt_terminate),
        ("ntdll.dll", "RtlUnwind", rtl_unwind),
        ("ntdll.dll", "RtlEnterCriticalSection", rtl_enter_critical_section),
        ("ntdll.dll", "RtlLeaveCriticalSection", rtl_leave_critical_section),
        (
            "ntdll.dll",
            "RtlInitializeCriticalSection",
            rtl_initialize_critical_section,
        ),
        (
            "ntdll.dll",
            "RtlInitializeCriticalSectionAndSpinCount",
            rtl_initialize_critical_section_and_spin,
        ),
        (
            "ntdll.dll",
            "RtlDeleteCriticalSection",
            rtl_delete_critical_section,
        ),
        (
            "ntdll.dll",
            "RtlTryEnterCriticalSection",
            rtl_try_enter_critical_section,
        ),
        (
            "ntdll.dll",
            "NtQueryInformationProcess",
            nt_query_information_process,
        ),
        ("ntdll.dll", "RtlExitUserProcess", |c| Handled::ExitProcess(c.arg(0))),
        ("ntdll.dll", "RtlExitUserThread", |c| Handled::ExitProcess(c.arg(0))),
        ("ntdll.dll", "NtWriteFile", nt_write_file),
        ("ntdll.dll", "NtReadFile", nt_read_file),
        ("ntdll.dll", "NtCreateFile", nt_create_file),
        ("ntdll.dll", "NtDeviceIoControlFile", nt_device_io_control_file),
        (
            "ntdll.dll",
            "NtQueryVolumeInformationFile",
            nt_query_volume_information_file,
        ),
        ("ntdll.dll", "RtlGetLastWin32Error", |c| {
            let e = c.cpu.last_error;
            c.ret_stdcall(e, 0);
            Handled::Ok
        }),
        ("ntdll.dll", "RtlSetLastWin32Error", |c| {
            c.cpu.last_error = c.arg(0);
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ntdll.dll", "RtlNtStatusToDosError", rtl_nt_status_to_dos_error),
        ("ntdll.dll", "RtlIsStateSeparationEnabled", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("ntdll.dll", "RtlDllShutdownInProgress", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        (
            "ntdll.dll",
            "RtlCreateUnicodeStringFromAsciiz",
            rtl_create_unicode_from_ascii,
        ),
        ("ntdll.dll", "RtlFreeUnicodeString", rtl_free_unicode_string),
        ("ntdll.dll", "NtQuerySystemInformation", nt_query_system_information),
        ("ntdll.dll", "RtlGetVersion", rtl_get_version),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn rtl_alloc(ctx: &mut ApiContext) -> Handled {
    // RtlAllocateHeap(HeapHandle, Flags, Size)
    const HEAP_ZERO_MEMORY: u32 = 0x0000_0008;
    let flags = ctx.arg(1);
    let size = ctx.arg(2);
    let ptr = if flags & HEAP_ZERO_MEMORY != 0 {
        ctx.heap_alloc_zeroed(size)
    } else {
        ctx.heap_alloc(size)
    };
    ctx.ret_stdcall(ptr, 3);
    Handled::Ok
}

fn rtl_free(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(2);
    if p != 0 {
        ctx.heap_sizes.remove(&p);
    }
    ctx.ret_stdcall(1, 3); // TRUE
    Handled::Ok
}

fn rtl_realloc(ctx: &mut ApiContext) -> Handled {
    let old = ctx.arg(2);
    let size = ctx.arg(3);
    let ptr = ctx.heap_realloc(old, size);
    ctx.ret_stdcall(ptr, 4);
    Handled::Ok
}

fn rtl_size(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(2);
    let n = ctx.heap_sizes.get(&p).copied().unwrap_or(0xFFFF_FFFF);
    ctx.ret_stdcall(n, 3);
    Handled::Ok
}

fn rtl_zero(ctx: &mut ApiContext) -> Handled {
    let ptr = ctx.arg(0);
    let n = ctx.arg(1) as usize;
    let _ = ctx.memory.write_bytes(ptr, &vec![0u8; n]);
    ctx.ret_stdcall(0, 2);
    Handled::Ok
}

fn rtl_fill(ctx: &mut ApiContext) -> Handled {
    let ptr = ctx.arg(0);
    let n = ctx.arg(1) as usize;
    let val = ctx.arg(2) as u8;
    let _ = ctx.memory.write_bytes(ptr, &vec![val; n]);
    ctx.ret_stdcall(0, 3);
    Handled::Ok
}

fn rtl_move(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let n = ctx.arg(2) as usize;
    if let Ok(bytes) = ctx.memory.read_bytes(src, n) {
        let _ = ctx.memory.write_bytes(dst, &bytes);
    }
    ctx.ret_stdcall(0, 3);
    Handled::Ok
}

/// RtlUnwind(TargetFrame, TargetIp, ExceptionRecord, ReturnValue) — no SEH frames.
fn rtl_unwind(ctx: &mut ApiContext) -> Handled {
    // Wine walks the frame chain; we have no registered frames so this is a no-op
    // that preserves the stdcall ABI (actually RtlUnwind is stdcall with 4 args
    // but never returns to the caller in the success path — it longjmps). For
    // guest code that only registers it as an import and never hits SEH, return.
    ctx.ret_stdcall(0, 4);
    Handled::Ok
}

/// RtlInitializeCriticalSection(cs): zero the RTL_CRITICAL_SECTION fields.
fn rtl_initialize_critical_section(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    if cs != 0 {
        init_critical_section(ctx, cs, 0);
    }
    ctx.ret_stdcall(0, 1); // STATUS_SUCCESS
    Handled::Ok
}

fn rtl_initialize_critical_section_and_spin(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    let spin = ctx.arg(1);
    if cs != 0 {
        init_critical_section(ctx, cs, spin);
    }
    ctx.ret_stdcall(0, 2);
    Handled::Ok
}

fn init_critical_section(ctx: &mut ApiContext, cs: u32, spin: u32) {
    let _ = ctx.memory.write_u32(cs + CS_DEBUG_INFO, 0xFFFF_FFFF); // no debug info
    let _ = ctx.memory.write_u32(cs + CS_LOCK_COUNT, 0xFFFF_FFFF); // unlocked
    let _ = ctx.memory.write_u32(cs + CS_RECURSION, 0);
    let _ = ctx.memory.write_u32(cs + CS_OWNING_THREAD, 0);
    let _ = ctx.memory.write_u32(cs + CS_LOCK_SEMAPHORE, 0);
    let _ = ctx.memory.write_u32(cs + CS_SPIN_COUNT, spin);
}

fn rtl_delete_critical_section(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    if cs != 0 {
        let _ = ctx.memory.write_bytes(cs, &[0u8; 24]);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

/// Single-threaded guest: enter always succeeds and marks the section owned.
fn rtl_enter_critical_section(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    if cs != 0 {
        let rec = ctx.memory.read_u32(cs + CS_RECURSION).unwrap_or(0);
        let _ = ctx.memory.write_u32(cs + CS_LOCK_COUNT, 0);
        let _ = ctx.memory.write_u32(cs + CS_RECURSION, rec.wrapping_add(1));
        let _ = ctx.memory.write_u32(cs + CS_OWNING_THREAD, 1);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn rtl_leave_critical_section(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    if cs != 0 {
        let rec = ctx.memory.read_u32(cs + CS_RECURSION).unwrap_or(1);
        let next = rec.saturating_sub(1);
        let _ = ctx.memory.write_u32(cs + CS_RECURSION, next);
        if next == 0 {
            let _ = ctx.memory.write_u32(cs + CS_OWNING_THREAD, 0);
            let _ = ctx.memory.write_u32(cs + CS_LOCK_COUNT, 0xFFFF_FFFF);
        }
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn rtl_try_enter_critical_section(ctx: &mut ApiContext) -> Handled {
    // Always succeed in the single-threaded model.
    let cs = ctx.arg(0);
    if cs != 0 {
        let rec = ctx.memory.read_u32(cs + CS_RECURSION).unwrap_or(0);
        let _ = ctx.memory.write_u32(cs + CS_LOCK_COUNT, 0);
        let _ = ctx.memory.write_u32(cs + CS_RECURSION, rec.wrapping_add(1));
        let _ = ctx.memory.write_u32(cs + CS_OWNING_THREAD, 1);
    }
    ctx.ret_stdcall(1, 1); // TRUE
    Handled::Ok
}

fn rtl_free_unicode_string(ctx: &mut ApiContext) -> Handled {
    // RtlFreeUnicodeString(UNICODE_STRING*): free Buffer if non-null.
    let us = ctx.arg(0);
    if us != 0 {
        let buf = ctx.memory.read_u32(us + 4).unwrap_or(0);
        if buf != 0 {
            ctx.heap_sizes.remove(&buf);
        }
        let _ = ctx.memory.write_u16(us, 0);
        let _ = ctx.memory.write_u16(us + 2, 0);
        let _ = ctx.memory.write_u32(us + 4, 0);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn nt_read_file(ctx: &mut ApiContext) -> Handled {
    // NtReadFile(FileHandle, Event, ApcRoutine, ApcContext, IoStatusBlock,
    //            Buffer, Length, ByteOffset, Key) — 9 args.
    let handle = ctx.arg(0);
    let iosb = ctx.arg(4);
    let buffer = ctx.arg(5);
    let length = ctx.arg(6) as usize;
    let byte_off = ctx.arg(7);

    let status;
    let mut info = 0u32;

    match ctx.handles.get(handle).cloned() {
        Some(KernelObject::VfsFile { path, cursor, .. }) => {
            let offset = if byte_off != 0 {
                match ctx.memory.read_u32(byte_off) {
                    Ok(lo) => {
                        let hi = ctx.memory.read_u32(byte_off + 4).unwrap_or(0);
                        let v = ((hi as u64) << 32) | lo as u64;
                        if v >= 0xFFFF_FFFF_FFFF_FFFE {
                            cursor
                        } else {
                            v
                        }
                    }
                    Err(_) => cursor,
                }
            } else {
                cursor
            };
            match ctx.fs.read_file(&path) {
                Ok(content) => {
                    let start = offset as usize;
                    if start >= content.len() {
                        status = 0xC000_0011; // STATUS_END_OF_FILE
                        info = 0;
                    } else {
                        let end = (start + length).min(content.len());
                        let slice = &content[start..end];
                        let _ = ctx.memory.write_bytes(buffer, slice);
                        info = slice.len() as u32;
                        if let Some(KernelObject::VfsFile { cursor, .. }) =
                            ctx.handles.get_mut(handle)
                        {
                            *cursor = end as u64;
                        }
                        status = 0; // STATUS_SUCCESS
                    }
                }
                Err(_) => {
                    status = 0xC000_000F; // STATUS_NO_SUCH_FILE
                }
            }
        }
        Some(KernelObject::ConsoleInput(_)) => {
            // No stdin data → EOF.
            status = 0xC000_0011;
        }
        _ => {
            status = 0xC000_0008; // STATUS_INVALID_HANDLE
        }
    }

    if iosb != 0 {
        let _ = ctx.memory.write_u32(iosb, status);
        let _ = ctx.memory.write_u32(iosb + 4, info);
    }
    ctx.ret_stdcall(status, 9);
    Handled::Ok
}

fn nt_create_file(ctx: &mut ApiContext) -> Handled {
    // Full NtCreateFile is large; report not implemented so callers fall back
    // to kernel32 CreateFile.
    let file_handle = ctx.arg(0);
    if file_handle != 0 {
        let _ = ctx.memory.write_u32(file_handle, 0);
    }
    ctx.ret_stdcall(0xC000_0002, 11); // STATUS_NOT_IMPLEMENTED
    Handled::Ok
}

fn nt_device_io_control_file(ctx: &mut ApiContext) -> Handled {
    let iosb = ctx.arg(4);
    if iosb != 0 {
        let _ = ctx.memory.write_u32(iosb, 0);
        let _ = ctx.memory.write_u32(iosb + 4, 0);
    }
    ctx.ret_stdcall(0, 10);
    Handled::Ok
}

fn nt_query_volume_information_file(ctx: &mut ApiContext) -> Handled {
    let iosb = ctx.arg(1);
    if iosb != 0 {
        let _ = ctx.memory.write_u32(iosb, 0);
        let _ = ctx.memory.write_u32(iosb + 4, 0);
    }
    ctx.ret_stdcall(0, 5);
    Handled::Ok
}

fn nt_query_system_information(ctx: &mut ApiContext) -> Handled {
    // NtQuerySystemInformation(class, buf, len, retlen) — unsupported class.
    let ret_len = ctx.arg(3);
    if ret_len != 0 {
        let _ = ctx.memory.write_u32(ret_len, 0);
    }
    ctx.ret_stdcall(0xC000_0003, 4); // STATUS_INVALID_INFO_CLASS
    Handled::Ok
}

fn rtl_get_version(ctx: &mut ApiContext) -> Handled {
    // RtlGetVersion(PRTL_OSVERSIONINFOW): fill like Win7 SP1.
    let info = ctx.arg(0);
    if info != 0 {
        let size = ctx.memory.read_u32(info).unwrap_or(0);
        if size >= 20 {
            let _ = ctx.memory.write_u32(info + 4, 6); // dwMajorVersion
            let _ = ctx.memory.write_u32(info + 8, 1); // dwMinorVersion
            let _ = ctx.memory.write_u32(info + 12, 7601); // dwBuildNumber
            let _ = ctx.memory.write_u32(info + 16, 2); // VER_PLATFORM_WIN32_NT
        }
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

// NtWriteFile(FileHandle, Event, ApcRoutine, ApcContext, IoStatusBlock,
//             Buffer, Length, ByteOffset, Key) — 9 args, stdcall.
fn nt_write_file(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let iosb = ctx.arg(4);
    let buffer = ctx.arg(5);
    let length = ctx.arg(6);
    let byte_off = ctx.arg(7);
    let bytes = ctx
        .memory
        .read_bytes(buffer, length as usize)
        .unwrap_or_default();

    let target = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, cursor, .. }) => Some((path.clone(), *cursor)),
        _ => None,
    };

    if let Some((path, cursor)) = target {
        let offset = if byte_off != 0 {
            match ctx.memory.read_u32(byte_off) {
                Ok(lo) => {
                    let hi = ctx.memory.read_u32(byte_off + 4).unwrap_or(0);
                    let v = ((hi as u64) << 32) | lo as u64;
                    if v >= 0xFFFF_FFFF_FFFF_FFFE {
                        cursor
                    } else {
                        v
                    }
                }
                Err(_) => cursor,
            }
        } else {
            cursor
        };

        let mut content = ctx.fs.read_file(&path).unwrap_or_default();
        let start = offset as usize;
        let end = start + bytes.len();
        if content.len() < end {
            content.resize(end, 0);
        }
        content[start..end].copy_from_slice(&bytes);
        let _ = ctx.fs.mount_file(&path, content);
        if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(handle) {
            *cursor = end as u64;
        }
    } else if handle == 0xFFFF_FFF4 {
        ctx.console.stderr.extend_from_slice(&bytes);
    } else {
        ctx.console.stdout.extend_from_slice(&bytes);
    }

    if iosb != 0 {
        let _ = ctx.memory.write_u32(iosb, 0);
        let _ = ctx.memory.write_u32(iosb + 4, length);
    }
    ctx.ret_stdcall(0, 9);
    Handled::Ok
}

fn nt_close(ctx: &mut ApiContext) -> Handled {
    let h = ctx.arg(0);
    ctx.handles.remove(h);
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn nt_terminate(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(ctx.arg(1))
}

fn rtl_create_unicode_from_ascii(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let source = ctx.arg(1);

    if dest == 0 {
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    }

    let s = ctx.memory.read_cstr(source);
    let encoded: Vec<u8> = s
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let byte_len = (encoded.len() - 2) as u16;
    let max_len = encoded.len() as u16;

    let buf = ctx.heap_alloc(encoded.len() as u32);
    let _ = ctx.memory.write_bytes(buf, &encoded);

    let _ = ctx.memory.write_u16(dest, byte_len);
    let _ = ctx.memory.write_u16(dest + 2, max_len);
    let _ = ctx.memory.write_u32(dest + 4, buf);

    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

const PROCESS_BASIC_INFORMATION: u32 = 0;
const PROCESS_DEBUG_PORT: u32 = 7;
const PROCESS_WOW64_INFORMATION: u32 = 26;
const PROCESS_DEBUG_OBJECT_HANDLE: u32 = 30;
const PROCESS_DEBUG_FLAGS: u32 = 31;
const PROCESS_EXECUTE_FLAGS: u32 = 34;

const STATUS_SUCCESS: u32 = 0;
const STATUS_INVALID_INFO_CLASS: u32 = 0xC000_0003;
const STATUS_INFO_LENGTH_MISMATCH: u32 = 0xC000_0004;
const STATUS_PORT_NOT_SET: u32 = 0xC000_0353;

fn nt_query_information_process(ctx: &mut ApiContext) -> Handled {
    let class = ctx.arg(1);
    let buf = ctx.arg(2);
    let len = ctx.arg(3);
    let ret_len = ctx.arg(4);

    let status = match class {
        PROCESS_BASIC_INFORMATION => {
            if len < 24 {
                STATUS_INFO_LENGTH_MISMATCH
            } else {
                let pid = ctx.pid;
                let _ = ctx.memory.write_u32(buf, 0x103);
                let _ = ctx.memory.write_u32(buf + 4, 0x7FFD_F000);
                let _ = ctx.memory.write_u32(buf + 8, 1);
                let _ = ctx.memory.write_u32(buf + 12, 8);
                let _ = ctx.memory.write_u32(buf + 16, pid);
                let _ = ctx.memory.write_u32(buf + 20, 0);
                if ret_len != 0 {
                    let _ = ctx.memory.write_u32(ret_len, 24);
                }
                STATUS_SUCCESS
            }
        }
        PROCESS_DEBUG_PORT | PROCESS_WOW64_INFORMATION => {
            if len < 4 {
                STATUS_INFO_LENGTH_MISMATCH
            } else {
                let _ = ctx.memory.write_u32(buf, 0);
                if ret_len != 0 {
                    let _ = ctx.memory.write_u32(ret_len, 4);
                }
                STATUS_SUCCESS
            }
        }
        PROCESS_DEBUG_FLAGS | PROCESS_EXECUTE_FLAGS => {
            if len < 4 {
                STATUS_INFO_LENGTH_MISMATCH
            } else {
                let _ = ctx.memory.write_u32(buf, 1);
                if ret_len != 0 {
                    let _ = ctx.memory.write_u32(ret_len, 4);
                }
                STATUS_SUCCESS
            }
        }
        PROCESS_DEBUG_OBJECT_HANDLE => {
            if len >= 4 {
                let _ = ctx.memory.write_u32(buf, 0);
            }
            STATUS_PORT_NOT_SET
        }
        _ => STATUS_INVALID_INFO_CLASS,
    };

    ctx.ret_stdcall(status, 5);
    Handled::Ok
}

fn rtl_nt_status_to_dos_error(ctx: &mut ApiContext) -> Handled {
    let err = match ctx.arg(0) {
        0x0000_0000 => 0,
        0x0000_0102 => 258,
        0x0000_0103 => 259,
        0x8000_000D => 234,
        0x8000_0005 => 234,
        0x8000_0006 => 18,
        0xC000_0001 => 31,
        0xC000_0002 => 1,
        0xC000_0003 => 87,
        0xC000_0004 => 24,
        0xC000_0005 => 998,
        0xC000_0008 => 6,
        0xC000_000D => 87,
        0xC000_000F => 2,
        0xC000_0011 => 38,
        0xC000_0017 => 8,
        0xC000_0022 => 5,
        0xC000_0023 => 122,
        0xC000_0034 => 2,
        0xC000_0035 => 183,
        0xC000_003A => 3,
        0xC000_007B => 193,
        0xC000_00BB => 50,
        0xC000_0100 => 203,
        0xC000_0103 => 267,
        0xC000_0135 => 126,
        0xC000_0139 => 127,
        0xC000_0353 => 6,
        s if s & 0xC000_0000 == 0 => 0,
        _ => 317,
    };
    ctx.ret_stdcall(err, 1);
    Handled::Ok
}
