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

fn run_sample(name: &str) -> (String, ProcessState) {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/"
    );
    let path = format!("{dir}{name}");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("sample not built ({path}): {e}"));

    let mut vm = WebWineVm::new();
    let guest = format!("C:\\Users\\guest\\Desktop\\{name}");
    vm.mount_file(&guest, bytes).expect("mount");
    let pid = vm.launch_process(&guest).expect("launch");
    run_to_completion(&mut vm, pid)
}

#[test]
fn runs_minimal_sample() {
    let (stdout, state) = run_sample("minimal.exe");
    assert!(stdout.contains("Hello from WebWINE"), "got: {stdout:?}");
    assert!(matches!(state, ProcessState::Exited { exit_code: 0 }), "got: {state:?}");
}

#[test]
fn runs_hello_world_crt_sample() {
    // Full MSVC UCRT binary: exercises _initterm initializer tables, TLS setup,
    // and NtWriteFile-backed stdout.
    let (stdout, state) = run_sample("hello_world.exe");
    assert!(stdout.contains("Hello, World!"), "got: {stdout:?}");
    assert!(matches!(state, ProcessState::Exited { exit_code: 0 }), "got: {state:?}");
}

#[test]
fn files_sample_writes_to_vfs() {
    // Milestone 7: CreateFileA/WriteFile/CreateDirectoryA backed by the VFS.
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/"
    );
    let bytes = std::fs::read(format!("{dir}files.exe"))
        .unwrap_or_else(|e| panic!("files.exe not built: {e}"));

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
