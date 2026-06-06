use super::{ApiContext, Handled, WinApiRegistry};
use crate::util::{register_entries, ret_0_0, ret_0_1, Entry};
use webwine_api::vm::process::UiEvent;

pub fn register(r: &mut WinApiRegistry) {
    register_entries(r, ENTRIES);
    // File pickers block the guest until the user chooses a file or cancels.
    r.add("comdlg32.dll", "GetOpenFileNameA", |c| file_dialog(c, false, false));
    r.add("comdlg32.dll", "GetOpenFileNameW", |c| file_dialog(c, false, true));
    r.add("comdlg32.dll", "GetSaveFileNameA", |c| file_dialog(c, true, false));
    r.add("comdlg32.dll", "GetSaveFileNameW", |c| file_dialog(c, true, true));
}

const ENTRIES: &[Entry] = &[
    ("comdlg32.dll", "ChooseColorA", ret_0_1),
    ("comdlg32.dll", "ChooseColorW", ret_0_1),
    ("comdlg32.dll", "ChooseFontA", ret_0_1),
    ("comdlg32.dll", "ChooseFontW", ret_0_1),
    ("comdlg32.dll", "PrintDlgA", ret_0_1),
    ("comdlg32.dll", "PrintDlgW", ret_0_1),
    ("comdlg32.dll", "CommDlgExtendedError", ret_0_0),
];

// OPENFILENAME field offsets (32-bit, same for A and W).
const OFN_FILTER: u32 = 12;
const OFN_FILE: u32 = 28; // lpstrFile (in/out buffer)
const OFN_MAXFILE: u32 = 32;
const OFN_INITIALDIR: u32 = 44;
const OFN_TITLE: u32 = 48;

/// GetOpenFileName / GetSaveFileName — modal, blocks until the user replies.
/// First call shows the picker and suspends; the host posts the chosen path (or
/// cancel) via `post_dialog_reply`, which resumes the call to fill lpstrFile and
/// return TRUE, or return FALSE on cancel.
fn file_dialog(ctx: &mut ApiContext, save: bool, wide: bool) -> Handled {
    let ofn = ctx.arg(0);

    if let Some(reply) = ctx.gui.dialog_reply.take() {
        ctx.gui.dialog_pending = false;
        let result = match reply.file {
            Some(path) => {
                let lp_file = ctx.memory.read_u32(ofn + OFN_FILE).unwrap_or(0);
                let max = ctx.memory.read_u32(ofn + OFN_MAXFILE).unwrap_or(260).max(1);
                if lp_file != 0 {
                    write_path(ctx, lp_file, &path, max, wide);
                }
                1 // TRUE
            }
            None => 0, // FALSE — cancelled
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
    let filter = read_filter(ctx, ctx.memory.read_u32(ofn + OFN_FILTER).unwrap_or(0), wide);

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

/// Read a string through a pointer field at `ptr_addr`.
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

/// Read a Win32 double-null filter ("Label\0pattern\0...\0\0") into
/// "Label|pattern|..." pairs.
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
        p += if wide { (units + 1) as u32 * 2 } else { (s.len() + 1) as u32 };
        parts.push(s);
    }
    parts.join("|")
}

/// Write `path` (+ null) into the guest buffer, capped at `max` chars.
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
