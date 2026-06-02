use super::{ApiContext, Handled, WinApiRegistry};
use crate::vm::process::UiEvent;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("user32.dll", "MessageBoxA",   msgbox_a),
        ("user32.dll", "MessageBoxW",   msgbox_w),
        ("user32.dll", "MessageBoxExA", msgbox_a),
        ("user32.dll", "MessageBoxExW", msgbox_w),
    ];
    for &(dll, name, f) in fns { r.add(dll, name, f); }
}

// MessageBoxA(hwnd, text, caption, type) — emit a UI event for the frontend
// to render as a real dialog window. Returns IDOK (1).
fn msgbox_a(ctx: &mut ApiContext) -> Handled {
    let text  = ctx.cstr(ctx.arg(1));
    let title = ctx.cstr(ctx.arg(2));
    let style = ctx.arg(3);
    ctx.ui_events.push(UiEvent::MessageBox { title, text, style });
    ctx.ret_stdcall(1, 4); // IDOK
    Handled::Ok
}

fn msgbox_w(ctx: &mut ApiContext) -> Handled {
    let text  = ctx.wstr(ctx.arg(1));
    let title = ctx.wstr(ctx.arg(2));
    let style = ctx.arg(3);
    ctx.ui_events.push(UiEvent::MessageBox { title, text, style });
    ctx.ret_stdcall(1, 4);
    Handled::Ok
}
