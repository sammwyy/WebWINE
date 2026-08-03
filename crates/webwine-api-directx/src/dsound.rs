//! DirectSound 8 (dsound.dll).
//!
//! Buffers allocate real guest scratch memory for Lock/Unlock. Playback is not
//! audible (browser audio is a separate path); Play/Stop/GetStatus track state
//! so games keep their audio threads happy.

use webwine_api::winapi::context::{ApiContext, Handled};
use webwine_api::winapi::WinApiRegistry;

use crate::{
    com_addref, com_qi, com_release, hr_ok_1, hr_ok_2, hr_ok_3, hr_ok_4, hr_ok_5, make_object_sized,
    register_vtable, Vtable,
};

const S_OK: u32 = 0;
const DSERR_INVALIDPARAM: u32 = 0x8878_000A;

// Buffer object layout: [vtable, size, scratch, play_cursor, write_cursor, status, volume, pan, freq]
const BUF_SIZE: u32 = 4;
const BUF_SCRATCH: u32 = 8;
const BUF_PLAY: u32 = 12;
const BUF_WRITE: u32 = 16;
const BUF_STATUS: u32 = 20;
const BUF_VOLUME: u32 = 24;
const BUF_PAN: u32 = 28;
const BUF_FREQ: u32 = 32;
const BUF_OBJ_SIZE: u32 = 36;

const DSBSTATUS_PLAYING: u32 = 0x0000_0001;
const DSBSTATUS_LOOPING: u32 = 0x0000_0004;

pub(crate) const IDIRECTSOUND8: Vtable = &[
    ("IDirectSound8::QueryInterface", com_qi),
    ("IDirectSound8::AddRef", com_addref),
    ("IDirectSound8::Release", com_release),
    ("IDirectSound8::CreateSoundBuffer", ds8_create_sound_buffer),
    ("IDirectSound8::GetCaps", ds8_get_caps),
    ("IDirectSound8::DuplicateSoundBuffer", ds8_duplicate_sound_buffer),
    ("IDirectSound8::SetCooperativeLevel", hr_ok_3),
    ("IDirectSound8::Compact", hr_ok_1),
    ("IDirectSound8::GetSpeakerConfig", ds8_get_speaker_config),
    ("IDirectSound8::SetSpeakerConfig", hr_ok_2),
    ("IDirectSound8::Initialize", hr_ok_2),
    ("IDirectSound8::VerifyCertification", ds8_verify_cert),
];

pub(crate) const IDIRECTSOUNDBUFFER8: Vtable = &[
    ("IDirectSoundBuffer8::QueryInterface", com_qi),
    ("IDirectSoundBuffer8::AddRef", com_addref),
    ("IDirectSoundBuffer8::Release", com_release),
    ("IDirectSoundBuffer8::GetCaps", dsb_get_caps),
    ("IDirectSoundBuffer8::GetCurrentPosition", dsb_get_current_position),
    ("IDirectSoundBuffer8::GetFormat", dsb_get_format),
    ("IDirectSoundBuffer8::GetVolume", dsb_get_volume),
    ("IDirectSoundBuffer8::GetPan", dsb_get_pan),
    ("IDirectSoundBuffer8::GetFrequency", dsb_get_frequency),
    ("IDirectSoundBuffer8::GetStatus", dsb_get_status),
    ("IDirectSoundBuffer8::Initialize", hr_ok_3),
    ("IDirectSoundBuffer8::Lock", dsb_lock),
    ("IDirectSoundBuffer8::Play", dsb_play),
    ("IDirectSoundBuffer8::SetCurrentPosition", dsb_set_current_position),
    ("IDirectSoundBuffer8::SetFormat", hr_ok_2),
    ("IDirectSoundBuffer8::SetVolume", dsb_set_volume),
    ("IDirectSoundBuffer8::SetPan", dsb_set_pan),
    ("IDirectSoundBuffer8::SetFrequency", dsb_set_frequency),
    ("IDirectSoundBuffer8::Stop", dsb_stop),
    ("IDirectSoundBuffer8::Unlock", hr_ok_5),
    ("IDirectSoundBuffer8::Restore", hr_ok_1),
    ("IDirectSoundBuffer8::GetObjectInPath", hr_ok_4),
    ("IDirectSoundBuffer8::SetFX", hr_ok_4),
    ("IDirectSoundBuffer8::AcquireResources", hr_ok_3),
];

pub fn register(r: &mut WinApiRegistry) {
    r.add("dsound.dll", "DirectSoundCreate8", direct_sound_create8);
    r.add("dsound.dll", "DirectSoundCreate", direct_sound_create8);
    r.add("dsound.dll", "#11", direct_sound_create8);
    r.add("dsound.dll", "DirectSoundEnumerateA", ds_enumerate);
    r.add("dsound.dll", "DirectSoundEnumerateW", ds_enumerate);
    register_vtable(r, IDIRECTSOUND8);
    register_vtable(r, IDIRECTSOUNDBUFFER8);
}

fn direct_sound_create8(ctx: &mut ApiContext) -> Handled {
    let out_ptr = ctx.arg(1);
    if out_ptr != 0 {
        let obj_va = make_object_sized(ctx, IDIRECTSOUND8, 0, 8);
        let _ = ctx.memory.write_u32(out_ptr, obj_va);
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn ds_enumerate(ctx: &mut ApiContext) -> Handled {
    // DirectSoundEnumerate(lpDSEnumCallback, lpContext) — report no devices via
    // not calling the callback (Wine still returns DS_OK).
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn ds8_get_caps(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 96); // dwSize
        let _ = ctx.memory.write_u32(out + 4, 0x0000_0001); // DSCAPS_PRIMARYMONO-ish
        let _ = ctx.memory.write_u32(out + 8, 2); // dwMinSecondarySampleRate
        let _ = ctx.memory.write_u32(out + 12, 100_000); // dwMaxSecondarySampleRate
        let _ = ctx.memory.write_u32(out + 16, 16); // dwPrimaryBuffers
        let _ = ctx.memory.write_u32(out + 20, 16);
        let _ = ctx.memory.write_u32(out + 24, 16);
        let _ = ctx.memory.write_u32(out + 28, 16 * 1024 * 1024); // free mem
        let _ = ctx.memory.write_u32(out + 32, 16 * 1024 * 1024);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn ds8_get_speaker_config(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0x0000_0004); // DSSPEAKER_STEREO
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn ds8_verify_cert(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn make_buffer(ctx: &mut ApiContext, byte_size: u32) -> u32 {
    let size = byte_size.max(4);
    let scratch = ctx.heap_alloc(size);
    let obj = make_object_sized(ctx, IDIRECTSOUNDBUFFER8, size, BUF_OBJ_SIZE);
    let _ = ctx.memory.write_u32(obj + BUF_SIZE, size);
    let _ = ctx.memory.write_u32(obj + BUF_SCRATCH, scratch);
    let _ = ctx.memory.write_u32(obj + BUF_PLAY, 0);
    let _ = ctx.memory.write_u32(obj + BUF_WRITE, 0);
    let _ = ctx.memory.write_u32(obj + BUF_STATUS, 0);
    let _ = ctx.memory.write_u32(obj + BUF_VOLUME, 0); // DSBVOLUME_MAX
    let _ = ctx.memory.write_u32(obj + BUF_PAN, 0);
    let _ = ctx.memory.write_u32(obj + BUF_FREQ, 44100);
    obj
}

fn ds8_create_sound_buffer(ctx: &mut ApiContext) -> Handled {
    // CreateSoundBuffer(this, pcDSBufferDesc, ppDSBuffer, pUnkOuter)
    let desc = ctx.arg(1);
    let out_ptr = ctx.arg(2);
    let mut size = 4096u32;
    if desc != 0 {
        // DSBUFFERDESC: dwSize(+0), dwFlags(+4), dwBufferBytes(+8)
        size = ctx.memory.read_u32(desc + 8).unwrap_or(4096).max(4);
    }
    if out_ptr == 0 {
        ctx.ret_stdcall(DSERR_INVALIDPARAM, 4);
        return Handled::Ok;
    }
    let obj = make_buffer(ctx, size);
    let _ = ctx.memory.write_u32(out_ptr, obj);
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn ds8_duplicate_sound_buffer(ctx: &mut ApiContext) -> Handled {
    let src = ctx.arg(1);
    let out_ptr = ctx.arg(2);
    let size = if src != 0 {
        ctx.memory.read_u32(src + BUF_SIZE).unwrap_or(4096)
    } else {
        4096
    };
    if out_ptr != 0 {
        let obj = make_buffer(ctx, size);
        let _ = ctx.memory.write_u32(out_ptr, obj);
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn dsb_get_caps(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(1);
    let size = ctx.memory.read_u32(this + BUF_SIZE).unwrap_or(0);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 20); // dwSize of DSBCAPS
        let _ = ctx.memory.write_u32(out + 4, 0x0000_0080); // DSBCAPS_CTRLVOLUME
        let _ = ctx.memory.write_u32(out + 8, size);
        let _ = ctx.memory.write_u32(out + 12, 0);
        let _ = ctx.memory.write_u32(out + 16, 0);
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dsb_get_current_position(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let play = ctx.arg(1);
    let write = ctx.arg(2);
    if play != 0 {
        let _ = ctx
            .memory
            .write_u32(play, ctx.memory.read_u32(this + BUF_PLAY).unwrap_or(0));
    }
    if write != 0 {
        let _ = ctx
            .memory
            .write_u32(write, ctx.memory.read_u32(this + BUF_WRITE).unwrap_or(0));
    }
    ctx.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn dsb_get_format(ctx: &mut ApiContext) -> Handled {
    // GetFormat(this, pwfxFormat, dwSizeAllocated, pdwSizeWritten)
    let out = ctx.arg(1);
    let alloc = ctx.arg(2);
    let written = ctx.arg(3);
    const WAVEFORMATEX_SIZE: u32 = 18;
    if written != 0 {
        let _ = ctx.memory.write_u32(written, WAVEFORMATEX_SIZE);
    }
    if out != 0 && alloc >= WAVEFORMATEX_SIZE {
        // PCM 44.1kHz stereo 16-bit
        let _ = ctx.memory.write_u16(out, 1); // wFormatTag WAVE_FORMAT_PCM
        let _ = ctx.memory.write_u16(out + 2, 2); // nChannels
        let _ = ctx.memory.write_u32(out + 4, 44100);
        let _ = ctx.memory.write_u32(out + 8, 44100 * 4);
        let _ = ctx.memory.write_u16(out + 12, 4); // nBlockAlign
        let _ = ctx.memory.write_u16(out + 14, 16); // wBitsPerSample
        let _ = ctx.memory.write_u16(out + 16, 0); // cbSize
    }
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn dsb_get_volume(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx
            .memory
            .write_u32(out, ctx.memory.read_u32(this + BUF_VOLUME).unwrap_or(0));
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dsb_get_pan(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx
            .memory
            .write_u32(out, ctx.memory.read_u32(this + BUF_PAN).unwrap_or(0));
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dsb_get_frequency(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx
            .memory
            .write_u32(out, ctx.memory.read_u32(this + BUF_FREQ).unwrap_or(44100));
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dsb_get_status(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx
            .memory
            .write_u32(out, ctx.memory.read_u32(this + BUF_STATUS).unwrap_or(0));
    }
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dsb_lock(ctx: &mut ApiContext) -> Handled {
    // Lock(this, dwOffset, dwBytes, ppv1, pdwBytes1, ppv2, pdwBytes2, flags)
    let this = ctx.arg(0);
    let offset = ctx.arg(1);
    let bytes = ctx.arg(2);
    let ptr1 = ctx.arg(3);
    let bytes1 = ctx.arg(4);
    let ptr2 = ctx.arg(5);
    let bytes2 = ctx.arg(6);
    let size = ctx.memory.read_u32(this + BUF_SIZE).unwrap_or(0);
    let scratch = ctx.memory.read_u32(this + BUF_SCRATCH).unwrap_or(0);
    let off = offset.min(size.saturating_sub(1));
    let avail = size.saturating_sub(off);
    let n = if bytes == 0 || bytes > avail {
        avail
    } else {
        bytes
    };
    if ptr1 != 0 {
        let _ = ctx.memory.write_u32(ptr1, scratch.wrapping_add(off));
    }
    if bytes1 != 0 {
        let _ = ctx.memory.write_u32(bytes1, n);
    }
    if ptr2 != 0 {
        let _ = ctx.memory.write_u32(ptr2, 0);
    }
    if bytes2 != 0 {
        let _ = ctx.memory.write_u32(bytes2, 0);
    }
    ctx.ret_stdcall(S_OK, 8);
    Handled::Ok
}

fn dsb_play(ctx: &mut ApiContext) -> Handled {
    // Play(this, reserved, priority, flags)
    let this = ctx.arg(0);
    let flags = ctx.arg(3);
    let mut status = DSBSTATUS_PLAYING;
    if flags & 1 != 0 {
        // DSBPLAY_LOOPING
        status |= DSBSTATUS_LOOPING;
    }
    let _ = ctx.memory.write_u32(this + BUF_STATUS, status);
    ctx.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn dsb_stop(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let _ = ctx.memory.write_u32(this + BUF_STATUS, 0);
    ctx.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn dsb_set_current_position(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let pos = ctx.arg(1);
    let _ = ctx.memory.write_u32(this + BUF_PLAY, pos);
    let _ = ctx.memory.write_u32(this + BUF_WRITE, pos);
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dsb_set_volume(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let _ = ctx.memory.write_u32(this + BUF_VOLUME, ctx.arg(1));
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dsb_set_pan(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let _ = ctx.memory.write_u32(this + BUF_PAN, ctx.arg(1));
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn dsb_set_frequency(ctx: &mut ApiContext) -> Handled {
    let this = ctx.arg(0);
    let _ = ctx.memory.write_u32(this + BUF_FREQ, ctx.arg(1));
    ctx.ret_stdcall(S_OK, 2);
    Handled::Ok
}
