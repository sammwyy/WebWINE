use super::{ApiContext, Handled, WinApiRegistry};

const HIMC: u32 = 0x0F00_0010;

fn ret0_1(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 1); Handled::Ok }
fn ret0_2(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 2); Handled::Ok }
fn ret0_3(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 3); Handled::Ok }
fn ret0_4(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 4); Handled::Ok }
fn ret0_6(c: &mut ApiContext) -> Handled { c.ret_stdcall(0, 6); Handled::Ok }
fn ret1_1(c: &mut ApiContext) -> Handled { c.ret_stdcall(1, 1); Handled::Ok }
fn ret1_2(c: &mut ApiContext) -> Handled { c.ret_stdcall(1, 2); Handled::Ok }
fn ret1_3(c: &mut ApiContext) -> Handled { c.ret_stdcall(1, 3); Handled::Ok }
fn ret1_4(c: &mut ApiContext) -> Handled { c.ret_stdcall(1, 4); Handled::Ok }

fn get_context(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(HIMC, 1);
    Handled::Ok
}

fn get_conversion_status(c: &mut ApiContext) -> Handled {
    if c.arg(1) != 0 { let _ = c.memory.write_u32(c.arg(1), 0); }
    if c.arg(2) != 0 { let _ = c.memory.write_u32(c.arg(2), 0); }
    c.ret_stdcall(1, 3);
    Handled::Ok
}

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, super::HandlerFn)] = &[
        ("ImmGetContext", get_context),
        ("ImmReleaseContext", ret1_2),
        ("ImmCreateContext", |c| { c.ret_stdcall(HIMC, 0); Handled::Ok }),
        ("ImmDestroyContext", ret1_1),
        ("ImmAssociateContext", ret0_2),
        ("ImmAssociateContextEx", ret1_3),
        ("ImmGetOpenStatus", ret0_1),
        ("ImmSetOpenStatus", ret1_2),
        ("ImmGetConversionStatus", get_conversion_status),
        ("ImmSetConversionStatus", ret1_3),
        ("ImmGetCompositionStringA", ret0_4),
        ("ImmGetCompositionStringW", ret0_4),
        ("ImmSetCompositionStringA", ret1_6),
        ("ImmSetCompositionStringW", ret1_6),
        ("ImmNotifyIME", ret1_4),
        ("ImmGetCandidateWindow", ret0_3),
        ("ImmSetCandidateWindow", ret1_2),
        ("ImmGetCompositionWindow", ret0_2),
        ("ImmSetCompositionWindow", ret1_2),
        ("ImmGetStatusWindowPos", ret0_2),
        ("ImmSetStatusWindowPos", ret1_2),
        ("ImmGetDefaultIMEWnd", ret0_1),
        ("ImmIsIME", ret0_1),
        ("ImmGetProperty", ret0_2),
        ("ImmGetVirtualKey", ret0_1),
        ("ImmDisableIME", ret1_1),
        ("ImmEscapeA", ret0_3),
        ("ImmEscapeW", ret0_3),
        ("ImmGetDescriptionA", ret0_3),
        ("ImmGetDescriptionW", ret0_3),
        ("ImmGetIMEFileNameA", ret0_3),
        ("ImmGetIMEFileNameW", ret0_3),
        ("ImmRegisterWordA", ret0_4),
        ("ImmRegisterWordW", ret0_4),
        ("ImmUnregisterWordA", ret0_4),
        ("ImmUnregisterWordW", ret0_4),
        ("ImmEnumRegisterWordA", ret0_6),
        ("ImmEnumRegisterWordW", ret0_6),
    ];
    for &(name, handler) in fns {
        r.add("imm32.dll", name, handler);
    }
}

fn ret1_6(c: &mut ApiContext) -> Handled { c.ret_stdcall(1, 6); Handled::Ok }
