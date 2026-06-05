//! DirectInput 8 stub (dinput8.dll). Devices are created and acknowledged but
//! report no input data (GetDeviceData returns 0 items). COM mechanism: see the
//! crate root.

use webwine_api::winapi::context::{ApiContext, Handled};
use webwine_api::winapi::WinApiRegistry;

use crate::{
    com_qi, make_object, register_vtable, s0_1, s0_2, s0_3, s0_4, s0_5, s0_6, s1_1, Vtable,
};

pub(crate) const IDIRECTINPUT8: Vtable = &[
    ("IDirectInput8::QueryInterface", com_qi),
    ("IDirectInput8::AddRef", s1_1),
    ("IDirectInput8::Release", s1_1),
    ("IDirectInput8::CreateDevice", di8_create_device),
    ("IDirectInput8::EnumDevices", s0_4),
    ("IDirectInput8::GetDeviceStatus", s0_2),
    ("IDirectInput8::RunControlPanel", s0_3),
    ("IDirectInput8::Initialize", s0_3),
    ("IDirectInput8::FindDevice", s0_4),
    ("IDirectInput8::EnumDevicesBySemantics", s0_6),
    ("IDirectInput8::ConfigureDevices", s0_1),
];

pub(crate) const IDIRECTINPUTDEVICE8: Vtable = &[
    ("IDirectInputDevice8::QueryInterface", com_qi),
    ("IDirectInputDevice8::AddRef", s1_1),
    ("IDirectInputDevice8::Release", s1_1),
    ("IDirectInputDevice8::GetCapabilities", s0_2),
    ("IDirectInputDevice8::EnumObjects", s0_4),
    ("IDirectInputDevice8::GetProperty", s0_2),
    ("IDirectInputDevice8::SetProperty", s0_2),
    ("IDirectInputDevice8::Acquire", s0_1),
    ("IDirectInputDevice8::Unacquire", s0_1),
    ("IDirectInputDevice8::GetDeviceState", s0_3),
    ("IDirectInputDevice8::GetDeviceData", didevice8_get_device_data),
    ("IDirectInputDevice8::SetDataFormat", s0_2),
    ("IDirectInputDevice8::SetEventNotification", s0_2),
    ("IDirectInputDevice8::SetCooperativeLevel", s0_3),
    ("IDirectInputDevice8::GetObjectInfo", s0_4),
    ("IDirectInputDevice8::GetDeviceInfo", s0_2),
    ("IDirectInputDevice8::RunControlPanel", s0_3),
    ("IDirectInputDevice8::Initialize", s0_4),
    ("IDirectInputDevice8::CreateEffect", s0_5),
    ("IDirectInputDevice8::EnumEffects", s0_4),
    ("IDirectInputDevice8::GetEffectInfo", s0_3),
    ("IDirectInputDevice8::GetForceFeedbackState", s0_2),
    ("IDirectInputDevice8::SendForceFeedbackCommand", s0_2),
    ("IDirectInputDevice8::EnumCreatedEffectObjects", s0_3),
    ("IDirectInputDevice8::Escape", s0_2),
    ("IDirectInputDevice8::Poll", s0_1),
    ("IDirectInputDevice8::SendDeviceData", s0_4),
    ("IDirectInputDevice8::EnumEffectsInFile", s0_5),
    ("IDirectInputDevice8::WriteEffectToFile", s0_4),
    ("IDirectInputDevice8::BuildActionMap", s0_4),
    ("IDirectInputDevice8::SetActionMap", s0_3),
    ("IDirectInputDevice8::GetImageInfo", s0_2),
];

pub fn register(r: &mut WinApiRegistry) {
    r.add("dinput8.dll", "DirectInput8Create", direct_input8_create);
    register_vtable(r, IDIRECTINPUT8);
    register_vtable(r, IDIRECTINPUTDEVICE8);
}

fn direct_input8_create(ctx: &mut ApiContext) -> Handled {
    // DirectInput8Create(hinst, dwVersion, riidltf, ppvOut, punkOuter)
    let out_ptr = ctx.arg(3);
    if out_ptr != 0 {
        let obj_va = make_object(ctx, IDIRECTINPUT8, 0);
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(0, 5); // S_OK
    Handled::Ok
}

fn di8_create_device(ctx: &mut ApiContext) -> Handled {
    // CreateDevice(this, rguid, lplpDirectInputDevice, pUnkOuter)
    let out_ptr = ctx.arg(2);
    if out_ptr != 0 {
        let obj_va = make_object(ctx, IDIRECTINPUTDEVICE8, 0);
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(0, 4);
    Handled::Ok
}

fn didevice8_get_device_data(ctx: &mut ApiContext) -> Handled {
    // GetDeviceData(this, cbObjectData, rgdod, pdwInOut, dwFlags) — no data.
    let count_ptr = ctx.arg(3);
    if count_ptr != 0 {
        let _ = ctx.memory.write_u32(count_ptr, 0);
    }
    ctx.ret_stdcall(0, 5);
    Handled::Ok
}
