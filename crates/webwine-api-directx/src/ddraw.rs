//! DirectDraw 7 stub (ddraw.dll) — enough of the classic "blit-to-primary-surface"
//! 2D path for games like Touhou 6. Output goes out as a `UiEvent::Blit` (RGBA8888),
//! the same seam GDI uses, which the frontend paints onto the window canvas.
//!
//! COM layout / vtable mechanism: see the crate root. Each interface below is a
//! `Vtable` table whose slots map to either a dedicated handler (real logic) or a
//! shared `sV_N` stub (return value V, clean N args). The surface id is stored in
//! the COM object at +4 (`extra`).

use webwine_api::vm::process::{DDrawSurface, UiEvent};
use webwine_api::winapi::context::{ApiContext, Handled};
use webwine_api::winapi::WinApiRegistry;

use crate::{
    com_qi, make_object, register_vtable, s0_1, s0_2, s0_3, s0_4, s1_1, Vtable,
};

// HRESULT constants
const S_OK: u32 = 0x0000_0000;
const DDERR_GENERIC: u32 = 0x8876_03E8;
const DDERR_INVALIDPARAMS: u32 = 0x8876_0057;

// DDSCAPS flags (ddraw.h)
const DDSCAPS_PRIMARYSURFACE: u32 = 0x0000_0200;
const DDSCAPS_OFFSCREENPLAIN: u32 = 0x0000_0040;
const DDSCAPS_FLIP: u32 = 0x0000_0400;

// DDSURFACEDESC2 field offsets (the subset we touch).
const DESC_FLAGS: u32 = 4;
const DESC_HEIGHT: u32 = 8;
const DESC_WIDTH: u32 = 12;
const DESC_PITCH: u32 = 16;
const DESC_BACK_COUNT: u32 = 20;
const DESC_SURFACE_PTR: u32 = 108;
const DESC_CAPS_CAPS1: u32 = 96;

// DDSD flags
const DDSD_HEIGHT: u32 = 0x0000_0002;
const DDSD_WIDTH: u32 = 0x0000_0004;
const DDSD_BACKBUFFERCOUNT: u32 = 0x0000_0020;
const DDSD_PITCH: u32 = 0x0000_0008;

// ─── interface vtables ───────────────────────────────────────────────────────

pub(crate) const IDDRAW7: Vtable = &[
    ("IDirectDraw7::QueryInterface", com_qi),
    ("IDirectDraw7::AddRef", s1_1),
    ("IDirectDraw7::Release", s1_1),
    ("IDirectDraw7::Compact", s0_1),
    ("IDirectDraw7::CreateClipper", iddraw7_create_clipper),
    ("IDirectDraw7::CreatePalette", s0_4),
    ("IDirectDraw7::CreateSurface", iddraw7_create_surface),
    ("IDirectDraw7::DuplicateSurface", s0_1),
    ("IDirectDraw7::EnumDisplayModes", s0_3),
    ("IDirectDraw7::EnumSurfaces", s0_1),
    ("IDirectDraw7::FlipToGDISurface", s0_1),
    ("IDirectDraw7::GetCaps", iddraw7_get_caps),
    ("IDirectDraw7::GetDisplayMode", iddraw7_get_display_mode),
    ("IDirectDraw7::GetFourCCCodes", s0_1),
    ("IDirectDraw7::GetGDISurface", s0_1),
    ("IDirectDraw7::GetMonitorFrequency", iddraw7_get_monitor_frequency),
    ("IDirectDraw7::GetScanLine", s0_1),
    ("IDirectDraw7::GetVerticalBlankStatus", iddraw7_get_vblank_status),
    ("IDirectDraw7::Initialize", s0_1),
    ("IDirectDraw7::RestoreDisplayMode", s0_1),
    ("IDirectDraw7::SetCooperativeLevel", s0_3),
    ("IDirectDraw7::SetDisplayMode", iddraw7_set_display_mode),
    ("IDirectDraw7::WaitForVerticalBlank", s0_3),
    ("IDirectDraw7::GetAvailableVidMem", iddraw7_get_available_vidmem),
    ("IDirectDraw7::GetSurfaceFromDC", s0_1),
    ("IDirectDraw7::RestoreAllSurfaces", s0_1),
    ("IDirectDraw7::TestCooperativeLevel", s0_1),
    ("IDirectDraw7::GetDeviceIdentifier", iddraw7_get_device_identifier),
    ("IDirectDraw7::StartModeTest", s0_1),
    ("IDirectDraw7::EvaluateMode", s0_1),
    ("IDirectDraw7::_reserved30", s0_1),
    ("IDirectDraw7::_reserved31", s0_1),
    ("IDirectDraw7::_reserved32", s0_1),
];

pub(crate) const IDDSURFACE7: Vtable = &[
    ("IDirectDrawSurface7::QueryInterface", com_qi),
    ("IDirectDrawSurface7::AddRef", s1_1),
    ("IDirectDrawSurface7::Release", iddsurface7_release),
    ("IDirectDrawSurface7::AddAttachedSurface", s0_1),
    ("IDirectDrawSurface7::AddOverlayDirtyRect", s0_1),
    ("IDirectDrawSurface7::Blt", iddsurface7_blt),
    ("IDirectDrawSurface7::BltBatch", s0_1),
    ("IDirectDrawSurface7::BltFast", iddsurface7_blt_fast),
    ("IDirectDrawSurface7::DeleteAttachedSurface", s0_1),
    ("IDirectDrawSurface7::EnumAttachedSurfaces", s0_1),
    ("IDirectDrawSurface7::EnumOverlayZOrders", s0_1),
    ("IDirectDrawSurface7::Flip", iddsurface7_flip),
    ("IDirectDrawSurface7::GetAttachedSurface", iddsurface7_get_attached_surface),
    ("IDirectDrawSurface7::GetBltStatus", s0_2),
    ("IDirectDrawSurface7::GetCaps", s0_1),
    ("IDirectDrawSurface7::GetClipper", s0_1),
    ("IDirectDrawSurface7::GetColorKey", s0_1),
    ("IDirectDrawSurface7::GetDC", s0_1),
    ("IDirectDrawSurface7::GetFlipStatus", s0_2),
    ("IDirectDrawSurface7::GetOverlayPosition", s0_1),
    ("IDirectDrawSurface7::GetPalette", s0_1),
    ("IDirectDrawSurface7::GetPixelFormat", s0_1),
    ("IDirectDrawSurface7::GetSurfaceDesc", iddsurface7_get_surface_desc),
    ("IDirectDrawSurface7::Initialize", s0_1),
    ("IDirectDrawSurface7::IsLost", s0_1),
    ("IDirectDrawSurface7::Lock", iddsurface7_lock),
    ("IDirectDrawSurface7::ReleaseDC", s0_1),
    ("IDirectDrawSurface7::Restore", s0_1),
    ("IDirectDrawSurface7::SetClipper", s0_2),
    ("IDirectDrawSurface7::SetColorKey", iddsurface7_set_color_key),
    ("IDirectDrawSurface7::SetOverlayPosition", s0_1),
    ("IDirectDrawSurface7::SetPalette", s0_1),
    ("IDirectDrawSurface7::Unlock", iddsurface7_unlock),
    ("IDirectDrawSurface7::UpdateOverlay", s0_1),
    ("IDirectDrawSurface7::UpdateOverlayDisplay", s0_1),
    ("IDirectDrawSurface7::UpdateOverlayZOrder", s0_1),
    ("IDirectDrawSurface7::GetDDInterface", iddsurface7_get_ddinterface),
];

// Clipper is a pure no-op (just balance the stack). Note: matching the original,
// its AddRef/Release/QI return DD_OK(0), not a refcount, and QI does not fill ppv.
pub(crate) const IDDCLIPPER: Vtable = &[
    ("IDirectDrawClipper::QueryInterface", s0_3),
    ("IDirectDrawClipper::AddRef", s0_1),
    ("IDirectDrawClipper::Release", s0_1),
    ("IDirectDrawClipper::GetClipList", s0_2),
    ("IDirectDrawClipper::GetHWnd", s0_2),
    ("IDirectDrawClipper::Initialize", s0_2),
    ("IDirectDrawClipper::IsClipListChanged", s0_2),
    ("IDirectDrawClipper::SetClipList", s0_3),
];

pub fn register(r: &mut WinApiRegistry) {
    r.add("ddraw.dll", "DirectDrawCreate", ddraw_create);
    r.add("ddraw.dll", "DirectDrawCreateEx", ddraw_create_ex);
    register_vtable(r, IDDRAW7);
    register_vtable(r, IDDSURFACE7);
    register_vtable(r, IDDCLIPPER);
}

// ─── surface-id helper ───────────────────────────────────────────────────────

/// Read the surface-id stored at object+4.
fn surface_id_of(ctx: &ApiContext, obj_va: u32) -> u32 {
    crate::object_extra(ctx, obj_va)
}

// ─── IAT exports ─────────────────────────────────────────────────────────────

/// `DirectDrawCreate(lpGUID, lplpDD, pUnkOuter)` — always returns an IDirectDraw7
/// (the QueryInterface upgrade path is transparent).
fn ddraw_create(ctx: &mut ApiContext) -> Handled {
    let out_ptr = ctx.arg(1);
    let obj_va = make_object(ctx, IDDRAW7, 0);
    if out_ptr != 0 {
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

/// `DirectDrawCreateEx(lpGUID, lplpDD, iid, pUnkOuter)` — iid ignored.
fn ddraw_create_ex(ctx: &mut ApiContext) -> Handled {
    let out_ptr = ctx.arg(1);
    let obj_va = make_object(ctx, IDDRAW7, 0);
    if out_ptr != 0 {
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

// ─── IDirectDraw7 methods ────────────────────────────────────────────────────

fn iddraw7_set_display_mode(ctx: &mut ApiContext) -> Handled {
    // SetDisplayMode(this, dwWidth, dwHeight, dwBPP, dwRefreshRate, dwFlags)
    let w = ctx.arg(1);
    let h = ctx.arg(2);
    let bpp = ctx.arg(3);
    ctx.gui.ddraw_display_w = w;
    ctx.gui.ddraw_display_h = h;
    ctx.gui.ddraw_display_bpp = bpp;
    ctx.ret_stdcall(S_OK, 6);
    Handled::Ok
}

fn iddraw7_create_surface(ctx: &mut ApiContext) -> Handled {
    // CreateSurface(this, lpDDSurfaceDesc2, lplpDDSurface, pUnkOuter) -> HRESULT
    let desc_va = ctx.arg(1);
    let out_ptr = ctx.arg(2);

    if desc_va == 0 || out_ptr == 0 {
        ctx.ret_stdcall(DDERR_INVALIDPARAMS, 4);
        return Handled::Ok;
    }

    let flags = ctx.memory.read_u32(desc_va + DESC_FLAGS).unwrap_or(0);
    let caps = ctx.memory.read_u32(desc_va + DESC_CAPS_CAPS1).unwrap_or(0);

    // Determine dimensions: primary surface uses display mode, others use desc.
    let (w, h, surface_kind) = if caps & DDSCAPS_PRIMARYSURFACE != 0 {
        (
            ctx.gui.ddraw_display_w.max(640),
            ctx.gui.ddraw_display_h.max(480),
            webwine_api::vm::process::DDrawSurfaceKind::Primary,
        )
    } else {
        let dw = if flags & DDSD_WIDTH != 0 {
            ctx.memory.read_u32(desc_va + DESC_WIDTH).unwrap_or(0)
        } else {
            ctx.gui.ddraw_display_w.max(640)
        };
        let dh = if flags & DDSD_HEIGHT != 0 {
            ctx.memory.read_u32(desc_va + DESC_HEIGHT).unwrap_or(0)
        } else {
            ctx.gui.ddraw_display_h.max(480)
        };
        (dw, dh, webwine_api::vm::process::DDrawSurfaceKind::Offscreen)
    };

    // Allocate pixel buffer on the guest heap: 4 bytes per pixel (BGRA8888).
    let stride = w * 4;
    let buf_size = stride * h;
    let pixels_va = ctx.heap_alloc(buf_size.max(4));

    // Assign a surface-id and record it.
    let sid = ctx.gui.next_ddraw_surface;
    ctx.gui.next_ddraw_surface += 1;

    ctx.gui.ddraw_surfaces.insert(
        sid,
        DDrawSurface {
            kind: surface_kind,
            width: w,
            height: h,
            stride,
            pixels_va,
            color_key: None,
            back_id: None,
        },
    );

    // If DDSCAPS_FLIP and backBufferCount ≥ 1, create the back buffer now.
    if caps & DDSCAPS_FLIP != 0 && flags & DDSD_BACKBUFFERCOUNT != 0 {
        let back_count = ctx.memory.read_u32(desc_va + DESC_BACK_COUNT).unwrap_or(0);
        if back_count >= 1 {
            let back_pixels = ctx.heap_alloc(buf_size.max(4));
            let back_sid = ctx.gui.next_ddraw_surface;
            ctx.gui.next_ddraw_surface += 1;
            ctx.gui.ddraw_surfaces.insert(
                back_sid,
                DDrawSurface {
                    kind: webwine_api::vm::process::DDrawSurfaceKind::Offscreen,
                    width: w,
                    height: h,
                    stride,
                    pixels_va: back_pixels,
                    color_key: None,
                    back_id: None,
                },
            );
            // Link back buffer to primary.
            if let Some(s) = ctx.gui.ddraw_surfaces.get_mut(&sid) {
                s.back_id = Some(back_sid);
            }
        }
    }

    // Write pitch and surface pointer back into the DDSURFACEDESC2.
    if flags & DDSD_PITCH != 0 {
        let _ = ctx.memory.write_u32(desc_va + DESC_PITCH, stride);
    }
    let _ = ctx.memory.write_u32(desc_va + DESC_SURFACE_PTR, pixels_va);

    // Allocate and return the COM object.
    let obj_va = make_object(ctx, IDDSURFACE7, sid);
    let _ = ctx.memory.write_u32(out_ptr, obj_va);

    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn iddraw7_create_clipper(ctx: &mut ApiContext) -> Handled {
    // CreateClipper(this, dwFlags, lplpDDClipper, pUnkOuter)
    let out_ptr = ctx.arg(2);
    let obj_va = make_object(ctx, IDDCLIPPER, 0);
    if out_ptr != 0 {
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn iddraw7_get_caps(ctx: &mut ApiContext) -> Handled {
    // GetCaps(this, lpDDDriverCaps, lpDDHELCaps) — fill with generous caps.
    for out in [ctx.arg(1), ctx.arg(2)] {
        if out != 0 {
            let _ = ctx.memory.write_u32(out, 344); // dwSize
            let _ = ctx.memory.write_u32(out + 4, 0xFFFF_FFFF); // dwCaps
            let _ = ctx.memory.write_u32(out + 8, 0xFFFF_FFFF); // dwCaps2
            let _ = ctx.memory.write_u32(out + 12, 0xFFFF_FFFF); // dwCKeyCaps
            let _ = ctx.memory.write_u32(out + 16, 0xFFFF_FFFF); // dwFXCaps
            let _ = ctx.memory.write_u32(out + 80, 32 * 1024 * 1024); // dwVidMemTotal
            let _ = ctx.memory.write_u32(out + 84, 32 * 1024 * 1024); // dwVidMemFree
        }
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn iddraw7_get_display_mode(ctx: &mut ApiContext) -> Handled {
    // GetDisplayMode(this, lpDDSurfaceDesc2)
    let desc = ctx.arg(1);
    if desc != 0 {
        let _ = ctx.memory.write_u32(desc + DESC_WIDTH, ctx.gui.ddraw_display_w);
        let _ = ctx.memory.write_u32(desc + DESC_HEIGHT, ctx.gui.ddraw_display_h);
        let _ = ctx.memory.write_u32(desc + 96, ctx.gui.ddraw_display_bpp);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn iddraw7_get_monitor_frequency(ctx: &mut ApiContext) -> Handled {
    // GetMonitorFrequency(this, lpdwFrequency)
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 60);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn iddraw7_get_vblank_status(ctx: &mut ApiContext) -> Handled {
    // GetVerticalBlankStatus(this, lpbIsInVB)
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn iddraw7_get_available_vidmem(ctx: &mut ApiContext) -> Handled {
    // GetAvailableVidMem(this, lpDDSCaps2, lpdwTotal, lpdwFree)
    let total = ctx.arg(2);
    let free = ctx.arg(3);
    if total != 0 {
        let _ = ctx.memory.write_u32(total, 32 * 1024 * 1024);
    }
    if free != 0 {
        let _ = ctx.memory.write_u32(free, 32 * 1024 * 1024);
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn iddraw7_get_device_identifier(ctx: &mut ApiContext) -> Handled {
    // GetDeviceIdentifier(this, lpDDDeviceIdentifier2, dwFlags)
    let out = ctx.arg(1);
    if out != 0 {
        let name = b"WebWINE DDraw\0";
        let _ = ctx.memory.write_bytes(out, name);
        let _ = ctx.memory.write_u32(out + 524, 0x1234); // UniqueId.dwVendorId
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

// ─── IDirectDrawSurface7 methods ─────────────────────────────────────────────

fn iddsurface7_release(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let sid = surface_id_of(ctx, this);
    ctx.gui.ddraw_surfaces.remove(&sid);
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn iddsurface7_lock(ctx: &mut ApiContext) -> Handled {
    // Lock(this, lpDestRect, lpDDSurfaceDesc, dwFlags, hEvent)
    let this = ctx.arg(0);
    let desc_va = ctx.arg(2);
    let sid = surface_id_of(ctx, this);

    if let Some(surf) = ctx.gui.ddraw_surfaces.get(&sid) {
        if desc_va != 0 {
            let _ = ctx.memory.write_u32(desc_va + DESC_PITCH, surf.stride);
            let _ = ctx.memory.write_u32(desc_va + DESC_SURFACE_PTR, surf.pixels_va);
            let _ = ctx.memory.write_u32(desc_va + DESC_WIDTH, surf.width);
            let _ = ctx.memory.write_u32(desc_va + DESC_HEIGHT, surf.height);
        }
    }
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}

fn iddsurface7_unlock(ctx: &mut ApiContext) -> Handled {
    // Unlock(this, lpRect) — pixels are already in guest mem.
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn iddsurface7_flip(ctx: &mut ApiContext) -> Handled {
    // Flip(this, lpDDSurfaceTargetOverride, dwFlags) — back→primary, emit Blit.
    let this = ctx.arg(0);
    let sid = surface_id_of(ctx, this);

    let (primary_id, back_id, w, h) = {
        if let Some(surf) = ctx.gui.ddraw_surfaces.get(&sid) {
            let primary = if matches!(surf.kind, webwine_api::vm::process::DDrawSurfaceKind::Primary) {
                sid
            } else {
                sid
            };
            (primary, surf.back_id, surf.width, surf.height)
        } else {
            ctx.ret_stdcall(S_OK, 3);
            return Handled::Ok;
        }
    };

    let src_sid = back_id.unwrap_or(primary_id);
    let pixels = read_surface_rgba(ctx, src_sid);
    let hwnd = ctx.gui.windows.keys().copied().next().unwrap_or(4);

    ctx.ui_events.push(UiEvent::Blit {
        hwnd,
        x: 0,
        y: 0,
        w: w as i32,
        h: h as i32,
        src_w: w as i32,
        src_h: h as i32,
        pixels,
    });

    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn iddsurface7_blt(ctx: &mut ApiContext) -> Handled {
    // Blt(this, lpDestRect, lpDDSrcSurface, lpSrcRect, dwFlags, lpDDBltFx)
    let this = ctx.arg(0);
    let dest_rect = ctx.arg(1);
    let src_obj = ctx.arg(2);
    let src_rect = ctx.arg(3);

    let dst_sid = surface_id_of(ctx, this);
    let src_sid = if src_obj != 0 {
        surface_id_of(ctx, src_obj)
    } else {
        0
    };

    let (dst_x, dst_y, dst_w, dst_h) = read_rect_or_full(ctx, dest_rect, dst_sid);
    let (sx, sy, sw, sh) = read_rect_or_full(ctx, src_rect, src_sid);

    if src_obj == 0 {
        let blt_fx = ctx.arg(5);
        let fill_color = if blt_fx != 0 {
            ctx.memory.read_u32(blt_fx + 16).unwrap_or(0)
        } else {
            0
        };
        surface_fill(ctx, dst_sid, dst_x, dst_y, dst_w, dst_h, fill_color);
    } else {
        surface_blit(ctx, src_sid, sx, sy, sw, sh, dst_sid, dst_x, dst_y, dst_w, dst_h);
    }

    ctx.ret_stdcall(S_OK, 6);
    Handled::Ok
}

fn iddsurface7_blt_fast(ctx: &mut ApiContext) -> Handled {
    // BltFast(this, dwX, dwY, lpDDSrcSurface, lpSrcRect, dwTrans)
    let this = ctx.arg(0);
    let dst_x = ctx.arg(1) as i32;
    let dst_y = ctx.arg(2) as i32;
    let src_obj = ctx.arg(3);
    let src_rect = ctx.arg(4);
    let trans = ctx.arg(5);

    let dst_sid = surface_id_of(ctx, this);
    let src_sid = surface_id_of(ctx, src_obj);

    let (sx, sy, sw, sh) = read_rect_or_full(ctx, src_rect, src_sid);
    surface_blit(ctx, src_sid, sx, sy, sw, sh, dst_sid, dst_x, dst_y, sw, sh);
    let _ = trans; // color-key is applied inside surface_blit

    ctx.ret_stdcall(S_OK, 6);
    Handled::Ok
}

fn iddsurface7_set_color_key(ctx: &mut ApiContext) -> Handled {
    // SetColorKey(this, dwFlags, lpDDColorKey)
    let this = ctx.arg(0);
    let ck_struct = ctx.arg(2);
    let sid = surface_id_of(ctx, this);

    if ck_struct != 0 {
        let low = ctx.memory.read_u32(ck_struct).unwrap_or(0);
        if let Some(surf) = ctx.gui.ddraw_surfaces.get_mut(&sid) {
            surf.color_key = Some(low);
        }
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn iddsurface7_get_attached_surface(ctx: &mut ApiContext) -> Handled {
    // GetAttachedSurface(this, lpDDSCaps2, lplpDDAttachedSurface)
    let this = ctx.arg(0);
    let out_ptr = ctx.arg(2);
    let sid = surface_id_of(ctx, this);

    let back_id = ctx.gui.ddraw_surfaces.get(&sid).and_then(|s| s.back_id);

    if let Some(bsid) = back_id {
        let obj_va = make_object(ctx, IDDSURFACE7, bsid);
        if out_ptr != 0 {
            let _ = ctx.memory.write_u32(out_ptr, obj_va);
        }
        ctx.ret_stdcall(S_OK, 3);
    } else {
        ctx.ret_stdcall(DDERR_GENERIC, 3);
    }
    Handled::Ok
}

fn iddsurface7_get_surface_desc(ctx: &mut ApiContext) -> Handled {
    // GetSurfaceDesc(this, lpDDSurfaceDesc2)
    let this = ctx.arg(0);
    let desc_va = ctx.arg(1);
    let sid = surface_id_of(ctx, this);

    if let Some(surf) = ctx.gui.ddraw_surfaces.get(&sid) {
        if desc_va != 0 {
            let caps = match surf.kind {
                webwine_api::vm::process::DDrawSurfaceKind::Primary => {
                    DDSCAPS_PRIMARYSURFACE | DDSCAPS_FLIP
                }
                webwine_api::vm::process::DDrawSurfaceKind::Offscreen => DDSCAPS_OFFSCREENPLAIN,
            };
            let _ = ctx.memory.write_u32(desc_va + DESC_WIDTH, surf.width);
            let _ = ctx.memory.write_u32(desc_va + DESC_HEIGHT, surf.height);
            let _ = ctx.memory.write_u32(desc_va + DESC_PITCH, surf.stride);
            let _ = ctx.memory.write_u32(desc_va + DESC_CAPS_CAPS1, caps);
            let _ = ctx.memory.write_u32(desc_va + DESC_SURFACE_PTR, surf.pixels_va);
        }
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn iddsurface7_get_ddinterface(ctx: &mut ApiContext) -> Handled {
    // GetDDInterface(this, lplpDD) — hand back a fresh IDirectDraw7.
    let out = ctx.arg(1);
    let obj = make_object(ctx, IDDRAW7, 0);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, obj);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

// ─── pixel buffer helpers ────────────────────────────────────────────────────

/// Read rect from a Windows RECT struct, falling back to the full surface if NULL.
fn read_rect_or_full(ctx: &ApiContext, rect_va: u32, sid: u32) -> (i32, i32, i32, i32) {
    if rect_va != 0 {
        let l = ctx.memory.read_u32(rect_va).unwrap_or(0) as i32;
        let t = ctx.memory.read_u32(rect_va + 4).unwrap_or(0) as i32;
        let r = ctx.memory.read_u32(rect_va + 8).unwrap_or(0) as i32;
        let b = ctx.memory.read_u32(rect_va + 12).unwrap_or(0) as i32;
        return (l, t, (r - l).max(0), (b - t).max(0));
    }
    if let Some(surf) = ctx.gui.ddraw_surfaces.get(&sid) {
        (0, 0, surf.width as i32, surf.height as i32)
    } else {
        (0, 0, 0, 0)
    }
}

/// Blit a region from src surface to dst surface in guest memory, honoring the
/// src surface's colour-key transparency.
fn surface_blit(
    ctx: &mut ApiContext,
    src_sid: u32,
    sx: i32,
    sy: i32,
    sw: i32,
    sh: i32,
    dst_sid: u32,
    dx: i32,
    dy: i32,
    dw: i32,
    dh: i32,
) {
    let src_info = ctx.gui.ddraw_surfaces.get(&src_sid).map(|s| {
        (s.pixels_va, s.width as i32, s.height as i32, s.stride as i32, s.color_key)
    });
    let dst_info = ctx.gui.ddraw_surfaces.get(&dst_sid).map(|s| {
        (s.pixels_va, s.width as i32, s.height as i32, s.stride as i32)
    });

    let (sp, sw_full, sh_full, s_stride, color_key) = match src_info {
        Some(v) => v,
        None => return,
    };
    let (dp, _dw_full, _dh_full, d_stride) = match dst_info {
        Some(v) => v,
        None => return,
    };

    let x_scale = if dw > 0 { sw as f32 / dw as f32 } else { 1.0 };
    let y_scale = if dh > 0 { sh as f32 / dh as f32 } else { 1.0 };

    for row in 0..dh {
        let src_row = (sy + (row as f32 * y_scale) as i32).clamp(0, sh_full - 1);
        let dst_row = dy + row;
        if dst_row < 0 {
            continue;
        }

        for col in 0..dw {
            let src_col = (sx + (col as f32 * x_scale) as i32).clamp(0, sw_full - 1);
            let dst_col = dx + col;
            if dst_col < 0 {
                continue;
            }

            let src_off = (src_row * s_stride + src_col * 4) as u32;
            let dst_off = (dst_row * d_stride + dst_col * 4) as u32;

            let pixel = ctx.memory.read_u32(sp + src_off).unwrap_or(0);

            if let Some(ck) = color_key {
                if pixel & 0x00FF_FFFF == ck & 0x00FF_FFFF {
                    continue;
                }
            }

            let _ = ctx.memory.write_u32(dp + dst_off, pixel);
        }
    }
}

/// Fill a region of a surface with a solid colour (for DDBLT_COLORFILL).
fn surface_fill(ctx: &mut ApiContext, sid: u32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    let (pixels_va, surf_w, surf_h, stride) = match ctx.gui.ddraw_surfaces.get(&sid) {
        Some(s) => (s.pixels_va, s.width as i32, s.height as i32, s.stride as i32),
        None => return,
    };
    for row in y..(y + h) {
        if row < 0 || row >= surf_h {
            continue;
        }
        for col in x..(x + w) {
            if col < 0 || col >= surf_w {
                continue;
            }
            let off = (row * stride + col * 4) as u32;
            let _ = ctx.memory.write_u32(pixels_va + off, color);
        }
    }
}

/// Read a surface's pixel buffer and convert BGRA8888 → RGBA8888 for the canvas.
fn read_surface_rgba(ctx: &ApiContext, sid: u32) -> Vec<u8> {
    let (pixels_va, w, h, stride) = match ctx.gui.ddraw_surfaces.get(&sid) {
        Some(s) => (s.pixels_va, s.width as i32, s.height as i32, s.stride as i32),
        None => return Vec::new(),
    };

    let total = (w * h * 4) as usize;
    let mut out = vec![0u8; total];

    for row in 0..h {
        for col in 0..w {
            let src_off = (row * stride + col * 4) as u32;
            let bgra = ctx.memory.read_u32(pixels_va + src_off).unwrap_or(0);
            let b = (bgra & 0xFF) as u8;
            let g = ((bgra >> 8) & 0xFF) as u8;
            let r = ((bgra >> 16) & 0xFF) as u8;
            let a = ((bgra >> 24) & 0xFF) as u8;
            let dst = ((row * w + col) * 4) as usize;
            out[dst] = r;
            out[dst + 1] = g;
            out[dst + 2] = b;
            out[dst + 3] = if a == 0 { 0xFF } else { a };
        }
    }
    out
}
