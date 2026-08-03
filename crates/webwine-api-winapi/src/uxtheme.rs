//! uxtheme.dll — visual styles. We run classic (unthemed) so most calls are
//! no-ops / NULL HTHEME; apps that delay-load these must still resolve.

use super::{ApiContext, Handled, WinApiRegistry};

const THEME_FLAGS_KEY: &str = "uxtheme.app_props";

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        // void WINAPI SetThemeAppProperties(DWORD dwFlags);
        ("uxtheme.dll", "SetThemeAppProperties", set_theme_app_properties),
        // DWORD WINAPI GetThemeAppProperties(void);
        ("uxtheme.dll", "GetThemeAppProperties", get_theme_app_properties),
        ("uxtheme.dll", "IsThemeActive", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("uxtheme.dll", "IsAppThemed", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("uxtheme.dll", "IsThemeDialogTextureEnabled", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("uxtheme.dll", "IsCompositionActive", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        // HTHEME OpenThemeData(HWND, LPCWSTR) → NULL = classic controls
        ("uxtheme.dll", "OpenThemeData", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("uxtheme.dll", "OpenThemeDataEx", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("uxtheme.dll", "CloseThemeData", |c| {
            c.ret_stdcall(0, 1); // S_OK
            Handled::Ok
        }),
        ("uxtheme.dll", "GetWindowTheme", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("uxtheme.dll", "SetWindowTheme", |c| {
            c.ret_stdcall(0, 3); // S_OK
            Handled::Ok
        }),
        ("uxtheme.dll", "EnableThemeDialogTexture", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("uxtheme.dll", "DrawThemeBackground", |c| {
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        ("uxtheme.dll", "DrawThemeBackgroundEx", |c| {
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        ("uxtheme.dll", "DrawThemeText", |c| {
            c.ret_stdcall(0, 9);
            Handled::Ok
        }),
        ("uxtheme.dll", "DrawThemeTextEx", |c| {
            c.ret_stdcall(0, 9);
            Handled::Ok
        }),
        ("uxtheme.dll", "DrawThemeEdge", |c| {
            c.ret_stdcall(0, 8);
            Handled::Ok
        }),
        ("uxtheme.dll", "DrawThemeIcon", |c| {
            c.ret_stdcall(0, 7);
            Handled::Ok
        }),
        ("uxtheme.dll", "DrawThemeParentBackground", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeColor", get_theme_color),
        ("uxtheme.dll", "GetThemePartSize", |c| {
            // HRESULT; fill SIZE with zeros if present
            let psz = c.arg(5);
            if psz != 0 {
                let _ = c.memory.write_u32(psz, 0);
                let _ = c.memory.write_u32(psz + 4, 0);
            }
            c.ret_stdcall(0, 7);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeBackgroundContentRect", |c| {
            c.ret_stdcall(0x8000_4005, 6); // E_FAIL — no theme
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeBackgroundExtent", |c| {
            c.ret_stdcall(0x8000_4005, 6);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeTextExtent", |c| {
            c.ret_stdcall(0x8000_4005, 9);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeMetric", |c| {
            let out = c.arg(5);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0x8000_4005, 6);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeInt", |c| {
            let out = c.arg(5);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0x8000_4005, 6);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeBool", |c| {
            let out = c.arg(5);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0x8000_4005, 6);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeSysColor", |c| {
            // COLORREF black when unthemed
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeSysColorBrush", |c| {
            c.ret_stdcall(0, 2); // NULL HBRUSH
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeSysFont", |c| {
            c.ret_stdcall(0x8000_4005, 3);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeSysSize", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetCurrentThemeName", |c| {
            c.ret_stdcall(0x8000_4005, 6); // E_FAIL
            Handled::Ok
        }),
        ("uxtheme.dll", "GetThemeDocumentationProperty", |c| {
            c.ret_stdcall(0x8000_4005, 4);
            Handled::Ok
        }),
        ("uxtheme.dll", "BufferedPaintInit", |c| {
            c.ret_stdcall(0, 0); // S_OK
            Handled::Ok
        }),
        ("uxtheme.dll", "BufferedPaintUnInit", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("uxtheme.dll", "BeginBufferedPaint", |c| {
            c.ret_stdcall(0, 5); // NULL HPAINTBUFFER
            Handled::Ok
        }),
        ("uxtheme.dll", "EndBufferedPaint", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("uxtheme.dll", "BufferedPaintClear", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("uxtheme.dll", "BufferedPaintSetAlpha", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetBufferedPaintBits", |c| {
            c.ret_stdcall(0x8000_4005, 3);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetBufferedPaintDC", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetBufferedPaintTargetRect", |c| {
            c.ret_stdcall(0x8000_4005, 2);
            Handled::Ok
        }),
        ("uxtheme.dll", "GetBufferedPaintTargetDC", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        // DWM companion often delay-loaded with themes
        ("dwmapi.dll", "DwmIsCompositionEnabled", |c| {
            let out = c.arg(0);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0, 1); // S_OK, composition off
            Handled::Ok
        }),
        ("dwmapi.dll", "DwmExtendFrameIntoClientArea", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("dwmapi.dll", "DwmSetWindowAttribute", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("dwmapi.dll", "DwmGetWindowAttribute", |c| {
            c.ret_stdcall(0x8007_0057, 4); // E_INVALIDARG
            Handled::Ok
        }),
        ("dwmapi.dll", "DwmEnableBlurBehindWindow", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("dwmapi.dll", "DwmDefWindowProc", |c| {
            let handled = c.arg(4);
            if handled != 0 {
                let _ = c.memory.write_u32(handled, 0);
            }
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn set_theme_app_properties(c: &mut ApiContext) -> Handled {
    let flags = c.arg(0);
    c.dll_state.insert(THEME_FLAGS_KEY.into(), flags);
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn get_theme_app_properties(c: &mut ApiContext) -> Handled {
    // Default classic: no STAP_* bits. If the app set flags, echo them.
    let flags = c.dll_state.get(THEME_FLAGS_KEY).copied().unwrap_or(0);
    c.ret_stdcall(flags, 0);
    Handled::Ok
}

fn get_theme_color(c: &mut ApiContext) -> Handled {
    let out = c.arg(5);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(0x8000_4005, 6); // E_FAIL
    Handled::Ok
}
