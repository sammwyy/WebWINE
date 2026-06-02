// Step-trace a sample exe to find where execution derails.
// Usage: cargo run -p webwine-core --example trace [path-to-exe] [max-steps]

use iced_x86::{Decoder, DecoderOptions, Instruction};
use webwine_core::vm::process::ProcessState;
use webwine_core::WebWineVm;

const TRAMPOLINE_BASE: u32 = 0x7FFE_0000;

fn disasm_mode() {
    use iced_x86::{OpKind, Register};
    let exe = std::env::args().nth(2).expect("exe path");
    let addr = u32::from_str_radix(
        std::env::args().nth(3).expect("addr").trim_start_matches("0x"), 16).expect("hex addr");
    let count: usize = std::env::args().nth(4).and_then(|s| s.parse().ok()).unwrap_or(40);

    let bytes = std::fs::read(&exe).expect("read exe");
    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\x.exe", bytes).unwrap();
    let pid = vm.launch_process("C:\\x.exe").unwrap();
    let proc = vm.processes.get(pid).unwrap();

    let mut ip = addr;
    for _ in 0..count {
        let win = match proc.memory.read_instruction_window(ip) { Ok(b) => b, Err(_) => break };
        let mut dec = Decoder::with_ip(32, win, ip as u64, DecoderOptions::NONE);
        let mut ins = Instruction::default();
        dec.decode_out(&mut ins);
        let mut ops = String::new();
        for i in 0..ins.op_count() {
            ops.push_str(&match ins.op_kind(i) {
                OpKind::Register => format!("{:?} ", ins.op_register(i)),
                OpKind::Memory => {
                    let seg = if ins.memory_segment() == Register::FS { "FS:" } else { "" };
                    format!("[{seg}{:?}+{:?}*{}+0x{:X}] ",
                        ins.memory_base(), ins.memory_index(), ins.memory_index_scale(),
                        ins.memory_displacement32())
                }
                OpKind::Immediate8 => format!("0x{:X} ", ins.immediate8()),
                OpKind::Immediate8to32 => format!("0x{:X} ", ins.immediate8to32()),
                OpKind::Immediate32 => format!("0x{:X} ", ins.immediate32()),
                OpKind::NearBranch32 => format!("0x{:X} ", ins.near_branch32()),
                k => format!("{k:?} "),
            });
        }
        println!("0x{:08X}: {:<10?} {}", ip, ins.mnemonic(), ops.trim());
        ip += ins.len() as u32;
    }
}

fn main() {
    // disasm mode: trace disasm <exe> <hexAddr> <count>
    if std::env::args().nth(1).as_deref() == Some("disasm") {
        disasm_mode();
        return;
    }

    let path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/target/i686-pc-windows-msvc/debug/minimal.exe"
        )
        .to_string()
    });
    let max_steps: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20000);

    let bytes = std::fs::read(&path).expect("read exe");
    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\Users\\guest\\Desktop\\sample.exe", bytes).unwrap();
    let pid = vm.launch_process("C:\\Users\\guest\\Desktop\\sample.exe").unwrap();
    let mut history: Vec<String> = Vec::new();
    let mut stdout = String::new();

    // Drive execution one instruction at a time through the real VM so API
    // dispatch uses the actual registered handlers.
    for i in 0..max_steps {
        let proc = vm.processes.get(pid).unwrap();
        let eip = proc.cpu.eip;
        let disasm = if eip >= TRAMPOLINE_BASE {
            let name = vm.api.lookup_name(eip)
                .map(|(d, n)| format!("{d}!{n}"))
                .unwrap_or_default();
            format!(">>> API {name}")
        } else {
            match proc.memory.read_instruction_window(eip) {
                Ok(b) => {
                    let mut dec = Decoder::with_ip(32, b, eip as u64, DecoderOptions::NONE);
                    let mut instr = Instruction::default();
                    dec.decode_out(&mut instr);
                    format!("{:<10?} len={}", instr.mnemonic(), instr.len())
                }
                Err(e) => format!("<fetch err: {e}>"),
            }
        };
        history.push(format!(
            "  [{i}] eip=0x{:08X} eax=0x{:08X} ecx=0x{:08X} edx=0x{:08X} esi=0x{:08X} edi=0x{:08X} ebp=0x{:08X}  {}",
            eip, proc.cpu.eax, proc.cpu.ecx, proc.cpu.edx, proc.cpu.esi, proc.cpu.edi, proc.cpu.ebp, disasm
        ));
        if history.len() > 80 { history.remove(0); }

        let r = vm.run_process_slice(pid, 1).unwrap();
        stdout.push_str(&r.stdout);
        match r.state {
            ProcessState::Exited { exit_code } => {
                println!("EXITED code={exit_code} after {i} steps");
                println!("stdout: {stdout:?}");
                return;
            }
            ProcessState::Crashed { reason } => {
                println!("CRASH after {i} steps: {reason}\n--- last {} instructions ---", history.len());
                for line in &history { println!("{line}"); }
                println!("--- cpu/api log ---");
                for ev in vm.drain_logs() {
                    if ev.target == "cpu" || ev.target == "api" { println!("  [{}] {}", ev.target, ev.message); }
                }
                return;
            }
            _ => {}
        }
    }
    println!("ran {max_steps} steps, no exit; eip=0x{:08X}", vm.processes.get(pid).unwrap().cpu.eip);
    println!("--- last instructions ---");
    for line in history.iter() { println!("{line}"); }
    println!("stdout so far: {stdout:?}");
}
