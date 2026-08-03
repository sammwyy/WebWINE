//! webwine-cli — run a Windows .exe under WebWINE against the *real* host disk,
//! no browser required. It plays the role the browser would: it registers host
//! "drivers" with the core. Today that's the storage driver (a 1:1 passthrough
//! of a real directory); video/shell/sound events surface on the UiEvent stream
//! and are printed here.
//!
//! Usage:
//!   webwine [--disk=C] [--max=N] <path-to-real-exe> [program args...]
//!
//! The directory containing the exe is mounted 1:1 as the chosen guest drive
//! (default C:), and the exe is launched from there. Relative paths the program
//! opens resolve to the real files on disk; system DLLs resolve to built-in
//! stubs as usual.

mod passthrough;

use passthrough::PassthroughStorageDriver;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use webwine_core::vm::process::{ProcessState, UiEvent};
use webwine_core::WebWineVm;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut disk = 'C';
    let mut max_slices: u64 = 2_000_000;
    let mut max_was_set = false;
    let mut smoke: Option<String> = None;
    let mut exe: Option<String> = None;
    let mut prog_args: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(d) = a.strip_prefix("--disk=") {
            disk = d.chars().next().unwrap_or('C').to_ascii_uppercase();
        } else if let Some(m) = a.strip_prefix("--max=") {
            max_slices = m.parse().unwrap_or(max_slices);
            max_was_set = true;
        } else if let Some(root) = a.strip_prefix("--smoke=") {
            smoke = Some(root.to_string());
        } else if exe.is_none() {
            exe = Some(a.clone());
        } else {
            prog_args.push(a.clone());
        }
        i += 1;
    }

    if let Some(root) = smoke {
        let slices = if max_was_set { max_slices } else { 500 };
        std::process::exit(run_smoke(Path::new(&root), disk, slices));
    }

    let Some(exe) = exe else {
        eprintln!("usage: webwine [--disk=C] [--max=N] <real-exe-path> [program args...]");
        std::process::exit(2);
    };

    let exe_path = Path::new(&exe);
    let host_dir = exe_path.parent().filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let exe_name = exe_path.file_name().expect("exe filename").to_string_lossy().into_owned();
    let guest_exe = format!("{disk}:\\{exe_name}");

    let mut vm = WebWineVm::new();
    // Register the real directory as the chosen drive — 1:1, no copy. The driver
    // exposes its own unit via drives().
    vm.fs.register_storage_driver(Box::new(PassthroughStorageDriver::new(disk, &host_dir)));
    eprintln!("[webwine] {} -> {disk}:\\  (passthrough, no copy)", host_dir.display());
    eprintln!("[webwine] launching {guest_exe}");

    let argline = prog_args.join(" ");
    let pid = match vm.launch_process_with_args(&guest_exe, &argline) {
        Ok(pid) => pid,
        Err(e) => {
            eprintln!("[webwine] launch failed: {e}");
            std::process::exit(1);
        }
    };

    for n in 0..max_slices {
        let r = match vm.run_process_slice(pid, 4000) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[webwine] run error: {e}");
                break;
            }
        };
        if !r.stdout.is_empty() {
            print!("{}", r.stdout);
        }
        for e in &r.ui_events {
            match e {
                UiEvent::MessageBox { title, text, .. } => {
                    eprintln!("[messagebox] {title}: {text}");
                }
                UiEvent::CreateWindow { title, width, height, .. } => {
                    eprintln!("[window] \"{title}\" {width}x{height}");
                }
                UiEvent::Blit { hwnd, w, h, .. } => {
                    eprintln!("[blit] hwnd={hwnd} {w}x{h}");
                }
                other => eprintln!("[ui] {:.120?}", other),
            }
        }
        match r.state {
            ProcessState::Exited { exit_code } => {
                eprintln!("[webwine] exited code={exit_code} after {n} slices");
                std::process::exit(exit_code as i32);
            }
            ProcessState::Crashed { reason } => {
                eprintln!("[webwine] crashed: {reason}");
                std::process::exit(1);
            }
            ProcessState::WaitingForInput => {
                eprintln!("[webwine] window up; waiting for input (no GUI in CLI) — stopping");
                return;
            }
            _ => {}
        }
    }
    eprintln!("[webwine] reached slice budget without exit");
}

fn collect_exes(root: &Path, out: &mut Vec<PathBuf>) {
    if root.is_file() {
        if root.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
            out.push(root.to_path_buf());
        }
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_exes(&path, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
            out.push(path);
        }
    }
}

/// Reproducible compatibility sweep for a file or directory tree. Each image
/// gets a fresh VM and its own parent directory mounted as C:, matching normal
/// CLI launch while keeping crashes and writes isolated from other cases.
fn run_smoke(root: &Path, disk: char, max_slices: u64) -> i32 {
    let mut exes = Vec::new();
    collect_exes(root, &mut exes);
    exes.sort_by_key(|p| p.to_string_lossy().to_ascii_lowercase());
    if exes.is_empty() {
        eprintln!("[smoke] no .exe files found under {}", root.display());
        return 2;
    }

    println!("status\tarch\tinstructions\tui\tunimplemented\tms\tpath\tdetail");
    let mut failures = 0usize;
    let mut missing_apis: HashMap<String, usize> = HashMap::new();
    for exe in &exes {
        let started = Instant::now();
        let bytes = match std::fs::read(exe) {
            Ok(bytes) => bytes,
            Err(e) => {
                failures += 1;
                println!("read_error\t?\t0\t0\t0\t{}\t{}\t{}", started.elapsed().as_millis(), exe.display(), clean_field(&e.to_string()));
                continue;
            }
        };
        let arch = webwine_core::pe::inspector::inspect_bytes(&bytes)
            .map(|p| if p.is_pe32 { p.machine } else { format!("{}-PE32+", p.machine) })
            .unwrap_or_else(|_| "invalid".to_string());
        let host_dir = exe.parent().unwrap_or_else(|| Path::new("."));
        let exe_name = exe.file_name().unwrap_or_default().to_string_lossy();
        let guest = format!("{disk}:\\{exe_name}");
        let mut vm = WebWineVm::new();
        vm.fs.register_storage_driver(Box::new(PassthroughStorageDriver::new(disk, host_dir)));
        let pid = match vm.launch_process(&guest) {
            Ok(pid) => pid,
            Err(e) => {
                failures += 1;
                println!("launch_error\t{arch}\t0\t0\t0\t{}\t{}\t{}", started.elapsed().as_millis(), exe.display(), clean_field(&e.to_string()));
                continue;
            }
        };

        let mut instructions = 0u64;
        let mut ui = 0usize;
        let mut status = "budget";
        let mut detail = format!("slice limit {max_slices}");
        for _ in 0..max_slices {
            let result = match vm.run_process_slice(pid, 4_000) {
                Ok(result) => result,
                Err(e) => {
                    status = "run_error";
                    detail = e.to_string();
                    break;
                }
            };
            instructions += result.instructions as u64;
            ui += result.ui_events.len();
            match result.state {
                ProcessState::Exited { exit_code } => {
                    status = if exit_code == 0 { "exited" } else { "exit_error" };
                    detail = format!("code=0x{exit_code:08X}");
                    break;
                }
                ProcessState::Crashed { reason } => {
                    status = "crashed";
                    detail = reason;
                    break;
                }
                ProcessState::WaitingForInput => {
                    status = if ui > 0 { "interactive" } else { "blocked" };
                    detail = "waiting for host input".to_string();
                    break;
                }
                _ => {}
            }
        }
        let logs = vm.drain_logs();
        let unimplemented = logs.iter().filter(|e| e.message.contains("unimplemented:")).count();
        for entry in &logs {
            if let Some(rest) = entry.message.split("unimplemented:").nth(1) {
                let name = rest.split(" — ").next().unwrap_or(rest).trim();
                if !name.is_empty() {
                    *missing_apis.entry(name.to_string()).or_default() += 1;
                }
            }
        }
        if matches!(status, "crashed" | "run_error" | "launch_error" | "exit_error") {
            failures += 1;
        }
        println!("{status}\t{arch}\t{instructions}\t{ui}\t{unimplemented}\t{}\t{}\t{}",
            started.elapsed().as_millis(), exe.display(), clean_field(&detail));
    }
    if !missing_apis.is_empty() {
        let mut missing: Vec<_> = missing_apis.into_iter().collect();
        missing.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        eprintln!("[smoke] most frequent unimplemented APIs:");
        for (name, count) in missing.into_iter().take(25) {
            eprintln!("  {count:>5}  {name}");
        }
    }
    eprintln!("[smoke] {} executables, {} failures", exes.len(), failures);
    if failures == 0 { 0 } else { 1 }
}

fn clean_field(s: &str) -> String {
    s.replace(['\t', '\r', '\n'], " ")
}
