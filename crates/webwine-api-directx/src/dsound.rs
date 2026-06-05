//! DirectSound 8 stub (dsound.dll). Buffers are accepted and Lock hands back a
//! scratch guest buffer to write into; nothing is actually played. COM mechanism:
//! see the crate root.

use webwine_api::winapi::context::{ApiContext, Handled};
use webwine_api::winapi::WinApiRegistry;

use crate::{com_qi, make_object, register_vtable, s0_1, s0_2, s0_3, s0_4, s0_5, s1_1, Vtable};

pub(crate) const IDIRECTSOUND8: Vtable = &[
    ("IDirectSound8::QueryInterface", com_qi),
    ("IDirectSound8::AddRef", s1_1),
    ("IDirectSound8::Release", s1_1),
    ("IDirectSound8::CreateSoundBuffer", ds8_create_sound_buffer),
    ("IDirectSound8::GetCaps", s0_2),
    ("IDirectSound8::DuplicateSoundBuffer", ds8_duplicate_sound_buffer),
    ("IDirectSound8::SetCooperativeLevel", s0_3),
    ("IDirectSound8::Compact", s0_1),
    ("IDirectSound8::GetSpeakerConfig", s0_2),
    ("IDirectSound8::SetSpeakerConfig", s0_2),
    ("IDirectSound8::Initialize", s0_2),
    ("IDirectSound8::VerifyCertification", s0_2),
];

pub(crate) const IDIRECTSOUNDBUFFER8: Vtable = &[
    ("IDirectSoundBuffer8::QueryInterface", com_qi),
    ("IDirectSoundBuffer8::AddRef", s1_1),
    ("IDirectSoundBuffer8::Release", s1_1),
    ("IDirectSoundBuffer8::GetCaps", s0_2),
    ("IDirectSoundBuffer8::GetCurrentPosition", s0_3),
    ("IDirectSoundBuffer8::GetFormat", s0_3),
    ("IDirectSoundBuffer8::GetVolume", s0_2),
    ("IDirectSoundBuffer8::GetPan", s0_2),
    ("IDirectSoundBuffer8::GetFrequency", s0_2),
    ("IDirectSoundBuffer8::GetStatus", s0_2),
    ("IDirectSoundBuffer8::Initialize", s0_4),
    ("IDirectSoundBuffer8::Lock", dsbuffer8_lock),
    ("IDirectSoundBuffer8::Play", s0_4),
    ("IDirectSoundBuffer8::SetCurrentPosition", s0_2),
    ("IDirectSoundBuffer8::SetFormat", s0_2),
    ("IDirectSoundBuffer8::SetVolume", s0_2),
    ("IDirectSoundBuffer8::SetPan", s0_2),
    ("IDirectSoundBuffer8::SetFrequency", s0_2),
    ("IDirectSoundBuffer8::Stop", s0_1),
    ("IDirectSoundBuffer8::Unlock", s0_5),
    ("IDirectSoundBuffer8::Restore", s0_1),
    ("IDirectSoundBuffer8::GetObjectInPath", s0_4),
    ("IDirectSoundBuffer8::SetFX", s0_4),
    ("IDirectSoundBuffer8::AcquireResources", s0_3),
];

pub fn register(r: &mut WinApiRegistry) {
    r.add("dsound.dll", "DirectSoundCreate8", direct_sound_create8);
    r.add("dsound.dll", "#11", direct_sound_create8);
    register_vtable(r, IDIRECTSOUND8);
    register_vtable(r, IDIRECTSOUNDBUFFER8);
}

fn direct_sound_create8(ctx: &mut ApiContext) -> Handled {
    // DirectSoundCreate8(pcGuidDevice, ppDS8, pUnkOuter)
    let out_ptr = ctx.arg(1);
    if out_ptr != 0 {
        let obj_va = make_object(ctx, IDIRECTSOUND8, 0);
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(0, 3); // S_OK
    Handled::Ok
}

fn ds8_duplicate_sound_buffer(ctx: &mut ApiContext) -> Handled {
    // DuplicateSoundBuffer(this, pDSBufferOriginal, ppDSBufferDuplicate) — hand
    // back a fresh buffer object so the guest doesn't deref a NULL duplicate.
    let out_ptr = ctx.arg(2);
    if out_ptr != 0 {
        let obj_va = make_object(ctx, IDIRECTSOUNDBUFFER8, 0);
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(0, 3);
    Handled::Ok
}

fn ds8_create_sound_buffer(ctx: &mut ApiContext) -> Handled {
    // CreateSoundBuffer(this, pcDSBufferDesc, ppDSBuffer, pUnkOuter)
    let out_ptr = ctx.arg(2);
    if out_ptr != 0 {
        let obj_va = make_object(ctx, IDIRECTSOUNDBUFFER8, 0);
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(0, 4);
    Handled::Ok
}

fn dsbuffer8_lock(ctx: &mut ApiContext) -> Handled {
    // Lock(this, dwOffset, dwBytes, ppvAudioPtr1, pdwAudioBytes1,
    //      ppvAudioPtr2, pdwAudioBytes2, dwFlags)
    let bytes = ctx.arg(2);
    let ptr1 = ctx.arg(3);
    let bytes1 = ctx.arg(4);
    let ptr2 = ctx.arg(5);
    let bytes2 = ctx.arg(6);
    let alloc_size = if bytes == 0 { 4096 } else { bytes };
    let buf_va = ctx.heap_alloc(alloc_size);
    if ptr1 != 0 {
        let _ = ctx.memory.write_u32(ptr1, buf_va);
    }
    if bytes1 != 0 {
        let _ = ctx.memory.write_u32(bytes1, bytes);
    }
    if ptr2 != 0 {
        let _ = ctx.memory.write_u32(ptr2, 0);
    }
    if bytes2 != 0 {
        let _ = ctx.memory.write_u32(bytes2, 0);
    }
    ctx.ret_stdcall(0, 8);
    Handled::Ok
}
