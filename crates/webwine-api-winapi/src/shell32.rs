use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        // â”€â”€ MFC42U.DLL Stubs â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // Paint (mspaint.exe) imports ordinal 1165 from MFC42U.DLL.
        // It's likely AfxGetModuleState() or AfxGetApp() which returns a pointer
        // to a large CWinApp / AFX_MODULE_STATE structure.
        // Returning 0 crashes because it tries to write to [EAX+0x14].
        // We return a dummy heap pointer so the writes succeed.
        ("shell32.dll", "#68", |c| {
            c.ret_stdcall(0, 6);
            Handled::Ok
        }), // RunFileDlg (6 args)
        ("shell32.dll", "#34", crate::kernel32::path_find_file_name_w),
        ("shell32.dll", "#52", |c| path_get_args_w(c)),
        ("shell32.dll", "#71", |c| {
            if c.arg(0) != 0 { let _ = c.memory.write_u32(c.arg(0), 0x494C_0001); }
            if c.arg(1) != 0 { let _ = c.memory.write_u32(c.arg(1), 0x494C_0002); }
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("shell32.dll", "#72", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("shell32.dll", "#89", |c| {
            let pidl = c.heap_alloc(2);
            let _ = c.memory.write_u16(pidl, 0);
            c.ret_stdcall(pidl, 3);
            Handled::Ok
        }),
        ("shell32.dll", "#155", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shell32.dll", "#165", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("shell32.dll", "#181", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("shell32.dll", "#188", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }), // SHGetSetSettings (3 args, void)
        ("shell32.dll", "#100", |c| {
            let out = c.arg(2);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0x8000_4005, 3);
            Handled::Ok
        }), // SHCreateStdEnumFmtEtc (3 args)
        ("shell32.dll", "#245", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }), // SHTestTokenMembership (2 args)
        ("shell32.dll", "#660", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }), // SHWaitForFileToOpen (3 args)
        ("shell32.dll", "#723", |c| {
            let out = c.arg(1);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        // Private Windows 7 shell bootstrap selector. Explorer passes one mode
        // value and ignores the result; preserving its stdcall ABI is what
        // matters here, since a generic fallback can desynchronise ESP.
        ("shell32.dll", "#899", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        (
            "shell32.dll",
            "SHGetSpecialFolderPathW",
            crate::kernel32::sh_get_special_folder_path_w,
        ),
        (
            "shell32.dll",
            "SHGetSpecialFolderPathA",
            crate::kernel32::sh_get_special_folder_path_a,
        ),
        // shell32 â€” return > 32 (success) with correct stdcall arg counts.
        ("shell32.dll", "ShellExecuteA", |c| {
            c.ret_stdcall(5, 6);
            Handled::Ok
        }),
        ("shell32.dll", "ShellExecuteW", |c| {
            let op = c.wstr(c.arg(1));
            let file = c.wstr(c.arg(2));
            let params = c.wstr(c.arg(3));
            c.logs.log(
                webwine_api::logs::LogLevel::Trace,
                "api",
                &format!("ShellExecuteW op={op:?} file={file:?} params={params:?}"),
                Some(c.pid),
            );
            c.ret_stdcall(5, 6);
            Handled::Ok
        }),
        ("shell32.dll", "ShellExecuteExA", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shell32.dll", "ShellExecuteExW", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("shell32.dll", "SHGetFolderPathA", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("shell32.dll", "SHGetFolderPathW", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("shell32.dll", "SHGetMalloc", |c| {
            if c.arg(0) != 0 { let _ = c.memory.write_u32(c.arg(0), 0); }
            c.ret_stdcall(0x8000_4001, 1);
            Handled::Ok
        }),
        ("shell32.dll", "SHGetSpecialFolderLocation", |c| {
            if c.arg(2) != 0 { let _ = c.memory.write_u32(c.arg(2), 0); }
            c.ret_stdcall(0x8000_4001, 3);
            Handled::Ok
        }),
        ("shell32.dll", "SHGetKnownFolderIDList", |c| {
            if c.arg(3) != 0 { let _ = c.memory.write_u32(c.arg(3), 0); }
            c.ret_stdcall(0x8000_4001, 4);
            Handled::Ok
        }),
        ("shell32.dll", "CommandLineToArgvW", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        (
            "shell32.dll",
            "SetCurrentProcessExplicitAppUserModelID",
            |c| {
                c.ret_stdcall(0, 1);
                Handled::Ok
            },
        ),
        ("shell32.dll", "SHGetPropertyStoreForWindow", |c| {
            c.ret_stdcall(0x8000_4001u32, 3);
            Handled::Ok
        }),
        ("shell32.dll", "SHAddToRecentDocs", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("shell32.dll", "SHGetFileInfoA", sh_get_file_info_a),
        ("shell32.dll", "SHGetFileInfoW", sh_get_file_info_w),
        ("shell32.dll", "DragAcceptFiles", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("shell32.dll", "ExtractIconW", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("shell32.dll", "Shell_NotifyIconW", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("shell32.dll", "Shell_NotifyIconA", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn sh_get_file_info_a(c: &mut webwine_api::winapi::ApiContext) -> Handled {
    let info = c.arg(2);
    let size = c.arg(3);
    if info != 0 && size > 0 {
        let zeros = vec![0; (size as usize).min(352)];
        let _ = c.memory.write_bytes(info, &zeros);
        if size >= 12 { let _ = c.memory.write_u32(info + 8, 0x80); }
    }
    c.ret_stdcall(1, 5);
    Handled::Ok
}

fn sh_get_file_info_w(c: &mut webwine_api::winapi::ApiContext) -> Handled {
    let info = c.arg(2);
    let size = c.arg(3);
    if info != 0 && size > 0 {
        let zeros = vec![0; (size as usize).min(692)];
        let _ = c.memory.write_bytes(info, &zeros);
        if size >= 12 { let _ = c.memory.write_u32(info + 8, 0x80); }
    }
    c.ret_stdcall(1, 5);
    Handled::Ok
}

fn path_get_args_w(c: &mut webwine_api::winapi::ApiContext) -> Handled {
    let start = c.arg(0);
    let value = c.wstr(start);
    let offset = value.find(char::is_whitespace).unwrap_or(value.len());
    let mut pointer = start + offset as u32 * 2;
    while c.memory.read_u16(pointer).unwrap_or(0) == b' ' as u16 { pointer += 2; }
    c.ret_stdcall(pointer, 1);
    Handled::Ok
}
