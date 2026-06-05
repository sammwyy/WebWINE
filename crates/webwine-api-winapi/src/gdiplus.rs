use super::{ApiContext, Handled, WinApiRegistry};
use crate::util::{register_entries, ret_0_1, ret_0_2, ret_0_3, ret_0_4, ret_0_6, Entry};
use webwine_api::winapi::context::ApiRuntimeEnv;

pub fn register(r: &mut WinApiRegistry) {
    register_entries(r, ENTRIES);
}

const ENTRIES: &[Entry] = &[
    ("gdiplus.dll", "GdiplusStartup", gdiplus_startup),
    ("gdiplus.dll", "GdiplusShutdown", ret_0_1),
    ("gdiplus.dll", "GdipAlloc", gdip_alloc),
    ("gdiplus.dll", "GdipFree", ret_0_1),
    ("gdiplus.dll", "GdipCreateFromHDC", ret_0_2),
    ("gdiplus.dll", "GdipDeleteGraphics", ret_0_1),
    ("gdiplus.dll", "GdipCreateBitmapFromScan0", ret_0_6),
    ("gdiplus.dll", "GdipCreateBitmapFromHBITMAP", ret_0_3),
    ("gdiplus.dll", "GdipDisposeImage", ret_0_1),
    ("gdiplus.dll", "GdipGetImageWidth", ret_0_2),
    ("gdiplus.dll", "GdipGetImageHeight", ret_0_2),
    ("gdiplus.dll", "GdipDrawImageRectI", ret_0_6),
    ("gdiplus.dll", "GdipDrawImageI", ret_0_4),
    ("gdiplus.dll", "GdipSetSmoothingMode", ret_0_2),
    ("gdiplus.dll", "GdipSetInterpolationMode", ret_0_2),
];

fn gdiplus_startup(c: &mut ApiContext) -> Handled {
    let token_ptr = c.arg(0);
    if token_ptr != 0 {
        c.write_u32(token_ptr, 1);
    }
    c.return_stdcall(0, 3);
    Handled::Ok
}

fn gdip_alloc(c: &mut ApiContext) -> Handled {
    let size = c.arg(0).max(1);
    let p = c.heap_alloc(size);
    c.return_stdcall(p, 1);
    Handled::Ok
}
