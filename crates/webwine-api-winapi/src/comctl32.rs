use super::{Handled, WinApiRegistry};
use webwine_api::vm::process::WindowEntry;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[

        ("comctl32.dll", "InitCommonControlsEx", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        // comctl32 â€” common controls (putty's config dialog uses drag lists etc.)
        ("comctl32.dll", "InitCommonControls", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("comctl32.dll", "DrawInsert", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("comctl32.dll", "LBItemFromPt", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 4);
            Handled::Ok
        }),
        ("comctl32.dll", "MakeDragList", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("comctl32.dll", "ImageList_Create", |c| {
            c.ret_stdcall(0x494C_0001, 5);
            Handled::Ok
        }),
        ("comctl32.dll", "ImageList_Destroy", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("comctl32.dll", "ImageList_AddMasked", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("comctl32.dll", "ImageList_ReplaceIcon", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("comctl32.dll", "ImageList_GetIconSize", |c| {
            if c.arg(1) != 0 { let _ = c.memory.write_u32(c.arg(1), 16); }
            if c.arg(2) != 0 { let _ = c.memory.write_u32(c.arg(2), 16); }
            c.ret_stdcall(1, 3);
            Handled::Ok
        }),
        ("comctl32.dll", "ImageList_GetIcon", |c| { c.ret_stdcall(1, 3); Handled::Ok }),
        ("comctl32.dll", "#328", |c| {
            let state = c.heap_alloc(16);
            c.ret_stdcall(state, 1);
            Handled::Ok
        }),
        ("comctl32.dll", "#334", |c| { c.ret_stdcall(c.arg(1), 3); Handled::Ok }),
        ("comctl32.dll", "_TrackMouseEvent", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("comctl32.dll", "CreateUpDownControl", |c| {
            c.ret_stdcall(0, 12);
            Handled::Ok
        }),
        ("comctl32.dll", "CreateStatusWindowA", create_status_window),
        ("comctl32.dll", "CreateStatusWindowW", create_status_window),
        ("comctl32.dll", "PropertySheetW", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("comctl32.dll", "PropertySheetA", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn create_status_window(c: &mut super::ApiContext) -> Handled {
    let hwnd = c.gui.next_hwnd;
    c.gui.next_hwnd += 4;
    c.gui.windows.insert(hwnd, WindowEntry {
        wndproc: 0,
        needs_paint: false,
        width: 400,
        height: 24,
        pen_color: 0,
        brush_color: 0xF0_F0F0,
        cur_x: 0,
        cur_y: 0,
    });
    c.ret_stdcall(hwnd, 4);
    Handled::Ok
}
