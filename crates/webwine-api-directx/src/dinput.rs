//! DirectInput 8 (dinput8.dll).
//!
//! Devices create successfully; input reports empty/zero state so games fall
//! back to Win32 message input or run with no joystick.

use webwine_api::winapi::context::{ApiContext, Handled};
use webwine_api::winapi::WinApiRegistry;

use crate::{
    com_addref, com_qi, com_release, hr_ok_2, hr_ok_3, hr_ok_4, hr_ok_5, hr_ok_6, make_object,
    make_object_sized, register_vtable, Vtable,
};

const S_OK: u32 = 0;
const DI_OK: u32 = 0;
const DIERR_INPUTLOST: u32 = 0x8007_001C;
const DIERR_NOTACQUIRED: u32 = 0x8007_000C;

// Device layout: [vtable, acquired, coop_flags]
const DEV_ACQUIRED: u32 = 4;
const DEV_OBJ_SIZE: u32 = 12;

pub(crate) const IDIRECTINPUT8: Vtable = &[
    ("IDirectInput8::QueryInterface", com_qi),
    ("IDirectInput8::AddRef", com_addref),
    ("IDirectInput8::Release", com_release),
    ("IDirectInput8::CreateDevice", di8_create_device),
    ("IDirectInput8::EnumDevices", di8_enum_devices),
    ("IDirectInput8::GetDeviceStatus", di8_get_device_status),
    ("IDirectInput8::RunControlPanel", hr_ok_3),
    ("IDirectInput8::Initialize", hr_ok_3),
    ("IDirectInput8::FindDevice", hr_ok_4),
    ("IDirectInput8::EnumDevicesBySemantics", hr_ok_6),
    ("IDirectInput8::ConfigureDevices", hr_ok_5),
];

pub(crate) const IDIRECTINPUTDEVICE8: Vtable = &[
    ("IDirectInputDevice8::QueryInterface", com_qi),
    ("IDirectInputDevice8::AddRef", com_addref),
    ("IDirectInputDevice8::Release", com_release),
    ("IDirectInputDevice8::GetCapabilities", didev_get_capabilities),
    ("IDirectInputDevice8::EnumObjects", hr_ok_4),
    ("IDirectInputDevice8::GetProperty", hr_ok_3),
    ("IDirectInputDevice8::SetProperty", hr_ok_3),
    ("IDirectInputDevice8::Acquire", didev_acquire),
    ("IDirectInputDevice8::Unacquire", didev_unacquire),
    ("IDirectInputDevice8::GetDeviceState", didev_get_device_state),
    ("IDirectInputDevice8::GetDeviceData", didev_get_device_data),
    ("IDirectInputDevice8::SetDataFormat", hr_ok_2),
    ("IDirectInputDevice8::SetEventNotification", hr_ok_2),
    ("IDirectInputDevice8::SetCooperativeLevel", didev_set_cooperative_level),
    ("IDirectInputDevice8::GetObjectInfo", hr_ok_4),
    ("IDirectInputDevice8::GetDeviceInfo", didev_get_device_info),
    ("IDirectInputDevice8::RunControlPanel", hr_ok_3),
    ("IDirectInputDevice8::Initialize", hr_ok_4),
    ("IDirectInputDevice8::CreateEffect", hr_ok_5),
    ("IDirectInputDevice8::EnumEffects", hr_ok_4),
    ("IDirectInputDevice8::GetEffectInfo", hr_ok_3),
    ("IDirectInputDevice8::GetForceFeedbackState", didev_get_ff_state),
    ("IDirectInputDevice8::SendForceFeedbackCommand", hr_ok_2),
    ("IDirectInputDevice8::EnumCreatedEffectObjects", hr_ok_4),
    ("IDirectInputDevice8::Escape", hr_ok_2),
    ("IDirectInputDevice8::Poll", didev_poll),
    ("IDirectInputDevice8::SendDeviceData", hr_ok_4),
    ("IDirectInputDevice8::EnumEffectsInFile", hr_ok_5),
    ("IDirectInputDevice8::WriteEffectToFile", hr_ok_5),
    ("IDirectInputDevice8::BuildActionMap", hr_ok_4),
    ("IDirectInputDevice8::SetActionMap", hr_ok_4),
    ("IDirectInputDevice8::GetImageInfo", hr_ok_2),
];

pub fn register(r: &mut WinApiRegistry) {
    r.add("dinput8.dll", "DirectInput8Create", direct_input8_create);
    r.add("dinput.dll", "DirectInputCreateA", direct_input8_create);
    r.add("dinput.dll", "DirectInputCreateW", direct_input8_create);
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
    ctx.ret_stdcall(S_OK, 5);
    Handled::Ok
}

fn di8_create_device(ctx: &mut ApiContext) -> Handled {
    // CreateDevice(this, rguid, lplpDirectInputDevice, pUnkOuter)
    let out_ptr = ctx.arg(2);
    if out_ptr != 0 {
        let obj = make_object_sized(ctx, IDIRECTINPUTDEVICE8, 0, DEV_OBJ_SIZE);
        let _ = ctx.memory.write_u32(obj + DEV_ACQUIRED, 0);
        let _ = ctx.memory.write_u32(out_ptr, obj);
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn di8_enum_devices(ctx: &mut ApiContext) -> Handled {
    // EnumDevices(this, dwDevType, callback, ref, flags) — no devices.
    ctx.ret_stdcall(DI_OK, 5);
    Handled::Ok
}

fn di8_get_device_status(ctx: &mut ApiContext) -> Handled {
    // DI_NOTATTACHED-ish: report attached so CreateDevice still works.
    ctx.ret_stdcall(DI_OK, 2);
    Handled::Ok
}

fn didev_get_capabilities(ctx: &mut ApiContext) -> Handled {
    // GetCapabilities(this, LPDIDEVCAPS)
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 44); // dwSize
        let _ = ctx.memory.write_u32(out + 4, 0x0000_0100); // DIDC_ATTACHED
        let _ = ctx.memory.write_u32(out + 8, 0x0000_0013); // DIDEVTYPE_KEYBOARD
        let _ = ctx.memory.write_u32(out + 12, 256); // dwAxes
        let _ = ctx.memory.write_u32(out + 16, 0);
        let _ = ctx.memory.write_u32(out + 20, 128); // dwButtons
        let _ = ctx.memory.write_u32(out + 24, 0);
        let _ = ctx.memory.write_u32(out + 28, 0);
        let _ = ctx.memory.write_u32(out + 32, 0);
        let _ = ctx.memory.write_u32(out + 36, 0);
        let _ = ctx.memory.write_u32(out + 40, 0);
    }
    ctx.ret_stdcall(DI_OK, 2);
    Handled::Ok
}

fn didev_acquire(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let _ = ctx.memory.write_u32(this + DEV_ACQUIRED, 1);
    ctx.ret_stdcall(DI_OK, 1);
    Handled::Ok
}

fn didev_unacquire(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let _ = ctx.memory.write_u32(this + DEV_ACQUIRED, 0);
    ctx.ret_stdcall(DI_OK, 1);
    Handled::Ok
}

fn didev_get_device_state(ctx: &mut ApiContext) -> Handled {
    // GetDeviceState(this, cbData, lpvData) — zero the buffer (no keys/axes).
    let this = ctx.arg(0);
    let cb = ctx.arg(1) as usize;
    let data = ctx.arg(2);
    let acquired = ctx.memory.read_u32(this + DEV_ACQUIRED).unwrap_or(0);
    if acquired == 0 {
        ctx.ret_stdcall(DIERR_NOTACQUIRED, 3);
        return Handled::Ok;
    }
    if data != 0 && cb > 0 {
        let zeros = vec![0u8; cb.min(512)];
        let _ = ctx.memory.write_bytes(data, &zeros);
    }
    ctx.ret_stdcall(DI_OK, 3);
    Handled::Ok
}

fn didev_get_device_data(ctx: &mut ApiContext) -> Handled {
    // GetDeviceData(this, cbObjectData, rgdod, pdwInOut, dwFlags) — 0 events.
    let this = ctx.arg(0);
    let count_ptr = ctx.arg(3);
    let acquired = ctx.memory.read_u32(this + DEV_ACQUIRED).unwrap_or(0);
    if count_ptr != 0 {
        let _ = ctx.memory.write_u32(count_ptr, 0);
    }
    if acquired == 0 {
        // Some games poll without acquire; still report empty rather than fatal.
        let _ = DIERR_INPUTLOST;
    }
    ctx.ret_stdcall(DI_OK, 5);
    Handled::Ok
}

fn didev_set_cooperative_level(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(DI_OK, 3);
    Handled::Ok
}

fn didev_get_device_info(ctx: &mut ApiContext) -> Handled {
    // GetDeviceInfo(this, LPDIDEVICEINSTANCE)
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 580); // dwSize of DIDEVICEINSTANCE
        // Leave GUIDs zeroed; product name at offset ~40 as wide string is optional.
        let name = "WebWINE Keyboard";
        // tszInstanceName is WCHAR[MAX_PATH] at offset 20 typically for A? For W
        // layout it's after two GUIDs (32) + dwDevType (4) = 36.
        let base = out + 36;
        for (i, u) in name.encode_utf16().enumerate() {
            let _ = ctx.memory.write_u16(base + i as u32 * 2, u);
        }
    }
    ctx.ret_stdcall(DI_OK, 2);
    Handled::Ok
}

fn didev_get_ff_state(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0);
    }
    ctx.ret_stdcall(DI_OK, 2);
    Handled::Ok
}

fn didev_poll(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(DI_OK, 1);
    Handled::Ok
}
