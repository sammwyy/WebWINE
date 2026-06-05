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
        ("ntdll.dll", "NtQueryInformationProcess", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("ntdll.dll", "RtlExitUserProcess", |c| Handled::ExitProcess(c.arg(0))),
        ("ntdll.dll", "RtlExitUserThread", |c| Handled::ExitProcess(c.arg(0))),
        ("ntdll.dll", "NtWriteFile",      nt_write_file),
        ("ntdll.dll", "NtReadFile",       |c| { c.ret_stdcall(0xC000_0001u32, 9); Handled::Ok }),
        ("ntdll.dll", "NtCreateFile",     |c| { c.ret_stdcall(0xC000_0001u32, 11); Handled::Ok }),
        ("ntdll.dll", "NtDeviceIoControlFile", |c| { c.ret_stdcall(0, 10); Handled::Ok }),
        ("ntdll.dll", "NtQueryVolumeInformationFile", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("ntdll.dll", "RtlGetLastWin32Error", |c| { let e = c.cpu.last_error; c.ret_stdcall(e, 0); Handled::Ok }),
        ("ntdll.dll", "RtlSetLastWin32Error", |c| { c.cpu.last_error = c.arg(0); c.ret_stdcall(0, 1); Handled::Ok }),
        ("ntdll.dll", "RtlNtStatusToDosError", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
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

fn rtl_size(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0, 3); Handled::Ok
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
