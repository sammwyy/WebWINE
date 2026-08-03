//! winmm.dll — multimedia timers, wave/MIDI device queries, joystick.

use std::sync::atomic::{AtomicU32, Ordering};

use super::{ApiContext, Handled, WinApiRegistry};

// Virtual millisecond clock shared with kernel32 timer paths.
static CLOCK_MS: AtomicU32 = AtomicU32::new(0);

/// Current virtual time in milliseconds, advancing one step per call.
pub fn tick_ms() -> u32 {
    CLOCK_MS.fetch_add(1, Ordering::Relaxed)
}

// MMRESULT / joystick errors
const MMSYSERR_NOERROR: u32 = 0;
const MMSYSERR_NODRIVER: u32 = 6;
const MMSYSERR_INVALPARAM: u32 = 11;
const JOYERR_UNPLUGGED: u32 = 167;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("winmm.dll", "timeGetTime", time_get_time),
        ("winmm.dll", "timeBeginPeriod", time_begin_period),
        ("winmm.dll", "timeEndPeriod", time_end_period),
        ("winmm.dll", "timeGetDevCaps", time_get_dev_caps),
        ("winmm.dll", "timeSetEvent", time_set_event),
        ("winmm.dll", "timeKillEvent", time_kill_event),
        ("winmm.dll", "timeGetSystemTime", time_get_system_time),
        ("winmm.dll", "waveOutGetNumDevs", wave_out_get_num_devs),
        ("winmm.dll", "waveOutOpen", wave_out_open),
        ("winmm.dll", "waveOutClose", wave_out_close),
        ("winmm.dll", "waveOutPrepareHeader", wave_out_prepare_header),
        ("winmm.dll", "waveOutUnprepareHeader", wave_out_unprepare_header),
        ("winmm.dll", "waveOutWrite", wave_out_write),
        ("winmm.dll", "waveOutReset", wave_out_reset),
        ("winmm.dll", "waveOutGetPosition", wave_out_get_position),
        ("winmm.dll", "waveOutSetVolume", wave_out_set_volume),
        ("winmm.dll", "waveOutGetVolume", wave_out_get_volume),
        ("winmm.dll", "midiOutGetNumDevs", midi_out_get_num_devs),
        ("winmm.dll", "midiOutOpen", midi_out_open),
        ("winmm.dll", "midiOutClose", midi_out_close),
        ("winmm.dll", "midiOutShortMsg", midi_out_short_msg),
        ("winmm.dll", "midiOutLongMsg", midi_out_long_msg),
        ("winmm.dll", "midiOutReset", midi_out_reset),
        ("winmm.dll", "midiOutPrepareHeader", midi_out_prepare_header),
        ("winmm.dll", "midiOutUnprepareHeader", midi_out_unprepare_header),
        ("winmm.dll", "midiOutSetVolume", midi_out_set_volume),
        ("winmm.dll", "midiStreamOpen", midi_stream_open),
        ("winmm.dll", "midiStreamClose", midi_stream_close),
        ("winmm.dll", "midiStreamOut", midi_stream_out),
        ("winmm.dll", "midiStreamProperty", midi_stream_property),
        ("winmm.dll", "midiStreamRestart", midi_stream_restart),
        ("winmm.dll", "midiStreamStop", midi_stream_stop),
        ("winmm.dll", "midiStreamPause", midi_stream_pause),
        ("winmm.dll", "mciSendCommandA", mci_send_command),
        ("winmm.dll", "mciSendCommandW", mci_send_command),
        ("winmm.dll", "mciSendStringA", mci_send_string),
        ("winmm.dll", "mciSendStringW", mci_send_string),
        ("winmm.dll", "PlaySoundA", play_sound),
        ("winmm.dll", "PlaySoundW", play_sound),
        ("winmm.dll", "sndPlaySoundA", snd_play_sound),
        ("winmm.dll", "sndPlaySoundW", snd_play_sound),
        ("winmm.dll", "joyGetNumDevs", joy_get_num_devs),
        ("winmm.dll", "joyGetPos", joy_get_pos),
        ("winmm.dll", "joyGetPosEx", joy_get_pos_ex),
        ("winmm.dll", "joyGetDevCapsA", joy_get_dev_caps),
        ("winmm.dll", "joyGetDevCapsW", joy_get_dev_caps),
        ("winmm.dll", "joyGetThreshold", joy_get_threshold),
        ("winmm.dll", "joySetThreshold", joy_set_threshold),
        ("winmm.dll", "joySetCapture", joy_set_capture),
        ("winmm.dll", "joyReleaseCapture", joy_release_capture),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn time_get_time(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(tick_ms(), 0);
    Handled::Ok
}

fn time_begin_period(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn time_end_period(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn time_get_dev_caps(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_u32(p, 1); // wPeriodMin
        let _ = ctx.memory.write_u32(p + 4, 1_000_000); // wPeriodMax
    }
    ctx.ret_stdcall(MMSYSERR_NOERROR, 2);
    Handled::Ok
}

fn time_set_event(c: &mut ApiContext) -> Handled {
    // UINT timeSetEvent(delay, resolution, callback, user, flags) → timer id
    let id = c
        .dll_state
        .entry("winmm.timer_id".into())
        .or_insert(1);
    let t = *id;
    *id = id.wrapping_add(1).max(1);
    c.ret_stdcall(t, 5);
    Handled::Ok
}

fn time_kill_event(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn time_get_system_time(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_u32(p, 0); // TIME_MS
        let _ = ctx.memory.write_u32(p + 4, tick_ms());
    }
    ctx.ret_stdcall(MMSYSERR_NOERROR, 2);
    Handled::Ok
}

fn wave_out_get_num_devs(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 0);
    Handled::Ok
}

fn wave_out_open(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NODRIVER, 6);
    Handled::Ok
}

fn wave_out_close(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn wave_out_prepare_header(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn wave_out_unprepare_header(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn wave_out_write(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn wave_out_reset(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn wave_out_get_position(c: &mut ApiContext) -> Handled {
    let p = c.arg(1);
    if p != 0 {
        let _ = c.memory.write_u32(p, 0); // wType
        let _ = c.memory.write_u32(p + 4, 0); // sample/ms
    }
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn wave_out_set_volume(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 2);
    Handled::Ok
}

fn wave_out_get_volume(c: &mut ApiContext) -> Handled {
    let out = c.arg(1);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0xFFFF_FFFF);
    }
    c.ret_stdcall(MMSYSERR_NOERROR, 2);
    Handled::Ok
}

fn midi_out_get_num_devs(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 0);
    Handled::Ok
}

fn midi_out_open(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NODRIVER, 5);
    Handled::Ok
}

fn midi_out_close(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn midi_out_short_msg(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 2);
    Handled::Ok
}

fn midi_out_long_msg(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn midi_out_reset(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn midi_out_prepare_header(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn midi_out_unprepare_header(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn midi_out_set_volume(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 2);
    Handled::Ok
}

fn midi_stream_open(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NODRIVER, 6);
    Handled::Ok
}

fn midi_stream_close(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn midi_stream_out(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn midi_stream_property(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 3);
    Handled::Ok
}

fn midi_stream_restart(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn midi_stream_stop(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn midi_stream_pause(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

fn mci_send_command(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 4); // MCIERR success = 0
    Handled::Ok
}

fn mci_send_string(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 4);
    Handled::Ok
}

fn play_sound(c: &mut ApiContext) -> Handled {
    // PlaySound returns TRUE even when silent (Wine often does for missing files).
    c.ret_stdcall(1, 3);
    Handled::Ok
}

fn snd_play_sound(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 2);
    Handled::Ok
}

fn joy_get_num_devs(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 0);
    Handled::Ok
}

fn joy_get_pos(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(JOYERR_UNPLUGGED, 2);
    Handled::Ok
}

fn joy_get_pos_ex(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(JOYERR_UNPLUGGED, 2);
    Handled::Ok
}

fn joy_get_dev_caps(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NODRIVER, 3);
    Handled::Ok
}

fn joy_get_threshold(c: &mut ApiContext) -> Handled {
    let out = c.arg(1);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(MMSYSERR_NOERROR, 2);
    Handled::Ok
}

fn joy_set_threshold(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 2);
    Handled::Ok
}

fn joy_set_capture(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 4);
    Handled::Ok
}

fn joy_release_capture(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MMSYSERR_NOERROR, 1);
    Handled::Ok
}

#[allow(dead_code)]
fn _inval() -> u32 {
    MMSYSERR_INVALPARAM
}
