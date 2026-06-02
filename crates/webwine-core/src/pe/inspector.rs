use goblin::pe::PE;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VmError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeSection {
    pub name: String,
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub raw_size: u32,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeImportModule {
    pub dll: String,
    pub functions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeInfo {
    pub machine: String,
    pub machine_id: u16,
    pub subsystem: String,
    pub subsystem_id: u16,
    pub is_pe32: bool,
    pub is_dll: bool,
    pub image_base: u64,
    pub entry_point_rva: u32,
    pub size_of_image: u32,
    pub sections: Vec<PeSection>,
    pub imports: Vec<PeImportModule>,
}

pub fn inspect_bytes(bytes: &[u8]) -> Result<PeInfo> {
    let pe = PE::parse(bytes).map_err(|e| VmError::Pe(e.to_string()))?;

    let machine_id = pe.header.coff_header.machine;
    let machine = machine_name(machine_id).to_string();

    let (subsystem_id, image_base, entry_point_rva, size_of_image, is_pe32) =
        match pe.header.optional_header {
            Some(oh) => {
                let sub = oh.windows_fields.subsystem;
                let base = oh.windows_fields.image_base;
                let ep = oh.standard_fields.address_of_entry_point as u32;
                let sz = oh.windows_fields.size_of_image;
                let pe32 = oh.standard_fields.magic == 0x10b;
                (sub, base, ep, sz, pe32)
            }
            None => return Err(VmError::NotPe("no optional header".into())),
        };

    let subsystem = subsystem_name(subsystem_id).to_string();
    let is_dll = pe.header.coff_header.characteristics & 0x2000 != 0;

    let sections = pe
        .sections
        .iter()
        .map(|s| {
            let raw_name = &s.name;
            let name = std::str::from_utf8(raw_name)
                .unwrap_or("?")
                .trim_end_matches('\0')
                .to_string();
            let ch = s.characteristics;
            PeSection {
                name,
                virtual_address: s.virtual_address,
                virtual_size: s.virtual_size,
                raw_size: s.size_of_raw_data,
                readable: ch & 0x4000_0000 != 0,
                writable: ch & 0x8000_0000 != 0,
                executable: ch & 0x2000_0000 != 0,
            }
        })
        .collect();

    let mut import_map: IndexMap<String, Vec<String>> = IndexMap::new();
    for import in &pe.imports {
        import_map
            .entry(import.dll.to_ascii_uppercase())
            .or_default()
            .push(import.name.to_string());
    }

    let imports = import_map
        .into_iter()
        .map(|(dll, functions)| PeImportModule { dll, functions })
        .collect();

    Ok(PeInfo {
        machine,
        machine_id,
        subsystem,
        subsystem_id,
        is_pe32,
        is_dll,
        image_base,
        entry_point_rva,
        size_of_image,
        sections,
        imports,
    })
}

fn machine_name(id: u16) -> &'static str {
    match id {
        0x014C => "i386",
        0x8664 => "x86_64",
        0x01C0 => "ARM",
        0xAA64 => "ARM64",
        0x0200 => "IA64",
        _ => "unknown",
    }
}

fn subsystem_name(id: u16) -> &'static str {
    match id {
        1 => "Native",
        2 => "Windows GUI",
        3 => "Windows Console",
        5 => "OS/2 Console",
        7 => "POSIX",
        9 => "Windows CE GUI",
        10 => "EFI Application",
        11 => "EFI Boot Service Driver",
        12 => "EFI Runtime Driver",
        13 => "EFI ROM",
        14 => "Xbox",
        16 => "Windows Boot Application",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_pe() {
        let result = inspect_bytes(b"this is not a pe file");
        assert!(result.is_err());
    }

    #[test]
    fn parses_hello_world_sample() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/target/i686-pc-windows-msvc/debug/hello_world.exe"
        );
        let Ok(bytes) = std::fs::read(path) else {
            // sample not built yet — skip
            return;
        };
        let info = inspect_bytes(&bytes).expect("should parse PE32");
        assert_eq!(info.machine, "i386");
        assert_eq!(info.machine_id, 0x014C);
        assert!(info.is_pe32);
        assert!(!info.is_dll);
        assert!(info.image_base > 0);
        assert!(info.entry_point_rva > 0);
        assert!(!info.sections.is_empty());
    }
}
