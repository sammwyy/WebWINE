use super::{ApiContext, Handled, WinApiRegistry};

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
    let size = ctx.arg(3);
    let ptr = ctx.heap_alloc(size);
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
