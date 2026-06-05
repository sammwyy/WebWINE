//! Direct3D 8 stub (d3d8.dll).

use webwine_api::winapi::context::{ApiContext, Handled};
use webwine_api::winapi::WinApiRegistry;

use crate::{
    com_qi, make_object, register_vtable, s0_1, s0_2, s0_3, s0_4, s0_5, s0_6, s0_7, s0_8, s1_1,
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
    ("IDirect3D8::CheckDeviceFormat", s0_6),
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
    ("IDirect3DDevice8::GetDirect3D", s0_4),
    ("IDirect3DDevice8::GetDeviceCaps", s0_4),
    ("IDirect3DDevice8::GetDisplayMode", s0_4),
    ("IDirect3DDevice8::GetCreationParameters", s0_4),
    ("IDirect3DDevice8::SetCursorProperties", s0_4),
    ("IDirect3DDevice8::SetCursorPosition", s0_4),
    ("IDirect3DDevice8::ShowCursor", s0_4),
    ("IDirect3DDevice8::CreateAdditionalSwapChain", s0_4),
    ("IDirect3DDevice8::Reset", s0_2),
    ("IDirect3DDevice8::Present", s0_5),
    ("IDirect3DDevice8::GetBackBuffer", s0_4),
    ("IDirect3DDevice8::GetRasterStatus", s0_4),
    ("IDirect3DDevice8::SetGammaRamp", s0_4),
    ("IDirect3DDevice8::GetGammaRamp", s0_4),
    ("IDirect3DDevice8::CreateTexture", s0_8),
    ("IDirect3DDevice8::CreateVolumeTexture", s0_4),
    ("IDirect3DDevice8::CreateCubeTexture", s0_4),
    ("IDirect3DDevice8::CreateVertexBuffer", s0_6),
    ("IDirect3DDevice8::CreateIndexBuffer", s0_6),
    ("IDirect3DDevice8::CreateRenderTarget", s0_4),
    ("IDirect3DDevice8::CreateDepthStencilSurface", s0_4),
    ("IDirect3DDevice8::CreateImageSurface", s0_4),
    ("IDirect3DDevice8::CopyRects", s0_4),
    ("IDirect3DDevice8::UpdateTexture", s0_4),
    ("IDirect3DDevice8::GetFrontBuffer", s0_4),
    ("IDirect3DDevice8::SetRenderTarget", s0_4),
    ("IDirect3DDevice8::GetRenderTarget", s0_4),
    ("IDirect3DDevice8::GetDepthStencilSurface", s0_4),
    ("IDirect3DDevice8::BeginScene", s0_1),
    ("IDirect3DDevice8::EndScene", s0_1),
    ("IDirect3DDevice8::Clear", s0_7),
    ("IDirect3DDevice8::SetTransform", s0_4),
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
    ("IDirect3DDevice8::DrawIndexedPrimitive", s0_4),
    ("IDirect3DDevice8::DrawPrimitiveUP", s0_4),
    ("IDirect3DDevice8::DrawIndexedPrimitiveUP", s0_4),
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

fn d3d8_create_device(ctx: &mut ApiContext) -> Handled {
    // CreateDevice(this, adapter, devtype, hwnd, behaviorflags, pparams, ppdevice)
    let out_ptr = ctx.arg(6);
    if out_ptr != 0 {
        let dev_va = make_object(ctx, IDIRECT3DDEVICE8, 0);
        let _ = ctx.memory.write_u32(out_ptr, dev_va);
    }
    ctx.ret_stdcall(S_OK, 7);
    Handled::Ok
}
