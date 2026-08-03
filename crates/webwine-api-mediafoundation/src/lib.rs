//! Media Foundation (mfplat.dll / mfreadwrite.dll / mf.dll).
//!
//! Provides platform init, attribute bags, media types, samples, and memory
//! buffers with real guest-side storage. Demux/decode (source reader, session)
//! still returns E_NOTIMPL with correct stack cleanup.

pub use webwine_api::winapi::{ApiContext, Handled, HandlerFn, WinApiRegistry};

const S_OK: u32 = 0x0000_0000;
const E_INVALIDARG: u32 = 0x8007_0057;
const E_POINTER: u32 = 0x8000_4003;
const E_NOTIMPL: u32 = 0x8000_4001;
const E_OUTOFMEMORY: u32 = 0x8007_000E;
const MF_E_ATTRIBUTENOTFOUND: u32 = 0xC00D_36E6;
const MF_E_INVALIDREQUEST: u32 = 0xC00D_36B2;

const VTBL: &str = "mediafoundation.vtbl";

// Object kinds stored at obj+4.
const KIND_ATTRS: u32 = 1;
const KIND_MEDIA_TYPE: u32 = 2;
const KIND_SAMPLE: u32 = 3;
const KIND_BUFFER: u32 = 4;
const KIND_COLLECTION: u32 = 5;

// Buffer object layout: [vtable, kind, max_len, cur_len, data_ptr]
const BUF_MAX: u32 = 8;
const BUF_CUR: u32 = 12;
const BUF_DATA: u32 = 16;
const BUF_OBJ_SIZE: u32 = 20;

// Sample layout: [vtable, kind, flags, time_lo, time_hi, dur_lo, dur_hi, buf_count, buf0..]
const SMP_FLAGS: u32 = 8;
const SMP_TIME_LO: u32 = 12;
const SMP_TIME_HI: u32 = 16;
const SMP_DUR_LO: u32 = 20;
const SMP_DUR_HI: u32 = 24;
const SMP_BUF_COUNT: u32 = 28;
const SMP_BUF0: u32 = 32;
const SMP_MAX_BUFS: u32 = 8;
const SMP_OBJ_SIZE: u32 = SMP_BUF0 + SMP_MAX_BUFS * 4;

// helpers

fn guid_key(ctx: &ApiContext, guid_ptr: u32) -> u32 {
    // Fold GUID bytes into a stable u32 key for dll_state.
    if guid_ptr == 0 {
        return 0;
    }
    let mut h = 0u32;
    for i in 0..4 {
        h ^= ctx
            .memory
            .read_u32(guid_ptr + i * 4)
            .unwrap_or(0)
            .rotate_left(i * 7);
    }
    h
}

fn attr_key(obj: u32, guid_hash: u32) -> String {
    format!("mf.a.{obj:08x}.{guid_hash:08x}")
}

fn attr_count_key(obj: u32) -> String {
    format!("mf.ac.{obj:08x}")
}

fn attr_set_u32(ctx: &mut ApiContext, obj: u32, guid_ptr: u32, val: u32) {
    let k = attr_key(obj, guid_key(ctx, guid_ptr));
    if !ctx.dll_state.contains_key(&k) {
        let ck = attr_count_key(obj);
        let n = ctx.dll_state.get(&ck).copied().unwrap_or(0);
        ctx.dll_state.insert(ck, n + 1);
    }
    ctx.dll_state.insert(k, val);
}

fn attr_get_u32(ctx: &ApiContext, obj: u32, guid_ptr: u32) -> Option<u32> {
    ctx.dll_state
        .get(&attr_key(obj, guid_key(ctx, guid_ptr)))
        .copied()
}

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

fn hr_ok_n(c: &mut ApiContext, n: u32) -> Handled {
    c.ret_stdcall(S_OK, n);
    Handled::Ok
}

fn hr_ni_n(c: &mut ApiContext, n: u32) -> Handled {
    c.ret_stdcall(E_NOTIMPL, n);
    Handled::Ok
}

// Named wrappers for vtable slots that only need fixed HRESULT + arg cleanup.
macro_rules! hr {
    ($($name:ident => ($code:expr, $n:expr)),* $(,)?) => {
        $( fn $name(c: &mut ApiContext) -> Handled { c.ret_stdcall($code, $n); Handled::Ok } )*
    };
}
hr! {
    ok_1 => (S_OK, 1), ok_2 => (S_OK, 2), ok_3 => (S_OK, 3), ok_4 => (S_OK, 4),
    ok_5 => (S_OK, 5), ok_6 => (S_OK, 6),
    ni_1 => (E_NOTIMPL, 1), ni_2 => (E_NOTIMPL, 2), ni_3 => (E_NOTIMPL, 3),
    ni_4 => (E_NOTIMPL, 4), ni_5 => (E_NOTIMPL, 5), ni_6 => (E_NOTIMPL, 6),
}

fn store_out(ctx: &mut ApiContext, arg_index: u32, obj: u32) {
    let pp = ctx.arg(arg_index);
    if pp != 0 {
        let _ = ctx.memory.write_u32(pp, obj);
    }
}

fn com_object(
    ctx: &mut ApiContext,
    segments: &[&[(&str, HandlerFn)]],
    kind: u32,
    size: u32,
) -> u32 {
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
    let obj_va = ctx.heap_alloc(size.max(8));
    let _ = ctx.memory.write_u32(obj_va, vtable_va);
    let _ = ctx.memory.write_u32(obj_va + 4, kind);
    obj_va
}

// IMFAttributes

fn attr_get_item(c: &mut ApiContext) -> Handled {
    // GetItem(this, guid, pValue) — PROPVARIANT not fully filled; report not found.
    c.ret_stdcall(MF_E_ATTRIBUTENOTFOUND, 3);
    Handled::Ok
}

fn attr_get_item_type(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(MF_E_ATTRIBUTENOTFOUND, 3);
    Handled::Ok
}

fn attr_get_uint32(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let guid = c.arg(1);
    let out = c.arg(2);
    match attr_get_u32(c, this, guid) {
        Some(v) => {
            if out != 0 {
                let _ = c.memory.write_u32(out, v);
            }
            c.ret_stdcall(S_OK, 3);
        }
        None => c.ret_stdcall(MF_E_ATTRIBUTENOTFOUND, 3),
    }
    Handled::Ok
}

fn attr_get_uint64(c: &mut ApiContext) -> Handled {
    // GetUINT64(this, guid, pulValue) — we store only low 32; high 0.
    let this = c.arg(0);
    let guid = c.arg(1);
    let out = c.arg(2);
    match attr_get_u32(c, this, guid) {
        Some(v) => {
            if out != 0 {
                let _ = c.memory.write_u32(out, v);
                let _ = c.memory.write_u32(out + 4, 0);
            }
            c.ret_stdcall(S_OK, 3);
        }
        None => c.ret_stdcall(MF_E_ATTRIBUTENOTFOUND, 3),
    }
    Handled::Ok
}

fn attr_set_item(c: &mut ApiContext) -> Handled {
    // SetItem(this, guid, value) — accept without decoding PROPVARIANT.
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn attr_set_uint32(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let guid = c.arg(1);
    let val = c.arg(2);
    if guid == 0 {
        c.ret_stdcall(E_INVALIDARG, 3);
        return Handled::Ok;
    }
    attr_set_u32(c, this, guid, val);
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn attr_set_uint64(c: &mut ApiContext) -> Handled {
    // SetUINT64(this, guid, unValue lo, hi) — store low dword.
    let this = c.arg(0);
    let guid = c.arg(1);
    let lo = c.arg(2);
    if guid == 0 {
        c.ret_stdcall(E_INVALIDARG, 4);
        return Handled::Ok;
    }
    attr_set_u32(c, this, guid, lo);
    c.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn attr_set_double(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn attr_set_guid(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn attr_set_string(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn attr_set_blob(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn attr_set_unknown(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn attr_delete_item(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let guid = c.arg(1);
    let k = attr_key(this, guid_key(c, guid));
    if c.dll_state.remove(&k).is_some() {
        let ck = attr_count_key(this);
        let n = c.dll_state.get(&ck).copied().unwrap_or(1);
        c.dll_state.insert(ck, n.saturating_sub(1));
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn attr_delete_all(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let prefix = format!("mf.a.{this:08x}.");
    let keys: Vec<String> = c
        .dll_state
        .keys()
        .filter(|k| k.starts_with(&prefix))
        .cloned()
        .collect();
    for k in keys {
        c.dll_state.remove(&k);
    }
    c.dll_state.insert(attr_count_key(this), 0);
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn attr_get_count(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let out = c.arg(1);
    let n = c.dll_state.get(&attr_count_key(this)).copied().unwrap_or(0);
    if out != 0 {
        let _ = c.memory.write_u32(out, n);
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn attr_copy_all(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn attr_compare(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 4);
    Handled::Ok
}

const IUNKNOWN: &[(&str, HandlerFn)] = &[
    ("IUnknown::QueryInterface", com_qi),
    ("IUnknown::AddRef", com_addref),
    ("IUnknown::Release", com_release),
];

const IMFATTRIBUTES_OWN: &[(&str, HandlerFn)] = &[
    ("IMFAttributes::GetItem", attr_get_item),
    ("IMFAttributes::GetItemType", attr_get_item_type),
    ("IMFAttributes::CompareItem", attr_compare),
    ("IMFAttributes::Compare", attr_compare),
    ("IMFAttributes::GetUINT32", attr_get_uint32),
    ("IMFAttributes::GetUINT64", attr_get_uint64),
    ("IMFAttributes::GetDouble", ni_3),
    ("IMFAttributes::GetGUID", ni_3),
    ("IMFAttributes::GetStringLength", ni_3),
    ("IMFAttributes::GetString", ni_5),
    ("IMFAttributes::GetAllocatedString", ni_4),
    ("IMFAttributes::GetBlobSize", ni_3),
    ("IMFAttributes::GetBlob", ni_5),
    ("IMFAttributes::GetAllocatedBlob", ni_4),
    ("IMFAttributes::GetUnknown", ni_4),
    ("IMFAttributes::SetItem", attr_set_item),
    ("IMFAttributes::DeleteItem", attr_delete_item),
    ("IMFAttributes::DeleteAllItems", attr_delete_all),
    ("IMFAttributes::SetUINT32", attr_set_uint32),
    ("IMFAttributes::SetUINT64", attr_set_uint64),
    ("IMFAttributes::SetDouble", attr_set_double),
    ("IMFAttributes::SetGUID", attr_set_guid),
    ("IMFAttributes::SetString", attr_set_string),
    ("IMFAttributes::SetBlob", attr_set_blob),
    ("IMFAttributes::SetUnknown", attr_set_unknown),
    ("IMFAttributes::LockStore", ok_1),
    ("IMFAttributes::UnlockStore", ok_1),
    ("IMFAttributes::GetCount", attr_get_count),
    ("IMFAttributes::GetItemByIndex", ni_4),
    ("IMFAttributes::CopyAllItems", attr_copy_all),
];

// IMFMediaType

fn mt_get_major_type(c: &mut ApiContext) -> Handled {
    // GetMajorType(this, pguidMajorType) — zero GUID if unset.
    let out = c.arg(1);
    if out != 0 {
        let _ = c.memory.write_bytes(out, &[0u8; 16]);
    }
    // Still S_OK with zeroed GUID (callers often overwrite).
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn mt_is_compressed(c: &mut ApiContext) -> Handled {
    let out = c.arg(1);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0); // FALSE
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn mt_is_equal(c: &mut ApiContext) -> Handled {
    let out = c.arg(2);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

const IMFMEDIATYPE_OWN: &[(&str, HandlerFn)] = &[
    ("IMFMediaType::GetMajorType", mt_get_major_type),
    ("IMFMediaType::IsCompressedFormat", mt_is_compressed),
    ("IMFMediaType::IsEqual", mt_is_equal),
    ("IMFMediaType::GetRepresentation", ni_6),
    ("IMFMediaType::FreeRepresentation", ok_6),
];

// IMFSample

fn sample_get_flags(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let out = c.arg(1);
    if out != 0 {
        let _ = c
            .memory
            .write_u32(out, c.memory.read_u32(this + SMP_FLAGS).unwrap_or(0));
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn sample_set_flags(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let _ = c.memory.write_u32(this + SMP_FLAGS, c.arg(1));
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn sample_get_time(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let out = c.arg(1);
    if out != 0 {
        let _ = c
            .memory
            .write_u32(out, c.memory.read_u32(this + SMP_TIME_LO).unwrap_or(0));
        let _ = c
            .memory
            .write_u32(out + 4, c.memory.read_u32(this + SMP_TIME_HI).unwrap_or(0));
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn sample_set_time(c: &mut ApiContext) -> Handled {
    // SetSampleTime(this, hnsSampleTime lo, hi)
    let this = c.arg(0);
    let _ = c.memory.write_u32(this + SMP_TIME_LO, c.arg(1));
    let _ = c.memory.write_u32(this + SMP_TIME_HI, c.arg(2));
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn sample_get_duration(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let out = c.arg(1);
    if out != 0 {
        let _ = c
            .memory
            .write_u32(out, c.memory.read_u32(this + SMP_DUR_LO).unwrap_or(0));
        let _ = c
            .memory
            .write_u32(out + 4, c.memory.read_u32(this + SMP_DUR_HI).unwrap_or(0));
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn sample_set_duration(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let _ = c.memory.write_u32(this + SMP_DUR_LO, c.arg(1));
    let _ = c.memory.write_u32(this + SMP_DUR_HI, c.arg(2));
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn sample_get_buffer_count(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let out = c.arg(1);
    if out != 0 {
        let _ = c
            .memory
            .write_u32(out, c.memory.read_u32(this + SMP_BUF_COUNT).unwrap_or(0));
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn sample_get_buffer_by_index(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let index = c.arg(1);
    let out = c.arg(2);
    let count = c.memory.read_u32(this + SMP_BUF_COUNT).unwrap_or(0);
    if index >= count {
        c.ret_stdcall(E_INVALIDARG, 3);
        return Handled::Ok;
    }
    let buf = c.memory.read_u32(this + SMP_BUF0 + index * 4).unwrap_or(0);
    if out != 0 {
        let _ = c.memory.write_u32(out, buf);
    }
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn sample_add_buffer(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let buf = c.arg(1);
    let count = c.memory.read_u32(this + SMP_BUF_COUNT).unwrap_or(0);
    if count >= SMP_MAX_BUFS {
        c.ret_stdcall(MF_E_INVALIDREQUEST, 2);
        return Handled::Ok;
    }
    let _ = c.memory.write_u32(this + SMP_BUF0 + count * 4, buf);
    let _ = c.memory.write_u32(this + SMP_BUF_COUNT, count + 1);
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn sample_remove_buffer_by_index(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let index = c.arg(1);
    let count = c.memory.read_u32(this + SMP_BUF_COUNT).unwrap_or(0);
    if index >= count {
        c.ret_stdcall(E_INVALIDARG, 2);
        return Handled::Ok;
    }
    // Shift remaining buffers down.
    for i in index..count.saturating_sub(1) {
        let next = c
            .memory
            .read_u32(this + SMP_BUF0 + (i + 1) * 4)
            .unwrap_or(0);
        let _ = c.memory.write_u32(this + SMP_BUF0 + i * 4, next);
    }
    let _ = c
        .memory
        .write_u32(this + SMP_BUF_COUNT, count.saturating_sub(1));
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn sample_remove_all_buffers(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let _ = c.memory.write_u32(this + SMP_BUF_COUNT, 0);
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn sample_get_total_length(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let out = c.arg(1);
    let count = c.memory.read_u32(this + SMP_BUF_COUNT).unwrap_or(0);
    let mut total = 0u32;
    for i in 0..count {
        let buf = c.memory.read_u32(this + SMP_BUF0 + i * 4).unwrap_or(0);
        if buf != 0 {
            total = total.saturating_add(c.memory.read_u32(buf + BUF_CUR).unwrap_or(0));
        }
    }
    if out != 0 {
        let _ = c.memory.write_u32(out, total);
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn sample_convert_contiguous(c: &mut ApiContext) -> Handled {
    // ConvertToContiguousBuffer(this, ppBuffer) — if one buffer, return it.
    let this = c.arg(0);
    let out = c.arg(1);
    let count = c.memory.read_u32(this + SMP_BUF_COUNT).unwrap_or(0);
    if count == 1 {
        let buf = c.memory.read_u32(this + SMP_BUF0).unwrap_or(0);
        if out != 0 {
            let _ = c.memory.write_u32(out, buf);
        }
        c.ret_stdcall(S_OK, 2);
    } else {
        if out != 0 {
            let _ = c.memory.write_u32(out, 0);
        }
        c.ret_stdcall(E_NOTIMPL, 2);
    }
    Handled::Ok
}

fn sample_copy_to_buffer(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(E_NOTIMPL, 2);
    Handled::Ok
}

const IMFSAMPLE_OWN: &[(&str, HandlerFn)] = &[
    ("IMFSample::GetSampleFlags", sample_get_flags),
    ("IMFSample::SetSampleFlags", sample_set_flags),
    ("IMFSample::GetSampleTime", sample_get_time),
    ("IMFSample::SetSampleTime", sample_set_time),
    ("IMFSample::GetSampleDuration", sample_get_duration),
    ("IMFSample::SetSampleDuration", sample_set_duration),
    ("IMFSample::GetBufferCount", sample_get_buffer_count),
    ("IMFSample::GetBufferByIndex", sample_get_buffer_by_index),
    (
        "IMFSample::ConvertToContiguousBuffer",
        sample_convert_contiguous,
    ),
    ("IMFSample::AddBuffer", sample_add_buffer),
    (
        "IMFSample::RemoveBufferByIndex",
        sample_remove_buffer_by_index,
    ),
    ("IMFSample::RemoveAllBuffers", sample_remove_all_buffers),
    ("IMFSample::GetTotalLength", sample_get_total_length),
    ("IMFSample::CopyToBuffer", sample_copy_to_buffer),
];

// IMFMediaBuffer

fn buffer_lock(c: &mut ApiContext) -> Handled {
    // Lock(this, ppbBuffer, pcbMaxLength, pcbCurrentLength)
    let this = c.arg(0);
    let ppb = c.arg(1);
    let pmax = c.arg(2);
    let pcur = c.arg(3);
    let data = c.memory.read_u32(this + BUF_DATA).unwrap_or(0);
    let max = c.memory.read_u32(this + BUF_MAX).unwrap_or(0);
    let cur = c.memory.read_u32(this + BUF_CUR).unwrap_or(0);
    if data == 0 {
        c.ret_stdcall(E_OUTOFMEMORY, 4);
        return Handled::Ok;
    }
    if ppb != 0 {
        let _ = c.memory.write_u32(ppb, data);
    }
    if pmax != 0 {
        let _ = c.memory.write_u32(pmax, max);
    }
    if pcur != 0 {
        let _ = c.memory.write_u32(pcur, cur);
    }
    c.ret_stdcall(S_OK, 4);
    Handled::Ok
}

fn buffer_unlock(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn buffer_get_current_length(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let out = c.arg(1);
    if out != 0 {
        let _ = c
            .memory
            .write_u32(out, c.memory.read_u32(this + BUF_CUR).unwrap_or(0));
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn buffer_set_current_length(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let len = c.arg(1);
    let max = c.memory.read_u32(this + BUF_MAX).unwrap_or(0);
    if len > max {
        c.ret_stdcall(E_INVALIDARG, 2);
        return Handled::Ok;
    }
    let _ = c.memory.write_u32(this + BUF_CUR, len);
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn buffer_get_max_length(c: &mut ApiContext) -> Handled {
    let this = c.arg(0);
    let out = c.arg(1);
    if out != 0 {
        let _ = c
            .memory
            .write_u32(out, c.memory.read_u32(this + BUF_MAX).unwrap_or(0));
    }
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

const IMFMEDIABUFFER_OWN: &[(&str, HandlerFn)] = &[
    ("IMFMediaBuffer::Lock", buffer_lock),
    ("IMFMediaBuffer::Unlock", buffer_unlock),
    (
        "IMFMediaBuffer::GetCurrentLength",
        buffer_get_current_length,
    ),
    (
        "IMFMediaBuffer::SetCurrentLength",
        buffer_set_current_length,
    ),
    ("IMFMediaBuffer::GetMaxLength", buffer_get_max_length),
];

// flat exports

fn mf_startup(c: &mut ApiContext) -> Handled {
    c.dll_state.insert("mf.started".into(), 1);
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn mf_shutdown(c: &mut ApiContext) -> Handled {
    c.dll_state.remove("mf.started");
    c.ret_stdcall(S_OK, 0);
    Handled::Ok
}

fn mf_create_attributes(c: &mut ApiContext) -> Handled {
    let obj = com_object(c, &[IUNKNOWN, IMFATTRIBUTES_OWN], KIND_ATTRS, 8);
    c.dll_state.insert(attr_count_key(obj), 0);
    store_out(c, 0, obj);
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn mf_create_media_type(c: &mut ApiContext) -> Handled {
    let obj = com_object(
        c,
        &[IUNKNOWN, IMFATTRIBUTES_OWN, IMFMEDIATYPE_OWN],
        KIND_MEDIA_TYPE,
        8,
    );
    c.dll_state.insert(attr_count_key(obj), 0);
    store_out(c, 0, obj);
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn mf_create_sample(c: &mut ApiContext) -> Handled {
    let obj = com_object(
        c,
        &[IUNKNOWN, IMFATTRIBUTES_OWN, IMFSAMPLE_OWN],
        KIND_SAMPLE,
        SMP_OBJ_SIZE,
    );
    c.dll_state.insert(attr_count_key(obj), 0);
    for off in (8..SMP_OBJ_SIZE).step_by(4) {
        let _ = c.memory.write_u32(obj + off, 0);
    }
    store_out(c, 0, obj);
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn make_memory_buffer(c: &mut ApiContext, max_len: u32) -> u32 {
    let max = max_len.max(1);
    let data = c.heap_alloc(max);
    let obj = com_object(
        c,
        &[IUNKNOWN, IMFMEDIABUFFER_OWN],
        KIND_BUFFER,
        BUF_OBJ_SIZE,
    );
    let _ = c.memory.write_u32(obj + BUF_MAX, max);
    let _ = c.memory.write_u32(obj + BUF_CUR, 0);
    let _ = c.memory.write_u32(obj + BUF_DATA, data);
    obj
}

fn mf_create_memory_buffer(c: &mut ApiContext) -> Handled {
    let max = c.arg(0);
    let obj = make_memory_buffer(c, max);
    store_out(c, 1, obj);
    c.ret_stdcall(S_OK, 2);
    Handled::Ok
}

fn mf_create_aligned_memory_buffer(c: &mut ApiContext) -> Handled {
    let max = c.arg(0);
    let obj = make_memory_buffer(c, max);
    store_out(c, 2, obj);
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn mf_enum_ex(c: &mut ApiContext) -> Handled {
    store_out(c, 7, 0);
    let pcount = c.arg(8);
    if pcount != 0 {
        let _ = c.memory.write_u32(pcount, 0);
    }
    c.ret_stdcall(S_OK, 9);
    Handled::Ok
}

fn mf_create_collection(c: &mut ApiContext) -> Handled {
    // Minimal empty collection object (IUnknown only is enough for QI fail paths).
    let obj = com_object(c, &[IUNKNOWN], KIND_COLLECTION, 8);
    store_out(c, 0, obj);
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn mf_allocate_work_queue(c: &mut ApiContext) -> Handled {
    let out = c.arg(0);
    if out != 0 {
        let _ = c.memory.write_u32(out, 1); // fake queue id
    }
    c.ret_stdcall(S_OK, 1);
    Handled::Ok
}

fn mf_init_media_type_from_wave(c: &mut ApiContext) -> Handled {
    // MFInitMediaTypeFromWaveFormatEx(pMFType, pWave, cb) — accept.
    c.ret_stdcall(S_OK, 3);
    Handled::Ok
}

fn mf_get_service(c: &mut ApiContext) -> Handled {
    let out = c.arg(3);
    if out != 0 {
        let _ = c.memory.write_u32(out, 0);
    }
    c.ret_stdcall(e_nointerface(), 4);
    Handled::Ok
}

fn e_nointerface() -> u32 {
    0x8000_4002 // E_NOINTERFACE
}

/// Register Media Foundation exports + COM vtables.
pub fn register(r: &mut WinApiRegistry) {
    let mfplat: &[(&str, HandlerFn)] = &[
        ("MFStartup", mf_startup),
        ("MFShutdown", mf_shutdown),
        ("MFLockPlatform", ok_0_fn),
        ("MFUnlockPlatform", ok_0_fn),
        ("MFCreateAttributes", mf_create_attributes),
        ("MFCreateMediaType", mf_create_media_type),
        ("MFCreateSample", mf_create_sample),
        ("MFCreateMemoryBuffer", mf_create_memory_buffer),
        (
            "MFCreateAlignedMemoryBuffer",
            mf_create_aligned_memory_buffer,
        ),
        ("MFTEnumEx", mf_enum_ex),
        ("MFGetService", mf_get_service),
        ("MFCreateSourceResolver", ni_1),
        ("MFCreateEventQueue", ni_1),
        ("MFCreateMediaEvent", ni_6),
        ("MFCreateCollection", mf_create_collection),
        ("MFCreateSystemTimeSource", ni_1),
        ("MFCreateDXGIDeviceManager", ni_2),
        (
            "MFInitMediaTypeFromWaveFormatEx",
            mf_init_media_type_from_wave,
        ),
        ("MFCreateWaveFormatExFromMFMediaType", ni_4),
        ("MFAllocateWorkQueue", mf_allocate_work_queue),
        ("MFPutWorkItem", ni_3),
        ("MFScheduleWorkItem", ni_4),
        ("MFCancelWorkItem", ok_1),
        ("MFGetSystemTime", mf_get_system_time),
    ];
    for &(name, f) in mfplat {
        r.add("mfplat.dll", name, f);
    }

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

    let mf: &[(&str, HandlerFn)] = &[
        ("MFCreateMediaSession", ni_2),
        ("MFCreateTopology", ni_1),
        ("MFCreateTopologyNode", ni_2),
        ("MFCreatePresentationDescriptor", ni_3),
    ];
    for &(name, f) in mf {
        r.add("mf.dll", name, f);
    }

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

fn ok_0_fn(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(S_OK, 0);
    Handled::Ok
}

fn mf_get_system_time(c: &mut ApiContext) -> Handled {
    // MFTIME (100ns units) — monotonic-ish value returned in EAX (low dword).
    let prev = c.dll_state.get("mf.time").copied().unwrap_or(0);
    let next = prev.wrapping_add(10_000); // +1ms
    c.dll_state.insert("mf.time".into(), next);
    c.cpu.edx = 0;
    c.ret_stdcall(next, 0);
    Handled::Ok
}

// Silence unused helper warnings when only used in comments paths.
#[allow(dead_code)]
fn _helpers() {
    let _ = (
        hr_ok_n as fn(&mut ApiContext, u32) -> Handled,
        hr_ni_n as fn(&mut ApiContext, u32) -> Handled,
    );
    let _ = (E_POINTER, KIND_ATTRS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mf_dlls_register_and_resolve() {
        let mut r = WinApiRegistry::new();
        register(&mut r);
        r.finalize();

        for dll in ["mfplat.dll", "mfreadwrite.dll", "mf.dll"] {
            assert!(r.has_stub_dll(dll), "{dll} not registered as a stub DLL");
        }
        for name in [
            "MFStartup",
            "MFCreateAttributes",
            "MFCreateSourceReaderFromURL",
            "IMFAttributes::SetUINT32",
            "IMFSample::AddBuffer",
            "IMFMediaBuffer::Lock",
        ] {
            assert!(r.proc_address(name) != 0, "{name} did not resolve");
        }
        assert!(!r.has_stub_dll(VTBL));
    }
}
