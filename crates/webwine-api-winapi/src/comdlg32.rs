//! comdlg32.dll — common dialogs (file / color / font / print).
//!
//! File pickers block on the host UI (existing path). Color/font/print dialogs
//! cancel cleanly (return FALSE, CommDlgExtendedError = 0) so apps fall back.

use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::vm::process::UiEvent;

// OPENFILENAME field offsets (32-bit, same for A and W).
const OFN_FILTER: u32 = 12;
const OFN_FILE: u32 = 28;
const OFN_MAXFILE: u32 = 32;
const OFN_INITIALDIR: u32 = 44;
const OFN_TITLE: u32 = 48;

// Per-process last extended error (CDERR_*).
const DLL_STATE_ERR: &str = "comdlg32.ext_err";

pub fn register(r: &mut WinApiRegistry) {
    r.add("comdlg32.dll", "GetOpenFileNameA", |c| file_dialog(c, false, false));
    r.add("comdlg32.dll", "GetOpenFileNameW", |c| file_dialog(c, false, true));
    r.add("comdlg32.dll", "GetSaveFileNameA", |c| file_dialog(c, true, false));
    r.add("comdlg32.dll", "GetSaveFileNameW", |c| file_dialog(c, true, true));
    r.add("comdlg32.dll", "ChooseColorA", |c| choose_color(c, false));
    r.add("comdlg32.dll", "ChooseColorW", |c| choose_color(c, true));
    r.add("comdlg32.dll", "ChooseFontA", |c| choose_font(c, false));
    r.add("comdlg32.dll", "ChooseFontW", |c| choose_font(c, true));
    r.add("comdlg32.dll", "PrintDlgA", |c| print_dlg(c, false));
    r.add("comdlg32.dll", "PrintDlgW", |c| print_dlg(c, true));
    r.add("comdlg32.dll", "PageSetupDlgA", |c| page_setup_dlg(c, false));
    r.add("comdlg32.dll", "PageSetupDlgW", |c| page_setup_dlg(c, true));
    r.add("comdlg32.dll", "FindTextA", |c| find_replace(c, false, false));
    r.add("comdlg32.dll", "FindTextW", |c| find_replace(c, true, false));
    r.add("comdlg32.dll", "ReplaceTextA", |c| find_replace(c, false, true));
    r.add("comdlg32.dll", "ReplaceTextW", |c| find_replace(c, true, true));
    r.add("comdlg32.dll", "CommDlgExtendedError", comm_dlg_extended_error);
    r.add("comdlg32.dll", "GetFileTitleA", |c| get_file_title(c, false));
    r.add("comdlg32.dll", "GetFileTitleW", |c| get_file_title(c, true));
}

fn set_ext_err(c: &mut ApiContext, err: u32) {
    c.dll_state.insert(DLL_STATE_ERR.into(), err);
}

fn comm_dlg_extended_error(c: &mut ApiContext) -> Handled {
    let err = c.dll_state.get(DLL_STATE_ERR).copied().unwrap_or(0);
    c.ret_stdcall(err, 0);
    Handled::Ok
}

/// GetOpenFileName / GetSaveFileName — modal, blocks until the host replies.
fn file_dialog(ctx: &mut ApiContext, save: bool, wide: bool) -> Handled {
    let ofn = ctx.arg(0);
    if ofn == 0 {
        set_ext_err(ctx, 0x0001); // CDERR_DIALOGFAILURE
        ctx.ret_stdcall(0, 1);
        return Handled::Ok;
    }

    if let Some(reply) = ctx.gui.dialog_reply.take() {
        ctx.gui.dialog_pending = false;
        let result = match reply.file {
            Some(path) => {
                let lp_file = ctx.memory.read_u32(ofn + OFN_FILE).unwrap_or(0);
                let max = ctx.memory.read_u32(ofn + OFN_MAXFILE).unwrap_or(260).max(1);
                if lp_file != 0 {
                    write_path(ctx, lp_file, &path, max, wide);
                }
                set_ext_err(ctx, 0);
                1
            }
            None => {
                set_ext_err(ctx, 0); // cancel → no extended error
                0
            }
        };
        ctx.ret_stdcall(result, 1);
        return Handled::Ok;
    }
    if ctx.gui.dialog_pending {
        return Handled::Block;
    }

    let title = read_str_at(ctx, ofn + OFN_TITLE, wide);
    let initial_dir = read_str_at(ctx, ofn + OFN_INITIALDIR, wide);
    let default_name = read_str_at(ctx, ofn + OFN_FILE, wide);
    let filter = read_filter(
        ctx,
        ctx.memory.read_u32(ofn + OFN_FILTER).unwrap_or(0),
        wide,
    );

    ctx.gui.dialog_pending = true;
    ctx.ui_events.push(UiEvent::FileDialog {
        save,
        title,
        filter,
        initial_dir,
        default_name,
    });
    Handled::Block
}

fn choose_color(c: &mut ApiContext, _wide: bool) -> Handled {
    // ChooseColor(lpcc) — cancel (no host color picker yet).
    let cc = c.arg(0);
    if cc == 0 {
        set_ext_err(c, 0x0001);
        c.ret_stdcall(0, 1);
        return Handled::Ok;
    }
    set_ext_err(c, 0);
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn choose_font(c: &mut ApiContext, _wide: bool) -> Handled {
    let cf = c.arg(0);
    if cf == 0 {
        set_ext_err(c, 0x0001);
        c.ret_stdcall(0, 1);
        return Handled::Ok;
    }
    set_ext_err(c, 0);
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn print_dlg(c: &mut ApiContext, _wide: bool) -> Handled {
    let pd = c.arg(0);
    if pd == 0 {
        set_ext_err(c, 0x0001);
        c.ret_stdcall(0, 1);
        return Handled::Ok;
    }
    set_ext_err(c, 0);
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn page_setup_dlg(c: &mut ApiContext, _wide: bool) -> Handled {
    let ps = c.arg(0);
    if ps == 0 {
        set_ext_err(c, 0x0001);
        c.ret_stdcall(0, 1);
        return Handled::Ok;
    }
    set_ext_err(c, 0);
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn find_replace(c: &mut ApiContext, _wide: bool, _replace: bool) -> Handled {
    // FindText/ReplaceText return an HWND on success; NULL on failure.
    set_ext_err(c, 0);
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn get_file_title(c: &mut ApiContext, wide: bool) -> Handled {
    // short GetFileTitle(file, buf, buflen)
    // Returns 0 on success; negative on buffer-too-small; positive = required size.
    let file = c.arg(0);
    let buf = c.arg(1);
    let buflen = c.arg(2) as usize;
    if file == 0 {
        c.ret_stdcall(0xFFFF_FFFF, 3);
        return Handled::Ok;
    }
    let path = if wide {
        c.wstr(file)
    } else {
        c.cstr(file)
    };
    let title = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(path.as_str());
    if buf == 0 || buflen == 0 {
        c.ret_stdcall(title.len() as u32 + 1, 3);
        return Handled::Ok;
    }
    if wide {
        let units: Vec<u16> = title.encode_utf16().collect();
        if units.len() + 1 > buflen {
            c.ret_stdcall((units.len() + 1) as u32, 3);
            return Handled::Ok;
        }
        for (i, u) in units.iter().enumerate() {
            let _ = c.memory.write_u16(buf + i as u32 * 2, *u);
        }
        let _ = c.memory.write_u16(buf + units.len() as u32 * 2, 0);
    } else {
        if title.len() + 1 > buflen {
            c.ret_stdcall((title.len() + 1) as u32, 3);
            return Handled::Ok;
        }
        let _ = c.memory.write_bytes(buf, title.as_bytes());
        let _ = c.memory.write_u8(buf + title.len() as u32, 0);
    }
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn read_str_at(ctx: &ApiContext, ptr_addr: u32, wide: bool) -> String {
    let p = ctx.memory.read_u32(ptr_addr).unwrap_or(0);
    if p == 0 {
        String::new()
    } else if wide {
        ctx.wstr(p)
    } else {
        ctx.cstr(p)
    }
}

fn read_filter(ctx: &ApiContext, mut p: u32, wide: bool) -> String {
    if p == 0 {
        return String::new();
    }
    let mut parts = Vec::new();
    while parts.len() < 32 {
        let s = if wide { ctx.wstr(p) } else { ctx.cstr(p) };
        if s.is_empty() {
            break;
        }
        let units = s.encode_utf16().count();
        p += if wide {
            (units + 1) as u32 * 2
        } else {
            (s.len() + 1) as u32
        };
        parts.push(s);
    }
    parts.join("|")
}

fn write_path(ctx: &mut ApiContext, dst: u32, path: &str, max: u32, wide: bool) {
    if wide {
        let mut units: Vec<u16> = path.encode_utf16().collect();
        units.truncate((max - 1) as usize);
        let mut bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
        bytes.extend_from_slice(&[0, 0]);
        let _ = ctx.memory.write_bytes(dst, &bytes);
    } else {
        let mut bytes = path.as_bytes().to_vec();
        bytes.truncate((max - 1) as usize);
        bytes.push(0);
        let _ = ctx.memory.write_bytes(dst, &bytes);
    }
}
