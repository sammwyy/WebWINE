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
use std::path::Path;
use webwine_core::vm::process::{ProcessState, UiEvent};
use webwine_core::WebWineVm;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut disk = 'C';
    let mut max_slices: u64 = 2_000_000;
    let mut exe: Option<String> = None;
    let mut prog_args: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(d) = a.strip_prefix("--disk=") {
            disk = d.chars().next().unwrap_or('C').to_ascii_uppercase();
        } else if let Some(m) = a.strip_prefix("--max=") {
            max_slices = m.parse().unwrap_or(max_slices);
        } else if exe.is_none() {
            exe = Some(a.clone());
        } else {
            prog_args.push(a.clone());
        }
        i += 1;
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
    // Register the real directory as the chosen drive — 1:1, no copy.
    vm.fs.register_driver(disk, Box::new(PassthroughStorageDriver::new(&host_dir)));
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
