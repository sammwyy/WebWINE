use crate::winapi::context::{ApiContext, Handled};
use crate::winapi::WinApiRegistry;

const IDIRECTINPUT8_METHODS: &[&str] = &[
    "IDirectInput8::QueryInterface", // 0
    "IDirectInput8::AddRef", // 1
    "IDirectInput8::Release", // 2
    "IDirectInput8::CreateDevice", // 3
    "IDirectInput8::EnumDevices", // 4
    "IDirectInput8::GetDeviceStatus", // 5
    "IDirectInput8::RunControlPanel", // 6
    "IDirectInput8::Initialize", // 7
    "IDirectInput8::FindDevice", // 8
    "IDirectInput8::EnumDevicesBySemantics", // 9
    "IDirectInput8::ConfigureDevices", // 10
];

const IDIRECTINPUTDEVICE8_METHODS: &[&str] = &[
    "IDirectInputDevice8::QueryInterface", // 0
    "IDirectInputDevice8::AddRef", // 1
    "IDirectInputDevice8::Release", // 2
    "IDirectInputDevice8::GetCapabilities", // 3
    "IDirectInputDevice8::EnumObjects", // 4
    "IDirectInputDevice8::GetProperty", // 5
    "IDirectInputDevice8::SetProperty", // 6
    "IDirectInputDevice8::Acquire", // 7
    "IDirectInputDevice8::Unacquire", // 8
    "IDirectInputDevice8::GetDeviceState", // 9
    "IDirectInputDevice8::GetDeviceData", // 10
    "IDirectInputDevice8::SetDataFormat", // 11
    "IDirectInputDevice8::SetEventNotification", // 12
    "IDirectInputDevice8::SetCooperativeLevel", // 13
    "IDirectInputDevice8::GetObjectInfo", // 14
    "IDirectInputDevice8::GetDeviceInfo", // 15
    "IDirectInputDevice8::RunControlPanel", // 16
    "IDirectInputDevice8::Initialize", // 17
    "IDirectInputDevice8::CreateEffect", // 18
    "IDirectInputDevice8::EnumEffects", // 19
    "IDirectInputDevice8::GetEffectInfo", // 20
    "IDirectInputDevice8::GetForceFeedbackState", // 21
    "IDirectInputDevice8::SendForceFeedbackCommand", // 22
    "IDirectInputDevice8::EnumCreatedEffectObjects", // 23
    "IDirectInputDevice8::Escape", // 24
    "IDirectInputDevice8::Poll", // 25
    "IDirectInputDevice8::SendDeviceData", // 26
    "IDirectInputDevice8::EnumEffectsInFile", // 27
    "IDirectInputDevice8::WriteEffectToFile", // 28
    "IDirectInputDevice8::BuildActionMap", // 29
    "IDirectInputDevice8::SetActionMap", // 30
    "IDirectInputDevice8::GetImageInfo", // 31
];

pub fn register(r: &mut WinApiRegistry) {
    r.add("dinput8.dll", "DirectInput8Create", direct_input8_create);

    for name in IDIRECTINPUT8_METHODS {
        r.add("dinput.vtbl", name, dispatch_idirectinput8);
    }
    for name in IDIRECTINPUTDEVICE8_METHODS {
        r.add("dinput.vtbl", name, dispatch_idirectinputdevice8);
    }
}

fn alloc_com_object(ctx: &mut ApiContext, method_names: &[&str], extra: u32) -> u32 {
    let vtable_size = (method_names.len() * 4) as u32;
    let vtable_va = ctx.heap_alloc(vtable_size);
    for (i, name) in method_names.iter().enumerate() {
        let tramp = ctx.api_resolve_trampoline("dinput.vtbl", name);
        let _ = ctx.memory.write_u32(vtable_va + i as u32 * 4, tramp);
    }
    let obj_va = ctx.heap_alloc(8);
    let _ = ctx.memory.write_u32(obj_va, vtable_va);
    let _ = ctx.memory.write_u32(obj_va + 4, extra);
    obj_va
}

fn direct_input8_create(ctx: &mut ApiContext) -> Handled {
    // DirectInput8Create(hinst, dwVersion, riidltf, ppvOut, punkOuter)
    let out_ptr = ctx.arg(3);
    if out_ptr != 0 {
        let obj_va = alloc_com_object(ctx, IDIRECTINPUT8_METHODS, 0);
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(0, 5); // S_OK
    Handled::Ok
}

fn dispatch_idirectinput8(ctx: &mut ApiContext) -> Handled {
    let slot = IDIRECTINPUT8_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("dinput.vtbl", name) == ctx.current_trampoline_va())
        .unwrap_or(99);
    match slot {
        0 => { // QueryInterface
            let this = ctx.arg(0);
            let out = ctx.arg(2);
            if out != 0 { let _ = ctx.memory.write_u32(out, this); }
            ctx.ret_stdcall(0, 3);
        }
        1 | 2 => ctx.ret_stdcall(1, 1),
        3 => { // CreateDevice(this, rguid, lplpDirectInputDevice, pUnkOuter)
            let out_ptr = ctx.arg(2);
            if out_ptr != 0 {
                let obj_va = alloc_com_object(ctx, IDIRECTINPUTDEVICE8_METHODS, 0);
                let _ = ctx.memory.write_u32(out_ptr, obj_va);
            }
            ctx.ret_stdcall(0, 4);
        }
        _ => {
            let nargs = match slot {
                4 => 4, 5 => 2, 6 => 3, 7 => 3, 8 => 4, 9 => 6, 10 => 4,
                _ => 1,
            };
            ctx.ret_stdcall(0, nargs);
        }
    }
    Handled::Ok
}

fn dispatch_idirectinputdevice8(ctx: &mut ApiContext) -> Handled {
    let slot = IDIRECTINPUTDEVICE8_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("dinput.vtbl", name) == ctx.current_trampoline_va())
        .unwrap_or(99);
    match slot {
        0 => { // QueryInterface
            let this = ctx.arg(0);
            let out = ctx.arg(2);
            if out != 0 { let _ = ctx.memory.write_u32(out, this); }
            ctx.ret_stdcall(0, 3);
        }
        1 | 2 => ctx.ret_stdcall(1, 1),
        10 => { // GetDeviceData(this, cbObjectData, rgdod, pdwInOut, dwFlags)
            // Just say no data available
            let count_ptr = ctx.arg(3);
            if count_ptr != 0 {
                let _ = ctx.memory.write_u32(count_ptr, 0);
            }
            ctx.ret_stdcall(0, 5);
        }
        _ => {
            let nargs = match slot {
                3 => 2, 4 => 4, 5 => 2, 6 => 2, 7 => 1, 8 => 1, 9 => 3,
                11 => 2, 12 => 2, 13 => 3, 14 => 4, 15 => 2, 16 => 3, 17 => 4,
                18 => 5, 19 => 4, 20 => 3, 21 => 2, 22 => 2, 23 => 3, 24 => 2,
                25 => 1, 26 => 4, 27 => 5, 28 => 4, 29 => 4, 30 => 3, 31 => 2,
                _ => 1,
            };
            ctx.ret_stdcall(0, nargs);
        }
    }
    Handled::Ok
}
