//! gdiplus.dll — GDI+ flat API (enough for startup + image draw paths).
//!
//! Object state is stored in the process `dll_state` map under stable handle keys.
//! Handles are non-zero tokens; callers that only Create/Dispose/Draw progress.

use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::winapi::context::ApiRuntimeEnv;

// GpStatus
const OK: u32 = 0;
const INVALID_PARAMETER: u32 = 2;
const OUT_OF_MEMORY: u32 = 3;

// Handle tags in the high nibble so we can tell kinds apart in logs.
const TAG_GRAPHICS: u32 = 0x1000_0000;
const TAG_IMAGE: u32 = 0x2000_0000;
const TAG_MASK: u32 = 0xF000_0000;

pub fn register(r: &mut WinApiRegistry) {
    r.add("gdiplus.dll", "GdiplusStartup", gdiplus_startup);
    r.add("gdiplus.dll", "GdiplusShutdown", gdiplus_shutdown);
    r.add("gdiplus.dll", "GdipAlloc", gdip_alloc);
    r.add("gdiplus.dll", "GdipFree", gdip_free);
    r.add("gdiplus.dll", "GdipCreateFromHDC", gdip_create_from_hdc);
    r.add("gdiplus.dll", "GdipCreateFromHWND", gdip_create_from_hwnd);
    r.add("gdiplus.dll", "GdipDeleteGraphics", gdip_delete_graphics);
    r.add("gdiplus.dll", "GdipCreateBitmapFromScan0", gdip_create_bitmap_from_scan0);
    r.add(
        "gdiplus.dll",
        "GdipCreateBitmapFromHBITMAP",
        gdip_create_bitmap_from_hbitmap,
    );
    r.add("gdiplus.dll", "GdipDisposeImage", gdip_dispose_image);
    r.add("gdiplus.dll", "GdipGetImageWidth", gdip_get_image_width);
    r.add("gdiplus.dll", "GdipGetImageHeight", gdip_get_image_height);
    r.add("gdiplus.dll", "GdipDrawImageRectI", gdip_draw_image_rect_i);
    r.add("gdiplus.dll", "GdipDrawImageI", gdip_draw_image_i);
    r.add("gdiplus.dll", "GdipDrawImageRect", gdip_draw_image_rect_i);
    r.add("gdiplus.dll", "GdipSetSmoothingMode", gdip_set_smoothing_mode);
    r.add(
        "gdiplus.dll",
        "GdipSetInterpolationMode",
        gdip_set_interpolation_mode,
    );
    r.add("gdiplus.dll", "GdipGraphicsClear", gdip_graphics_clear);
    r.add("gdiplus.dll", "GdipGetImageGraphicsContext", gdip_get_image_graphics);
    r.add("gdiplus.dll", "GdipCloneImage", gdip_clone_image);
    r.add("gdiplus.dll", "GdipGetImagePixelFormat", gdip_get_image_pixel_format);
    r.add("gdiplus.dll", "GdipImageGetFrameCount", gdip_image_get_frame_count);
    r.add("gdiplus.dll", "GdipImageSelectActiveFrame", gdip_image_select_frame);
}

fn gdiplus_startup(c: &mut ApiContext) -> Handled {
    // GdiplusStartup(token*, input*, output*) → GpStatus
    let token_ptr = c.arg(0);
    if token_ptr == 0 {
        c.return_stdcall(INVALID_PARAMETER, 3);
        return Handled::Ok;
    }
    let token = next_token(c, 0x6000_0000);
    c.write_u32(token_ptr, token);
    c.dll_state.insert(format!("gdiplus.token.{token}"), 1);
    c.return_stdcall(OK, 3);
    Handled::Ok
}

fn gdiplus_shutdown(c: &mut ApiContext) -> Handled {
    let token = c.arg(0);
    c.dll_state.remove(&format!("gdiplus.token.{token}"));
    c.return_stdcall(0, 1); // void
    Handled::Ok
}

fn gdip_alloc(c: &mut ApiContext) -> Handled {
    let size = c.arg(0).max(1);
    let p = c.heap_alloc(size);
    c.return_stdcall(p, 1);
    Handled::Ok
}

fn gdip_free(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p != 0 {
        c.heap_sizes.remove(&p);
    }
    c.return_stdcall(0, 1); // void
    Handled::Ok
}

fn gdip_create_from_hdc(c: &mut ApiContext) -> Handled {
    // GdipCreateFromHDC(hdc, graphics**)
    let hdc = c.arg(0);
    let out = c.arg(1);
    if out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    let g = alloc_handle(c, TAG_GRAPHICS);
    c.dll_state.insert(format!("gdiplus.g.{g}"), hdc);
    c.write_u32(out, g);
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_create_from_hwnd(c: &mut ApiContext) -> Handled {
    // GdipCreateFromHWND(hwnd, graphics**)
    let hwnd = c.arg(0);
    let out = c.arg(1);
    if out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    let g = alloc_handle(c, TAG_GRAPHICS);
    c.dll_state.insert(format!("gdiplus.g.{g}"), hwnd);
    c.write_u32(out, g);
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_delete_graphics(c: &mut ApiContext) -> Handled {
    let g = c.arg(0);
    if g == 0 || g & TAG_MASK != TAG_GRAPHICS {
        c.return_stdcall(INVALID_PARAMETER, 1);
        return Handled::Ok;
    }
    c.dll_state.remove(&format!("gdiplus.g.{g}"));
    c.return_stdcall(OK, 1);
    Handled::Ok
}

fn gdip_create_bitmap_from_scan0(c: &mut ApiContext) -> Handled {
    // GdipCreateBitmapFromScan0(width, height, stride, format, scan0, bitmap**)
    let width = c.arg(0);
    let height = c.arg(1);
    let _stride = c.arg(2);
    let format = c.arg(3);
    let scan0 = c.arg(4);
    let out = c.arg(5);
    if out == 0 || width == 0 || height == 0 {
        c.return_stdcall(INVALID_PARAMETER, 6);
        return Handled::Ok;
    }
    let img = alloc_handle(c, TAG_IMAGE);
    // Pack width | height into dll_state; format/scan0 in sibling keys.
    c.dll_state
        .insert(format!("gdiplus.img.{img}.w"), width);
    c.dll_state
        .insert(format!("gdiplus.img.{img}.h"), height);
    c.dll_state
        .insert(format!("gdiplus.img.{img}.fmt"), format);
    c.dll_state
        .insert(format!("gdiplus.img.{img}.scan0"), scan0);
    c.write_u32(out, img);
    c.return_stdcall(OK, 6);
    Handled::Ok
}

fn gdip_create_bitmap_from_hbitmap(c: &mut ApiContext) -> Handled {
    // GdipCreateBitmapFromHBITMAP(hbm, hpal, bitmap**)
    let _hbm = c.arg(0);
    let _hpal = c.arg(1);
    let out = c.arg(2);
    if out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 3);
        return Handled::Ok;
    }
    let img = alloc_handle(c, TAG_IMAGE);
    // Default 1×1 placeholder when we can't inspect the HBITMAP.
    c.dll_state.insert(format!("gdiplus.img.{img}.w"), 1);
    c.dll_state.insert(format!("gdiplus.img.{img}.h"), 1);
    c.dll_state
        .insert(format!("gdiplus.img.{img}.fmt"), 0x0002_6200); // PixelFormat32bppARGB
    c.write_u32(out, img);
    c.return_stdcall(OK, 3);
    Handled::Ok
}

fn gdip_dispose_image(c: &mut ApiContext) -> Handled {
    let img = c.arg(0);
    if img == 0 || img & TAG_MASK != TAG_IMAGE {
        c.return_stdcall(INVALID_PARAMETER, 1);
        return Handled::Ok;
    }
    for suffix in ["w", "h", "fmt", "scan0"] {
        c.dll_state
            .remove(&format!("gdiplus.img.{img}.{suffix}"));
    }
    c.return_stdcall(OK, 1);
    Handled::Ok
}

fn gdip_get_image_width(c: &mut ApiContext) -> Handled {
    let img = c.arg(0);
    let out = c.arg(1);
    if img == 0 || out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    let w = c
        .dll_state
        .get(&format!("gdiplus.img.{img}.w"))
        .copied()
        .unwrap_or(0);
    c.write_u32(out, w);
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_get_image_height(c: &mut ApiContext) -> Handled {
    let img = c.arg(0);
    let out = c.arg(1);
    if img == 0 || out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    let h = c
        .dll_state
        .get(&format!("gdiplus.img.{img}.h"))
        .copied()
        .unwrap_or(0);
    c.write_u32(out, h);
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_draw_image_rect_i(c: &mut ApiContext) -> Handled {
    // GdipDrawImageRectI(graphics, image, x, y, w, h) — 6 args
    let g = c.arg(0);
    let img = c.arg(1);
    if g == 0 || img == 0 {
        c.return_stdcall(INVALID_PARAMETER, 6);
        return Handled::Ok;
    }
    // Drawing is host-side later; acknowledge success so UI loops continue.
    c.return_stdcall(OK, 6);
    Handled::Ok
}

fn gdip_draw_image_i(c: &mut ApiContext) -> Handled {
    // GdipDrawImageI(graphics, image, x, y) — 4 args
    let g = c.arg(0);
    let img = c.arg(1);
    if g == 0 || img == 0 {
        c.return_stdcall(INVALID_PARAMETER, 4);
        return Handled::Ok;
    }
    c.return_stdcall(OK, 4);
    Handled::Ok
}

fn gdip_set_smoothing_mode(c: &mut ApiContext) -> Handled {
    let g = c.arg(0);
    if g == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_set_interpolation_mode(c: &mut ApiContext) -> Handled {
    let g = c.arg(0);
    if g == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_graphics_clear(c: &mut ApiContext) -> Handled {
    let g = c.arg(0);
    if g == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_get_image_graphics(c: &mut ApiContext) -> Handled {
    // GdipGetImageGraphicsContext(image, graphics**)
    let img = c.arg(0);
    let out = c.arg(1);
    if img == 0 || out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    let g = alloc_handle(c, TAG_GRAPHICS);
    c.dll_state.insert(format!("gdiplus.g.{g}"), img);
    c.write_u32(out, g);
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_clone_image(c: &mut ApiContext) -> Handled {
    let img = c.arg(0);
    let out = c.arg(1);
    if img == 0 || out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    let clone = alloc_handle(c, TAG_IMAGE);
    for suffix in ["w", "h", "fmt", "scan0"] {
        if let Some(v) = c.dll_state.get(&format!("gdiplus.img.{img}.{suffix}")).copied() {
            c.dll_state
                .insert(format!("gdiplus.img.{clone}.{suffix}"), v);
        }
    }
    c.write_u32(out, clone);
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_get_image_pixel_format(c: &mut ApiContext) -> Handled {
    let img = c.arg(0);
    let out = c.arg(1);
    if img == 0 || out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 2);
        return Handled::Ok;
    }
    let fmt = c
        .dll_state
        .get(&format!("gdiplus.img.{img}.fmt"))
        .copied()
        .unwrap_or(0x0002_6200);
    c.write_u32(out, fmt);
    c.return_stdcall(OK, 2);
    Handled::Ok
}

fn gdip_image_get_frame_count(c: &mut ApiContext) -> Handled {
    let img = c.arg(0);
    let _dim = c.arg(1);
    let out = c.arg(2);
    if img == 0 || out == 0 {
        c.return_stdcall(INVALID_PARAMETER, 3);
        return Handled::Ok;
    }
    c.write_u32(out, 1);
    c.return_stdcall(OK, 3);
    Handled::Ok
}

fn gdip_image_select_frame(c: &mut ApiContext) -> Handled {
    let img = c.arg(0);
    if img == 0 {
        c.return_stdcall(INVALID_PARAMETER, 3);
        return Handled::Ok;
    }
    c.return_stdcall(OK, 3);
    Handled::Ok
}

fn alloc_handle(c: &mut ApiContext, tag: u32) -> u32 {
    let n = c
        .dll_state
        .entry("gdiplus.next".into())
        .or_insert(1);
    let id = *n;
    *n = n.wrapping_add(1);
    if id == 0 {
        // never return a null handle
        *n = 2;
        return tag | 1;
    }
    tag | (id & 0x0FFF_FFFF)
}

fn next_token(c: &mut ApiContext, base: u32) -> u32 {
    let n = c
        .dll_state
        .entry("gdiplus.tok_seq".into())
        .or_insert(1);
    let id = *n;
    *n = n.wrapping_add(1);
    base | (id & 0x0FFF_FFFF)
}

#[allow(dead_code)]
fn _out_of_memory() -> u32 {
    OUT_OF_MEMORY
}
