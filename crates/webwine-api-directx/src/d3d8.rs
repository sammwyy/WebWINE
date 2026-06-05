//! Direct3D 8 stub (d3d8.dll).

use webwine_api::winapi::context::{ApiContext, Handled};
use webwine_api::winapi::WinApiRegistry;

use crate::{
    com_qi, make_object, register_vtable, s0_1, s0_2, s0_3, s0_4, s0_5, s0_6, s0_7, s0_9, s1_1,
    s1_2, Vtable,
};

const S_OK: u32 = 0x0000_0000;

pub(crate) const IDIRECT3D8: Vtable = &[
    ("IDirect3D8::QueryInterface", com_qi),
    ("IDirect3D8::AddRef", s1_1),
    ("IDirect3D8::Release", s1_1),
    ("IDirect3D8::RegisterSoftwareDevice", s0_2),
    ("IDirect3D8::GetAdapterCount", s1_1), // returns 1
    (
        "IDirect3D8::GetAdapterIdentifier",
        d3d8_get_adapter_identifier,
    ),
    ("IDirect3D8::GetAdapterModeCount", s1_2), // returns 1
    ("IDirect3D8::EnumAdapterModes", s0_4),
    (
        "IDirect3D8::GetAdapterDisplayMode",
        d3d8_get_adapter_display_mode,
    ),
    ("IDirect3D8::CheckDeviceType", s0_6),
    ("IDirect3D8::CheckDeviceFormat", s0_7), // this + 6 args
    ("IDirect3D8::CheckDeviceMultiSampleType", s0_6),
    ("IDirect3D8::CheckDepthStencilMatch", s0_6),
    ("IDirect3D8::GetDeviceCaps", d3d8_get_device_caps),
    ("IDirect3D8::GetAdapterMonitor", s0_2),
    ("IDirect3D8::CreateDevice", d3d8_create_device),
];

// IDirect3DDevice8: 94 slots, all stubs except IUnknown. Per-slot arg counts
// preserved from the original device dispatcher (most default to 4).
pub(crate) const IDIRECT3DDEVICE8: Vtable = &[
    ("IDirect3DDevice8::QueryInterface", com_qi),
    ("IDirect3DDevice8::AddRef", s1_1),
    ("IDirect3DDevice8::Release", s1_1),
    ("IDirect3DDevice8::TestCooperativeLevel", s0_4),
    ("IDirect3DDevice8::GetAvailableTextureMem", s0_4),
    ("IDirect3DDevice8::ResourceManagerDiscardBytes", s0_4),
    ("IDirect3DDevice8::GetDirect3D", d3d8_device_get_direct3d),
    ("IDirect3DDevice8::GetDeviceCaps", s0_4),
    ("IDirect3DDevice8::GetDisplayMode", s0_4),
    ("IDirect3DDevice8::GetCreationParameters", s0_4),
    ("IDirect3DDevice8::SetCursorProperties", s0_4),
    ("IDirect3DDevice8::SetCursorPosition", s0_4),
    ("IDirect3DDevice8::ShowCursor", s0_4),
    ("IDirect3DDevice8::CreateAdditionalSwapChain", s0_4),
    ("IDirect3DDevice8::Reset", s0_2),
    ("IDirect3DDevice8::Present", dev_present),
    ("IDirect3DDevice8::GetBackBuffer", dev_get_backbuffer),
    ("IDirect3DDevice8::GetRasterStatus", s0_4),
    ("IDirect3DDevice8::SetGammaRamp", s0_4),
    ("IDirect3DDevice8::GetGammaRamp", s0_4),
    ("IDirect3DDevice8::CreateTexture", dev_create_texture),
    ("IDirect3DDevice8::CreateVolumeTexture", s0_4),
    ("IDirect3DDevice8::CreateCubeTexture", s0_4),
    ("IDirect3DDevice8::CreateVertexBuffer", dev_create_vertex_buffer),
    ("IDirect3DDevice8::CreateIndexBuffer", s0_6),
    ("IDirect3DDevice8::CreateRenderTarget", s0_4),
    ("IDirect3DDevice8::CreateDepthStencilSurface", s0_4),
    ("IDirect3DDevice8::CreateImageSurface", dev_create_image_surface),
    ("IDirect3DDevice8::CopyRects", s0_4),
    ("IDirect3DDevice8::UpdateTexture", s0_4),
    ("IDirect3DDevice8::GetFrontBuffer", s0_4),
    ("IDirect3DDevice8::SetRenderTarget", s0_4),
    ("IDirect3DDevice8::GetRenderTarget", dev_get_surface_2),
    ("IDirect3DDevice8::GetDepthStencilSurface", dev_get_surface_2),
    ("IDirect3DDevice8::BeginScene", s0_1),
    ("IDirect3DDevice8::EndScene", s0_1),
    ("IDirect3DDevice8::Clear", dev_clear),
    ("IDirect3DDevice8::SetTransform", s0_3),
    ("IDirect3DDevice8::GetTransform", s0_4),
    ("IDirect3DDevice8::MultiplyTransform", s0_4),
    ("IDirect3DDevice8::SetViewport", s0_2),
    ("IDirect3DDevice8::GetViewport", s0_4),
    ("IDirect3DDevice8::SetMaterial", s0_4),
    ("IDirect3DDevice8::GetMaterial", s0_4),
    ("IDirect3DDevice8::SetLight", s0_4),
    ("IDirect3DDevice8::GetLight", s0_4),
    ("IDirect3DDevice8::LightEnable", s0_4),
    ("IDirect3DDevice8::GetLightEnable", s0_4),
    ("IDirect3DDevice8::SetClipPlane", s0_4),
    ("IDirect3DDevice8::GetClipPlane", s0_4),
    ("IDirect3DDevice8::SetRenderState", s0_3),
    ("IDirect3DDevice8::GetRenderState", s0_4),
    ("IDirect3DDevice8::BeginStateBlock", s0_4),
    ("IDirect3DDevice8::EndStateBlock", s0_4),
    ("IDirect3DDevice8::ApplyStateBlock", s0_4),
    ("IDirect3DDevice8::CaptureStateBlock", s0_4),
    ("IDirect3DDevice8::DeleteStateBlock", s0_4),
    ("IDirect3DDevice8::CreateStateBlock", s0_4),
    ("IDirect3DDevice8::SetClipStatus", s0_4),
    ("IDirect3DDevice8::GetClipStatus", s0_4),
    ("IDirect3DDevice8::GetTexture", s0_4),
    ("IDirect3DDevice8::SetTexture", s0_3),
    ("IDirect3DDevice8::GetTextureStageState", s0_4),
    ("IDirect3DDevice8::SetTextureStageState", s0_4),
    ("IDirect3DDevice8::ValidateDevice", s0_4),
    ("IDirect3DDevice8::GetInfo", s0_4),
    ("IDirect3DDevice8::SetPaletteEntries", s0_4),
    ("IDirect3DDevice8::GetPaletteEntries", s0_4),
    ("IDirect3DDevice8::SetCurrentTexturePalette", s0_4),
    ("IDirect3DDevice8::GetCurrentTexturePalette", s0_4),
    ("IDirect3DDevice8::DrawPrimitive", s0_4),
    ("IDirect3DDevice8::DrawIndexedPrimitive", s0_6),
    ("IDirect3DDevice8::DrawPrimitiveUP", s0_5),
    ("IDirect3DDevice8::DrawIndexedPrimitiveUP", s0_9),
    ("IDirect3DDevice8::ProcessVertices", s0_4),
    ("IDirect3DDevice8::CreateVertexShader", s0_4),
    ("IDirect3DDevice8::SetVertexShader", s0_2),
    ("IDirect3DDevice8::GetVertexShader", s0_4),
    ("IDirect3DDevice8::DeleteVertexShader", s0_4),
    ("IDirect3DDevice8::SetVertexShaderConstant", s0_4),
    ("IDirect3DDevice8::GetVertexShaderConstant", s0_4),
    ("IDirect3DDevice8::GetVertexShaderDeclaration", s0_4),
    ("IDirect3DDevice8::GetVertexShaderFunction", s0_4),
    ("IDirect3DDevice8::SetStreamSource", s0_4),
    ("IDirect3DDevice8::GetStreamSource", s0_4),
    ("IDirect3DDevice8::SetIndices", s0_3),
    ("IDirect3DDevice8::GetIndices", s0_4),
    ("IDirect3DDevice8::CreatePixelShader", s0_4),
    ("IDirect3DDevice8::SetPixelShader", s0_4),
    ("IDirect3DDevice8::GetPixelShader", s0_4),
    ("IDirect3DDevice8::DeletePixelShader", s0_4),
    ("IDirect3DDevice8::SetPixelShaderConstant", s0_4),
    ("IDirect3DDevice8::GetPixelShaderConstant", s0_4),
    ("IDirect3DDevice8::GetPixelShaderFunction", s0_4),
];

pub fn register(r: &mut WinApiRegistry) {
    r.add("d3d8.dll", "Direct3DCreate8", d3d8_create_stub);
    register_vtable(r, IDIRECT3D8);
    register_vtable(r, IDIRECT3DDEVICE8);
    register_vtable(r, IDIRECT3DSURFACE8);
    register_vtable(r, IDIRECT3DVERTEXBUFFER8);
    register_vtable(r, IDIRECT3DTEXTURE8);
}

/// `Direct3DCreate8(SDKVersion) -> IDirect3D8*` (returned directly in EAX).
fn d3d8_create_stub(ctx: &mut ApiContext) -> Handled {
    let obj_va = make_object(ctx, IDIRECT3D8, 0);
    ctx.ret_stdcall(obj_va, 1);
    Handled::Ok
}

fn d3d8_get_adapter_identifier(ctx: &mut ApiContext) -> Handled {
    // GetAdapterIdentifier(this, adapter, flags, pIdentifier)
    let out = ctx.arg(3);
    if out != 0 {
        let name = b"WebWINE D3D8 Stub\0";
        let _ = ctx.memory.write_bytes(out, name);
        let _ = ctx.memory.write_bytes(out + 512, name);
        let _ = ctx.memory.write_u32(out + 1024, 0x1234); // vendor id
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn d3d8_get_adapter_display_mode(ctx: &mut ApiContext) -> Handled {
    // GetAdapterDisplayMode(this, adapter, pMode)
    let out = ctx.arg(2);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 640);
        let _ = ctx.memory.write_u32(out + 4, 480);
        let _ = ctx.memory.write_u32(out + 8, 0); // RefreshRate
        let _ = ctx.memory.write_u32(out + 12, 22); // D3DFMT_X8R8G8B8
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn d3d8_get_device_caps(ctx: &mut ApiContext) -> Handled {
    // GetDeviceCaps(this, adapter, devtype, pCaps)
    let out = ctx.arg(3);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 1); // DeviceType
        let _ = ctx.memory.write_u32(out + 12, 0xFFFF_FFFF); // Caps
        let _ = ctx.memory.write_u32(out + 16, 0xFFFF_FFFF); // Caps2
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn d3d8_device_get_direct3d(ctx: &mut ApiContext) -> Handled {
    // IDirect3DDevice8::GetDirect3D(this, IDirect3D8** ppD3D8) — hand back the
    // parent IDirect3D8 so the guest can run its format/caps checks through it.
    let out = ctx.arg(1);
    if out != 0 {
        let obj = make_object(ctx, IDIRECT3D8, 0);
        let _ = ctx.memory.write_u32(out, obj);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn d3d8_create_device(ctx: &mut ApiContext) -> Handled {
    // CreateDevice(this, Adapter, DeviceType, hFocusWindow, BehaviorFlags,
    //              pPresentationParameters, ppReturnedDeviceInterface)
    let hfocus = ctx.arg(3);
    let pparams = ctx.arg(5);
    // Prefer the presentation params' hDeviceWindow (+24); fall back to hFocusWindow.
    let hwnd = ctx
        .memory
        .read_u32(pparams + 24)
        .ok()
        .filter(|&w| w != 0)
        .unwrap_or(hfocus);
    let out_ptr = ctx.arg(6);
    if out_ptr != 0 {
        // Wider device object: [vtable, hwnd, backbuffer-VA, depth/RT-VA]. The
        // window is at +4 so Clear/Present know where to draw; the two surface
        // caches (+8/+12) keep GetBackBuffer/GetDepthStencilSurface from
        // allocating a fresh ~1 MB surface on every frame (which would grow the
        // heap without bound and eventually corrupt memory).
        let vtable_va = ctx.heap_alloc((IDIRECT3DDEVICE8.len() * 4) as u32);
        for (i, (name, _)) in IDIRECT3DDEVICE8.iter().enumerate() {
            let tramp = ctx.api_resolve_trampoline(crate::VTBL, name);
            let _ = ctx.memory.write_u32(vtable_va + i as u32 * 4, tramp);
        }
        let dev_va = ctx.heap_alloc(16);
        let _ = ctx.memory.write_u32(dev_va, vtable_va);
        let _ = ctx.memory.write_u32(dev_va + 4, hwnd);
        let _ = ctx.memory.write_u32(dev_va + 8, 0); // backbuffer cache
        let _ = ctx.memory.write_u32(dev_va + 12, 0); // depth/RT cache
        let _ = ctx.memory.write_u32(out_ptr, dev_va);
    }
    ctx.ret_stdcall(S_OK, 7);
    Handled::Ok
}

/// Return the surface cached at `dev+cache_off`, creating a 640x480 one bound to
/// the device window on first use. Avoids per-frame allocation churn.
fn cached_device_surface(ctx: &mut ApiContext, dev: u32, cache_off: u32) -> u32 {
    let cached = ctx.memory.read_u32(dev + cache_off).unwrap_or(0);
    if cached != 0 {
        return cached;
    }
    let hwnd = ctx.memory.read_u32(dev + 4).unwrap_or(0);
    let surf = make_resource(ctx, IDIRECT3DSURFACE8, hwnd, 640, 480, 640 * 480 * 4);
    let _ = ctx.memory.write_u32(dev + cache_off, surf);
    surf
}

// ── D3D8 resource objects (texture / surface / vertex buffer) ────────────────
//
// th06 (and most fixed-function D3D8 games) create these, Lock them to upload
// vertex/pixel data, and draw. We back each with a guest-heap scratch buffer so
// Lock hands the game a writable pointer (no real GPU resource yet — the host
// VideoDriver gets the data via the GpuTexture/GpuDrawTris command stream).
//
// Resource object layout on the guest heap (wider than the 8-byte COM default):
//   +0  vtable ptr
//   +4  hwnd (owning device window)
//   +8  width  (or buffer length in bytes for a vertex buffer)
//   +12 height (1 for a vertex buffer)
//   +16 scratch buffer VA (width*height*4, or length bytes)

fn make_resource(ctx: &mut ApiContext, vt: Vtable, hwnd: u32, w: u32, h: u32, bytes: u32) -> u32 {
    let vtable_va = ctx.heap_alloc((vt.len() * 4) as u32);
    for (i, (name, _)) in vt.iter().enumerate() {
        let tramp = ctx.api_resolve_trampoline(crate::VTBL, name);
        let _ = ctx.memory.write_u32(vtable_va + i as u32 * 4, tramp);
    }
    let scratch = ctx.heap_alloc(bytes.max(4));
    let obj = ctx.heap_alloc(20);
    let _ = ctx.memory.write_u32(obj, vtable_va);
    let _ = ctx.memory.write_u32(obj + 4, hwnd);
    let _ = ctx.memory.write_u32(obj + 8, w);
    let _ = ctx.memory.write_u32(obj + 12, h);
    let _ = ctx.memory.write_u32(obj + 16, scratch);
    obj
}

fn res_w(ctx: &ApiContext, obj: u32) -> u32 { ctx.memory.read_u32(obj + 8).unwrap_or(0) }
fn res_h(ctx: &ApiContext, obj: u32) -> u32 { ctx.memory.read_u32(obj + 12).unwrap_or(0) }
fn res_scratch(ctx: &ApiContext, obj: u32) -> u32 { ctx.memory.read_u32(obj + 16).unwrap_or(0) }

// IDirect3DSurface8 (IUnknown + 8): GetDevice, {Set,Get,Free}PrivateData,
// GetContainer, GetDesc, LockRect, UnlockRect.
pub(crate) const IDIRECT3DSURFACE8: Vtable = &[
    ("IDirect3DSurface8::QueryInterface", com_qi),
    ("IDirect3DSurface8::AddRef", s1_1),
    ("IDirect3DSurface8::Release", s1_1),
    ("IDirect3DSurface8::GetDevice", s0_2),
    ("IDirect3DSurface8::SetPrivateData", s0_5),
    ("IDirect3DSurface8::GetPrivateData", s0_4),
    ("IDirect3DSurface8::FreePrivateData", s0_2),
    ("IDirect3DSurface8::GetContainer", s0_3),
    ("IDirect3DSurface8::GetDesc", surf_get_desc),
    ("IDirect3DSurface8::LockRect", surf_lock_rect),
    ("IDirect3DSurface8::UnlockRect", s0_1),
];

// IDirect3DVertexBuffer8 (IUnknown + IDirect3DResource8 methods + Lock/Unlock/GetDesc).
pub(crate) const IDIRECT3DVERTEXBUFFER8: Vtable = &[
    ("IDirect3DVertexBuffer8::QueryInterface", com_qi),
    ("IDirect3DVertexBuffer8::AddRef", s1_1),
    ("IDirect3DVertexBuffer8::Release", s1_1),
    ("IDirect3DVertexBuffer8::GetDevice", s0_2),
    ("IDirect3DVertexBuffer8::SetPrivateData", s0_5),
    ("IDirect3DVertexBuffer8::GetPrivateData", s0_4),
    ("IDirect3DVertexBuffer8::FreePrivateData", s0_2),
    ("IDirect3DVertexBuffer8::SetPriority", s0_2),
    ("IDirect3DVertexBuffer8::GetPriority", s0_1),
    ("IDirect3DVertexBuffer8::PreLoad", s0_1),
    ("IDirect3DVertexBuffer8::GetType", s0_1),
    ("IDirect3DVertexBuffer8::Lock", vb_lock),
    ("IDirect3DVertexBuffer8::Unlock", s0_1),
    ("IDirect3DVertexBuffer8::GetDesc", s0_2),
];

// IDirect3DTexture8 (IUnknown + Resource8 + BaseTexture8 + texture methods).
pub(crate) const IDIRECT3DTEXTURE8: Vtable = &[
    ("IDirect3DTexture8::QueryInterface", com_qi),
    ("IDirect3DTexture8::AddRef", s1_1),
    ("IDirect3DTexture8::Release", s1_1),
    ("IDirect3DTexture8::GetDevice", s0_2),
    ("IDirect3DTexture8::SetPrivateData", s0_5),
    ("IDirect3DTexture8::GetPrivateData", s0_4),
    ("IDirect3DTexture8::FreePrivateData", s0_2),
    ("IDirect3DTexture8::SetPriority", s0_2),
    ("IDirect3DTexture8::GetPriority", s0_1),
    ("IDirect3DTexture8::PreLoad", s0_1),
    ("IDirect3DTexture8::GetType", s0_1),
    ("IDirect3DTexture8::SetLOD", s0_2),
    ("IDirect3DTexture8::GetLOD", s0_1),
    ("IDirect3DTexture8::GetLevelCount", s1_1), // 1 level
    ("IDirect3DTexture8::GetLevelDesc", s0_3),
    ("IDirect3DTexture8::GetSurfaceLevel", tex_get_surface_level),
    ("IDirect3DTexture8::LockRect", tex_lock_rect),
    ("IDirect3DTexture8::UnlockRect", s0_2),
    ("IDirect3DTexture8::AddDirtyRect", s0_2),
];

/// Fill a D3DSURFACE_DESC enough for the caller (Format, Type, w/h at the usual
/// offsets). Minimal: write width/height.
fn surf_get_desc(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let desc = ctx.arg(1);
    if desc != 0 {
        let (w, h) = (res_w(ctx, this), res_h(ctx, this));
        // D3DSURFACE_DESC: Format(+0) Type(+4) Usage(+8) Pool(+12) Size(+16)
        // MultiSampleType(+20) Width(+24) Height(+28)
        let _ = ctx.memory.write_u32(desc + 24, w);
        let _ = ctx.memory.write_u32(desc + 28, h);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

/// Write a D3DLOCKED_RECT { INT Pitch; void* pBits } pointing at our scratch.
fn write_locked_rect(ctx: &mut ApiContext, this: u32, out: u32) {
    if out != 0 {
        let pitch = res_w(ctx, this).max(1) * 4;
        let _ = ctx.memory.write_u32(out, pitch); // Pitch
        let _ = ctx.memory.write_u32(out + 4, res_scratch(ctx, this)); // pBits
    }
}

// Surface::LockRect(this, pLockedRect, pRect, Flags)
fn surf_lock_rect(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(1);
    write_locked_rect(ctx, this, out);
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

// Texture::LockRect(this, Level, pLockedRect, pRect, Flags)
fn tex_lock_rect(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(2);
    write_locked_rect(ctx, this, out);
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}

// VertexBuffer::Lock(this, OffsetToLock, SizeToLock, ppbData, Flags)
fn vb_lock(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let offset = ctx.arg(1);
    let ppb = ctx.arg(3);
    if ppb != 0 {
        let _ = ctx.memory.write_u32(ppb, res_scratch(ctx, this).wrapping_add(offset));
    }
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}

// Texture::GetSurfaceLevel(this, Level, ppSurfaceLevel) -> a surface aliasing the
// texture's storage.
fn tex_get_surface_level(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(2);
    let (hwnd, w, h) = (
        ctx.memory.read_u32(this + 4).unwrap_or(0),
        res_w(ctx, this),
        res_h(ctx, this),
    );
    if out != 0 {
        let surf = make_resource(ctx, IDIRECT3DSURFACE8, hwnd, w, h, w * h * 4);
        let _ = ctx.memory.write_u32(out, surf);
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

// ── device resource creators ─────────────────────────────────────────────────

fn dev_create_texture(ctx: &mut ApiContext) -> Handled {
    // CreateTexture(this, Width, Height, Levels, Usage, Format, Pool, ppTexture)
    let (w, h) = (ctx.arg(1), ctx.arg(2));
    let hwnd = ctx.memory.read_u32(ctx.arg(0) + 4).unwrap_or(0);
    let out = ctx.arg(7);
    if out != 0 {
        let tex = make_resource(ctx, IDIRECT3DTEXTURE8, hwnd, w, h, w * h * 4);
        let _ = ctx.memory.write_u32(out, tex);
    }
    ctx.ret_stdcall(S_OK, 8);
    Handled::Ok
}

fn dev_create_vertex_buffer(ctx: &mut ApiContext) -> Handled {
    // CreateVertexBuffer(this, Length, Usage, FVF, Pool, ppVertexBuffer)
    let len = ctx.arg(1);
    let hwnd = ctx.memory.read_u32(ctx.arg(0) + 4).unwrap_or(0);
    let out = ctx.arg(5);
    if out != 0 {
        let vb = make_resource(ctx, IDIRECT3DVERTEXBUFFER8, hwnd, len, 1, len);
        let _ = ctx.memory.write_u32(out, vb);
    }
    ctx.ret_stdcall(S_OK, 6);
    Handled::Ok
}

fn dev_create_image_surface(ctx: &mut ApiContext) -> Handled {
    // CreateImageSurface(this, Width, Height, Format, ppSurface)
    let (w, h) = (ctx.arg(1), ctx.arg(2));
    let hwnd = ctx.memory.read_u32(ctx.arg(0) + 4).unwrap_or(0);
    let out = ctx.arg(4);
    if out != 0 {
        let surf = make_resource(ctx, IDIRECT3DSURFACE8, hwnd, w, h, w * h * 4);
        let _ = ctx.memory.write_u32(out, surf);
    }
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}

/// GetBackBuffer(this, BackBuffer, Type, ppBackBuffer) and the GetXxxSurface
/// getters: hand back a 640x480 surface bound to the device window.
fn dev_get_backbuffer(ctx: &mut ApiContext) -> Handled {
    let dev = ctx.arg(0);
    let out = ctx.arg(3);
    if out != 0 {
        let surf = cached_device_surface(ctx, dev, 8);
        let _ = ctx.memory.write_u32(out, surf);
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn dev_get_surface_2(ctx: &mut ApiContext) -> Handled {
    // GetRenderTarget(this, ppRenderTarget) / GetDepthStencilSurface(this, ppZ)
    let dev = ctx.arg(0);
    let out = ctx.arg(1);
    if out != 0 {
        let surf = cached_device_surface(ctx, dev, 12);
        let _ = ctx.memory.write_u32(out, surf);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

// ── frame ops (emit to the GPU command stream) ───────────────────────────────

fn dev_clear(ctx: &mut ApiContext) -> Handled {
    // Clear(this, Count, pRects, Flags, Color, Z, Stencil) — Color is D3DCOLOR ARGB.
    let hwnd = ctx.memory.read_u32(ctx.arg(0) + 4).unwrap_or(0);
    let color = ctx.arg(4);
    ctx.ui_events.push(webwine_api::vm::process::UiEvent::GpuClear { hwnd, color });
    ctx.ret_stdcall(S_OK, 7);
    Handled::Ok
}

fn dev_present(ctx: &mut ApiContext) -> Handled {
    // Present(this, pSourceRect, pDestRect, hDestWindowOverride, pDirtyRegion)
    let hwnd = ctx.memory.read_u32(ctx.arg(0) + 4).unwrap_or(0);
    ctx.ui_events.push(webwine_api::vm::process::UiEvent::GpuPresent { hwnd });
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}
