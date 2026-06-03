// ECMA-335 CLI metadata reader. A managed assembly stores its real program as
// CIL bytecode plus a set of metadata tables (types, methods, members, …) inside
// the PE. This module parses the COR20 header, the metadata root and its streams,
// and the `#~` compressed tables stream into something we can inspect and later
// execute.

use crate::error::{Result, VmError};

/// Heap index sizes, taken from the `#~` HeapSizes byte. Each heap index is 2 or
/// 4 bytes wide depending on whether the heap is larger than 64 KiB.
#[derive(Clone, Copy)]
struct HeapSizes {
    str_wide: bool,
    guid_wide: bool,
    blob_wide: bool,
}

impl HeapSizes {
    fn from_byte(b: u8) -> Self {
        HeapSizes {
            str_wide: b & 0x01 != 0,
            guid_wide: b & 0x02 != 0,
            blob_wide: b & 0x04 != 0,
        }
    }
}

// Metadata table numbers (ECMA-335 II.22).
pub const T_MODULE: u8 = 0x00;
pub const T_TYPEREF: u8 = 0x01;
pub const T_TYPEDEF: u8 = 0x02;
pub const T_FIELD: u8 = 0x04;
pub const T_METHODDEF: u8 = 0x06;
pub const T_PARAM: u8 = 0x08;
pub const T_MEMBERREF: u8 = 0x0A;
pub const T_STANDALONESIG: u8 = 0x11;
pub const T_TYPESPEC: u8 = 0x1B;
pub const T_MODULEREF: u8 = 0x1A;
pub const T_ASSEMBLY: u8 = 0x20;
pub const T_ASSEMBLYREF: u8 = 0x23;
pub const MAX_TABLE: usize = 0x2D;

/// A column in a metadata table row.
#[derive(Clone, Copy)]
enum Col {
    C2,        // constant 2-byte
    C4,        // constant 4-byte
    Str,       // #Strings heap index
    Guid,      // #GUID heap index
    Blob,      // #Blob heap index
    Tab(u8),   // simple index into another table
    Cod(Coded),// coded index (tag bits + table set)
}

/// Coded index groups (ECMA-335 II.24.2.6). Each maps a small tag to one of
/// several tables, so the index encodes both which table and which row.
#[derive(Clone, Copy, PartialEq)]
enum Coded {
    TypeDefOrRef,
    HasConstant,
    HasCustomAttribute,
    HasFieldMarshal,
    HasDeclSecurity,
    MemberRefParent,
    HasSemantics,
    MethodDefOrRef,
    MemberForwarded,
    Implementation,
    CustomAttributeType,
    ResolutionScope,
    TypeOrMethodDef,
}

impl Coded {
    fn tag_bits(self) -> u32 {
        match self {
            Coded::TypeDefOrRef => 2,
            Coded::HasConstant => 2,
            Coded::HasCustomAttribute => 5,
            Coded::HasFieldMarshal => 1,
            Coded::HasDeclSecurity => 2,
            Coded::MemberRefParent => 3,
            Coded::HasSemantics => 1,
            Coded::MethodDefOrRef => 1,
            Coded::MemberForwarded => 1,
            Coded::Implementation => 2,
            Coded::CustomAttributeType => 3,
            Coded::ResolutionScope => 2,
            Coded::TypeOrMethodDef => 1,
        }
    }

    /// Tables this coded index can point at, in tag order. `0xFF` marks an unused
    /// tag slot (it still consumes a tag value but references no table).
    fn tables(self) -> &'static [u8] {
        match self {
            Coded::TypeDefOrRef => &[0x02, 0x01, 0x1B],
            Coded::HasConstant => &[0x04, 0x08, 0x17],
            Coded::HasCustomAttribute => &[
                0x06, 0x04, 0x01, 0x02, 0x08, 0x09, 0x0A, 0x00, 0x0E, 0x17, 0x14,
                0x11, 0x1A, 0x1B, 0x20, 0x23, 0x26, 0x27, 0x28, 0x2A, 0x2C, 0x2B,
            ],
            Coded::HasFieldMarshal => &[0x04, 0x08],
            Coded::HasDeclSecurity => &[0x02, 0x06, 0x20],
            Coded::MemberRefParent => &[0x02, 0x01, 0x1A, 0x06, 0x1B],
            Coded::HasSemantics => &[0x14, 0x17],
            Coded::MethodDefOrRef => &[0x06, 0x0A],
            Coded::MemberForwarded => &[0x04, 0x06],
            Coded::Implementation => &[0x26, 0x23, 0x27],
            Coded::CustomAttributeType => &[0xFF, 0xFF, 0x06, 0x0A, 0xFF],
            Coded::ResolutionScope => &[0x00, 0x1A, 0x23, 0x01],
            Coded::TypeOrMethodDef => &[0x02, 0x06],
        }
    }
}

/// Column layout for every metadata table (ECMA-335 II.22).
fn columns(table: u8) -> &'static [Col] {
    use Col::*;
    use Coded::*;
    match table {
        0x00 => &[C2, Str, Guid, Guid, Guid],                          // Module
        0x01 => &[Cod(ResolutionScope), Str, Str],                     // TypeRef
        0x02 => &[C4, Str, Str, Cod(TypeDefOrRef), Tab(0x04), Tab(0x06)], // TypeDef
        0x03 => &[Tab(0x04)],                                          // FieldPtr
        0x04 => &[C2, Str, Blob],                                      // Field
        0x05 => &[Tab(0x06)],                                          // MethodPtr
        0x06 => &[C4, C2, C2, Str, Blob, Tab(0x08)],                   // MethodDef
        0x07 => &[Tab(0x08)],                                          // ParamPtr
        0x08 => &[C2, C2, Str],                                        // Param
        0x09 => &[Tab(0x02), Cod(TypeDefOrRef)],                       // InterfaceImpl
        0x0A => &[Cod(MemberRefParent), Str, Blob],                    // MemberRef
        0x0B => &[C2, Cod(HasConstant), Blob],                         // Constant
        0x0C => &[Cod(HasCustomAttribute), Cod(CustomAttributeType), Blob], // CustomAttribute
        0x0D => &[Cod(HasFieldMarshal), Blob],                         // FieldMarshal
        0x0E => &[C2, Cod(HasDeclSecurity), Blob],                     // DeclSecurity
        0x0F => &[C2, C4, Tab(0x02)],                                  // ClassLayout
        0x10 => &[C4, Tab(0x04)],                                      // FieldLayout
        0x11 => &[Blob],                                              // StandAloneSig
        0x12 => &[Tab(0x02), Tab(0x14)],                               // EventMap
        0x13 => &[Tab(0x14)],                                          // EventPtr
        0x14 => &[C2, Str, Cod(TypeDefOrRef)],                         // Event
        0x15 => &[Tab(0x02), Tab(0x17)],                               // PropertyMap
        0x16 => &[Tab(0x17)],                                          // PropertyPtr
        0x17 => &[C2, Str, Blob],                                      // Property
        0x18 => &[C2, Tab(0x06), Cod(HasSemantics)],                   // MethodSemantics
        0x19 => &[Tab(0x02), Cod(MethodDefOrRef), Cod(MethodDefOrRef)],// MethodImpl
        0x1A => &[Str],                                               // ModuleRef
        0x1B => &[Blob],                                              // TypeSpec
        0x1C => &[C2, Cod(MemberForwarded), Str, Tab(0x1A)],           // ImplMap
        0x1D => &[C4, Tab(0x04)],                                      // FieldRVA
        0x1E => &[C4, C4],                                             // EncLog
        0x1F => &[C4],                                                // EncMap
        0x20 => &[C4, C2, C2, C2, C2, C4, Blob, Str, Str],            // Assembly
        0x21 => &[C4],                                                // AssemblyProcessor
        0x22 => &[C4, C4, C4],                                        // AssemblyOS
        0x23 => &[C2, C2, C2, C2, C4, Blob, Str, Str, Blob],          // AssemblyRef
        0x24 => &[C4, Tab(0x23)],                                     // AssemblyRefProcessor
        0x25 => &[C4, C4, C4, Tab(0x23)],                             // AssemblyRefOS
        0x26 => &[C4, Str, Blob],                                     // File
        0x27 => &[C4, C4, Str, Str, Cod(Implementation)],            // ExportedType
        0x28 => &[C4, C4, Str, Cod(Implementation)],                 // ManifestResource
        0x29 => &[Tab(0x02), Tab(0x02)],                             // NestedClass
        0x2A => &[C2, C2, Cod(TypeOrMethodDef), Str],                // GenericParam
        0x2B => &[Cod(MethodDefOrRef), Blob],                        // MethodSpec
        0x2C => &[Tab(0x2A), Cod(TypeDefOrRef)],                      // GenericParamConstraint
        _ => &[],
    }
}

/// Parsed metadata for one managed assembly.
pub struct ClrMetadata {
    pub runtime_version: String,
    pub stream_names: Vec<String>,
    pub heap_sizes_byte: u8,
    pub row_counts: [u32; MAX_TABLE],
    // Per-table byte layout, computed once from the index sizes.
    row_width: [usize; MAX_TABLE],
    col_off: [Vec<usize>; MAX_TABLE],
    table_base: [usize; MAX_TABLE], // offset into `tables_blob`
    tables_blob: Vec<u8>,
    heaps: HeapSizes,
    strings: Vec<u8>,
    blobs: Vec<u8>,
    user_strings: Vec<u8>,
    guids: Vec<u8>,
}

struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Reader { b, p: 0 }
    }
    fn u8(&mut self) -> Result<u8> {
        let v = *self.b.get(self.p).ok_or_else(eof)?;
        self.p += 1;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16> {
        let s = self.b.get(self.p..self.p + 2).ok_or_else(eof)?;
        self.p += 2;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Result<u32> {
        let s = self.b.get(self.p..self.p + 4).ok_or_else(eof)?;
        self.p += 4;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn u64(&mut self) -> Result<u64> {
        let lo = self.u32()? as u64;
        let hi = self.u32()? as u64;
        Ok((hi << 32) | lo)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.p..self.p + n).ok_or_else(eof)?;
        self.p += n;
        Ok(s)
    }
}

fn eof() -> VmError {
    VmError::Pe("unexpected end of metadata".into())
}

fn rd_u16(b: &[u8], off: usize) -> u32 {
    u16::from_le_bytes([b[off], b[off + 1]]) as u32
}
fn rd_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

impl ClrMetadata {
    /// Parse the metadata root (`BSJB`) and the `#~` tables stream from the raw
    /// bytes of the metadata directory.
    pub fn parse(meta: &[u8]) -> Result<ClrMetadata> {
        let mut r = Reader::new(meta);
        if r.u32()? != 0x424A_5342 {
            return Err(VmError::Pe("bad metadata signature (expected BSJB)".into()));
        }
        let _major = r.u16()?;
        let _minor = r.u16()?;
        let _reserved = r.u32()?;
        let ver_len = r.u32()? as usize;
        let ver_bytes = r.take((ver_len + 3) & !3)?;
        let runtime_version = String::from_utf8_lossy(ver_bytes)
            .trim_end_matches('\0')
            .to_string();
        let _flags = r.u16()?;
        let stream_count = r.u16()? as usize;

        let mut tables_off = 0usize;
        let mut tables_size = 0usize;
        let mut strings = Vec::new();
        let mut blobs = Vec::new();
        let mut user_strings = Vec::new();
        let mut guids = Vec::new();
        let mut stream_names = Vec::new();

        for _ in 0..stream_count {
            let off = r.u32()? as usize;
            let size = r.u32()? as usize;
            // Stream name: null-terminated, padded to the next 4-byte boundary.
            let name_start = r.p;
            while r.u8()? != 0 {}
            let mut name_len = r.p - name_start - 1;
            // skip padding so the cursor lands on a 4-byte boundary
            let consumed = r.p - name_start;
            let pad = (4 - (consumed % 4)) % 4;
            r.p += pad;
            let name = String::from_utf8_lossy(&meta[name_start..name_start + name_len]).to_string();
            name_len = name.len();
            let _ = name_len;
            let slice = meta.get(off..off + size).unwrap_or(&[]).to_vec();
            match name.as_str() {
                "#~" | "#-" => {
                    tables_off = off;
                    tables_size = size;
                }
                "#Strings" => strings = slice,
                "#Blob" => blobs = slice,
                "#US" => user_strings = slice,
                "#GUID" => guids = slice,
                _ => {}
            }
            stream_names.push(name);
        }

        if tables_off == 0 {
            return Err(VmError::Pe("no #~ metadata tables stream".into()));
        }
        let tables = meta
            .get(tables_off..tables_off + tables_size)
            .ok_or_else(eof)?;

        Self::parse_tables(tables, runtime_version, stream_names, strings, blobs, user_strings, guids)
    }

    fn parse_tables(
        tables: &[u8],
        runtime_version: String,
        stream_names: Vec<String>,
        strings: Vec<u8>,
        blobs: Vec<u8>,
        user_strings: Vec<u8>,
        guids: Vec<u8>,
    ) -> Result<ClrMetadata> {
        let mut r = Reader::new(tables);
        let _reserved = r.u32()?;
        let _major = r.u8()?;
        let _minor = r.u8()?;
        let heap_sizes_byte = r.u8()?;
        let _reserved2 = r.u8()?;
        let valid = r.u64()?;
        let _sorted = r.u64()?;

        let heaps = HeapSizes::from_byte(heap_sizes_byte);

        let mut row_counts = [0u32; MAX_TABLE];
        for t in 0..MAX_TABLE {
            if valid & (1u64 << t) != 0 {
                row_counts[t] = r.u32()?;
            }
        }

        // Column widths depend on row counts and heap sizes, so compute them now.
        let table_index_size = |target: u8| -> usize {
            if (target as usize) < MAX_TABLE && row_counts[target as usize] >= (1 << 16) {
                4
            } else {
                2
            }
        };
        let coded_index_size = |c: Coded| -> usize {
            let bits = c.tag_bits();
            let max_rows = c
                .tables()
                .iter()
                .filter(|&&t| t != 0xFF && (t as usize) < MAX_TABLE)
                .map(|&t| row_counts[t as usize])
                .max()
                .unwrap_or(0);
            if max_rows < (1u32 << (16 - bits)) { 2 } else { 4 }
        };
        let col_width = |c: &Col| -> usize {
            match c {
                Col::C2 => 2,
                Col::C4 => 4,
                Col::Str => if heaps.str_wide { 4 } else { 2 },
                Col::Guid => if heaps.guid_wide { 4 } else { 2 },
                Col::Blob => if heaps.blob_wide { 4 } else { 2 },
                Col::Tab(t) => table_index_size(*t),
                Col::Cod(c) => coded_index_size(*c),
            }
        };

        const EMPTY: Vec<usize> = Vec::new();
        let mut row_width = [0usize; MAX_TABLE];
        let mut col_off: [Vec<usize>; MAX_TABLE] = [EMPTY; MAX_TABLE];
        let mut table_base = [0usize; MAX_TABLE];

        // The row data begins right after the row-count array (current cursor).
        let mut cursor = r.p;
        for t in 0..MAX_TABLE {
            if row_counts[t] == 0 {
                continue;
            }
            let cols = columns(t as u8);
            let mut offs = Vec::with_capacity(cols.len());
            let mut w = 0usize;
            for c in cols {
                offs.push(w);
                w += col_width(c);
            }
            row_width[t] = w;
            col_off[t] = offs;
            table_base[t] = cursor;
            cursor += w * row_counts[t] as usize;
        }

        Ok(ClrMetadata {
            runtime_version,
            stream_names,
            heap_sizes_byte,
            row_counts,
            row_width,
            col_off,
            table_base,
            tables_blob: tables.to_vec(),
            heaps,
            strings,
            blobs,
            user_strings,
            guids,
        })
    }

    pub fn row_count(&self, table: u8) -> u32 {
        self.row_counts.get(table as usize).copied().unwrap_or(0)
    }

    /// Read column `col` (0-based) of `row` (1-based) in `table` as a u32.
    pub fn col(&self, table: u8, row: u32, col: usize) -> u32 {
        let t = table as usize;
        if row == 0 || row > self.row_counts[t] {
            return 0;
        }
        let cols = columns(table);
        let Some(kind) = cols.get(col) else { return 0 };
        let base = self.table_base[t] + (row as usize - 1) * self.row_width[t] + self.col_off[t][col];
        let wide = matches!(kind, Col::C4)
            || matches!(kind, Col::Str if self.heaps.str_wide)
            || matches!(kind, Col::Guid if self.heaps.guid_wide)
            || matches!(kind, Col::Blob if self.heaps.blob_wide)
            || matches!(kind, Col::Tab(t) if self.row_counts[*t as usize] >= (1 << 16))
            || matches!(kind, Col::Cod(c) if self.coded_wide(*c));
        if wide {
            if base + 4 <= self.tables_blob.len() { rd_u32(&self.tables_blob, base) } else { 0 }
        } else if base + 2 <= self.tables_blob.len() {
            rd_u16(&self.tables_blob, base)
        } else {
            0
        }
    }

    fn coded_wide(&self, c: Coded) -> bool {
        let bits = c.tag_bits();
        let max_rows = c
            .tables()
            .iter()
            .filter(|&&t| t != 0xFF && (t as usize) < MAX_TABLE)
            .map(|&t| self.row_counts[t as usize])
            .max()
            .unwrap_or(0);
        max_rows >= (1u32 << (16 - bits))
    }

    /// Decode a coded index into `(table, row)`.
    pub fn decode_coded(&self, c_table: u8, row: u32, col: usize, group: CodedKind) -> (u8, u32) {
        let raw = self.col(c_table, row, col);
        let coded = group.to_internal();
        let bits = coded.tag_bits();
        let tag = (raw & ((1 << bits) - 1)) as usize;
        let index = raw >> bits;
        let tables = coded.tables();
        let table = tables.get(tag).copied().unwrap_or(0xFF);
        (table, index)
    }

    pub fn get_string(&self, idx: u32) -> String {
        let start = idx as usize;
        if start >= self.strings.len() {
            return String::new();
        }
        let end = self.strings[start..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| start + p)
            .unwrap_or(self.strings.len());
        String::from_utf8_lossy(&self.strings[start..end]).to_string()
    }

    /// A blob is a compressed-length prefix followed by the raw bytes.
    pub fn get_blob(&self, idx: u32) -> &[u8] {
        let start = idx as usize;
        if start >= self.blobs.len() {
            return &[];
        }
        let (len, hdr) = decompress_uint(&self.blobs[start..]);
        let data_start = start + hdr;
        let data_end = (data_start + len as usize).min(self.blobs.len());
        self.blobs.get(data_start..data_end).unwrap_or(&[])
    }

    /// A user string (`#US`) is a blob of UTF-16 code units plus a trailing flag.
    pub fn get_user_string(&self, idx: u32) -> String {
        let start = idx as usize;
        if start >= self.user_strings.len() {
            return String::new();
        }
        let (len, hdr) = decompress_uint(&self.user_strings[start..]);
        let data_start = start + hdr;
        // The stored length includes the trailing flag byte; the UTF-16 payload
        // is everything but that final byte.
        let payload = len.saturating_sub(1) as usize;
        let data_end = (data_start + payload).min(self.user_strings.len());
        let bytes = self.user_strings.get(data_start..data_end).unwrap_or(&[]);
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    }

    pub fn guid_count(&self) -> usize {
        self.guids.len() / 16
    }
}

/// Coded-index groups exposed to callers (mirrors the internal `Coded`).
#[derive(Clone, Copy)]
pub enum CodedKind {
    TypeDefOrRef,
    MemberRefParent,
    ResolutionScope,
    MethodDefOrRef,
}

impl CodedKind {
    fn to_internal(self) -> Coded {
        match self {
            CodedKind::TypeDefOrRef => Coded::TypeDefOrRef,
            CodedKind::MemberRefParent => Coded::MemberRefParent,
            CodedKind::ResolutionScope => Coded::ResolutionScope,
            CodedKind::MethodDefOrRef => Coded::MethodDefOrRef,
        }
    }
}

/// Read an ECMA-335 compressed unsigned integer. Returns (value, bytes_read).
pub fn decompress_uint(b: &[u8]) -> (u32, usize) {
    match b.first() {
        None => (0, 0),
        Some(&x) if x & 0x80 == 0 => (x as u32, 1),
        Some(&x) if x & 0xC0 == 0x80 => {
            let v = ((x as u32 & 0x3F) << 8) | b.get(1).copied().unwrap_or(0) as u32;
            (v, 2)
        }
        Some(&x) => {
            let v = ((x as u32 & 0x1F) << 24)
                | (b.get(1).copied().unwrap_or(0) as u32) << 16
                | (b.get(2).copied().unwrap_or(0) as u32) << 8
                | b.get(3).copied().unwrap_or(0) as u32;
            (v, 4)
        }
    }
}
