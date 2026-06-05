use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[

        // gdi32 drawing
        ("gdi32.dll", "TextOutA", crate::user32::text_out_a),
        ("gdi32.dll", "TextOutW", crate::user32::text_out_w),
        ("gdi32.dll", "SetTextColor", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("gdi32.dll", "SetBkMode", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("gdi32.dll", "GetStockObject", crate::user32::get_stock_object),
        ("gdi32.dll", "CreateSolidBrush", crate::user32::create_solid_brush),
        ("gdi32.dll", "CreatePen", crate::user32::create_pen),
        ("gdi32.dll", "SelectObject", crate::user32::select_object),
        ("gdi32.dll", "DeleteObject", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("gdi32.dll", "MoveToEx", crate::user32::move_to_ex),
        ("gdi32.dll", "LineTo", crate::user32::line_to),
        ("gdi32.dll", "Rectangle", crate::user32::gdi_rectangle),
        ("gdi32.dll", "Ellipse", crate::user32::gdi_ellipse),
        ("gdi32.dll", "SetPixel", crate::user32::gdi_set_pixel),
        // gdi32 framebuffer (DIB section + blit) â€” the SDL/windib video path.
        ("gdi32.dll", "CreateCompatibleDC", crate::user32::create_compatible_dc),
        ("gdi32.dll", "CreateDIBSection", crate::user32::create_dib_section),
        ("gdi32.dll", "CreateCompatibleBitmap", crate::user32::create_compatible_bitmap),
        ("gdi32.dll", "DeleteDC", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("gdi32.dll", "BitBlt", crate::user32::bit_blt),
        ("gdi32.dll", "StretchBlt", crate::user32::stretch_blt),
        ("gdi32.dll", "StretchDIBits", crate::user32::stretch_dibits),
        ("gdi32.dll", "SetDIBitsToDevice", crate::user32::set_dibits_to_device),
        ("gdi32.dll", "GetDeviceCaps", crate::user32::get_device_caps),
        ("gdi32.dll", "GetDIBits", |c| { c.ret_stdcall(0, 7); Handled::Ok }),
        // palette / pixel-format / gamma: stubbed with correct arg counts so the
        // guest stack stays balanced (32bpp DIBs don't need a palette).
        ("gdi32.dll", "ChoosePixelFormat", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("gdi32.dll", "SetPixelFormat", |c| { c.ret_stdcall(1, 3); Handled::Ok }),
        ("gdi32.dll", "DescribePixelFormat", |c| { c.ret_stdcall(1, 4); Handled::Ok }),
        ("gdi32.dll", "SwapBuffers", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("gdi32.dll", "CreatePalette", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("gdi32.dll", "SelectPalette", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("gdi32.dll", "RealizePalette", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("gdi32.dll", "UnrealizeObject", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("gdi32.dll", "SetDIBColorTable", |c| { c.ret_stdcall(0, 4); Handled::Ok }),
        ("gdi32.dll", "SetPaletteEntries", |c| { c.ret_stdcall(0, 4); Handled::Ok }),
        ("gdi32.dll", "GetSystemPaletteEntries", |c| { c.ret_stdcall(0, 4); Handled::Ok }),
        ("gdi32.dll", "GetSystemPaletteUse", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("gdi32.dll", "SetSystemPaletteUse", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("gdi32.dll", "GetDeviceGammaRamp", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("gdi32.dll", "SetDeviceGammaRamp", |c| { c.ret_stdcall(1, 2); Handled::Ok }),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
