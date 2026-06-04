//! DirectDraw / Direct3D stub for WebWINE.
//!
//! Implements enough of the DDraw COM interface for 2D games that use the
//! classic "blit-to-primary-surface" rendering loop (Touhou 6, Dune 2000,
//! Warcraft II, etc.).  The output path is identical to GDI: we emit a
//! `UiEvent::Blit` with an RGBA8888 pixel buffer that the frontend paints
//! onto the guest window's canvas.
//!
//! ## How DDraw COM works (abridged)
//!
//! Every COM object is laid out in guest memory as:
//!
//!   [object VA]  → [vtable ptr]  (4 bytes, points at vtable)
//!   [vtable VA]  → [slot 0 ptr]  (QueryInterface)
//!                  [slot 1 ptr]  (AddRef)
//!                  [slot 2 ptr]  (Release)
//!                  [slot 3 ptr]  (first real method)
//!                  ...
//!
//! The guest calls `object->Method(args)` as `CALL [vtable + N*4]`, which
//! pushes `this` as the first stdcall argument.  Each vtable slot is a
//! trampoline VA allocated by `WinApiRegistry::resolve_trampoline`, which the
//! executor intercepts and routes to a handler here.
//!
//! ## Guest memory layout (all allocated on the guest heap)
//!
//! ```text
//! IDirectDraw7 object  (4 bytes: vtable ptr)
//!   └─ IDirectDraw7 vtable  (N * 4 bytes: trampoline VAs)
//!
//! IDirectDrawSurface7 object  (8 bytes: vtable ptr + surface-id u32)
//!   └─ IDirectDrawSurface7 vtable  (M * 4 bytes: trampoline VAs)
//!
//! IDirectDrawClipper object  (4 bytes: vtable ptr)
//!   └─ IDirectDrawClipper vtable  (K * 4 bytes)
//! ```
//!
//! The surface-id stored in the object at +4 is an index into
//! `GuiState::ddraw_surfaces`, which lives in the guest process state.
//!
//! ## Supported operations
//!
//! - `DirectDrawCreate` / `DirectDrawCreateEx`  →  IDirectDraw7 object
//! - `IDirectDraw7::SetCooperativeLevel`         →  S_OK
//! - `IDirectDraw7::SetDisplayMode`              →  S_OK (records w/h/bpp)
//! - `IDirectDraw7::CreateSurface`               →  IDirectDrawSurface7
//!     - DDSCAPS_PRIMARYSURFACE  →  primary (front-buffer)
//!     - DDSCAPS_OFFSCREENPLAIN  →  offscreen (back-buffer / sprite sheet)
//!     - With DDSCAPS_FLIP / backBufferCount → automatically creates a back surface
//! - `IDirectDrawSurface7::Lock` / `Unlock`      →  direct pixel pointer
//! - `IDirectDrawSurface7::Blt` / `BltFast`      →  surface-to-surface blit
//! - `IDirectDrawSurface7::Flip`                 →  back→primary, emits Blit event
//! - `IDirectDrawSurface7::SetColorKey`          →  records transparent colour
//! - `IDirectDrawSurface7::GetAttachedSurface`   →  back-buffer handle
//! - `IDirectDrawSurface7::Release`              →  frees surface slot
//! - `IDirectDraw7::CreateClipper`               →  IDirectDrawClipper (stub)
//! - `IDirectDrawSurface7::SetClipper`           →  no-op
//! - `IDirectDraw7::QueryInterface`              →  self (IID ignored, works for
//!                                                   IDirectDraw → IDirectDraw7 upgrades)

use crate::vm::process::{DDrawSurface, UiEvent};
use crate::winapi::context::{ApiContext, Handled};
use crate::winapi::WinApiRegistry;

// HRESULT constants
const S_OK: u32 = 0x0000_0000;
const DDERR_GENERIC: u32 = 0x8876_03E8; // returned on real errors
const DDERR_INVALIDPARAMS: u32 = 0x8876_0057;

// DDSCAPS flags (from ddraw.h)
const DDSCAPS_PRIMARYSURFACE: u32 = 0x0000_0200;
const DDSCAPS_OFFSCREENPLAIN: u32 = 0x0000_0040;
const DDSCAPS_FLIP: u32 = 0x0000_0400; // front/back flip chain
const DDSCAPS_BACKBUFFER: u32 = 0x0000_0004;

// DDSURFACEDESC2 offsets (simplified flat layout we care about)
// typedef struct { DWORD dwSize; DWORD dwFlags; DWORD dwHeight; DWORD dwWidth;
//   LONG  lPitch; DWORD dwBackBufferCount; ... DDSCAPS2 ddsCaps; ... LPVOID lpSurface; }
const DESC_FLAGS: u32 = 4;
const DESC_HEIGHT: u32 = 8;
const DESC_WIDTH: u32 = 12;
const DESC_PITCH: u32 = 16;
const DESC_BACK_COUNT: u32 = 20;
const DESC_SURFACE_PTR: u32 = 108; // lpSurface (varies by SDK; most common offset)
const DESC_CAPS_CAPS1: u32 = 96; // DDSCAPS2.dwCaps (within DDSURFACEDESC2)

// DDSD flags
const DDSD_CAPS: u32 = 0x0000_0001;
const DDSD_HEIGHT: u32 = 0x0000_0002;
const DDSD_WIDTH: u32 = 0x0000_0004;
const DDSD_BACKBUFFERCOUNT: u32 = 0x0000_0020;
const DDSD_PITCH: u32 = 0x0000_0008;

// vtable slot counts
// IDirectDraw7 has 33 methods (3 IUnknown + 30 DDraw-specific).
const IDDRAW7_SLOTS: usize = 33;
// IDirectDrawSurface7 has 37 methods.
const IDDSURFACE7_SLOTS: usize = 37;
// IDirectDrawClipper has 8 methods.
const IDDCLIPPER_SLOTS: usize = 8;

// "magic" surface IDs stored inside each surface object at offset +4
// Surface ID 0 is reserved / "none".

// vtable method name tables
// Names are used as the trampoline key registered with WinApiRegistry.
// The fake DLL name "ddraw.vtbl" separates them from IAT imports.
const IDDRAW7_METHODS: &[&str] = &[
    // IUnknown
    "IDirectDraw7::QueryInterface", // 0
    "IDirectDraw7::AddRef",         // 1
    "IDirectDraw7::Release",        // 2
    // IDirectDraw7
    "IDirectDraw7::Compact",                // 3
    "IDirectDraw7::CreateClipper",          // 4
    "IDirectDraw7::CreatePalette",          // 5
    "IDirectDraw7::CreateSurface",          // 6
    "IDirectDraw7::DuplicateSurface",       // 7
    "IDirectDraw7::EnumDisplayModes",       // 8
    "IDirectDraw7::EnumSurfaces",           // 9
    "IDirectDraw7::FlipToGDISurface",       // 10
    "IDirectDraw7::GetCaps",                // 11
    "IDirectDraw7::GetDisplayMode",         // 12
    "IDirectDraw7::GetFourCCCodes",         // 13
    "IDirectDraw7::GetGDISurface",          // 14
    "IDirectDraw7::GetMonitorFrequency",    // 15
    "IDirectDraw7::GetScanLine",            // 16
    "IDirectDraw7::GetVerticalBlankStatus", //17
    "IDirectDraw7::Initialize",             // 18
    "IDirectDraw7::RestoreDisplayMode",     // 19
    "IDirectDraw7::SetCooperativeLevel",    // 20
    "IDirectDraw7::SetDisplayMode",         // 21
    "IDirectDraw7::WaitForVerticalBlank",   //22
    "IDirectDraw7::GetAvailableVidMem",     // 23
    "IDirectDraw7::GetSurfaceFromDC",       // 24
    "IDirectDraw7::RestoreAllSurfaces",     // 25
    "IDirectDraw7::TestCooperativeLevel",   //26
    "IDirectDraw7::GetDeviceIdentifier",    // 27
    "IDirectDraw7::StartModeTest",          // 28
    "IDirectDraw7::EvaluateMode",           // 29
    // IDirectDraw4 extras promoted to 7 (some impls add these)
    "IDirectDraw7::_reserved30", // 30
    "IDirectDraw7::_reserved31", // 31
    "IDirectDraw7::_reserved32", // 32
];

const IDDSURFACE7_METHODS: &[&str] = &[
    // IUnknown
    "IDirectDrawSurface7::QueryInterface", // 0
    "IDirectDrawSurface7::AddRef",         // 1
    "IDirectDrawSurface7::Release",        // 2
    // IDirectDrawSurface7
    "IDirectDrawSurface7::AddAttachedSurface",    // 3
    "IDirectDrawSurface7::AddOverlayDirtyRect",   // 4
    "IDirectDrawSurface7::Blt",                   // 5
    "IDirectDrawSurface7::BltBatch",              // 6
    "IDirectDrawSurface7::BltFast",               // 7
    "IDirectDrawSurface7::DeleteAttachedSurface", //8
    "IDirectDrawSurface7::EnumAttachedSurfaces",  // 9
    "IDirectDrawSurface7::EnumOverlayZOrders",    // 10
    "IDirectDrawSurface7::Flip",                  // 11
    "IDirectDrawSurface7::GetAttachedSurface",    // 12
    "IDirectDrawSurface7::GetBltStatus",          // 13
    "IDirectDrawSurface7::GetCaps",               // 14
    "IDirectDrawSurface7::GetClipper",            // 15
    "IDirectDrawSurface7::GetColorKey",           // 16
    "IDirectDrawSurface7::GetDC",                 // 17
    "IDirectDrawSurface7::GetFlipStatus",         // 18
    "IDirectDrawSurface7::GetOverlayPosition",    // 19
    "IDirectDrawSurface7::GetPalette",            // 20
    "IDirectDrawSurface7::GetPixelFormat",        // 21
    "IDirectDrawSurface7::GetSurfaceDesc",        // 22
    "IDirectDrawSurface7::Initialize",            // 23
    "IDirectDrawSurface7::IsLost",                // 24
    "IDirectDrawSurface7::Lock",                  // 25
    "IDirectDrawSurface7::ReleaseDC",             // 26
    "IDirectDrawSurface7::Restore",               // 27
    "IDirectDrawSurface7::SetClipper",            // 28
    "IDirectDrawSurface7::SetColorKey",           // 29
    "IDirectDrawSurface7::SetOverlayPosition",    // 30
    "IDirectDrawSurface7::SetPalette",            // 31
    "IDirectDrawSurface7::Unlock",                // 32
    "IDirectDrawSurface7::UpdateOverlay",         // 33
    "IDirectDrawSurface7::UpdateOverlayDisplay",  // 34
    "IDirectDrawSurface7::UpdateOverlayZOrder",   // 35
    "IDirectDrawSurface7::GetDDInterface",        // 36
];

const IDDCLIPPER_METHODS: &[&str] = &[
    "IDirectDrawClipper::QueryInterface",    // 0
    "IDirectDrawClipper::AddRef",            // 1
    "IDirectDrawClipper::Release",           // 2
    "IDirectDrawClipper::GetClipList",       // 3
    "IDirectDrawClipper::GetHWnd",           // 4
    "IDirectDrawClipper::Initialize",        // 5
    "IDirectDrawClipper::IsClipListChanged", // 6
    "IDirectDrawClipper::SetClipList",       // 7
];

const IDIRECT3D8_METHODS: &[&str] = &[
    "IDirect3D8::QueryInterface", // 0
    "IDirect3D8::AddRef", // 1
    "IDirect3D8::Release", // 2
    "IDirect3D8::RegisterSoftwareDevice", // 3
    "IDirect3D8::GetAdapterCount", // 4
    "IDirect3D8::GetAdapterIdentifier", // 5
    "IDirect3D8::GetAdapterModeCount", // 6
    "IDirect3D8::EnumAdapterModes", // 7
    "IDirect3D8::GetAdapterDisplayMode", // 8
    "IDirect3D8::CheckDeviceType", // 9
    "IDirect3D8::CheckDeviceFormat", // 10
    "IDirect3D8::CheckDeviceMultiSampleType", // 11
    "IDirect3D8::CheckDepthStencilMatch", // 12
    "IDirect3D8::GetDeviceCaps", // 13
    "IDirect3D8::GetAdapterMonitor", // 14
    "IDirect3D8::CreateDevice", // 15
];

const IDIRECT3DDEVICE8_METHODS: &[&str] = &[
    "IDirect3DDevice8::QueryInterface", // 0
    "IDirect3DDevice8::AddRef", // 1
    "IDirect3DDevice8::Release", // 2
    "IDirect3DDevice8::TestCooperativeLevel", // 3
    "IDirect3DDevice8::GetAvailableTextureMem", // 4
    "IDirect3DDevice8::ResourceManagerDiscardBytes", // 5
    "IDirect3DDevice8::GetDirect3D", // 6
    "IDirect3DDevice8::GetDeviceCaps", // 7
    "IDirect3DDevice8::GetDisplayMode", // 8
    "IDirect3DDevice8::GetCreationParameters", // 9
    "IDirect3DDevice8::SetCursorProperties", // 10
    "IDirect3DDevice8::SetCursorPosition", // 11
    "IDirect3DDevice8::ShowCursor", // 12
    "IDirect3DDevice8::CreateAdditionalSwapChain", // 13
    "IDirect3DDevice8::Reset", // 14
    "IDirect3DDevice8::Present", // 15
    "IDirect3DDevice8::GetBackBuffer", // 16
    "IDirect3DDevice8::GetRasterStatus", // 17
    "IDirect3DDevice8::SetGammaRamp", // 18
    "IDirect3DDevice8::GetGammaRamp", // 19
    "IDirect3DDevice8::CreateTexture", // 20
    "IDirect3DDevice8::CreateVolumeTexture", // 21
    "IDirect3DDevice8::CreateCubeTexture", // 22
    "IDirect3DDevice8::CreateVertexBuffer", // 23
    "IDirect3DDevice8::CreateIndexBuffer", // 24
    "IDirect3DDevice8::CreateRenderTarget", // 25
    "IDirect3DDevice8::CreateDepthStencilSurface", // 26
    "IDirect3DDevice8::CreateImageSurface", // 27
    "IDirect3DDevice8::CopyRects", // 28
    "IDirect3DDevice8::UpdateTexture", // 29
    "IDirect3DDevice8::GetFrontBuffer", // 30
    "IDirect3DDevice8::SetRenderTarget", // 31
    "IDirect3DDevice8::GetRenderTarget", // 32
    "IDirect3DDevice8::GetDepthStencilSurface", // 33
    "IDirect3DDevice8::BeginScene", // 34
    "IDirect3DDevice8::EndScene", // 35
    "IDirect3DDevice8::Clear", // 36
    "IDirect3DDevice8::SetTransform", // 37
    "IDirect3DDevice8::GetTransform", // 38
    "IDirect3DDevice8::MultiplyTransform", // 39
    "IDirect3DDevice8::SetViewport", // 40
    "IDirect3DDevice8::GetViewport", // 41
    "IDirect3DDevice8::SetMaterial", // 42
    "IDirect3DDevice8::GetMaterial", // 43
    "IDirect3DDevice8::SetLight", // 44
    "IDirect3DDevice8::GetLight", // 45
    "IDirect3DDevice8::LightEnable", // 46
    "IDirect3DDevice8::GetLightEnable", // 47
    "IDirect3DDevice8::SetClipPlane", // 48
    "IDirect3DDevice8::GetClipPlane", // 49
    "IDirect3DDevice8::SetRenderState", // 50
    "IDirect3DDevice8::GetRenderState", // 51
    "IDirect3DDevice8::BeginStateBlock", // 52
    "IDirect3DDevice8::EndStateBlock", // 53
    "IDirect3DDevice8::ApplyStateBlock", // 54
    "IDirect3DDevice8::CaptureStateBlock", // 55
    "IDirect3DDevice8::DeleteStateBlock", // 56
    "IDirect3DDevice8::CreateStateBlock", // 57
    "IDirect3DDevice8::SetClipStatus", // 58
    "IDirect3DDevice8::GetClipStatus", // 59
    "IDirect3DDevice8::GetTexture", // 60
    "IDirect3DDevice8::SetTexture", // 61
    "IDirect3DDevice8::GetTextureStageState", // 62
    "IDirect3DDevice8::SetTextureStageState", // 63
    "IDirect3DDevice8::ValidateDevice", // 64
    "IDirect3DDevice8::GetInfo", // 65
    "IDirect3DDevice8::SetPaletteEntries", // 66
    "IDirect3DDevice8::GetPaletteEntries", // 67
    "IDirect3DDevice8::SetCurrentTexturePalette", // 68
    "IDirect3DDevice8::GetCurrentTexturePalette", // 69
    "IDirect3DDevice8::DrawPrimitive", // 70
    "IDirect3DDevice8::DrawIndexedPrimitive", // 71
    "IDirect3DDevice8::DrawPrimitiveUP", // 72
    "IDirect3DDevice8::DrawIndexedPrimitiveUP", // 73
    "IDirect3DDevice8::ProcessVertices", // 74
    "IDirect3DDevice8::CreateVertexShader", // 75
    "IDirect3DDevice8::SetVertexShader", // 76
    "IDirect3DDevice8::GetVertexShader", // 77
    "IDirect3DDevice8::DeleteVertexShader", // 78
    "IDirect3DDevice8::SetVertexShaderConstant", // 79
    "IDirect3DDevice8::GetVertexShaderConstant", // 80
    "IDirect3DDevice8::GetVertexShaderDeclaration", // 81
    "IDirect3DDevice8::GetVertexShaderFunction", // 82
    "IDirect3DDevice8::SetStreamSource", // 83
    "IDirect3DDevice8::GetStreamSource", // 84
    "IDirect3DDevice8::SetIndices", // 85
    "IDirect3DDevice8::GetIndices", // 86
    "IDirect3DDevice8::CreatePixelShader", // 87
    "IDirect3DDevice8::SetPixelShader", // 88
    "IDirect3DDevice8::GetPixelShader", // 89
    "IDirect3DDevice8::DeletePixelShader", // 90
    "IDirect3DDevice8::SetPixelShaderConstant", // 91
    "IDirect3DDevice8::GetPixelShaderConstant", // 92
    "IDirect3DDevice8::GetPixelShaderFunction", // 93
];

// Public registration entry point

pub fn register(r: &mut WinApiRegistry) {
    // IAT exports
    r.add("ddraw.dll", "DirectDrawCreate", ddraw_create);
    r.add("ddraw.dll", "DirectDrawCreateEx", ddraw_create_ex);

    // d3d8.dll is also linked by Touhou 6 but only used for device enumeration.
    // Returning E_NOINTERFACE from CoCreateInstance is enough; the game falls
    // back to DDraw when D3D8 isn't available.
    r.add("d3d8.dll", "Direct3DCreate8", d3d8_create_stub);

    // IDirectDraw7 vtable methods
    for name in IDDRAW7_METHODS {
        r.add("ddraw.vtbl", name, dispatch_iddraw7);
    }

    // IDirectDrawSurface7 vtable methods
    for name in IDDSURFACE7_METHODS {
        r.add("ddraw.vtbl", name, dispatch_iddsurface7);
    }

    // IDirectDrawClipper vtable methods
    for name in IDDCLIPPER_METHODS {
        r.add("ddraw.vtbl", name, dispatch_iddclipper);
    }

    // IDirect3D8 vtable methods
    for name in IDIRECT3D8_METHODS {
        r.add("ddraw.vtbl", name, dispatch_idirect3d8);
    }

    // IDirect3DDevice8 vtable methods
    for name in IDIRECT3DDEVICE8_METHODS {
        r.add("ddraw.vtbl", name, dispatch_idirect3ddevice8);
    }
}

// COM object constructors (in guest heap)

/// Allocate a vtable + object in the guest heap.  Returns the object VA.
///
/// Layout:
///   object VA  +0:  vtable_va  (4 bytes)
///   object VA  +4:  extra      (4 bytes, used to store surface-id for surfaces)
///
/// The vtable is an array of trampoline VAs, one per method slot.
fn alloc_com_object(ctx: &mut ApiContext, method_names: &[&str], extra: u32) -> u32 {
    // Allocate vtable: N slots × 4 bytes
    let vtable_size = (method_names.len() * 4) as u32;
    let vtable_va = ctx.heap_alloc(vtable_size);

    for (i, name) in method_names.iter().enumerate() {
        let tramp = ctx.api_resolve_trampoline("ddraw.vtbl", name);
        let _ = ctx.memory.write_u32(vtable_va + i as u32 * 4, tramp);
    }

    // Allocate object: vtable ptr (4) + extra (4)
    let obj_va = ctx.heap_alloc(8);
    let _ = ctx.memory.write_u32(obj_va, vtable_va);
    let _ = ctx.memory.write_u32(obj_va + 4, extra);
    obj_va
}

// Surface-id helpers

/// Read the surface-id stored at object+4.
fn surface_id_of(ctx: &ApiContext, obj_va: u32) -> u32 {
    ctx.memory.read_u32(obj_va + 4).unwrap_or(0)
}

// IAT-level exports

/// `DirectDrawCreate(lpGUID, lplpDD, pUnkOuter) -> HRESULT`
///
/// Creates an IDirectDraw7 object (we always return the v7 interface regardless
/// of the requested version; the QueryInterface upgrade path is transparent).
fn ddraw_create(ctx: &mut ApiContext) -> Handled {
    // args: lpGUID (0=default), lplpDD (out ptr), pUnkOuter
    let out_ptr = ctx.arg(1);

    let obj_va = alloc_com_object(ctx, IDDRAW7_METHODS, 0);

    if out_ptr != 0 {
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }

    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

/// `DirectDrawCreateEx(lpGUID, lplpDD, iid, pUnkOuter) -> HRESULT`
fn ddraw_create_ex(ctx: &mut ApiContext) -> Handled {
    let out_ptr = ctx.arg(1);
    // iid (arg 2) is ignored — we always return IDirectDraw7 vtable layout.

    let obj_va = alloc_com_object(ctx, IDDRAW7_METHODS, 0);
    if out_ptr != 0 {
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }

    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

/// `Direct3DCreate8(SDKVersion) -> IDirect3D8*`
///
/// Touhou 6 imports this from d3d8.dll for device enumeration.  We return NULL
/// which makes the game skip D3D enumeration and fall back to DDraw properly.
fn d3d8_create_stub(ctx: &mut ApiContext) -> Handled {
    let obj_va = alloc_com_object(ctx, IDIRECT3D8_METHODS, 0);
    ctx.ret_stdcall(obj_va, 1);
    Handled::Ok
}

fn dispatch_idirect3d8(ctx: &mut ApiContext) -> Handled {
    let slot = IDIRECT3D8_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("ddraw.vtbl", name) == ctx.current_trampoline_va())
        .unwrap_or(99);
    match slot {
        0 => { // QueryInterface
            let this = ctx.arg(0);
            let out = ctx.arg(2);
            if out != 0 { let _ = ctx.memory.write_u32(out, this); }
            ctx.ret_stdcall(S_OK, 3);
        }
        1 | 2 => ctx.ret_stdcall(1, 1), // AddRef / Release
        4 => ctx.ret_stdcall(1, 1), // GetAdapterCount -> returns 1 (no HRESULT, direct u32)
        5 => { // GetAdapterIdentifier(this, adapter, flags, pIdentifier)
            let out = ctx.arg(3);
            if out != 0 {
                let name = b"WebWINE D3D8 Stub\0";
                let _ = ctx.memory.write_bytes(out, name);
                let _ = ctx.memory.write_bytes(out + 512, name);
                let _ = ctx.memory.write_u32(out + 1024, 0x1234); // vendor id
            }
            ctx.ret_stdcall(S_OK, 4);
        }
        6 => ctx.ret_stdcall(1, 2), // GetAdapterModeCount -> returns 1
        8 => { // GetAdapterDisplayMode(this, adapter, pMode)
            let out = ctx.arg(2);
            if out != 0 {
                let _ = ctx.memory.write_u32(out, 640);
                let _ = ctx.memory.write_u32(out + 4, 480);
                let _ = ctx.memory.write_u32(out + 8, 0); // RefreshRate
                let _ = ctx.memory.write_u32(out + 12, 22); // D3DFMT_X8R8G8B8
            }
            ctx.ret_stdcall(S_OK, 3);
        }
        13 => { // GetDeviceCaps(this, adapter, devtype, pCaps)
            let out = ctx.arg(3);
            if out != 0 {
                // Return a generous D3DCAPS8 structure
                let _ = ctx.memory.write_u32(out, 1); // DeviceType
                let _ = ctx.memory.write_u32(out + 12, 0xFFFF_FFFF); // Caps
                let _ = ctx.memory.write_u32(out + 16, 0xFFFF_FFFF); // Caps2
            }
            ctx.ret_stdcall(S_OK, 4);
        }
        15 => { // CreateDevice(this, adapter, devtype, hwnd, behaviorflags, pparams, ppdevice)
            let out_ptr = ctx.arg(6);
            if out_ptr != 0 {
                let dev_va = alloc_com_object(ctx, IDIRECT3DDEVICE8_METHODS, 0);
                let _ = ctx.memory.write_u32(out_ptr, dev_va);
            }
            ctx.ret_stdcall(S_OK, 7);
        }
        _ => { // default
            let nargs = match slot {
                3 => 2, 7 => 4, 9 => 6, 10 => 6, 11 => 6, 12 => 6, 14 => 2,
                _ => 1,
            };
            ctx.ret_stdcall(S_OK, nargs);
        }
    }
    Handled::Ok
}

fn dispatch_idirect3ddevice8(ctx: &mut ApiContext) -> Handled {
    let slot = IDIRECT3DDEVICE8_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("ddraw.vtbl", name) == ctx.current_trampoline_va())
        .unwrap_or(999);
    
    match slot {
        0 => { // QueryInterface
            let this = ctx.arg(0);
            let out = ctx.arg(2);
            if out != 0 { let _ = ctx.memory.write_u32(out, this); }
            ctx.ret_stdcall(S_OK, 3);
        }
        1 | 2 => ctx.ret_stdcall(1, 1),
        // Just return S_OK for everything with a very crude arg count guess
        _ => {
            // Most D3D8 device methods take between 1 and 6 arguments.
            // We can guess safely for stdcall by looking at typical methods, 
            // but returning S_OK and popping 1 arg (just `this`) will eventually
            // unbalance the stack if we guess wrong. We will use a safe stub logic
            // based on the slot to balance the stack properly!
            let nargs = match slot {
                14 => 2, // Reset(pparams)
                15 => 5, // Present(src, dst, wnd, rgn)
                16 => 4, // GetBackBuffer
                20 => 8, // CreateTexture
                23 => 6, // CreateVertexBuffer
                24 => 6, // CreateIndexBuffer
                34 | 35 => 1, // BeginScene/EndScene
                36 => 7, // Clear
                40 => 2, // SetViewport
                50 => 3, // SetRenderState
                61 => 3, // SetTexture
                70 => 4, // DrawPrimitive
                76 => 2, // SetVertexShader
                83 => 4, // SetStreamSource
                85 => 3, // SetIndices
                _ => 4, // Wild guess
            };
            ctx.ret_stdcall(S_OK, nargs);
        }
    }
    Handled::Ok
}

// IDirectDraw7 dispatch

fn dispatch_iddraw7(ctx: &mut ApiContext) -> Handled {
    // Determine which slot was called by looking up the trampoline VA in EIP
    // (the executor sets eip to the trampoline before dispatching).
    let slot = match iddraw7_slot(ctx) {
        Some(s) => s,
        None => {
            ctx.ret_stdcall(S_OK, 1);
            return Handled::Ok;
        }
    };
    match slot {
        0 => iddraw7_query_interface(ctx),
        1 | 2 => {
            ctx.ret_stdcall(1, 1);
            Handled::Ok
        } // AddRef / Release
        3 => {
            ctx.ret_stdcall(S_OK, 1);
            Handled::Ok
        } // Compact
        4 => iddraw7_create_clipper(ctx),
        5 => {
            ctx.ret_stdcall(S_OK, 4);
            Handled::Ok
        } // CreatePalette
        6 => iddraw7_create_surface(ctx),
        8 => {
            ctx.ret_stdcall(S_OK, 3);
            Handled::Ok
        } // EnumDisplayModes
        11 => iddraw7_get_caps(ctx),
        12 => iddraw7_get_display_mode(ctx),
        15 => {
            // GetMonitorFrequency
            let out = ctx.arg(1);
            if out != 0 {
                let _ = ctx.memory.write_u32(out, 60);
            }
            ctx.ret_stdcall(S_OK, 2);
            Handled::Ok
        }
        17 => {
            // GetVerticalBlankStatus
            let out = ctx.arg(1);
            if out != 0 {
                let _ = ctx.memory.write_u32(out, 0);
            }
            ctx.ret_stdcall(S_OK, 2);
            Handled::Ok
        }
        19 => {
            ctx.ret_stdcall(S_OK, 1);
            Handled::Ok
        } // RestoreDisplayMode
        20 => {
            ctx.ret_stdcall(S_OK, 3);
            Handled::Ok
        } // SetCooperativeLevel
        21 => iddraw7_set_display_mode(ctx),
        22 => {
            ctx.ret_stdcall(S_OK, 3);
            Handled::Ok
        } // WaitForVerticalBlank
        23 => iddraw7_get_available_vidmem(ctx),
        25 => {
            ctx.ret_stdcall(S_OK, 1);
            Handled::Ok
        } // RestoreAllSurfaces
        26 => {
            ctx.ret_stdcall(S_OK, 1);
            Handled::Ok
        } // TestCooperativeLevel
        27 => iddraw7_get_device_identifier(ctx),
        _ => {
            ctx.ret_stdcall(S_OK, 1);
            Handled::Ok
        }
    }
}

fn iddraw7_slot(ctx: &ApiContext) -> Option<usize> {
    let tramp_va = ctx.current_trampoline_va(); // EIP at time of call
    IDDRAW7_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("ddraw.vtbl", name) == tramp_va)
}

fn iddraw7_query_interface(ctx: &mut ApiContext) -> Handled {
    // QueryInterface(this, riid, ppvObject)
    // We return `this` for any IID — covers IDirectDraw → IDirectDraw7 upgrades.
    let this = ctx.arg(0);
    let out = ctx.arg(2);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, this);
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

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
            crate::vm::process::DDrawSurfaceKind::Primary,
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
        (dw, dh, crate::vm::process::DDrawSurfaceKind::Offscreen)
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
                    kind: crate::vm::process::DDrawSurfaceKind::Offscreen,
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
    let obj_va = alloc_com_object(ctx, IDDSURFACE7_METHODS, sid);
    let _ = ctx.memory.write_u32(out_ptr, obj_va);

    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn iddraw7_create_clipper(ctx: &mut ApiContext) -> Handled {
    // CreateClipper(this, dwFlags, lplpDDClipper, pUnkOuter)
    let out_ptr = ctx.arg(2);
    let obj_va = alloc_com_object(ctx, IDDCLIPPER_METHODS, 0);
    if out_ptr != 0 {
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn iddraw7_get_caps(ctx: &mut ApiContext) -> Handled {
    // GetCaps(this, lpDDDriverCaps, lpDDHELCaps) — fill with generous caps so
    // capability checks pass.  DDCAPS is 344 bytes; we only set the key fields.
    for out in [ctx.arg(1), ctx.arg(2)] {
        if out != 0 {
            let _ = ctx.memory.write_u32(out, 344); // dwSize
            let _ = ctx.memory.write_u32(out + 4, 0xFFFF_FFFF); // dwCaps  (all)
            let _ = ctx.memory.write_u32(out + 8, 0xFFFF_FFFF); // dwCaps2
            let _ = ctx.memory.write_u32(out + 12, 0xFFFF_FFFF); // dwCKeyCaps
            let _ = ctx.memory.write_u32(out + 16, 0xFFFF_FFFF); // dwFXCaps
                                                                 // dwVidMemTotal / dwVidMemFree — claim 32 MB available.
            let _ = ctx.memory.write_u32(out + 80, 32 * 1024 * 1024);
            let _ = ctx.memory.write_u32(out + 84, 32 * 1024 * 1024);
        }
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn iddraw7_get_display_mode(ctx: &mut ApiContext) -> Handled {
    // GetDisplayMode(this, lpDDSurfaceDesc2)
    let desc = ctx.arg(1);
    if desc != 0 {
        let _ = ctx
            .memory
            .write_u32(desc + DESC_WIDTH, ctx.gui.ddraw_display_w);
        let _ = ctx
            .memory
            .write_u32(desc + DESC_HEIGHT, ctx.gui.ddraw_display_h);
        // Pixel format BPP at DDSURFACEDESC2.ddpfPixelFormat.dwRGBBitCount (+88+8)
        let _ = ctx.memory.write_u32(desc + 96, ctx.gui.ddraw_display_bpp);
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
    // DDDEVICEIDENTIFIER2 is 784 bytes; just zero it out and fill the string.
    let out = ctx.arg(1);
    if out != 0 {
        // szDriver (MAX_PATH = 260): "WebWINE DDraw"
        let name = b"WebWINE DDraw\0";
        let _ = ctx.memory.write_bytes(out, name);
        // UniqueId.dwVendorId at +524
        let _ = ctx.memory.write_u32(out + 524, 0x1234);
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

// IDirectDrawSurface7 dispatch

fn dispatch_iddsurface7(ctx: &mut ApiContext) -> Handled {
    let slot = match iddsurface7_slot(ctx) {
        Some(s) => s,
        None => {
            ctx.ret_stdcall(S_OK, 1);
            return Handled::Ok;
        }
    };
    match slot {
        0 => iddsurface7_query_interface(ctx),
        1 => {
            ctx.ret_stdcall(1, 1);
            Handled::Ok
        } // AddRef
        2 => iddsurface7_release(ctx),
        5 => iddsurface7_blt(ctx),
        7 => iddsurface7_blt_fast(ctx),
        11 => iddsurface7_flip(ctx),
        12 => iddsurface7_get_attached_surface(ctx),
        13 => {
            ctx.ret_stdcall(0 /* DD_OK = not busy */, 2);
            Handled::Ok
        } // GetBltStatus
        18 => {
            ctx.ret_stdcall(0, 2);
            Handled::Ok
        } // GetFlipStatus
        22 => iddsurface7_get_surface_desc(ctx),
        24 => {
            ctx.ret_stdcall(S_OK, 1);
            Handled::Ok
        } // IsLost → DD_OK = not lost
        25 => iddsurface7_lock(ctx),
        27 => {
            ctx.ret_stdcall(S_OK, 1);
            Handled::Ok
        } // Restore
        28 => {
            ctx.ret_stdcall(S_OK, 2);
            Handled::Ok
        } // SetClipper (no-op)
        29 => iddsurface7_set_color_key(ctx),
        32 => iddsurface7_unlock(ctx),
        36 => {
            // GetDDInterface(this, lplpDD)
            // Return a fresh IDirectDraw7 object — caller just wants to call
            // SetCooperativeLevel on it again.
            let out = ctx.arg(1);
            let obj = alloc_com_object(ctx, IDDRAW7_METHODS, 0);
            if out != 0 {
                let _ = ctx.memory.write_u32(out, obj);
            }
            ctx.ret_stdcall(S_OK, 2);
            Handled::Ok
        }
        _ => {
            ctx.ret_stdcall(S_OK, 1);
            Handled::Ok
        }
    }
}

fn iddsurface7_slot(ctx: &ApiContext) -> Option<usize> {
    let tramp_va = ctx.current_trampoline_va();
    IDDSURFACE7_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("ddraw.vtbl", name) == tramp_va)
}

fn iddsurface7_query_interface(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(2);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, this);
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

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
            // Write pitch and lpSurface back so the guest can write pixels.
            let _ = ctx.memory.write_u32(desc_va + DESC_PITCH, surf.stride);
            let _ = ctx
                .memory
                .write_u32(desc_va + DESC_SURFACE_PTR, surf.pixels_va);
            let _ = ctx.memory.write_u32(desc_va + DESC_WIDTH, surf.width);
            let _ = ctx.memory.write_u32(desc_va + DESC_HEIGHT, surf.height);
        }
    }
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}

fn iddsurface7_unlock(ctx: &mut ApiContext) -> Handled {
    // Unlock(this, lpRect) — just acknowledge; pixels are already in guest mem.
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn iddsurface7_flip(ctx: &mut ApiContext) -> Handled {
    // Flip(this, lpDDSurfaceTargetOverride, dwFlags)
    // Copies back-buffer pixels → primary, then emits a Blit event.
    let this = ctx.arg(0);
    let sid = surface_id_of(ctx, this);

    // Gather what we need before any mutable borrow.
    let (primary_id, back_id, w, h) = {
        if let Some(surf) = ctx.gui.ddraw_surfaces.get(&sid) {
            let primary = if matches!(surf.kind, crate::vm::process::DDrawSurfaceKind::Primary) {
                sid
            } else {
                sid // fallback: treat self as primary
            };
            (primary, surf.back_id, surf.width, surf.height)
        } else {
            ctx.ret_stdcall(S_OK, 3);
            return Handled::Ok;
        }
    };

    // Determine which surface holds the pixels to display.
    let src_sid = back_id.unwrap_or(primary_id);

    let pixels = read_surface_rgba(ctx, src_sid);

    // Find a window to blit to (use the first visible one, or hwnd 4 as default).
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
    let dest_rect = ctx.arg(1); // may be NULL → whole surface
    let src_obj = ctx.arg(2); // IDirectDrawSurface7* (may be NULL for solid fills)
    let src_rect = ctx.arg(3);

    let dst_sid = surface_id_of(ctx, this);
    let src_sid = if src_obj != 0 {
        surface_id_of(ctx, src_obj)
    } else {
        0
    };

    // Read dest rect (or use full surface).
    let (dst_x, dst_y, dst_w, dst_h) = read_rect_or_full(ctx, dest_rect, dst_sid);
    let (sx, sy, sw, sh) = read_rect_or_full(ctx, src_rect, src_sid);

    if src_obj == 0 {
        // DDBLT_COLORFILL: fill dest region with colour from BltFx.dwFillColor.
        let blt_fx = ctx.arg(5);
        let fill_color = if blt_fx != 0 {
            ctx.memory.read_u32(blt_fx + 16).unwrap_or(0) // dwFillColor at +16
        } else {
            0
        };
        surface_fill(ctx, dst_sid, dst_x, dst_y, dst_w, dst_h, fill_color);
    } else {
        surface_blit(
            ctx, src_sid, sx, sy, sw, sh, dst_sid, dst_x, dst_y, dst_w, dst_h,
        );
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
    let trans = ctx.arg(5); // DDBLTFAST_SRCCOLORKEY = 1

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
    // DDCKEY_SRCBLT = 0x08; we just record the low-color value.
    let this = ctx.arg(0);
    let ck_struct = ctx.arg(2); // DDCOLORKEY { dwColorSpaceLowValue, dwColorSpaceHighValue }
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
        let obj_va = alloc_com_object(ctx, IDDSURFACE7_METHODS, bsid);
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
                crate::vm::process::DDrawSurfaceKind::Primary => {
                    DDSCAPS_PRIMARYSURFACE | DDSCAPS_FLIP
                }
                crate::vm::process::DDrawSurfaceKind::Offscreen => DDSCAPS_OFFSCREENPLAIN,
            };
            let _ = ctx.memory.write_u32(desc_va + DESC_WIDTH, surf.width);
            let _ = ctx.memory.write_u32(desc_va + DESC_HEIGHT, surf.height);
            let _ = ctx.memory.write_u32(desc_va + DESC_PITCH, surf.stride);
            let _ = ctx.memory.write_u32(desc_va + DESC_CAPS_CAPS1, caps);
            let _ = ctx
                .memory
                .write_u32(desc_va + DESC_SURFACE_PTR, surf.pixels_va);
        }
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

// IDirectDrawClipper dispatch

fn dispatch_iddclipper(ctx: &mut ApiContext) -> Handled {
    // All Clipper methods are no-ops for our purposes; just balance the stack.
    // Slot 0 = QI (3 args), 1/2 = AddRef/Release (1 arg), rest vary.
    let slot = IDDCLIPPER_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("ddraw.vtbl", name) == ctx.current_trampoline_va())
        .unwrap_or(99);
    let nargs: u32 = match slot {
        0 => 3,
        1 | 2 => 1,
        3 | 5 | 6 => 2,
        4 => 2,
        7 => 3,
        _ => 1,
    };
    ctx.ret_stdcall(S_OK, nargs);
    Handled::Ok
}

// Pixel buffer helpers

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

/// Blit a region from src surface to dst surface in guest memory.
/// Handles colour-key transparency: pixels matching src.color_key are skipped.
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
    // Gather surface info without holding a borrow on ctx.gui.
    let src_info = ctx.gui.ddraw_surfaces.get(&src_sid).map(|s| {
        (
            s.pixels_va,
            s.width as i32,
            s.height as i32,
            s.stride as i32,
            s.color_key,
        )
    });
    let dst_info = ctx.gui.ddraw_surfaces.get(&dst_sid).map(|s| {
        (
            s.pixels_va,
            s.width as i32,
            s.height as i32,
            s.stride as i32,
        )
    });

    let (sp, sw_full, sh_full, s_stride, color_key) = match src_info {
        Some(v) => v,
        None => return,
    };
    let (dp, _dw_full, _dh_full, d_stride) = match dst_info {
        Some(v) => v,
        None => return,
    };

    // Scale factor for stretching (integer is fine for the common 1:1 case).
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

            // Colour-key check: skip transparent pixels.
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
        Some(s) => (
            s.pixels_va,
            s.width as i32,
            s.height as i32,
            s.stride as i32,
        ),
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

/// Read a surface's pixel buffer and convert BGRA8888 → RGBA8888 for the
/// frontend canvas (`putImageData` expects RGBA).
fn read_surface_rgba(ctx: &ApiContext, sid: u32) -> Vec<u8> {
    let (pixels_va, w, h, stride) = match ctx.gui.ddraw_surfaces.get(&sid) {
        Some(s) => (
            s.pixels_va,
            s.width as i32,
            s.height as i32,
            s.stride as i32,
        ),
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
            out[dst + 3] = if a == 0 { 0xFF } else { a }; // force opaque if alpha is 0
        }
    }
    out
}
