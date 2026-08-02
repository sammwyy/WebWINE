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
        std::env::args()
            .nth(3)
            .expect("addr")
            .trim_start_matches("0x"),
        16,
    )
    .expect("hex addr");
    let count: usize = std::env::args()
        .nth(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or(40);

    let bytes = std::fs::read(&exe).expect("read exe");
    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\x.exe", bytes).unwrap();
    let pid = vm.launch_process("C:\\x.exe").unwrap();
    let proc = vm.processes.get(pid).unwrap();

    let mut ip = addr;
    for _ in 0..count {
        let win = match proc.memory.read_instruction_window(ip) {
            Ok(b) => b,
            Err(_) => break,
        };
        let mut dec = Decoder::with_ip(32, win, ip as u64, DecoderOptions::NONE);
        let mut ins = Instruction::default();
        dec.decode_out(&mut ins);
        let mut ops = String::new();
        for i in 0..ins.op_count() {
            ops.push_str(&match ins.op_kind(i) {
                OpKind::Register => format!("{:?} ", ins.op_register(i)),
                OpKind::Memory => {
                    let seg = if ins.memory_segment() == Register::FS {
                        "FS:"
                    } else {
                        ""
                    };
                    let mut rendered = format!(
                        "[{seg}{:?}+{:?}*{}+0x{:X}]",
                        ins.memory_base(),
                        ins.memory_index(),
                        ins.memory_index_scale(),
                        ins.memory_displacement32()
                    );
                    // For absolute IAT operands, show the resolved WebWINE API.
                    // This turns `jmp [0x1001AF4]` into an actionable symbol in
                    // real-binary traces without requiring external PE tools.
                    if ins.memory_base() == Register::None && ins.memory_index() == Register::None {
                        let slot = ins.memory_displacement32();
                        if let Ok(target) = proc.memory.read_u32(slot) {
                            rendered.push_str(&format!(" -> 0x{target:08X}"));
                            if let Some((dll, name)) = vm.api.lookup_name(target) {
                                rendered.push_str(&format!(" {dll}!{name}"));
                            }
                        }
                    }
                    format!("{rendered} ")
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
    // GAMEDIR=<host dir> mounts the whole directory tree under C:\game\ and
    // launches C:\game\<exe-basename>, so the guest sees the exe next to its
    // data files/archives (replicates how the frontend mounts a game folder).
    let exe_guest = if let Ok(dir) = std::env::var("GAMEDIR") {
        let base = std::path::Path::new(&dir);
        let mut count = 0usize;
        let mut stack = vec![base.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(data) = std::fs::read(&p) {
                    let rel = p
                        .strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('/', "\\");
                    let guest = format!("C:\\game\\{rel}");
                    // Create parent dirs (mount_file does not auto-create them).
                    let parts: Vec<&str> = guest.trim_start_matches("C:\\").split('\\').collect();
                    let mut acc = String::from("C:");
                    for seg in &parts[..parts.len() - 1] {
                        acc.push('\\');
                        acc.push_str(seg);
                        let _ = vm.create_dir(&acc); // ignore AlreadyExists
                    }
                    vm.mount_file(&guest, data).unwrap();
                    count += 1;
                }
            }
        }
        let exe_base = std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy();
        let exe_guest = format!("C:\\game\\{exe_base}");
        println!("[trace] mounted {count} files from {dir} -> launching {exe_guest}");
        exe_guest
    } else {
        vm.mount_file("C:\\Users\\guest\\Desktop\\sample.exe", bytes)
            .unwrap();
        "C:\\Users\\guest\\Desktop\\sample.exe".to_string()
    };
    // Optional args via the ARGS env var (e.g. ARGS="-iwad doom1.wad").
    let pid = match std::env::var("ARGS") {
        Ok(args) => vm.launch_process_with_args(&exe_guest, &args).unwrap(),
        Err(_) => vm.launch_process(&exe_guest).unwrap(),
    };
    // Optional stdin via the STDIN env var (use \n for newlines).
    if let Ok(s) = std::env::var("STDIN") {
        let _ = vm.write_stdin(pid, &s.replace("\\n", "\n"));
    }
    let mut history: Vec<String> = Vec::new();
    let mut stdout = String::new();
    // WATCH=0xADDR[,0xADDR2] prints regs + key derefs every time EIP hits the
    // address (capped by WATCHMAX). For single-stepping hot routines like the
    // PBG3 bit reader. WATCHMAX default 200.
    let watch: Vec<u32> = std::env::var("WATCH")
        .ok()
        .map(|s| {
            s.split(',')
                .filter_map(|x| u32::from_str_radix(x.trim().trim_start_matches("0x"), 16).ok())
                .collect()
        })
        .unwrap_or_default();
    let watch_max: usize = std::env::var("WATCHMAX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let mut watch_hits = 0usize;

    // Drive execution one instruction at a time through the real VM so API
    // dispatch uses the actual registered handlers.
    for i in 0..max_steps {
        let proc = vm.processes.get(pid).unwrap();
        let eip = proc.cpu.eip;
        if watch.contains(&eip) && watch_hits < watch_max {
            watch_hits += 1;
            let c = &proc.cpu;
            let m = &proc.memory;
            // For the PBG3 reader (ESI=reader): mask@+0x10, byte@+0xC, cnt@+0x4,
            // checksum@+0x14, eof-flag@+0x1C.
            let mask = m.read_u32(c.esi + 0x10).unwrap_or(0xDEAD);
            let byte = m.read_u32(c.esi + 0xC).unwrap_or(0xDEAD);
            let cnt = m.read_u32(c.esi + 0x4).unwrap_or(0xDEAD);
            let flag = m.read_u32(c.esi + 0x1C).unwrap_or(0xDEAD);
            let _ = (mask, byte, cnt, flag);
            let rdstr = |addr: u32| -> String {
                let p = m.read_u32(addr).unwrap_or(0);
                (0..40)
                    .filter_map(|k| m.read_u8(p.wrapping_add(k)).ok())
                    .take_while(|&b| b != 0)
                    .map(|b| b as char)
                    .collect()
            };
            let arg1 = rdstr(c.esp + 4); // string at [esp+4]
            let ecx8 = m.read_u32(c.ecx + 8).unwrap_or(0xDEAD);
            let ecx10 = m.read_u32(c.ecx + 0x10).unwrap_or(0xDEAD);
            println!("[watch {i}] eip=0x{eip:08X} eax=0x{:08X} ecx=0x{:08X} esi=0x{:08X} arg1@[esp+4]={arg1:?} [ecx+8]=0x{ecx8:08X} [ecx+0x10]=0x{ecx10:08X}",
                c.eax, c.ecx, c.esi);
        }
        let disasm = if eip >= TRAMPOLINE_BASE {
            let name = vm
                .api
                .lookup_name(eip)
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
        if history.len() > 80 {
            history.remove(0);
        }

        // SLICE>1 runs faster for long GUI apps; SLICE=1 (default) gives
        // per-instruction history for precise crash diagnosis.
        let slice = std::env::var("SLICE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let r = vm.run_process_slice(pid, slice).unwrap();
        stdout.push_str(&r.stdout);
        for e in &r.ui_events {
            let d = format!("{e:?}");
            // Keep it short for blits (the pixel vec is huge).
            let d_short: String = d.chars().take(120).collect();
            println!("  [ui] {}", d_short);
        }
        if matches!(r.state, ProcessState::WaitingForInput) {
            println!("WAITING FOR INPUT after ~{} slices (window is up, message loop blocked in GetMessage)", i);
            println!("stdout: {stdout:?}");
            return;
        }
        match r.state {
            ProcessState::Exited { exit_code } => {
                println!("EXITED code={exit_code} after {i} steps");
                println!("stdout: {stdout:?}");
                let p = vm.processes.get(pid).unwrap();
                let stderr = String::from_utf8_lossy(&p.console.stderr).into_owned();
                if !stderr.is_empty() {
                    println!("stderr: {stderr:?}");
                }
                if std::env::var("HIST").is_ok() {
                    for line in &history {
                        println!("{line}");
                    }
                }
                let apis: Vec<String> = vm
                    .drain_logs()
                    .into_iter()
                    .filter(|e| e.target == "api")
                    .map(|e| e.message)
                    .collect();
                println!("--- last api ---");
                let take = std::env::var("APITAIL")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(25);
                for m in apis.iter().rev().take(take).rev() {
                    println!("  {m}");
                }
                return;
            }
            ProcessState::Crashed { reason } => {
                println!(
                    "CRASH after {i} steps: {reason}\n--- last {} instructions ---",
                    history.len()
                );
                for line in &history {
                    println!("{line}");
                }
                let p = vm.processes.get(pid).unwrap();
                println!(
                    "--- regs: esi=0x{:08X} edi=0x{:08X} ebx=0x{:08X} ebp=0x{:08X} ---",
                    p.cpu.esi, p.cpu.edi, p.cpu.ebx, p.cpu.ebp
                );
                for base in [p.cpu.esi, p.cpu.ebx] {
                    print!("  struct@0x{base:08X}:");
                    for off in 0..4u32 {
                        print!(
                            " [+{}]=0x{:08X}",
                            off * 4,
                            p.memory.read_u32(base + off * 4).unwrap_or(0xDEAD)
                        );
                    }
                    println!();
                }
                println!("--- cpu/api log ---");
                for ev in vm.drain_logs() {
                    if ev.target == "cpu" || ev.target == "api" {
                        println!("  [{}] {}", ev.target, ev.message);
                    }
                }
                return;
            }
            _ => {}
        }
    }
    println!(
        "ran {max_steps} steps, no exit; eip=0x{:08X}",
        vm.processes.get(pid).unwrap().cpu.eip
    );
    println!("--- last instructions ---");
    for line in history.iter() {
        println!("{line}");
    }
    println!("stdout so far: {stdout:?}");
}
