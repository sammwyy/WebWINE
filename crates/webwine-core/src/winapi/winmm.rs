use std::sync::atomic::{AtomicU32, Ordering};

use super::{ApiContext, Handled, WinApiRegistry};

// A monotonic millisecond clock shared by the WINMM and kernel32 timers. We have
// no real wall clock under WASM, so each query advances time by a small step.
// That keeps game loops of the form `while (timeGetTime() < target) {}` making
// progress instead of spinning forever.
static CLOCK_MS: AtomicU32 = AtomicU32::new(0);

/// Current virtual time in milliseconds, advancing one step per call.
pub fn tick_ms() -> u32 {
    CLOCK_MS.fetch_add(1, Ordering::Relaxed)
}

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("winmm.dll", "timeGetTime", time_get_time),         // 0 args -> DWORD ms
        ("winmm.dll", "timeBeginPeriod", ok_1),              // 1 arg  -> MMRESULT 0
        ("winmm.dll", "timeEndPeriod", ok_1),                // 1 arg
        ("winmm.dll", "timeGetDevCaps", time_get_dev_caps),  // 2 args
        ("winmm.dll", "timeSetEvent", ok_5),                 // returns a timer id
        ("winmm.dll", "timeKillEvent", ok_1),
        ("winmm.dll", "timeGetSystemTime", time_get_system_time), // 2 args

        // Audio/MIDI: we have no synthesizer yet, so report "no devices / success"
        // with the correct stdcall arg counts so the guest stack stays balanced.
        ("winmm.dll", "waveOutGetNumDevs", zero_0),
        ("winmm.dll", "waveOutOpen", fail_6),
        ("winmm.dll", "waveOutClose", ok_1),
        ("winmm.dll", "waveOutPrepareHeader", ok_3),
        ("winmm.dll", "waveOutUnprepareHeader", ok_3),
        ("winmm.dll", "waveOutWrite", ok_3),
        ("winmm.dll", "waveOutReset", ok_1),
        ("winmm.dll", "waveOutGetPosition", ok_3),
        ("winmm.dll", "waveOutSetVolume", ok_2),
        ("winmm.dll", "midiOutGetNumDevs", zero_0),
        ("winmm.dll", "midiOutOpen", fail_5),
        ("winmm.dll", "midiOutClose", ok_1),
        ("winmm.dll", "midiOutShortMsg", ok_2),
        ("winmm.dll", "midiOutLongMsg", ok_3),
        ("winmm.dll", "midiOutReset", ok_1),
        ("winmm.dll", "midiOutPrepareHeader", ok_3),
        ("winmm.dll", "midiOutUnprepareHeader", ok_3),
        ("winmm.dll", "midiOutSetVolume", ok_2),
        ("winmm.dll", "midiStreamOpen", fail_6),
        ("winmm.dll", "midiStreamClose", ok_1),
        ("winmm.dll", "midiStreamOut", ok_3),
        ("winmm.dll", "midiStreamProperty", ok_3),
        ("winmm.dll", "midiStreamRestart", ok_1),
        ("winmm.dll", "midiStreamStop", ok_1),
        ("winmm.dll", "midiStreamPause", ok_1),
        ("winmm.dll", "mciSendCommandA", zero_4),
        ("winmm.dll", "mciSendStringA", zero_4),
        ("winmm.dll", "PlaySoundA", ok_3),
        ("winmm.dll", "PlaySoundW", ok_3),
        ("winmm.dll", "sndPlaySoundA", ok_2),

        // Joystick: report "no joystick connected". Correct stdcall arg counts
        // matter here -- the default fallback guessed 1 arg for joyGetPosEx (2)
        // and joyGetDevCapsA (3), under-popping the stack and corrupting the
        // return address (Touhou 6 crashed with a null deref right after).
        ("winmm.dll", "joyGetNumDevs", zero_0),       // 0 args -> 0 devices
        ("winmm.dll", "joyGetPos", joy_unplugged_2),  // (uJoyID, LPJOYINFO)
        ("winmm.dll", "joyGetPosEx", joy_unplugged_2), // (uJoyID, LPJOYINFOEX)
        ("winmm.dll", "joyGetDevCapsA", joy_nodriver_3), // (uJoyID, LPJOYCAPS, cb)
        ("winmm.dll", "joyGetDevCapsW", joy_nodriver_3),
        ("winmm.dll", "joyGetThreshold", ok_2),       // (uJoyID, LPDWORD)
        ("winmm.dll", "joySetThreshold", ok_2),
        ("winmm.dll", "joySetCapture", ok_4),
        ("winmm.dll", "joyReleaseCapture", ok_1),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn time_get_time(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(tick_ms(), 0);
    Handled::Ok
}

// timeGetDevCaps(LPTIMECAPS, cbtc): fill {wPeriodMin=1, wPeriodMax=1000000}.
fn time_get_dev_caps(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_u32(p, 1);
        let _ = ctx.memory.write_u32(p + 4, 1_000_000);
    }
    ctx.ret_stdcall(0, 2);
    Handled::Ok
}

// timeGetSystemTime(LPMMTIME, cbmmt): MMTIME.wType=TIME_MS(0), u.ms=tick.
fn time_get_system_time(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_u32(p, 0);
        let _ = ctx.memory.write_u32(p + 4, tick_ms());
    }
    ctx.ret_stdcall(0, 2);
    Handled::Ok
}

fn zero_0(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 0); Handled::Ok }
fn ok_1(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 1); Handled::Ok }
fn ok_4(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 4); Handled::Ok }
// JOYERR_UNPLUGGED (167): no joystick at this id.
fn joy_unplugged_2(c: &mut ApiContext) -> Handled { c.ret_stdcall(167, 2); Handled::Ok }
// MMSYSERR_NODRIVER (6): no joystick driver.
fn joy_nodriver_3(c: &mut ApiContext) -> Handled { c.ret_stdcall(6, 3); Handled::Ok }
fn ok_2(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 2); Handled::Ok }
fn ok_3(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 3); Handled::Ok }
fn ok_5(c: &mut ApiContext) -> Handled { c.ret_stdcall(1, 5); Handled::Ok }
fn zero_4(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 4); Handled::Ok }
// MMSYSERR_NODRIVER (6): no audio device available.
fn fail_5(c: &mut ApiContext) -> Handled { c.ret_stdcall(6, 5); Handled::Ok }
fn fail_6(c: &mut ApiContext) -> Handled { c.ret_stdcall(6, 6); Handled::Ok }
