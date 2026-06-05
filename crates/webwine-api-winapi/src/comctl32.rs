use super::{Handled, WinApiRegistry};

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
        ("comctl32.dll", "_TrackMouseEvent", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("comctl32.dll", "CreateUpDownControl", |c| {
            c.ret_stdcall(0, 12);
            Handled::Ok
        }),
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
