use super::{ApiContext, Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("user32.dll", "MessageBoxA",     msgbox_a),
        ("user32.dll", "MessageBoxW",     msgbox_w),
        ("user32.dll", "MessageBoxExA",   msgbox_a),
    ];
    for &(dll, name, f) in fns { r.add(dll, name, f); }
}

fn msgbox_a(ctx: &mut ApiContext) -> Handled {
    let text  = ctx.cstr(ctx.arg(1));
    let title = ctx.cstr(ctx.arg(2));
    // Emit as stderr so it shows up in the process console
    let msg = format!("[MessageBox] {title}: {text}\n");
    ctx.console.stderr.extend_from_slice(msg.as_bytes());
    ctx.ret_stdcall(1, 4); Handled::Ok // IDOK
}

fn msgbox_w(ctx: &mut ApiContext) -> Handled {
    let text  = ctx.wstr(ctx.arg(1));
    let title = ctx.wstr(ctx.arg(2));
    let msg = format!("[MessageBox] {title}: {text}\n");
    ctx.console.stderr.extend_from_slice(msg.as_bytes());
    ctx.ret_stdcall(1, 4); Handled::Ok
}
