use bitflags::bitflags;
use crate::error::{Result, VmError};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageProt: u32 {
        const READ    = 0b001;
        const WRITE   = 0b010;
        const EXECUTE = 0b100;
        const RW  = Self::READ.bits() | Self::WRITE.bits();
        const RX  = Self::READ.bits() | Self::EXECUTE.bits();
        const RWX = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }
}

pub struct MemoryRegion {
    pub base: u32,
    pub size: u32,
    pub prot: PageProt,
    pub bytes: Vec<u8>,
}

pub struct GuestMemory {
    pub regions: std::collections::BTreeMap<u32, MemoryRegion>,
}

impl GuestMemory {
    pub fn new() -> Self { GuestMemory { regions: std::collections::BTreeMap::new() } }

    pub fn allocate(&mut self, base: u32, size: u32, prot: PageProt) -> Result<()> {
        let new_end = base.wrapping_add(size);
        if let Some((_, r)) = self.regions.range(..=base).next_back() {
            let end = r.base.wrapping_add(r.size);
            if base < end {
                return Err(VmError::Memory(format!(
                    "0x{base:08X}+0x{size:X} overlaps existing 0x{:08X}+0x{:X}", r.base, r.size
                )));
            }
        }
        if let Some((_, r)) = self.regions.range(base..).next() {
            if new_end > r.base {
                return Err(VmError::Memory(format!(
                    "0x{base:08X}+0x{size:X} overlaps existing 0x{:08X}+0x{:X}", r.base, r.size
                )));
            }
        }
        self.regions.insert(base, MemoryRegion { base, size, prot, bytes: vec![0u8; size as usize] });
        Ok(())
    }

    /// Grow the region that owns `ptr` so it covers up to `end_va`, without
    /// overlapping the next region above it. Used by the bump heap allocator so
    /// large `malloc`s (e.g. a 16 MB game zone) are actually backed by memory
    /// instead of faulting past a fixed initial heap size.
    pub fn ensure_mapped(&mut self, ptr: u32, end_va: u32) {
        let base = match self.regions.range(..=ptr).next_back() {
            Some((&b, _)) => b,
            None => return,
        };
        let cap = self.regions
            .range((std::ops::Bound::Excluded(base), std::ops::Bound::Unbounded))
            .next()
            .map(|(&b, _)| b)
            .unwrap_or(u32::MAX);
        let want_end = end_va.min(cap);
        let Some(r) = self.regions.get_mut(&base) else { return };
        if want_end <= r.base {
            return;
        }
        let needed = (want_end - r.base) as usize;
        if needed > r.bytes.len() {
            // Round up to 1 MB to avoid reallocating on every small bump.
            let rounded = (needed + 0xF_FFFF) & !0xF_FFFF;
            let new_size = rounded.min((cap.wrapping_sub(r.base)) as usize);
            if new_size > r.bytes.len() {
                r.bytes.resize(new_size, 0);
                r.size = new_size as u32;
            }
        }
    }

    fn region_mut(&mut self, va: u32) -> Option<(&mut MemoryRegion, usize)> {
        if let Some((_, r)) = self.regions.range_mut(..=va).next_back() {
            let base = r.base;
            let size = r.size;
            if va >= base && va < base.wrapping_add(size) {
                let off = (va - base) as usize;
                return Some((r, off));
            }
        }
        None
    }

    fn region(&self, va: u32) -> Option<(&MemoryRegion, usize)> {
        if let Some((_, r)) = self.regions.range(..=va).next_back() {
            if va >= r.base && va < r.base.wrapping_add(r.size) {
                return Some((r, (va - r.base) as usize));
            }
        }
        None
    }

    pub fn write_bytes(&mut self, va: u32, data: &[u8]) -> Result<()> {
        let (r, off) = self.region_mut(va)
            .ok_or_else(|| VmError::Memory(format!("write unmapped 0x{va:08X}")))?;
        let end = off + data.len();
        if end > r.bytes.len() {
            return Err(VmError::Memory(format!("write 0x{va:08X}+{} overflows", data.len())));
        }
        r.bytes[off..end].copy_from_slice(data);
        Ok(())
    }

    pub fn read_bytes(&self, va: u32, len: usize) -> Result<Vec<u8>> {
        let (r, off) = self.region(va)
            .ok_or_else(|| VmError::Memory(format!("read unmapped 0x{va:08X}")))?;
        let end = off + len;
        if end > r.bytes.len() {
            return Err(VmError::Memory(format!("read 0x{va:08X}+{len} overflows")));
        }
        Ok(r.bytes[off..end].to_vec())
    }

    pub fn write_u8 (&mut self, va: u32, v: u8)  -> Result<()> { self.write_bytes(va, &[v]) }
    pub fn write_u16(&mut self, va: u32, v: u16) -> Result<()> { self.write_bytes(va, &v.to_le_bytes()) }
    pub fn write_u32(&mut self, va: u32, v: u32) -> Result<()> { self.write_bytes(va, &v.to_le_bytes()) }

    pub fn read_u8(&self, va: u32) -> Result<u8> {
        Ok(self.read_bytes(va, 1)?[0])
    }
    pub fn read_u16(&self, va: u32) -> Result<u16> {
        let b = self.read_bytes(va, 2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    pub fn read_u32(&self, va: u32) -> Result<u32> {
        let b = self.read_bytes(va, 4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_instruction_window(&self, va: u32) -> Result<&[u8]> {
        let (r, off) = self.region(va)
            .ok_or_else(|| VmError::Memory(format!("fetch unmapped 0x{va:08X}")))?;
        let len = (r.bytes.len() - off).min(15);
        Ok(&r.bytes[off..off + len])
    }

    /// Longest string any of the string readers will return. Guest strings are
    /// bounded only by their NUL, so this is purely a runaway guard; the old
    /// 4096 limit silently truncated ordinary data (long command lines, file
    /// contents passed through the CRT string functions).
    const MAX_STR: usize = 1 << 20;

    /// Raw bytes of a NUL-terminated string, terminator excluded.
    ///
    /// Prefer this over `read_cstr` whenever the bytes are going to be written
    /// back to guest memory or measured: `read_cstr` decodes lossily, so any
    /// byte >= 0x80 (i.e. any CP1252 text) becomes a 3-byte U+FFFD and both the
    /// length and the content change.
    pub fn read_cstr_bytes(&self, va: u32) -> Vec<u8> {
        let mut s = Vec::new();
        let mut addr = va;
        loop {
            match self.read_u8(addr) {
                Ok(0) | Err(_) => break,
                Ok(b) => { s.push(b); addr = addr.wrapping_add(1); }
            }
            if s.len() >= Self::MAX_STR { break; }
        }
        s
    }

    /// Raw UTF-16 code units of a NUL-terminated wide string, terminator
    /// excluded. Preserves unpaired surrogates, which `read_wstr` replaces.
    pub fn read_wstr_units(&self, va: u32) -> Vec<u16> {
        let mut s = Vec::new();
        let mut addr = va;
        loop {
            match self.read_u16(addr) {
                Ok(0) | Err(_) => break,
                Ok(c) => { s.push(c); addr = addr.wrapping_add(2); }
            }
            if s.len() >= Self::MAX_STR { break; }
        }
        s
    }

    pub fn read_cstr(&self, va: u32) -> String {
        String::from_utf8_lossy(&self.read_cstr_bytes(va)).into_owned()
    }

    pub fn read_wstr(&self, va: u32) -> String {
        String::from_utf16_lossy(&self.read_wstr_units(va))
    }
}

impl Default for GuestMemory {
    fn default() -> Self { Self::new() }
}
