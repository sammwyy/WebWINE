// Dump the raw IL of each MethodDef in a managed assembly, to validate method
// body parsing and see which CIL opcodes a sample actually uses.
// Usage: cargo run -p webwine-core --example cildump <managed.exe>

use webwine_core::clr::metadata::{T_METHODDEF, T_TYPEDEF};
use webwine_core::clr::ClrImage;

fn main() {
    let path = std::env::args().nth(1).expect("path to managed exe");
    let bytes = std::fs::read(&path).expect("read exe");
    let img = ClrImage::parse(&bytes).expect("parse managed image");
    let m = &img.meta;

    println!(
        "runtime {}  entry token 0x{:08X}",
        m.runtime_version, img.header.entry_point_token
    );

    for r in 1..=m.row_count(T_METHODDEF) {
        let name = m.get_string(m.col(T_METHODDEF, r, 3));
        let rva = m.col(T_METHODDEF, r, 0);
        let owner = img.method_owner(r);
        print!("\nMethodDef {r}: {owner}::{name}  rva=0x{rva:X}");
        if rva == 0 {
            println!("  (no body)");
            continue;
        }
        // Read only the method header first. Large real-world methods regularly
        // exceed 512 bytes, so fetch the complete body after decoding its size.
        let hdr = img.rva_bytes(rva, 12).or_else(|| img.rva_bytes(rva, 1));
        let Some(hdr) = hdr else {
            println!("  (rva unreadable)");
            continue;
        };
        let b0 = hdr[0];
        let (code_off, code_size, max_stack, locals_tok) = if b0 & 0x03 == 0x02 {
            (1usize, (b0 >> 2) as usize, 8u16, 0u32)
        } else {
            let max_stack = u16::from_le_bytes([hdr[2], hdr[3]]);
            let code_size = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
            let locals = u32::from_le_bytes([hdr[8], hdr[9], hdr[10], hdr[11]]);
            (12usize, code_size, max_stack, locals)
        };
        println!(
            "  hdr={} code_size={code_size} max_stack={max_stack} locals_tok=0x{locals_tok:08X}",
            if code_off == 1 { "tiny" } else { "fat" }
        );
        let Some(body) = img.rva_bytes(rva, code_off + code_size) else {
            println!("  (body truncated)");
            continue;
        };
        let code = &body[code_off..code_off + code_size];
        print!("  IL:");
        for (i, byte) in code.iter().enumerate() {
            if i % 16 == 0 {
                print!("\n    ");
            }
            print!("{byte:02X} ");
        }
        println!();

        if (T_TYPEDEF as u32) == 0 { /* keep import used */ }
    }
}
