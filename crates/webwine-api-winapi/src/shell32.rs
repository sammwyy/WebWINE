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
        ("shell32.dll", "CommandLineToArgvW", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("shell32.dll", "SetCurrentProcessExplicitAppUserModelID", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shell32.dll", "SHGetPropertyStoreForWindow", |c| { c.ret_stdcall(0x8000_4001u32, 3); Handled::Ok }),
        ("shell32.dll", "SHAddToRecentDocs", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("shell32.dll", "DragAcceptFiles", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("shell32.dll", "ExtractIconW", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("shell32.dll", "Shell_NotifyIconW", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("shell32.dll", "Shell_NotifyIconA", |c| { c.ret_stdcall(1, 2); Handled::Ok }),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
