//! Media Foundation stub (mfplat.dll / mfreadwrite.dll / mf.dll).
//!
//! Media Foundation is the modern Windows media subsystem (used by WMP, many
//! players/transcoders, browsers' fallback decoders, …). This crate gets an MF
//! app past *platform init and object creation* without crashing:
//!
//!   - the flat exports (`MFStartup`, `MFCreateAttributes`, `MFCreateSample`, …)
//!     succeed and hand back real COM objects;
//!   - those objects (`IMFAttributes`, `IMFMediaType`, `IMFSample`,
//!     `IMFMediaBuffer`) expose full, correctly-sized vtables so the guest's
//!     `CALL [vtable + N*4]` lands on a handler that returns a sane HRESULT and
//!     cleans the exact stdcall arg count (a wrong count drifts the guest stack).
//!
//! What is NOT done yet (returns E_NOTIMPL, but with correct arg cleanup so it
//! fails gracefully instead of crashing): actual demux/decode — the source
//! reader / sink writer (`mfreadwrite`) and the media session/topology (`mf`).
//! Those are the next layer; the COM object-model scaffolding here is what they
//! build on.
//!
//! COM layout mirrors the DDraw stub: each object is `[vtable ptr]` on the guest
//! heap; the vtable is N trampoline VAs; `this` arrives as stdcall arg 0.

pub use webwine_api::winapi::{ApiContext, Handled, HandlerFn, WinApiRegistry};

// HRESULTs.
const S_OK: u32 = 0x0000_0000;
const E_NOTIMPL: u32 = 0x8000_4001;
const MF_E_ATTRIBUTENOTFOUND: u32 = 0xC00D_36E6;

// Fake DLL bucket for vtable method trampolines (never an importable name).
const VTBL: &str = "mediafoundation.vtbl";

/// Return-a-constant / clean-N-args handlers. Naming `xx_n`: `ok`=S_OK,
/// `ni`=E_NOTIMPL, `nf`=MF_E_ATTRIBUTENOTFOUND; `n` = stdcall args incl. `this`.
macro_rules! ret_stubs {
    ($($name:ident => ($val:expr, $n:expr)),* $(,)?) => {
        $( fn $name(c: &mut ApiContext) -> Handled { c.ret_stdcall($val, $n); Handled::Ok } )*
    };
}
ret_stubs! {
    ok_0 => (S_OK, 0), ok_1 => (S_OK, 1), ok_2 => (S_OK, 2), ok_3 => (S_OK, 3),
    ok_4 => (S_OK, 4), ok_6 => (S_OK, 6),
    ni_1 => (E_NOTIMPL, 1), ni_2 => (E_NOTIMPL, 2), ni_3 => (E_NOTIMPL, 3),
    ni_4 => (E_NOTIMPL, 4), ni_6 => (E_NOTIMPL, 6),
    nf_3 => (MF_E_ATTRIBUTENOTFOUND, 3), nf_4 => (MF_E_ATTRIBUTENOTFOUND, 4),
    nf_5 => (MF_E_ATTRIBUTENOTFOUND, 5),
}

/// Write 0 to the out-param at arg1 and return S_OK (clean 2). Used by the
/// "count/length" getters (GetCount, GetBufferCount, GetTotalLength, …) so an
/// app that enumerates sees an empty-but-valid object.
fn write0_2(c: &mut ApiContext) -> Handled {
    let p = c.arg(1);
    if p != 0 {
        let _ = c.memory.write_u32(p, 0);
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

// ---- IUnknown (shared by every interface) ----

/// QueryInterface(this, riid, ppv): hand back `this` for any IID (pragmatic, like
/// the DDraw stub) so interface casts succeed.
fn com_qi(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let ppv = c.arg(2);
    if ppv != 0 {
        let _ = c.memory.write_u32(ppv, this);
    }
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}
fn com_addref(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(2, 1);
    Handled::Ok
}
fn com_release(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}

const IUNKNOWN: &[(&str, HandlerFn)] = &[
    ("IUnknown::QueryInterface", com_qi),
    ("IUnknown::AddRef", com_addref),
    ("IUnknown::Release", com_release),
];

// ---- IMFAttributes (slots 3..32; setters S_OK, getters MF_E_ATTRIBUTENOTFOUND) ----
// REFGUID / REFPROPVARIANT are pointers (1 slot); UINT64 / double pass as 2 slots.
const IMFATTRIBUTES_OWN: &[(&str, HandlerFn)] = &[
    ("IMFAttributes::GetItem", nf_3),
    ("IMFAttributes::GetItemType", nf_3),
    ("IMFAttributes::CompareItem", nf_4),
    ("IMFAttributes::Compare", nf_4),
    ("IMFAttributes::GetUINT32", nf_3),
    ("IMFAttributes::GetUINT64", nf_3),
    ("IMFAttributes::GetDouble", nf_3),
    ("IMFAttributes::GetGUID", nf_3),
    ("IMFAttributes::GetStringLength", nf_3),
    ("IMFAttributes::GetString", nf_5),
    ("IMFAttributes::GetAllocatedString", nf_4),
    ("IMFAttributes::GetBlobSize", nf_3),
    ("IMFAttributes::GetBlob", nf_5),
    ("IMFAttributes::GetAllocatedBlob", nf_4),
    ("IMFAttributes::GetUnknown", nf_4),
    ("IMFAttributes::SetItem", ok_3),
    ("IMFAttributes::DeleteItem", ok_2),
    ("IMFAttributes::DeleteAllItems", ok_1),
    ("IMFAttributes::SetUINT32", ok_3),
    ("IMFAttributes::SetUINT64", ok_4), // UINT64 = 2 slots
    ("IMFAttributes::SetDouble", ok_4), // double = 2 slots
    ("IMFAttributes::SetGUID", ok_3),
    ("IMFAttributes::SetString", ok_3),
    ("IMFAttributes::SetBlob", ok_4),
    ("IMFAttributes::SetUnknown", ok_3),
    ("IMFAttributes::LockStore", ok_1),
    ("IMFAttributes::UnlockStore", ok_1),
    ("IMFAttributes::GetCount", write0_2),
    ("IMFAttributes::GetItemByIndex", nf_4),
    ("IMFAttributes::CopyAllItems", ok_2),
];

// ---- IMFMediaType = IMFAttributes + 5 (slots 36..40) ----
const IMFMEDIATYPE_OWN: &[(&str, HandlerFn)] = &[
    ("IMFMediaType::GetMajorType", ni_2),
    ("IMFMediaType::IsCompressedFormat", ni_2),
    ("IMFMediaType::IsEqual", ni_3),
    ("IMFMediaType::GetRepresentation", ni_6), // GUID by value (4 slots) + ppv
    ("IMFMediaType::FreeRepresentation", ok_6),
];

// ---- IMFSample = IMFAttributes + 14 (slots 36..49) ----
const IMFSAMPLE_OWN: &[(&str, HandlerFn)] = &[
    ("IMFSample::GetSampleFlags", ni_2),
    ("IMFSample::SetSampleFlags", ok_2),
    ("IMFSample::GetSampleTime", ni_2),
    ("IMFSample::SetSampleTime", ok_3), // LONGLONG = 2 slots
    ("IMFSample::GetSampleDuration", ni_2),
    ("IMFSample::SetSampleDuration", ok_3),
    ("IMFSample::GetBufferCount", write0_2),
    ("IMFSample::GetBufferByIndex", ni_3),
    ("IMFSample::ConvertToContiguousBuffer", ni_2),
    ("IMFSample::AddBuffer", ok_2),
    ("IMFSample::RemoveBufferByIndex", ok_2),
    ("IMFSample::RemoveAllBuffers", ok_1),
    ("IMFSample::GetTotalLength", write0_2),
    ("IMFSample::CopyToBuffer", ni_2),
];

// ---- IMFMediaBuffer (IUnknown + 5) ----
const IMFMEDIABUFFER_OWN: &[(&str, HandlerFn)] = &[
    ("IMFMediaBuffer::Lock", ni_4),
    ("IMFMediaBuffer::Unlock", ok_1),
    ("IMFMediaBuffer::GetCurrentLength", write0_2),
    ("IMFMediaBuffer::SetCurrentLength", ok_2),
    ("IMFMediaBuffer::GetMaxLength", write0_2),
];

/// Build a COM object on the guest heap from one or more vtable segments (each a
/// slice of named slots, laid out in order). Returns the object VA.
fn com_object(ctx: &mut ApiContext, segments: &[&[(&str, HandlerFn)]]) -> u32 {
    let count: u32 = segments.iter().map(|s| s.len() as u32).sum();
    let vtable_va = ctx.heap_alloc(count * 4);
    let mut i = 0u32;
    for seg in segments {
        for (name, _) in *seg {
            let tramp = ctx.api_resolve_trampoline(VTBL, name);
            let _ = ctx.memory.write_u32(vtable_va + i * 4, tramp);
            i += 1;
        }
    }
    let obj_va = ctx.heap_alloc(4);
    let _ = ctx.memory.write_u32(obj_va, vtable_va);
    obj_va
}

/// Write `obj` to the out-param pointer at `arg_index`, if non-null.
fn store_out(ctx: &mut ApiContext, arg_index: u32, obj: u32) {
    let pp = ctx.arg(arg_index);
    if pp != 0 {
        let _ = ctx.memory.write_u32(pp, obj);
    }
}

// ---- mfplat flat exports that create objects ----

fn mf_create_attributes(c: &mut ApiContext) -> Handled {
    let obj = com_object(c, &[IUNKNOWN, IMFATTRIBUTES_OWN]);
    store_out(c, 0, obj); // ppMFAttributes
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn mf_create_media_type(c: &mut ApiContext) -> Handled {
    let obj = com_object(c, &[IUNKNOWN, IMFATTRIBUTES_OWN, IMFMEDIATYPE_OWN]);
    store_out(c, 0, obj);
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn mf_create_sample(c: &mut ApiContext) -> Handled {
    let obj = com_object(c, &[IUNKNOWN, IMFATTRIBUTES_OWN, IMFSAMPLE_OWN]);
    store_out(c, 0, obj);
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn mf_create_memory_buffer(c: &mut ApiContext) -> Handled {
    let obj = com_object(c, &[IUNKNOWN, IMFMEDIABUFFER_OWN]);
    store_out(c, 1, obj); // (cbMaxLength, ppBuffer)
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn mf_create_aligned_memory_buffer(c: &mut ApiContext) -> Handled {
    let obj = com_object(c, &[IUNKNOWN, IMFMEDIABUFFER_OWN]);
    store_out(c, 2, obj); // (cbMaxLength, cbAligment, ppBuffer)
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

/// MFTEnumEx(guidCategory[GUID by value=4 slots], Flags, pIn, pOut,
/// pppMFTActivate, pnumMFTActivate) — report "no MFTs found" (count 0, S_OK).
fn mf_enum_ex(c: &mut ApiContext) -> Handled {
    store_out(c, 7, 0); // pppMFTActivate
    let pcount = c.arg(8);
    if pcount != 0 {
        let _ = c.memory.write_u32(pcount, 0);
    }
    c.ret_stdcall(S_OK, 9);
    Handled::Ok
}

/// Register the Media Foundation DLL stubs.
pub fn register(r: &mut WinApiRegistry) {
    // mfplat.dll — platform + object model.
    let mfplat: &[(&str, HandlerFn)] = &[
        ("MFStartup", ok_2),
        ("MFShutdown", ok_0),
        ("MFLockPlatform", ok_0),
        ("MFUnlockPlatform", ok_0),
        ("MFCreateAttributes", mf_create_attributes),
        ("MFCreateMediaType", mf_create_media_type),
        ("MFCreateSample", mf_create_sample),
        ("MFCreateMemoryBuffer", mf_create_memory_buffer),
        ("MFCreateAlignedMemoryBuffer", mf_create_aligned_memory_buffer),
        ("MFTEnumEx", mf_enum_ex),
        ("MFGetService", ni_4),
        ("MFCreateSourceResolver", ni_1),
        ("MFCreateEventQueue", ni_1),
        ("MFCreateMediaEvent", ni_6),
        ("MFCreateCollection", ni_1),
        ("MFCreateSystemTimeSource", ni_1),
        ("MFCreateDXGIDeviceManager", ni_2),
        ("MFInitMediaTypeFromWaveFormatEx", ok_3),
        ("MFCreateWaveFormatExFromMFMediaType", ni_4),
        ("MFAllocateWorkQueue", write0_2),
    ];
    for &(name, f) in mfplat {
        r.add("mfplat.dll", name, f);
    }

    // mfreadwrite.dll — source reader / sink writer (decode layer, not yet
    // implemented: fail gracefully with E_NOTIMPL and correct arg cleanup).
    let mfreadwrite: &[(&str, HandlerFn)] = &[
        ("MFCreateSourceReaderFromURL", ni_3),
        ("MFCreateSourceReaderFromByteStream", ni_3),
        ("MFCreateSourceReaderFromMediaSource", ni_3),
        ("MFCreateSinkWriterFromURL", ni_4),
        ("MFCreateSinkWriterFromMediaSink", ni_3),
    ];
    for &(name, f) in mfreadwrite {
        r.add("mfreadwrite.dll", name, f);
    }

    // mf.dll — media session / topology (pipeline layer, not yet implemented).
    let mf: &[(&str, HandlerFn)] = &[
        ("MFCreateMediaSession", ni_2),
        ("MFCreateTopology", ni_1),
        ("MFCreateTopologyNode", ni_2),
        ("MFCreatePresentationDescriptor", ni_3),
    ];
    for &(name, f) in mf {
        r.add("mf.dll", name, f);
    }

    // Register every COM vtable method so each slot routes to a handler with the
    // right arg count (resolved by the globally-unique method name).
    for seg in [
        IUNKNOWN,
        IMFATTRIBUTES_OWN,
        IMFMEDIATYPE_OWN,
        IMFSAMPLE_OWN,
        IMFMEDIABUFFER_OWN,
    ] {
        for &(name, f) in seg {
            r.add(VTBL, name, f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mf_dlls_register_and_resolve() {
        let mut r = WinApiRegistry::new();
        register(&mut r);
        r.finalize();

        // The three MF DLLs are recognized as built-in stubs (loader treats them
        // as provided; they get System32 ghost files).
        for dll in ["mfplat.dll", "mfreadwrite.dll", "mf.dll"] {
            assert!(r.has_stub_dll(dll), "{dll} not registered as a stub DLL");
        }
        // Flat exports and vtable methods resolve to callable trampolines.
        for name in [
            "MFStartup",
            "MFCreateAttributes",
            "MFCreateSourceReaderFromURL",
            "IMFAttributes::SetUINT32",
            "IMFSample::AddBuffer",
        ] {
            assert!(r.proc_address(name) != 0, "{name} did not resolve");
        }
        // The synthetic vtable bucket is not exposed as an importable DLL.
        assert!(!r.has_stub_dll(VTBL));
    }
}
