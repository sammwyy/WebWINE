// Minimal CLR (Common Language Runtime) support: enough to read a managed
// assembly's metadata and, in later milestones, execute its CIL. A .NET exe's
// native entry point only bootstraps the runtime; the real program lives as CIL
// bytecode described by the metadata tables parsed in `metadata.rs`.

pub mod bcl;
pub mod interp;
pub mod metadata;

pub use interp::ClrRuntime;

use serde::{Deserialize, Serialize};

use crate::error::{Result, VmError};
use metadata::{ClrMetadata, T_ASSEMBLYREF, T_METHODDEF, T_MODULE, T_TYPEDEF, T_TYPEREF};

/// The COR20 / CLI header (PE data directory 14).
#[derive(Debug, Clone, Copy)]
pub struct Cor20Header {
    pub runtime_major: u16,
    pub runtime_minor: u16,
    pub metadata_rva: u32,
    pub metadata_size: u32,
    pub flags: u32,
    pub entry_point_token: u32,
}

/// A loaded managed image: its CLI header plus parsed metadata. Holds the raw
/// PE bytes and section map so method bodies can be read by RVA later.
pub struct ClrImage {
    pub header: Cor20Header,
    pub meta: ClrMetadata,
    image: Vec<u8>,
    sections: Vec<SectionMap>,
}

#[derive(Clone, Copy)]
struct SectionMap {
    va: u32,
    vsize: u32,
    raw_off: u32,
    raw_size: u32,
}

impl ClrImage {
    pub fn parse(bytes: &[u8]) -> Result<ClrImage> {
        let pe = crate::pe::parse_pe(bytes).map_err(|e| VmError::Pe(e.to_string()))?;
        let oh = pe
            .header
            .optional_header
            .ok_or_else(|| VmError::NotPe("no optional header".into()))?;

        let clr = oh
            .data_directories
            .get_clr_runtime_header()
            .filter(|d| d.virtual_address != 0)
            .ok_or_else(|| VmError::NotPe("not a managed (CLI) image".into()))?;

        let sections: Vec<SectionMap> = pe
            .sections
            .iter()
            .map(|s| SectionMap {
                va: s.virtual_address,
                vsize: s.virtual_size,
                raw_off: s.pointer_to_raw_data,
                raw_size: s.size_of_raw_data,
            })
            .collect();

        let cor_off = rva_to_off(&sections, clr.virtual_address)
            .ok_or_else(|| VmError::Pe("CLI header RVA not in any section".into()))?
            as usize;
        let cor = bytes
            .get(cor_off..cor_off + 72)
            .ok_or_else(|| VmError::Pe("truncated CLI header".into()))?;

        let header = Cor20Header {
            runtime_major: u16::from_le_bytes([cor[4], cor[5]]),
            runtime_minor: u16::from_le_bytes([cor[6], cor[7]]),
            metadata_rva: u32::from_le_bytes([cor[8], cor[9], cor[10], cor[11]]),
            metadata_size: u32::from_le_bytes([cor[12], cor[13], cor[14], cor[15]]),
            flags: u32::from_le_bytes([cor[16], cor[17], cor[18], cor[19]]),
            entry_point_token: u32::from_le_bytes([cor[20], cor[21], cor[22], cor[23]]),
        };

        let meta_off = rva_to_off(&sections, header.metadata_rva)
            .ok_or_else(|| VmError::Pe("metadata RVA not in any section".into()))?
            as usize;
        let meta_bytes = bytes
            .get(meta_off..meta_off + header.metadata_size as usize)
            .ok_or_else(|| VmError::Pe("truncated metadata".into()))?;
        let meta = ClrMetadata::parse(meta_bytes)?;

        Ok(ClrImage {
            header,
            meta,
            image: bytes.to_vec(),
            sections,
        })
    }

    /// Resolve the assembly entry-point token to its MethodDef row, if it is one.
    pub fn entry_method_row(&self) -> Option<u32> {
        let tok = self.header.entry_point_token;
        if (tok >> 24) as u8 == T_METHODDEF {
            Some(tok & 0x00FF_FFFF)
        } else {
            None
        }
    }

    /// Raw bytes at an RVA (used to read method bodies / IL).
    pub fn rva_bytes(&self, rva: u32, len: usize) -> Option<&[u8]> {
        let off = rva_to_off(&self.sections, rva)? as usize;
        self.image.get(off..off + len)
    }

    /// Owning type name for a MethodDef row, by scanning the TypeDef method ranges.
    pub fn method_owner(&self, method_row: u32) -> String {
        let types = self.meta.row_count(T_TYPEDEF);
        for t in 1..=types {
            // TypeDef.MethodList (col 5) is the first method; the range ends where
            // the next type's list begins (or the end of the MethodDef table).
            let start = self.meta.col(T_TYPEDEF, t, 5);
            let end = if t < types {
                self.meta.col(T_TYPEDEF, t + 1, 5)
            } else {
                self.meta.row_count(T_METHODDEF) + 1
            };
            if method_row >= start && method_row < end {
                let name = self.meta.get_string(self.meta.col(T_TYPEDEF, t, 1));
                let ns = self.meta.get_string(self.meta.col(T_TYPEDEF, t, 2));
                return if ns.is_empty() { name } else { format!("{ns}.{name}") };
            }
        }
        String::new()
    }
}

/// True if the PE carries a CLI header (data directory 14) — i.e. it is a
/// managed (.NET) assembly that must run on the CLR path, not the x86 loader.
pub fn is_managed(bytes: &[u8]) -> bool {
    crate::pe::parse_pe(bytes)
        .ok()
        .and_then(|pe| pe.header.optional_header)
        .and_then(|oh| oh.data_directories.get_clr_runtime_header().copied())
        .map(|d| d.virtual_address != 0)
        .unwrap_or(false)
}

fn rva_to_off(sections: &[SectionMap], rva: u32) -> Option<u32> {
    for s in sections {
        let size = s.vsize.max(s.raw_size);
        if rva >= s.va && rva < s.va + size {
            return Some(s.raw_off + (rva - s.va));
        }
    }
    None
}

// Inspection view, serialized to the frontend Inspect panel.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClrMethodInfo {
    pub name: String,
    pub type_name: String,
    pub rva: u32,
    pub is_entry: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClrTypeInfo {
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClrInfo {
    pub runtime_version: String,
    pub is_il_only: bool,
    pub entry_point_token: u32,
    pub entry_point_method: String,
    pub module_name: String,
    pub streams: Vec<String>,
    pub table_counts: Vec<(String, u32)>,
    pub assembly_refs: Vec<String>,
    pub types: Vec<ClrTypeInfo>,
    pub methods: Vec<ClrMethodInfo>,
}

pub fn inspect_clr(bytes: &[u8]) -> Result<ClrInfo> {
    let img = ClrImage::parse(bytes)?;
    let m = &img.meta;

    let module_name = if m.row_count(T_MODULE) >= 1 {
        m.get_string(m.col(T_MODULE, 1, 1))
    } else {
        String::new()
    };

    let mut assembly_refs = Vec::new();
    for r in 1..=m.row_count(T_ASSEMBLYREF) {
        assembly_refs.push(m.get_string(m.col(T_ASSEMBLYREF, r, 6)));
    }

    let mut types = Vec::new();
    for t in 1..=m.row_count(T_TYPEDEF) {
        types.push(ClrTypeInfo {
            namespace: m.get_string(m.col(T_TYPEDEF, t, 2)),
            name: m.get_string(m.col(T_TYPEDEF, t, 1)),
        });
    }

    let entry_row = img.entry_method_row();
    let mut methods = Vec::new();
    for r in 1..=m.row_count(T_METHODDEF) {
        methods.push(ClrMethodInfo {
            name: m.get_string(m.col(T_METHODDEF, r, 3)),
            type_name: img.method_owner(r),
            rva: m.col(T_METHODDEF, r, 0),
            is_entry: entry_row == Some(r),
        });
    }

    let entry_point_method = entry_row
        .map(|r| {
            let owner = img.method_owner(r);
            let name = m.get_string(m.col(T_METHODDEF, r, 3));
            if owner.is_empty() { name } else { format!("{owner}.{name}") }
        })
        .unwrap_or_default();

    let table_counts = nonzero_table_counts(m);
    let _ = T_TYPEREF; // referenced for table-name coverage below

    Ok(ClrInfo {
        runtime_version: m.runtime_version.clone(),
        is_il_only: img.header.flags & 0x1 != 0, // COMIMAGE_FLAGS_ILONLY
        entry_point_token: img.header.entry_point_token,
        entry_point_method,
        module_name,
        streams: m.stream_names.clone(),
        table_counts,
        assembly_refs,
        types,
        methods,
    })
}

fn nonzero_table_counts(m: &ClrMetadata) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for t in 0u8..(metadata::MAX_TABLE as u8) {
        let c = m.row_count(t);
        if c > 0 {
            out.push((table_name(t).to_string(), c));
        }
    }
    out
}

fn table_name(t: u8) -> &'static str {
    match t {
        0x00 => "Module",
        0x01 => "TypeRef",
        0x02 => "TypeDef",
        0x04 => "Field",
        0x06 => "MethodDef",
        0x08 => "Param",
        0x09 => "InterfaceImpl",
        0x0A => "MemberRef",
        0x0B => "Constant",
        0x0C => "CustomAttribute",
        0x0E => "DeclSecurity",
        0x11 => "StandAloneSig",
        0x14 => "Event",
        0x17 => "Property",
        0x18 => "MethodSemantics",
        0x1A => "ModuleRef",
        0x1B => "TypeSpec",
        0x1C => "ImplMap",
        0x1D => "FieldRVA",
        0x20 => "Assembly",
        0x23 => "AssemblyRef",
        0x26 => "File",
        0x27 => "ExportedType",
        0x28 => "ManifestResource",
        0x29 => "NestedClass",
        0x2A => "GenericParam",
        0x2B => "MethodSpec",
        0x2C => "GenericParamConstraint",
        _ => "Table",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Option<Vec<u8>> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/10_dotnet_hello/dotnet_hello.exe"
        );
        std::fs::read(path).ok()
    }

    #[test]
    fn parses_dotnet_hello() {
        let Some(bytes) = sample() else { return };
        let info = inspect_clr(&bytes).expect("parse managed image");

        assert!(info.runtime_version.starts_with('v'), "version: {}", info.runtime_version);
        assert!(info.is_il_only);
        assert_eq!(info.entry_point_token >> 24, 0x06, "entry should be a MethodDef token");
        assert!(info.streams.iter().any(|s| s == "#~" || s == "#-"));
        assert!(info.streams.iter().any(|s| s == "#Strings"));

        // The C# program declares a `Program` type with `Main` and `Add`.
        assert!(info.types.iter().any(|t| t.name == "Program"), "types: {:?}", info.types);
        assert!(info.methods.iter().any(|m| m.name == "Main"), "methods: {:?}", info.methods);
        assert!(info.methods.iter().any(|m| m.name == "Add"));

        // Entry point resolves to Program.Main with a real IL RVA.
        let entry = info.methods.iter().find(|m| m.is_entry).expect("entry method");
        assert_eq!(entry.name, "Main");
        assert!(entry.rva > 0, "entry RVA should be non-zero");

        // mscorlib is referenced.
        assert!(info.assembly_refs.iter().any(|a| a == "mscorlib"),
            "refs: {:?}", info.assembly_refs);
    }

    #[test]
    fn runs_dotnet_hello() {
        let Some(bytes) = sample() else { return };
        let img = ClrImage::parse(&bytes).expect("parse managed image");
        let mut rt = ClrRuntime::new(&img);
        let code = rt.run_entry().expect("run managed entry point");
        assert_eq!(code, 0);
        // Main prints a greeting then the result of Add(2, 40) = 42.
        assert_eq!(rt.stdout, "Hello from managed WebWINE!\n42\n", "stdout: {:?}", rt.stdout);
    }
}
