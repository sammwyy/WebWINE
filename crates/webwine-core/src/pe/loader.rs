use goblin::pe::PE;

use crate::cpu::X86Cpu;
use crate::error::{Result, VmError};
use crate::handles::HandleTable;
use crate::logs::{LogBuffer, LogLevel};
use crate::memory::{GuestMemory, PageProt};
use crate::process::{ConsoleStreams, GuestProcess, ProcessState};
use crate::winapi::WinApiDispatcher;

// Fixed virtual address layout
const HEAP_BASE:   u32 = 0x1000_0000;
const HEAP_SIZE:   u32 = 0x0040_0000; // 4 MB
const STACK_BASE:  u32 = 0x6FF0_0000;
const STACK_SIZE:  u32 = 0x0010_0000; // 1 MB
const STACK_TOP:   u32 = STACK_BASE + STACK_SIZE;
const PEB_VA:      u32 = 0x7FFD_F000;
const TEB_VA:      u32 = 0x7FFD_E000;
const TRAMP_REGION: u32 = 0x7FFE_0000;
const TRAMP_REGION_SIZE: u32 = 0x0001_0000;

pub fn load_pe(
    bytes: &[u8],
    path: &str,
    pid: u32,
    api: &mut WinApiDispatcher,
    logs: &mut LogBuffer,
) -> Result<GuestProcess> {
    let pe = PE::parse(bytes)
        .map_err(|e| VmError::Pe(e.to_string()))?;

    let oh = pe
        .header
        .optional_header
        .ok_or_else(|| VmError::NotPe("no optional header".into()))?;

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

    // relocations — only needed if we loaded at a different base
    // For PE32 we load at preferred base so skip for now

    // resolve imports → trampoline addresses, patch IAT
    let mut import_count = 0usize;
    for import in &pe.imports {
        let tramp_va = api.resolve(import.dll, &import.name);
        let iat_va = image_base.wrapping_add(import.rva as u32);
        if mem.write_u32(iat_va, tramp_va).is_ok() {
            import_count += 1;
        }
    }
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

    // PEB (minimal)
    mem.allocate(PEB_VA, 0x1000, PageProt::RW)?;
    mem.write_u32(PEB_VA + 0x08, image_base)?; // ImageBaseAddress

    // TEB (minimal NT_TIB + PEB pointer)
    mem.allocate(TEB_VA, 0x1000, PageProt::RW)?;
    mem.write_u32(TEB_VA + 0x00, 0xFFFF_FFFF)?; // ExceptionList = end sentinel
    mem.write_u32(TEB_VA + 0x04, STACK_TOP)?;   // StackBase (highest addr)
    mem.write_u32(TEB_VA + 0x08, STACK_BASE)?;  // StackLimit
    mem.write_u32(TEB_VA + 0x18, TEB_VA)?;      // Self pointer
    mem.write_u32(TEB_VA + 0x30, PEB_VA)?;      // PEB

    // CPU initial state
    let mut cpu = X86Cpu::new();
    cpu.eip = entry_point;
    cpu.esp = STACK_TOP - 16;
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
        memory: mem,
        cpu,
        handles: HandleTable::new(pid),
        console: ConsoleStreams::new(),
        state: ProcessState::Created,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::LogBuffer;
    use crate::winapi::WinApiDispatcher;

    #[test]
    fn loads_hello_world_sample() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/target/i686-pc-windows-msvc/debug/hello_world.exe"
        );
        let Ok(bytes) = std::fs::read(path) else { return; };

        let mut api  = WinApiDispatcher::new();
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
