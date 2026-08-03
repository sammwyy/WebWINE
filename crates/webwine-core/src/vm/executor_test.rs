use crate::top_vm::WebWineVm;
use crate::vm::process::ProcessState;

fn run_to_completion(vm: &mut WebWineVm, pid: u32) -> (String, ProcessState) {
    let mut stdout = String::new();
    let mut slices = 0;
    loop {
        let r = vm.run_process_slice(pid, 200_000).expect("slice");
        stdout.push_str(&r.stdout);
        match r.state {
            ProcessState::Exited { .. } | ProcessState::Crashed { .. } => {
                return (stdout, r.state)
            }
            _ => {}
        }
        slices += 1;
        if slices > 50 {
            return (stdout, r.state); // safety cap against infinite loops
        }
    }
}

fn run_sample(name: &str) -> Option<(String, ProcessState)> {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/"
    );
    let path = format!("{dir}{name}");
    let Ok(bytes) = std::fs::read(&path) else { return None };

    let mut vm = WebWineVm::new();
    let guest = format!("C:\\Users\\guest\\Desktop\\{name}");
    vm.mount_file(&guest, bytes).expect("mount");
    let pid = vm.launch_process(&guest).expect("launch");
    Some(run_to_completion(&mut vm, pid))
}

#[test]
fn runs_minimal_sample() {
    let Some((stdout, state)) = run_sample("minimal.exe") else { return };
    assert!(stdout.contains("Hello from WebWINE"), "got: {stdout:?}");
    assert!(matches!(state, ProcessState::Exited { exit_code: 0 }), "got: {state:?}");
}

#[test]
fn runs_hello_world_crt_sample() {
    // Full MSVC UCRT binary: exercises _initterm initializer tables, TLS setup,
    // and NtWriteFile-backed stdout.
    let Some((stdout, state)) = run_sample("hello_world.exe") else { return };
    assert!(stdout.contains("Hello, World!"), "got: {stdout:?}");
    assert!(matches!(state, ProcessState::Exited { exit_code: 0 }), "got: {state:?}");
}

#[test]
fn gui_sample_creates_window_and_paints() {
    use crate::vm::process::UiEvent;

    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/"
    );
    let Ok(bytes) = std::fs::read(format!("{dir}gui.exe")) else { return };

    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\Users\\guest\\Desktop\\gui.exe", bytes).unwrap();
    let pid = vm.launch_process("C:\\Users\\guest\\Desktop\\gui.exe").unwrap();

    // Run until the message loop blocks in GetMessage.
    let mut ui: Vec<UiEvent> = Vec::new();
    for _ in 0..50 {
        let r = vm.run_process_slice(pid, 300_000).unwrap();
        ui.extend(r.ui_events);
        if matches!(r.state, ProcessState::WaitingForInput
            | ProcessState::Exited { .. } | ProcessState::Crashed { .. }) {
            break;
        }
    }

    // A window was created and painted via the WndProc.
    let hwnd = ui.iter().find_map(|e| match e {
        UiEvent::CreateWindow { hwnd, .. } => Some(*hwnd),
        _ => None,
    }).expect("CreateWindow event");

    let painted = ui.iter().any(|e| matches!(e,
        UiEvent::DrawText { text, .. } if text.contains("Hello from a WebWINE window")));
    assert!(painted, "expected paint via WndProc, ui: {ui:?}");

    // Close the window → WM_CLOSE → WM_DESTROY → PostQuitMessage → exit.
    vm.post_window_message(pid, hwnd, 0x0010, 0, 0).unwrap();

    let mut final_state = None;
    for _ in 0..50 {
        let r = vm.run_process_slice(pid, 300_000).unwrap();
        if matches!(r.state, ProcessState::Exited { .. } | ProcessState::Crashed { .. }) {
            final_state = Some(r.state);
            break;
        }
    }
    assert!(matches!(final_state, Some(ProcessState::Exited { exit_code: 0 })),
        "expected clean exit, got: {final_state:?}");
}

#[test]
fn graphics_sample_emits_gdi_and_beep() {
    use crate::vm::process::UiEvent;
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/"
    );
    let Ok(bytes) = std::fs::read(format!("{dir}graphics.exe")) else { return };

    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\Users\\guest\\Desktop\\graphics.exe", bytes).unwrap();
    let pid = vm.launch_process("C:\\Users\\guest\\Desktop\\graphics.exe").unwrap();

    let mut ui: Vec<UiEvent> = Vec::new();
    for _ in 0..50 {
        let r = vm.run_process_slice(pid, 300_000).unwrap();
        ui.extend(r.ui_events);
        if matches!(r.state, ProcessState::WaitingForInput
            | ProcessState::Exited { .. } | ProcessState::Crashed { .. }) { break; }
    }

    assert!(ui.iter().any(|e| matches!(e, UiEvent::Beep { .. })), "expected a beep");
    assert!(ui.iter().any(|e| matches!(e, UiEvent::CreateWindow { .. })), "expected a window");
    assert!(ui.iter().any(|e| matches!(e, UiEvent::Rect { .. })), "expected Rectangle");
    assert!(ui.iter().any(|e| matches!(e, UiEvent::Ellipse { .. })), "expected Ellipse");
    assert!(ui.iter().any(|e| matches!(e, UiEvent::Line { .. })), "expected LineTo");
}

#[test]
fn runs_filesystem_std_sample() {
    // Full Rust std binary doing std::fs file I/O. Exercises the optimized,
    // CMOV-heavy std allocation/Vec code paths (RawVec::finish_grow), so this
    // is the regression guard for conditional-move support in the interpreter.
    let mut vm = WebWineVm::new();
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/"
    );
    let Ok(bytes) = std::fs::read(format!("{dir}filesystem.exe")) else { return };
    vm.mount_file("C:\\Users\\guest\\Desktop\\filesystem.exe", bytes).unwrap();
    let pid = vm.launch_process("C:\\Users\\guest\\Desktop\\filesystem.exe").unwrap();
    let (stdout, state) = run_to_completion(&mut vm, pid);

    assert!(matches!(state, ProcessState::Exited { exit_code: 0 }), "got: {state:?}");
    assert!(stdout.contains("created hello.txt"), "got: {stdout:?}");

    let hello = vm.read_file("C:\\Users\\guest\\Desktop\\hello.txt").expect("hello.txt");
    assert_eq!(hello, b"world", "hello.txt content");
    let bar = vm.read_file("C:\\Users\\guest\\Desktop\\foo\\bar.txt").expect("foo\\bar.txt");
    assert_eq!(bar.len(), 0, "bar.txt should be empty");
}

#[test]
fn runs_managed_dotnet_sample() {
    // A real .NET 2.0 assembly launched through the full VM: it must be routed
    // to the CLR interpreter (not the x86 loader) and produce console output.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/10_dotnet_hello/dotnet_hello.exe"
    );
    let Ok(bytes) = std::fs::read(path) else { return }; // sample not built — skip

    let mut vm = WebWineVm::new();
    let guest = "C:\\Users\\guest\\Desktop\\dotnet_hello.exe";
    vm.mount_file(guest, bytes).expect("mount");
    let pid = vm.launch_process(guest).expect("launch managed");
    let (stdout, state) = run_to_completion(&mut vm, pid);

    assert!(matches!(state, ProcessState::Exited { exit_code: 0 }), "got: {state:?}");
    assert_eq!(stdout, "Hello from managed WebWINE!\n42\n", "stdout: {stdout:?}");
}

fn run_vendored_gui(relative: &str) -> Option<(ProcessState, usize)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/vendored")
        .join(relative);
    let Ok(bytes) = std::fs::read(&path) else { return None };
    let mut vm = WebWineVm::new();
    let name = path.file_name()?.to_string_lossy();
    let guest = format!("C:\\Users\\guest\\Desktop\\{name}");
    vm.mount_file(&guest, bytes).ok()?;
    let pid = vm.launch_process(&guest).ok()?;
    let mut ui_count = 0;
    for _ in 0..50 {
        let result = vm.run_process_slice(pid, 100_000).ok()?;
        ui_count += result.ui_events.len();
        if matches!(result.state, ProcessState::WaitingForInput | ProcessState::Exited { .. } | ProcessState::Crashed { .. }) {
            return Some((result.state, ui_count));
        }
    }
    Some((vm.get_process_info(pid).ok()?.state, ui_count))
}

#[test]
fn vendored_notepads_reach_interactive_windows() {
    for relative in ["notepad.exe", "Notepad/notepad-nt.exe"] {
        let Some((state, ui_count)) = run_vendored_gui(relative) else { continue };
        assert!(matches!(state, ProcessState::WaitingForInput), "{relative}: {state:?}");
        assert!(ui_count > 0, "{relative}: expected UI events");
    }
}

#[test]
fn vendored_mspaint_reaches_mfc_window_bridge() {
    let Some((state, ui_count)) = run_vendored_gui("mspaint.exe") else { return };
    assert!(matches!(state, ProcessState::WaitingForInput), "mspaint: {state:?}");
    assert!(ui_count >= 3, "mspaint: expected window, show, and canvas events");
}

#[test]
fn rejects_x64_executable() {
    // A PE32+ (x86-64) image must be rejected with a clear error, not crash.
    // Synthesize a minimal header with machine = 0x8664.
    let mut buf = vec![0u8; 0x200];
    buf[0] = b'M'; buf[1] = b'Z';
    let pe_off = 0x80usize;
    buf[0x3C] = pe_off as u8;
    buf[pe_off] = b'P'; buf[pe_off + 1] = b'E';
    buf[pe_off + 4] = 0x64; buf[pe_off + 5] = 0x86; // machine = 0x8664
    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\Users\\guest\\Desktop\\x64.exe", buf).unwrap();
    let err = vm.launch_process("C:\\Users\\guest\\Desktop\\x64.exe");
    assert!(err.is_err(), "x64 image should be rejected");
}

#[test]
fn spawn_sample_launches_child() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/"
    );
    let Ok(parent_bytes) = std::fs::read(format!("{dir}spawn.exe")) else { return };
    let Ok(child_bytes) = std::fs::read(format!("{dir}minimal.exe")) else { return };

    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\Users\\guest\\Desktop\\spawn.exe", parent_bytes).unwrap();
    vm.mount_file("C:\\Users\\guest\\Desktop\\minimal.exe", child_bytes).unwrap();

    let parent = vm.launch_process("C:\\Users\\guest\\Desktop\\spawn.exe").unwrap();

    // Run the parent; capture its stdout and the child it spawns.
    let mut parent_out = String::new();
    let mut child_pid = None;
    for _ in 0..50 {
        let r = vm.run_process_slice(parent, 200_000).unwrap();
        parent_out.push_str(&r.stdout);
        if let Some((cpid, _)) = r.spawned.first() { child_pid = Some(*cpid); }
        if matches!(r.state, ProcessState::Exited { .. } | ProcessState::Crashed { .. }) { break; }
    }

    assert!(parent_out.contains("child created"), "parent out: {parent_out:?}");
    let cpid = child_pid.expect("a child was spawned");

    // Run the child to completion and check it actually executed.
    let mut child_out = String::new();
    for _ in 0..50 {
        let r = vm.run_process_slice(cpid, 200_000).unwrap();
        child_out.push_str(&r.stdout);
        if matches!(r.state, ProcessState::Exited { .. } | ProcessState::Crashed { .. }) { break; }
    }
    assert!(child_out.contains("Hello from WebWINE"), "child out: {child_out:?}");
}

#[test]
fn files_sample_writes_to_vfs() {
    // Milestone 7: CreateFileA/WriteFile/CreateDirectoryA backed by the VFS.
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/"
    );
    let Ok(bytes) = std::fs::read(format!("{dir}files.exe")) else { return };

    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\Users\\guest\\Desktop\\files.exe", bytes).unwrap();
    let pid = vm.launch_process("C:\\Users\\guest\\Desktop\\files.exe").unwrap();
    let (stdout, state) = run_to_completion(&mut vm, pid);

    assert!(stdout.contains("done"), "got: {stdout:?}");
    assert!(matches!(state, ProcessState::Exited { exit_code: 0 }), "got: {state:?}");

    // hello.txt with "world"
    let hello = vm.read_file("C:\\Users\\guest\\Desktop\\hello.txt").expect("hello.txt");
    assert_eq!(hello, b"world", "hello.txt content");

    // foo/ directory with an (empty) bar.txt
    let bar = vm.read_file("C:\\Users\\guest\\Desktop\\foo\\bar.txt").expect("foo\\bar.txt");
    assert_eq!(bar.len(), 0, "bar.txt should be empty");
}

#[test]
fn messagebox_blocks_then_resumes_with_user_reply() {
    use crate::vm::process::UiEvent;
    // Native MinGW C sample at samples/build/ (gitignored); skip if not built.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/build/00_messagebox.exe");
    let Ok(bytes) = std::fs::read(path) else { return };

    let mut vm = WebWineVm::new();
    let guest = "C:\\Users\\guest\\Desktop\\00_messagebox.exe";
    vm.mount_file(guest, bytes).unwrap();
    let pid = vm.launch_process(guest).unwrap();

    // Run until the modal MessageBox blocks the process.
    let mut shown = false;
    for _ in 0..50 {
        let r = vm.run_process_slice(pid, 200_000).unwrap();
        if r.ui_events.iter().any(|e| matches!(e, UiEvent::MessageBox { .. })) {
            shown = true;
        }
        match r.state {
            ProcessState::WaitingForInput
            | ProcessState::Exited { .. }
            | ProcessState::Crashed { .. } => break,
            _ => {}
        }
    }
    assert!(shown, "MessageBox UiEvent was not emitted");
    assert!(
        matches!(vm.get_process_info(pid).unwrap().state, ProcessState::WaitingForInput),
        "process should block on the modal MessageBox, not return immediately"
    );

    // Click OK (IDOK = 1) and let it resume.
    vm.post_dialog_reply(pid, 1, None).unwrap();
    let (_out, state) = run_to_completion(&mut vm, pid);
    // 00_messagebox returns `IDOK ? 0 : 1`, so a click of OK exits 0.
    assert!(matches!(state, ProcessState::Exited { exit_code: 0 }), "got: {state:?}");
}
