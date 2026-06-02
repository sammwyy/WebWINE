use super::{ApiContext, Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("msvcrt.dll", "exit", exit),
        ("msvcrt.dll", "_exit", exit),
        ("msvcrt.dll", "_cexit", stub_void_0),
        ("msvcrt.dll", "malloc", malloc),
        ("msvcrt.dll", "_malloc_base", malloc),
        ("msvcrt.dll", "calloc", calloc),
        ("msvcrt.dll", "free", stub_void_1),
        ("msvcrt.dll", "_free_base", stub_void_1),
        ("msvcrt.dll", "realloc", realloc),
        ("msvcrt.dll", "memcpy", memcpy),
        ("msvcrt.dll", "memmove", memcpy),
        ("msvcrt.dll", "memcpy_s", memcpy_s),
        ("msvcrt.dll", "memset", memset),
        ("msvcrt.dll", "memcmp", memcmp),
        ("msvcrt.dll", "strlen", strlen),
        ("msvcrt.dll", "wcslen", wcslen),
        ("msvcrt.dll", "strcmp", strcmp),
        ("msvcrt.dll", "strncmp", strncmp),
        ("msvcrt.dll", "strcpy", strcpy),
        ("msvcrt.dll", "strcat", strcat),
        ("msvcrt.dll", "strchr", strchr),
        ("msvcrt.dll", "strrchr", strrchr),
        ("msvcrt.dll", "strstr", strstr),
        ("msvcrt.dll", "strtol", strtol),
        ("msvcrt.dll", "strtoul", strtoul),
        ("msvcrt.dll", "atoi", atoi),
        ("msvcrt.dll", "atof", stub_zero_cdecl_1),
        ("msvcrt.dll", "puts", puts),
        ("msvcrt.dll", "putchar", putchar),
        ("msvcrt.dll", "fflush", stub_zero_cdecl_1),
        ("msvcrt.dll", "printf", printf),
        ("msvcrt.dll", "fprintf", fprintf),
        ("msvcrt.dll", "sprintf", sprintf_fn),
        ("msvcrt.dll", "snprintf", snprintf_fn),
        ("msvcrt.dll", "_snprintf", snprintf_fn),
        ("msvcrt.dll", "_vsnprintf", stub_zero_cdecl_1),
        ("msvcrt.dll", "abort", |c| Handled::ExitProcess(3)),
        ("msvcrt.dll", "_initterm", initterm),
        ("msvcrt.dll", "_initterm_e", initterm_e),
        ("msvcrt.dll", "__p___argc", p_argc),
        ("msvcrt.dll", "__p___argv", p_argv),
        ("msvcrt.dll", "__p__environ", |c| {
            let v = 0x7FFD_F300u32;
            c.ret_cdecl(v);
            Handled::Ok
        }),
        ("msvcrt.dll", "_get_initial_narrow_environment", |c| {
            c.ret_cdecl(0);
            Handled::Ok
        }),
        ("msvcrt.dll", "_get_narrow_winmain_command_line", |c| {
            c.ret_cdecl(0);
            Handled::Ok
        }),
        ("msvcrt.dll", "__acrt_iob_func", acrt_iob),
        ("msvcrt.dll", "__stdio_common_vfprintf", stdio_vfprintf),
        ("msvcrt.dll", "__stdio_common_vfprintf_s", stdio_vfprintf),
        ("msvcrt.dll", "__stdio_common_vsprintf", stub_zero_cdecl_1),
        ("msvcrt.dll", "fwrite", fwrite),
        ("msvcrt.dll", "fputc", fputc),
        ("msvcrt.dll", "fputs", fputs),
        ("msvcrt.dll", "__set_app_type", stub_void_1_cdecl),
        ("msvcrt.dll", "_set_fmode", stub_zero_cdecl_1),
        ("msvcrt.dll", "_setmode", stub_zero_cdecl_1),
        ("msvcrt.dll", "_set_new_mode", stub_zero_cdecl_1),
        ("msvcrt.dll", "_configthreadlocale", stub_zero_cdecl_1),
        ("msvcrt.dll", "setlocale", stub_zero_cdecl_2),
        ("msvcrt.dll", "_wsetlocale", stub_zero_cdecl_2),
        ("msvcrt.dll", "__p__commode", p_commode),
        ("msvcrt.dll", "__p__fmode", p_fmode),
        ("msvcrt.dll", "_crt_atexit", stub_zero_cdecl_1),
        ("msvcrt.dll", "atexit", stub_zero_cdecl_1),
        ("msvcrt.dll", "_lock", stub_void_1_cdecl),
        ("msvcrt.dll", "_unlock", stub_void_1_cdecl),
        ("msvcrt.dll", "__lconv_init", stub_zero_cdecl_0),
        ("msvcrt.dll", "_controlfp", stub_zero_cdecl_2),
        ("msvcrt.dll", "_controlfp_s", stub_zero_cdecl_3),
        ("msvcrt.dll", "__current_exception", stub_zero_cdecl_0),
        (
            "msvcrt.dll",
            "__current_exception_context",
            stub_zero_cdecl_0,
        ),
        ("msvcrt.dll", "_except_handler4_common", stub_zero_cdecl_1),
        // ucrtbase / vcruntime aliases
        ("ucrtbase.dll", "exit", exit),
        ("ucrtbase.dll", "_exit", exit),
        ("ucrtbase.dll", "malloc", malloc),
        ("ucrtbase.dll", "free", stub_void_1),
        ("ucrtbase.dll", "printf", printf),
        ("ucrtbase.dll", "puts", puts),
        ("ucrtbase.dll", "__stdio_common_vfprintf", stdio_vfprintf),
        ("ucrtbase.dll", "_initterm", initterm),
        ("ucrtbase.dll", "_initterm_e", initterm_e),
        ("ucrtbase.dll", "__acrt_iob_func", acrt_iob),
        ("vcruntime140.dll", "memcpy", memcpy),
        ("vcruntime140.dll", "memset", memset),
        ("vcruntime140.dll", "memmove", memcpy),
        (
            "vcruntime140.dll",
            "__C_specific_handler",
            stub_zero_cdecl_1,
        ),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn exit(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(ctx.arg(0))
}

fn malloc(ctx: &mut ApiContext) -> Handled {
    let size = ctx.arg(0);
    let ptr = ctx.heap_alloc(size);
    ctx.ret_cdecl(ptr);
    Handled::Ok
}

fn calloc(ctx: &mut ApiContext) -> Handled {
    let n = ctx.arg(0);
    let sz = ctx.arg(1);
    let total = n.saturating_mul(sz);
    let ptr = ctx.heap_alloc(total);
    // heap is already zeroed from allocate()
    ctx.ret_cdecl(ptr);
    Handled::Ok
}

fn realloc(ctx: &mut ApiContext) -> Handled {
    let size = ctx.arg(1);
    let ptr = ctx.heap_alloc(size);
    ctx.ret_cdecl(ptr);
    Handled::Ok
}

fn memcpy(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let n = ctx.arg(2) as usize;
    if n > 0 {
        if let Ok(bytes) = ctx.memory.read_bytes(src, n) {
            let _ = ctx.memory.write_bytes(dst, &bytes);
        }
    }
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn memcpy_s(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let _cap = ctx.arg(1);
    let src = ctx.arg(2);
    let n = ctx.arg(3) as usize;
    if n > 0 {
        if let Ok(bytes) = ctx.memory.read_bytes(src, n) {
            let _ = ctx.memory.write_bytes(dst, &bytes);
        }
    }
    ctx.ret_cdecl(0);
    Handled::Ok
}

fn memset(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let val = ctx.arg(1) as u8;
    let n = ctx.arg(2) as usize;
    let buf = vec![val; n];
    let _ = ctx.memory.write_bytes(dst, &buf);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn memcmp(ctx: &mut ApiContext) -> Handled {
    let a = ctx.arg(0);
    let b = ctx.arg(1);
    let n = ctx.arg(2) as usize;
    let ba = ctx.memory.read_bytes(a, n).unwrap_or_default();
    let bb = ctx.memory.read_bytes(b, n).unwrap_or_default();
    let r = ba.cmp(&bb) as i32;
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

fn strlen(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let s = ctx.cstr(p);
    ctx.ret_cdecl(s.len() as u32);
    Handled::Ok
}

fn wcslen(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let s = ctx.wstr(p);
    ctx.ret_cdecl(s.len() as u32);
    Handled::Ok
}

fn strcmp(ctx: &mut ApiContext) -> Handled {
    let a = ctx.cstr(ctx.arg(0));
    let b = ctx.cstr(ctx.arg(1));
    let r = a.as_str().cmp(b.as_str()) as i32;
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

fn strncmp(ctx: &mut ApiContext) -> Handled {
    let a = ctx.cstr(ctx.arg(0));
    let b = ctx.cstr(ctx.arg(1));
    let n = ctx.arg(2) as usize;
    let r = a[..a.len().min(n)].cmp(&b[..b.len().min(n)]) as i32;
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

fn strcpy(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let s = ctx.cstr(src);
    let mut bytes = s.into_bytes();
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn strcat(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let existing = ctx.cstr(dst);
    let append = ctx.cstr(src);
    let result = existing + &append;
    let mut bytes = result.into_bytes();
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn strchr(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let c = ctx.arg(1) as u8;
    let s = ctx.memory.read_cstr(p);
    let pos = s.bytes().position(|b| b == c);
    let r = pos.map(|i| p + i as u32).unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

fn strrchr(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let c = ctx.arg(1) as u8;
    let s = ctx.memory.read_cstr(p);
    let pos = s.bytes().rposition(|b| b == c);
    let r = pos.map(|i| p + i as u32).unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

fn strstr(ctx: &mut ApiContext) -> Handled {
    let hay = ctx.cstr(ctx.arg(0));
    let needle = ctx.cstr(ctx.arg(1));
    let r = hay
        .find(&needle[..])
        .map(|i| ctx.arg(0) + i as u32)
        .unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

fn strtol(ctx: &mut ApiContext) -> Handled {
    let s = ctx.cstr(ctx.arg(0));
    let r = s.trim().parse::<i32>().unwrap_or(0) as u32;
    ctx.ret_cdecl(r);
    Handled::Ok
}

fn strtoul(ctx: &mut ApiContext) -> Handled {
    let s = ctx.cstr(ctx.arg(0));
    let r = s.trim().parse::<u32>().unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

fn atoi(ctx: &mut ApiContext) -> Handled {
    let s = ctx.cstr(ctx.arg(0));
    let r = s.trim().parse::<i32>().unwrap_or(0) as u32;
    ctx.ret_cdecl(r);
    Handled::Ok
}

fn puts(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let mut s = ctx.cstr(p);
    s.push('\n');
    ctx.console.stdout.extend_from_slice(s.as_bytes());
    ctx.ret_cdecl(0);
    Handled::Ok
}

fn putchar(ctx: &mut ApiContext) -> Handled {
    let c = ctx.arg(0) as u8;
    ctx.console.stdout.push(c);
    ctx.ret_cdecl(c as u32);
    Handled::Ok
}

fn printf(ctx: &mut ApiContext) -> Handled {
    let fmt_ptr = ctx.arg(0);
    let fmt = ctx.cstr(fmt_ptr);
    let result = format_string(ctx, &fmt, 1);
    let n = result.len();
    ctx.console.stdout.extend_from_slice(result.as_bytes());
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

fn fprintf(ctx: &mut ApiContext) -> Handled {
    // arg0 = FILE*, arg1 = fmt, rest = args
    let fmt_ptr = ctx.arg(1);
    let fmt = ctx.cstr(fmt_ptr);
    let result = format_string(ctx, &fmt, 2);
    let n = result.len();
    ctx.console.stdout.extend_from_slice(result.as_bytes());
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

fn sprintf_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let fmt_ptr = ctx.arg(1);
    let fmt = ctx.cstr(fmt_ptr);
    let result = format_string(ctx, &fmt, 2);
    let n = result.len();
    let mut bytes = result.into_bytes();
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

fn snprintf_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let cap = ctx.arg(1) as usize;
    let fmt_ptr = ctx.arg(2);
    let fmt = ctx.cstr(fmt_ptr);
    let result = format_string(ctx, &fmt, 3);
    let n = result.len().min(cap.saturating_sub(1));
    let mut bytes = result.into_bytes();
    bytes.truncate(n);
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

fn stdio_vfprintf(ctx: &mut ApiContext) -> Handled {
    // __stdio_common_vfprintf(options, stream, format, locale, va_list)
    let fmt_ptr = ctx.arg(2);
    let fmt = ctx.cstr(fmt_ptr);
    let result = format_string(ctx, &fmt, 4);
    ctx.console.stdout.extend_from_slice(result.as_bytes());
    ctx.ret_cdecl(result.len() as u32);
    Handled::Ok
}

fn fwrite(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(0);
    let size = ctx.arg(1);
    let n = ctx.arg(2);
    let bytes = ctx
        .memory
        .read_bytes(buf, (size * n) as usize)
        .unwrap_or_default();
    ctx.console.stdout.extend_from_slice(&bytes);
    ctx.ret_cdecl(n);
    Handled::Ok
}

fn fputc(ctx: &mut ApiContext) -> Handled {
    let c = ctx.arg(0) as u8;
    ctx.console.stdout.push(c);
    ctx.ret_cdecl(c as u32);
    Handled::Ok
}

fn fputs(ctx: &mut ApiContext) -> Handled {
    let s = ctx.cstr(ctx.arg(0));
    ctx.console.stdout.extend_from_slice(s.as_bytes());
    ctx.ret_cdecl(0);
    Handled::Ok
}

fn acrt_iob(ctx: &mut ApiContext) -> Handled {
    // Return a fake FILE* based on the fd (0=stdin, 1=stdout, 2=stderr)
    let fd = ctx.arg(0);
    let fake: u32 = 0x7FFD_F400 + fd * 0x20;
    ctx.ret_cdecl(fake);
    Handled::Ok
}

fn initterm(ctx: &mut ApiContext) -> Handled {
    // Walk table of function pointers and call each non-null one.
    // Table is [start, end) of fn* in guest memory.
    // We skip calling them to avoid complexity — return immediately.
    ctx.ret_cdecl(0);
    Handled::Ok
}

fn initterm_e(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(0);
    Handled::Ok
}

fn p_argc(ctx: &mut ApiContext) -> Handled {
    let va = 0x7FFD_F500u32;
    let _ = ctx.memory.write_u32(va, 1);
    ctx.ret_cdecl(va);
    Handled::Ok
}

fn p_argv(ctx: &mut ApiContext) -> Handled {
    let va = 0x7FFD_F510u32;
    let _ = ctx.memory.write_u32(va, 0x7FFD_F520);
    let _ = ctx.memory.write_bytes(0x7FFD_F520, b"program.exe\0");
    ctx.ret_cdecl(va);
    Handled::Ok
}

fn p_commode(ctx: &mut ApiContext) -> Handled {
    let va = 0x7FFD_F530u32;
    let _ = ctx.memory.write_u32(va, 0);
    ctx.ret_cdecl(va);
    Handled::Ok
}

fn p_fmode(ctx: &mut ApiContext) -> Handled {
    let va = 0x7FFD_F540u32;
    let _ = ctx.memory.write_u32(va, 0);
    ctx.ret_cdecl(va);
    Handled::Ok
}

// printf formatter

fn format_string(ctx: &ApiContext, fmt: &str, first_arg: u32) -> String {
    let mut out = String::new();
    let mut arg_idx = first_arg;
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        i += 1;
        if i >= chars.len() {
            break;
        }
        // skip flags, width, precision
        while i < chars.len() && "0123456789-+ #.*".contains(chars[i]) {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let spec = chars[i];
        i += 1;
        match spec {
            'd' | 'i' => {
                let v = ctx
                    .memory
                    .read_u32(ctx.cpu.esp + 4 + 4 * arg_idx)
                    .unwrap_or(0) as i32;
                out.push_str(&v.to_string());
                arg_idx += 1;
            }
            'u' => {
                let v = ctx
                    .memory
                    .read_u32(ctx.cpu.esp + 4 + 4 * arg_idx)
                    .unwrap_or(0);
                out.push_str(&v.to_string());
                arg_idx += 1;
            }
            'x' => {
                let v = ctx
                    .memory
                    .read_u32(ctx.cpu.esp + 4 + 4 * arg_idx)
                    .unwrap_or(0);
                out.push_str(&format!("{v:x}"));
                arg_idx += 1;
            }
            'X' => {
                let v = ctx
                    .memory
                    .read_u32(ctx.cpu.esp + 4 + 4 * arg_idx)
                    .unwrap_or(0);
                out.push_str(&format!("{v:X}"));
                arg_idx += 1;
            }
            'p' => {
                let v = ctx
                    .memory
                    .read_u32(ctx.cpu.esp + 4 + 4 * arg_idx)
                    .unwrap_or(0);
                out.push_str(&format!("{v:08X}"));
                arg_idx += 1;
            }
            'c' => {
                let v = ctx
                    .memory
                    .read_u32(ctx.cpu.esp + 4 + 4 * arg_idx)
                    .unwrap_or(0) as u8;
                out.push(v as char);
                arg_idx += 1;
            }
            's' => {
                let ptr = ctx
                    .memory
                    .read_u32(ctx.cpu.esp + 4 + 4 * arg_idx)
                    .unwrap_or(0);
                out.push_str(&ctx.memory.read_cstr(ptr));
                arg_idx += 1;
            }
            '%' => out.push('%'),
            'n' => {
                arg_idx += 1;
            } // skip
            _ => {
                out.push('%');
                out.push(spec);
            }
        }
    }
    out
}

// cdecl stubs

fn stub_zero_cdecl_0(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}
fn stub_zero_cdecl_1(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}
fn stub_zero_cdecl_2(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}
fn stub_zero_cdecl_3(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}
fn stub_void_1_cdecl(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}
fn stub_void_0(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}
fn stub_void_1(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}
