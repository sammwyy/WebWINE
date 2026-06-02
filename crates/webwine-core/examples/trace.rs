// Step-trace a sample exe to find where execution derails.
// Usage: cargo run -p webwine-core --example trace [path-to-exe] [max-steps]

use iced_x86::{Decoder, DecoderOptions, Instruction};
use webwine_core::vm::process::ProcessState;
use webwine_core::WebWineVm;

const TRAMPOLINE_BASE: u32 = 0x7FFE_0000;

fn main() {
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
            "  [{i}] eip=0x{:08X} esp=0x{:08X} ebp=0x{:08X} eax=0x{:08X} ecx=0x{:08X}  {}",
            eip, proc.cpu.esp, proc.cpu.ebp, proc.cpu.eax, proc.cpu.ecx, disasm
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
                println!("--- unimplemented APIs hit ---");
                for ev in vm.drain_logs() {
                    if ev.target == "api" { println!("  {}", ev.message); }
                }
                return;
            }
            _ => {}
        }
    }
    println!("ran {max_steps} steps, no exit; eip=0x{:08X}", vm.processes.get(pid).unwrap().cpu.eip);
    println!("stdout so far: {stdout:?}");
}
