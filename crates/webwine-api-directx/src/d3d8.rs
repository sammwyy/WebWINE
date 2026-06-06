//! Direct3D 8 stub (d3d8.dll).

use webwine_api::winapi::context::{ApiContext, Handled};
use webwine_api::winapi::WinApiRegistry;

use crate::{
    com_qi, make_object, register_vtable, s0_1, s0_2, s0_3, s0_4, s0_5, s0_6, s0_7, s0_9, s1_1,
    s1_2, Vtable,
};

const S_OK: u32 = 0x0000_0000;

// IDirect3DDevice8 object layout (guest heap). Beyond the COM vtable ptr the
// device carries its window + the render state the draw calls need, so the D3D8
// state tracker stays self-contained (no host-side per-device map).
const DEV_HWND: u32 = 4;
const DEV_BB: u32 = 8; // cached backbuffer surface VA
const DEV_DEPTH: u32 = 12; // cached depth/RT surface VA
const DEV_FVF: u32 = 16; // current FVF (vertex format) from SetVertexShader
const DEV_STREAM_VB: u32 = 20; // bound vertex buffer VA (SetStreamSource)
const DEV_STREAM_STRIDE: u32 = 24; // bound vertex stride
const DEV_TEXTURE: u32 = 28; // current texture VA (stage 0) — used as the GPU texture id
const DEV_BLEND_EN: u32 = 32; // D3DRS_ALPHABLENDENABLE
const DEV_DESTBLEND: u32 = 36; // D3DRS_DESTBLEND
const DEV_SIZE: u32 = 40;

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
    ("IDirect3DDevice8::GetDeviceCaps", dev_get_device_caps),
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
    (
        "IDirect3DDevice8::CreateVertexBuffer",
        dev_create_vertex_buffer,
    ),
    ("IDirect3DDevice8::CreateIndexBuffer", s0_6),
    ("IDirect3DDevice8::CreateRenderTarget", s0_4),
    ("IDirect3DDevice8::CreateDepthStencilSurface", s0_4),
    (
        "IDirect3DDevice8::CreateImageSurface",
        dev_create_image_surface,
    ),
    ("IDirect3DDevice8::CopyRects", s0_4),
    ("IDirect3DDevice8::UpdateTexture", s0_4),
    ("IDirect3DDevice8::GetFrontBuffer", s0_4),
    ("IDirect3DDevice8::SetRenderTarget", s0_4),
    ("IDirect3DDevice8::GetRenderTarget", dev_get_surface_2),
    (
        "IDirect3DDevice8::GetDepthStencilSurface",
        dev_get_surface_2,
    ),
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
    ("IDirect3DDevice8::SetRenderState", dev_set_render_state),
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
    ("IDirect3DDevice8::SetTexture", dev_set_texture),
    ("IDirect3DDevice8::GetTextureStageState", s0_4),
    ("IDirect3DDevice8::SetTextureStageState", s0_4),
    ("IDirect3DDevice8::ValidateDevice", s0_4),
    ("IDirect3DDevice8::GetInfo", s0_4),
    ("IDirect3DDevice8::SetPaletteEntries", s0_4),
    ("IDirect3DDevice8::GetPaletteEntries", s0_4),
    ("IDirect3DDevice8::SetCurrentTexturePalette", s0_4),
    ("IDirect3DDevice8::GetCurrentTexturePalette", s0_4),
    ("IDirect3DDevice8::DrawPrimitive", dev_draw_primitive),
    ("IDirect3DDevice8::DrawIndexedPrimitive", s0_6),
    ("IDirect3DDevice8::DrawPrimitiveUP", dev_draw_primitive_up),
    ("IDirect3DDevice8::DrawIndexedPrimitiveUP", s0_9),
    ("IDirect3DDevice8::ProcessVertices", s0_4),
    ("IDirect3DDevice8::CreateVertexShader", s0_4),
    ("IDirect3DDevice8::SetVertexShader", dev_set_vertex_shader),
    ("IDirect3DDevice8::GetVertexShader", s0_4),
    ("IDirect3DDevice8::DeleteVertexShader", s0_4),
    ("IDirect3DDevice8::SetVertexShaderConstant", s0_4),
    ("IDirect3DDevice8::GetVertexShaderConstant", s0_4),
    ("IDirect3DDevice8::GetVertexShaderDeclaration", s0_4),
    ("IDirect3DDevice8::GetVertexShaderFunction", s0_4),
    ("IDirect3DDevice8::SetStreamSource", dev_set_stream_source),
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

/// Fill a 212-byte D3DCAPS8 advertising a full hardware device (HW transform &
/// lighting + HW rasterization), so games take the hardware DrawPrimitive path
/// instead of a CPU/software-rasterizer fallback. The struct is fully written
/// (zeroed first), with cap FLAG fields enabled and MAX/limit fields set to sane
/// values — leaving any field garbage can make a game compute a huge buffer.
fn fill_d3dcaps8(ctx: &mut ApiContext, out: u32) {
    if out == 0 {
        return;
    }
    // Zero the whole struct first.
    let _ = ctx.memory.write_bytes(out, &[0u8; 212]);
    let mut w = |off: u32, v: u32| {
        let _ = ctx.memory.write_u32(out + off, v);
    };
    w(0, 1); // DeviceType = D3DDEVTYPE_HAL
    w(8, 0x0000_0001); // Caps = D3DCAPS_READ_SCANLINE
    w(12, 0x0000_0002); // Caps2 = D3DCAPS2_FULLSCREENGAMMA
    w(20, 0x8000_0000); // PresentationIntervals = IMMEDIATE
    // DevCaps: HWTRANSFORMANDLIGHT(0x10000) + HWRASTERIZATION(0x80000) +
    // DRAWPRIMTLVERTEX/DRAWPRIMITIVES2(EX) + memory caps — enable all.
    w(28, 0x00FF_FFFF); // DevCaps
    w(32, 0x00FF_FFFF); // PrimitiveMiscCaps
    w(36, 0x00FF_FFFF); // RasterCaps
    w(40, 0x0000_00FF); // ZCmpCaps
    w(44, 0x0000_1FFF); // SrcBlendCaps (all blend modes)
    w(48, 0x0000_1FFF); // DestBlendCaps
    w(52, 0x0000_00FF); // AlphaCmpCaps
    w(56, 0x00FF_FFFF); // ShadeCaps
    w(60, 0x00FF_FFFF); // TextureCaps
    w(64, 0x0FFF_FFFF); // TextureFilterCaps
    w(76, 0x0000_003F); // TextureAddressCaps
    w(84, 0x0000_001F); // LineCaps
    w(88, 4096); // MaxTextureWidth
    w(92, 4096); // MaxTextureHeight
    w(96, 256); // MaxVolumeExtent
    w(100, 8192); // MaxTextureRepeat
    w(104, 4096); // MaxTextureAspectRatio
    w(108, 16); // MaxAnisotropy
    w(112, 0x5000_0000); // MaxVertexW (~large float)
    w(116, 0xC47A_0000); // GuardBandLeft  = -1000.0
    w(120, 0xC47A_0000); // GuardBandTop   = -1000.0
    w(124, 0x447A_0000); // GuardBandRight =  1000.0
    w(128, 0x447A_0000); // GuardBandBottom=  1000.0
    w(136, 0x0000_00FF); // StencilCaps
    w(140, 0x0010_0008); // FVFCaps (8 texcoord sets + PSIZE)
    w(144, 0x00FF_FFFF); // TextureOpCaps
    w(148, 8); // MaxTextureBlendStages
    w(152, 8); // MaxSimultaneousTextures
    w(156, 0x0000_003F); // VertexProcessingCaps
    w(160, 8); // MaxActiveLights
    w(164, 6); // MaxUserClipPlanes
    w(168, 4); // MaxVertexBlendMatrices
    w(176, 0x4380_0000); // MaxPointSize = 256.0
    w(180, 0x000F_FFFF); // MaxPrimitiveCount
    w(184, 0x000F_FFFF); // MaxVertexIndex
    w(188, 1); // MaxStreams
    w(192, 256); // MaxStreamStride
    // VertexShaderVersion / PixelShaderVersion left 0 → fixed-function only.
}

fn d3d8_get_device_caps(ctx: &mut ApiContext) -> Handled {
    // IDirect3D8::GetDeviceCaps(this, Adapter, DeviceType, pCaps)
    let out = ctx.arg(3);
    fill_d3dcaps8(ctx, out);
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn dev_get_device_caps(ctx: &mut ApiContext) -> Handled {
    // IDirect3DDevice8::GetDeviceCaps(this, pCaps)
    let out = ctx.arg(1);
    fill_d3dcaps8(ctx, out);
    ctx.ret_stdcall(S_OK, 2);
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
        let dev_va = ctx.heap_alloc(DEV_SIZE);
        let _ = ctx.memory.write_u32(dev_va, vtable_va);
        let _ = ctx.memory.write_u32(dev_va + DEV_HWND, hwnd);
        for off in [DEV_BB, DEV_DEPTH, DEV_FVF, DEV_STREAM_VB, DEV_STREAM_STRIDE, DEV_TEXTURE, DEV_BLEND_EN, DEV_DESTBLEND] {
            let _ = ctx.memory.write_u32(dev_va + off, 0);
        }
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

// D3D8 resource objects (texture / surface / vertex buffer)
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

fn res_w(ctx: &ApiContext, obj: u32) -> u32 {
    ctx.memory.read_u32(obj + 8).unwrap_or(0)
}
fn res_h(ctx: &ApiContext, obj: u32) -> u32 {
    ctx.memory.read_u32(obj + 12).unwrap_or(0)
}
fn res_scratch(ctx: &ApiContext, obj: u32) -> u32 {
    ctx.memory.read_u32(obj + 16).unwrap_or(0)
}

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
    ("IDirect3DTexture8::UnlockRect", tex_unlock_rect),
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
        let _ = ctx.memory.write_u32(desc, 21); // D3DFMT_A8R8G8B8
        let _ = ctx.memory.write_u32(desc + 4, 1); // D3DRTYPE_SURFACE
        let _ = ctx.memory.write_u32(desc + 12, 1); // D3DPOOL_MANAGED
        let _ = ctx.memory.write_u32(desc + 16, w * h * 4); // Size
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
        let _ = ctx
            .memory
            .write_u32(ppb, res_scratch(ctx, this).wrapping_add(offset));
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

// device resource creators

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

// frame ops (emit to the GPU command stream)

fn dev_clear(ctx: &mut ApiContext) -> Handled {
    // Clear(this, Count, pRects, Flags, Color, Z, Stencil) — Color is D3DCOLOR ARGB.
    let hwnd = ctx.memory.read_u32(ctx.arg(0) + 4).unwrap_or(0);
    let color = ctx.arg(4);
    ctx.ui_events
        .push(webwine_api::vm::process::UiEvent::GpuClear { hwnd, color });
    ctx.ret_stdcall(S_OK, 7);
    Handled::Ok
}

fn dev_present(ctx: &mut ApiContext) -> Handled {
    // Present(this, pSourceRect, pDestRect, hDestWindowOverride, pDirtyRegion)
    let hwnd = ctx.memory.read_u32(ctx.arg(0) + 4).unwrap_or(0);
    ctx.ui_events
        .push(webwine_api::vm::process::UiEvent::GpuPresent { hwnd });
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}

// ── D3D8 render-state setters + draw emission (generic, no game-specific code) ─

fn dev_set_vertex_shader(ctx: &mut ApiContext) -> Handled {
    // SetVertexShader(this, Handle). For the fixed-function pipeline `Handle` is
    // an FVF code (vertex layout); programmable shader handles are a separate
    // range we do not model — storing it as the FVF is harmless for those.
    let dev = ctx.arg(0);
    let fvf = ctx.arg(1);
    let _ = ctx.memory.write_u32(dev + DEV_FVF, fvf);
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dev_set_stream_source(ctx: &mut ApiContext) -> Handled {
    // SetStreamSource(this, StreamNumber, pStreamData, Stride)
    let dev = ctx.arg(0);
    let _ = ctx.memory.write_u32(dev + DEV_STREAM_VB, ctx.arg(2));
    let _ = ctx.memory.write_u32(dev + DEV_STREAM_STRIDE, ctx.arg(3));
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn dev_set_texture(ctx: &mut ApiContext) -> Handled {
    // SetTexture(this, Stage, pTexture). We render stage 0 only.
    let dev = ctx.arg(0);
    if ctx.arg(1) == 0 {
        let _ = ctx.memory.write_u32(dev + DEV_TEXTURE, ctx.arg(2));
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn dev_set_render_state(ctx: &mut ApiContext) -> Handled {
    // SetRenderState(this, State, Value) — track just alpha-blend enable + dest
    // blend, enough to pick a blend mode for sprites.
    const D3DRS_ALPHABLENDENABLE: u32 = 27;
    const D3DRS_DESTBLEND: u32 = 20;
    let dev = ctx.arg(0);
    match ctx.arg(1) {
        D3DRS_ALPHABLENDENABLE => {
            let _ = ctx.memory.write_u32(dev + DEV_BLEND_EN, ctx.arg(2));
        }
        D3DRS_DESTBLEND => {
            let _ = ctx.memory.write_u32(dev + DEV_DESTBLEND, ctx.arg(2));
        }
        _ => {}
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn dev_blend_mode(ctx: &ApiContext, dev: u32) -> u32 {
    // 0 none, 1 alpha, 2 additive. Additive = D3DBLEND_ONE(2) dest blend.
    if ctx.memory.read_u32(dev + DEV_BLEND_EN).unwrap_or(0) == 0 {
        return 0;
    }
    if ctx.memory.read_u32(dev + DEV_DESTBLEND).unwrap_or(0) == 2 { 2 } else { 1 }
}

/// Byte offsets of diffuse color and the first texcoord within a vertex, derived
/// from the FVF. x,y are always at +0/+4 (screen-space for XYZRHW sprites).
struct FvfLayout {
    color_off: Option<u32>,
    uv_off: Option<u32>,
}

fn fvf_layout(fvf: u32) -> FvfLayout {
    let pos_size = match fvf & 0x00E {
        0x004 => 16, // D3DFVF_XYZRHW (x,y,z,rhw)
        0x002 => 12, // D3DFVF_XYZ
        _ => 12,
    };
    let mut off = pos_size;
    if fvf & 0x010 != 0 {
        off += 12; // D3DFVF_NORMAL
    }
    if fvf & 0x020 != 0 {
        off += 4; // D3DFVF_PSIZE
    }
    let color_off = if fvf & 0x040 != 0 {
        let o = off;
        off += 4;
        Some(o) // D3DFVF_DIFFUSE
    } else {
        None
    };
    if fvf & 0x080 != 0 {
        off += 4; // D3DFVF_SPECULAR
    }
    let uv_off = if (fvf >> 8) & 0xF >= 1 { Some(off) } else { None }; // D3DFVF_TEX1+
    FvfLayout { color_off, uv_off }
}

/// One source vertex -> [x, y, u, v, r, g, b, a] (screen px, 0..1 uv, 0..1 rgba).
fn read_vertex(ctx: &ApiContext, base: u32, layout: &FvfLayout) -> [f32; 8] {
    let f = |o: u32| f32::from_bits(ctx.memory.read_u32(base + o).unwrap_or(0));
    let (u, v) = layout.uv_off.map(|o| (f(o), f(o + 4))).unwrap_or((0.0, 0.0));
    let (r, g, b, a) = layout
        .color_off
        .map(|o| {
            let c = ctx.memory.read_u32(base + o).unwrap_or(0xFFFF_FFFF);
            (
                ((c >> 16) & 0xFF) as f32 / 255.0,
                ((c >> 8) & 0xFF) as f32 / 255.0,
                (c & 0xFF) as f32 / 255.0,
                ((c >> 24) & 0xFF) as f32 / 255.0,
            )
        })
        .unwrap_or((1.0, 1.0, 1.0, 1.0));
    [f(0), f(4), u, v, r, g, b, a]
}

/// Source-vertex count consumed by `prim_count` triangle primitives of a type.
fn vertex_count_for(prim_type: u32, prim_count: u32) -> u32 {
    match prim_type {
        4 => prim_count * 3,     // TRIANGLELIST
        5 | 6 => prim_count + 2, // TRIANGLESTRIP / TRIANGLEFAN
        _ => 0,
    }
}

/// Expand a triangle primitive into a flat triangle-vertex index list.
fn triangle_indices(prim_type: u32, prim_count: u32) -> Vec<usize> {
    let mut idx = Vec::new();
    let n = prim_count as usize;
    match prim_type {
        4 => {
            for i in 0..n {
                let b = i * 3;
                idx.extend([b, b + 1, b + 2]);
            }
        }
        5 => {
            for i in 0..n {
                if i % 2 == 0 {
                    idx.extend([i, i + 1, i + 2]);
                } else {
                    idx.extend([i + 1, i, i + 2]);
                }
            }
        }
        6 => {
            for i in 0..n {
                idx.extend([0, i + 1, i + 2]);
            }
        }
        _ => {}
    }
    idx
}

/// Read `prim_count` triangle primitives starting at `base` (stride bytes/vertex)
/// and emit them as a GpuDrawTris command (expanded to a flat TRIANGLES list).
fn emit_draw(ctx: &mut ApiContext, dev: u32, prim_type: u32, base: u32, stride: u32, prim_count: u32) {
    if !(4..=6).contains(&prim_type) || prim_count == 0 || stride == 0 {
        return; // only triangle topologies are drawn
    }
    let fvf = ctx.memory.read_u32(dev + DEV_FVF).unwrap_or(0);
    let layout = fvf_layout(fvf);
    let vcount = vertex_count_for(prim_type, prim_count);
    let src: Vec<[f32; 8]> = (0..vcount).map(|i| read_vertex(ctx, base + i * stride, &layout)).collect();
    let mut verts: Vec<f32> = Vec::new();
    for i in triangle_indices(prim_type, prim_count) {
        if let Some(vrt) = src.get(i) {
            verts.extend_from_slice(vrt);
        }
    }
    if verts.is_empty() {
        return;
    }
    let hwnd = ctx.memory.read_u32(dev + DEV_HWND).unwrap_or(0);
    let texture = ctx.memory.read_u32(dev + DEV_TEXTURE).unwrap_or(0);
    let blend = dev_blend_mode(ctx, dev);
    ctx.ui_events.push(webwine_api::vm::process::UiEvent::GpuDrawTris { hwnd, texture, blend, verts });
}

fn dev_draw_primitive(ctx: &mut ApiContext) -> Handled {
    // DrawPrimitive(this, PrimitiveType, StartVertex, PrimitiveCount) — verts from
    // the bound stream-source vertex buffer.
    let dev = ctx.arg(0);
    let prim_type = ctx.arg(1);
    let start = ctx.arg(2);
    let prim_count = ctx.arg(3);
    let vb = ctx.memory.read_u32(dev + DEV_STREAM_VB).unwrap_or(0);
    let stride = ctx.memory.read_u32(dev + DEV_STREAM_STRIDE).unwrap_or(0);
    if vb != 0 {
        let scratch = ctx.memory.read_u32(vb + 16).unwrap_or(0); // VB data buffer
        if scratch != 0 {
            emit_draw(ctx, dev, prim_type, scratch + start * stride, stride, prim_count);
        }
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn dev_draw_primitive_up(ctx: &mut ApiContext) -> Handled {
    // DrawPrimitiveUP(this, PrimitiveType, PrimitiveCount, pVertexStreamZeroData,
    //                 VertexStreamZeroStride) — verts supplied inline.
    let dev = ctx.arg(0);
    let prim_type = ctx.arg(1);
    let prim_count = ctx.arg(2);
    let data = ctx.arg(3);
    let stride = ctx.arg(4);
    if data != 0 {
        emit_draw(ctx, dev, prim_type, data, stride, prim_count);
    }
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}

fn tex_unlock_rect(ctx: &mut ApiContext) -> Handled {
    // UnlockRect(this, Level) — upload what the guest wrote into our scratch as a
    // GPU texture. D3DFMT_A8R8G8B8 is BGRA in memory; WebGL wants RGBA.
    let tex = ctx.arg(0);
    let (w, h, scratch) = (res_w(ctx, tex), res_h(ctx, tex), res_scratch(ctx, tex));
    let hwnd = ctx.memory.read_u32(tex + 4).unwrap_or(0);
    if w > 0 && h > 0 && scratch != 0 {
        let bgra = ctx.memory.read_bytes(scratch, (w * h * 4) as usize).unwrap_or_default();
        let mut rgba = vec![0u8; bgra.len()];
        for o in (0..bgra.len()).step_by(4) {
            rgba[o] = bgra[o + 2];
            rgba[o + 1] = bgra[o + 1];
            rgba[o + 2] = bgra[o];
            rgba[o + 3] = bgra[o + 3];
        }
        ctx.ui_events.push(webwine_api::vm::process::UiEvent::GpuTexture { hwnd, id: tex, w, h, pixels: rgba });
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}
