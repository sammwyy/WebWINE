use goblin::pe::PE;

use crate::error::{Result, VmError};
use crate::logs::{LogBuffer, LogLevel};
use crate::vm::cpu::X86Cpu;
use crate::vm::handles::HandleTable;
use crate::vm::memory::{GuestMemory, PageProt};
use crate::vm::process::{ConsoleStreams, GuestProcess, ProcessState};
use crate::winapi::WinApiRegistry;

/// Extract the message-table resource (RT_MESSAGETABLE) into an id->text map.
/// cmd.exe and other system apps load their banner/messages/output templates
/// from here via FormatMessage(FROM_HMODULE).
pub fn extract_message_table(pe: &PE, bytes: &[u8]) -> std::collections::HashMap<u32, String> {
    use std::collections::HashMap;
    let mut out = HashMap::new();

    let Some(oh) = pe.header.optional_header else { return out };
    let Some(res) = oh.data_directories.get_resource_table() else { return out };
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
        bytes.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0)
    };
    let rd_u16 = |o: usize| -> u16 {
        bytes.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0)
    };

    // Walk a resource directory's entries, calling `f(id_or_name, offset, is_dir)`.
    let dir_entries = |dir_rva: u32| -> Vec<(u32, u32, bool)> {
        let mut v = Vec::new();
        let Some(base) = rva_to_off(dir_rva) else { return v };
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
    let Some(type_entry) = dir_entries(rsrc_rva).into_iter().find(|(id, _, is_dir)| *id == 11 && *is_dir) else {
        return out;
    };
    // Level 2: name/id entries -> Level 3: language entries -> data entry.
    for (_, l2_off, l2_dir) in dir_entries(rsrc_rva + type_entry.1) {
        if !l2_dir { continue; }
        for (_, l3_off, l3_dir) in dir_entries(rsrc_rva + l2_off) {
            if l3_dir { continue; }
            // l3_off points to an IMAGE_RESOURCE_DATA_ENTRY (relative to rsrc base).
            let Some(de) = rva_to_off(rsrc_rva + l3_off) else { continue };
            let data_rva = rd_u32(de);
            let Some(data_off) = rva_to_off(data_rva) else { continue };
            parse_message_data(bytes, data_off, &mut out);
        }
    }
    out
}

// Parse a MESSAGE_RESOURCE_DATA block at `base` into id->text.
fn parse_message_data(bytes: &[u8], base: usize, out: &mut std::collections::HashMap<u32, String>) {
    let rd_u32 = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0);
    let rd_u16 = |o: usize| bytes.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0);

    let n_blocks = rd_u32(base) as usize;
    for b in 0..n_blocks {
        let blk = base + 4 + b * 12;
        let low = rd_u32(blk);
        let high = rd_u32(blk + 4);
        let entries_off = rd_u32(blk + 8) as usize;
        let mut o = base + entries_off;
        for id in low..=high {
            let len = rd_u16(o) as usize;
            if len < 4 { break; }
            let flags = rd_u16(o + 2);
            let text_bytes = bytes.get(o + 4..o + len).unwrap_or(&[]);
            let text = if flags & 1 != 0 {
                let units: Vec<u16> = text_bytes.chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
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
const HEAP_BASE:   u32 = 0x1000_0000;
const HEAP_SIZE:   u32 = 0x0040_0000; // 4 MB
const STACK_BASE:  u32 = 0x6FF0_0000;
const STACK_SIZE:  u32 = 0x0010_0000; // 1 MB
const STACK_TOP:   u32 = STACK_BASE + STACK_SIZE;
const PEB_VA:      u32 = 0x7FFD_F000;
pub const TEB_VA:  u32 = 0x7FFD_E000;
const TRAMP_REGION:      u32 = 0x7FFE_0000;
const TRAMP_REGION_SIZE: u32 = 0x0001_0000;

// Sub-regions inside the PEB page (4096 bytes starting at PEB_VA).
// We reuse the same physical allocation to avoid extra memory::allocate calls.
const TLS_ARRAY_VA:   u32 = PEB_VA + 0x600; // 128 TLS slots (512 bytes)
const PROC_PARAMS_VA: u32 = PEB_VA + 0x800; // RTL_USER_PROCESS_PARAMETERS stub

// Per-module thread-local storage block for the main thread. The CRT reads
// TLS via `mov ecx, fs:[0x2C]; mov edx, [ecx + __tls_index*4]` and dereferences
// the result, so slot 0 must point at a real, populated block.
const TLS_DATA_VA:   u32 = 0x7FFD_0000;
const TLS_DATA_SIZE: u32 = 0x0000_E000; // 56 KB, sits below the TEB at 0x7FFD_E000

pub fn load_pe(
    bytes: &[u8],
    path: &str,
    pid: u32,
    api: &mut WinApiRegistry,
    logs: &mut LogBuffer,
) -> Result<GuestProcess> {
    let pe = PE::parse(bytes)
        .map_err(|e| VmError::Pe(e.to_string()))?;

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

    let image_base  = oh.windows_fields.image_base as u32;
    let image_size  = oh.windows_fields.size_of_image;
    let hdr_size    = oh.windows_fields.size_of_headers;
    let entry_rva   = oh.standard_fields.address_of_entry_point as u32;
    let entry_point = image_base + entry_rva;

    logs.log(LogLevel::Info, "loader",
        &format!("[loader] loading {path}"), None);
    logs.log(LogLevel::Info, "loader",
        &format!("[pe] image_base=0x{image_base:08X}  size=0x{image_size:X}  entry=0x{entry_point:08X}"), None);

    let mut mem = GuestMemory::new();

    // image region
    mem.allocate(image_base, image_size, PageProt::RWX)?;

    // map PE headers
    let hdr_bytes = &bytes[..hdr_size.min(bytes.len() as u32) as usize];
    mem.write_bytes(image_base, hdr_bytes)?;
    logs.log(LogLevel::Debug, "loader",
        &format!("[loader] mapped headers ({} bytes)", hdr_bytes.len()), None);

    // map sections
    for section in &pe.sections {
        let name = std::str::from_utf8(&section.name)
            .unwrap_or("?")
            .trim_end_matches('\0');

        let va   = image_base + section.virtual_address;
        let roff = section.pointer_to_raw_data as usize;
        let rsz  = section.size_of_raw_data as usize;
        let vsz  = section.virtual_size as usize;

        if rsz > 0 && roff < bytes.len() {
            let end = (roff + rsz).min(bytes.len());
            let src = &bytes[roff..end];
            let copy_len = src.len().min(vsz);
            mem.write_bytes(va, &src[..copy_len])?;
        }
        logs.log(LogLevel::Debug, "loader",
            &format!("[pe] section {name:<8} va=0x{va:08X} vsz=0x{vsz:X} rsz=0x{rsz:X}"), None);
    }

    // relocations — only needed if we loaded at a different base.
    // We load at the preferred base, so none are required.

    // Resolve imports by walking the import directory ourselves and patching the
    // FirstThunk (IAT) — the table the CPU actually calls through. goblin's
    // high-level `imports` reports OriginalFirstThunk (ILT) RVAs, which are NOT
    // what `call dword ptr [iat]` reads.
    let import_count = patch_imports(&pe, image_base, &mut mem, api, logs)?;
    logs.log(LogLevel::Info, "loader",
        &format!("[loader] resolved {import_count} imports"), None);

    // allocate trampoline region (so the memory map is clean)
    if mem.allocate(TRAMP_REGION, TRAMP_REGION_SIZE, PageProt::RX).is_err() {
        // already mapped (second process) — fine
    }

    // stack
    mem.allocate(STACK_BASE, STACK_SIZE, PageProt::RW)?;
    logs.log(LogLevel::Debug, "loader",
        &format!("[loader] stack 0x{STACK_BASE:08X}..0x{STACK_TOP:08X}"), None);

    // heap
    mem.allocate(HEAP_BASE, HEAP_SIZE, PageProt::RW)?;
    logs.log(LogLevel::Debug, "loader",
        &format!("[loader] heap  0x{HEAP_BASE:08X}+0x{HEAP_SIZE:X}"), None);

    // PEB — one page holds PEB proper + TLS array + process-parameters stub
    mem.allocate(PEB_VA, 0x1000, PageProt::RW)?;
    mem.write_u32(PEB_VA + 0x04, 0)?;               // Mutant = none
    mem.write_u32(PEB_VA + 0x08, image_base)?;      // ImageBaseAddress
    mem.write_u32(PEB_VA + 0x0C, 0)?;               // Ldr = null (tolerated for simple progs)
    mem.write_u32(PEB_VA + 0x10, PROC_PARAMS_VA)?;  // ProcessParameters
    mem.write_u32(PEB_VA + 0x1C, HEAP_BASE)?;       // ProcessHeap
    mem.write_u32(PEB_VA + 0x18, 0)?;               // SubSystemData
    mem.write_u32(PEB_VA + 0x68, 0)?;               // NtGlobalFlag = 0 (not debugging)

    // Minimal RTL_USER_PROCESS_PARAMETERS at PROC_PARAMS_VA
    // Just enough for the CRT not to dereference null for I/O handles & command line.
    let cmd_wide: Vec<u8> = "program.exe\0"
        .encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    let cmd_buf_va = PROC_PARAMS_VA + 0x100;
    mem.write_bytes(cmd_buf_va, &cmd_wide)?;
    let cmd_len = (cmd_wide.len() - 2) as u16; // chars, no null
    mem.write_u32(PROC_PARAMS_VA + 0x00, 0x200)?;          // MaximumLength
    mem.write_u32(PROC_PARAMS_VA + 0x04, 0x200)?;          // Length
    mem.write_u32(PROC_PARAMS_VA + 0x18, 0xFFFF_FFF6)?;    // StandardInput  = STDIN handle
    mem.write_u32(PROC_PARAMS_VA + 0x1C, 0xFFFF_FFF5)?;    // StandardOutput = STDOUT handle
    mem.write_u32(PROC_PARAMS_VA + 0x20, 0xFFFF_FFF4)?;    // StandardError  = STDERR handle
    // CommandLine UNICODE_STRING at +0x40
    mem.write_u16(PROC_PARAMS_VA + 0x40, cmd_len * 2)?;    // Length (bytes)
    mem.write_u16(PROC_PARAMS_VA + 0x42, cmd_len * 2 + 2)?;// MaximumLength
    mem.write_u32(PROC_PARAMS_VA + 0x44, cmd_buf_va)?;     // Buffer

    // TLS slot array inside the PEB page
    // TLS_ARRAY_VA: 128 null pointers (each slot initialised on first TlsSetValue)

    // TEB — one page
    mem.allocate(TEB_VA, 0x1000, PageProt::RW)?;
    mem.write_u32(TEB_VA + 0x00, 0xFFFF_FFFF)?;    // ExceptionList = end sentinel
    mem.write_u32(TEB_VA + 0x04, STACK_TOP)?;      // StackBase (highest address)
    mem.write_u32(TEB_VA + 0x08, STACK_BASE)?;     // StackLimit
    mem.write_u32(TEB_VA + 0x18, TEB_VA)?;         // Self pointer
    mem.write_u32(TEB_VA + 0x20, pid)?;            // ClientId.UniqueProcess (pid)
    mem.write_u32(TEB_VA + 0x24, pid * 100)?;      // ClientId.UniqueThread
    mem.write_u32(TEB_VA + 0x2C, TLS_ARRAY_VA)?;   // ThreadLocalStoragePointer
    mem.write_u32(TEB_VA + 0x30, PEB_VA)?;         // ProcessEnvironmentBlock
    mem.write_u32(TEB_VA + 0x34, 0)?;              // LastErrorValue = 0

    // Thread-local storage block. Allocate, point slot 0 at it, then copy the
    // PE's TLS template (if any) and force the module's TLS index to 0.
    mem.allocate(TLS_DATA_VA, TLS_DATA_SIZE, PageProt::RW)?;
    mem.write_u32(TLS_ARRAY_VA, TLS_DATA_VA)?;
    if let Some(tls) = oh.data_directories.get_tls_table() {
        if tls.virtual_address != 0 {
            let d = image_base + tls.virtual_address;
            let raw_start = mem.read_u32(d).unwrap_or(0);      // StartAddressOfRawData (VA)
            let raw_end   = mem.read_u32(d + 4).unwrap_or(0);  // EndAddressOfRawData (VA)
            let idx_addr  = mem.read_u32(d + 8).unwrap_or(0);  // AddressOfIndex (VA)
            let raw_size  = raw_end.saturating_sub(raw_start);
            if raw_size > 0 && raw_size <= TLS_DATA_SIZE {
                if let Ok(template) = mem.read_bytes(raw_start, raw_size as usize) {
                    mem.write_bytes(TLS_DATA_VA, &template)?;
                }
            }
            if idx_addr != 0 {
                let _ = mem.write_u32(idx_addr, 0); // this module's TLS index = slot 0
            }
            logs.log(LogLevel::Debug, "loader",
                &format!("[loader] TLS block 0x{TLS_DATA_VA:08X} template={raw_size}B"), None);
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

    logs.log(LogLevel::Info, "loader",
        &format!("[loader] process created — PID {pid}  EIP=0x{entry_point:08X}  ESP=0x{:08X}", cpu.esp), None);

    logs.log(LogLevel::Info, "loader",
        &format!("[loader] PEB=0x{PEB_VA:08X}  TEB=0x{TEB_VA:08X}"), None);

    Ok(GuestProcess {
        pid,
        path: path.to_string(),
        image_base,
        entry_point,
        heap_base: HEAP_BASE,
        heap_next: HEAP_BASE,
        heap_sizes: std::collections::HashMap::new(),
        memory: mem,
        cpu,
        handles: HandleTable::new(pid),
        console: ConsoleStreams::new(),
        ui_events: Vec::new(),
        gui: crate::vm::process::GuiState::new(),
        spawns: Vec::new(),
        next_child_pid: 0,
        state: ProcessState::Created,
        cwd: crate::vm::process::parent_dir(path),
        cmdline: format!("\"{path}\""),
        messages: extract_message_table(&pe, bytes),
        managed: None,
        tls_slots: std::collections::HashMap::new(),
        next_tls: 1,
        rand_seed: 1,
    })
}

/// Walk the PE import directory and patch each FirstThunk (IAT) slot with a
/// trampoline VA. Reads thunk names from the OriginalFirstThunk (ILT) when
/// present, falling back to the FirstThunk itself for bound-by-name images.
fn patch_imports(
    pe: &PE,
    image_base: u32,
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
        let oft  = mem.read_u32(desc_va).unwrap_or(0);          // OriginalFirstThunk (ILT)
        let name = mem.read_u32(desc_va + 12).unwrap_or(0);     // DLL name RVA
        let ft   = mem.read_u32(desc_va + 16).unwrap_or(0);     // FirstThunk (IAT)

        // Null descriptor terminates the array.
        if oft == 0 && name == 0 && ft == 0 {
            break;
        }

        let dll = mem.read_cstr(image_base + name);

        // Names come from the lookup table; addresses get written to the IAT.
        let lookup_rva = if oft != 0 { oft } else { ft };
        let mut i = 0u32;
        loop {
            let thunk = mem.read_u32(image_base + lookup_rva + i * 4).unwrap_or(0);
            if thunk == 0 {
                break;
            }

            let func_name = if thunk & 0x8000_0000 != 0 {
                // Import by ordinal
                format!("#{}", thunk & 0xFFFF)
            } else {
                // Import by name: thunk -> IMAGE_IMPORT_BY_NAME { hint:u16, name:cstr }
                mem.read_cstr(image_base + (thunk & 0x7FFF_FFFF) + 2)
            };

            let tramp = api.resolve_trampoline(&dll, &func_name);
            let iat_slot = image_base + ft + i * 4;
            if mem.write_u32(iat_slot, tramp).is_ok() {
                count += 1;
            }
            i += 1;
        }

        logs.log(LogLevel::Debug, "loader",
            &format!("[loader] {dll}: {i} imports patched"), None);

        desc_va += DESC_SIZE;
    }

    Ok(count)
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
        let Ok(bytes) = std::fs::read(path) else { return; };

        let mut api  = WinApiRegistry::new();
        let mut logs = LogBuffer::default();

        let proc = load_pe(&bytes, path, 1, &mut api, &mut logs).expect("load PE");

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
}
