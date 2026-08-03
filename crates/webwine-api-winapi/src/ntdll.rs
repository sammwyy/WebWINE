use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::vm::handles::KernelObject;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("ntdll.dll", "RtlAllocateHeap",  rtl_alloc),
        ("ntdll.dll", "RtlFreeHeap",      rtl_free),
        ("ntdll.dll", "RtlReAllocateHeap",rtl_realloc),
        ("ntdll.dll", "RtlSizeHeap",      rtl_size),
        ("ntdll.dll", "RtlZeroMemory",    rtl_zero),
        ("ntdll.dll", "RtlFillMemory",    rtl_fill),
        ("ntdll.dll", "RtlMoveMemory",    rtl_move),
        ("ntdll.dll", "RtlCopyMemory",    rtl_move),
        ("ntdll.dll", "NtClose",          nt_close),
        ("ntdll.dll", "NtTerminateProcess", nt_terminate),
        ("ntdll.dll", "RtlUnwind",        stub_void_0),
        ("ntdll.dll", "RtlEnterCriticalSection",  stub_void_1),
        ("ntdll.dll", "RtlLeaveCriticalSection",  stub_void_1),
        ("ntdll.dll", "RtlInitializeCriticalSection", stub_void_1),
        ("ntdll.dll", "NtQueryInformationProcess", nt_query_information_process),
        ("ntdll.dll", "RtlExitUserProcess", |c| Handled::ExitProcess(c.arg(0))),
        ("ntdll.dll", "RtlExitUserThread", |c| Handled::ExitProcess(c.arg(0))),
        ("ntdll.dll", "NtWriteFile",      nt_write_file),
        ("ntdll.dll", "NtReadFile",       |c| { c.ret_stdcall(0xC000_0001u32, 9); Handled::Ok }),
        ("ntdll.dll", "NtCreateFile",     |c| { c.ret_stdcall(0xC000_0001u32, 11); Handled::Ok }),
        ("ntdll.dll", "NtDeviceIoControlFile", |c| { c.ret_stdcall(0, 10); Handled::Ok }),
        ("ntdll.dll", "NtQueryVolumeInformationFile", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("ntdll.dll", "RtlGetLastWin32Error", |c| { let e = c.cpu.last_error; c.ret_stdcall(e, 0); Handled::Ok }),
        ("ntdll.dll", "RtlSetLastWin32Error", |c| { c.cpu.last_error = c.arg(0); c.ret_stdcall(0, 1); Handled::Ok }),
        ("ntdll.dll", "RtlNtStatusToDosError", rtl_nt_status_to_dos_error),
        ("ntdll.dll", "RtlIsStateSeparationEnabled", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("ntdll.dll", "RtlDllShutdownInProgress", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("ntdll.dll", "RtlCreateUnicodeStringFromAsciiz", rtl_create_unicode_from_ascii),
        ("ntdll.dll", "RtlFreeUnicodeString", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
    ];
    for &(dll, name, f) in fns { r.add(dll, name, f); }
}

fn rtl_alloc(ctx: &mut ApiContext) -> Handled {
    let size = ctx.arg(2);
    let ptr = ctx.heap_alloc(size);
    ctx.ret_stdcall(ptr, 3); Handled::Ok
}

fn rtl_free(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(1, 3); Handled::Ok
}

fn rtl_realloc(ctx: &mut ApiContext) -> Handled {
    let old = ctx.arg(2);
    let size = ctx.arg(3);
    let ptr = ctx.heap_realloc(old, size);
    ctx.ret_stdcall(ptr, 4); Handled::Ok
}

/// RtlSizeHeap(heap, flags, ptr) -> block size, (SIZE_T)-1 when unknown.
/// The bump allocator records every block size; reporting 0 read back as a
/// valid zero-byte block, which breaks msvcrt `_msize`/`realloc` paths.
fn rtl_size(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(2);
    let n = ctx.heap_sizes.get(&p).copied().unwrap_or(0xFFFF_FFFF);
    ctx.ret_stdcall(n, 3); Handled::Ok
}

fn rtl_zero(ctx: &mut ApiContext) -> Handled {
    let ptr = ctx.arg(0);
    let n   = ctx.arg(1) as usize;
    let _ = ctx.memory.write_bytes(ptr, &vec![0u8; n]);
    ctx.ret_stdcall(0, 2); Handled::Ok
}

fn rtl_fill(ctx: &mut ApiContext) -> Handled {
    let ptr = ctx.arg(0);
    let n   = ctx.arg(1) as usize;
    let val = ctx.arg(2) as u8;
    let _ = ctx.memory.write_bytes(ptr, &vec![val; n]);
    ctx.ret_stdcall(0, 3); Handled::Ok
}

fn rtl_move(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let n   = ctx.arg(2) as usize;
    if let Ok(bytes) = ctx.memory.read_bytes(src, n) {
        let _ = ctx.memory.write_bytes(dst, &bytes);
    }
    ctx.ret_stdcall(0, 3); Handled::Ok
}

// NtWriteFile(FileHandle, Event, ApcRoutine, ApcContext, IoStatusBlock,
//             Buffer, Length, ByteOffset, Key) â€” 9 args, stdcall.
// std's File::write goes through here too (not just the console path), so a
// VFS-backed handle must write to the file. Anything else routes to the
// process console (the UCRT/std stdout/stderr path).
fn nt_write_file(ctx: &mut ApiContext) -> Handled {
    let handle  = ctx.arg(0);
    let iosb    = ctx.arg(4);
    let buffer  = ctx.arg(5);
    let length  = ctx.arg(6);
    let byte_off = ctx.arg(7);
    let bytes = ctx.memory.read_bytes(buffer, length as usize).unwrap_or_default();

    // VFS-backed file handle? Write into the file at the requested offset (or
    // the handle's current cursor when ByteOffset is NULL/special).
    let target = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, cursor, .. }) => Some((path.clone(), *cursor)),
        _ => None,
    };

    if let Some((path, cursor)) = target {
        // ByteOffset is a pointer to a LARGE_INTEGER. A NULL pointer or the
        // FILE_USE_FILE_POINTER_POSITION sentinel (-1/-2) means "current pos".
        let offset = if byte_off != 0 {
            match ctx.memory.read_u32(byte_off) {
                Ok(lo) => {
                    let hi = ctx.memory.read_u32(byte_off + 4).unwrap_or(0);
                    let v = ((hi as u64) << 32) | lo as u64;
                    if v >= 0xFFFF_FFFF_FFFF_FFFE { cursor } else { v }
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
        // STD_ERROR magic handle.
        ctx.console.stderr.extend_from_slice(&bytes);
    } else {
        ctx.console.stdout.extend_from_slice(&bytes);
    }

    // IO_STATUS_BLOCK { Status: u32; Information: u32 }
    if iosb != 0 {
        let _ = ctx.memory.write_u32(iosb, 0);              // STATUS_SUCCESS
        let _ = ctx.memory.write_u32(iosb + 4, length);     // bytes written
    }
    ctx.ret_stdcall(0, 9); // STATUS_SUCCESS
    Handled::Ok
}

fn nt_close(ctx: &mut ApiContext) -> Handled {
    let h = ctx.arg(0);
    ctx.handles.remove(h);
    ctx.ret_stdcall(0, 1); Handled::Ok
}

fn nt_terminate(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(ctx.arg(1))
}

fn stub_void_0(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 0); Handled::Ok }
fn stub_void_1(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 1); Handled::Ok }

/// RtlCreateUnicodeStringFromAsciiz(DestinationString, SourceString)
/// Allocates a UNICODE_STRING (Length u16, MaximumLength u16, Buffer ptr) on the
/// process heap, converts the ASCII source to UTF-16, and writes the struct.
/// Returns TRUE (1) on success, FALSE (0) on failure.
fn rtl_create_unicode_from_ascii(ctx: &mut ApiContext) -> Handled {
    let dest   = ctx.arg(0); // PUNICODE_STRING
    let source = ctx.arg(1); // PCSZ (const char*)

    if dest == 0 {
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    }

    let s = ctx.memory.read_cstr(source);
    let encoded: Vec<u8> = s.encode_utf16()
        .chain(std::iter::once(0u16)) // null terminator
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let byte_len = (encoded.len() - 2) as u16; // length without null
    let max_len  = encoded.len() as u16;

    let buf = ctx.heap_alloc(encoded.len() as u32);
    let _ = ctx.memory.write_bytes(buf, &encoded);

    // UNICODE_STRING layout: Length(u16), MaximumLength(u16), Buffer(u32)
    let _ = ctx.memory.write_u16(dest,     byte_len);
    let _ = ctx.memory.write_u16(dest + 2, max_len);
    let _ = ctx.memory.write_u32(dest + 4, buf);

    ctx.ret_stdcall(1, 2); // TRUE, 2 args
    Handled::Ok
}

// PROCESSINFOCLASS values we answer (include/winternl.h).
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

/// NtQueryInformationProcess(handle, class, buf, len, retlen).
///
/// The old stub returned STATUS_SUCCESS without touching the output buffer, so
/// every caller read uninitialised stack as the answer - anti-debug checks in
/// particular saw a random non-zero ProcessDebugPort and bailed out. Fill the
/// classes we can answer and report STATUS_INVALID_INFO_CLASS for the rest,
/// which callers are written to handle.
fn nt_query_information_process(ctx: &mut ApiContext) -> Handled {
    let class = ctx.arg(1);
    let buf = ctx.arg(2);
    let len = ctx.arg(3);
    let ret_len = ctx.arg(4);

    // (required length, writer). None => class not supported.
    let status = match class {
        PROCESS_BASIC_INFORMATION => {
            // PROCESS_BASIC_INFORMATION on x86: ExitStatus, PebBaseAddress,
            // AffinityMask, BasePriority, UniqueProcessId, InheritedFromPid.
            if len < 24 {
                STATUS_INFO_LENGTH_MISMATCH
            } else {
                let pid = ctx.pid;
                let _ = ctx.memory.write_u32(buf, 0x103); // STILL_ACTIVE
                let _ = ctx.memory.write_u32(buf + 4, 0x7FFD_F000); // PEB
                let _ = ctx.memory.write_u32(buf + 8, 1); // affinity mask
                let _ = ctx.memory.write_u32(buf + 12, 8); // base priority
                let _ = ctx.memory.write_u32(buf + 16, pid);
                let _ = ctx.memory.write_u32(buf + 20, 0);
                if ret_len != 0 {
                    let _ = ctx.memory.write_u32(ret_len, 24);
                }
                STATUS_SUCCESS
            }
        }
        // No debugger is attached: port 0, no debug object, default flags.
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

/// RtlNtStatusToDosError(status): map an NTSTATUS to its Win32 error.
///
/// Returning 0 (success) for everything meant a caller that checked
/// GetLastError() after a failed Nt* call saw "no error" and carried on with an
/// invalid handle. Covers the statuses our own Nt* stubs and the loader
/// actually produce; anything else falls back to ERROR_MR_MID_NOT_FOUND, the
/// same catch-all Wine uses for an unmapped status.
fn rtl_nt_status_to_dos_error(ctx: &mut ApiContext) -> Handled {
    let err = match ctx.arg(0) {
        0x0000_0000 => 0,             // STATUS_SUCCESS            -> NO_ERROR
        0x0000_0102 => 258,           // STATUS_TIMEOUT            -> WAIT_TIMEOUT
        0x0000_0103 => 259,           // STATUS_PENDING            -> ERROR_NO_MORE_ITEMS
        0x8000_000D => 234,           // STATUS_PARTIAL_COPY       -> ERROR_MORE_DATA
        0x8000_0005 => 234,           // STATUS_BUFFER_OVERFLOW    -> ERROR_MORE_DATA
        0x8000_0006 => 18,            // STATUS_NO_MORE_FILES      -> ERROR_NO_MORE_FILES
        0xC000_0001 => 31,            // STATUS_UNSUCCESSFUL       -> ERROR_GEN_FAILURE
        0xC000_0002 => 1,             // STATUS_NOT_IMPLEMENTED    -> ERROR_INVALID_FUNCTION
        0xC000_0003 => 87,            // STATUS_INVALID_INFO_CLASS -> ERROR_INVALID_PARAMETER
        0xC000_0004 => 24,            // STATUS_INFO_LENGTH_MISMATCH -> ERROR_BAD_LENGTH
        0xC000_0005 => 998,           // STATUS_ACCESS_VIOLATION   -> ERROR_NOACCESS
        0xC000_0008 => 6,             // STATUS_INVALID_HANDLE     -> ERROR_INVALID_HANDLE
        0xC000_000D => 87,            // STATUS_INVALID_PARAMETER  -> ERROR_INVALID_PARAMETER
        0xC000_000F => 2,             // STATUS_NO_SUCH_FILE       -> ERROR_FILE_NOT_FOUND
        0xC000_0011 => 38,            // STATUS_END_OF_FILE        -> ERROR_HANDLE_EOF
        0xC000_0017 => 8,             // STATUS_NO_MEMORY          -> ERROR_NOT_ENOUGH_MEMORY
        0xC000_0022 => 5,             // STATUS_ACCESS_DENIED      -> ERROR_ACCESS_DENIED
        0xC000_0023 => 122,           // STATUS_BUFFER_TOO_SMALL   -> ERROR_INSUFFICIENT_BUFFER
        0xC000_0034 => 2,             // STATUS_OBJECT_NAME_NOT_FOUND -> ERROR_FILE_NOT_FOUND
        0xC000_0035 => 183,           // STATUS_OBJECT_NAME_COLLISION -> ERROR_ALREADY_EXISTS
        0xC000_003A => 3,             // STATUS_OBJECT_PATH_NOT_FOUND -> ERROR_PATH_NOT_FOUND
        0xC000_007B => 193,           // STATUS_INVALID_IMAGE_FORMAT -> ERROR_BAD_EXE_FORMAT
        0xC000_00BB => 50,            // STATUS_NOT_SUPPORTED      -> ERROR_NOT_SUPPORTED
        0xC000_0100 => 203,           // STATUS_VARIABLE_NOT_FOUND -> ERROR_ENVVAR_NOT_FOUND
        0xC000_0103 => 267,           // STATUS_NOT_A_DIRECTORY    -> ERROR_DIRECTORY
        0xC000_0135 => 126,           // STATUS_DLL_NOT_FOUND      -> ERROR_MOD_NOT_FOUND
        0xC000_0139 => 127,           // STATUS_ENTRYPOINT_NOT_FOUND -> ERROR_PROC_NOT_FOUND
        0xC000_0353 => 6,             // STATUS_PORT_NOT_SET       -> ERROR_INVALID_HANDLE
        s if s & 0xC000_0000 == 0 => 0, // any other success/informational status
        _ => 317,                     // ERROR_MR_MID_NOT_FOUND
    };
    ctx.ret_stdcall(err, 1);
    Handled::Ok
}
