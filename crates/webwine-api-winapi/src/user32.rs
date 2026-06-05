use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::vm::process::{GdiObject, GuestMsg, UiEvent, WindowEntry, GDI_TAG};

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
        ("user32.dll", "EndDialog", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("user32.dll", "IsDialogMessageW", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "IsDialogMessageA", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "GetDlgItem", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "SendDlgItemMessageW", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("user32.dll", "SendDlgItemMessageA", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("user32.dll", "SetDlgItemTextW", |c| { c.ret_stdcall(1, 3); Handled::Ok }),
        ("user32.dll", "GetDlgItemTextW", |c| { c.ret_stdcall(0, 4); Handled::Ok }),
        ("user32.dll", "CheckDlgButton", |c| { c.ret_stdcall(1, 3); Handled::Ok }),
        ("user32.dll", "IsDlgButtonChecked", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
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
        ("user32.dll", "LoadStringA", |c| {
            let out = c.arg(2);
            let max = c.arg(3);
            if out != 0 && max > 0 {
                let _ = c.memory.write_u8(out, 0);
            }
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("user32.dll", "LoadStringW", |c| {
            let out = c.arg(2);
            let max = c.arg(3);
            if out != 0 && max > 0 {
                let _ = c.memory.write_u16(out, 0);
            }
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
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
            c.ret_stdcall(h, 1);
            Handled::Ok
        }),
        ("user32.dll", "GetShellWindow", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("user32.dll", "FindWindowA", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "FindWindowW", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("user32.dll", "GetDoubleClickTime", |c| { c.ret_stdcall(500, 0); Handled::Ok }),
        ("user32.dll", "GetCaretBlinkTime", |c| { c.ret_stdcall(500, 0); Handled::Ok }),
        // RegisterWindowMessage: hand out unique IDs in the 0xC000-0xFFFF range
        // (0 means failure, which makes apps that register many messages misbehave).
        ("user32.dll", "RegisterWindowMessageW", register_window_message),
        ("user32.dll", "RegisterWindowMessageA", register_window_message),
        ("user32.dll", "SystemParametersInfoW", system_parameters_info),
        ("user32.dll", "SystemParametersInfoA", system_parameters_info),
        ("user32.dll", "ReleaseDC", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("user32.dll", "FillRect", fill_rect),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn msgbox_a(ctx: &mut ApiContext) -> Handled {
    let text = ctx.cstr(ctx.arg(1));
    let title = ctx.cstr(ctx.arg(2));
    let style = ctx.arg(3);
    ctx.ui_events
        .push(UiEvent::MessageBox { title, text, style });
    ctx.ret_stdcall(message_box_result(style), 4);
    Handled::Ok
}

fn msgbox_w(ctx: &mut ApiContext) -> Handled {
    let text = ctx.wstr(ctx.arg(1));
    let title = ctx.wstr(ctx.arg(2));
    let style = ctx.arg(3);
    ctx.ui_events
        .push(UiEvent::MessageBox { title, text, style });
    ctx.ret_stdcall(message_box_result(style), 4);
    Handled::Ok
}

fn message_box_result(style: u32) -> u32 {
    match style & 0xF {
        0x1 | 0x3 | 0x5 => 2, // IDCANCEL
        0x4 => 7,             // IDNO
        _ => 1,               // IDOK
    }
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
    ctx.gui.queue.push_back(GuestMsg { hwnd, message: WM_INITDIALOG, wparam: 0, lparam: init_param });
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
    ctx.gui.gdi_objects.insert(h, GdiObject::MemDc { bitmap: 0 });
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
    ctx.gui.gdi_objects.insert(h, GdiObject::Dib { bits, width, height, bpp, top_down });
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
    ctx.gui.gdi_objects.insert(h, GdiObject::Dib { bits, width, height, bpp: 32, top_down: true });
    ctx.ret_stdcall(h, 3);
    Handled::Ok
}

// Resolve a source DC (memory DC with a selected DIB) to its DIB.
fn dib_of_dc(ctx: &ApiContext, dc: u32) -> Option<(u32, i32, i32, u16, bool)> {
    let bitmap = match ctx.gui.gdi_objects.get(&dc) {
        Some(GdiObject::MemDc { bitmap }) => *bitmap,
        Some(GdiObject::Dib { bits, width, height, bpp, top_down }) =>
            return Some((*bits, *width, *height, *bpp, *top_down)),
        None => return None,
    };
    match ctx.gui.gdi_objects.get(&bitmap) {
        Some(GdiObject::Dib { bits, width, height, bpp, top_down }) =>
            Some((*bits, *width, *height, *bpp, *top_down)),
        _ => None,
    }
}

// Read a w*h region from a DIB's guest memory and convert to RGBA8888.
fn read_dib_rgba(
    ctx: &ApiContext, bits: u32, dib_w: i32, dib_h: i32, bpp: u16, top_down: bool,
    sx: i32, sy: i32, w: i32, h: i32,
) -> Vec<u8> {
    let bytespp = (bpp / 8).max(1) as i32;
    let stride = ((dib_w * bpp as i32 + 31) / 32) * 4;
    let mut out = vec![0u8; (w.max(0) * h.max(0) * 4) as usize];
    for row in 0..h {
        // For bottom-up DIBs, row 0 is at the bottom of the buffer.
        let src_row = if top_down { sy + row } else { dib_h - 1 - (sy + row) };
        if src_row < 0 || src_row >= dib_h { continue; }
        for col in 0..w {
            let src_col = sx + col;
            if src_col < 0 || src_col >= dib_w { continue; }
            let off = bits as i64 + src_row as i64 * stride as i64 + src_col as i64 * bytespp as i64;
            let px = ctx.memory.read_u32(off as u32).unwrap_or(0);
            // DIB pixels are BGRA/BGRX little-endian.
            let (b, g, r) = ((px & 0xFF) as u8, ((px >> 8) & 0xFF) as u8, ((px >> 16) & 0xFF) as u8);
            let di = ((row * w + col) * 4) as usize;
            out[di] = r; out[di + 1] = g; out[di + 2] = b; out[di + 3] = 0xFF;
        }
    }
    out
}

// BitBlt(hdcDest, x, y, w, h, hdcSrc, sx, sy, rop) -> blit memory DC to a window.
pub(crate) fn bit_blt(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let (x, y, w, h) = (ctx.arg(1) as i32, ctx.arg(2) as i32, ctx.arg(3) as i32, ctx.arg(4) as i32);
    let src = ctx.arg(5);
    let (sx, sy) = (ctx.arg(6) as i32, ctx.arg(7) as i32);

    if let Some((bits, dw, dh, bpp, td)) = dib_of_dc(ctx, src) {
        if ctx.gui.windows.contains_key(&dest) {
            let pixels = read_dib_rgba(ctx, bits, dw, dh, bpp, td, sx, sy, w, h);
            ctx.ui_events.push(UiEvent::Blit { hwnd: dest, x, y, w, h, src_w: w, src_h: h, pixels });
        }
    }
    ctx.ret_stdcall(1, 9);
    Handled::Ok
}

// StretchBlt(hdcDest, x, y, w, h, hdcSrc, sx, sy, sw, sh, rop) -> 11 args.
pub(crate) fn stretch_blt(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let (x, y, w, h) = (ctx.arg(1) as i32, ctx.arg(2) as i32, ctx.arg(3) as i32, ctx.arg(4) as i32);
    let src = ctx.arg(5);
    let (sx, sy, sw, sh) = (ctx.arg(6) as i32, ctx.arg(7) as i32, ctx.arg(8) as i32, ctx.arg(9) as i32);

    if let Some((bits, dw, dh, bpp, td)) = dib_of_dc(ctx, src) {
        if ctx.gui.windows.contains_key(&dest) {
            let pixels = read_dib_rgba(ctx, bits, dw, dh, bpp, td, sx, sy, sw, sh);
            ctx.ui_events.push(UiEvent::Blit { hwnd: dest, x, y, w, h, src_w: sw, src_h: sh, pixels });
        }
    }
    ctx.ret_stdcall(1, 11);
    Handled::Ok
}

// StretchDIBits(hdc, xDest,yDest,wDest,hDest, xSrc,ySrc,wSrc,hSrc, *bits, *bmi,
//               usage, rop) -> 13 args. Blits a caller-supplied DIB buffer.
pub(crate) fn stretch_dibits(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let (xd, yd, wd, hd) = (ctx.arg(1) as i32, ctx.arg(2) as i32, ctx.arg(3) as i32, ctx.arg(4) as i32);
    let (xs, ys, ws, hs) = (ctx.arg(5) as i32, ctx.arg(6) as i32, ctx.arg(7) as i32, ctx.arg(8) as i32);
    let bits = ctx.arg(9);
    let bmi = ctx.arg(10);
    let dib_w = ctx.memory.read_u32(bmi + 4).unwrap_or(0) as i32;
    let raw_h = ctx.memory.read_u32(bmi + 8).unwrap_or(0) as i32;
    let bpp = ((ctx.memory.read_u32(bmi + 12).unwrap_or(0) >> 16) as u16).max(1);

    if ctx.gui.windows.contains_key(&dest) {
        let pixels = read_dib_rgba(ctx, bits, dib_w, raw_h.abs(), bpp, raw_h < 0, xs, ys, ws, hs);
        ctx.ui_events.push(UiEvent::Blit { hwnd: dest, x: xd, y: yd, w: wd, h: hd, src_w: ws, src_h: hs, pixels });
    }
    ctx.ret_stdcall(hs.max(0) as u32, 13);
    Handled::Ok
}

// SetDIBitsToDevice(hdc, xDest,yDest, w,h, xSrc,ySrc, startScan,scanLines,
//                   *bits, *bmi, usage) -> 12 args.
pub(crate) fn set_dibits_to_device(ctx: &mut ApiContext) -> Handled {
    let dest = ctx.arg(0);
    let (xd, yd, w, h) = (ctx.arg(1) as i32, ctx.arg(2) as i32, ctx.arg(3) as i32, ctx.arg(4) as i32);
    let (xs, ys) = (ctx.arg(5) as i32, ctx.arg(6) as i32);
    let bits = ctx.arg(9);
    let bmi = ctx.arg(10);
    let dib_w = ctx.memory.read_u32(bmi + 4).unwrap_or(0) as i32;
    let raw_h = ctx.memory.read_u32(bmi + 8).unwrap_or(0) as i32;
    let bpp = ((ctx.memory.read_u32(bmi + 12).unwrap_or(0) >> 16) as u16).max(1);

    if ctx.gui.windows.contains_key(&dest) {
        let pixels = read_dib_rgba(ctx, bits, dib_w, raw_h.abs(), bpp, raw_h < 0, xs, ys, w, h);
        ctx.ui_events.push(UiEvent::Blit { hwnd: dest, x: xd, y: yd, w, h, src_w: w, src_h: h, pixels });
    }
    ctx.ret_stdcall(h.max(0) as u32, 12);
    Handled::Ok
}

// GetDeviceCaps(hdc, index) -> capability value.
pub(crate) fn get_device_caps(ctx: &mut ApiContext) -> Handled {
    let index = ctx.arg(1);
    let v = match index {
        8 => 1920,   // HORZRES
        10 => 1080,  // VERTRES
        12 => 32,    // BITSPIXEL
        14 => 1,     // PLANES
        88 | 90 => 96, // LOGPIXELSX / LOGPIXELSY (DPI)
        104 => 1,    // SHADEBLENDCAPS
        _ => 0,
    };
    ctx.ret_stdcall(v, 2);
    Handled::Ok
}
