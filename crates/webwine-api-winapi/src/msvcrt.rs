use super::{ApiContext, Handled, WinApiRegistry};

// Process-lifetime CRT globals. The slots live in the mapped PEB scratch page;
// their pointed-to strings/arrays live in a separate data page initialized by
// the native loader before the entry point runs.
pub const CRT_ACMDLN_SLOT: u32 = 0x7FFD_F560;
pub const CRT_WCMDLN_SLOT: u32 = 0x7FFD_F564;
pub const CRT_ARGC_SLOT: u32 = 0x7FFD_F568;
pub const CRT_ARGV_SLOT: u32 = 0x7FFD_F56C;
pub const CRT_WARGV_SLOT: u32 = 0x7FFD_F570;
pub const CRT_ENVIRON_SLOT: u32 = 0x7FFD_F574;
pub const CRT_WENVIRON_SLOT: u32 = 0x7FFD_F578;
pub const CRT_FMODE_SLOT: u32 = 0x7FFD_F57C;
pub const CRT_COMMODE_SLOT: u32 = 0x7FFD_F580;
const CRT_DATA_BASE: u32 = 0x7FFC_0000;
const CRT_DATA_SIZE: u32 = 0x0001_0000;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("msvcrt.dll", "exit", exit),
        ("msvcrt.dll", "_exit", exit),
        ("msvcrt.dll", "_cexit", cexit),
        ("msvcrt.dll", "malloc", malloc),
        ("msvcrt.dll", "_malloc_base", malloc),
        ("msvcrt.dll", "calloc", calloc),
        ("msvcrt.dll", "free", free_fn),
        ("msvcrt.dll", "_free_base", free_fn),
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
        ("msvcrt.dll", "_stricmp", stricmp),
        ("msvcrt.dll", "stricmp", stricmp),
        ("msvcrt.dll", "_strcmpi", stricmp),
        ("msvcrt.dll", "strcmpi", stricmp),
        ("msvcrt.dll", "_strnicmp", strnicmp),
        ("msvcrt.dll", "strnicmp", strnicmp),
        ("msvcrt.dll", "_strncmpi", strnicmp),
        ("msvcrt.dll", "strcpy", strcpy),
        ("msvcrt.dll", "strncpy", strncpy),
        ("msvcrt.dll", "strcat", strcat),
        ("msvcrt.dll", "strncat", strncat),
        // File delete (CRT); mirrors kernel32 DeleteFile against the VFS.
        ("msvcrt.dll", "remove", remove_fn),
        ("msvcrt.dll", "_unlink", remove_fn),
        ("msvcrt.dll", "unlink", remove_fn),
        ("msvcrt.dll", "_wremove", wremove_fn),
        ("msvcrt.dll", "_wunlink", wremove_fn),
        // Multibyte helpers. Process code page is Windows-1252 (SBCS).
        ("msvcrt.dll", "_ismbblead", ismbblead),
        ("msvcrt.dll", "_ismbbtrail", ismbbtrail),
        ("msvcrt.dll", "_mbclen", mbclen),
        ("msvcrt.dll", "_mbsinc", mbsinc),
        ("msvcrt.dll", "_getmbcp", getmbcp),
        ("msvcrt.dll", "_setmbcp", setmbcp),
        // x87 stack intrinsics: CPU treats x87 as no-ops, so these only keep
        // control flow alive (Wine also has soft-float paths for similar cases).
        ("msvcrt.dll", "_CIpow", ci_math_nop),
        ("msvcrt.dll", "_CIsin", ci_math_nop),
        ("msvcrt.dll", "_CIcos", ci_math_nop),
        ("msvcrt.dll", "_CItan", ci_math_nop),
        ("msvcrt.dll", "_CIsqrt", ci_math_nop),
        ("msvcrt.dll", "_CIlog", ci_math_nop),
        ("msvcrt.dll", "_CIexp", ci_math_nop),
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
        ("msvcrt.dll", "wcscmp", |c| wcs_compare(c, u32::MAX, false)),
        ("msvcrt.dll", "_wcsicmp", |c| wcs_compare(c, u32::MAX, true)),
        ("msvcrt.dll", "wcschr", wcschr_fn),
        ("msvcrt.dll", "wcsrchr", wcsrchr_fn),
        ("msvcrt.dll", "wcsstr", wcsstr_fn),
        ("msvcrt.dll", "wcsncmp", |c| {
            let n = c.arg(2);
            wcs_compare(c, n, false)
        }),
        ("msvcrt.dll", "_wcsnicmp", |c| {
            let n = c.arg(2);
            wcs_compare(c, n, true)
        }),
        ("msvcrt.dll", "towupper", |c| {
            let v = c.arg(0);
            c.ret_cdecl(
                char::from_u32(v)
                    .map(|ch| ch.to_ascii_uppercase() as u32)
                    .unwrap_or(v),
            );
            Handled::Ok
        }),
        ("msvcrt.dll", "towlower", |c| {
            let v = c.arg(0);
            c.ret_cdecl(
                char::from_u32(v)
                    .map(|ch| ch.to_ascii_lowercase() as u32)
                    .unwrap_or(v),
            );
            Handled::Ok
        }),
        ("msvcrt.dll", "iswalpha", |c| {
            let v = c.arg(0);
            c.ret_cdecl(
                char::from_u32(v)
                    .map(|ch| ch.is_alphabetic() as u32)
                    .unwrap_or(0),
            );
            Handled::Ok
        }),
        ("msvcrt.dll", "iswdigit", |c| {
            let v = c.arg(0);
            c.ret_cdecl(
                char::from_u32(v)
                    .map(|ch| ch.is_numeric() as u32)
                    .unwrap_or(0),
            );
            Handled::Ok
        }),
        ("msvcrt.dll", "iswspace", |c| {
            let v = c.arg(0);
            c.ret_cdecl(
                char::from_u32(v)
                    .map(|ch| ch.is_whitespace() as u32)
                    .unwrap_or(0),
            );
            Handled::Ok
        }),
        // time(t): seconds since the Unix epoch, from the shared virtual clock.
        // A frozen constant made srand(time(NULL)) reproduce one seed forever.
        ("msvcrt.dll", "time", |c| {
            let t = c.arg(0);
            let now = crate::kernel32::unix_time_secs();
            if t != 0 {
                let _ = c.memory.write_u32(t, now);
            }
            c.ret_cdecl(now);
            Handled::Ok
        }),
        // _time64(t): same value widened to 64 bits (returned in EDX:EAX).
        ("msvcrt.dll", "_time64", |c| {
            let t = c.arg(0);
            let now = crate::kernel32::unix_time_secs();
            if t != 0 {
                let _ = c.memory.write_u32(t, now);
                let _ = c.memory.write_u32(t + 4, 0);
            }
            c.cpu.edx = 0;
            c.ret_cdecl(now);
            Handled::Ok
        }),
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
        ("msvcrt.dll", "_open_osfhandle", open_osfhandle),
        ("msvcrt.dll", "_isatty", |c| {
            let fd = c.arg(0);
            c.ret_cdecl(if fd <= 2 { 1 } else { 0 });
            Handled::Ok
        }),
        ("msvcrt.dll", "_fileno", |c| {
            let s = c.arg(0);
            c.ret_cdecl(s);
            Handled::Ok
        }),
        // fd lifecycle (stdio redirection). We don't keep a real fd table, so
        // _dup returns the same fd, _dup2/_close report success.
        ("msvcrt.dll", "_dup", |c| {
            let fd = c.arg(0);
            c.ret_cdecl(fd);
            Handled::Ok
        }),
        ("msvcrt.dll", "_dup2", |c| {
            c.ret_cdecl(0);
            Handled::Ok
        }),
        ("msvcrt.dll", "_close", |c| {
            c.ret_cdecl(0);
            Handled::Ok
        }),
        ("msvcrt.dll", "atof", atof_fn),
        ("msvcrt.dll", "puts", puts),
        ("msvcrt.dll", "putchar", putchar),
        ("msvcrt.dll", "fflush", fflush_fn),
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
        ("msvcrt.dll", "__p___wargv", p_wargv),
        ("msvcrt.dll", "__p__acmdln", p_acmdln),
        ("msvcrt.dll", "__p__wcmdln", p_wcmdln),
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
        // scanf family: report EOF (-1). Returning 0 ("no fields") makes
        // `while (fscanf(..)!=EOF)` spin forever; EOF terminates cleanly.
        ("msvcrt.dll", "fscanf", scanf_eof),
        ("msvcrt.dll", "scanf", scanf_eof),
        ("msvcrt.dll", "sscanf", scanf_eof),
        ("msvcrt.dll", "vfscanf", scanf_eof),
        ("msvcrt.dll", "vsscanf", scanf_eof),
        ("msvcrt.dll", "ferror", ferror_fn),
        ("msvcrt.dll", "setvbuf", setvbuf_fn),
        ("msvcrt.dll", "setbuf", setbuf_fn),
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
                char::from_u32(wc)
                    .map(|ch| ch.to_uppercase().next().unwrap_or(ch) as u32)
                    .unwrap_or(wc)
            } else {
                wc
            };
            c.ret_cdecl(out);
            Handled::Ok
        }),
        ("msvcrt.dll", "towlower", |c| {
            let wc = c.arg(0) as u32;
            let out = if wc <= 0xFFFF {
                char::from_u32(wc)
                    .map(|ch| ch.to_lowercase().next().unwrap_or(ch) as u32)
                    .unwrap_or(wc)
            } else {
                wc
            };
            c.ret_cdecl(out);
            Handled::Ok
        }),
        ("msvcrt.dll", "iswalpha", |c| {
            let wc = c.arg(0) as u32;
            let r = char::from_u32(wc)
                .map(|ch| ch.is_alphabetic())
                .unwrap_or(false);
            c.ret_cdecl(if r { 1 } else { 0 });
            Handled::Ok
        }),
        ("msvcrt.dll", "iswdigit", |c| {
            let wc = c.arg(0) as u32;
            let r = char::from_u32(wc)
                .map(|ch| ch.is_ascii_digit())
                .unwrap_or(false);
            c.ret_cdecl(if r { 1 } else { 0 });
            Handled::Ok
        }),
        ("msvcrt.dll", "iswspace", |c| {
            let wc = c.arg(0) as u32;
            let r = char::from_u32(wc)
                .map(|ch| ch.is_whitespace())
                .unwrap_or(false);
            c.ret_cdecl(if r { 1 } else { 0 });
            Handled::Ok
        }),
        ("msvcrt.dll", "iswalnum", |c| {
            let wc = c.arg(0) as u32;
            let r = char::from_u32(wc)
                .map(|ch| ch.is_alphanumeric())
                .unwrap_or(false);
            c.ret_cdecl(if r { 1 } else { 0 });
            Handled::Ok
        }),
        // _wcsnicmp(s1, s2, count) â€” case-insensitive wide-string compare, cdecl, 3 args.
        ("msvcrt.dll", "_wcsnicmp", wcsnicmp_fn),
        ("msvcrt.dll", "wcsnicmp", wcsnicmp_fn),
        // C++ operator delete[] / new[] / delete (MSVC decorated names).
        ("msvcrt.dll", "??_V@YAXPAX@Z", free_fn),
        ("msvcrt.dll", "??2@YAPAXI@Z", malloc),
        ("msvcrt.dll", "??3@YAXPAX@Z", free_fn),
        // MSVC SEH helpers — frame-walking not modelled; safe no-ops that
        // preserve cdecl ABI so SEH tables still balance the stack.
        ("msvcrt.dll", "_local_unwind4", local_unwind4),
        ("msvcrt.dll", "__local_unwind4", local_unwind4),
        ("msvcrt.dll", "_global_unwind2", global_unwind2),
        ("msvcrt.dll", "__set_app_type", set_app_type),
        ("msvcrt.dll", "_set_app_type", set_app_type),
        ("msvcrt.dll", "_configure_narrow_argv", configure_argv),
        ("msvcrt.dll", "_configure_wide_argv", configure_argv),
        (
            "msvcrt.dll",
            "_initialize_narrow_environment",
            initialize_environment,
        ),
        (
            "msvcrt.dll",
            "_initialize_wide_environment",
            initialize_environment,
        ),
        (
            "msvcrt.dll",
            "_get_initial_wide_environment",
            get_initial_wide_environment,
        ),
        ("msvcrt.dll", "_set_fmode", set_fmode),
        ("msvcrt.dll", "_setmode", setmode_fn),
        ("msvcrt.dll", "_set_new_mode", set_new_mode),
        ("msvcrt.dll", "_configthreadlocale", configthreadlocale),
        ("msvcrt.dll", "setlocale", setlocale_fn),
        ("msvcrt.dll", "_wsetlocale", wsetlocale_fn),
        ("msvcrt.dll", "__p__commode", p_commode),
        ("msvcrt.dll", "__p__fmode", p_fmode),
        ("msvcrt.dll", "_crt_atexit", atexit_fn),
        ("msvcrt.dll", "atexit", atexit_fn),
        // _onexit/__onexit return the registered function pointer on success
        // (NULL means failure, which some CRTs treat as fatal).
        ("msvcrt.dll", "_onexit", onexit_fn),
        ("msvcrt.dll", "__onexit", onexit_fn),
        ("msvcrt.dll", "_lock", crt_lock),
        ("msvcrt.dll", "_unlock", crt_unlock),
        ("msvcrt.dll", "__lconv_init", lconv_init),
        ("msvcrt.dll", "_controlfp", controlfp),
        ("msvcrt.dll", "_controlfp_s", controlfp_s),
        ("msvcrt.dll", "__current_exception", current_exception),
        (
            "msvcrt.dll",
            "__current_exception_context",
            current_exception_context,
        ),
        ("msvcrt.dll", "_except_handler3", except_handler),
        ("msvcrt.dll", "_except_handler4_common", except_handler),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
    for (name, address) in [
        ("_acmdln", CRT_ACMDLN_SLOT),
        ("_wcmdln", CRT_WCMDLN_SLOT),
        ("__argc", CRT_ARGC_SLOT),
        ("__argv", CRT_ARGV_SLOT),
        ("__wargv", CRT_WARGV_SLOT),
        ("_environ", CRT_ENVIRON_SLOT),
        ("_wenviron", CRT_WENVIRON_SLOT),
        ("_fmode", CRT_FMODE_SLOT),
        ("_commode", CRT_COMMODE_SLOT),
    ] {
        r.add_data("msvcrt.dll", name, address);
    }
}

pub(crate) fn exit(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(ctx.arg(0))
}

pub(crate) fn malloc(ctx: &mut ApiContext) -> Handled {
    let size = ctx.arg(0);
    let ptr = ctx.heap_alloc(size);
    ctx.ret_cdecl(ptr);
    Handled::Ok
}

pub(crate) fn calloc(ctx: &mut ApiContext) -> Handled {
    let n = ctx.arg(0);
    let sz = ctx.arg(1);
    let total = n.saturating_mul(sz);
    let ptr = ctx.heap_alloc(total);
    // heap is already zeroed from allocate()
    ctx.ret_cdecl(ptr);
    Handled::Ok
}

pub(crate) fn realloc(ctx: &mut ApiContext) -> Handled {
    let old = ctx.arg(0);
    let size = ctx.arg(1);
    let ptr = ctx.heap_realloc(old, size);
    ctx.ret_cdecl(ptr);
    Handled::Ok
}

pub(crate) fn memcpy(ctx: &mut ApiContext) -> Handled {
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

pub(crate) fn memcpy_s(ctx: &mut ApiContext) -> Handled {
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

pub(crate) fn memset(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let val = ctx.arg(1) as u8;
    let n = ctx.arg(2) as usize;
    let buf = vec![val; n];
    let _ = ctx.memory.write_bytes(dst, &buf);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

pub(crate) fn memcmp(ctx: &mut ApiContext) -> Handled {
    let a = ctx.arg(0);
    let b = ctx.arg(1);
    let n = ctx.arg(2) as usize;
    let ba = ctx.memory.read_bytes(a, n).unwrap_or_default();
    let bb = ctx.memory.read_bytes(b, n).unwrap_or_default();
    let r = ba.cmp(&bb) as i32;
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

// strlen: count raw bytes up to the NUL. Going through `cstr` first decodes
// with from_utf8_lossy, which turns every byte >= 0x80 into a 3-byte U+FFFD and
// inflates the answer for any non-ASCII (CP1252) string.
pub(crate) fn strlen(ctx: &mut ApiContext) -> Handled {
    let mut n = 0u32;
    let p = ctx.arg(0);
    while ctx.memory.read_u8(p.wrapping_add(n)).unwrap_or(0) != 0 {
        n += 1;
    }
    ctx.ret_cdecl(n);
    Handled::Ok
}

// wcslen: count UTF-16 code units, not the UTF-8 length of the decoded string.
// The old version returned 3 for a single WCHAR like U+20AC, so callers sized
// buffers from it and overran them.
pub(crate) fn wcslen(ctx: &mut ApiContext) -> Handled {
    let mut n = 0u32;
    let p = ctx.arg(0);
    while ctx.memory.read_u16(p.wrapping_add(n * 2)).unwrap_or(0) != 0 {
        n += 1;
    }
    ctx.ret_cdecl(n);
    Handled::Ok
}

// wcscmp / wcsncmp / _wcsicmp / _wcsnicmp over raw UTF-16 units, comparing up
// to `limit` units. Decoding to a Rust String first replaced unpaired
// surrogates and made `.chars()` counts diverge from WCHAR counts.
pub(crate) fn wcs_compare(ctx: &mut ApiContext, limit: u32, fold: bool) -> Handled {
    let (pa, pb) = (ctx.arg(0), ctx.arg(1));
    let mut r = 0i32;
    for i in 0..limit {
        let mut a = ctx.memory.read_u16(pa.wrapping_add(i * 2)).unwrap_or(0);
        let mut b = ctx.memory.read_u16(pb.wrapping_add(i * 2)).unwrap_or(0);
        if fold {
            a = fold_case(a);
            b = fold_case(b);
        }
        if a != b {
            r = a as i32 - b as i32;
            break;
        }
        if a == 0 {
            break;
        }
    }
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

/// Lowercase a single UTF-16 unit. Covers ASCII, Latin-1, Latin Extended-A and
/// Greek/Cyrillic, the ranges the CRT's `towlower` actually folds for the
/// codepages we expose; anything else is left alone.
pub(crate) fn fold_case(c: u16) -> u16 {
    match c {
        0x41..=0x5A => c + 0x20,                              // A-Z
        0xC0..=0xD6 | 0xD8..=0xDE => c + 0x20,                // Latin-1
        0x100..=0x137 | 0x14A..=0x177 if c % 2 == 0 => c + 1, // Latin Ext-A pairs
        0x139..=0x148 | 0x179..=0x17E if c % 2 == 1 => c + 1,
        0x391..=0x3A1 | 0x3A3..=0x3AB => c + 0x20, // Greek
        0x410..=0x42F => c + 0x20,                 // Cyrillic
        0x400..=0x40F => c + 0x50,
        _ => c,
    }
}

pub(crate) fn strcmp(ctx: &mut ApiContext) -> Handled {
    let (pa, pb) = (ctx.arg(0), ctx.arg(1));
    let mut r = 0i32;
    for i in 0.. {
        let a = ctx.memory.read_u8(pa.wrapping_add(i)).unwrap_or(0);
        let b = ctx.memory.read_u8(pb.wrapping_add(i)).unwrap_or(0);
        if a != b {
            r = a as i32 - b as i32;
            break;
        }
        if a == 0 {
            break;
        }
    }
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

// strncmp: compare raw bytes. Slicing the decoded Strings at `n` panicked
// whenever `n` landed inside a multi-byte UTF-8 sequence (any CP1252 input),
// and compared replacement characters rather than the actual bytes.
pub(crate) fn strncmp(ctx: &mut ApiContext) -> Handled {
    let (pa, pb) = (ctx.arg(0), ctx.arg(1));
    let n = ctx.arg(2);
    let mut r = 0i32;
    for i in 0..n {
        let a = ctx.memory.read_u8(pa.wrapping_add(i)).unwrap_or(0);
        let b = ctx.memory.read_u8(pb.wrapping_add(i)).unwrap_or(0);
        if a != b {
            r = a as i32 - b as i32;
            break;
        }
        if a == 0 {
            break;
        }
    }
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

/// Case-insensitive strcmp (`_stricmp` / `stricmp` / `_strcmpi`).
/// MSVC folds A–Z only for the "C" locale we expose; ASCII is enough for games.
pub(crate) fn stricmp(ctx: &mut ApiContext) -> Handled {
    let (pa, pb) = (ctx.arg(0), ctx.arg(1));
    let mut r = 0i32;
    for i in 0.. {
        let a = ctx
            .memory
            .read_u8(pa.wrapping_add(i))
            .unwrap_or(0)
            .to_ascii_lowercase();
        let b = ctx
            .memory
            .read_u8(pb.wrapping_add(i))
            .unwrap_or(0)
            .to_ascii_lowercase();
        if a != b {
            r = a as i32 - b as i32;
            break;
        }
        if a == 0 {
            break;
        }
    }
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

/// Case-insensitive strncmp (`_strnicmp` / `strnicmp`).
pub(crate) fn strnicmp(ctx: &mut ApiContext) -> Handled {
    let (pa, pb) = (ctx.arg(0), ctx.arg(1));
    let n = ctx.arg(2);
    let mut r = 0i32;
    for i in 0..n {
        let a = ctx
            .memory
            .read_u8(pa.wrapping_add(i))
            .unwrap_or(0)
            .to_ascii_lowercase();
        let b = ctx
            .memory
            .read_u8(pb.wrapping_add(i))
            .unwrap_or(0)
            .to_ascii_lowercase();
        if a != b {
            r = a as i32 - b as i32;
            break;
        }
        if a == 0 {
            break;
        }
    }
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

/// `remove` / `_unlink` — delete a file from the guest VFS. 0 success, -1 fail.
pub(crate) fn remove_fn(ctx: &mut ApiContext) -> Handled {
    let path = ctx.cstr(ctx.arg(0));
    let full = ctx.resolve_path(&path);
    let ok = if full.is_empty() {
        false
    } else {
        ctx.fs.delete_node(&full).is_ok()
    };
    ctx.ret_cdecl(if ok { 0 } else { 0xFFFF_FFFF });
    Handled::Ok
}

/// `_wremove` / `_wunlink` — wide-path variant.
pub(crate) fn wremove_fn(ctx: &mut ApiContext) -> Handled {
    let path = ctx.wstr(ctx.arg(0));
    let full = ctx.resolve_path(&path);
    let ok = if full.is_empty() {
        false
    } else {
        ctx.fs.delete_node(&full).is_ok()
    };
    ctx.ret_cdecl(if ok { 0 } else { 0xFFFF_FFFF });
    Handled::Ok
}

pub(crate) fn strcpy(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let mut bytes = ctx.cstr_bytes(src);
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

// getenv(name): we have no environment, so every variable is unset (NULL).
pub(crate) fn getenv_fn(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(0);
    Handled::Ok
}

// _getcwd(buf, size): write the CWD; if buf is NULL, malloc one (size bytes).
pub(crate) fn getcwd_fn(ctx: &mut ApiContext) -> Handled {
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
pub(crate) fn chdir_fn(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.cstr(ctx.arg(0));
    *ctx.cwd = ctx.resolve_path(&raw);
    ctx.ret_cdecl(0);
    Handled::Ok
}

pub(crate) fn strncpy(ctx: &mut ApiContext) -> Handled {
    // strncpy(dst, src, n): copy at most n bytes; pad with NUL if src is shorter.
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let n = ctx.arg(2) as usize;
    let s = ctx.cstr_bytes(src);
    let mut buf = vec![0u8; n];
    let copy = s.len().min(n);
    buf[..copy].copy_from_slice(&s[..copy]);
    let _ = ctx.memory.write_bytes(dst, &buf);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

pub(crate) fn strncat(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let n = ctx.arg(2) as usize;
    let mut bytes = ctx.cstr_bytes(dst);
    let append = ctx.cstr_bytes(src);
    bytes.extend_from_slice(&append[..append.len().min(n)]);
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

pub(crate) fn strcat(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let src = ctx.arg(1);
    let mut bytes = ctx.cstr_bytes(dst);
    bytes.extend_from_slice(&ctx.cstr_bytes(src));
    bytes.push(0);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

// setjmp/_setjmp3(jmp_buf, ...): save callee-saved regs, esp and the return
// address into the buffer (MSVC _JUMP_BUFFER: Ebp, Ebx, Edi, Esi, Esp, Eip),
// then return 0. longjmp restores them to resume here. We don't model SEH
// unwinding (Registration/Cookie), which is fine for plain error-recovery jumps.
pub(crate) fn setjmp_fn(ctx: &mut ApiContext) -> Handled {
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
pub(crate) fn longjmp_fn(ctx: &mut ApiContext) -> Handled {
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

pub(crate) fn wcscpy_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let units = ctx.wstr_units(ctx.arg(1));
    let mut bytes: Vec<u8> = units.iter().flat_map(|c| c.to_le_bytes()).collect();
    bytes.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

pub(crate) fn wcsncpy_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let n = ctx.arg(2) as usize;
    let units = ctx.wstr_units(ctx.arg(1));
    let mut bytes: Vec<u8> = Vec::with_capacity(n * 2);
    for i in 0..n {
        let c = units.get(i).copied().unwrap_or(0); // NUL-pad if src shorter
        bytes.extend_from_slice(&c.to_le_bytes());
    }
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

pub(crate) fn wcscat_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let mut units = ctx.wstr_units(dst);
    units.extend_from_slice(&ctx.wstr_units(ctx.arg(1)));
    let mut bytes: Vec<u8> = units.iter().flat_map(|c| c.to_le_bytes()).collect();
    bytes.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    ctx.ret_cdecl(dst);
    Handled::Ok
}

pub(crate) fn wcschr_fn(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let needle = ctx.arg(1) as u16;
    let s = ctx.wstr_units(p);
    let pos = s.iter().position(|&c| c == needle);
    let r = pos.map(|i| p + (i as u32) * 2).unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

pub(crate) fn wcsrchr_fn(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let needle = ctx.arg(1) as u16;
    let s = ctx.wstr_units(p);
    let pos = s.iter().rposition(|&c| c == needle);
    let r = pos.map(|i| p + (i as u32) * 2).unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

pub(crate) fn wcsstr_fn(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let hay = ctx.wstr_units(p);
    let needle = ctx.wstr_units(ctx.arg(1));
    let r = if needle.is_empty() {
        p
    } else if needle.len() > hay.len() {
        0
    } else {
        hay.windows(needle.len())
            .position(|w| w == needle.as_slice())
            .map(|i| p + (i as u32) * 2)
            .unwrap_or(0)
    };
    ctx.ret_cdecl(r);
    Handled::Ok
}

pub(crate) fn strchr(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let c = ctx.arg(1) as u8;
    let s = ctx.cstr_bytes(p);
    let pos = s.iter().position(|&b| b == c);
    let r = pos.map(|i| p + i as u32).unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

pub(crate) fn strrchr(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let c = ctx.arg(1) as u8;
    let s = ctx.cstr_bytes(p);
    let pos = s.iter().rposition(|&b| b == c);
    let r = pos.map(|i| p + i as u32).unwrap_or(0);
    ctx.ret_cdecl(r);
    Handled::Ok
}

pub(crate) fn strstr(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let hay = ctx.cstr_bytes(p);
    let needle = ctx.cstr_bytes(ctx.arg(1));
    let r = if needle.is_empty() {
        p
    } else if needle.len() > hay.len() {
        0
    } else {
        hay.windows(needle.len())
            .position(|w| w == needle.as_slice())
            .map(|i| p + i as u32)
            .unwrap_or(0)
    };
    ctx.ret_cdecl(r);
    Handled::Ok
}

/// Shared strtol/strtoul/atoi scanner: skip leading whitespace, take an
/// optional sign, auto-detect 0x/0 when `base` is 0, then consume the longest
/// valid digit prefix. `Rust`'s `parse` rejects the whole string when there is
/// a trailing suffix, so "12abc" used to come back as 0 instead of 12.
fn scan_integer(bytes: &[u8], base: u32) -> (u64, bool, usize) {
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let negative = match bytes.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let mut base = base;
    if (base == 0 || base == 16)
        && bytes.get(i) == Some(&b'0')
        && matches!(bytes.get(i + 1), Some(b'x') | Some(b'X'))
    {
        base = 16;
        i += 2;
    } else if base == 0 {
        base = if bytes.get(i) == Some(&b'0') { 8 } else { 10 };
    }

    let digits_start = i;
    let mut value: u64 = 0;
    while let Some(d) = bytes.get(i).and_then(|c| (*c as char).to_digit(base)) {
        value = value.saturating_mul(base as u64).saturating_add(d as u64);
        i += 1;
    }
    // No digits consumed: nothing was converted, endptr stays at the start.
    if i == digits_start {
        return (0, negative, 0);
    }
    (value, negative, i)
}

/// strtol(nptr, endptr, base) / strtoul(...). Writes `*endptr` and saturates
/// like the CRT does (LONG_MIN/LONG_MAX, ULONG_MAX).
fn strto_common(ctx: &mut ApiContext, signed: bool) -> Handled {
    let p = ctx.arg(0);
    let endptr = ctx.arg(1);
    let base = ctx.arg(2);
    let bytes = ctx.cstr_bytes(p);
    let (value, negative, consumed) = scan_integer(&bytes, base);
    if endptr != 0 {
        let _ = ctx.memory.write_u32(endptr, p + consumed as u32);
    }
    let r = if signed {
        if negative {
            (value.min(0x8000_0000) as i64).wrapping_neg() as i32 as u32
        } else {
            value.min(0x7FFF_FFFF) as u32
        }
    } else {
        let v = value.min(0xFFFF_FFFF) as u32;
        if negative {
            v.wrapping_neg()
        } else {
            v
        }
    };
    ctx.ret_cdecl(r);
    Handled::Ok
}

pub(crate) fn strtol(ctx: &mut ApiContext) -> Handled {
    strto_common(ctx, true)
}

pub(crate) fn strtoul(ctx: &mut ApiContext) -> Handled {
    strto_common(ctx, false)
}

pub(crate) fn atoi(ctx: &mut ApiContext) -> Handled {
    let bytes = ctx.cstr_bytes(ctx.arg(0));
    let (value, negative, _) = scan_integer(&bytes, 10);
    let r = if negative {
        (value.min(0x8000_0000) as i64).wrapping_neg() as i32 as u32
    } else {
        value.min(0x7FFF_FFFF) as u32
    };
    ctx.ret_cdecl(r);
    Handled::Ok
}

// _itoa/_ltoa/_ultoa(value, char* str, int radix): write value in `radix` to str.
pub(crate) fn itoa_radix(ctx: &mut ApiContext, signed: bool) -> Handled {
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

pub(crate) fn to_radix(mut v: u64, radix: u32) -> String {
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

// _open_osfhandle(osfhandle, flags): inverse of _get_osfhandle â€” wrap a Win32
// HANDLE in a CRT fd. The std handles map back to fds 0/1/2; anything else gets
// a generic non-std fd (3), enough for apps that wrap a console handle for
// stdio redirection.
pub(crate) fn open_osfhandle(ctx: &mut ApiContext) -> Handled {
    let fd = match ctx.arg(0) {
        0xFFFF_FFF6 => 0,            // stdin
        0xFFFF_FFF5 => 1,            // stdout
        0xFFFF_FFF4 => 2,            // stderr
        0xFFFF_FFFF => -1i32 as u32, // INVALID_HANDLE_VALUE -> error
        _ => 3,
    };
    ctx.ret_cdecl(fd);
    Handled::Ok
}

// _get_osfhandle(fd): map CRT fd 0/1/2 to the std Win32 HANDLEs.
pub(crate) fn get_osfhandle(ctx: &mut ApiContext) -> Handled {
    let h = match ctx.arg(0) {
        0 => 0xFFFF_FFF6u32, // stdin
        1 => 0xFFFF_FFF5,    // stdout
        2 => 0xFFFF_FFF4,    // stderr
        _ => 0xFFFF_FFFF,    // INVALID_HANDLE_VALUE
    };
    ctx.ret_cdecl(h);
    Handled::Ok
}

pub(crate) fn puts(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let mut s = ctx.cstr(p);
    s.push('\n');
    ctx.console.stdout.extend_from_slice(s.as_bytes());
    ctx.ret_cdecl(0);
    Handled::Ok
}

pub(crate) fn putchar(ctx: &mut ApiContext) -> Handled {
    let c = ctx.arg(0) as u8;
    ctx.console.stdout.push(c);
    ctx.ret_cdecl(c as u32);
    Handled::Ok
}

pub(crate) fn printf(ctx: &mut ApiContext) -> Handled {
    let fmt_ptr = ctx.arg(0);
    let fmt = ctx.cstr(fmt_ptr);
    let result = format_string(ctx, &fmt, 1);
    let n = result.len();
    ctx.console.stdout.extend_from_slice(result.as_bytes());
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

pub(crate) fn fprintf(ctx: &mut ApiContext) -> Handled {
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

pub(crate) fn sprintf_fn(ctx: &mut ApiContext) -> Handled {
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

pub(crate) fn snprintf_fn(ctx: &mut ApiContext) -> Handled {
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
pub(crate) fn vprintf_fn(ctx: &mut ApiContext) -> Handled {
    let fmt = ctx.cstr(ctx.arg(0));
    let va = ctx.arg(1);
    let result = format_va(ctx, &fmt, va);
    let n = result.len();
    ctx.console.stdout.extend_from_slice(result.as_bytes());
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

// vfprintf(FILE*, fmt, va_list)
pub(crate) fn vfprintf_fn(ctx: &mut ApiContext) -> Handled {
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
pub(crate) fn vsprintf_fn(ctx: &mut ApiContext) -> Handled {
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
pub(crate) fn vsnprintf_fn(ctx: &mut ApiContext) -> Handled {
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
pub(crate) fn strdup_fn(ctx: &mut ApiContext) -> Handled {
    let mut bytes = ctx.cstr_bytes(ctx.arg(0));
    bytes.push(0);
    let p = ctx.heap_alloc(bytes.len() as u32);
    let _ = ctx.memory.write_bytes(p, &bytes);
    ctx.ret_cdecl(p);
    Handled::Ok
}

// Wide formatting shared core. Reads a wide format string and produces wide
// output. %s is a wide string arg (wprintf convention); %d/%u/%x/%c handled.
pub(crate) fn format_wide(ctx: &ApiContext, fmt: &str, mut src: ArgSrc) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            let mut b = [0u16; 2];
            for u in chars[i].encode_utf16(&mut b) {
                out.push(*u);
            }
            i += 1;
            continue;
        }
        i += 1;
        while i < chars.len() && "0123456789-+ #.*lh".contains(chars[i]) {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
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
            's' => {
                let ptr = src.next(&ctx.memory);
                out.extend(ctx.memory.read_wstr(ptr).encode_utf16());
            }
            'S' => {
                let ptr = src.next(&ctx.memory);
                push_str(&mut out, &ctx.memory.read_cstr(ptr));
            }
            '%' => out.push(b'%' as u16),
            _ => {
                out.push(b'%' as u16);
                out.push(spec as u16);
            }
        }
    }
    out
}

pub(crate) fn write_wide(ctx: &mut ApiContext, dst: u32, cap: usize, units: &[u16]) -> u32 {
    let n = if cap > 0 {
        units.len().min(cap - 1)
    } else {
        units.len()
    };
    let mut bytes: Vec<u8> = units[..n].iter().flat_map(|u| u.to_le_bytes()).collect();
    bytes.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(dst, &bytes);
    n as u32
}

// _snwprintf(buf, count, fmt, ...)
pub(crate) fn snwprintf_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let cap = ctx.arg(1) as usize;
    let fmt = ctx.wstr(ctx.arg(2));
    let units = format_wide(
        ctx,
        &fmt,
        ArgSrc::Stack {
            esp: ctx.cpu.esp,
            idx: 3,
        },
    );
    let n = write_wide(ctx, dst, cap, &units);
    ctx.ret_cdecl(n);
    Handled::Ok
}

// swprintf(buf, fmt, ...) â€” no count argument.
pub(crate) fn snwprintf_no_count_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let fmt = ctx.wstr(ctx.arg(1));
    let units = format_wide(
        ctx,
        &fmt,
        ArgSrc::Stack {
            esp: ctx.cpu.esp,
            idx: 2,
        },
    );
    let n = write_wide(ctx, dst, 0, &units);
    ctx.ret_cdecl(n);
    Handled::Ok
}

// _vsnwprintf(buf, count, fmt, va_list)
pub(crate) fn vsnwprintf_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let cap = ctx.arg(1) as usize;
    let fmt = ctx.wstr(ctx.arg(2));
    let va = ctx.arg(3);
    let units = format_wide(ctx, &fmt, ArgSrc::Va { ptr: va, idx: 0 });
    let n = write_wide(ctx, dst, cap, &units);
    ctx.ret_cdecl(n);
    Handled::Ok
}

// vswprintf(buf, fmt, va_list) â€” no count argument.
pub(crate) fn vsnwprintf_no_count_fn(ctx: &mut ApiContext) -> Handled {
    let dst = ctx.arg(0);
    let fmt = ctx.wstr(ctx.arg(1));
    let va = ctx.arg(2);
    let units = format_wide(ctx, &fmt, ArgSrc::Va { ptr: va, idx: 0 });
    let n = write_wide(ctx, dst, 0, &units);
    ctx.ret_cdecl(n);
    Handled::Ok
}

pub(crate) fn stdio_vfprintf(ctx: &mut ApiContext) -> Handled {
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
pub(crate) fn stdio_vswprintf(ctx: &mut ApiContext) -> Handled {
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
pub(crate) fn stdio_vsprintf(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(2);
    let count = ctx.arg(3) as usize;
    let fmt = ctx.cstr(ctx.arg(4));
    let va = ctx.arg(6);
    let result = format_va(ctx, &fmt, va);
    let n = if count > 0 {
        result.len().min(count - 1)
    } else {
        result.len()
    };
    let mut bytes = result.into_bytes();
    bytes.truncate(n);
    bytes.push(0);
    let _ = ctx.memory.write_bytes(buf, &bytes);
    ctx.ret_cdecl(n as u32);
    Handled::Ok
}

pub(crate) fn fwrite(ctx: &mut ApiContext) -> Handled {
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

pub(crate) fn fputc(ctx: &mut ApiContext) -> Handled {
    // fputc(c, FILE*)
    let c = ctx.arg(0) as u8;
    let stream = ctx.arg(1);
    write_stream(ctx, stream, &[c]);
    ctx.ret_cdecl(c as u32);
    Handled::Ok
}

pub(crate) fn fputs(ctx: &mut ApiContext) -> Handled {
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

pub(crate) fn is_vfs_stream(stream: u32) -> bool {
    stream != 0 && stream < 0x7FFD_0000
}

pub(crate) fn write_stream(ctx: &mut ApiContext, stream: u32, bytes: &[u8]) {
    use webwine_api::vm::handles::KernelObject;
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
pub(crate) fn fopen(ctx: &mut ApiContext) -> Handled {
    let path = ctx.cstr(ctx.arg(0));
    let mode = ctx.cstr(ctx.arg(1));
    let h = open_vfs(ctx, &path, &mode);
    ctx.ret_cdecl(h);
    Handled::Ok
}

// _fsopen(path, mode, shflag) -> FILE*
pub(crate) fn fsopen(ctx: &mut ApiContext) -> Handled {
    let path = ctx.cstr(ctx.arg(0));
    let mode = ctx.cstr(ctx.arg(1));
    let h = open_vfs(ctx, &path, &mode);
    ctx.ret_cdecl(h);
    Handled::Ok
}

// freopen(path, mode, stream) -> stream (or NULL). We open/truncate the file but
// keep redirecting the original stream's writes to the console, so program log
// output stays visible rather than vanishing into a file.
pub(crate) fn freopen(ctx: &mut ApiContext) -> Handled {
    let path = ctx.cstr(ctx.arg(0));
    let mode = ctx.cstr(ctx.arg(1));
    let stream = ctx.arg(2);
    open_vfs(ctx, &path, &mode);
    ctx.ret_cdecl(stream);
    Handled::Ok
}

pub(crate) fn open_vfs(ctx: &mut ApiContext, raw_path: &str, mode: &str) -> u32 {
    use webwine_api::vm::handles::KernelObject;
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

pub(crate) fn fclose(ctx: &mut ApiContext) -> Handled {
    let stream = ctx.arg(0);
    if is_vfs_stream(stream) {
        ctx.handles.remove(stream);
    }
    ctx.ret_cdecl(0);
    Handled::Ok
}

// fread(buf, size, count, FILE*) -> count of full elements read
pub(crate) fn fread(ctx: &mut ApiContext) -> Handled {
    use webwine_api::vm::handles::KernelObject;
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
    let chunk = ctx
        .fs
        .read_range(&path, cursor as usize, total)
        .unwrap_or_default();
    let read_bytes = chunk.len();
    let _ = ctx.memory.write_bytes(buf, &chunk);
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(stream) {
        *cursor += read_bytes as u64;
    }
    ctx.ret_cdecl((read_bytes / size as usize) as u32);
    Handled::Ok
}

// fseek(FILE*, offset, origin): 0=SEEK_SET, 1=SEEK_CUR, 2=SEEK_END
pub(crate) fn fseek(ctx: &mut ApiContext) -> Handled {
    use webwine_api::vm::handles::KernelObject;
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

pub(crate) fn ftell(ctx: &mut ApiContext) -> Handled {
    use webwine_api::vm::handles::KernelObject;
    let stream = ctx.arg(0);
    let pos = match ctx.handles.get(stream) {
        Some(KernelObject::VfsFile { cursor, .. }) if is_vfs_stream(stream) => *cursor as u32,
        _ => 0xFFFF_FFFF,
    };
    ctx.ret_cdecl(pos);
    Handled::Ok
}

pub(crate) fn rewind(ctx: &mut ApiContext) -> Handled {
    use webwine_api::vm::handles::KernelObject;
    let stream = ctx.arg(0);
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(stream) {
        *cursor = 0;
    }
    ctx.ret_cdecl(0);
    Handled::Ok
}

// fgetc(FILE*) -> int (byte or EOF=-1)
pub(crate) fn fgetc(ctx: &mut ApiContext) -> Handled {
    use webwine_api::vm::handles::KernelObject;
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
    let byte = ctx
        .fs
        .read_range(&path, cursor as usize, 1)
        .ok()
        .and_then(|b| b.first().copied());
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
pub(crate) fn fgets(ctx: &mut ApiContext) -> Handled {
    use webwine_api::vm::handles::KernelObject;
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
    let window = ctx
        .fs
        .read_range(&path, cursor as usize, n - 1)
        .unwrap_or_default();
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

pub(crate) fn feof(ctx: &mut ApiContext) -> Handled {
    use webwine_api::vm::handles::KernelObject;
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
pub(crate) fn scanf_eof(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(0xFFFF_FFFF);
    Handled::Ok
}

pub(crate) fn ret_class(ctx: &mut ApiContext, pred: impl Fn(u8) -> bool) -> Handled {
    let v = ctx.arg(0) as u8;
    ctx.ret_cdecl(if pred(v) { 1 } else { 0 });
    Handled::Ok
}

pub(crate) fn acrt_iob(ctx: &mut ApiContext) -> Handled {
    // Return a fake FILE* based on the fd (0=stdin, 1=stdout, 2=stderr)
    let fd = ctx.arg(0);
    let fake: u32 = 0x7FFD_F400 + fd * 0x20;
    ctx.ret_cdecl(fake);
    Handled::Ok
}

// _initterm(first, last): call each non-null fn pointer in [first, last).
// We collect the pointers and hand them to the executor, which actually
// runs them (a handler can't call guest code itself).
pub(crate) fn initterm(ctx: &mut ApiContext) -> Handled {
    let first = ctx.arg(0);
    let last = ctx.arg(1);
    Handled::CallChain(collect_init_table(ctx, first, last))
}

pub(crate) fn initterm_e(ctx: &mut ApiContext) -> Handled {
    let first = ctx.arg(0);
    let last = ctx.arg(1);
    Handled::CallChainE(collect_init_table(ctx, first, last))
}

pub(crate) fn collect_init_table(ctx: &ApiContext, first: u32, last: u32) -> Vec<u32> {
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

pub(crate) fn p_argc(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(CRT_ARGC_SLOT);
    Handled::Ok
}

pub(crate) fn p_argv(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(CRT_ARGV_SLOT);
    Handled::Ok
}

pub(crate) fn p_wargv(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(CRT_WARGV_SLOT);
    Handled::Ok
}

// Split a Windows command line into argv, respecting double-quoted segments.
pub(crate) fn tokenize_cmdline(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut started = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                started = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if started {
                    args.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(cur);
    }
    if args.is_empty() {
        args.push(String::new());
    }
    args
}

/// Materialize the MSVCRT data imports for a newly loaded native process.
/// Function imports are routed through API trampolines, but variables such as
/// `_wcmdln` are dereferenced directly from the IAT before any handler can run.
pub fn initialize_process_data(
    memory: &mut webwine_api::vm::memory::GuestMemory,
    cmdline: &str,
) -> webwine_api::error::Result<()> {
    use webwine_api::vm::memory::PageProt;

    memory.allocate(CRT_DATA_BASE, CRT_DATA_SIZE, PageProt::RW)?;
    let mut cursor = CRT_DATA_BASE;

    let mut narrow_cmd = cmdline.as_bytes().to_vec();
    narrow_cmd.push(0);
    let acmdln = cursor;
    memory.write_bytes(cursor, &narrow_cmd)?;
    cursor += narrow_cmd.len() as u32;

    cursor = (cursor + 3) & !3;
    let mut wide_cmd: Vec<u8> = cmdline
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    wide_cmd.extend_from_slice(&[0, 0]);
    let wcmdln = cursor;
    memory.write_bytes(cursor, &wide_cmd)?;
    cursor += wide_cmd.len() as u32;

    let args = tokenize_cmdline(cmdline);
    let argc = args.len() as u32;
    cursor = (cursor + 3) & !3;
    let argv = cursor;
    cursor += (argc + 1) * 4;
    let wargv = cursor;
    cursor += (argc + 1) * 4;
    let environ = cursor;
    cursor += 4;
    let wenviron = cursor;
    cursor += 4;

    for (i, arg) in args.iter().enumerate() {
        let mut narrow = arg.as_bytes().to_vec();
        narrow.push(0);
        let arg_va = cursor;
        memory.write_bytes(cursor, &narrow)?;
        cursor += narrow.len() as u32;
        memory.write_u32(argv + i as u32 * 4, arg_va)?;

        cursor = (cursor + 1) & !1;
        let mut wide: Vec<u8> = arg
            .encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        wide.extend_from_slice(&[0, 0]);
        let arg_wva = cursor;
        memory.write_bytes(cursor, &wide)?;
        cursor += wide.len() as u32;
        memory.write_u32(wargv + i as u32 * 4, arg_wva)?;
    }
    if cursor > CRT_DATA_BASE + CRT_DATA_SIZE {
        return Err(webwine_api::error::VmError::Internal(
            "command line exceeds CRT data region".into(),
        ));
    }

    memory.write_u32(argv + argc * 4, 0)?;
    memory.write_u32(wargv + argc * 4, 0)?;
    memory.write_u32(environ, 0)?;
    memory.write_u32(wenviron, 0)?;
    memory.write_u32(CRT_ACMDLN_SLOT, acmdln)?;
    memory.write_u32(CRT_WCMDLN_SLOT, wcmdln)?;
    memory.write_u32(CRT_ARGC_SLOT, argc)?;
    memory.write_u32(CRT_ARGV_SLOT, argv)?;
    memory.write_u32(CRT_WARGV_SLOT, wargv)?;
    memory.write_u32(CRT_ENVIRON_SLOT, environ)?;
    memory.write_u32(CRT_WENVIRON_SLOT, wenviron)?;
    memory.write_u32(CRT_FMODE_SLOT, 0)?;
    memory.write_u32(CRT_COMMODE_SLOT, 0)?;
    Ok(())
}

// __getmainargs(int* argc, char*** argv, char*** env, int wild, _startupinfo*)
// Fills argc/argv/env so the CRT can call main(argc, argv, env). cdecl.
pub(crate) fn getmainargs(ctx: &mut ApiContext) -> Handled {
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

    if argc_p != 0 {
        let _ = ctx.memory.write_u32(argc_p, argc);
    }
    if argv_p != 0 {
        let _ = ctx.memory.write_u32(argv_p, argv);
    }
    if env_p != 0 {
        let _ = ctx.memory.write_u32(env_p, env);
    }
    let _ = ctx.memory.write_u32(CRT_ARGC_SLOT, argc);
    let _ = ctx.memory.write_u32(CRT_ARGV_SLOT, argv);
    let _ = ctx.memory.write_u32(CRT_ENVIRON_SLOT, env);
    ctx.ret_cdecl(0);
    Handled::Ok
}

// Wide variant: argv/env are wchar_t**.
pub(crate) fn wgetmainargs(ctx: &mut ApiContext) -> Handled {
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

    if argc_p != 0 {
        let _ = ctx.memory.write_u32(argc_p, argc);
    }
    if argv_p != 0 {
        let _ = ctx.memory.write_u32(argv_p, argv);
    }
    if env_p != 0 {
        let _ = ctx.memory.write_u32(env_p, env);
    }
    let _ = ctx.memory.write_u32(CRT_ARGC_SLOT, argc);
    let _ = ctx.memory.write_u32(CRT_WARGV_SLOT, argv);
    let _ = ctx.memory.write_u32(CRT_WENVIRON_SLOT, env);
    ctx.ret_cdecl(0);
    Handled::Ok
}

pub(crate) fn p_commode(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(CRT_COMMODE_SLOT);
    Handled::Ok
}

// __p__acmdln() -> char**: pointer to the `_acmdln` global (a char* holding the
// raw command line). MinGW's CRT startup reads `*__p__acmdln()` to get the
// command line, so returning 0 here makes it dereference NULL and crash. We keep
// the char* in a fixed scratch slot and point it at a fresh cmdline buffer.
pub(crate) fn p_acmdln(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(CRT_ACMDLN_SLOT);
    Handled::Ok
}

// __p__wcmdln() -> wchar_t**: wide-char variant of the above.
pub(crate) fn p_wcmdln(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(CRT_WCMDLN_SLOT);
    Handled::Ok
}

pub(crate) fn p_fmode(ctx: &mut ApiContext) -> Handled {
    ctx.ret_cdecl(CRT_FMODE_SLOT);
    Handled::Ok
}

// printf formatter

// Source of variadic arguments for the printf family. Stack-based functions
// (printf, fprintf) read successive 4-byte slots above ESP; the v* variants read
// from a va_list pointer into guest memory.
pub(crate) enum ArgSrc {
    Stack { esp: u32, idx: u32 },
    Va { ptr: u32, idx: u32 },
}

impl ArgSrc {
    fn next(&mut self, mem: &webwine_api::vm::memory::GuestMemory) -> u32 {
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

pub(crate) fn format_string(ctx: &ApiContext, fmt: &str, first_arg: u32) -> String {
    format_args_src(
        ctx,
        fmt,
        ArgSrc::Stack {
            esp: ctx.cpu.esp,
            idx: first_arg,
        },
    )
}

pub(crate) fn format_va(ctx: &ApiContext, fmt: &str, va_ptr: u32) -> String {
    format_args_src(
        ctx,
        fmt,
        ArgSrc::Va {
            ptr: va_ptr,
            idx: 0,
        },
    )
}

pub(crate) fn format_args_src(ctx: &ApiContext, fmt: &str, mut src: ArgSrc) -> String {
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

// CRT lifecycle / locale / SEH (named implementations)

/// free(ptr): return the block to the process free list (coalescing).
pub(crate) fn free_fn(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    c.heap_free_block(p);
    c.ret_cdecl(0);
    Handled::Ok
}

/// _cexit: run atexit handlers conceptually, then return (no process exit).
fn cexit(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

/// _ismbblead: CP1252 is SBCS → never a lead byte.
fn ismbblead(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

/// _ismbbtrail: CP1252 is SBCS → never a trail byte.
fn ismbbtrail(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn mbclen(c: &mut ApiContext) -> Handled {
    // _mbclen(c): length of multibyte char at *c. SBCS → always 1 (or 0 if NUL).
    let p = c.arg(0);
    let b = if p != 0 {
        c.memory.read_u8(p).unwrap_or(0)
    } else {
        0
    };
    c.ret_cdecl(if b == 0 { 0 } else { 1 });
    Handled::Ok
}

fn mbsinc(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(c.arg(0).wrapping_add(1));
    Handled::Ok
}

fn getmbcp(c: &mut ApiContext) -> Handled {
    let cp = c.dll_state.get("msvcrt.mbcp").copied().unwrap_or(1252);
    c.ret_cdecl(cp);
    Handled::Ok
}

fn setmbcp(c: &mut ApiContext) -> Handled {
    // _setmbcp(codepage) → previous codepage, or -1 on failure.
    let cp = c.arg(0);
    let prev = c.dll_state.get("msvcrt.mbcp").copied().unwrap_or(1252);
    // Accept 1252 and -1 (restore); other IDs still stored for apps that probe.
    if cp == 0xFFFF_FFFF {
        c.dll_state.insert("msvcrt.mbcp".into(), 1252);
    } else {
        c.dll_state.insert("msvcrt.mbcp".into(), cp);
    }
    c.ret_cdecl(prev);
    Handled::Ok
}

fn ci_math_nop(c: &mut ApiContext) -> Handled {
    // Operands/results live on the x87 stack; nothing to do.
    c.ret_cdecl(0);
    Handled::Ok
}

/// atof: parse a C string to f64, return via x87 ST(0) is ideal; without x87 we
/// return 0 in EAX (Wine soft-float still converts). Callers rarely use EAX.
fn atof_fn(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    let s = if p != 0 { c.cstr(p) } else { String::new() };
    let _val: f64 = s.trim().parse().unwrap_or(0.0);
    // No x87 store path yet; return 0 integer bits. Parsed value is discarded
    // until the FPU model is wired (same class of limitation as _CI*).
    c.ret_cdecl(0);
    Handled::Ok
}

fn fflush_fn(c: &mut ApiContext) -> Handled {
    // fflush(stream): success. Console/VFS writes are already committed.
    c.ret_cdecl(0);
    Handled::Ok
}

fn ferror_fn(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn setvbuf_fn(c: &mut ApiContext) -> Handled {
    // setvbuf(stream, buf, mode, size) → 0 on success. We ignore buffering.
    c.ret_cdecl(0);
    Handled::Ok
}

fn setbuf_fn(c: &mut ApiContext) -> Handled {
    // setbuf(stream, buf): void
    c.ret_cdecl(0);
    Handled::Ok
}

fn local_unwind4(c: &mut ApiContext) -> Handled {
    // _local_unwind4(cookie, funcinfo, target_level) — no frame walk.
    c.ret_cdecl(0);
    Handled::Ok
}

fn global_unwind2(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn set_app_type(c: &mut ApiContext) -> Handled {
    // __set_app_type(type): record console vs GUI for CRT diagnostics.
    let t = c.arg(0);
    c.dll_state.insert("msvcrt.app_type".into(), t);
    c.ret_cdecl(0);
    Handled::Ok
}

fn configure_argv(c: &mut ApiContext) -> Handled {
    // _configure_narrow_argv / _configure_wide_argv → 0 (success).
    c.ret_cdecl(0);
    Handled::Ok
}

fn initialize_environment(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn get_initial_wide_environment(c: &mut ApiContext) -> Handled {
    // Returns wchar_t**; point at the CRT_WENVIRON slot contents.
    c.ret_cdecl(CRT_WENVIRON_SLOT);
    Handled::Ok
}

fn set_fmode(c: &mut ApiContext) -> Handled {
    // _set_fmode(mode) → 0; store into CRT fmode slot when possible.
    let mode = c.arg(0);
    let _ = c.memory.write_u32(CRT_FMODE_SLOT, mode);
    c.ret_cdecl(0);
    Handled::Ok
}

fn setmode_fn(c: &mut ApiContext) -> Handled {
    // _setmode(fd, mode) → previous mode (O_TEXT=0x4000 typical).
    c.ret_cdecl(0x4000);
    Handled::Ok
}

fn set_new_mode(c: &mut ApiContext) -> Handled {
    let mode = c.arg(0);
    let prev = c.dll_state.get("msvcrt.new_mode").copied().unwrap_or(0);
    c.dll_state.insert("msvcrt.new_mode".into(), mode);
    c.ret_cdecl(prev);
    Handled::Ok
}

fn configthreadlocale(c: &mut ApiContext) -> Handled {
    // _configthreadlocale(type) → previous setting.
    let t = c.arg(0);
    let prev = c.dll_state.get("msvcrt.threadlocale").copied().unwrap_or(0);
    if t != 0xFFFF_FFFF {
        // -1 means query only
        c.dll_state.insert("msvcrt.threadlocale".into(), t);
    }
    c.ret_cdecl(prev);
    Handled::Ok
}

fn setlocale_fn(c: &mut ApiContext) -> Handled {
    // setlocale(category, locale) → pointer to locale string or NULL.
    let locale = c.arg(1);
    // Return a stable "C" string in the CRT data page.
    let c_locale = ensure_c_locale_string(c);
    if locale == 0 {
        // Query only.
        c.ret_cdecl(c_locale);
        return Handled::Ok;
    }
    // Accept any non-null as "C".
    c.ret_cdecl(c_locale);
    Handled::Ok
}

fn wsetlocale_fn(c: &mut ApiContext) -> Handled {
    let locale = c.arg(1);
    let w_locale = ensure_c_locale_wstring(c);
    if locale == 0 {
        c.ret_cdecl(w_locale);
        return Handled::Ok;
    }
    c.ret_cdecl(w_locale);
    Handled::Ok
}

fn ensure_c_locale_string(c: &mut ApiContext) -> u32 {
    const SLOT: u32 = CRT_DATA_BASE + 0x100;
    // "C\0"
    let _ = c.memory.ensure_mapped(SLOT, SLOT + 4);
    let _ = c.memory.write_bytes(SLOT, b"C\0");
    SLOT
}

fn ensure_c_locale_wstring(c: &mut ApiContext) -> u32 {
    const SLOT: u32 = CRT_DATA_BASE + 0x110;
    let _ = c.memory.ensure_mapped(SLOT, SLOT + 4);
    let _ = c.memory.write_u16(SLOT, b'C' as u16);
    let _ = c.memory.write_u16(SLOT + 2, 0);
    SLOT
}

fn atexit_fn(c: &mut ApiContext) -> Handled {
    // atexit(func) → 0 on success. We accept and ignore (no exit-run list yet).
    let f = c.arg(0);
    if f != 0 {
        let n = c.dll_state.entry("msvcrt.atexit_n".into()).or_insert(0);
        *n = n.wrapping_add(1);
    }
    c.ret_cdecl(0);
    Handled::Ok
}

fn onexit_fn(c: &mut ApiContext) -> Handled {
    let f = c.arg(0);
    c.ret_cdecl(f);
    Handled::Ok
}

fn crt_lock(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn crt_unlock(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn lconv_init(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn controlfp(c: &mut ApiContext) -> Handled {
    // _controlfp(new, mask) → previous CW. Default MSVC precision/mask.
    let new = c.arg(0);
    let mask = c.arg(1);
    let prev = c
        .dll_state
        .get("msvcrt.controlfp")
        .copied()
        .unwrap_or(0x0008_0001); // _PC_53 | _RC_NEAR-ish default
    let updated = (prev & !mask) | (new & mask);
    c.dll_state.insert("msvcrt.controlfp".into(), updated);
    c.ret_cdecl(prev);
    Handled::Ok
}

fn controlfp_s(c: &mut ApiContext) -> Handled {
    // errno_t _controlfp_s(current, new, mask)
    let cur = c.arg(0);
    let new = c.arg(1);
    let mask = c.arg(2);
    let prev = c
        .dll_state
        .get("msvcrt.controlfp")
        .copied()
        .unwrap_or(0x0008_0001);
    let updated = (prev & !mask) | (new & mask);
    c.dll_state.insert("msvcrt.controlfp".into(), updated);
    if cur != 0 {
        let _ = c.memory.write_u32(cur, updated);
    }
    c.ret_cdecl(0);
    Handled::Ok
}

fn current_exception(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn current_exception_context(c: &mut ApiContext) -> Handled {
    c.ret_cdecl(0);
    Handled::Ok
}

fn except_handler(c: &mut ApiContext) -> Handled {
    // ExceptionContinueSearch = 1
    c.ret_cdecl(1);
    Handled::Ok
}

/// Public alias for vcruntime140 / SEH imports (cdecl, ignores args).
pub(crate) fn except_handler_cdecl_1(c: &mut ApiContext) -> Handled {
    except_handler(c)
}

/// _wcsnicmp(s1, s2, count) â€” case-insensitive wide-string comparison, cdecl, 3 args.
/// Returns negative/0/positive like strcmp.
pub(crate) fn wcsnicmp_fn(ctx: &mut ApiContext) -> Handled {
    let p1 = ctx.arg(0);
    let p2 = ctx.arg(1);
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
        units
            .iter()
            .flat_map(|&u| {
                char::from_u32(u as u32)
                    .map(|ch| {
                        ch.to_uppercase()
                            .flat_map(|c| c.encode_utf16(&mut [0u16; 2]).to_vec())
                            .collect::<Vec<u16>>()
                    })
                    .unwrap_or_else(|| vec![u])
            })
            .collect()
    };
    let fa = fold(&a);
    let fb = fold(&b);
    let r = fa.cmp(&fb) as i32;
    ctx.ret_cdecl(r as u32);
    Handled::Ok
}

#[cfg(test)]
mod tests {
    use super::scan_integer;

    #[test]
    fn strtol_consumes_the_longest_valid_prefix() {
        // The old `parse()` implementation rejected any trailing suffix and
        // returned 0 for all of these.
        assert_eq!(scan_integer(b"12abc", 10), (12, false, 2));
        assert_eq!(scan_integer(b"  -42xyz", 10), (42, true, 5));
        assert_eq!(scan_integer(b"7", 10), (7, false, 1));
    }

    #[test]
    fn strtol_honours_the_base_argument() {
        assert_eq!(scan_integer(b"ff", 16), (255, false, 2));
        assert_eq!(scan_integer(b"0x1F", 16), (31, false, 4));
        assert_eq!(scan_integer(b"0x1F", 0), (31, false, 4)); // auto-detect
        assert_eq!(scan_integer(b"017", 0), (15, false, 3)); // octal
        assert_eq!(scan_integer(b"101", 2), (5, false, 3));
    }

    #[test]
    fn strtol_reports_no_conversion() {
        // endptr must stay at the start when nothing was converted.
        assert_eq!(scan_integer(b"abc", 10), (0, false, 0));
        assert_eq!(scan_integer(b"", 10), (0, false, 0));
    }
}
