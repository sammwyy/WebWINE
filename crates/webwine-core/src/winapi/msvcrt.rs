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
        ("msvcrt.dll", "strncpy", strncpy),
        ("msvcrt.dll", "strcat", strcat),
        ("msvcrt.dll", "strncat", strncat),
        ("msvcrt.dll", "getenv", getenv_fn),
        ("msvcrt.dll", "_getcwd", getcwd_fn),
        ("msvcrt.dll", "getcwd", getcwd_fn),
        ("msvcrt.dll", "_chdir", chdir_fn),
        ("msvcrt.dll", "chdir", chdir_fn),
        // setjmp/longjmp: cmd.exe uses these for command-loop error recovery.
        ("msvcrt.dll", "_setjmp", setjmp_fn),
        ("msvcrt.dll", "_setjmp3", setjmp_fn),
        ("msvcrt.dll", "longjmp", longjmp_fn),
        // wide-string helpers
        ("msvcrt.dll", "wcscpy", wcscpy_fn),
        ("msvcrt.dll", "wcscat", wcscat_fn),
        ("msvcrt.dll", "wcsncpy", wcsncpy_fn),
        ("msvcrt.dll", "wcscmp", |c| { let a = c.wstr(c.arg(0)); let b = c.wstr(c.arg(1)); c.ret_cdecl(a.cmp(&b) as i32 as u32); Handled::Ok }),
        ("msvcrt.dll", "_wcsicmp", |c| { let a = c.wstr(c.arg(0)).to_lowercase(); let b = c.wstr(c.arg(1)).to_lowercase(); c.ret_cdecl(a.cmp(&b) as i32 as u32); Handled::Ok }),
        ("msvcrt.dll", "wcschr", wcschr_fn),
        ("msvcrt.dll", "wcsrchr", wcsrchr_fn),
        ("msvcrt.dll", "wcsstr", wcsstr_fn),
        ("msvcrt.dll", "wcsncmp", |c| { let a = c.wstr(c.arg(0)); let b = c.wstr(c.arg(1)); let n = c.arg(2) as usize; let r = a.chars().take(n).cmp(b.chars().take(n)) as i32; c.ret_cdecl(r as u32); Handled::Ok }),
        ("msvcrt.dll", "_wcsnicmp", |c| { let a = c.wstr(c.arg(0)).to_lowercase(); let b = c.wstr(c.arg(1)).to_lowercase(); let n = c.arg(2) as usize; let r = a.chars().take(n).cmp(b.chars().take(n)) as i32; c.ret_cdecl(r as u32); Handled::Ok }),
        ("msvcrt.dll", "towupper", |c| { let v = c.arg(0); c.ret_cdecl(char::from_u32(v).map(|ch| ch.to_ascii_uppercase() as u32).unwrap_or(v)); Handled::Ok }),
        ("msvcrt.dll", "towlower", |c| { let v = c.arg(0); c.ret_cdecl(char::from_u32(v).map(|ch| ch.to_ascii_lowercase() as u32).unwrap_or(v)); Handled::Ok }),
        ("msvcrt.dll", "iswalpha", |c| { let v = c.arg(0); c.ret_cdecl(char::from_u32(v).map(|ch| ch.is_alphabetic() as u32).unwrap_or(0)); Handled::Ok }),
        ("msvcrt.dll", "iswdigit", |c| { let v = c.arg(0); c.ret_cdecl(char::from_u32(v).map(|ch| ch.is_numeric() as u32).unwrap_or(0)); Handled::Ok }),
        ("msvcrt.dll", "iswspace", |c| { let v = c.arg(0); c.ret_cdecl(char::from_u32(v).map(|ch| ch.is_whitespace() as u32).unwrap_or(0)); Handled::Ok }),
        ("msvcrt.dll", "time", |c| { let t = c.arg(0); let now = 1_577_836_800u32; if t != 0 { let _ = c.memory.write_u32(t, now); } c.ret_cdecl(now); Handled::Ok }),
        ("msvcrt.dll", "srand", |c| {
            *c.rand_seed = c.arg(0);
            c.ret_cdecl(0);
            Handled::Ok
        }),
        ("msvcrt.dll", "rand", |c| {
            // Simple LCG from MSVC: rand_seed = rand_seed * 214013 + 2531011; return (rand_seed >> 16) & 0x7FFF;
            *c.rand_seed = c.rand_seed.wrapping_mul(214013).wrapping_add(2531011);
            let result = (*c.rand_seed >> 16) & 0x7FFF;
            c.ret_cdecl(result);
            Handled::Ok
        }),
        ("msvcrt.dll", "strchr", strchr),
        ("msvcrt.dll", "strrchr", strrchr),
        ("msvcrt.dll", "strstr", strstr),
        ("msvcrt.dll", "strtol", strtol),
        ("msvcrt.dll", "strtoul", strtoul),
        ("msvcrt.dll", "atoi", atoi),
        ("msvcrt.dll", "atol", atoi),
        ("msvcrt.dll", "_ultoa", |c| itoa_radix(c, false)),
        ("msvcrt.dll", "_ltoa", |c| itoa_radix(c, true)),
        ("msvcrt.dll", "_itoa", |c| itoa_radix(c, true)),
        ("msvcrt.dll", "ultoa", |c| itoa_radix(c, false)),
        ("msvcrt.dll", "ltoa", |c| itoa_radix(c, true)),
        ("msvcrt.dll", "itoa", |c| itoa_radix(c, true)),
        // CRT fd <-> Win32 HANDLE mapping (cmd.exe queries its std handles).
        ("msvcrt.dll", "_get_osfhandle", get_osfhandle),
        ("msvcrt.dll", "_isatty", |c| { let fd = c.arg(0); c.ret_cdecl(if fd <= 2 { 1 } else { 0 }); Handled::Ok }),
        ("msvcrt.dll", "_fileno", |c| { let s = c.arg(0); c.ret_cdecl(s); Handled::Ok }),
        ("msvcrt.dll", "atof", stub_zero_cdecl_1),
        ("msvcrt.dll", "puts", puts),
        ("msvcrt.dll", "putchar", putchar),
        ("msvcrt.dll", "fflush", stub_zero_cdecl_1),
        ("msvcrt.dll", "printf", printf),
        ("msvcrt.dll", "fprintf", fprintf),
        ("msvcrt.dll", "sprintf", sprintf_fn),
        ("msvcrt.dll", "snprintf", snprintf_fn),
        ("msvcrt.dll", "_snprintf", snprintf_fn),
        ("msvcrt.dll", "vprintf", vprintf_fn),
        ("msvcrt.dll", "vfprintf", vfprintf_fn),
        ("msvcrt.dll", "vsprintf", vsprintf_fn),
        ("msvcrt.dll", "_vsnprintf", vsnprintf_fn),
        ("msvcrt.dll", "vsnprintf", vsnprintf_fn),
        ("msvcrt.dll", "_snwprintf", snwprintf_fn),
        ("msvcrt.dll", "swprintf", snwprintf_no_count_fn),
        ("msvcrt.dll", "_vsnwprintf", vsnwprintf_fn),
        ("msvcrt.dll", "vswprintf", vsnwprintf_no_count_fn),
        ("msvcrt.dll", "_strdup", strdup_fn),
        ("msvcrt.dll", "strdup", strdup_fn),
        ("msvcrt.dll", "signal", |c| {
            c.ret_cdecl(0);
            Handled::Ok
        }),
        ("msvcrt.dll", "raise", |c| {
            c.ret_cdecl(0);
            Handled::Ok
        }),
        ("msvcrt.dll", "abort", |_c| Handled::ExitProcess(3)),
        ("msvcrt.dll", "_initterm", initterm),
        ("msvcrt.dll", "_initterm_e", initterm_e),
        ("msvcrt.dll", "__p___argc", p_argc),
        ("msvcrt.dll", "__p___argv", p_argv),
        ("msvcrt.dll", "__getmainargs", getmainargs),
        ("msvcrt.dll", "__wgetmainargs", wgetmainargs),
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
        // Wide buffer formatting: __stdio_common_vswprintf(opts, buf, count, fmt, locale, va)
        ("msvcrt.dll", "__stdio_common_vswprintf", stdio_vswprintf),
        ("msvcrt.dll", "__stdio_common_vswprintf_s", stdio_vswprintf),
        ("msvcrt.dll", "__stdio_common_vsnwprintf_s", stdio_vswprintf),
        ("msvcrt.dll", "__stdio_common_vsprintf", stdio_vsprintf),
        ("msvcrt.dll", "__stdio_common_vsprintf_s", stdio_vsprintf),
        // C++ runtime mutex primitives (msvcp_win). Correct arg counts; no-op.
        ("msvcp_win.dll", "_Mtx_init_in_situ", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Mtx_destroy_in_situ", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Mtx_lock", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Mtx_unlock", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Mtx_trylock", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Cnd_init_in_situ", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcp_win.dll", "_Cnd_destroy_in_situ", |c| { c.ret_cdecl(0); Handled::Ok }),
        ("msvcrt.dll", "__stdio_common_vsprintf", stub_zero_cdecl_1),
        ("msvcrt.dll", "fwrite", fwrite),
        ("msvcrt.dll", "fputc", fputc),
        ("msvcrt.dll", "fputs", fputs),
        // stdio backed by the VFS
        ("msvcrt.dll", "fopen", fopen),
        ("msvcrt.dll", "_fsopen", fsopen),
        ("msvcrt.dll", "freopen", freopen),
        ("msvcrt.dll", "fclose", fclose),
        ("msvcrt.dll", "fread", fread),
        ("msvcrt.dll", "fseek", fseek),
        ("msvcrt.dll", "ftell", ftell),
        ("msvcrt.dll", "rewind", rewind),
        ("msvcrt.dll", "fgetc", fgetc),
        ("msvcrt.dll", "getc", fgetc),
        ("msvcrt.dll", "fgets", fgets),
        ("msvcrt.dll", "feof", feof),
        // scanf family: we don't parse formatted input yet, so report EOF (-1).
        // Returning 0 ("no fields") makes typical `while (fscanf(..)!=EOF)` config
        // readers spin forever; EOF terminates them cleanly (defaults are used).
        ("msvcrt.dll", "fscanf", scanf_eof),
        ("msvcrt.dll", "scanf", scanf_eof),
        ("msvcrt.dll", "sscanf", scanf_eof),
        ("msvcrt.dll", "vfscanf", scanf_eof),
        ("msvcrt.dll", "vsscanf", scanf_eof),
        ("msvcrt.dll", "ferror", stub_zero_cdecl_1),
        ("msvcrt.dll", "setvbuf", stub_zero_cdecl_1),
        ("msvcrt.dll", "setbuf", stub_void_1_cdecl),
        // character classification / conversion
        ("msvcrt.dll", "isspace", |c| {
            ret_class(c, |b| b.is_ascii_whitespace())
        }),
        ("msvcrt.dll", "isdigit", |c| {
            ret_class(c, |b| b.is_ascii_digit())
        }),
        ("msvcrt.dll", "isalpha", |c| {
            ret_class(c, |b| b.is_ascii_alphabetic())
        }),
        ("msvcrt.dll", "isalnum", |c| {
            ret_class(c, |b| b.is_ascii_alphanumeric())
        }),
        ("msvcrt.dll", "isupper", |c| {
            ret_class(c, |b| b.is_ascii_uppercase())
        }),
        ("msvcrt.dll", "islower", |c| {
            ret_class(c, |b| b.is_ascii_lowercase())
        }),
        ("msvcrt.dll", "isxdigit", |c| {
            ret_class(c, |b| b.is_ascii_hexdigit())
        }),
        ("msvcrt.dll", "ispunct", |c| {
            ret_class(c, |b| b.is_ascii_punctuation())
        }),
        ("msvcrt.dll", "iscntrl", |c| {
            ret_class(c, |b| b.is_ascii_control())
        }),
        ("msvcrt.dll", "isprint", |c| {
            ret_class(c, |b| b.is_ascii_graphic() || b == b' ')
        }),
        ("msvcrt.dll", "isgraph", |c| {
            ret_class(c, |b| b.is_ascii_graphic())
        }),
        ("msvcrt.dll", "tolower", |c| {
            let v = c.arg(0) as u8;
            c.ret_cdecl(v.to_ascii_lowercase() as u32);
            Handled::Ok
        }),
        ("msvcrt.dll", "toupper", |c| {
            let v = c.arg(0) as u8;
            c.ret_cdecl(v.to_ascii_uppercase() as u32);
            Handled::Ok
        }),
        // wide-char classification / conversion
        ("msvcrt.dll", "towupper", |c| {
            let wc = c.arg(0) as u32;
            // Simple ASCII-range upcasing; for full Unicode use char::to_uppercase.
            let out = if wc <= 0xFFFF {
                char::from_u32(wc).map(|ch| ch.to_uppercase().next().unwrap_or(ch) as u32).unwrap_or(wc)
            } else { wc };
            c.ret_cdecl(out);
            Handled::Ok
        }),
        ("msvcrt.dll", "towlower", |c| {
            let wc = c.arg(0) as u32;
            let out = if wc <= 0xFFFF {
                char::from_u32(wc).map(|ch| ch.to_lowercase().next().unwrap_or(ch) as u32).unwrap_or(wc)
            } else { wc };
            c.ret_cdecl(out);
            Handled::Ok
        }),
        ("msvcrt.dll", "iswalpha", |c| {
            let wc = c.arg(0) as u32;
            let r = char::from_u32(wc).map(|ch| ch.is_alphabetic()).unwrap_or(false);
            c.ret_cdecl(if r { 1 } else { 0 });
            Handled::Ok
        }),
        ("msvcrt.dll", "iswdigit", |c| {
            let wc = c.arg(0) as u32;
            let r = char::from_u32(wc).map(|ch| ch.is_ascii_digit()).unwrap_or(false);
            c.ret_cdecl(if r { 1 } else { 0 });
            Handled::Ok
        }),
        ("msvcrt.dll", "iswspace", |c| {
            let wc = c.arg(0) as u32;
            let r = char::from_u32(wc).map(|ch| ch.is_whitespace()).unwrap_or(false);
            c.ret_cdecl(if r { 1 } else { 0 });
            Handled::Ok
        }),
        ("msvcrt.dll", "iswalnum", |c| {
            let wc = c.arg(0) as u32;
            let r = char::from_u32(wc).map(|ch| ch.is_alphanumeric()).unwrap_or(false);
            c.ret_cdecl(if r { 1 } else { 0 });
            Handled::Ok
        }),
        // _wcsnicmp(s1, s2, count) — case-insensitive wide-string compare, cdecl, 3 args.
        ("msvcrt.dll", "_wcsnicmp", wcsnicmp_fn),
        ("msvcrt.dll", "wcsnicmp",  wcsnicmp_fn),
        // C++ operator delete[] (decorated as ??_V@YAXPAX@Z) — 1 arg, cdecl, returns void.
        ("msvcrt.dll", "??_V@YAXPAX@Z", stub_void_1_cdecl),
        // operator new[] / delete
        ("msvcrt.dll", "??2@YAPAXI@Z",  malloc),    // operator new[](size)
        ("msvcrt.dll", "??3@YAXPAX@Z",  stub_void_1_cdecl), // operator delete
        // MSVC SEH helpers
        // _local_unwind4(cookie*, funcinfo*, target_level) — 3 args, cdecl.
        // We stub it as a no-op; the actual unwind (destructor calls) would
        // require full frame-walking, not needed for basic cmd.exe.
        ("msvcrt.dll", "_local_unwind4", stub_zero_cdecl_3),
        ("msvcrt.dll", "__local_unwind4", stub_zero_cdecl_3),
        ("msvcrt.dll", "_global_unwind2", stub_zero_cdecl_1),
        ("msvcrt.dll", "__set_app_type", stub_void_1_cdecl),
        ("msvcrt.dll", "_set_app_type", stub_void_1_cdecl),
        ("msvcrt.dll", "_configure_narrow_argv", stub_zero_cdecl_1),
        ("msvcrt.dll", "_configure_wide_argv", stub_zero_cdecl_1),
        (
            "msvcrt.dll",
            "_initialize_narrow_environment",
            stub_zero_cdecl_0,
        ),
        (
            "msvcrt.dll",
            "_initialize_wide_environment",
            stub_zero_cdecl_0,
        ),
        (
            "msvcrt.dll",
            "_get_initial_wide_environment",
            stub_zero_cdecl_0,
        ),
        ("msvcrt.dll", "__p___wargv", p_argv),
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
        // _onexit/__onexit return the registered function pointer on success
        // (NULL means failure, which some CRTs treat as fatal).
        ("msvcrt.dll", "_onexit", |c| {
            let f = c.arg(0);
            c.ret_cdecl(f);
            Handled::Ok
        }),
        ("msvcrt.dll", "__onexit", |c| {
            let f = c.arg(0);
            c.ret_cdecl(f);
            Handled::Ok
        }),
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
        ("msvcrt.dll", "_except_handler3", stub_one_cdecl_4),
        ("msvcrt.dll", "_except_handler4_common", stub_one_cdecl_4),
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
    let old = ctx.arg(0);
    let size = ctx.arg(1);
    let ptr = ctx.heap_realloc(old, size);
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

// getenv(name): we have no environment, so every variable is unset (NULL).
fn getenv_fn(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(0);
    Handled::Ok
}

// _getcwd(buf, size): write the CWD; if buf is NULL, malloc one (size bytes).
fn getcwd_fn(ctx: &mut ApiContext) -> Handled {
    let mut buf = ctx.arg(0);
    let size = ctx.arg(1);
    let mut bytes = ctx.cwd.clone().into_bytes();
    bytes.push(0);
    if buf == 0 {
        buf = ctx.heap_alloc(size.max(bytes.len() as u32));
    }
    let _ = ctx.memory.write_bytes(buf, &bytes);
    ctx.ret_cdecl(buf);
    Handled::Ok
}

// _chdir(path): set the working directory. Returns 0 on success.
fn chdir_fn(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.cstr(ctx.arg(0));
    *ctx.cwd = ctx.resolve_path(&raw);
    ctx.ret_cdecl(0);
    Handled::Ok
}

fn strncpy(ctx: &mut ApiContext) -> Handled {
    // strncpy(dst, src, n): copy at most n bytes; pad with NUL if src is shorter.
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let n = ctx.arg(2) as usize;
    let s = ctx.memory.read_cstr(src).into_bytes();
    let mut buf = vec![0u8; n];
    let copy = s.len().min(n);
    buf[..copy].copy_from_slice(&s[..copy]);
    let _ = ctx.memory.write_bytes(dst, &buf);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn strncat(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let n = ctx.arg(2) as usize;
    let existing = ctx.memory.read_cstr(dst);
    let append = ctx.memory.read_cstr(src);
    let take: String = append.chars().take(n).collect();
    let mut bytes = (existing + &take).into_bytes();
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

// setjmp/_setjmp3(jmp_buf, ...): save callee-saved regs, esp and the return
// address into the buffer (MSVC _JUMP_BUFFER: Ebp, Ebx, Edi, Esi, Esp, Eip),
// then return 0. longjmp restores them to resume here. We don't model SEH
// unwinding (Registration/Cookie), which is fine for plain error-recovery jumps.
fn setjmp_fn(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(0);
    let ret_addr = ctx.memory.read_u32(ctx.cpu.esp).unwrap_or(0);
    let esp_after = ctx.cpu.esp.wrapping_add(4); // esp once the return address is popped
    let _ = ctx.memory.write_u32(buf, ctx.cpu.ebp);
    let _ = ctx.memory.write_u32(buf + 4, ctx.cpu.ebx);
    let _ = ctx.memory.write_u32(buf + 8, ctx.cpu.edi);
    let _ = ctx.memory.write_u32(buf + 12, ctx.cpu.esi);
    let _ = ctx.memory.write_u32(buf + 16, esp_after);
    let _ = ctx.memory.write_u32(buf + 20, ret_addr);
    ctx.ret_cdecl(0);
    Handled::Ok
}

// longjmp(jmp_buf, val): restore the saved state and resume at the setjmp site,
// returning `val` (or 1 if val == 0).
fn longjmp_fn(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(0);
    let val = ctx.arg(1);
    ctx.cpu.ebp = ctx.memory.read_u32(buf).unwrap_or(0);
    ctx.cpu.ebx = ctx.memory.read_u32(buf + 4).unwrap_or(0);
    ctx.cpu.edi = ctx.memory.read_u32(buf + 8).unwrap_or(0);
    ctx.cpu.esi = ctx.memory.read_u32(buf + 12).unwrap_or(0);
    ctx.cpu.esp = ctx.memory.read_u32(buf + 16).unwrap_or(0);
    ctx.cpu.eip = ctx.memory.read_u32(buf + 20).unwrap_or(0);
    ctx.cpu.eax = if val == 0 { 1 } else { val };
    Handled::Ok
}

fn wcscpy_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let s = ctx.wstr(ctx.arg(1));
    let mut bytes: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    bytes.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn wcsncpy_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let n = ctx.arg(2) as usize;
    let s = ctx.wstr(ctx.arg(1));
    let units: Vec<u16> = s.encode_utf16().collect();
    let mut bytes: Vec<u8> = Vec::with_capacity(n * 2);
    for i in 0..n {
        let c = units.get(i).copied().unwrap_or(0); // NUL-pad if src shorter
        bytes.extend_from_slice(&c.to_le_bytes());
    }
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn wcscat_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let existing = ctx.wstr(dst);
    let append = ctx.wstr(ctx.arg(1));
    let combined = existing + &append;
    let mut bytes: Vec<u8> = combined.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    bytes.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn wcschr_fn(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let needle = ctx.arg(1) as u16;
    let s = ctx.wstr(p);
    let pos = s.encode_utf16().position(|c| c == needle);
    let r = pos.map(|i| p + (i as u32) * 2).unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

fn wcsrchr_fn(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let needle = ctx.arg(1) as u16;
    let s = ctx.wstr(p);
    let pos = s.encode_utf16().enumerate().filter(|&(_, c)| c == needle).last().map(|(i, _)| i);
    let r = pos.map(|i| p + (i as u32) * 2).unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

fn wcsstr_fn(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let hay: Vec<u16> = ctx.wstr(p).encode_utf16().collect();
    let needle: Vec<u16> = ctx.wstr(ctx.arg(1)).encode_utf16().collect();
    let r = if needle.is_empty() {
        p
    } else {
        hay.windows(needle.len()).position(|w| w == needle.as_slice())
            .map(|i| p + (i as u32) * 2).unwrap_or(0)
    };
    ctx.ret_cdecl(r);
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

// _itoa/_ltoa/_ultoa(value, char* str, int radix): write value in `radix` to str.
fn itoa_radix(ctx: &mut ApiContext, signed: bool) -> Handled {
    let value = ctx.arg(0);
    let dst = ctx.arg(1);
    let radix = ctx.arg(2).clamp(2, 36);

    let mut s = if signed && radix == 10 && (value as i32) < 0 {
        format!("-{}", (value as i32).unsigned_abs() as u64)
    } else {
        to_radix(value as u64, radix)
    };
    s.push('\0');
    let _ = ctx.memory.write_bytes(dst, s.as_bytes());
    ctx.ret_cdecl(dst);
    Handled::Ok
}

fn to_radix(mut v: u64, radix: u32) -> String {
    if v == 0 {
        return "0".to_string();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buf = Vec::new();
    while v > 0 {
        buf.push(digits[(v % radix as u64) as usize]);
        v /= radix as u64;
    }
    buf.reverse();
    String::from_utf8(buf).unwrap()
}

// _get_osfhandle(fd): map CRT fd 0/1/2 to the std Win32 HANDLEs.
fn get_osfhandle(ctx: &mut ApiContext) -> Handled {
    let h = match ctx.arg(0) {
        0 => 0xFFFF_FFF6u32, // stdin
        1 => 0xFFFF_FFF5,    // stdout
        2 => 0xFFFF_FFF4,    // stderr
        _ => 0xFFFF_FFFF,    // INVALID_HANDLE_VALUE
    };
    ctx.ret_cdecl(h);
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
    let stream = ctx.arg(0);
    let fmt_ptr = ctx.arg(1);
    let fmt = ctx.cstr(fmt_ptr);
    let result = format_string(ctx, &fmt, 2);
    let n = result.len();
    write_stream(ctx, stream, result.as_bytes());
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

// vprintf(fmt, va_list) -> console
fn vprintf_fn(ctx: &mut ApiContext) -> Handled {
    let fmt = ctx.cstr(ctx.arg(0));
    let va = ctx.arg(1);
    let result = format_va(ctx, &fmt, va);
    let n = result.len();
    ctx.console.stdout.extend_from_slice(result.as_bytes());
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

// vfprintf(FILE*, fmt, va_list)
fn vfprintf_fn(ctx: &mut ApiContext) -> Handled {
    let stream = ctx.arg(0);
    let fmt = ctx.cstr(ctx.arg(1));
    let va = ctx.arg(2);
    let result = format_va(ctx, &fmt, va);
    let n = result.len();
    write_stream(ctx, stream, result.as_bytes());
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

// vsprintf(buf, fmt, va_list)
fn vsprintf_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let fmt = ctx.cstr(ctx.arg(1));
    let va = ctx.arg(2);
    let result = format_va(ctx, &fmt, va);
    let n = result.len();
    let mut bytes = result.into_bytes();
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

// _vsnprintf(buf, count, fmt, va_list)
fn vsnprintf_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let cap = ctx.arg(1) as usize;
    let fmt = ctx.cstr(ctx.arg(2));
    let va = ctx.arg(3);
    let result = format_va(ctx, &fmt, va);
    let n = result.len().min(cap.saturating_sub(1));
    let mut bytes = result.into_bytes();
    bytes.truncate(n);
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

// _strdup(s): malloc a copy of the C string.
fn strdup_fn(ctx: &mut ApiContext) -> Handled {
    let s = ctx.cstr(ctx.arg(0));
    let mut bytes = s.into_bytes();
    bytes.push(0);
    let p = ctx.heap_alloc(bytes.len() as u32);
    let _ = ctx.memory.write_bytes(p, &bytes);
    ctx.ret_cdecl(p);
    Handled::Ok
}

// Wide formatting shared core. Reads a wide format string and produces wide
// output. %s is a wide string arg (wprintf convention); %d/%u/%x/%c handled.
fn format_wide(ctx: &ApiContext, fmt: &str, mut src: ArgSrc) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            let mut b = [0u16; 2];
            for u in chars[i].encode_utf16(&mut b) { out.push(*u); }
            i += 1;
            continue;
        }
        i += 1;
        while i < chars.len() && "0123456789-+ #.*lh".contains(chars[i]) { i += 1; }
        if i >= chars.len() { break; }
        let spec = chars[i];
        i += 1;
        let push_str = |out: &mut Vec<u16>, s: &str| out.extend(s.encode_utf16());
        match spec {
            'd' | 'i' => push_str(&mut out, &(src.next(&ctx.memory) as i32).to_string()),
            'u' => push_str(&mut out, &src.next(&ctx.memory).to_string()),
            'x' => push_str(&mut out, &format!("{:x}", src.next(&ctx.memory))),
            'X' => push_str(&mut out, &format!("{:X}", src.next(&ctx.memory))),
            'p' => push_str(&mut out, &format!("{:08X}", src.next(&ctx.memory))),
            'c' => out.push(src.next(&ctx.memory) as u16),
            's' => { let ptr = src.next(&ctx.memory); out.extend(ctx.memory.read_wstr(ptr).encode_utf16()); }
            'S' => { let ptr = src.next(&ctx.memory); push_str(&mut out, &ctx.memory.read_cstr(ptr)); }
            '%' => out.push(b'%' as u16),
            _ => { out.push(b'%' as u16); out.push(spec as u16); }
        }
    }
    out
}

fn write_wide(ctx: &mut ApiContext, dst: u32, cap: usize, units: &[u16]) -> u32 {
    let n = if cap > 0 { units.len().min(cap - 1) } else { units.len() };
    let mut bytes: Vec<u8> = units[..n].iter().flat_map(|u| u.to_le_bytes()).collect();
    bytes.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    n as u32
}

// _snwprintf(buf, count, fmt, ...)
fn snwprintf_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let cap = ctx.arg(1) as usize;
    let fmt = ctx.wstr(ctx.arg(2));
    let units = format_wide(ctx, &fmt, ArgSrc::Stack { esp: ctx.cpu.esp, idx: 3 });
    let n = write_wide(ctx, dst, cap, &units);
    ctx.ret_cdecl(n);
    Handled::Ok
}

// swprintf(buf, fmt, ...) — no count argument.
fn snwprintf_no_count_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let fmt = ctx.wstr(ctx.arg(1));
    let units = format_wide(ctx, &fmt, ArgSrc::Stack { esp: ctx.cpu.esp, idx: 2 });
    let n = write_wide(ctx, dst, 0, &units);
    ctx.ret_cdecl(n);
    Handled::Ok
}

// _vsnwprintf(buf, count, fmt, va_list)
fn vsnwprintf_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let cap = ctx.arg(1) as usize;
    let fmt = ctx.wstr(ctx.arg(2));
    let va = ctx.arg(3);
    let units = format_wide(ctx, &fmt, ArgSrc::Va { ptr: va, idx: 0 });
    let n = write_wide(ctx, dst, cap, &units);
    ctx.ret_cdecl(n);
    Handled::Ok
}

// vswprintf(buf, fmt, va_list) — no count argument.
fn vsnwprintf_no_count_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let fmt = ctx.wstr(ctx.arg(1));
    let va = ctx.arg(2);
    let units = format_wide(ctx, &fmt, ArgSrc::Va { ptr: va, idx: 0 });
    let n = write_wide(ctx, dst, 0, &units);
    ctx.ret_cdecl(n);
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

// __stdio_common_vswprintf(opts(u64), buffer, count, format(wide), locale, va_list)
// -> wide buffer formatting. opts occupies two arg slots (it is __int64).
fn stdio_vswprintf(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(2);
    let count = ctx.arg(3) as usize;
    let fmt = ctx.wstr(ctx.arg(4));
    let va = ctx.arg(6);
    let units = format_wide(ctx, &fmt, ArgSrc::Va { ptr: va, idx: 0 });
    let n = write_wide(ctx, buf, count, &units);
    ctx.ret_cdecl(n);
    Handled::Ok
}

// __stdio_common_vsprintf(opts(u64), buffer, count, format, locale, va_list)
fn stdio_vsprintf(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(2);
    let count = ctx.arg(3) as usize;
    let fmt = ctx.cstr(ctx.arg(4));
    let va = ctx.arg(6);
    let result = format_va(ctx, &fmt, va);
    let n = if count > 0 { result.len().min(count - 1) } else { result.len() };
    let mut bytes = result.into_bytes();
    bytes.truncate(n);
    bytes.push(0);
    let _ = ctx.memory.write_bytes(buf, &bytes);
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

fn fwrite(ctx: &mut ApiContext) -> Handled {
    // fwrite(buf, size, count, FILE*)
    let buf = ctx.arg(0);
    let size = ctx.arg(1);
    let n = ctx.arg(2);
    let stream = ctx.arg(3);
    let bytes = ctx
        .memory
        .read_bytes(buf, (size * n) as usize)
        .unwrap_or_default();
    write_stream(ctx, stream, &bytes);
    ctx.ret_cdecl(n);
    Handled::Ok
}

fn fputc(ctx: &mut ApiContext) -> Handled {
    // fputc(c, FILE*)
    let c = ctx.arg(0) as u8;
    let stream = ctx.arg(1);
    write_stream(ctx, stream, &[c]);
    ctx.ret_cdecl(c as u32);
    Handled::Ok
}

fn fputs(ctx: &mut ApiContext) -> Handled {
    // fputs(str, FILE*)
    let s = ctx.cstr(ctx.arg(0));
    let stream = ctx.arg(1);
    write_stream(ctx, stream, s.as_bytes());
    ctx.ret_cdecl(0);
    Handled::Ok
}

// VFS-backed stdio
// A FILE* is either one of the fake stdout/stderr pointers from __acrt_iob_func
// (>= 0x7FFD_0000), or a small VFS handle returned by fopen. Writes to a VFS
// handle hit the file; everything else goes to the console.

fn is_vfs_stream(stream: u32) -> bool {
    stream != 0 && stream < 0x7FFD_0000
}

fn write_stream(ctx: &mut ApiContext, stream: u32, bytes: &[u8]) {
    use crate::vm::handles::KernelObject;
    let target = match ctx.handles.get(stream) {
        Some(KernelObject::VfsFile { path, cursor, .. }) if is_vfs_stream(stream) => {
            Some((path.clone(), *cursor))
        }
        _ => None,
    };
    match target {
        Some((path, cursor)) => {
            let mut content = ctx.fs.read_file(&path).unwrap_or_default();
            let start = cursor as usize;
            let end = start + bytes.len();
            if content.len() < end {
                content.resize(end, 0);
            }
            content[start..end].copy_from_slice(bytes);
            let _ = ctx.fs.mount_file(&path, content);
            if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(stream) {
                *cursor += bytes.len() as u64;
            }
        }
        None => ctx.console.stdout.extend_from_slice(bytes),
    }
}

// fopen(path, mode) -> FILE*
fn fopen(ctx: &mut ApiContext) -> Handled {
    let path = ctx.cstr(ctx.arg(0));
    let mode = ctx.cstr(ctx.arg(1));
    let h = open_vfs(ctx, &path, &mode);
    ctx.ret_cdecl(h);
    Handled::Ok
}

// _fsopen(path, mode, shflag) -> FILE*
fn fsopen(ctx: &mut ApiContext) -> Handled {
    let path = ctx.cstr(ctx.arg(0));
    let mode = ctx.cstr(ctx.arg(1));
    let h = open_vfs(ctx, &path, &mode);
    ctx.ret_cdecl(h);
    Handled::Ok
}

// freopen(path, mode, stream) -> stream (or NULL). We open/truncate the file but
// keep redirecting the original stream's writes to the console, so program log
// output stays visible rather than vanishing into a file.
fn freopen(ctx: &mut ApiContext) -> Handled {
    let path = ctx.cstr(ctx.arg(0));
    let mode = ctx.cstr(ctx.arg(1));
    let stream = ctx.arg(2);
    open_vfs(ctx, &path, &mode);
    ctx.ret_cdecl(stream);
    Handled::Ok
}

fn open_vfs(ctx: &mut ApiContext, raw_path: &str, mode: &str) -> u32 {
    use crate::vm::handles::KernelObject;
    let path = ctx.resolve_path(raw_path);
    let writable = mode.contains('w') || mode.contains('a') || mode.contains('+');
    let truncate = mode.contains('w');
    let append = mode.contains('a');
    let exists = ctx.fs.node_exists(&path);

    if !writable && !exists {
        return 0; // fopen("r") on a missing file fails
    }
    if truncate || !exists {
        if ctx.fs.mount_file(&path, Vec::new()).is_err() {
            return 0;
        }
    }
    let cursor = if append {
        ctx.fs.read_file(&path).map(|b| b.len() as u64).unwrap_or(0)
    } else {
        0
    };
    ctx.handles.insert(KernelObject::VfsFile {
        path,
        cursor,
        writable,
    })
}

fn fclose(ctx: &mut ApiContext) -> Handled {
    let stream = ctx.arg(0);
    if is_vfs_stream(stream) {
        ctx.handles.remove(stream);
    }
    ctx.ret_cdecl(0);
    Handled::Ok
}

// fread(buf, size, count, FILE*) -> count of full elements read
fn fread(ctx: &mut ApiContext) -> Handled {
    use crate::vm::handles::KernelObject;
    let buf = ctx.arg(0);
    let size = ctx.arg(1).max(1);
    let count = ctx.arg(2);
    let stream = ctx.arg(3);
    let total = (size * count) as usize;

    let info = match ctx.handles.get(stream) {
        Some(KernelObject::VfsFile { path, cursor, .. }) if is_vfs_stream(stream) => {
            Some((path.clone(), *cursor))
        }
        _ => None,
    };
    let Some((path, cursor)) = info else {
        ctx.ret_cdecl(0);
        return Handled::Ok;
    };

    // Read only the requested range (not the whole file) so large wad reads stay fast.
    let chunk = ctx.fs.read_range(&path, cursor as usize, total).unwrap_or_default();
    let read_bytes = chunk.len();
    let _ = ctx.memory.write_bytes(buf, &chunk);
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(stream) {
        *cursor += read_bytes as u64;
    }
    ctx.ret_cdecl((read_bytes / size as usize) as u32);
    Handled::Ok
}

// fseek(FILE*, offset, origin): 0=SEEK_SET, 1=SEEK_CUR, 2=SEEK_END
fn fseek(ctx: &mut ApiContext) -> Handled {
    use crate::vm::handles::KernelObject;
    let stream = ctx.arg(0);
    let offset = ctx.arg(1) as i32 as i64;
    let origin = ctx.arg(2);

    let len = match ctx.handles.get(stream) {
        Some(KernelObject::VfsFile { path, .. }) if is_vfs_stream(stream) => {
            ctx.fs.file_len(path).unwrap_or(0) as i64
        }
        _ => {
            ctx.ret_cdecl(0xFFFF_FFFF);
            return Handled::Ok;
        }
    };
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(stream) {
        let base = match origin {
            1 => *cursor as i64,
            2 => len,
            _ => 0,
        };
        *cursor = (base + offset).max(0) as u64;
    }
    ctx.ret_cdecl(0);
    Handled::Ok
}

fn ftell(ctx: &mut ApiContext) -> Handled {
    use crate::vm::handles::KernelObject;
    let stream = ctx.arg(0);
    let pos = match ctx.handles.get(stream) {
        Some(KernelObject::VfsFile { cursor, .. }) if is_vfs_stream(stream) => *cursor as u32,
        _ => 0xFFFF_FFFF,
    };
    ctx.ret_cdecl(pos);
    Handled::Ok
}

fn rewind(ctx: &mut ApiContext) -> Handled {
    use crate::vm::handles::KernelObject;
    let stream = ctx.arg(0);
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(stream) {
        *cursor = 0;
    }
    ctx.ret_cdecl(0);
    Handled::Ok
}

// fgetc(FILE*) -> int (byte or EOF=-1)
fn fgetc(ctx: &mut ApiContext) -> Handled {
    use crate::vm::handles::KernelObject;
    let stream = ctx.arg(0);
    let info = match ctx.handles.get(stream) {
        Some(KernelObject::VfsFile { path, cursor, .. }) if is_vfs_stream(stream) => {
            Some((path.clone(), *cursor))
        }
        _ => None,
    };
    let Some((path, cursor)) = info else {
        ctx.ret_cdecl(0xFFFF_FFFF);
        return Handled::Ok;
    };
    let byte = ctx.fs.read_range(&path, cursor as usize, 1).ok().and_then(|b| b.first().copied());
    match byte {
        Some(b) => {
            if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(stream) {
                *cursor += 1;
            }
            ctx.ret_cdecl(b as u32);
        }
        None => ctx.ret_cdecl(0xFFFF_FFFF), // EOF
    }
    Handled::Ok
}

// fgets(buf, n, FILE*): read up to n-1 bytes or until newline.
fn fgets(ctx: &mut ApiContext) -> Handled {
    use crate::vm::handles::KernelObject;
    let buf = ctx.arg(0);
    let n = ctx.arg(1) as usize;
    let stream = ctx.arg(2);
    let info = match ctx.handles.get(stream) {
        Some(KernelObject::VfsFile { path, cursor, .. }) if is_vfs_stream(stream) => {
            Some((path.clone(), *cursor))
        }
        _ => None,
    };
    let Some((path, cursor)) = info else {
        ctx.ret_cdecl(0);
        return Handled::Ok;
    };
    if n == 0 {
        ctx.ret_cdecl(0);
        return Handled::Ok;
    }

    // Read up to n-1 bytes from the cursor; stop at newline.
    let window = ctx.fs.read_range(&path, cursor as usize, n - 1).unwrap_or_default();
    if window.is_empty() {
        ctx.ret_cdecl(0);
        return Handled::Ok;
    }
    let mut line = Vec::new();
    for &b in &window {
        line.push(b);
        if b == b'\n' {
            break;
        }
    }
    let read = line.len();
    line.push(0);
    let _ = ctx.memory.write_bytes(buf, &line);
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(stream) {
        *cursor += read as u64;
    }
    ctx.ret_cdecl(buf);
    Handled::Ok
}

fn feof(ctx: &mut ApiContext) -> Handled {
    use crate::vm::handles::KernelObject;
    let stream = ctx.arg(0);
    let eof = match ctx.handles.get(stream) {
        Some(KernelObject::VfsFile { path, cursor, .. }) if is_vfs_stream(stream) => {
            let len = ctx.fs.file_len(path).unwrap_or(0) as u64;
            *cursor >= len
        }
        _ => false,
    };
    ctx.ret_cdecl(eof as u32);
    Handled::Ok
}

// scanf/fscanf/sscanf: return EOF (-1). cdecl, caller cleans the stack.
fn scanf_eof(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(0xFFFF_FFFF);
    Handled::Ok
}

fn ret_class(ctx: &mut ApiContext, pred: impl Fn(u8) -> bool) -> Handled {
    let v = ctx.arg(0) as u8;
    ctx.ret_cdecl(if pred(v) { 1 } else { 0 });
    Handled::Ok
}

fn acrt_iob(ctx: &mut ApiContext) -> Handled {
    // Return a fake FILE* based on the fd (0=stdin, 1=stdout, 2=stderr)
    let fd = ctx.arg(0);
    let fake: u32 = 0x7FFD_F400 + fd * 0x20;
    ctx.ret_cdecl(fake);
    Handled::Ok
}

// _initterm(first, last): call each non-null fn pointer in [first, last).
// We collect the pointers and hand them to the executor, which actually
// runs them (a handler can't call guest code itself).
fn initterm(ctx: &mut ApiContext) -> Handled {
    let first = ctx.arg(0);
    let last = ctx.arg(1);
    Handled::CallChain(collect_init_table(ctx, first, last))
}

fn initterm_e(ctx: &mut ApiContext) -> Handled {
    let first = ctx.arg(0);
    let last = ctx.arg(1);
    Handled::CallChainE(collect_init_table(ctx, first, last))
}

fn collect_init_table(ctx: &ApiContext, first: u32, last: u32) -> Vec<u32> {
    let mut funcs = Vec::new();
    let mut p = first;
    while p < last {
        if let Ok(pfn) = ctx.memory.read_u32(p) {
            if pfn != 0 {
                funcs.push(pfn);
            }
        }
        p = p.wrapping_add(4);
    }
    funcs
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

// Split a Windows command line into argv, respecting double-quoted segments.
fn tokenize_cmdline(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut started = false;
    for c in s.chars() {
        match c {
            '"' => { in_quotes = !in_quotes; started = true; }
            c if c.is_whitespace() && !in_quotes => {
                if started { args.push(std::mem::take(&mut cur)); started = false; }
            }
            c => { cur.push(c); started = true; }
        }
    }
    if started { args.push(cur); }
    if args.is_empty() { args.push(String::new()); }
    args
}

// __getmainargs(int* argc, char*** argv, char*** env, int wild, _startupinfo*)
// Fills argc/argv/env so the CRT can call main(argc, argv, env). cdecl.
fn getmainargs(ctx: &mut ApiContext) -> Handled {
    let argc_p = ctx.arg(0);
    let argv_p = ctx.arg(1);
    let env_p = ctx.arg(2);

    let args = tokenize_cmdline(ctx.cmdline);
    let argc = args.len() as u32;
    let argv = ctx.heap_alloc((argc + 1) * 4);
    for (i, a) in args.iter().enumerate() {
        let mut b = a.clone().into_bytes();
        b.push(0);
        let p = ctx.heap_alloc(b.len() as u32);
        let _ = ctx.memory.write_bytes(p, &b);
        let _ = ctx.memory.write_u32(argv + i as u32 * 4, p);
    }
    let _ = ctx.memory.write_u32(argv + argc * 4, 0); // NULL terminator
    let env = ctx.heap_alloc(4);
    let _ = ctx.memory.write_u32(env, 0);

    if argc_p != 0 { let _ = ctx.memory.write_u32(argc_p, argc); }
    if argv_p != 0 { let _ = ctx.memory.write_u32(argv_p, argv); }
    if env_p != 0 { let _ = ctx.memory.write_u32(env_p, env); }
    ctx.ret_cdecl(0);
    Handled::Ok
}

// Wide variant: argv/env are wchar_t**.
fn wgetmainargs(ctx: &mut ApiContext) -> Handled {
    let argc_p = ctx.arg(0);
    let argv_p = ctx.arg(1);
    let env_p = ctx.arg(2);

    let args = tokenize_cmdline(ctx.cmdline);
    let argc = args.len() as u32;
    let argv = ctx.heap_alloc((argc + 1) * 4);
    for (i, a) in args.iter().enumerate() {
        let mut s = a.clone();
        s.push('\0');
        let wide: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        let p = ctx.heap_alloc(wide.len() as u32);
        let _ = ctx.memory.write_bytes(p, &wide);
        let _ = ctx.memory.write_u32(argv + i as u32 * 4, p);
    }
    let _ = ctx.memory.write_u32(argv + argc * 4, 0);
    let env = ctx.heap_alloc(4);
    let _ = ctx.memory.write_u32(env, 0);

    if argc_p != 0 { let _ = ctx.memory.write_u32(argc_p, argc); }
    if argv_p != 0 { let _ = ctx.memory.write_u32(argv_p, argv); }
    if env_p != 0 { let _ = ctx.memory.write_u32(env_p, env); }
    ctx.ret_cdecl(0);
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

// Source of variadic arguments for the printf family. Stack-based functions
// (printf, fprintf) read successive 4-byte slots above ESP; the v* variants read
// from a va_list pointer into guest memory.
enum ArgSrc {
    Stack { esp: u32, idx: u32 },
    Va { ptr: u32, idx: u32 },
}

impl ArgSrc {
    fn next(&mut self, mem: &crate::vm::memory::GuestMemory) -> u32 {
        match self {
            ArgSrc::Stack { esp, idx } => {
                let v = mem.read_u32(*esp + 4 + 4 * *idx).unwrap_or(0);
                *idx += 1;
                v
            }
            ArgSrc::Va { ptr, idx } => {
                let v = mem.read_u32(*ptr + 4 * *idx).unwrap_or(0);
                *idx += 1;
                v
            }
        }
    }
}

fn format_string(ctx: &ApiContext, fmt: &str, first_arg: u32) -> String {
    format_args_src(
        ctx,
        fmt,
        ArgSrc::Stack {
            esp: ctx.cpu.esp,
            idx: first_arg,
        },
    )
}

fn format_va(ctx: &ApiContext, fmt: &str, va_ptr: u32) -> String {
    format_args_src(
        ctx,
        fmt,
        ArgSrc::Va {
            ptr: va_ptr,
            idx: 0,
        },
    )
}

fn format_args_src(ctx: &ApiContext, fmt: &str, mut src: ArgSrc) -> String {
    let mut out = String::new();
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
            'd' | 'i' => out.push_str(&(src.next(&ctx.memory) as i32).to_string()),
            'u' => out.push_str(&src.next(&ctx.memory).to_string()),
            'x' => out.push_str(&format!("{:x}", src.next(&ctx.memory))),
            'X' => out.push_str(&format!("{:X}", src.next(&ctx.memory))),
            'p' => out.push_str(&format!("{:08X}", src.next(&ctx.memory))),
            'c' => out.push(src.next(&ctx.memory) as u8 as char),
            's' => {
                let ptr = src.next(&ctx.memory);
                out.push_str(&ctx.memory.read_cstr(ptr));
            }
            'f' | 'g' | 'e' => {
                // doubles occupy two arg slots; reconstruct the f64 bit pattern.
                let lo = src.next(&ctx.memory) as u64;
                let hi = src.next(&ctx.memory) as u64;
                out.push_str(&f64::from_bits((hi << 32) | lo).to_string());
            }
            '%' => out.push('%'),
            'n' => {
                src.next(&ctx.memory);
            }
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
fn stub_one_cdecl_4(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(1); // EXCEPTION_CONTINUE_SEARCH
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

/// _wcsnicmp(s1, s2, count) — case-insensitive wide-string comparison, cdecl, 3 args.
/// Returns negative/0/positive like strcmp.
fn wcsnicmp_fn(ctx: &mut ApiContext) -> Handled {
    let p1    = ctx.arg(0);
    let p2    = ctx.arg(1);
    let count = ctx.arg(2) as usize;
    let a: Vec<u16> = {
        let s = ctx.wstr(p1);
        s.encode_utf16().take(count).collect()
    };
    let b: Vec<u16> = {
        let s = ctx.wstr(p2);
        s.encode_utf16().take(count).collect()
    };
    // Case-fold both sides using to_uppercase on each char.
    let fold = |units: &[u16]| -> Vec<u16> {
        units.iter().flat_map(|&u| {
            char::from_u32(u as u32)
                .map(|ch| {
                    ch.to_uppercase()
                        .flat_map(|c| c.encode_utf16(&mut [0u16; 2]).to_vec())
                        .collect::<Vec<u16>>()
                })
                .unwrap_or_else(|| vec![u])
        }).collect()
    };
    let fa = fold(&a);
    let fb = fold(&b);
    let r = fa.cmp(&fb) as i32;
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}
