//! shell32.dll — shell helpers (paths, execute, file info, PIDLs).

use super::{ApiContext, Handled, WinApiRegistry};

// Common CSIDL values → guest paths.
fn csidl_path(csidl: u32) -> &'static str {
    match csidl & 0xFF {
        0x00 => "C:\\Users\\guest\\Desktop",          // CSIDL_DESKTOP
        0x05 => "C:\\Users\\guest\\Documents",        // CSIDL_PERSONAL / My Documents
        0x06 => "C:\\Users\\guest\\Favorites",        // CSIDL_FAVORITES
        0x10 => "C:\\Users\\guest\\Desktop",          // CSIDL_DESKTOPDIRECTORY
        0x1a => "C:\\Users\\guest\\AppData\\Roaming", // CSIDL_APPDATA
        0x1c => "C:\\Users\\guest\\AppData\\Local",   // CSIDL_LOCAL_APPDATA
        0x28 => "C:\\Users\\guest",                   // CSIDL_PROFILE
        0x23 => "C:\\ProgramData",                    // CSIDL_COMMON_APPDATA
        0x26 => "C:\\Program Files",                  // CSIDL_PROGRAM_FILES
        0x2b => "C:\\Program Files\\Common Files",    // CSIDL_PROGRAM_FILES_COMMON
        0x24 => "C:\\Windows",                        // CSIDL_WINDOWS
        0x25 => "C:\\Windows\\System32",              // CSIDL_SYSTEM
        0x0d => "C:\\Users\\guest\\Music",            // CSIDL_MYMUSIC
        0x27 => "C:\\Users\\guest\\Pictures",         // CSIDL_MYPICTURES
        0x0e => "C:\\Users\\guest\\Videos",           // CSIDL_MYVIDEO
        _ => "C:\\Users\\guest",
    }
}

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("shell32.dll", "#68", run_file_dlg),
        ("shell32.dll", "#34", crate::kernel32::path_find_file_name_w),
        ("shell32.dll", "#52", path_get_args_w),
        ("shell32.dll", "#71", sh_alloc_shared),
        ("shell32.dll", "#72", sh_lock_shared),
        ("shell32.dll", "#89", il_create_from_path),
        ("shell32.dll", "#155", il_free),
        ("shell32.dll", "#165", sh_change_notify),
        ("shell32.dll", "#181", is_user_an_admin),
        ("shell32.dll", "#188", sh_get_set_settings),
        ("shell32.dll", "#100", sh_create_std_enum_fmt_etc),
        ("shell32.dll", "#245", sh_test_token_membership),
        ("shell32.dll", "#660", sh_wait_for_file_to_open),
        ("shell32.dll", "#723", sh_get_image_list),
        ("shell32.dll", "#899", sh_explorer_bootstrap),
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
        ("shell32.dll", "ShellExecuteA", shell_execute_a),
        ("shell32.dll", "ShellExecuteW", shell_execute_w),
        ("shell32.dll", "ShellExecuteExA", shell_execute_ex),
        ("shell32.dll", "ShellExecuteExW", shell_execute_ex),
        ("shell32.dll", "SHGetFolderPathA", |c| sh_get_folder_path(c, false)),
        ("shell32.dll", "SHGetFolderPathW", |c| sh_get_folder_path(c, true)),
        ("shell32.dll", "SHGetFolderPathEx", sh_get_folder_path_ex),
        ("shell32.dll", "SHGetMalloc", sh_get_malloc),
        (
            "shell32.dll",
            "SHGetSpecialFolderLocation",
            sh_get_special_folder_location,
        ),
        ("shell32.dll", "SHGetKnownFolderIDList", sh_get_known_folder_id_list),
        ("shell32.dll", "CommandLineToArgvW", command_line_to_argv_w),
        (
            "shell32.dll",
            "SetCurrentProcessExplicitAppUserModelID",
            set_app_user_model_id,
        ),
        (
            "shell32.dll",
            "SHGetPropertyStoreForWindow",
            sh_get_property_store_for_window,
        ),
        ("shell32.dll", "SHAddToRecentDocs", sh_add_to_recent_docs),
        ("shell32.dll", "SHGetFileInfoA", sh_get_file_info_a),
        ("shell32.dll", "SHGetFileInfoW", sh_get_file_info_w),
        ("shell32.dll", "DragAcceptFiles", drag_accept_files),
        ("shell32.dll", "ExtractIconW", extract_icon_w),
        ("shell32.dll", "ExtractIconA", extract_icon_a),
        ("shell32.dll", "Shell_NotifyIconW", shell_notify_icon),
        ("shell32.dll", "Shell_NotifyIconA", shell_notify_icon),
        ("shell32.dll", "SHFileOperationA", sh_file_operation),
        ("shell32.dll", "SHFileOperationW", sh_file_operation),
        ("shell32.dll", "SHBrowseForFolderA", sh_browse_for_folder),
        ("shell32.dll", "SHBrowseForFolderW", sh_browse_for_folder),
        ("shell32.dll", "SHGetPathFromIDListA", |c| sh_get_path_from_id_list(c, false)),
        ("shell32.dll", "SHGetPathFromIDListW", |c| sh_get_path_from_id_list(c, true)),
        ("shell32.dll", "ILFree", il_free),
        ("shell32.dll", "SHFree", sh_free),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn run_file_dlg(c: &mut ApiContext) -> Handled {
    // RunFileDlg — 6 args, void. No UI.
    c.ret_stdcall(0, 6);
    Handled::Ok
}

fn sh_alloc_shared(c: &mut ApiContext) -> Handled {
    if c.arg(0) != 0 {
        let _ = c.memory.write_u32(c.arg(0), 0x494C_0001);
    }
    if c.arg(1) != 0 {
        let _ = c.memory.write_u32(c.arg(1), 0x494C_0002);
    }
    c.ret_stdcall(1, 2);
    Handled::Ok
}

fn sh_lock_shared(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn il_create_from_path(c: &mut ApiContext) -> Handled {
    // Returns a minimal absolute PIDL (two zero bytes = empty ITEMIDLIST).
    let pidl = c.heap_alloc(2);
    let _ = c.memory.write_u16(pidl, 0);
    c.ret_stdcall(pidl, 3);
    Handled::Ok
}

fn il_free(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p != 0 {
        c.heap_sizes.remove(&p);
    }
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn sh_free(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p != 0 {
        c.heap_sizes.remove(&p);
    }
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn sh_change_notify(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn is_user_an_admin(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 2);
    Handled::Ok
}

fn sh_get_set_settings(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn sh_create_std_enum_fmt_etc(c: &mut ApiContext) -> Handled {
    let out = c.arg(2);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(0x8000_4005, 3); // E_FAIL
    Handled::Ok
}

fn sh_test_token_membership(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn sh_wait_for_file_to_open(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn sh_get_image_list(c: &mut ApiContext) -> Handled {
    let out = c.arg(1);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn sh_explorer_bootstrap(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn shell_execute_a(c: &mut ApiContext) -> Handled {
    // HINSTANCE > 32 means success.
    let file = c.cstr(c.arg(2));
    c.logs.log(
        webwine_api::logs::LogLevel::Trace,
        "api",
        &format!("ShellExecuteA file={file:?}"),
        Some(c.pid),
    );
    c.ret_stdcall(42, 6);
    Handled::Ok
}

fn shell_execute_w(c: &mut ApiContext) -> Handled {
    let op = c.wstr(c.arg(1));
    let file = c.wstr(c.arg(2));
    let params = c.wstr(c.arg(3));
    c.logs.log(
        webwine_api::logs::LogLevel::Trace,
        "api",
        &format!("ShellExecuteW op={op:?} file={file:?} params={params:?}"),
        Some(c.pid),
    );
    c.ret_stdcall(42, 6);
    Handled::Ok
}

fn shell_execute_ex(c: &mut ApiContext) -> Handled {
    // SHELLEXECUTEINFO: fMask @4, hwnd @8, lpVerb @12, lpFile @16, ...
    // Report failure (no child process spawn from shell yet).
    c.cpu.last_error = 1155; // ERROR_NO_ASSOCIATION
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn sh_get_folder_path(c: &mut ApiContext, wide: bool) -> Handled {
    // HRESULT SHGetFolderPath(hwnd, csidl, token, flags, path)
    let csidl = c.arg(1);
    let out = c.arg(4);
    let path = csidl_path(csidl);
    if out != 0 {
        write_path_buf(c, out, path, wide, 260);
    }
    c.ret_stdcall(0, 5); // S_OK
    Handled::Ok
}

fn sh_get_folder_path_ex(c: &mut ApiContext) -> Handled {
    // SHGetFolderPathEx(rfid, flags, token, path, cch) — rfid is a GUID ptr.
    // Fall back to profile path.
    let out = c.arg(3);
    let cch = c.arg(4) as usize;
    if out != 0 && cch > 0 {
        write_path_buf(c, out, "C:\\Users\\guest", true, cch);
    }
    c.ret_stdcall(0, 5);
    Handled::Ok
}

fn sh_get_malloc(c: &mut ApiContext) -> Handled {
    if c.arg(0) != 0 {
        let _ = c.memory.write_u32(c.arg(0), 0);
    }
    c.ret_stdcall(0x8000_4001, 1); // E_NOTIMPL
    Handled::Ok
}

fn sh_get_special_folder_location(c: &mut ApiContext) -> Handled {
    // SHGetSpecialFolderLocation(hwnd, csidl, pidl*)
    let csidl = c.arg(1);
    let out = c.arg(2);
    let path = csidl_path(csidl);
    // Encode path into a simple absolute PIDL-like blob (not a real ITEMIDLIST
    // parse, but non-null so callers that only free it work).
    let bytes = path.as_bytes();
    let pidl = c.heap_alloc((bytes.len() + 4) as u32);
    let _ = c.memory.write_u16(pidl, (bytes.len() + 2) as u16);
    let _ = c.memory.write_bytes(pidl + 2, bytes);
    let _ = c.memory.write_u16(pidl + 2 + bytes.len() as u32, 0);
    if out != 0 {
        let _ = c.memory.write_u32(out, pidl);
    }
    c.ret_stdcall(0, 3); // S_OK
    Handled::Ok
}

fn sh_get_known_folder_id_list(c: &mut ApiContext) -> Handled {
    let out = c.arg(3);
    if out != 0 {
        let pidl = c.heap_alloc(2);
        let _ = c.memory.write_u16(pidl, 0);
        let _ = c.memory.write_u32(out, pidl);
    }
    c.ret_stdcall(0, 4);
    Handled::Ok
}

/// CommandLineToArgvW(cmd, pNumArgs) → LPWSTR* (array of pointers + string data).
fn command_line_to_argv_w(c: &mut ApiContext) -> Handled {
    let cmd_ptr = c.arg(0);
    let num_ptr = c.arg(1);
    let cmd = if cmd_ptr != 0 {
        c.wstr(cmd_ptr)
    } else {
        c.cmdline.to_string()
    };
    let args = split_command_line(&cmd);
    let argc = args.len() as u32;
    if num_ptr != 0 {
        let _ = c.memory.write_u32(num_ptr, argc);
    }
    if argc == 0 {
        c.ret_stdcall(0, 2);
        return Handled::Ok;
    }

    // Layout: [argc pointers][string0\0][string1\0]...
    let mut str_bytes: Vec<u8> = Vec::new();
    let mut offsets = Vec::new();
    for a in &args {
        offsets.push(str_bytes.len() as u32);
        for u in a.encode_utf16() {
            str_bytes.extend_from_slice(&u.to_le_bytes());
        }
        str_bytes.extend_from_slice(&0u16.to_le_bytes());
    }
    let ptr_table_size = argc * 4;
    let total = ptr_table_size + str_bytes.len() as u32;
    let base = c.heap_alloc(total);
    if base == 0 {
        c.ret_stdcall(0, 2);
        return Handled::Ok;
    }
    for (i, off) in offsets.iter().enumerate() {
        let _ = c
            .memory
            .write_u32(base + i as u32 * 4, base + ptr_table_size + off);
    }
    let _ = c.memory.write_bytes(base + ptr_table_size, &str_bytes);
    c.ret_stdcall(base, 2);
    Handled::Ok
}

/// Split a Windows command line into argv (handles quotes, like CommandLineToArgvW).
fn split_command_line(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = cmd.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    args.push(std::mem::take(&mut cur));
                }
            }
            '\\' if in_quotes => {
                // Backslash-escape only before a quote (simplified Wine rules).
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cur.push('"');
                } else {
                    cur.push('\\');
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        args.push(cur);
    }
    if args.is_empty() {
        args.push(String::new());
    }
    args
}

fn set_app_user_model_id(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1); // S_OK
    Handled::Ok
}

fn sh_get_property_store_for_window(c: &mut ApiContext) -> Handled {
    let out = c.arg(2);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(0x8000_4001u32, 3); // E_NOTIMPL
    Handled::Ok
}

fn sh_add_to_recent_docs(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn sh_get_file_info_a(c: &mut ApiContext) -> Handled {
    let info = c.arg(2);
    let size = c.arg(3);
    if info != 0 && size > 0 {
        let zeros = vec![0; (size as usize).min(352)];
        let _ = c.memory.write_bytes(info, &zeros);
        if size >= 12 {
            let _ = c.memory.write_u32(info + 8, 0x80); // FILE_ATTRIBUTE_NORMAL
        }
    }
    c.ret_stdcall(1, 5);
    Handled::Ok
}

fn sh_get_file_info_w(c: &mut ApiContext) -> Handled {
    let info = c.arg(2);
    let size = c.arg(3);
    if info != 0 && size > 0 {
        let zeros = vec![0; (size as usize).min(692)];
        let _ = c.memory.write_bytes(info, &zeros);
        if size >= 12 {
            let _ = c.memory.write_u32(info + 8, 0x80);
        }
    }
    c.ret_stdcall(1, 5);
    Handled::Ok
}

fn drag_accept_files(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn extract_icon_w(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3); // no icon
    Handled::Ok
}

fn extract_icon_a(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn shell_notify_icon(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 2); // TRUE
    Handled::Ok
}

fn sh_file_operation(c: &mut ApiContext) -> Handled {
    // SHFileOperation → 0 success. No real multi-file ops yet.
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn sh_browse_for_folder(c: &mut ApiContext) -> Handled {
    // Returns PIDL or NULL (cancel). Cancel.
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn sh_get_path_from_id_list(c: &mut ApiContext, wide: bool) -> Handled {
    let out = c.arg(1);
    if out != 0 {
        write_path_buf(c, out, "C:\\Users\\guest", wide, 260);
    }
    c.ret_stdcall(1, 2); // TRUE
    Handled::Ok
}

fn write_path_buf(c: &mut ApiContext, out: u32, path: &str, wide: bool, max_chars: usize) {
    if wide {
        let units: Vec<u16> = path.encode_utf16().take(max_chars.saturating_sub(1)).collect();
        for (i, u) in units.iter().enumerate() {
            let _ = c.memory.write_u16(out + i as u32 * 2, *u);
        }
        let _ = c.memory.write_u16(out + units.len() as u32 * 2, 0);
    } else {
        let n = path.len().min(max_chars.saturating_sub(1));
        let _ = c.memory.write_bytes(out, path.as_bytes()[..n].as_ref());
        let _ = c.memory.write_u8(out + n as u32, 0);
    }
}

fn path_get_args_w(c: &mut ApiContext) -> Handled {
    let start = c.arg(0);
    let value = c.wstr(start);
    let offset = value.find(char::is_whitespace).unwrap_or(value.len());
    let mut pointer = start + offset as u32 * 2;
    while c.memory.read_u16(pointer).unwrap_or(0) == b' ' as u16 {
        pointer += 2;
    }
    c.ret_stdcall(pointer, 1);
    Handled::Ok
}
