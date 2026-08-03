//! Single-byte codepage tables for MultiByteToWideChar / WideCharToMultiByte.
//!
//! Wine converts through a CPTABLEINFO built from the NLS files: for a
//! single-byte codepage the conversion is strictly 1 byte <-> 1 WCHAR
//! (`mbstowcs_sbcs` / `wcstombs_sbcs` in dlls/kernelbase/locale.c). We report
//! GetACP() = 1252 and GetOEMCP() = 437, so those two tables plus UTF-8 cover
//! everything a guest can ask for. Treating the ANSI codepage as UTF-8 (what we
//! used to do) breaks both the byte<->char count and the round-trip for any
//! character above 0x7F.

/// CP1252 (Windows Latin-1) high half, 0x80..=0x9F. Everything else is
/// identity-mapped to the matching Unicode code point.
const CP1252_HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

/// CP437 (OEM US) high half, 0x80..=0xFF.
const CP437_HIGH: [u16; 128] = [
    0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF,
    0x00EE, 0x00EC, 0x00C4, 0x00C5, 0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9,
    0x00FF, 0x00D6, 0x00DC, 0x00A2, 0x00A3, 0x00A5, 0x20A7, 0x0192, 0x00E1, 0x00ED, 0x00F3, 0x00FA,
    0x00F1, 0x00D1, 0x00AA, 0x00BA, 0x00BF, 0x2310, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB,
    0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561, 0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557,
    0x255D, 0x255C, 0x255B, 0x2510, 0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x255E, 0x255F,
    0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x2567, 0x2568, 0x2564, 0x2565, 0x2559,
    0x2558, 0x2552, 0x2553, 0x256B, 0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580,
    0x03B1, 0x00DF, 0x0393, 0x03C0, 0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4,
    0x221E, 0x03C6, 0x03B5, 0x2229, 0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248,
    0x00B0, 0x2219, 0x00B7, 0x221A, 0x207F, 0x00B2, 0x25A0, 0x00A0,
];

/// The codepage our `CP_ACP` (0) / `CP_THREAD_ACP` (3) resolve to, matching
/// kernel32!GetACP. `CP_OEMCP` (1) / `CP_MACCP` (2) resolve to GetOEMCP.
pub fn resolve(codepage: u32) -> u32 {
    match codepage {
        0 | 3 => 1252,
        1 | 2 => 437,
        cp => cp,
    }
}

pub fn is_utf8(codepage: u32) -> bool {
    resolve(codepage) == 65001
}

/// One source byte -> one WCHAR, for a single-byte codepage.
pub fn byte_to_wchar(codepage: u32, b: u8) -> u16 {
    if b < 0x80 {
        return b as u16;
    }
    match resolve(codepage) {
        437 => CP437_HIGH[(b - 0x80) as usize],
        1252 if b < 0xA0 => CP1252_HIGH[(b - 0x80) as usize],
        // 1252 and every other SBCS we don't have a table for: Latin-1.
        _ => b as u16,
    }
}

/// One WCHAR -> one byte, for a single-byte codepage. Unmappable characters
/// become the codepage default char ('?'), like Wine's `wcstombs_sbcs`.
pub fn wchar_to_byte(codepage: u32, w: u16) -> u8 {
    if w < 0x80 {
        return w as u8;
    }
    match resolve(codepage) {
        437 => CP437_HIGH
            .iter()
            .position(|&c| c == w)
            .map(|i| (i + 0x80) as u8)
            .unwrap_or(b'?'),
        1252 => {
            if let Some(i) = CP1252_HIGH.iter().position(|&c| c == w) {
                (i + 0x80) as u8
            } else if (0xA0..=0xFF).contains(&w) {
                w as u8
            } else {
                b'?'
            }
        }
        _ => {
            if (0x80..=0xFF).contains(&w) {
                w as u8
            } else {
                b'?'
            }
        }
    }
}

/// Decode `src` (exactly `src.len()` bytes, terminators included) to UTF-16.
pub fn decode(codepage: u32, src: &[u8]) -> Vec<u16> {
    if is_utf8(codepage) {
        return String::from_utf8_lossy(src).encode_utf16().collect();
    }
    src.iter().map(|&b| byte_to_wchar(codepage, b)).collect()
}

/// Encode `src` (exactly `src.len()` WCHARs, terminators included) to bytes.
pub fn encode(codepage: u32, src: &[u16]) -> Vec<u8> {
    if is_utf8(codepage) {
        return String::from_utf16_lossy(src).into_bytes();
    }
    src.iter().map(|&w| wchar_to_byte(codepage, w)).collect()
}

// character classification (GetStringTypeW)

pub const C1_UPPER: u16 = 0x0001;
pub const C1_LOWER: u16 = 0x0002;
pub const C1_DIGIT: u16 = 0x0004;
pub const C1_SPACE: u16 = 0x0008;
pub const C1_PUNCT: u16 = 0x0010;
pub const C1_CNTRL: u16 = 0x0020;
pub const C1_BLANK: u16 = 0x0040;
pub const C1_XDIGIT: u16 = 0x0080;
pub const C1_ALPHA: u16 = 0x0100;
pub const C1_DEFINED: u16 = 0x0200;

const C2_NOTAPPLICABLE: u16 = 0x0000;
const C2_LEFTTORIGHT: u16 = 0x0001;
const C2_EUROPENUMBER: u16 = 0x0003;
const C2_EUROPESEPARATOR: u16 = 0x0004;
const C2_COMMONSEPARATOR: u16 = 0x0007;
const C2_WHITESPACE: u16 = 0x000C;
const C2_OTHERNEUTRAL: u16 = 0x000B;

const C3_NONSPACING: u16 = 0x0001;
const C3_SYMBOL: u16 = 0x0008;
const C3_ALPHA: u16 = 0x8000;

/// CT_CTYPE1 flags for one code unit, matching the C1_* semantics Wine's
/// `get_char_type` returns from the NLS tables.
pub fn char_type1(c: u16) -> u16 {
    let Some(ch) = char::from_u32(c as u32) else {
        return 0;
    };
    let mut t = C1_DEFINED;
    if ch.is_uppercase() {
        t |= C1_UPPER | C1_ALPHA;
    }
    if ch.is_lowercase() {
        t |= C1_LOWER | C1_ALPHA;
    }
    if ch.is_alphabetic() {
        t |= C1_ALPHA;
    }
    if ch.is_ascii_digit() {
        t |= C1_DIGIT;
    }
    if ch.is_ascii_hexdigit() {
        t |= C1_XDIGIT;
    }
    if ch.is_whitespace() {
        t |= C1_SPACE;
    }
    if ch == ' ' || ch == '\t' {
        t |= C1_BLANK;
    }
    if ch.is_control() {
        t |= C1_CNTRL;
    }
    if ch.is_ascii_punctuation() || matches!(c, 0xA1..=0xBF | 0x2010..=0x205E) {
        t |= C1_PUNCT;
    }
    t
}

/// CT_CTYPE2 (bidirectional layout class).
pub fn char_type2(c: u16) -> u16 {
    let Some(ch) = char::from_u32(c as u32) else {
        return C2_NOTAPPLICABLE;
    };
    match ch {
        '0'..='9' => C2_EUROPENUMBER,
        '+' | '-' => C2_EUROPESEPARATOR,
        ',' | '.' | ':' => C2_COMMONSEPARATOR,
        c if c.is_whitespace() => C2_WHITESPACE,
        c if c.is_alphabetic() => C2_LEFTTORIGHT,
        c if c.is_control() => C2_NOTAPPLICABLE,
        _ => C2_OTHERNEUTRAL,
    }
}

/// CT_CTYPE3 (text-processing class).
pub fn char_type3(c: u16) -> u16 {
    let Some(ch) = char::from_u32(c as u32) else {
        return 0;
    };
    let mut t = 0;
    if ch.is_alphabetic() {
        t |= C3_ALPHA;
    }
    if matches!(c, 0x0300..=0x036F) {
        t |= C3_NONSPACING;
    }
    if !ch.is_alphanumeric()
        && !ch.is_whitespace()
        && !ch.is_control()
        && !ch.is_ascii_punctuation()
    {
        t |= C3_SYMBOL;
    }
    t
}

/// Upper/lowercase one code unit. Only single-unit mappings are applied, which
/// is what LCMapString's in-place contract allows anyway.
pub fn to_upper(c: u16) -> u16 {
    char::from_u32(c as u32)
        .and_then(|ch| {
            let mut it = ch.to_uppercase();
            let up = it.next()?;
            if it.next().is_some() {
                return None; // multi-char expansion (ß -> SS): leave as is
            }
            u16::try_from(up as u32).ok()
        })
        .unwrap_or(c)
}

pub fn to_lower(c: u16) -> u16 {
    char::from_u32(c as u32)
        .and_then(|ch| {
            let mut it = ch.to_lowercase();
            let low = it.next()?;
            if it.next().is_some() {
                return None;
            }
            u16::try_from(low as u32).ok()
        })
        .unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_ascii_like_the_c1_flags() {
        assert_eq!(
            char_type1(b'A' as u16) & (C1_UPPER | C1_ALPHA),
            C1_UPPER | C1_ALPHA
        );
        assert_eq!(
            char_type1(b'7' as u16) & (C1_DIGIT | C1_XDIGIT),
            C1_DIGIT | C1_XDIGIT
        );
        assert_eq!(
            char_type1(b' ' as u16) & (C1_SPACE | C1_BLANK),
            C1_SPACE | C1_BLANK
        );
        assert_eq!(char_type1(b'A' as u16) & C1_DIGIT, 0);
        assert_ne!(char_type1(0xE9) & C1_LOWER, 0); // é
    }

    #[test]
    fn case_maps_single_units_only() {
        assert_eq!(to_upper(b'a' as u16), b'A' as u16);
        assert_eq!(to_lower(0xC9), 0xE9); // É -> é
        assert_eq!(to_upper(0xDF), 0xDF); // ß has no single-unit uppercase
    }

    #[test]
    fn sbcs_round_trips_one_to_one() {
        // CP1252: every byte maps to exactly one WCHAR and back.
        for b in 0u8..=255 {
            let w = byte_to_wchar(1252, b);
            assert_eq!(wchar_to_byte(1252, w), b, "byte 0x{b:02X}");
        }
        // High bytes are single WCHARs, not multi-byte UTF-8 sequences.
        assert_eq!(decode(1252, b"\xE9t\xE9").len(), 3);
        assert_eq!(byte_to_wchar(1252, 0x80), 0x20AC); // euro sign
    }

    #[test]
    fn utf8_codepage_uses_utf8() {
        assert_eq!(decode(65001, "é".as_bytes()), vec![0x00E9u16]);
        assert_eq!(encode(65001, &[0x00E9u16]), "é".as_bytes());
    }

    #[test]
    fn unmappable_becomes_default_char() {
        assert_eq!(wchar_to_byte(1252, 0x4E2D), b'?'); // CJK
    }
}
