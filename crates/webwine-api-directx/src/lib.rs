//! DirectX-era COM APIs — ddraw / d3d8 / dsound / dinput.
//!
//! COM objects are `[vtable_ptr, extra…]` on the guest heap. Each interface is a
//! `Vtable` of `(method_name, handler)` pairs that is both the slot order and the
//! trampoline registry key. Guest `CALL [vtable+N*4]` hits the right handler with
//! the correct stdcall cleanup count.
//!
//! Shared helpers: `hr_ok_N` / `ref_N` for pure success paths, dedicated fns when
//! out-params or state matter. Arg counts are load-bearing — a wrong N corrupts ESP.

pub mod d3d8;
pub mod ddraw;
pub mod dinput;
pub mod dsound;

pub use webwine_api::winapi::{ApiContext, Handled, HandlerFn, WinApiRegistry};

pub(crate) type Vtable = &'static [(&'static str, HandlerFn)];

/// Synthetic trampoline bucket (not an importable DLL name).
pub(crate) const VTBL: &str = "directx.vtbl";

const S_OK: u32 = 0;

/// Return S_OK and clean `n` stdcall args (incl. `this`).
macro_rules! hr_ok {
    ($($name:ident => $n:expr),* $(,)?) => {
        $( pub(crate) fn $name(c: &mut ApiContext) -> Handled {
            c.ret_stdcall(S_OK, $n);
            Handled::Ok
        } )*
    };
}
hr_ok! {
    hr_ok_1 => 1, hr_ok_2 => 2, hr_ok_3 => 3, hr_ok_4 => 4,
    hr_ok_5 => 5, hr_ok_6 => 6, hr_ok_7 => 7, hr_ok_9 => 9,
}

/// IUnknown::QueryInterface — hand back `this` for any IID (pragmatic).
pub(crate) fn com_qi(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let ppv = c.arg(2);
    if ppv != 0 {
        let _ = c.memory.write_u32(ppv, this);
    }
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

pub(crate) fn com_addref(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(2, 1);
    Handled::Ok
}

pub(crate) fn com_release(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}

/// Allocate a COM object: vtable of trampoline VAs + `[vtable_ptr, extra]`.
pub(crate) fn make_object(ctx: &mut ApiContext, vt: Vtable, extra: u32) -> u32 {
    make_object_sized(ctx, vt, extra, 8)
}

/// Like `make_object` but with a larger payload after the vtable pointer.
pub(crate) fn make_object_sized(ctx: &mut ApiContext, vt: Vtable, extra: u32, size: u32) -> u32 {
    let vtable_va = ctx.heap_alloc((vt.len() * 4) as u32);
    for (i, (name, _)) in vt.iter().enumerate() {
        let tramp = ctx.api_resolve_trampoline(VTBL, name);
        let _ = ctx.memory.write_u32(vtable_va + i as u32 * 4, tramp);
    }
    let obj_va = ctx.heap_alloc(size.max(8));
    let _ = ctx.memory.write_u32(obj_va, vtable_va);
    let _ = ctx.memory.write_u32(obj_va + 4, extra);
    obj_va
}

pub(crate) fn object_extra(ctx: &ApiContext, obj_va: u32) -> u32 {
    ctx.memory.read_u32(obj_va + 4).unwrap_or(0)
}

pub(crate) fn register_vtable(r: &mut WinApiRegistry, vt: Vtable) {
    for (name, f) in vt {
        r.add(VTBL, name, *f);
    }
}

pub fn register(r: &mut WinApiRegistry) {
    ddraw::register(r);
    d3d8::register(r);
    dsound::register(r);
    dinput::register(r);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dlls_and_methods_register() {
        let mut r = WinApiRegistry::new();
        register(&mut r);
        r.finalize();

        for dll in ["ddraw.dll", "d3d8.dll", "dsound.dll", "dinput8.dll"] {
            assert!(r.has_stub_dll(dll), "{dll} not registered");
        }
        for name in [
            "DirectDrawCreate",
            "Direct3DCreate8",
            "DirectSoundCreate8",
            "DirectInput8Create",
            "IDirectDraw7::CreateSurface",
            "IDirect3DDevice8::DrawPrimitive",
            "IDirectSoundBuffer8::Lock",
            "IDirectInputDevice8::GetDeviceData",
        ] {
            assert!(r.proc_address(name) != 0, "{name} did not resolve");
        }
        assert!(!r.has_stub_dll(VTBL));
    }

    #[test]
    fn vtable_lengths_match_interfaces() {
        assert_eq!(ddraw::IDDRAW7.len(), 33);
        assert_eq!(ddraw::IDDSURFACE7.len(), 37);
        assert_eq!(ddraw::IDDCLIPPER.len(), 8);
        assert_eq!(d3d8::IDIRECT3D8.len(), 16);
        assert_eq!(d3d8::IDIRECT3DDEVICE8.len(), 94);
        assert_eq!(dsound::IDIRECTSOUND8.len(), 12);
        assert_eq!(dsound::IDIRECTSOUNDBUFFER8.len(), 24);
        assert_eq!(dinput::IDIRECTINPUT8.len(), 11);
        assert_eq!(dinput::IDIRECTINPUTDEVICE8.len(), 32);
    }
}
