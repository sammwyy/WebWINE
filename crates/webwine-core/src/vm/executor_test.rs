use crate::top_vm::WebWineVm;
use crate::vm::process::ProcessState;

fn run_to_completion(vm: &mut WebWineVm, pid: u32) -> String {
    let mut stdout = String::new();
    loop {
        let r = vm.run_process_slice(pid, 100_000).expect("slice");
        stdout.push_str(&r.stdout);
        match r.state {
            ProcessState::Exited { .. } | ProcessState::Crashed { .. } => break,
            _ => {}
        }
    }
    stdout
}

#[test]
fn runs_minimal_sample() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../samples/target/i686-pc-windows-msvc/debug/minimal.exe"
    );
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("sample not built ({path}): {e}"));

    let mut vm = WebWineVm::new();
    vm.mount_file("C:\\Users\\guest\\Desktop\\minimal.exe", bytes)
        .expect("mount");

    let pid = vm
        .launch_process("C:\\Users\\guest\\Desktop\\minimal.exe")
        .expect("launch");

    let stdout = run_to_completion(&mut vm, pid);

    assert!(
        stdout.contains("Hello from WebWINE"),
        "expected output, got: {:?}",
        stdout
    );

    let proc = vm.get_process_info(pid).expect("info");
    assert!(
        matches!(proc.state, ProcessState::Exited { exit_code: 0 }),
        "expected exit 0, got: {:?}",
        proc.state
    );
}
