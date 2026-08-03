use super::{ApiContext, Handled, WinApiRegistry};
use std::collections::HashMap;
use webwine_api::vm::process::{
    GdiObject, GuestMsg, MenuItem, MenuItemData, UiEvent, WindowEntry, GDI_TAG,
};

// Window messages we care about.
const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000F;
const WM_CLOSE: u32 = 0x0010;
const WM_QUIT: u32 = 0x0012;

const CW_USEDEFAULT: u32 = 0x8000_0000;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("user32.dll", "MessageBoxA", msgbox_a),
        ("user32.dll", "MessageBoxW", msgbox_w),
        ("user32.dll", "MessageBoxExA", msgbox_a),
        ("user32.dll", "MessageBoxExW", msgbox_w),
        ("user32.dll", "MessageBoxIndirectA", msgbox_indirect_a),
        ("user32.dll", "MessageBoxIndirectW", msgbox_indirect_w),
        ("user32.dll", "MessageBeep", |c| {
            c.ui_events.push(UiEvent::Beep {
                freq: 800,
                duration: 200,
            });
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("user32.dll", "RegisterClassA", register_class_a),
        ("user32.dll", "RegisterClassW", register_class_a),
        ("user32.dll", "RegisterClassExA", register_class_ex_a),
        ("user32.dll", "RegisterClassExW", register_class_ex_a),
        ("user32.dll", "CreateWindowExA", create_window_ex_a),
        ("user32.dll", "CreateWindowExW", create_window_ex_w),
        // Dialogs: create a real guest window with the dialog proc as its WndProc.
        // Controls aren't laid out from the template yet, but the window + message
        // loop run (WM_INITDIALOG is queued).
        ("user32.dll", "CreateDialogParamA", create_dialog),
        ("user32.dll", "CreateDialogParamW", create_dialog),
        ("user32.dll", "CreateDialogIndirectParamA", create_dialog),
        ("user32.dll", "CreateDialogIndirectParamW", create_dialog),
        ("user32.dll", "DialogBoxParamA", create_dialog),
        ("user32.dll", "DialogBoxParamW", create_dialog),
        ("user32.dll", "DialogBoxIndirectParamA", create_dialog),
        ("user32.dll", "DialogBoxIndirectParamW", create_dialog),
        ("user32.dll", "EndDialog", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("user32.dll", "IsDialogMessageW", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("user32.dll", "IsDialogMessageA", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("user32.dll", "GetDlgItem", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("user32.dll", "SendDlgItemMessageW", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("user32.dll", "SendDlgItemMessageA", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("user32.dll", "SetDlgItemTextA", |c| {
            c.ret_stdcall(1, 3);
            Handled::Ok
        }),
        ("user32.dll", "SetDlgItemTextW", |c| {
            c.ret_stdcall(1, 3);
            Handled::Ok
        }),
        ("user32.dll", "GetDlgItemTextW", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("user32.dll", "CheckDlgButton", |c| {
            c.ret_stdcall(1, 3);
            Handled::Ok
        }),
        ("user32.dll", "IsDlgButtonChecked", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("user32.dll", "ShowWindow", show_window),
        ("user32.dll", "UpdateWindow", update_window),
        ("user32.dll", "DestroyWindow", destroy_window),
        ("user32.dll", "SetWindowTextA", set_window_text_a),
        ("user32.dll", "SetWindowTextW", set_window_text_w),
        ("user32.dll", "DefWindowProcA", def_window_proc),
        ("user32.dll", "DefWindowProcW", def_window_proc),
        ("user32.dll", "PostQuitMessage", post_quit_message),
        ("user32.dll", "GetMessageA", get_message),
        ("user32.dll", "GetMessageW", get_message),
        ("user32.dll", "PeekMessageA", peek_message),
        ("user32.dll", "PeekMessageW", peek_message),
        ("user32.dll", "TranslateMessage", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("user32.dll", "DispatchMessageA", dispatch_message),
        ("user32.dll", "DispatchMessageW", dispatch_message),
        ("user32.dll", "BeginPaint", begin_paint),
        ("user32.dll", "EndPaint", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("user32.dll", "GetClientRect", get_client_rect),
        ("user32.dll", "SetRect", set_rect),
        ("user32.dll", "SetRectEmpty", set_rect_empty),
        ("user32.dll", "GetSysColor", |c| {
            // COLOR_WINDOW / COLOR_BTNFACE / COLOR_WINDOWTEXT defaults.
            let color = match c.arg(0) {
                5 => 0x00FF_FFFF,
                15 => 0x00F0_F0F0,
                8 => 0x0000_0000,
                _ => 0x00C0_C0C0,
            };
            c.ret_stdcall(color, 1);
            Handled::Ok
        }),
        ("user32.dll", "InvalidateRect", invalidate_rect),
        ("user32.dll", "PostMessageA", |c| {
            c.ret_stdcall(1, 4);
            Handled::Ok
        }),
        ("user32.dll", "PostMessageW", |c| {
            c.ret_stdcall(1, 4);
            Handled::Ok
        }),
        ("user32.dll", "SendMessageA", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("user32.dll", "SendMessageW", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("user32.dll", "IsWindow", |c| {
            let exists = c.gui.windows.contains_key(&c.arg(0));
            c.ret_stdcall(exists as u32, 1);
            Handled::Ok
        }),
        ("user32.dll", "CharNextA", char_next_a),
        ("user32.dll", "CharNextW", char_next_w),
        ("user32.dll", "CharPrevA", char_prev_a),
        ("user32.dll", "CharPrevW", char_prev_w),
        ("user32.dll", "CharUpperA", char_upper_a),
        ("user32.dll", "CharUpperW", char_upper_w),
        ("user32.dll", "CharLowerA", char_lower_a),
        ("user32.dll", "CharLowerW", char_lower_w),
        ("user32.dll", "DdeInitializeA", dde_initialize),
        ("user32.dll", "DdeInitializeW", dde_initialize),
        ("user32.dll", "DdeUninitialize", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("user32.dll", "DdeCreateStringHandleA", dde_create_string_handle),
        ("user32.dll", "DdeCreateStringHandleW", dde_create_string_handle),
        ("user32.dll", "DdeFreeStringHandle", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("user32.dll", "DdeKeepStringHandle", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("user32.dll", "DdeNameService", |c| { c.ret_stdcall(1, 4); Handled::Ok }),
        ("user32.dll", "DdeGetLastError", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        // common resource / paint stubs
        ("user32.dll", "LoadCursorA", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("user32.dll", "LoadCursorW", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("user32.dll", "LoadIconA", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("user32.dll", "LoadIconW", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("user32.dll", "LoadAcceleratorsA", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("user32.dll", "LoadAcceleratorsW", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("user32.dll", "TranslateAcceleratorA", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("user32.dll", "TranslateAcceleratorW", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("user32.dll", "LoadMenuA", load_menu),
        ("user32.dll", "LoadMenuW", load_menu),
        ("user32.dll", "GetSystemMenu", get_system_menu),
        ("user32.dll", "SetCursor", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("user32.dll", "SetWindowLongA", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("user32.dll", "SetWindowLongW", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("user32.dll", "GetWindowLongA", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "GetWindowLongW", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "GetClassLongA", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "GetClassLongW", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "LoadBitmapA", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("user32.dll", "LoadBitmapW", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("user32.dll", "LoadImageA", |c| { c.ret_stdcall(1, 6); Handled::Ok }),
        ("user32.dll", "LoadImageW", |c| { c.ret_stdcall(1, 6); Handled::Ok }),
        ("user32.dll", "LoadStringA", load_string_a),
        ("user32.dll", "LoadStringW", load_string_w),
        ("user32.dll", "GetSystemMetrics", |c| {
            // SM_CXSCREEN=0, SM_CYSCREEN=1, SM_CXFULLSCREEN=16, SM_CYFULLSCREEN=17.
            let v = match c.arg(0) {
                0 | 16 => 1920,
                1 | 17 => 1080,
                _ => 0,
            };
            c.ret_stdcall(v, 1);
            Handled::Ok
        }),
        ("user32.dll", "EnumDisplayDevicesA", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("user32.dll", "EnumDisplayDevicesW", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("user32.dll", "GetDC", |c| {
            let h = c.arg(0);
            c.ret_stdcall(if h == 0 { 1 } else { h }, 1);
            Handled::Ok
        }),
        ("user32.dll", "GetDesktopWindow", |c| {
            c.ret_stdcall(0x0001_0000, 0);
            Handled::Ok
        }),
        ("user32.dll", "GetKeyboardLayout", |c| {
            c.ret_stdcall(0x0409_0409, 1);
            Handled::Ok
        }),
        ("user32.dll", "SetWinEventHook", |c| {
            c.ret_stdcall(1, 7);
            Handled::Ok
        }),
        ("user32.dll", "UnhookWinEvent", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("user32.dll", "SetActiveWindow", |c| {
            c.ret_stdcall(c.arg(0), 1);
            Handled::Ok
        }),
        ("user32.dll", "SetProcessDPIAware", |c| {
            c.ret_stdcall(1, 0);
            Handled::Ok
        }),
        ("user32.dll", "GetShellWindow", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("user32.dll", "FindWindowA", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("user32.dll", "FindWindowW", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("user32.dll", "GetDoubleClickTime", |c| {
            c.ret_stdcall(500, 0);
            Handled::Ok
        }),
        ("user32.dll", "GetCaretBlinkTime", |c| {
            c.ret_stdcall(500, 0);
            Handled::Ok
        }),
        // RegisterWindowMessage: hand out unique IDs in the 0xC000-0xFFFF range
        // (0 means failure, which makes apps that register many messages misbehave).
        (
            "user32.dll",
            "RegisterWindowMessageW",
            register_window_message,
        ),
        (
            "user32.dll",
            "RegisterWindowMessageA",
            register_window_message,
        ),
        (
            "user32.dll",
            "SystemParametersInfoW",
            system_parameters_info,
        ),
        (
            "user32.dll",
            "SystemParametersInfoA",
            system_parameters_info,
        ),
        ("user32.dll", "ReleaseDC", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("user32.dll", "FillRect", fill_rect),
        // Menus
        ("user32.dll", "CreateMenu", create_menu),
        ("user32.dll", "CreatePopupMenu", create_menu),
        ("user32.dll", "AppendMenuA", |c| append_menu(c, false)),
        ("user32.dll", "AppendMenuW", |c| append_menu(c, true)),
        ("user32.dll", "SetMenu", set_menu),
        ("user32.dll", "GetMenu", get_menu),
        ("user32.dll", "DestroyMenu", destroy_menu),
        ("user32.dll", "DrawMenuBar", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("user32.dll", "EnableMenuItem", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("user32.dll", "CheckMenuItem", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("user32.dll", "wsprintfA", |c| wsprintf(c, false)),
        ("user32.dll", "wsprintfW", |c| wsprintf(c, true)),
        // GetKeyboardState/SetKeyboardState: no real input-polling backing yet,
        // so report "all keys up" / accept-and-ignore rather than failing —
        // callers that don't check the (currently-always-0) return value were
        // reading whatever garbage sat in the guest buffer.
        ("user32.dll", "GetKeyboardState", |c| {
            let buf = c.arg(0);
            let _ = c.memory.write_bytes(buf, &[0u8; 256]);
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("user32.dll", "SetKeyboardState", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("user32.dll", "SetTimer", set_timer),
        ("user32.dll", "KillTimer", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn char_next_a(c: &mut ApiContext) -> Handled {
    let ptr = c.arg(0);
    let next = if ptr != 0 && c.memory.read_u8(ptr).unwrap_or(0) != 0 { ptr + 1 } else { ptr };
    c.ret_stdcall(next, 1);
    Handled::Ok
}

fn char_next_w(c: &mut ApiContext) -> Handled {
    let ptr = c.arg(0);
    let next = if ptr != 0 && c.memory.read_u16(ptr).unwrap_or(0) != 0 { ptr + 2 } else { ptr };
    c.ret_stdcall(next, 1);
    Handled::Ok
}

fn char_prev_a(c: &mut ApiContext) -> Handled {
    let start = c.arg(0);
    let current = c.arg(1);
    c.ret_stdcall(if current > start { current - 1 } else { start }, 2);
    Handled::Ok
}

fn char_prev_w(c: &mut ApiContext) -> Handled {
    let start = c.arg(0);
    let current = c.arg(1);
    c.ret_stdcall(if current >= start + 2 { current - 2 } else { start }, 2);
    Handled::Ok
}

fn char_upper_a(c: &mut ApiContext) -> Handled { change_case_a(c, true) }
fn char_lower_a(c: &mut ApiContext) -> Handled { change_case_a(c, false) }
fn char_upper_w(c: &mut ApiContext) -> Handled { change_case_w(c, true) }
fn char_lower_w(c: &mut ApiContext) -> Handled { change_case_w(c, false) }

fn change_case_a(c: &mut ApiContext, upper: bool) -> Handled {
    let value = c.arg(0);
    if value <= 0xFFFF {
        let byte = value as u8;
        let changed = if upper { byte.to_ascii_uppercase() } else { byte.to_ascii_lowercase() };
        c.ret_stdcall(changed as u32, 1);
    } else {
        let original = c.cstr(value);
        let changed = if upper { original.to_ascii_uppercase() } else { original.to_ascii_lowercase() };
        let mut bytes = changed.into_bytes();
        bytes.push(0);
        let _ = c.memory.write_bytes(value, &bytes);
        c.ret_stdcall(value, 1);
    }
    Handled::Ok
}

fn change_case_w(c: &mut ApiContext, upper: bool) -> Handled {
    let value = c.arg(0);
    if value <= 0xFFFF {
        let character = char::from_u32(value).unwrap_or('\0');
        let changed = if upper { character.to_uppercase().next() } else { character.to_lowercase().next() }
            .unwrap_or(character) as u32;
        c.ret_stdcall(changed, 1);
    } else {
        let original = c.wstr(value);
        let changed = if upper { original.to_uppercase() } else { original.to_lowercase() };
        for (index, unit) in changed.encode_utf16().chain(std::iter::once(0)).enumerate() {
            let _ = c.memory.write_u16(value + index as u32 * 2, unit);
        }
        c.ret_stdcall(value, 1);
    }
    Handled::Ok
}

fn dde_initialize(c: &mut ApiContext) -> Handled {
    let instance = c.arg(0);
    if instance != 0 { let _ = c.memory.write_u32(instance, 1); }
    c.ret_stdcall(0, 4);
    Handled::Ok
}

fn dde_create_string_handle(c: &mut ApiContext) -> Handled {
    let handle = c.arg(1);
    c.ret_stdcall(if handle == 0 { 1 } else { handle }, 3);
    Handled::Ok
}

fn load_string_a(c: &mut ApiContext) -> Handled {
    let id = c.arg(1);
    let value = c.strings.get(&id).cloned().unwrap_or_default();
    c.logs.log(webwine_api::logs::LogLevel::Trace, "api", &format!("LoadStringA id={id} -> {value:?}"), Some(c.pid));
    let out = c.arg(2);
    let max = c.arg(3) as usize;
    let count = value.len().min(max.saturating_sub(1));
    if out != 0 && max > 0 {
        let _ = c.memory.write_bytes(out, &value.as_bytes()[..count]);
        let _ = c.memory.write_u8(out + count as u32, 0);
    }
    c.ret_stdcall(count as u32, 4);
    Handled::Ok
}

fn load_string_w(c: &mut ApiContext) -> Handled {
    let id = c.arg(1);
    let value = c.strings.get(&id).cloned().unwrap_or_default();
    c.logs.log(webwine_api::logs::LogLevel::Trace, "api", &format!("LoadStringW id={id} -> {value:?}"), Some(c.pid));
    let out = c.arg(2);
    let max = c.arg(3) as usize;
    let units: Vec<u16> = value.encode_utf16().take(max.saturating_sub(1)).collect();
    if out != 0 && max > 0 {
        for (index, unit) in units.iter().enumerate() {
            let _ = c.memory.write_u16(out + index as u32 * 2, *unit);
        }
        let _ = c.memory.write_u16(out + units.len() as u32 * 2, 0);
    }
    c.ret_stdcall(units.len() as u32, 4);
    Handled::Ok
}

fn load_menu(c: &mut ApiContext) -> Handled {
    let handle = c.gui.next_menu;
    c.gui.next_menu += 1;
    c.gui.menus.insert(handle, Vec::new());
    c.ret_stdcall(handle, 2);
    Handled::Ok
}

fn get_system_menu(c: &mut ApiContext) -> Handled {
    if c.arg(1) != 0 {
        c.ret_stdcall(0, 2);
        return Handled::Ok;
    }
    let handle = c.gui.next_menu;
    c.gui.next_menu += 1;
    c.gui.menus.entry(handle).or_default();
    c.ret_stdcall(handle, 2);
    Handled::Ok
}

fn set_rect(ctx: &mut ApiContext) -> Handled {
    let rect = ctx.arg(0);
    if rect != 0 {
        let _ = ctx.memory.write_u32(rect, ctx.arg(1));
        let _ = ctx.memory.write_u32(rect + 4, ctx.arg(2));
        let _ = ctx.memory.write_u32(rect + 8, ctx.arg(3));
        let _ = ctx.memory.write_u32(rect + 12, ctx.arg(4));
    }
    ctx.ret_stdcall((rect != 0) as u32, 5);
    Handled::Ok
}

fn set_rect_empty(ctx: &mut ApiContext) -> Handled {
    let rect = ctx.arg(0);
    if rect != 0 {
        for offset in [0, 4, 8, 12] {
            let _ = ctx.memory.write_u32(rect + offset, 0);
        }
    }
    ctx.ret_stdcall((rect != 0) as u32, 1);
    Handled::Ok
}

fn msgbox_a(ctx: &mut ApiContext) -> Handled {
    let text = ctx.cstr(ctx.arg(1));
    let title = ctx.cstr(ctx.arg(2));
    msgbox_common(ctx, title, text, ctx.arg(3))
}

fn msgbox_w(ctx: &mut ApiContext) -> Handled {
    let text = ctx.wstr(ctx.arg(1));
    let title = ctx.wstr(ctx.arg(2));
    msgbox_common(ctx, title, text, ctx.arg(3))
}

fn msgbox_indirect_a(ctx: &mut ApiContext) -> Handled {
    let params = ctx.arg(0);
    let text = ctx.cstr(ctx.memory.read_u32(params + 12).unwrap_or(0));
    let title = ctx.cstr(ctx.memory.read_u32(params + 16).unwrap_or(0));
    msgbox_common_n(ctx, title, text, ctx.memory.read_u32(params + 20).unwrap_or(0), 1)
}

fn msgbox_indirect_w(ctx: &mut ApiContext) -> Handled {
    let params = ctx.arg(0);
    let text = ctx.wstr(ctx.memory.read_u32(params + 12).unwrap_or(0));
    let title = ctx.wstr(ctx.memory.read_u32(params + 16).unwrap_or(0));
    msgbox_common_n(ctx, title, text, ctx.memory.read_u32(params + 20).unwrap_or(0), 1)
}

/// MessageBox blocks the guest until the user clicks a button (modal). On the
/// first call it shows the box and suspends; the host posts the clicked button
/// via `post_dialog_reply`, which resumes the call to return that ID.
fn msgbox_common(ctx: &mut ApiContext, title: String, text: String, style: u32) -> Handled {
    msgbox_common_n(ctx, title, text, style, 4)
}

fn msgbox_common_n(ctx: &mut ApiContext, title: String, text: String, style: u32, nargs: u32) -> Handled {
    if let Some(reply) = ctx.gui.dialog_reply.take() {
        ctx.gui.dialog_pending = false;
        ctx.ret_stdcall(reply.button, nargs);
        return Handled::Ok;
    }
    if ctx.gui.dialog_pending {
        return Handled::Block; // still on screen, awaiting the user
    }
    ctx.gui.dialog_pending = true;
    ctx.ui_events
        .push(UiEvent::MessageBox { title, text, style });
    Handled::Block
}

fn register_window_message(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0xC000, 1);
    Handled::Ok
}

fn system_parameters_info(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(1, 4);
    Handled::Ok
}

// window classes

// WNDCLASSA: style@0, lpfnWndProc@4, â€¦, lpszClassName@36
fn register_class_a(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let wndproc = ctx.memory.read_u32(p + 4).unwrap_or(0);
    let name_ptr = ctx.memory.read_u32(p + 36).unwrap_or(0);
    let name = read_class_name(ctx, name_ptr);
    ctx.gui.classes.insert(name, wndproc);
    ctx.ret_stdcall(1, 1); // atom
    Handled::Ok
}

// WNDCLASSEXA: cbSize@0, style@4, lpfnWndProc@8, â€¦, lpszClassName@40
fn register_class_ex_a(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let wndproc = ctx.memory.read_u32(p + 8).unwrap_or(0);
    let name_ptr = ctx.memory.read_u32(p + 40).unwrap_or(0);
    let name = read_class_name(ctx, name_ptr);
    ctx.gui.classes.insert(name, wndproc);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn read_class_name(ctx: &ApiContext, ptr: u32) -> String {
    // Class name may be an atom (small integer) rather than a string pointer.
    if ptr == 0 {
        return String::new();
    }
    if ptr < 0x1_0000 {
        return format!("#atom{ptr}");
    }
    ctx.cstr(ptr)
}

// window creation

fn create_window_ex_a(ctx: &mut ApiContext) -> Handled {
    let class = read_class_name(ctx, ctx.arg(1));
    let title = ctx.cstr(ctx.arg(2));
    create_window(ctx, class, title)
}

fn create_window_ex_w(ctx: &mut ApiContext) -> Handled {
    let class = read_class_name(ctx, ctx.arg(1));
    let title = ctx.wstr(ctx.arg(2));
    create_window(ctx, class, title)
}

fn create_window(ctx: &mut ApiContext, class: String, title: String) -> Handled {
    let x = norm_coord(ctx.arg(4), 80);
    let y = norm_coord(ctx.arg(5), 80);
    let w = norm_dim(ctx.arg(6), 480);
    let h = norm_dim(ctx.arg(7), 320);

    let wndproc = ctx.gui.classes.get(&class).copied().unwrap_or(0);
    let hwnd = ctx.gui.next_hwnd;
    ctx.gui.next_hwnd += 4;
    ctx.gui.windows.insert(
        hwnd,
        WindowEntry {
            wndproc,
            needs_paint: true,
            width: w,
            height: h,
            pen_color: 0x00_0000,   // black
            brush_color: 0xFF_FFFF, // white
            cur_x: 0,
            cur_y: 0,
        },
    );

    ctx.ui_events.push(UiEvent::CreateWindow {
        hwnd,
        title,
        x,
        y,
        width: w,
        height: h,
    });
    ctx.ret_stdcall(hwnd, 12);
    Handled::Ok
}

// menus
// Menu items added via AppendMenu build a tree in `gui.menus`; SetMenu resolves
// it and emits UiEvent::SetMenu so the frontend draws the menu bar. A clicked
// leaf posts WM_COMMAND(id) back through the normal message pump.

const MF_GRAYED: u32 = 0x0001;
const MF_DISABLED: u32 = 0x0002;
const MF_POPUP: u32 = 0x0010;
const MF_SEPARATOR: u32 = 0x0800;

fn create_menu(ctx: &mut ApiContext) -> Handled {
    let h = ctx.gui.next_menu;
    ctx.gui.next_menu += 1;
    ctx.gui.menus.insert(h, Vec::new());
    ctx.ret_stdcall(h, 0);
    Handled::Ok
}

// AppendMenu(hMenu, uFlags, uIDNewItem, lpNewItem) — A and W share this; the
// text read differs by `wide`.
fn append_menu(ctx: &mut ApiContext, wide: bool) -> Handled {
    let hmenu = ctx.arg(0);
    let flags = ctx.arg(1);
    let id_or_sub = ctx.arg(2);
    let text_ptr = ctx.arg(3);

    let item = if flags & MF_SEPARATOR != 0 {
        MenuItem {
            text: String::new(),
            id: 0,
            submenu: None,
            separator: true,
            disabled: false,
        }
    } else {
        let text = if text_ptr == 0 {
            String::new()
        } else if wide {
            ctx.wstr(text_ptr)
        } else {
            ctx.cstr(text_ptr)
        };
        let disabled = flags & (MF_GRAYED | MF_DISABLED) != 0;
        if flags & MF_POPUP != 0 {
            MenuItem {
                text,
                id: 0,
                submenu: Some(id_or_sub),
                separator: false,
                disabled,
            }
        } else {
            MenuItem {
                text,
                id: id_or_sub,
                submenu: None,
                separator: false,
                disabled,
            }
        }
    };
    if let Some(items) = ctx.gui.menus.get_mut(&hmenu) {
        items.push(item);
    }
    ctx.ret_stdcall(1, 4); // TRUE
    Handled::Ok
}

/// Expand a menu handle into a resolved tree for the frontend.
fn resolve_menu(menus: &HashMap<u32, Vec<MenuItem>>, handle: u32, depth: u8) -> Vec<MenuItemData> {
    if depth > 8 {
        return Vec::new();
    }
    menus
        .get(&handle)
        .map(|items| {
            items
                .iter()
                .map(|it| MenuItemData {
                    text: it.text.clone(),
                    id: it.id,
                    separator: it.separator,
                    disabled: it.disabled,
                    children: it
                        .submenu
                        .map(|h| resolve_menu(menus, h, depth + 1))
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn set_menu(ctx: &mut ApiContext) -> Handled {
    // SetMenu(hWnd, hMenu) — hMenu 0 removes the menu.
    let hwnd = ctx.arg(0);
    let hmenu = ctx.arg(1);
    let items = if hmenu == 0 {
        Vec::new()
    } else {
        resolve_menu(&ctx.gui.menus, hmenu, 0)
    };
    if hmenu == 0 {
        ctx.gui.hwnd_menu.remove(&hwnd);
    } else {
        ctx.gui.hwnd_menu.insert(hwnd, hmenu);
    }
    ctx.ui_events.push(UiEvent::SetMenu { hwnd, items });
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

fn get_menu(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let h = ctx.gui.hwnd_menu.get(&hwnd).copied().unwrap_or(0);
    ctx.ret_stdcall(h, 1);
    Handled::Ok
}

fn destroy_menu(ctx: &mut ApiContext) -> Handled {
    let h = ctx.arg(0);
    ctx.gui.menus.remove(&h);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// CreateDialogParam/DialogBoxParam(hInst, lpTemplate, hWndParent, lpDialogFunc,
// dwInitParam) â€” 5 args. Create a guest window whose WndProc is the dialog proc,
// then queue WM_INITDIALOG so the proc initializes. Returns the HWND.
const WM_INITDIALOG: u32 = 0x0110;
fn create_dialog(ctx: &mut ApiContext) -> Handled {
    let dlgproc = ctx.arg(3);
    let init_param = ctx.arg(4);
    let (w, h) = (600, 460);
    let hwnd = ctx.gui.next_hwnd;
    ctx.gui.next_hwnd += 4;
    ctx.gui.windows.insert(
        hwnd,
        WindowEntry {
            wndproc: dlgproc,
            needs_paint: true,
            width: w,
            height: h,
            pen_color: 0x00_0000,
            brush_color: 0xFF_FFFF,
            cur_x: 0,
            cur_y: 0,
        },
    );
    ctx.ui_events.push(UiEvent::CreateWindow {
        hwnd,
        title: "Dialog".to_string(),
        x: 120,
        y: 70,
        width: w,
        height: h,
    });
    // The dialog manager sends WM_INITDIALOG before the dialog becomes visible.
    ctx.gui.queue.push_back(GuestMsg {
        hwnd,
        message: WM_INITDIALOG,
        wparam: 0,
        lparam: init_param,
    });
    ctx.ret_stdcall(hwnd, 5);
    Handled::Ok
}

fn norm_coord(v: u32, default: i32) -> i32 {
    if v == CW_USEDEFAULT {
        default
    } else {
        v as i32
    }
}
fn norm_dim(v: u32, default: i32) -> i32 {
    if v == CW_USEDEFAULT || v == 0 {
        default
    } else {
        v as i32
    }
}

fn show_window(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let cmd = ctx.arg(1);
    ctx.ui_events.push(UiEvent::ShowWindow {
        hwnd,
        show: cmd != 0,
    });
    if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
        w.needs_paint = true;
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

fn update_window(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
        w.needs_paint = true;
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn invalidate_rect(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
        w.needs_paint = true;
    }
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

fn destroy_window(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    // Stop painting, close the DOM window, but keep the WndProc mapping so the
    // WM_DESTROY we enqueue can still be dispatched to it.
    if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
        w.needs_paint = false;
    }
    ctx.ui_events.push(UiEvent::DestroyWindow { hwnd });
    ctx.gui.queue.push_back(GuestMsg {
        hwnd,
        message: WM_DESTROY,
        wparam: 0,
        lparam: 0,
    });
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn set_window_text_a(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let title = ctx.cstr(ctx.arg(1));
    ctx.ui_events.push(UiEvent::SetWindowText { hwnd, title });
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}
fn set_window_text_w(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let title = ctx.wstr(ctx.arg(1));
    ctx.ui_events.push(UiEvent::SetWindowText { hwnd, title });
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// message loop

fn def_window_proc(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let msg = ctx.arg(1);
    if msg == WM_CLOSE {
        // default WM_CLOSE handling = DestroyWindow: close the DOM window and
        // queue WM_DESTROY (keeping the WndProc mapping so it can be dispatched).
        if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
            w.needs_paint = false;
        }
        ctx.ui_events.push(UiEvent::DestroyWindow { hwnd });
        ctx.gui.queue.push_back(GuestMsg {
            hwnd,
            message: WM_DESTROY,
            wparam: 0,
            lparam: 0,
        });
    }
    ctx.ret_stdcall(0, 4);
    Handled::Ok
}

fn post_quit_message(ctx: &mut ApiContext) -> Handled {
    let code = ctx.arg(0);
    ctx.gui.quit = Some(code);
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

// Build the next message into the MSG struct at lpMsg. Returns true if one was
// produced, false if the queue is empty (caller decides block/quit).
fn next_message(ctx: &mut ApiContext) -> Option<GuestMsg> {
    if let Some(m) = ctx.gui.queue.pop_front() {
        return Some(m);
    }
    // Synthesize a WM_PAINT for the first window that needs repainting.
    let paint_hwnd = ctx
        .gui
        .windows
        .iter()
        .find(|(_, w)| w.needs_paint)
        .map(|(h, _)| *h);
    if let Some(hwnd) = paint_hwnd {
        if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
            w.needs_paint = false;
        }
        return Some(GuestMsg {
            hwnd,
            message: WM_PAINT,
            wparam: 0,
            lparam: 0,
        });
    }
    None
}

fn write_msg(ctx: &mut ApiContext, lp: u32, m: &GuestMsg) {
    // MSG { hwnd, message, wParam, lParam, time, pt.x, pt.y }
    let _ = ctx.memory.write_u32(lp, m.hwnd);
    let _ = ctx.memory.write_u32(lp + 4, m.message);
    let _ = ctx.memory.write_u32(lp + 8, m.wparam);
    let _ = ctx.memory.write_u32(lp + 12, m.lparam);
    let _ = ctx.memory.write_u32(lp + 16, 0);
    let _ = ctx.memory.write_u32(lp + 20, 0);
    let _ = ctx.memory.write_u32(lp + 24, 0);
}

fn get_message(ctx: &mut ApiContext) -> Handled {
    let lp = ctx.arg(0);

    // WM_QUIT when quit requested and the queue is drained.
    if ctx.gui.quit.is_some() && ctx.gui.queue.is_empty() {
        let code = ctx.gui.quit.take().unwrap_or(0);
        write_msg(
            ctx,
            lp,
            &GuestMsg {
                hwnd: 0,
                message: WM_QUIT,
                wparam: code,
                lparam: 0,
            },
        );
        ctx.ret_stdcall(0, 4); // 0 => loop exits
        return Handled::Ok;
    }

    match next_message(ctx) {
        Some(m) => {
            write_msg(ctx, lp, &m);
            ctx.ret_stdcall(1, 4);
            Handled::Ok
        }
        None => {
            // No work and no quit â€” suspend until the frontend posts a message.
            Handled::Block
        }
    }
}

fn peek_message(ctx: &mut ApiContext) -> Handled {
    let lp = ctx.arg(0);
    let remove = ctx.arg(4) & 1 != 0; // PM_REMOVE
    match if remove {
        next_message(ctx)
    } else {
        ctx.gui.queue.front().cloned()
    } {
        Some(m) => {
            write_msg(ctx, lp, &m);
            ctx.ret_stdcall(1, 5);
        }
        None => {
            ctx.ret_stdcall(0, 5);
        }
    }
    Handled::Ok
}

fn dispatch_message(ctx: &mut ApiContext) -> Handled {
    let lp = ctx.arg(0);
    let hwnd = ctx.memory.read_u32(lp).unwrap_or(0);
    let msg = ctx.memory.read_u32(lp + 4).unwrap_or(0);
    let wp = ctx.memory.read_u32(lp + 8).unwrap_or(0);
    let lpm = ctx.memory.read_u32(lp + 12).unwrap_or(0);

    let wndproc = ctx
        .gui
        .windows
        .get(&hwnd)
        .map(|w| w.wndproc)
        .filter(|p| *p != 0);

    match wndproc {
        Some(func) => Handled::Invoke {
            func,
            args: vec![hwnd, msg, wp, lpm],
            ret_args: 1,
        },
        None => {
            ctx.ret_stdcall(0, 1);
            Handled::Ok
        }
    }
}

// painting

fn begin_paint(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let ps = ctx.arg(1);
    // Clear the client area before the guest repaints it.
    ctx.ui_events.push(UiEvent::ClearClient { hwnd });
    // PAINTSTRUCT { hdc, fErase, rcPaint(4), â€¦ } â€” provide hdc = hwnd.
    if ps != 0 {
        let _ = ctx.memory.write_u32(ps, hwnd);
        let _ = ctx.memory.write_u32(ps + 4, 0);
    }
    ctx.ret_stdcall(hwnd, 2); // return HDC (= hwnd)
    Handled::Ok
}

fn get_client_rect(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let rp = ctx.arg(1);
    let (w, h) = ctx
        .gui
        .windows
        .get(&hwnd)
        .map(|e| (e.width, e.height))
        .unwrap_or((480, 320));
    if rp != 0 {
        let _ = ctx.memory.write_u32(rp, 0);
        let _ = ctx.memory.write_u32(rp + 4, 0);
        let _ = ctx.memory.write_u32(rp + 8, w as u32);
        let _ = ctx.memory.write_u32(rp + 12, h as u32);
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// TextOutA(hdc, x, y, lpString, count) â€” hdc is the hwnd from BeginPaint.
pub(crate) fn text_out_a(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let x = ctx.arg(1) as i32;
    let y = ctx.arg(2) as i32;
    let count = ctx.arg(4) as usize;
    let bytes = ctx.memory.read_bytes(ctx.arg(3), count).unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    ctx.ui_events.push(UiEvent::DrawText {
        hwnd,
        x,
        y,
        text,
        color: 0,
    });
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

pub(crate) fn text_out_w(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let x = ctx.arg(1) as i32;
    let y = ctx.arg(2) as i32;
    let count = ctx.arg(4);
    let mut text = ctx.memory.read_wstr(ctx.arg(3));
    text = text.chars().take(count as usize).collect();
    ctx.ui_events.push(UiEvent::DrawText {
        hwnd,
        x,
        y,
        text,
        color: 0,
    });
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

// GDI objects & drawing
// We encode GDI objects in their handle value: 0x0B<rgb> = solid brush,
// 0x0C<rgb> = pen. hdc == hwnd (BeginPaint/GetDC return the hwnd).

const BRUSH_TAG: u32 = 0x0B00_0000;
const PEN_TAG: u32 = 0x0C00_0000;
const OBJ_MASK: u32 = 0x00FF_FFFF;

pub(crate) fn create_solid_brush(ctx: &mut ApiContext) -> Handled {
    let color = ctx.arg(0) & OBJ_MASK;
    ctx.ret_stdcall(BRUSH_TAG | color, 1);
    Handled::Ok
}

pub(crate) fn create_pen(ctx: &mut ApiContext) -> Handled {
    let color = ctx.arg(2) & OBJ_MASK; // CreatePen(style, width, color)
    ctx.ret_stdcall(PEN_TAG | color, 3);
    Handled::Ok
}

pub(crate) fn get_stock_object(ctx: &mut ApiContext) -> Handled {
    // WHITE_BRUSH=0, LTGRAY=1, GRAY=2, DKGRAY=3, BLACK_BRUSH=4, NULL_BRUSH=5,
    // BLACK_PEN=7, WHITE_PEN=6
    let obj = match ctx.arg(0) {
        0 => BRUSH_TAG | 0xFFFFFF,
        1 => BRUSH_TAG | 0xC0C0C0,
        2 => BRUSH_TAG | 0x808080,
        3 => BRUSH_TAG | 0x404040,
        4 => BRUSH_TAG | 0x000000,
        6 => PEN_TAG | 0xFFFFFF,
        7 => PEN_TAG | 0x000000,
        _ => BRUSH_TAG | 0xFFFFFF,
    };
    ctx.ret_stdcall(obj, 1);
    Handled::Ok
}

pub(crate) fn select_object(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let obj = ctx.arg(1);

    // Selecting a bitmap into a memory DC (the DIB-section blit path).
    if hwnd & 0xFF00_0000 == GDI_TAG {
        let mut prev = 0;
        if let Some(GdiObject::MemDc { bitmap }) = ctx.gui.gdi_objects.get_mut(&hwnd) {
            prev = *bitmap;
            *bitmap = obj;
        }
        ctx.ret_stdcall(prev, 2);
        return Handled::Ok;
    }

    let mut prev = 0;
    if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
        if obj & 0xFF00_0000 == PEN_TAG {
            prev = PEN_TAG | w.pen_color;
            w.pen_color = obj & OBJ_MASK;
        } else if obj & 0xFF00_0000 == BRUSH_TAG {
            prev = BRUSH_TAG | w.brush_color;
            w.brush_color = obj & OBJ_MASK;
        }
    }
    ctx.ret_stdcall(prev, 2);
    Handled::Ok
}

pub(crate) fn move_to_ex(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let x = ctx.arg(1) as i32;
    let y = ctx.arg(2) as i32;
    let lppt = ctx.arg(3);
    if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
        if lppt != 0 {
            let _ = ctx.memory.write_u32(lppt, w.cur_x as u32);
            let _ = ctx.memory.write_u32(lppt + 4, w.cur_y as u32);
        }
        w.cur_x = x;
        w.cur_y = y;
    }
    ctx.ret_stdcall(1, 4);
    Handled::Ok
}

pub(crate) fn line_to(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let x = ctx.arg(1) as i32;
    let y = ctx.arg(2) as i32;
    if let Some(w) = ctx.gui.windows.get(&hwnd) {
        let (x1, y1, color) = (w.cur_x, w.cur_y, w.pen_color);
        ctx.ui_events.push(UiEvent::Line {
            hwnd,
            x1,
            y1,
            x2: x,
            y2: y,
            color,
        });
    }
    if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
        w.cur_x = x;
        w.cur_y = y;
    }
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

pub(crate) fn gdi_rectangle(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let (l, t, r, b) = (
        ctx.arg(1) as i32,
        ctx.arg(2) as i32,
        ctx.arg(3) as i32,
        ctx.arg(4) as i32,
    );
    if let Some(w) = ctx.gui.windows.get(&hwnd) {
        let (fill, stroke) = (w.brush_color, w.pen_color);
        ctx.ui_events.push(UiEvent::Rect {
            hwnd,
            x: l,
            y: t,
            w: r - l,
            h: b - t,
            fill,
            stroke,
        });
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

pub(crate) fn gdi_ellipse(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let (l, t, r, b) = (
        ctx.arg(1) as i32,
        ctx.arg(2) as i32,
        ctx.arg(3) as i32,
        ctx.arg(4) as i32,
    );
    if let Some(w) = ctx.gui.windows.get(&hwnd) {
        let (fill, stroke) = (w.brush_color, w.pen_color);
        ctx.ui_events.push(UiEvent::Ellipse {
            hwnd,
            x: l,
            y: t,
            w: r - l,
            h: b - t,
            fill,
            stroke,
        });
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

pub(crate) fn gdi_set_pixel(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let x = ctx.arg(1) as i32;
    let y = ctx.arg(2) as i32;
    let color = ctx.arg(3) & OBJ_MASK;
    ctx.ui_events.push(UiEvent::SetPixel { hwnd, x, y, color });
    ctx.ret_stdcall(color, 4);
    Handled::Ok
}

// FillRect(hdc, const RECT*, hbrush) â€” user32
fn fill_rect(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let rp = ctx.arg(1);
    let brush = ctx.arg(2);
    let color = if brush & 0xFF00_0000 == BRUSH_TAG {
        brush & OBJ_MASK
    } else {
        0xFFFFFF
    };
    let l = ctx.memory.read_u32(rp).unwrap_or(0) as i32;
    let t = ctx.memory.read_u32(rp + 4).unwrap_or(0) as i32;
    let r = ctx.memory.read_u32(rp + 8).unwrap_or(0) as i32;
    let b = ctx.memory.read_u32(rp + 12).unwrap_or(0) as i32;
    ctx.ui_events.push(UiEvent::FillRect {
        hwnd,
        x: l,
        y: t,
        w: r - l,
        h: b - t,
        color,
    });
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

// GDI framebuffer: memory DCs, DIB sections, and blitting.

fn new_gdi_handle(ctx: &mut ApiContext) -> u32 {
    let h = ctx.gui.next_gdi;
    ctx.gui.next_gdi = ctx.gui.next_gdi.wrapping_add(4);
    h
}

// CreateCompatibleDC(hdc) -> memory DC handle.
pub(crate) fn create_compatible_dc(ctx: &mut ApiContext) -> Handled {
    let h = new_gdi_handle(ctx);
    ctx.gui
        .gdi_objects
        .insert(h, GdiObject::MemDc { bitmap: 0 });
    ctx.ret_stdcall(h, 1);
    Handled::Ok
}

// CreateDIBSection(hdc, *BITMAPINFO, usage, **ppvBits, hSection, offset) -> HBITMAP.
// Allocates the pixel buffer in guest memory and writes its pointer to *ppvBits.
pub(crate) fn create_dib_section(ctx: &mut ApiContext) -> Handled {
    let bmi = ctx.arg(1);
    let ppv_bits = ctx.arg(3);
    // BITMAPINFOHEADER: biSize@0, biWidth@4, biHeight@8, biPlanes@12(u16),
    // biBitCount@14(u16).
    let width = ctx.memory.read_u32(bmi + 4).unwrap_or(0) as i32;
    let raw_h = ctx.memory.read_u32(bmi + 8).unwrap_or(0) as i32;
    let bpp = (ctx.memory.read_u32(bmi + 12).unwrap_or(0) >> 16) as u16;
    let top_down = raw_h < 0;
    let height = raw_h.abs();
    let bpp = if bpp == 0 { 32 } else { bpp };

    let stride = (((width * bpp as i32 + 31) / 32) * 4).max(0) as u32;
    let size = stride * height.max(0) as u32;
    let bits = ctx.heap_alloc(size.max(4));
    if ppv_bits != 0 {
        let _ = ctx.memory.write_u32(ppv_bits, bits);
    }

    let h = new_gdi_handle(ctx);
    ctx.gui.gdi_objects.insert(
        h,
        GdiObject::Dib {
            bits,
            width,
            height,
            bpp,
            top_down,
        },
    );
    ctx.ret_stdcall(h, 6);
    Handled::Ok
}

// CreateCompatibleBitmap(hdc, w, h) -> HBITMAP (32bpp top-down, own buffer).
pub(crate) fn create_compatible_bitmap(ctx: &mut ApiContext) -> Handled {
    let width = ctx.arg(1) as i32;
    let height = ctx.arg(2) as i32;
    let size = (width.max(0) * height.max(0) * 4).max(4) as u32;
    let bits = ctx.heap_alloc(size);
    let h = new_gdi_handle(ctx);
    ctx.gui.gdi_objects.insert(
        h,
        GdiObject::Dib {
            bits,
            width,
            height,
            bpp: 32,
            top_down: true,
        },
    );
    ctx.ret_stdcall(h, 3);
    Handled::Ok
}

// Resolve a source DC (memory DC with a selected DIB) to its DIB.
fn dib_of_dc(ctx: &ApiContext, dc: u32) -> Option<(u32, i32, i32, u16, bool)> {
    let bitmap = match ctx.gui.gdi_objects.get(&dc) {
        Some(GdiObject::MemDc { bitmap }) => *bitmap,
        Some(GdiObject::Dib {
            bits,
            width,
            height,
            bpp,
            top_down,
        }) => return Some((*bits, *width, *height, *bpp, *top_down)),
        None => return None,
    };
    match ctx.gui.gdi_objects.get(&bitmap) {
        Some(GdiObject::Dib {
            bits,
            width,
            height,
            bpp,
            top_down,
        }) => Some((*bits, *width, *height, *bpp, *top_down)),
        _ => None,
    }
}

// Read a w*h region from a DIB's guest memory and convert to RGBA8888.
fn read_dib_rgba(
    ctx: &ApiContext,
    bits: u32,
    dib_w: i32,
    dib_h: i32,
    bpp: u16,
    top_down: bool,
    sx: i32,
    sy: i32,
    w: i32,
    h: i32,
) -> Vec<u8> {
    let bytespp = (bpp / 8).max(1) as i32;
    let stride = ((dib_w * bpp as i32 + 31) / 32) * 4;
    let mut out = vec![0u8; (w.max(0) * h.max(0) * 4) as usize];
    for row in 0..h {
        // For bottom-up DIBs, row 0 is at the bottom of the buffer.
        let src_row = if top_down {
            sy + row
        } else {
            dib_h - 1 - (sy + row)
        };
        if src_row < 0 || src_row >= dib_h {
            continue;
        }
        for col in 0..w {
            let src_col = sx + col;
            if src_col < 0 || src_col >= dib_w {
                continue;
            }
            let off =
                bits as i64 + src_row as i64 * stride as i64 + src_col as i64 * bytespp as i64;
            let px = ctx.memory.read_u32(off as u32).unwrap_or(0);
            // DIB pixels are BGRA/BGRX little-endian.
            let (b, g, r) = (
                (px & 0xFF) as u8,
                ((px >> 8) & 0xFF) as u8,
                ((px >> 16) & 0xFF) as u8,
            );
            let di = ((row * w + col) * 4) as usize;
            out[di] = r;
            out[di + 1] = g;
            out[di + 2] = b;
            out[di + 3] = 0xFF;
        }
    }
    out
}

// BitBlt(hdcDest, x, y, w, h, hdcSrc, sx, sy, rop) -> blit memory DC to a window.
pub(crate) fn bit_blt(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let (x, y, w, h) = (
        ctx.arg(1) as i32,
        ctx.arg(2) as i32,
        ctx.arg(3) as i32,
        ctx.arg(4) as i32,
    );
    let src = ctx.arg(5);
    let (sx, sy) = (ctx.arg(6) as i32, ctx.arg(7) as i32);

    if let Some((bits, dw, dh, bpp, td)) = dib_of_dc(ctx, src) {
        if ctx.gui.windows.contains_key(&dest) {
            let pixels = read_dib_rgba(ctx, bits, dw, dh, bpp, td, sx, sy, w, h);
            ctx.ui_events.push(UiEvent::Blit {
                hwnd: dest,
                x,
                y,
                w,
                h,
                src_w: w,
                src_h: h,
                pixels,
            });
        }
    }
    ctx.ret_stdcall(1, 9);
    Handled::Ok
}

// StretchBlt(hdcDest, x, y, w, h, hdcSrc, sx, sy, sw, sh, rop) -> 11 args.
pub(crate) fn stretch_blt(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let (x, y, w, h) = (
        ctx.arg(1) as i32,
        ctx.arg(2) as i32,
        ctx.arg(3) as i32,
        ctx.arg(4) as i32,
    );
    let src = ctx.arg(5);
    let (sx, sy, sw, sh) = (
        ctx.arg(6) as i32,
        ctx.arg(7) as i32,
        ctx.arg(8) as i32,
        ctx.arg(9) as i32,
    );

    if let Some((bits, dw, dh, bpp, td)) = dib_of_dc(ctx, src) {
        if ctx.gui.windows.contains_key(&dest) {
            let pixels = read_dib_rgba(ctx, bits, dw, dh, bpp, td, sx, sy, sw, sh);
            ctx.ui_events.push(UiEvent::Blit {
                hwnd: dest,
                x,
                y,
                w,
                h,
                src_w: sw,
                src_h: sh,
                pixels,
            });
        }
    }
    ctx.ret_stdcall(1, 11);
    Handled::Ok
}

// StretchDIBits(hdc, xDest,yDest,wDest,hDest, xSrc,ySrc,wSrc,hSrc, *bits, *bmi,
//               usage, rop) -> 13 args. Blits a caller-supplied DIB buffer.
pub(crate) fn stretch_dibits(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let (xd, yd, wd, hd) = (
        ctx.arg(1) as i32,
        ctx.arg(2) as i32,
        ctx.arg(3) as i32,
        ctx.arg(4) as i32,
    );
    let (xs, ys, ws, hs) = (
        ctx.arg(5) as i32,
        ctx.arg(6) as i32,
        ctx.arg(7) as i32,
        ctx.arg(8) as i32,
    );
    let bits = ctx.arg(9);
    let bmi = ctx.arg(10);
    let dib_w = ctx.memory.read_u32(bmi + 4).unwrap_or(0) as i32;
    let raw_h = ctx.memory.read_u32(bmi + 8).unwrap_or(0) as i32;
    let bpp = ((ctx.memory.read_u32(bmi + 12).unwrap_or(0) >> 16) as u16).max(1);

    if ctx.gui.windows.contains_key(&dest) {
        let pixels = read_dib_rgba(
            ctx,
            bits,
            dib_w,
            raw_h.abs(),
            bpp,
            raw_h < 0,
            xs,
            ys,
            ws,
            hs,
        );
        ctx.ui_events.push(UiEvent::Blit {
            hwnd: dest,
            x: xd,
            y: yd,
            w: wd,
            h: hd,
            src_w: ws,
            src_h: hs,
            pixels,
        });
    }
    ctx.ret_stdcall(hs.max(0) as u32, 13);
    Handled::Ok
}

// SetDIBitsToDevice(hdc, xDest,yDest, w,h, xSrc,ySrc, startScan,scanLines,
//                   *bits, *bmi, usage) -> 12 args.
pub(crate) fn set_dibits_to_device(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let (xd, yd, w, h) = (
        ctx.arg(1) as i32,
        ctx.arg(2) as i32,
        ctx.arg(3) as i32,
        ctx.arg(4) as i32,
    );
    let (xs, ys) = (ctx.arg(5) as i32, ctx.arg(6) as i32);
    let bits = ctx.arg(9);
    let bmi = ctx.arg(10);
    let dib_w = ctx.memory.read_u32(bmi + 4).unwrap_or(0) as i32;
    let raw_h = ctx.memory.read_u32(bmi + 8).unwrap_or(0) as i32;
    let bpp = ((ctx.memory.read_u32(bmi + 12).unwrap_or(0) >> 16) as u16).max(1);

    if ctx.gui.windows.contains_key(&dest) {
        let pixels = read_dib_rgba(ctx, bits, dib_w, raw_h.abs(), bpp, raw_h < 0, xs, ys, w, h);
        ctx.ui_events.push(UiEvent::Blit {
            hwnd: dest,
            x: xd,
            y: yd,
            w,
            h,
            src_w: w,
            src_h: h,
            pixels,
        });
    }
    ctx.ret_stdcall(h.max(0) as u32, 12);
    Handled::Ok
}

// wsprintfA/W(dst, fmt, ...) -> chars written (excl. null terminator).
// Declared WINAPI in the header, but MSVC always compiles a call to a
// variadic function as __cdecl regardless of the declared convention — so
// every call site does `call [wsprintfW]; add esp, N` itself. The real
// user32.dll implementation therefore pops nothing (plain `ret`); it must
// NOT clean any args here, or the caller's own `add esp` double-cleans the
// stack and corrupts it. (Confirmed against foobar2000's installer: call
// sites at 0x406630/0x406ab2 are followed by `add esp, 0xc` / `add esp,
// 0x10`.) The previous "unimplemented" fallback popped a fixed 1 arg, and
// an earlier version of this stub popped 2+consumed — both wrong for the
// same reason.
fn wsprintf(ctx: &mut ApiContext, wide: bool) -> Handled {
    let dst = ctx.arg(0);
    let fmt_ptr = ctx.arg(1);
    let fmt = if wide { ctx.wstr(fmt_ptr) } else { ctx.cstr(fmt_ptr) };

    let mut out = String::new();
    let mut consumed = 0u32;
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
        while i < chars.len() && "0123456789-+ #.*lh".contains(chars[i]) {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let spec = chars[i];
        i += 1;
        if spec == '%' {
            out.push('%');
            continue;
        }
        let arg = ctx.arg(2 + consumed);
        consumed += 1;
        match spec {
            'd' | 'i' => out.push_str(&(arg as i32).to_string()),
            'u' => out.push_str(&arg.to_string()),
            'x' => out.push_str(&format!("{:x}", arg)),
            'X' => out.push_str(&format!("{:X}", arg)),
            'c' => out.push(char::from_u32(arg).unwrap_or('\0')),
            's' => {
                let s = if wide { ctx.wstr(arg) } else { ctx.cstr(arg) };
                out.push_str(&s);
            }
            _ => {
                out.push('%');
                out.push(spec);
            }
        }
    }

    let n = out.chars().count() as u32;
    if wide {
        let mut units: Vec<u16> = out.encode_utf16().collect();
        units.push(0);
        let bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
        let _ = ctx.memory.write_bytes(dst, &bytes);
    } else {
        let mut bytes = out.into_bytes();
        bytes.push(0);
        let _ = ctx.memory.write_bytes(dst, &bytes);
    }
    ctx.ret_cdecl(n);
    Handled::Ok
}

// SetTimer(hWnd, nIDEvent, uElapse, lpTimerFunc) -> timer id.
// We don't run a real timer queue yet (WM_TIMER is never posted), but callers
// must get a stable non-zero id back — returning 0 (as the generic
// "unimplemented" stub did) signals failure, which some apps treat as fatal.
fn set_timer(ctx: &mut ApiContext) -> Handled {
    let id_arg = ctx.arg(1);
    let id = if id_arg != 0 {
        id_arg
    } else {
        let next = ctx.dll_state.entry("user32:next_timer_id".to_string()).or_insert(1);
        let id = *next;
        *next += 1;
        id
    };
    ctx.ret_stdcall(id, 4);
    Handled::Ok
}

// GetDeviceCaps(hdc, index) -> capability value.
pub(crate) fn get_device_caps(ctx: &mut ApiContext) -> Handled {
    let index = ctx.arg(1);
    let v = match index {
        8 => 1920,     // HORZRES
        10 => 1080,    // VERTRES
        12 => 32,      // BITSPIXEL
        14 => 1,       // PLANES
        88 | 90 => 96, // LOGPIXELSX / LOGPIXELSY (DPI)
        104 => 1,      // SHADEBLENDCAPS
        _ => 0,
    };
    ctx.ret_stdcall(v, 2);
    Handled::Ok
}
