use goblin::pe::PE;
use std::collections::HashMap;

use crate::error::{Result, VmError};
use crate::fs::vfs::VirtualFileSystem;
use crate::logs::{LogBuffer, LogLevel};
use crate::vm::cpu::X86Cpu;
use crate::vm::handles::HandleTable;
use crate::vm::memory::{GuestMemory, PageProt};
use crate::vm::process::{ConsoleStreams, GuestProcess, ProcessState, UiEvent};
use crate::winapi::WinApiRegistry;

// Region where dependent DLLs are mapped (above the growable process heap).
// Each DLL is placed at the next free, section-aligned slot and base-relocated.
// Layout (low → high):
//   0x0040_0000  main PE image
//   0x1000_0000  process heap (grows up toward DLL_REGION_BASE) ≈ 1 GiB
//   0x5000_0000  loaded DLL images
//   0x6FF0_0000  stack
const DLL_REGION_BASE: u32 = 0x5000_0000;
const DLL_REGION_END: u32 = 0x6FE0_0000;

/// Extract the message-table resource (RT_MESSAGETABLE) into an id->text map.
/// cmd.exe and other system apps load their banner/messages/output templates
/// from here via FormatMessage(FROM_HMODULE).
pub fn extract_message_table(pe: &PE, bytes: &[u8]) -> std::collections::HashMap<u32, String> {
    use std::collections::HashMap;
    let mut out = HashMap::new();

    let Some(oh) = pe.header.optional_header else {
        return out;
    };
    let Some(res) = oh.data_directories.get_resource_table() else {
        return out;
    };
    if res.virtual_address == 0 {
        return out;
    }
    let rsrc_rva = res.virtual_address;

    let rva_to_off = |rva: u32| -> Option<usize> {
        for s in &pe.sections {
            let size = s.virtual_size.max(s.size_of_raw_data);
            if rva >= s.virtual_address && rva < s.virtual_address + size {
                return Some((s.pointer_to_raw_data + (rva - s.virtual_address)) as usize);
            }
        }
        None
    };
    let rd_u32 = |o: usize| -> u32 {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0)
    };
    let rd_u16 = |o: usize| -> u16 {
        bytes
            .get(o..o + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .unwrap_or(0)
    };

    // Walk a resource directory's entries, calling `f(id_or_name, offset, is_dir)`.
    let dir_entries = |dir_rva: u32| -> Vec<(u32, u32, bool)> {
        let mut v = Vec::new();
        let Some(base) = rva_to_off(dir_rva) else {
            return v;
        };
        let named = rd_u16(base + 12) as usize;
        let ids = rd_u16(base + 14) as usize;
        for i in 0..(named + ids) {
            let e = base + 16 + i * 8;
            let name = rd_u32(e);
            let off = rd_u32(e + 4);
            v.push((name, off & 0x7FFF_FFFF, off & 0x8000_0000 != 0));
        }
        v
    };

    // Level 1: find type == 11 (RT_MESSAGETABLE).
    let Some(type_entry) = dir_entries(rsrc_rva)
        .into_iter()
        .find(|(id, _, is_dir)| *id == 11 && *is_dir)
    else {
        return out;
    };
    // Level 2: name/id entries -> Level 3: language entries -> data entry.
    for (_, l2_off, l2_dir) in dir_entries(rsrc_rva + type_entry.1) {
        if !l2_dir {
            continue;
        }
        for (_, l3_off, l3_dir) in dir_entries(rsrc_rva + l2_off) {
            if l3_dir {
                continue;
            }
            // l3_off points to an IMAGE_RESOURCE_DATA_ENTRY (relative to rsrc base).
            let Some(de) = rva_to_off(rsrc_rva + l3_off) else {
                continue;
            };
            let data_rva = rd_u32(de);
            let Some(data_off) = rva_to_off(data_rva) else {
                continue;
            };
            parse_message_data(bytes, data_off, &mut out);
        }
    }
    out
}

/// RT_DIALOG = 5. Returns (id → template bytes, lowercase name → template bytes).
pub fn extract_dialogs(pe: &PE, bytes: &[u8]) -> (HashMap<u32, Vec<u8>>, HashMap<String, Vec<u8>>) {
    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    let Some(oh) = pe.header.optional_header else {
        return (by_id, by_name);
    };
    let Some(res) = oh.data_directories.get_resource_table() else {
        return (by_id, by_name);
    };
    if res.virtual_address == 0 {
        return (by_id, by_name);
    }
    let rsrc_rva = res.virtual_address;

    let rva_to_off = |rva: u32| -> Option<usize> {
        pe.sections.iter().find_map(|section| {
            let size = section.virtual_size.max(section.size_of_raw_data);
            (rva >= section.virtual_address && rva < section.virtual_address + size).then_some(
                (section.pointer_to_raw_data + rva - section.virtual_address) as usize,
            )
        })
    };
    let rd_u16 = |offset: usize| -> u16 {
        bytes
            .get(offset..offset + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .unwrap_or(0)
    };
    let rd_u32 = |offset: usize| -> u32 {
        bytes
            .get(offset..offset + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0)
    };
    let entries = |directory_rva: u32| -> Vec<(u32, u32, bool, bool)> {
        // (id_or_name_offset, target, is_dir, is_name)
        let Some(base) = rva_to_off(directory_rva) else {
            return Vec::new();
        };
        let n_names = rd_u16(base + 12) as usize;
        let n_ids = rd_u16(base + 14) as usize;
        let mut out = Vec::with_capacity(n_names + n_ids);
        for index in 0..(n_names + n_ids) {
            let entry = base + 16 + index * 8;
            let name_or_id = rd_u32(entry);
            let target = rd_u32(entry + 4);
            let is_name = name_or_id & 0x8000_0000 != 0;
            let id = name_or_id & 0x7FFF_FFFF;
            out.push((id, target & 0x7FFF_FFFF, target & 0x8000_0000 != 0, is_name));
        }
        out
    };
    let read_name = |name_off: u32| -> String {
        let Some(base) = rva_to_off(rsrc_rva + name_off) else {
            return String::new();
        };
        let len = rd_u16(base) as usize;
        let units: Vec<u16> = (0..len).map(|i| rd_u16(base + 2 + i * 2)).collect();
        String::from_utf16_lossy(&units)
    };

    // Type level: RT_DIALOG = 5
    let Some((_, type_off, true, _)) = entries(rsrc_rva)
        .into_iter()
        .find(|(id, _, is_dir, is_name)| !*is_name && *is_dir && *id == 5)
    else {
        return (by_id, by_name);
    };

    for (res_id, name_lang_off, is_dir, is_name) in entries(rsrc_rva + type_off) {
        if !is_dir {
            continue;
        }
        let name_key = if is_name {
            Some(read_name(res_id).to_ascii_lowercase())
        } else {
            None
        };
        let id_key = if is_name { None } else { Some(res_id) };

        // Language level: take first data entry
        let Some((_, data_off, false, _)) = entries(rsrc_rva + name_lang_off).into_iter().next()
        else {
            continue;
        };
        let Some(data_entry) = rva_to_off(rsrc_rva + data_off) else {
            continue;
        };
        let data_rva = rd_u32(data_entry);
        let data_size = rd_u32(data_entry + 4) as usize;
        let Some(data_file_off) = rva_to_off(data_rva) else {
            continue;
        };
        let blob = bytes
            .get(data_file_off..data_file_off + data_size)
            .unwrap_or(&[])
            .to_vec();
        if blob.is_empty() {
            continue;
        }
        if let Some(id) = id_key {
            by_id.insert(id, blob.clone());
        }
        if let Some(n) = name_key {
            by_name.insert(n, blob);
        }
    }
    (by_id, by_name)
}

/// Extract Win32 RT_STRING blocks. A block with resource id N contains sixteen
/// length-prefixed UTF-16 strings with ids (N-1)*16 through (N-1)*16+15.
pub fn extract_string_table(pe: &PE, bytes: &[u8]) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    let Some(oh) = pe.header.optional_header else { return out };
    let Some(res) = oh.data_directories.get_resource_table() else { return out };
    if res.virtual_address == 0 { return out }
    let rsrc_rva = res.virtual_address;

    let rva_to_off = |rva: u32| -> Option<usize> {
        pe.sections.iter().find_map(|section| {
            let size = section.virtual_size.max(section.size_of_raw_data);
            (rva >= section.virtual_address && rva < section.virtual_address + size)
                .then_some((section.pointer_to_raw_data + rva - section.virtual_address) as usize)
        })
    };
    let rd_u16 = |offset: usize| -> u16 {
        bytes.get(offset..offset + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0)
    };
    let rd_u32 = |offset: usize| -> u32 {
        bytes.get(offset..offset + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0)
    };
    let entries = |directory_rva: u32| -> Vec<(u32, u32, bool)> {
        let Some(base) = rva_to_off(directory_rva) else { return Vec::new() };
        let count = rd_u16(base + 12) as usize + rd_u16(base + 14) as usize;
        (0..count).map(|index| {
            let entry = base + 16 + index * 8;
            let target = rd_u32(entry + 4);
            (rd_u32(entry), target & 0x7FFF_FFFF, target & 0x8000_0000 != 0)
        }).collect()
    };

    let Some((_, type_offset, true)) = entries(rsrc_rva).into_iter().find(|entry| entry.0 == 6)
    else { return out };
    for (block_id, block_offset, is_directory) in entries(rsrc_rva + type_offset) {
        if !is_directory { continue }
        let Some((_, data_offset, false)) = entries(rsrc_rva + block_offset).into_iter().next()
        else { continue };
        let Some(data_entry) = rva_to_off(rsrc_rva + data_offset) else { continue };
        let Some(mut cursor) = rva_to_off(rd_u32(data_entry)) else { continue };
        for index in 0..16u32 {
            let length = rd_u16(cursor) as usize;
            cursor += 2;
            let units: Vec<u16> = (0..length).map(|i| rd_u16(cursor + i * 2)).collect();
            cursor += length * 2;
            if length != 0 {
                out.insert((block_id.saturating_sub(1)) * 16 + index, String::from_utf16_lossy(&units));
            }
        }
    }
    out
}

// Parse a MESSAGE_RESOURCE_DATA block at `base` into id->text.
fn parse_message_data(bytes: &[u8], base: usize, out: &mut std::collections::HashMap<u32, String>) {
    let rd_u32 = |o: usize| {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0)
    };
    let rd_u16 = |o: usize| {
        bytes
            .get(o..o + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .unwrap_or(0)
    };

    let n_blocks = rd_u32(base) as usize;
    for b in 0..n_blocks {
        let blk = base + 4 + b * 12;
        let low = rd_u32(blk);
        let high = rd_u32(blk + 4);
        let entries_off = rd_u32(blk + 8) as usize;
        let mut o = base + entries_off;
        for id in low..=high {
            let len = rd_u16(o) as usize;
            if len < 4 {
                break;
            }
            let flags = rd_u16(o + 2);
            let text_bytes = bytes.get(o + 4..o + len).unwrap_or(&[]);
            let text = if flags & 1 != 0 {
                let units: Vec<u16> = text_bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&units)
            } else {
                String::from_utf8_lossy(text_bytes).into_owned()
            };
            out.insert(id, text.trim_end_matches('\0').to_string());
            o += len;
        }
    }
}

// Fixed virtual address layout
const HEAP_BASE: u32 = 0x1000_0000;
// Initial reservation; ensure_mapped grows toward DLL_REGION_BASE (0x5000_0000)
// → up to ~1 GiB of guest heap for large games. 64 MB covers CRT/startup; big
// game asset loads grow the region in 1 MB steps without pre-committing 1 GiB
// of host RAM in the browser.
const HEAP_SIZE: u32 = 0x0400_0000; // 64 MB
const STACK_BASE: u32 = 0x6FF0_0000;
const STACK_SIZE: u32 = 0x0010_0000; // 1 MB
const STACK_TOP: u32 = STACK_BASE + STACK_SIZE;
const PEB_VA: u32 = 0x7FFD_F000;
pub const TEB_VA: u32 = 0x7FFD_E000;
const TRAMP_REGION: u32 = 0x7FFE_0000;
const TRAMP_REGION_SIZE: u32 = 0x0001_0000;

// Sub-regions inside the PEB page (4096 bytes starting at PEB_VA).
// We reuse the same physical allocation to avoid extra memory::allocate calls.
const TLS_ARRAY_VA: u32 = PEB_VA + 0x600; // 128 TLS slots (512 bytes)
const PROC_PARAMS_VA: u32 = PEB_VA + 0x800; // RTL_USER_PROCESS_PARAMETERS stub

// Per-module thread-local storage block for the main thread. The CRT reads
// TLS via `mov ecx, fs:[0x2C]; mov edx, [ecx + __tls_index*4]` and dereferences
// the result, so slot 0 must point at a real, populated block.
const TLS_DATA_VA: u32 = 0x7FFD_0000;
const TLS_DATA_SIZE: u32 = 0x0000_E000; // 56 KB, sits below the TEB at 0x7FFD_E000

pub fn load_pe(
    bytes: &[u8],
    path: &str,
    cmdline: &str,
    pid: u32,
    api: &mut WinApiRegistry,
    fs: &VirtualFileSystem,
    logs: &mut LogBuffer,
) -> Result<GuestProcess> {
    let pe = crate::pe::parse_pe(bytes).map_err(|e| VmError::Pe(e.to_string()))?;

    // We are an x86 (i386) user-mode interpreter. Reject anything else up front
    // with a clear message — otherwise a 64-bit image would be decoded as 32-bit
    // garbage and crash mysteriously (its image base also truncates to 32 bits).
    let machine = pe.header.coff_header.machine;
    if machine != 0x014C {
        let arch = match machine {
            0x8664 => "x86-64 (64-bit)",
            0x01C0 | 0x01C4 => "ARM",
            0xAA64 => "ARM64",
            _ => "non-x86",
        };
        return Err(VmError::Unsupported(format!(
            "{arch} executable — WebWINE only runs 32-bit x86 (i386) PE32 images. \
             Most modern Windows system binaries (calc, cmd, mspaint) are 64-bit; \
             use a 32-bit build."
        )));
    }

    let oh = pe
        .header
        .optional_header
        .ok_or_else(|| VmError::NotPe("no optional header".into()))?;

    // .NET / managed image? The CLR header (data directory 14) means the entry
    // just bounces into mscoree!_CorExeMain to start the runtime. We can't host
    // the .NET CLR, so reject clearly instead of crashing in the stub.
    if let Some(clr) = oh.data_directories.get_clr_runtime_header() {
        if clr.virtual_address != 0 {
            return Err(VmError::Unsupported(
                ".NET (managed/CLR) executable — This function only loads native Win32 PE32 images. \
                 Callers should route this to the CLR engine (run_managed) instead."
                    .into(),
            ));
        }
    }

    let image_base = oh.windows_fields.image_base as u32;
    let image_size = oh.windows_fields.size_of_image;
    let hdr_size = oh.windows_fields.size_of_headers;
    let entry_rva = oh.standard_fields.address_of_entry_point as u32;
    let entry_point = image_base + entry_rva;

    logs.log(
        LogLevel::Info,
        "loader",
        &format!("[loader] loading {path}"),
        None,
    );
    logs.log(
        LogLevel::Info,
        "loader",
        &format!(
            "[pe] image_base=0x{image_base:08X}  size=0x{image_size:X}  entry=0x{entry_point:08X}"
        ),
        None,
    );

    let mut mem = GuestMemory::new();

    // image region
    mem.allocate(image_base, image_size, PageProt::RWX)?;

    // map PE headers
    let hdr_bytes = &bytes[..hdr_size.min(bytes.len() as u32) as usize];
    mem.write_bytes(image_base, hdr_bytes)?;
    logs.log(
        LogLevel::Debug,
        "loader",
        &format!("[loader] mapped headers ({} bytes)", hdr_bytes.len()),
        None,
    );

    // map sections
    for section in &pe.sections {
        let name = std::str::from_utf8(&section.name)
            .unwrap_or("?")
            .trim_end_matches('\0');

        let va = image_base + section.virtual_address;
        let roff = section.pointer_to_raw_data as usize;
        let rsz = section.size_of_raw_data as usize;
        let vsz = section.virtual_size as usize;

        if rsz > 0 && roff < bytes.len() {
            let end = (roff + rsz).min(bytes.len());
            let src = &bytes[roff..end];
            // Copy the full raw data, NOT min(rsz, vsz). Windows maps
            // SizeOfRawData bytes into the section's reserved virtual extent
            // (VirtualSize rounded up to section alignment, always large
            // enough). When VirtualSize < SizeOfRawData the trailing raw bytes
            // still contain real code/data the program references -- e.g.
            // Touhou 6's .text has vsz=0x68A5F < rsz=0x69000 and jumps into
            // the gap at 0x469AA0. Truncating to vsz zero-filled that code and
            // crashed. (When vsz > rsz this is a BSS tail; leaving it zeroed is
            // correct since guest memory is zero-initialised.)
            mem.write_bytes(va, src)?;
        }
        logs.log(
            LogLevel::Debug,
            "loader",
            &format!("[pe] section {name:<8} va=0x{va:08X} vsz=0x{vsz:X} rsz=0x{rsz:X}"),
            None,
        );
    }

    // relocations — only needed if we loaded at a different base.
    // We load at the preferred base, so none are required.

    // Resolve imports by walking the import directory ourselves and patching the
    // FirstThunk (IAT) — the table the CPU actually calls through. goblin's
    // high-level `imports` reports OriginalFirstThunk (ILT) RVAs, which are NOT
    // what `call dword ptr [iat]` reads. App/third-party DLLs found in the search
    // path (exe dir, System32, Windows) are mapped and base-relocated here so the
    // IAT points at their real code; genuinely-missing DLLs produce a warning.
    let mut mctx = ModuleCtx::new(fs, crate::vm::process::parent_dir(path));
    let import_count = resolve_imports(&pe, image_base, &mut mctx, &mut mem, api, logs)?;
    let load_warnings = std::mem::take(&mut mctx.warnings);
    logs.log(
        LogLevel::Info,
        "loader",
        &format!("[loader] resolved {import_count} imports"),
        None,
    );

    // allocate trampoline region (so the memory map is clean)
    if mem
        .allocate(TRAMP_REGION, TRAMP_REGION_SIZE, PageProt::RX)
        .is_err()
    {
        // already mapped (second process) — fine
    }

    // stack
    mem.allocate(STACK_BASE, STACK_SIZE, PageProt::RW)?;
    logs.log(
        LogLevel::Debug,
        "loader",
        &format!("[loader] stack 0x{STACK_BASE:08X}..0x{STACK_TOP:08X}"),
        None,
    );

    // heap
    mem.allocate(HEAP_BASE, HEAP_SIZE, PageProt::RW)?;
    logs.log(
        LogLevel::Debug,
        "loader",
        &format!("[loader] heap  0x{HEAP_BASE:08X}+0x{HEAP_SIZE:X}"),
        None,
    );

    // PEB — one page holds PEB proper + TLS array + process-parameters stub
    mem.allocate(PEB_VA, 0x1000, PageProt::RW)?;
    mem.write_u32(PEB_VA + 0x04, 0)?; // Mutant = none
    mem.write_u32(PEB_VA + 0x08, image_base)?; // ImageBaseAddress
    mem.write_u32(PEB_VA + 0x0C, 0)?; // Ldr = null (tolerated for simple progs)
    mem.write_u32(PEB_VA + 0x10, PROC_PARAMS_VA)?; // ProcessParameters
    mem.write_u32(PEB_VA + 0x1C, HEAP_BASE)?; // ProcessHeap
    mem.write_u32(PEB_VA + 0x18, 0)?; // SubSystemData
    mem.write_u32(PEB_VA + 0x68, 0)?; // NtGlobalFlag = 0 (not debugging)

    // Imported CRT globals must be backed by data before guest startup. This
    // also builds the narrow/wide command lines and argv arrays.
    webwine_api_winapi::msvcrt::initialize_process_data(&mut mem, cmdline)?;

    // Minimal RTL_USER_PROCESS_PARAMETERS at PROC_PARAMS_VA
    // Just enough for the CRT not to dereference null for I/O handles & command line.
    let cmd_buf_va = mem
        .read_u32(webwine_api_winapi::msvcrt::CRT_WCMDLN_SLOT)
        .unwrap_or(0);
    let cmd_len_bytes = cmdline
        .encode_utf16()
        .count()
        .saturating_mul(2)
        .min(u16::MAX as usize) as u16;
    mem.write_u32(PROC_PARAMS_VA + 0x00, 0x200)?; // MaximumLength
    mem.write_u32(PROC_PARAMS_VA + 0x04, 0x200)?; // Length
    mem.write_u32(PROC_PARAMS_VA + 0x18, 0xFFFF_FFF6)?; // StandardInput  = STDIN handle
    mem.write_u32(PROC_PARAMS_VA + 0x1C, 0xFFFF_FFF5)?; // StandardOutput = STDOUT handle
    mem.write_u32(PROC_PARAMS_VA + 0x20, 0xFFFF_FFF4)?; // StandardError  = STDERR handle
                                                        // CommandLine UNICODE_STRING at +0x40
    mem.write_u16(PROC_PARAMS_VA + 0x40, cmd_len_bytes)?; // Length (bytes)
    mem.write_u16(PROC_PARAMS_VA + 0x42, cmd_len_bytes.saturating_add(2))?; // MaximumLength
    mem.write_u32(PROC_PARAMS_VA + 0x44, cmd_buf_va)?; // Buffer

    // TLS slot array inside the PEB page
    // TLS_ARRAY_VA: 128 null pointers (each slot initialised on first TlsSetValue)

    // TEB — one page
    mem.allocate(TEB_VA, 0x1000, PageProt::RW)?;
    mem.write_u32(TEB_VA + 0x00, 0xFFFF_FFFF)?; // ExceptionList = end sentinel
    mem.write_u32(TEB_VA + 0x04, STACK_TOP)?; // StackBase (highest address)
    mem.write_u32(TEB_VA + 0x08, STACK_BASE)?; // StackLimit
    mem.write_u32(TEB_VA + 0x18, TEB_VA)?; // Self pointer
    mem.write_u32(TEB_VA + 0x20, pid)?; // ClientId.UniqueProcess (pid)
    mem.write_u32(TEB_VA + 0x24, pid * 100)?; // ClientId.UniqueThread
    mem.write_u32(TEB_VA + 0x2C, TLS_ARRAY_VA)?; // ThreadLocalStoragePointer
    mem.write_u32(TEB_VA + 0x30, PEB_VA)?; // ProcessEnvironmentBlock
    mem.write_u32(TEB_VA + 0x34, 0)?; // LastErrorValue = 0

    // Thread-local storage block. Allocate, point slot 0 at it, then copy the
    // PE's TLS template (if any) and force the module's TLS index to 0.
    mem.allocate(TLS_DATA_VA, TLS_DATA_SIZE, PageProt::RW)?;
    mem.write_u32(TLS_ARRAY_VA, TLS_DATA_VA)?;
    if let Some(tls) = oh.data_directories.get_tls_table() {
        if tls.virtual_address != 0 {
            let d = image_base + tls.virtual_address;
            let raw_start = mem.read_u32(d).unwrap_or(0); // StartAddressOfRawData (VA)
            let raw_end = mem.read_u32(d + 4).unwrap_or(0); // EndAddressOfRawData (VA)
            let idx_addr = mem.read_u32(d + 8).unwrap_or(0); // AddressOfIndex (VA)
            let raw_size = raw_end.saturating_sub(raw_start);
            if raw_size > 0 && raw_size <= TLS_DATA_SIZE {
                if let Ok(template) = mem.read_bytes(raw_start, raw_size as usize) {
                    mem.write_bytes(TLS_DATA_VA, &template)?;
                }
            }
            if idx_addr != 0 {
                let _ = mem.write_u32(idx_addr, 0); // this module's TLS index = slot 0
            }
            logs.log(
                LogLevel::Debug,
                "loader",
                &format!("[loader] TLS block 0x{TLS_DATA_VA:08X} template={raw_size}B"),
                None,
            );
        }
    }

    // CPU initial state
    let mut cpu = X86Cpu::new();
    cpu.eip = entry_point;
    cpu.esp = STACK_TOP - 16;

    // Some CRTs expect argc, argv, envp on the stack
    // char* envp[] = { NULL }
    cpu.esp -= 4;
    mem.write_u32(cpu.esp, 0)?;
    let envp = cpu.esp;

    // char* argv[] = { "program.exe", NULL }
    cpu.esp -= 16;
    let argv_str = cpu.esp;
    mem.write_bytes(argv_str, b"program.exe\0")?;
    cpu.esp -= 8;
    mem.write_u32(cpu.esp + 4, 0)?; // NULL
    mem.write_u32(cpu.esp, argv_str)?;
    let argv = cpu.esp;

    // push envp, argv, argc
    cpu.esp -= 12;
    mem.write_u32(cpu.esp + 8, envp)?;
    mem.write_u32(cpu.esp + 4, argv)?;
    mem.write_u32(cpu.esp, 1)?; // argc = 1

    // push zero return address
    cpu.esp -= 4;
    mem.write_u32(cpu.esp, 0)?;

    logs.log(
        LogLevel::Info,
        "loader",
        &format!(
            "[loader] process created — PID {pid}  EIP=0x{entry_point:08X}  ESP=0x{:08X}",
            cpu.esp
        ),
        None,
    );

    logs.log(
        LogLevel::Info,
        "loader",
        &format!("[loader] PEB=0x{PEB_VA:08X}  TEB=0x{TEB_VA:08X}"),
        None,
    );

    let (dialogs, dialogs_by_name) = extract_dialogs(&pe, bytes);

    Ok(GuestProcess {
        pid,
        path: path.to_string(),
        image_base,
        entry_point,
        heap_base: HEAP_BASE,
        heap_next: HEAP_BASE,
        heap_sizes: std::collections::HashMap::new(),
        heap_free_list: Vec::new(),
        // Bump heap may grow up to the DLL region (≈1 GiB of guest VA).
        heap_limit: DLL_REGION_BASE,
        memory: mem,
        cpu,
        handles: HandleTable::new(pid),
        console: ConsoleStreams::new(),
        // Surface any missing-DLL warnings as a dialog on the first slice; the
        // process still runs (with stub fallbacks) per the warn-and-continue policy.
        ui_events: if load_warnings.is_empty() {
            Vec::new()
        } else {
            vec![UiEvent::MessageBox {
                title: "WebWINE".to_string(),
                text: format!(
                    "The program may not run correctly:\n\n{}",
                    load_warnings.join("\n")
                ),
                style: 0x30, // MB_OK | MB_ICONWARNING
            }]
        },
        gui: crate::vm::process::GuiState::new(),
        spawns: Vec::new(),
        next_child_pid: 0,
        state: ProcessState::Created,
        cwd: crate::vm::process::parent_dir(path),
        cmdline: cmdline.to_string(),
        messages: extract_message_table(&pe, bytes),
        strings: extract_string_table(&pe, bytes),
        dialogs,
        dialogs_by_name,
        managed: None,
        tls_slots: std::collections::HashMap::new(),
        next_tls: 1,
        rand_seed: 1,
        dll_state: std::collections::HashMap::new(),
    })
}

/// Exports of a loaded dependent DLL: function name / ordinal -> resolved VA.
struct DllExports {
    by_name: HashMap<String, u32>,
    by_ord: HashMap<u32, u32>,
}

/// State carried while resolving a module tree (the exe plus its dependent DLLs).
struct ModuleCtx<'a> {
    fs: &'a VirtualFileSystem,
    exe_dir: String,
    next_dll_base: u32,
    /// Upper-cased DLL name -> its exports. `None` marks a DLL currently being
    /// loaded (cycle guard) or one that failed to load.
    loaded: HashMap<String, Option<DllExports>>,
    warnings: Vec<String>,
}

impl<'a> ModuleCtx<'a> {
    fn new(fs: &'a VirtualFileSystem, exe_dir: String) -> Self {
        ModuleCtx {
            fs,
            exe_dir,
            next_dll_base: DLL_REGION_BASE,
            loaded: HashMap::new(),
            warnings: Vec::new(),
        }
    }
}

/// Read a NUL-terminated ASCII name from a section's raw bytes via its RVA.
fn read_cstr_at(mem: &GuestMemory, va: u32) -> String {
    mem.read_cstr(va)
}

/// Locate a DLL file by Windows search order: the exe's own directory, then
/// C:\Windows\System32, then C:\Windows. Returns the guest path if it exists.
fn find_dll_file(fs: &VirtualFileSystem, exe_dir: &str, dll: &str) -> Option<String> {
    let dir = exe_dir.trim_end_matches('\\');
    let candidates = [
        format!("{dir}\\{dll}"),
        format!("C:\\Windows\\System32\\{dll}"),
        format!("C:\\Windows\\{dll}"),
    ];
    candidates.into_iter().find(|p| fs.node_exists(p))
}

/// Walk a PE's import directory (the exe or a dependent DLL) and patch each
/// FirstThunk (IAT) slot. App/third-party DLLs found in the search path are
/// loaded for real and their exports wired in; system DLLs route to trampolines;
/// genuinely-missing DLLs (and missing entry points) record a warning and fall
/// back to a trampoline so execution can continue.
fn resolve_imports(
    pe: &PE,
    image_base: u32,
    mctx: &mut ModuleCtx,
    mem: &mut GuestMemory,
    api: &mut WinApiRegistry,
    logs: &mut LogBuffer,
) -> Result<usize> {
    let oh = match pe.header.optional_header {
        Some(oh) => oh,
        None => return Ok(0),
    };
    let import_dir = match oh.data_directories.get_import_table() {
        Some(d) if d.virtual_address != 0 => *d,
        _ => return Ok(0),
    };

    const DESC_SIZE: u32 = 20; // IMAGE_IMPORT_DESCRIPTOR
    let mut count = 0usize;
    let mut desc_va = image_base + import_dir.virtual_address;
    let desc_end = desc_va + import_dir.size;

    loop {
        if desc_va + DESC_SIZE > desc_end {
            break;
        }
        let oft = mem.read_u32(desc_va).unwrap_or(0); // OriginalFirstThunk (ILT)
        let name = mem.read_u32(desc_va + 12).unwrap_or(0); // DLL name RVA
        let ft = mem.read_u32(desc_va + 16).unwrap_or(0); // FirstThunk (IAT)

        if oft == 0 && name == 0 && ft == 0 {
            break; // null descriptor terminates the array
        }

        let dll = read_cstr_at(mem, image_base + name);

        // Decide where this DLL's exports come from. DLLs we stub always use our
        // built-in handlers. Everything else is looked up as a real file.
        let stub = api.has_stub_dll(&dll);
        let from_file = if stub {
            false
        } else {
            ensure_dll_loaded(&dll, mctx, mem, api, logs, 0)
        };

        let lookup_rva = if oft != 0 { oft } else { ft };
        let mut i = 0u32;
        let mut any_resolved = false; // a real export or an implemented stub
        loop {
            let thunk = mem.read_u32(image_base + lookup_rva + i * 4).unwrap_or(0);
            if thunk == 0 {
                break;
            }
            let (func_name, ordinal) = if thunk & 0x8000_0000 != 0 {
                (format!("#{}", thunk & 0xFFFF), Some(thunk & 0xFFFF))
            } else {
                (mem.read_cstr(image_base + (thunk & 0x7FFF_FFFF) + 2), None)
            };

            // Prefer a real export VA from a loaded file; otherwise a trampoline.
            let mut addr = 0u32;
            if from_file {
                if let Some(Some(exp)) = mctx.loaded.get(&dll.to_ascii_uppercase()) {
                    addr = ordinal
                        .and_then(|o| exp.by_ord.get(&o).copied())
                        .or_else(|| exp.by_name.get(&func_name).copied())
                        .unwrap_or(0);
                }
                any_resolved |= addr != 0;
            } else if stub || api.is_implemented(&dll, &func_name) {
                any_resolved = true;
            }
            if addr == 0 {
                if let Some(data) = api.data_address(&dll, &func_name) {
                    addr = data;
                    any_resolved = true;
                }
            }
            if addr == 0 {
                addr = api.resolve_trampoline(&dll, &func_name);
            }

            let iat_slot = image_base + ft + i * 4;
            if mem.write_u32(iat_slot, addr).is_ok() {
                count += 1;
            }
            i += 1;
        }

        // Warn (once) only for a DLL we couldn't satisfy at all: not stubbed, no
        // file loaded, and not one function resolved. Apisets and partially-
        // implemented system DLLs resolve enough to stay quiet. Per the
        // warn-and-continue policy the IAT still has trampolines.
        if !stub && !from_file && i > 0 && !any_resolved {
            let msg = format!("'{dll}' was not found");
            if !mctx.warnings.contains(&msg) {
                mctx.warnings.push(msg);
            }
        }

        logs.log(
            LogLevel::Debug,
            "loader",
            &format!(
                "[loader] {dll}: {i} imports patched{}",
                if from_file { " (real DLL)" } else { "" }
            ),
            None,
        );

        desc_va += DESC_SIZE;
    }

    Ok(count)
}

/// Map, relocate and link a dependent DLL (recursively resolving its own
/// imports), recording its exports in `mctx.loaded`. Returns true if the DLL is
/// available as a real file and was (or already is) loaded. `depth` guards
/// against runaway dependency chains.
fn ensure_dll_loaded(
    dll: &str,
    mctx: &mut ModuleCtx,
    mem: &mut GuestMemory,
    api: &mut WinApiRegistry,
    logs: &mut LogBuffer,
    depth: u32,
) -> bool {
    let key = dll.to_ascii_uppercase();
    if let Some(slot) = mctx.loaded.get(&key) {
        return slot.is_some(); // already loaded (Some) or in-progress/failed (None)
    }
    if depth > 16 {
        return false;
    }

    let Some(path) = find_dll_file(mctx.fs, &mctx.exe_dir, dll) else {
        return false; // no file -> caller treats as missing/stub
    };
    let Ok(bytes) = mctx.fs.read_file(&path) else {
        return false;
    };

    // Mark in-progress so a circular import returns "loaded" instead of recursing.
    mctx.loaded.insert(key.clone(), None);

    let pe = match crate::pe::parse_pe(&bytes) {
        Ok(pe) => pe,
        Err(e) => {
            mctx.warnings.push(format!("{dll}: not a valid PE ({e})"));
            return false;
        }
    };
    if pe.header.coff_header.machine != 0x014C {
        mctx.warnings.push(format!("{dll}: not a 32-bit (x86) DLL"));
        return false;
    }
    let Some(oh) = pe.header.optional_header else {
        return false;
    };
    let preferred = oh.windows_fields.image_base as u32;
    let image_size = oh.windows_fields.size_of_image;
    let hdr_size = oh.windows_fields.size_of_headers;

    // Place the DLL at the next free, page-aligned slot in the DLL region.
    let base = (mctx.next_dll_base + 0xFFF) & !0xFFF;
    let span = (image_size + 0xFFF) & !0xFFF;
    if base
        .checked_add(span)
        .map(|e| e > DLL_REGION_END)
        .unwrap_or(true)
    {
        mctx.warnings
            .push(format!("{dll}: no room to map ({image_size} bytes)"));
        return false;
    }
    if mem.allocate(base, span, PageProt::RWX).is_err() {
        mctx.warnings
            .push(format!("{dll}: could not map at 0x{base:08X}"));
        return false;
    }
    mctx.next_dll_base = base + span;

    // Headers + sections (copy SizeOfRawData, same rule as the main image).
    let _ = mem.write_bytes(base, &bytes[..hdr_size.min(bytes.len() as u32) as usize]);
    for s in &pe.sections {
        let roff = s.pointer_to_raw_data as usize;
        let rsz = s.size_of_raw_data as usize;
        if rsz > 0 && roff < bytes.len() {
            let end = (roff + rsz).min(bytes.len());
            let _ = mem.write_bytes(base + s.virtual_address, &bytes[roff..end]);
        }
    }

    apply_relocations(&pe, base, preferred, mem);
    let exports = parse_exports(&pe, base, mem);
    mctx.loaded.insert(key.clone(), Some(exports));

    logs.log(
        LogLevel::Info,
        "loader",
        &format!("[loader] loaded DLL {dll} at 0x{base:08X} (preferred 0x{preferred:08X})"),
        None,
    );

    // Resolve the DLL's own imports now that it is mapped.
    let _ = resolve_imports_recursive(&pe, base, mctx, mem, api, logs, depth + 1);
    true
}

/// Like `resolve_imports` but reached from a dependent DLL, so nested file DLLs
/// recurse with an incremented depth.
fn resolve_imports_recursive(
    pe: &PE,
    image_base: u32,
    mctx: &mut ModuleCtx,
    mem: &mut GuestMemory,
    api: &mut WinApiRegistry,
    logs: &mut LogBuffer,
    depth: u32,
) -> Result<usize> {
    let oh = match pe.header.optional_header {
        Some(oh) => oh,
        None => return Ok(0),
    };
    let import_dir = match oh.data_directories.get_import_table() {
        Some(d) if d.virtual_address != 0 => *d,
        _ => return Ok(0),
    };
    const DESC_SIZE: u32 = 20;
    let mut desc_va = image_base + import_dir.virtual_address;
    let desc_end = desc_va + import_dir.size;
    loop {
        if desc_va + DESC_SIZE > desc_end {
            break;
        }
        let oft = mem.read_u32(desc_va).unwrap_or(0);
        let name = mem.read_u32(desc_va + 12).unwrap_or(0);
        let ft = mem.read_u32(desc_va + 16).unwrap_or(0);
        if oft == 0 && name == 0 && ft == 0 {
            break;
        }
        let dll = read_cstr_at(mem, image_base + name);
        let from_file = if api.has_stub_dll(&dll) {
            false
        } else {
            ensure_dll_loaded(&dll, mctx, mem, api, logs, depth)
        };
        let lookup_rva = if oft != 0 { oft } else { ft };
        let mut i = 0u32;
        loop {
            let thunk = mem.read_u32(image_base + lookup_rva + i * 4).unwrap_or(0);
            if thunk == 0 {
                break;
            }
            let (func_name, ordinal) = if thunk & 0x8000_0000 != 0 {
                (format!("#{}", thunk & 0xFFFF), Some(thunk & 0xFFFF))
            } else {
                (mem.read_cstr(image_base + (thunk & 0x7FFF_FFFF) + 2), None)
            };
            let mut addr = 0u32;
            if from_file {
                if let Some(Some(exp)) = mctx.loaded.get(&dll.to_ascii_uppercase()) {
                    addr = ordinal
                        .and_then(|o| exp.by_ord.get(&o).copied())
                        .or_else(|| exp.by_name.get(&func_name).copied())
                        .unwrap_or(0);
                }
            }
            if addr == 0 {
                if let Some(data) = api.data_address(&dll, &func_name) {
                    addr = data;
                }
            }
            if addr == 0 {
                addr = api.resolve_trampoline(&dll, &func_name);
            }
            let _ = mem.write_u32(image_base + ft + i * 4, addr);
            i += 1;
        }
        desc_va += DESC_SIZE;
    }
    Ok(0)
}

/// Apply base relocations to a freshly-mapped image. delta = actual - preferred.
fn apply_relocations(pe: &PE, base: u32, preferred: u32, mem: &mut GuestMemory) {
    let delta = base.wrapping_sub(preferred);
    if delta == 0 {
        return;
    }
    let Some(oh) = pe.header.optional_header else {
        return;
    };
    let reloc = match oh.data_directories.get_base_relocation_table() {
        Some(d) if d.virtual_address != 0 => *d,
        _ => return,
    };
    let mut p = base + reloc.virtual_address;
    let end = p + reloc.size;
    while p + 8 <= end {
        let page_rva = mem.read_u32(p).unwrap_or(0);
        let block_size = mem.read_u32(p + 4).unwrap_or(0);
        if block_size < 8 {
            break;
        }
        let entries = (block_size - 8) / 2;
        for k in 0..entries {
            let e = mem.read_u16(p + 8 + k * 2).unwrap_or(0);
            let typ = e >> 12;
            let off = (e & 0x0FFF) as u32;
            if typ == 3 {
                // IMAGE_REL_BASED_HIGHLOW: patch a 32-bit address in place.
                let at = base + page_rva + off;
                let v = mem.read_u32(at).unwrap_or(0);
                let _ = mem.write_u32(at, v.wrapping_add(delta));
            }
            // type 0 (ABSOLUTE) is padding; other types are rare on x86.
        }
        p += block_size;
    }
}

/// Parse a mapped DLL's export directory into name/ordinal -> VA maps. Forwarded
/// exports (RVA inside the export dir) are skipped here; they resolve to a stub
/// trampoline at import time instead.
fn parse_exports(pe: &PE, base: u32, mem: &GuestMemory) -> DllExports {
    let mut out = DllExports {
        by_name: HashMap::new(),
        by_ord: HashMap::new(),
    };
    let Some(oh) = pe.header.optional_header else {
        return out;
    };
    let dir = match oh.data_directories.get_export_table() {
        Some(d) if d.virtual_address != 0 => *d,
        _ => return out,
    };
    let exp = base + dir.virtual_address;
    let exp_end = exp + dir.size;
    let ord_base = mem.read_u32(exp + 16).unwrap_or(0);
    let num_funcs = mem.read_u32(exp + 20).unwrap_or(0);
    let num_names = mem.read_u32(exp + 24).unwrap_or(0);
    let funcs_rva = mem.read_u32(exp + 28).unwrap_or(0);
    let names_rva = mem.read_u32(exp + 32).unwrap_or(0);
    let ords_rva = mem.read_u32(exp + 36).unwrap_or(0);

    let is_forwarder = |va: u32| va >= exp && va < exp_end;

    // Ordinal -> VA (skip empty and forwarded slots).
    for i in 0..num_funcs.min(0x10000) {
        let frva = mem.read_u32(base + funcs_rva + i * 4).unwrap_or(0);
        if frva == 0 {
            continue;
        }
        let va = base + frva;
        if is_forwarder(va) {
            continue;
        }
        out.by_ord.insert(ord_base + i, va);
    }
    // Name -> VA via the name-ordinal table.
    for i in 0..num_names.min(0x10000) {
        let name_rva = mem.read_u32(base + names_rva + i * 4).unwrap_or(0);
        if name_rva == 0 {
            continue;
        }
        let name = mem.read_cstr(base + name_rva);
        let oi = mem.read_u16(base + ords_rva + i * 2).unwrap_or(0) as u32;
        let frva = mem.read_u32(base + funcs_rva + oi * 4).unwrap_or(0);
        if frva == 0 {
            continue;
        }
        let va = base + frva;
        if is_forwarder(va) {
            continue;
        }
        out.by_name.insert(name, va);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::LogBuffer;
    use crate::winapi::WinApiRegistry;

    #[test]
    fn loads_hello_world_sample() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/target/i686-pc-windows-msvc/debug/hello_world.exe"
        );
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };

        let mut api = WinApiRegistry::new();
        let mut logs = LogBuffer::default();
        let fs = VirtualFileSystem::new();

        let proc = load_pe(
            &bytes,
            path,
            &format!("\"{path}\""),
            1,
            &mut api,
            &fs,
            &mut logs,
        )
        .expect("load PE");

        assert_eq!(proc.pid, 1);
        assert!(proc.image_base > 0);
        assert!(proc.entry_point > proc.image_base);
        assert!(proc.cpu.eip == proc.entry_point);
        assert!(proc.cpu.esp > 0);

        // CPU state should be Created
        assert!(matches!(proc.state, ProcessState::Created));

        // At least one import should have been resolved
        assert!(!api.is_trampoline(0), "addr 0 should not be a trampoline");
    }

    #[test]
    fn extracts_vendored_win32_string_resources() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/vendored/Notepad/notepad-nt.exe"
        );
        let Ok(bytes) = std::fs::read(path) else { return };
        let pe = PE::parse(&bytes).expect("parse native notepad");
        let strings = extract_string_table(&pe, &bytes);
        assert!(!strings.is_empty(), "expected RT_STRING resources");
    }
}
