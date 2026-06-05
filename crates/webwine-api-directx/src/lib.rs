//! DirectX-era DLL stubs — one submodule per DLL (ddraw / d3d8 / dsound / dinput).
//!
//! Shared COM scaffolding lives here. A COM object is `[vtable ptr, extra]` on the
//! guest heap; an interface is a `Vtable` = `&[(method_name, handler)]` that is
//! BOTH the vtable slot order AND the per-slot handler. The method name doubles as
//! the (globally-unique) trampoline key, so the guest's `CALL [vtable + N*4]` lands
//! directly on the right handler — no reverse "which slot is this EIP?" lookup.
//! Trivial slots reuse the shared `sV_N` return-constant stubs (value V, clean N
//! stdcall args); slots with real logic get a dedicated fn.

pub mod d3d8;
pub mod ddraw;
pub mod dinput;
pub mod dsound;

pub use webwine_api::winapi::{ApiContext, Handled, HandlerFn, WinApiRegistry};

/// One COM interface: vtable slots in order, each pairing a method name with its
/// handler. The name is the trampoline key (globally unique across all DLLs).
pub(crate) type Vtable = &'static [(&'static str, HandlerFn)];

/// Shared trampoline bucket for every DirectX vtable method. The registry excludes
/// `*.vtbl` buckets from the importable-DLL set; method resolution is by the unique
/// name regardless of bucket.
pub(crate) const VTBL: &str = "directx.vtbl";

/// Return-a-constant / clean-N-args stubs. `sV_N`: value V in EAX, clean N stdcall
/// args (incl. `this`). A wrong N drifts the guest stack, so the count per slot is
/// load-bearing.
macro_rules! stubs {
    ($($name:ident => ($val:expr, $n:expr)),* $(,)?) => {
        $( #[allow(dead_code)] pub(crate) fn $name(c: &mut ApiContext) -> Handled {
            c.ret_stdcall($val, $n); Handled::Ok
        } )*
    };
}
stubs! {
    s0_1 => (0, 1), s0_2 => (0, 2), s0_3 => (0, 3), s0_4 => (0, 4),
    s0_5 => (0, 5), s0_6 => (0, 6), s0_7 => (0, 7), s0_8 => (0, 8),
    s1_1 => (1, 1), s1_2 => (1, 2),
}

/// IUnknown::QueryInterface — hand back `this` for any IID (covers vN→vN upgrades,
/// like the rest of these pragmatic stubs).
pub(crate) fn com_qi(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let ppv = c.arg(2);
    if ppv != 0 {
        let _ = c.memory.write_u32(ppv, this);
    }
    c.ret_stdcall(0, 3);
    Handled::Ok
}

/// Allocate a COM object on the guest heap: a vtable of trampoline VAs (one per
/// slot) plus the object `[vtable_ptr, extra]`. `extra` carries per-object state
/// (e.g. a DDraw surface id). Returns the object VA.
pub(crate) fn make_object(ctx: &mut ApiContext, vt: Vtable, extra: u32) -> u32 {
    let vtable_va = ctx.heap_alloc((vt.len() * 4) as u32);
    for (i, (name, _)) in vt.iter().enumerate() {
        let tramp = ctx.api_resolve_trampoline(VTBL, name);
        let _ = ctx.memory.write_u32(vtable_va + i as u32 * 4, tramp);
    }
    let obj_va = ctx.heap_alloc(8);
    let _ = ctx.memory.write_u32(obj_va, vtable_va);
    let _ = ctx.memory.write_u32(obj_va + 4, extra);
    obj_va
}

/// Read the `extra` word stored at object+4 (surface id, etc.).
pub(crate) fn object_extra(ctx: &ApiContext, obj_va: u32) -> u32 {
    ctx.memory.read_u32(obj_va + 4).unwrap_or(0)
}

/// Register every slot of an interface under the shared vtable bucket.
pub(crate) fn register_vtable(r: &mut WinApiRegistry, vt: Vtable) {
    for (name, f) in vt {
        r.add(VTBL, name, *f);
    }
}

/// Register all DirectX-era DLL stubs.
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
        // The synthetic vtable bucket is not an importable DLL.
        assert!(!r.has_stub_dll(VTBL));
    }

    /// Vtable lengths must equal the real COM interface slot counts — a wrong
    /// length means a shifted/missing slot, which silently breaks every method
    /// above it. (Counts: IUnknown=3 + interface methods.)
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
