//! imm32.dll — Input Method Editor (IME). WebWINE has no IME; APIs report a
//! closed, non-IME environment with correct stdcall layouts (Wine imm32).

use super::{ApiContext, Handled, WinApiRegistry};

const HIMC_DEFAULT: u32 = 0x0F00_0010;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, super::HandlerFn)] = &[
        ("ImmGetContext", imm_get_context),
        ("ImmReleaseContext", imm_release_context),
        ("ImmCreateContext", imm_create_context),
        ("ImmDestroyContext", imm_destroy_context),
        ("ImmAssociateContext", imm_associate_context),
        ("ImmAssociateContextEx", imm_associate_context_ex),
        ("ImmGetOpenStatus", imm_get_open_status),
        ("ImmSetOpenStatus", imm_set_open_status),
        ("ImmGetConversionStatus", imm_get_conversion_status),
        ("ImmSetConversionStatus", imm_set_conversion_status),
        ("ImmGetCompositionStringA", imm_get_composition_string),
        ("ImmGetCompositionStringW", imm_get_composition_string),
        ("ImmSetCompositionStringA", imm_set_composition_string),
        ("ImmSetCompositionStringW", imm_set_composition_string),
        ("ImmNotifyIME", imm_notify_ime),
        ("ImmGetCandidateWindow", imm_get_candidate_window),
        ("ImmSetCandidateWindow", imm_set_candidate_window),
        ("ImmGetCompositionWindow", imm_get_composition_window),
        ("ImmSetCompositionWindow", imm_set_composition_window),
        ("ImmGetStatusWindowPos", imm_get_status_window_pos),
        ("ImmSetStatusWindowPos", imm_set_status_window_pos),
        ("ImmGetDefaultIMEWnd", imm_get_default_ime_wnd),
        ("ImmIsIME", imm_is_ime),
        ("ImmGetProperty", imm_get_property),
        ("ImmGetVirtualKey", imm_get_virtual_key),
        ("ImmDisableIME", imm_disable_ime),
        ("ImmEscapeA", imm_escape),
        ("ImmEscapeW", imm_escape),
        ("ImmGetDescriptionA", |c| imm_get_description(c, false)),
        ("ImmGetDescriptionW", |c| imm_get_description(c, true)),
        ("ImmGetIMEFileNameA", |c| imm_get_ime_file_name(c, false)),
        ("ImmGetIMEFileNameW", |c| imm_get_ime_file_name(c, true)),
        ("ImmRegisterWordA", imm_register_word),
        ("ImmRegisterWordW", imm_register_word),
        ("ImmUnregisterWordA", imm_unregister_word),
        ("ImmUnregisterWordW", imm_unregister_word),
        ("ImmEnumRegisterWordA", imm_enum_register_word),
        ("ImmEnumRegisterWordW", imm_enum_register_word),
    ];
    for &(name, handler) in fns {
        r.add("imm32.dll", name, handler);
    }
}

fn imm_get_context(c: &mut ApiContext) -> Handled {
    // HIMC ImmGetContext(hwnd) — return a stable default context.
    c.ret_stdcall(HIMC_DEFAULT, 1);
    Handled::Ok
}

fn imm_release_context(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 2); // TRUE
    Handled::Ok
}

fn imm_create_context(c: &mut ApiContext) -> Handled {
    let h = c
        .dll_state
        .entry("imm32.next".into())
        .or_insert(HIMC_DEFAULT + 1);
    let himc = *h;
    *h = h.wrapping_add(1);
    c.dll_state.insert(format!("imm32.ctx.{himc}"), 0); // closed
    c.ret_stdcall(himc, 0);
    Handled::Ok
}

fn imm_destroy_context(c: &mut ApiContext) -> Handled {
    let h = c.arg(0);
    c.dll_state.remove(&format!("imm32.ctx.{h}"));
    c.ret_stdcall(1, 1);
    Handled::Ok
}

fn imm_associate_context(c: &mut ApiContext) -> Handled {
    // Returns previous HIMC (0 = none).
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn imm_associate_context_ex(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 3);
    Handled::Ok
}

fn imm_get_open_status(c: &mut ApiContext) -> Handled {
    let h = c.arg(0);
    let open = c
        .dll_state
        .get(&format!("imm32.ctx.{h}"))
        .copied()
        .unwrap_or(0);
    c.ret_stdcall(open, 1);
    Handled::Ok
}

fn imm_set_open_status(c: &mut ApiContext) -> Handled {
    let h = c.arg(0);
    let open = c.arg(1);
    c.dll_state.insert(format!("imm32.ctx.{h}"), open);
    c.ret_stdcall(1, 2);
    Handled::Ok
}

fn imm_get_conversion_status(c: &mut ApiContext) -> Handled {
    if c.arg(1) != 0 {
        let _ = c.memory.write_u32(c.arg(1), 0);
    }
    if c.arg(2) != 0 {
        let _ = c.memory.write_u32(c.arg(2), 0);
    }
    c.ret_stdcall(1, 3);
    Handled::Ok
}

fn imm_set_conversion_status(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 3);
    Handled::Ok
}

fn imm_get_composition_string(c: &mut ApiContext) -> Handled {
    // LONG ImmGetCompositionString(himc, index, buf, buflen)
    // No composition → 0 bytes / IMM_ERROR_NODATA (-1) for some indices.
    let buf = c.arg(2);
    let len = c.arg(3);
    if buf != 0 && len > 0 {
        let _ = c.memory.write_u8(buf, 0);
    }
    c.ret_stdcall(0, 4);
    Handled::Ok
}

fn imm_set_composition_string(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 6);
    Handled::Ok
}

fn imm_notify_ime(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 4);
    Handled::Ok
}

fn imm_get_candidate_window(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn imm_set_candidate_window(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 2);
    Handled::Ok
}

fn imm_get_composition_window(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn imm_set_composition_window(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 2);
    Handled::Ok
}

fn imm_get_status_window_pos(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn imm_set_status_window_pos(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 2);
    Handled::Ok
}

fn imm_get_default_ime_wnd(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1); // no IME window
    Handled::Ok
}

fn imm_is_ime(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1); // HKL is not an IME
    Handled::Ok
}

fn imm_get_property(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn imm_get_virtual_key(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn imm_disable_ime(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}

fn imm_escape(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn imm_get_description(c: &mut ApiContext, wide: bool) -> Handled {
    // UINT ImmGetDescription(hkl, lpsz, uBufLen)
    let buf = c.arg(1);
    let cch = c.arg(2) as usize;
    let name = "WebWINE";
    if buf == 0 || cch == 0 {
        c.ret_stdcall(name.len() as u32, 3);
        return Handled::Ok;
    }
    if wide {
        let units: Vec<u16> = name.encode_utf16().take(cch.saturating_sub(1)).collect();
        for (i, u) in units.iter().enumerate() {
            let _ = c.memory.write_u16(buf + i as u32 * 2, *u);
        }
        let _ = c.memory.write_u16(buf + units.len() as u32 * 2, 0);
        c.ret_stdcall(units.len() as u32, 3);
    } else {
        let n = name.len().min(cch.saturating_sub(1));
        let _ = c.memory.write_bytes(buf, name.as_bytes()[..n].as_ref());
        let _ = c.memory.write_u8(buf + n as u32, 0);
        c.ret_stdcall(n as u32, 3);
    }
    Handled::Ok
}

fn imm_get_ime_file_name(c: &mut ApiContext, wide: bool) -> Handled {
    let buf = c.arg(1);
    let cch = c.arg(2) as usize;
    if buf != 0 && cch > 0 {
        if wide {
            let _ = c.memory.write_u16(buf, 0);
        } else {
            let _ = c.memory.write_u8(buf, 0);
        }
    }
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn imm_register_word(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 4);
    Handled::Ok
}

fn imm_unregister_word(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 4);
    Handled::Ok
}

fn imm_enum_register_word(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 6);
    Handled::Ok
}
