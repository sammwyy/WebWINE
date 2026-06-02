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
    pub regions: Vec<MemoryRegion>,
}

impl GuestMemory {
    pub fn new() -> Self { GuestMemory { regions: Vec::new() } }

    pub fn allocate(&mut self, base: u32, size: u32, prot: PageProt) -> Result<()> {
        for r in &self.regions {
            let end = r.base.wrapping_add(r.size);
            let new_end = base.wrapping_add(size);
            if base < end && new_end > r.base {
                return Err(VmError::Memory(format!(
                    "0x{base:08X}+0x{size:X} overlaps existing 0x{:08X}+0x{:X}", r.base, r.size
                )));
            }
        }
        self.regions.push(MemoryRegion { base, size, prot, bytes: vec![0u8; size as usize] });
        Ok(())
    }

    fn region_mut(&mut self, va: u32) -> Option<(&mut MemoryRegion, usize)> {
        for r in &mut self.regions {
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
        for r in &self.regions {
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

    pub fn read_cstr(&self, va: u32) -> String {
        let mut s = Vec::new();
        let mut addr = va;
        loop {
            match self.read_u8(addr) {
                Ok(0) | Err(_) => break,
                Ok(b) => { s.push(b); addr = addr.wrapping_add(1); }
            }
            if s.len() > 4096 { break; }
        }
        String::from_utf8_lossy(&s).into_owned()
    }

    pub fn read_wstr(&self, va: u32) -> String {
        let mut s = Vec::new();
        let mut addr = va;
        loop {
            match self.read_u16(addr) {
                Ok(0) | Err(_) => break,
                Ok(c) => {
                    s.push(char::from_u32(c as u32).unwrap_or('?'));
                    addr = addr.wrapping_add(2);
                }
            }
            if s.len() > 4096 { break; }
        }
        s.into_iter().collect()
    }
}

impl Default for GuestMemory {
    fn default() -> Self { Self::new() }
}
