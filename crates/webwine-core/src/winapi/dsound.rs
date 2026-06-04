use crate::winapi::context::{ApiContext, Handled};
use crate::winapi::WinApiRegistry;

// IDirectSound8
const IDIRECTSOUND8_METHODS: &[&str] = &[
    "IDirectSound8::QueryInterface", // 0
    "IDirectSound8::AddRef", // 1
    "IDirectSound8::Release", // 2
    "IDirectSound8::CreateSoundBuffer", // 3
    "IDirectSound8::GetCaps", // 4
    "IDirectSound8::DuplicateSoundBuffer", // 5
    "IDirectSound8::SetCooperativeLevel", // 6
    "IDirectSound8::Compact", // 7
    "IDirectSound8::GetSpeakerConfig", // 8
    "IDirectSound8::SetSpeakerConfig", // 9
    "IDirectSound8::Initialize", // 10
    "IDirectSound8::VerifyCertification", // 11
];

// IDirectSoundBuffer8
const IDIRECTSOUNDBUFFER8_METHODS: &[&str] = &[
    "IDirectSoundBuffer8::QueryInterface", // 0
    "IDirectSoundBuffer8::AddRef", // 1
    "IDirectSoundBuffer8::Release", // 2
    "IDirectSoundBuffer8::GetCaps", // 3
    "IDirectSoundBuffer8::GetCurrentPosition", // 4
    "IDirectSoundBuffer8::GetFormat", // 5
    "IDirectSoundBuffer8::GetVolume", // 6
    "IDirectSoundBuffer8::GetPan", // 7
    "IDirectSoundBuffer8::GetFrequency", // 8
    "IDirectSoundBuffer8::GetStatus", // 9
    "IDirectSoundBuffer8::Initialize", // 10
    "IDirectSoundBuffer8::Lock", // 11
    "IDirectSoundBuffer8::Play", // 12
    "IDirectSoundBuffer8::SetCurrentPosition", // 13
    "IDirectSoundBuffer8::SetFormat", // 14
    "IDirectSoundBuffer8::SetVolume", // 15
    "IDirectSoundBuffer8::SetPan", // 16
    "IDirectSoundBuffer8::SetFrequency", // 17
    "IDirectSoundBuffer8::Stop", // 18
    "IDirectSoundBuffer8::Unlock", // 19
    "IDirectSoundBuffer8::Restore", // 20
    "IDirectSoundBuffer8::GetObjectInPath", // 21
    "IDirectSoundBuffer8::SetFX", // 22
    "IDirectSoundBuffer8::AcquireResources", // 23
];

pub fn register(r: &mut WinApiRegistry) {
    // Both name and ordinal
    r.add("dsound.dll", "DirectSoundCreate8", direct_sound_create8);
    r.add("dsound.dll", "#11", direct_sound_create8);

    for name in IDIRECTSOUND8_METHODS {
        r.add("dsound.vtbl", name, dispatch_idirectsound8);
    }
    for name in IDIRECTSOUNDBUFFER8_METHODS {
        r.add("dsound.vtbl", name, dispatch_idirectsoundbuffer8);
    }
}

fn alloc_com_object(ctx: &mut ApiContext, method_names: &[&str], extra: u32) -> u32 {
    let vtable_size = (method_names.len() * 4) as u32;
    let vtable_va = ctx.heap_alloc(vtable_size);
    for (i, name) in method_names.iter().enumerate() {
        let tramp = ctx.api_resolve_trampoline("dsound.vtbl", name);
        let _ = ctx.memory.write_u32(vtable_va + i as u32 * 4, tramp);
    }
    let obj_va = ctx.heap_alloc(8);
    let _ = ctx.memory.write_u32(obj_va, vtable_va);
    let _ = ctx.memory.write_u32(obj_va + 4, extra);
    obj_va
}

fn direct_sound_create8(ctx: &mut ApiContext) -> Handled {
    // DirectSoundCreate8(pcGuidDevice, ppDS8, pUnkOuter)
    let out_ptr = ctx.arg(1);
    if out_ptr != 0 {
        let obj_va = alloc_com_object(ctx, IDIRECTSOUND8_METHODS, 0);
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(0, 3); // S_OK
    Handled::Ok
}

fn dispatch_idirectsound8(ctx: &mut ApiContext) -> Handled {
    let slot = IDIRECTSOUND8_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("dsound.vtbl", name) == ctx.current_trampoline_va())
        .unwrap_or(99);
    match slot {
        0 => { // QueryInterface
            let this = ctx.arg(0);
            let out = ctx.arg(2);
            if out != 0 { let _ = ctx.memory.write_u32(out, this); }
            ctx.ret_stdcall(0, 3);
        }
        1 | 2 => ctx.ret_stdcall(1, 1),
        3 => { // CreateSoundBuffer(this, pcDSBufferDesc, ppDSBuffer, pUnkOuter)
            let out_ptr = ctx.arg(2);
            if out_ptr != 0 {
                let obj_va = alloc_com_object(ctx, IDIRECTSOUNDBUFFER8_METHODS, 0);
                let _ = ctx.memory.write_u32(out_ptr, obj_va);
            }
            ctx.ret_stdcall(0, 4);
        }
        _ => {
            let nargs = match slot {
                4 => 2, 5 => 3, 6 => 3, 7 => 1, 8 => 2, 9 => 2, 10 => 2, 11 => 2,
                _ => 1,
            };
            ctx.ret_stdcall(0, nargs);
        }
    }
    Handled::Ok
}

fn dispatch_idirectsoundbuffer8(ctx: &mut ApiContext) -> Handled {
    let slot = IDIRECTSOUNDBUFFER8_METHODS
        .iter()
        .position(|name| ctx.api_trampoline_va("dsound.vtbl", name) == ctx.current_trampoline_va())
        .unwrap_or(99);
    match slot {
        0 => { // QueryInterface
            let this = ctx.arg(0);
            let out = ctx.arg(2);
            if out != 0 { let _ = ctx.memory.write_u32(out, this); }
            ctx.ret_stdcall(0, 3);
        }
        1 | 2 => ctx.ret_stdcall(1, 1),
        11 => { // Lock(this, dwOffset, dwBytes, ppvAudioPtr1, pdwAudioBytes1, ppvAudioPtr2, pdwAudioBytes2, dwFlags)
            let bytes = ctx.arg(2);
            let ptr1 = ctx.arg(3);
            let bytes1 = ctx.arg(4);
            let ptr2 = ctx.arg(5);
            let bytes2 = ctx.arg(6);
            let alloc_size = if bytes == 0 { 4096 } else { bytes };
            let buf_va = ctx.heap_alloc(alloc_size);
            if ptr1 != 0 { let _ = ctx.memory.write_u32(ptr1, buf_va); }
            if bytes1 != 0 { let _ = ctx.memory.write_u32(bytes1, bytes); }
            if ptr2 != 0 { let _ = ctx.memory.write_u32(ptr2, 0); }
            if bytes2 != 0 { let _ = ctx.memory.write_u32(bytes2, 0); }
            ctx.ret_stdcall(0, 8);
        }
        _ => {
            let nargs = match slot {
                3 => 2, 4 => 3, 5 => 3, 6 => 2, 7 => 2, 8 => 2, 9 => 2, 10 => 4,
                12 => 4, 13 => 2, 14 => 2, 15 => 2, 16 => 2, 17 => 2, 18 => 1,
                19 => 5, 20 => 1, 21 => 4, 22 => 4, 23 => 3,
                _ => 1,
            };
            ctx.ret_stdcall(0, nargs);
        }
    }
    Handled::Ok
}
