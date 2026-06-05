#[derive(Debug, Clone)]
pub struct X86Cpu {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub eip: u32,
    pub eflags: u32,
    pub last_error: u32,
    pub xmm: [[u8; 16]; 8],
}

impl Default for X86Cpu {
    fn default() -> Self {
        X86Cpu {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
            esi: 0,
            edi: 0,
            ebp: 0,
            esp: 0,
            eip: 0,
            eflags: 0,
            last_error: 0,
            xmm: [[0u8; 16]; 8],
        }
    }
}

impl X86Cpu {
    pub fn new() -> Self {
        X86Cpu {
            eflags: 0x202,
            ..Default::default()
        }
    }
}

// EFLAGS bit positions
pub const CF: u32 = 1 << 0;
pub const PF: u32 = 1 << 2;
pub const AF: u32 = 1 << 4;
pub const ZF: u32 = 1 << 6;
pub const SF: u32 = 1 << 7;
pub const DF: u32 = 1 << 10;
pub const OF: u32 = 1 << 11;

pub fn get_cf(f: u32) -> bool {
    f & CF != 0
}
pub fn get_pf(f: u32) -> bool {
    f & PF != 0
}
pub fn get_zf(f: u32) -> bool {
    f & ZF != 0
}
pub fn get_sf(f: u32) -> bool {
    f & SF != 0
}
pub fn get_df(f: u32) -> bool {
    f & DF != 0
}
pub fn get_of(f: u32) -> bool {
    f & OF != 0
}

pub fn set(f: &mut u32, bit: u32, v: bool) {
    if v {
        *f |= bit
    } else {
        *f &= !bit
    }
}

pub fn set_szp(f: &mut u32, r: u32) {
    set(f, SF, r >> 31 != 0);
    set(f, ZF, r == 0);
    set(f, PF, r.count_ones() % 2 == 0);
}

pub fn set_add32(f: &mut u32, a: u32, b: u32, r: u32) {
    set(f, CF, (r as u64) < (a as u64) + (b as u64));
    set(f, OF, (!(a ^ b) & (a ^ r) & 0x8000_0000) != 0);
    set_szp(f, r);
}

pub fn set_sub32(f: &mut u32, a: u32, b: u32, r: u32) {
    set(f, CF, (a as u64) < (b as u64));
    set(f, OF, ((a ^ b) & (a ^ r) & 0x8000_0000) != 0);
    set_szp(f, r);
}

pub fn set_add8(f: &mut u32, a: u8, b: u8, r: u8) {
    set(f, CF, (r as u16) < (a as u16) + (b as u16));
    set(f, OF, (!(a ^ b) & (a ^ r) & 0x80) != 0);
    set(f, SF, r >> 7 != 0);
    set(f, ZF, r == 0);
    set(f, PF, r.count_ones() % 2 == 0);
}

pub fn set_sub8(f: &mut u32, a: u8, b: u8, r: u8) {
    set(f, CF, (a as u16) < (b as u16));
    set(f, OF, ((a ^ b) & (a ^ r) & 0x80) != 0);
    set(f, SF, r >> 7 != 0);
    set(f, ZF, r == 0);
    set(f, PF, r.count_ones() % 2 == 0);
}

// width-aware flag helpers (8/16/32-bit)
// `w` is the operand width in bytes (1, 2, or 4). These compute SF/ZF/PF/CF/OF
// on the correctly-sized result, which the fixed-width helpers above did not do
// for 16-bit operands.

fn mask_for(w: u32) -> u64 {
    if w >= 4 {
        0xFFFF_FFFF
    } else {
        (1u64 << (w * 8)) - 1
    }
}

pub fn set_szp_w(f: &mut u32, r: u32, w: u32) {
    let sign = 1u64 << (w * 8 - 1);
    set(f, SF, (r as u64 & sign) != 0);
    set(f, ZF, r == 0);
    set(f, PF, (r & 0xFF).count_ones() % 2 == 0); // PF = parity of low byte
}

/// ADD/ADC with carry-in. Returns the width-masked result.
pub fn flags_add(f: &mut u32, a: u32, b: u32, carry: u32, w: u32) -> u32 {
    let m = mask_for(w);
    let (a, b, c) = (a as u64 & m, b as u64 & m, carry as u64);
    let full = a + b + c;
    let r = (full & m) as u32;
    let sign = 1u64 << (w * 8 - 1);
    set(f, CF, full & (1u64 << (w * 8)) != 0);
    set(f, OF, (!(a ^ b) & (a ^ r as u64) & sign) != 0);
    set_szp_w(f, r, w);
    r
}

/// SUB/SBB/CMP with borrow-in. Returns the width-masked result.
pub fn flags_sub(f: &mut u32, a: u32, b: u32, borrow: u32, w: u32) -> u32 {
    let m = mask_for(w);
    let (a, b, bo) = (a as u64 & m, b as u64 & m, borrow as u64);
    let r = (a.wrapping_sub(b).wrapping_sub(bo) & m) as u32;
    let sign = 1u64 << (w * 8 - 1);
    set(f, CF, a < b + bo);
    set(f, OF, ((a ^ b) & (a ^ r as u64) & sign) != 0);
    set_szp_w(f, r, w);
    r
}

/// AND/OR/XOR/TEST: CF=OF=0, SF/ZF/PF on the width-masked result.
pub fn flags_logic(f: &mut u32, r: u32, w: u32) {
    *f &= !(CF | OF);
    set_szp_w(f, r, w);
}
