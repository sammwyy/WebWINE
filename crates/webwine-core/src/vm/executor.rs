use iced_x86::{Decoder, DecoderOptions, Instruction, Mnemonic, OpKind, Register};

use crate::error::Result;
use crate::fs::vfs::VirtualFileSystem;
use crate::logs::{LogBuffer, LogLevel};
use crate::vm::cpu::*;
use crate::vm::memory::GuestMemory;
use crate::vm::process::{GuestProcess, ProcessState};
use crate::winapi::{ApiContext, Handled, WinApiRegistry};

const TRAMPOLINE_BASE: u32 = 0x7FFE_0000;

pub enum StepResult {
    Continue,
    ApiTrap(u32),
    Exit(u32),
    Fault(String),
}

pub fn run_slice(
    proc: &mut GuestProcess,
    budget: u32,
    api: &WinApiRegistry,
    fs: &mut VirtualFileSystem,
    logs: &mut LogBuffer,
) -> Result<SliceResult> {
    match &proc.state {
        ProcessState::Exited { .. } | ProcessState::Crashed { .. } => {
            return Ok(SliceResult::done(proc));
        }
        _ => {}
    }
    proc.state = ProcessState::Running;

    let mut executed = 0u32;
    let mut prev_eip = proc.cpu.eip; // last instruction address, for crash reports
    // Consecutive faults absorbed by SEH without a clean instruction in between.
    // A garbage pointer makes every access fault and SEH resume forever; cap it
    // so the process crashes cleanly instead of hanging.
    let mut seh_fault_streak = 0u32;
    const SEH_FAULT_LIMIT: u32 = 512;

    loop {
        if executed >= budget {
            break;
        }
        let cur_eip = proc.cpu.eip;

        if proc.cpu.eip >= TRAMPOLINE_BASE {
            match handle_trampoline(proc, api, fs, logs, 0) {
                Flow::Continue => {}
                Flow::Block => {
                    // Suspend at the call site; resumed when a message arrives.
                    proc.state = ProcessState::WaitingForInput;
                    break;
                }
                Flow::Exit(code) => {
                    proc.state = ProcessState::Exited { exit_code: code };
                    break;
                }
                Flow::Fault(r) => {
                    proc.state = ProcessState::Crashed { reason: r };
                    break;
                }
            }
            executed += 1;
            continue;
        }

        match step(proc) {
            StepResult::Continue => {
                executed += 1;
                prev_eip = cur_eip;
                seh_fault_streak = 0; // a clean step breaks the fault streak
            }
            StepResult::ApiTrap(va) => {
                proc.cpu.eip = va; /* handled next iteration */
                prev_eip = cur_eip;
            }
            StepResult::Exit(code) => {
                proc.state = ProcessState::Exited { exit_code: code };
                break;
            }
            StepResult::Fault(r) => {
                let last = describe_instr(proc, prev_eip);
                logs.log(
                    LogLevel::Error,
                    "cpu",
                    &format!("[cpu] fault at EIP=0x{:08X}: {r}\n  last: {last}", proc.cpu.eip),
                    Some(proc.pid),
                );
                // Try SEH: walk the exception chain at fs:[0] (TEB+0x00).
                // Each node: { next: u32, handler: u32 }.  Sentinel = 0xFFFFFFFF.
                // We call handler(exceptionRecord, establisherFrame, context, dispatcher)
                // with a stub EXCEPTION_RECORD and CONTEXT.  If the handler returns
                // EXCEPTION_CONTINUE_EXECUTION (0) we resume; any other value crashes.
                let handled_by_seh = try_seh(proc, api, fs, logs, &r);
                if handled_by_seh {
                    // Handler absorbed the exception. EIP was updated (possibly via
                    // longjmp inside _except_handler4_common). Continue executing
                    // from wherever EIP now points — don't break the slice.
                    executed += 1;
                    seh_fault_streak += 1;
                    if seh_fault_streak >= SEH_FAULT_LIMIT {
                        // SEH keeps resuming into faulting code (garbage pointer /
                        // unhandled AV loop). Bail with a clear reason.
                        proc.state = ProcessState::Crashed {
                            reason: format!(
                                "SEH fault loop: {SEH_FAULT_LIMIT} consecutive faults absorbed near EIP=0x{:08X} ({r})",
                                proc.cpu.eip
                            ),
                        };
                        break;
                    }
                    continue;
                }
                proc.state = ProcessState::Crashed { reason: r };
                break;
            }
        }
    }

    Ok(SliceResult::done(proc))
}

/// Sentinel return address pushed before a nested guest call. Recognised by
/// `call_guest_fn` to detect when the called function returns.
const CALL_SENTINEL: u32 = 0xFFFF_FF00;

enum Flow {
    Continue,
    Block,
    Exit(u32),
    Fault(String),
}

/// Handle one API trampoline at the current EIP. May recurse into guest code
/// (for `_initterm`) via `call_guest_fn`. `depth` guards against runaway
/// re-entrancy.
fn handle_trampoline(
    proc: &mut GuestProcess,
    api: &WinApiRegistry,
    fs: &mut VirtualFileSystem,
    logs: &mut LogBuffer,
    depth: u32,
) -> Flow {
    let va = proc.cpu.eip;
    // Trace every API call (shown in "Run as debug" and useful for diagnosis).
    if let Some((dll, name)) = api.lookup_name(va) {
        logs.log(LogLevel::Trace, "api", &format!("{dll}!{name}"), Some(proc.pid));
    }
    let result = {
        let mut ctx = ApiContext {
            cpu: &mut proc.cpu,
            memory: &mut proc.memory,
            handles: &mut proc.handles,
            console: &mut proc.console,
            ui_events: &mut proc.ui_events,
            gui: &mut proc.gui,
            spawns: &mut proc.spawns,
            next_child_pid: proc.next_child_pid,
            heap_next: &mut proc.heap_next,
            heap_sizes: &mut proc.heap_sizes,
            fs,
            logs,
            pid: proc.pid,
            exe_path: proc.path.as_str(),
            cwd: &mut proc.cwd,
            cmdline: proc.cmdline.as_str(),
            messages: &proc.messages,
            proc_addr: api.proc_addr_map(),
            tls_slots: &mut proc.tls_slots,
            next_tls: &mut proc.next_tls,
            rand_seed: &mut proc.rand_seed,
        };
        api.dispatch(va, &mut ctx)
    };

    match result {
        Some(Handled::Ok) => Flow::Continue,
        Some(Handled::ExitProcess(code)) => Flow::Exit(code),
        Some(Handled::CallChain(funcs)) => {
            if depth < 8 {
                for pfn in funcs {
                    match call_guest_fn(proc, api, fs, logs, pfn, depth + 1) {
                        Flow::Continue => {}
                        other => return other,
                    }
                }
            }
            // Return from the _initterm call itself (cdecl).
            let ret = proc.memory.read_u32(proc.cpu.esp).unwrap_or(0);
            proc.cpu.esp = proc.cpu.esp.wrapping_add(4);
            proc.cpu.eax = 0;
            proc.cpu.eip = ret;
            Flow::Continue
        }
        Some(Handled::Block) => {
            // Leave EIP at the trampoline so the call re-dispatches on resume.
            Flow::Block
        }
        Some(Handled::Invoke { func, args, ret_args }) => {
            // Call the guest function (stdcall: callee cleans its own args).
            match call_guest_fn_args(proc, api, fs, logs, func, &args, depth + 1) {
                Flow::Continue => {}
                other => return other,
            }
            let result = proc.cpu.eax;
            // Return from the current API (e.g. DispatchMessage), stdcall cleanup.
            let ret = proc.memory.read_u32(proc.cpu.esp).unwrap_or(0);
            proc.cpu.esp = proc.cpu.esp.wrapping_add(4 + 4 * ret_args);
            proc.cpu.eax = result;
            proc.cpu.eip = ret;
            Flow::Continue
        }
        Some(Handled::Unimplemented) | None => {
            let nargs = api.unimpl_stdcall_args(va);
            let name = api
                .lookup_name(va)
                .map(|(d, n)| format!("{d}!{n}"))
                .unwrap_or_else(|| format!("0x{va:08X}"));
            logs.log(
                LogLevel::Warn,
                "api",
                &format!("[api] unimplemented: {name} — returning 0 (cleaned {nargs} args)"),
                Some(proc.pid),
            );
            // Clean the stack as the real (stdcall) function would, so a later
            // `ret` doesn't pop a leaked argument.
            let ret = proc.memory.read_u32(proc.cpu.esp).unwrap_or(0);
            proc.cpu.esp = proc.cpu.esp.wrapping_add(4 + 4 * nargs);
            proc.cpu.eax = 0;
            proc.cpu.eip = ret;
            Flow::Continue
        }
    }
}

/// Call a guest function with no args (cdecl). Used by `_initterm`.
fn call_guest_fn(
    proc: &mut GuestProcess,
    api: &WinApiRegistry,
    fs: &mut VirtualFileSystem,
    logs: &mut LogBuffer,
    target: u32,
    depth: u32,
) -> Flow {
    call_guest_fn_args(proc, api, fs, logs, target, &[], depth)
}

/// Call a guest function `target(args...)` and run until it returns.
/// Args are pushed right-to-left, then a sentinel return address; the matching
/// `ret` lands on the sentinel. For stdcall callees the callee cleans the args;
/// for cdecl with no args the stack is balanced regardless.
fn call_guest_fn_args(
    proc: &mut GuestProcess,
    api: &WinApiRegistry,
    fs: &mut VirtualFileSystem,
    logs: &mut LogBuffer,
    target: u32,
    args: &[u32],
    depth: u32,
) -> Flow {
    for &arg in args.iter().rev() {
        proc.cpu.esp = proc.cpu.esp.wrapping_sub(4);
        if proc.memory.write_u32(proc.cpu.esp, arg).is_err() {
            return Flow::Fault("stack overflow pushing call args".into());
        }
    }
    proc.cpu.esp = proc.cpu.esp.wrapping_sub(4);
    if proc.memory.write_u32(proc.cpu.esp, CALL_SENTINEL).is_err() {
        return Flow::Fault("stack overflow setting up guest call".into());
    }
    proc.cpu.eip = target;

    let mut budget = 20_000_000u32;
    loop {
        if proc.cpu.eip == CALL_SENTINEL {
            return Flow::Continue;
        }
        budget -= 1;
        if budget == 0 {
            return Flow::Fault(format!("guest function 0x{target:08X} did not return"));
        }

        if proc.cpu.eip >= TRAMPOLINE_BASE {
            match handle_trampoline(proc, api, fs, logs, depth) {
                Flow::Continue => {}
                other => return other,
            }
            continue;
        }

        match step(proc) {
            StepResult::Continue => {}
            StepResult::ApiTrap(va) => proc.cpu.eip = va,
            StepResult::Exit(code) => return Flow::Exit(code),
            StepResult::Fault(r) => {
                logs.log(
                    LogLevel::Error,
                    "cpu",
                    &format!("[cpu] fault in init at EIP=0x{:08X}: {r}", proc.cpu.eip),
                    Some(proc.pid),
                );
                return Flow::Fault(r);
            }
        }
    }
}

/// Attempt to handle a CPU fault via the Win32 SEH chain (fs:[0]).
///
/// Walks the EXCEPTION_REGISTRATION_RECORD chain stored at TEB+0x00.
/// For each node {next, handler} we build a minimal EXCEPTION_RECORD and
/// CONTEXT on the guest stack and call `handler(record, frame, ctx, dispatch)`.
///
/// **MSVC `_except_handler4_common` behaviour**: instead of returning a
/// disposition code it calls `longjmp` internally, which restores EIP and ESP
/// from the jmp_buf saved by `_setjmp3`.  Our `longjmp_fn` updates cpu.eip /
/// cpu.esp directly, so after `call_guest_fn_args` returns the new EIP will
/// be the `__except`-block address — NOT the CALL_SENTINEL.
///
/// Returns `true` if a handler absorbed the exception (process continues),
/// `false` if no handler was found (caller should crash the process).
fn try_seh(
    proc: &mut GuestProcess,
    api: &WinApiRegistry,
    fs: &mut VirtualFileSystem,
    logs: &mut LogBuffer,
    reason: &str,
) -> bool {
    use crate::pe::loader::TEB_VA;
    // Exception disposition values (for handlers that return normally).
    const EXCEPTION_CONTINUE_EXECUTION: u32 = 0;
    const EXCEPTION_EXECUTE_HANDLER:    u32 = 0xFFFF_FFFF; // -1i32 as u32
    const EXCEPTION_CODE: u32 = 0xC000_0005; // STATUS_ACCESS_VIOLATION

    // ExceptionList is at TEB+0x00.
    let mut node = proc.memory.read_u32(TEB_VA).unwrap_or(0xFFFF_FFFF);
    let fault_eip = proc.cpu.eip;
    let fault_esp = proc.cpu.esp;
    let mut depth = 0u32;

    while node != 0xFFFF_FFFF && node != 0 && depth < 32 {
        depth += 1;
        let handler = proc.memory.read_u32(node + 4).unwrap_or(0);
        if handler == 0 {
            node = proc.memory.read_u32(node).unwrap_or(0xFFFF_FFFF);
            continue;
        }

        let mut resolved_handler = handler;
        
        if handler != 0 && handler < TRAMPOLINE_BASE {
            // Check if it's an IAT thunk: jmp dword ptr [iat_addr] (FF 25 xx xx xx xx)
            let bytes = proc.memory.read_bytes(handler, 6).unwrap_or_default();
            if bytes.len() == 6 && bytes[0] == 0xFF && bytes[1] == 0x25 {
                let iat_addr = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
                let dest = proc.memory.read_u32(iat_addr).unwrap_or(0);
                resolved_handler = dest;
            }
        }

        if resolved_handler >= TRAMPOLINE_BASE {
            // Check if it resolves to _except_handler3. If so, handle it natively.
            let mut is_eh3 = false;
            if let Some((_, name)) = api.lookup_name(resolved_handler) {
                if name == "_except_handler3" {
                    is_eh3 = true;
                }
            }

            if is_eh3 {
                let scopetable = proc.memory.read_u32(node + 8).unwrap_or(0);
                let trylevel = proc.memory.read_u32(node + 12).unwrap_or(0xFFFF_FFFF);
                let _ebp = proc.memory.read_u32(node + 16).unwrap_or(0);

                let mut level = trylevel as i32;
                let mut absorbed = false;
                
                while level != -1 {
                    let entry = scopetable + (level as u32) * 12;
                    let enclosing = proc.memory.read_u32(entry).unwrap_or(0xFFFF_FFFF) as i32;
                    let filter = proc.memory.read_u32(entry + 4).unwrap_or(0);
                    let specific_handler = proc.memory.read_u32(entry + 8).unwrap_or(0);
                    
                    let mut filter_action = 1; // EXCEPTION_EXECUTE_HANDLER if filter is null
                    if filter != 0 {
                        // evaluate filter
                        let saved_ebp = proc.cpu.ebp;
                        let saved_eip = proc.cpu.eip;
                        let saved_esp = proc.cpu.esp;
                        
                        proc.cpu.ebp = _ebp; // MSVC filters expect EBP = _ebp
                        let flow = call_guest_fn_args(proc, api, fs, logs, filter, &[], depth + 1);
                        
                        // We must restore CPU registers that shouldn't be clobbered
                        let action = proc.cpu.eax as i32;
                        proc.cpu.ebp = saved_ebp;
                        proc.cpu.eip = saved_eip;
                        proc.cpu.esp = saved_esp;
                        
                        if let Flow::Continue = flow {
                            filter_action = action;
                        } else {
                            // If filter faulted, ignore and keep searching
                            level = enclosing;
                            continue;
                        }
                    }
                    
                    if filter_action == 1 { // EXCEPTION_EXECUTE_HANDLER
                        logs.log(LogLevel::Info, "seh", &format!("_except_handler3 executing handler at 0x{:08X}", specific_handler), Some(proc.pid));
                        // Jump to handler.
                        // MSVC specific_handler expects EBP = _ebp, and ESP is usually restored.
                        // To be safe, we set EBP = _ebp, and leave ESP at the fault_esp (or maybe the node?).
                        // Usually local variables are accessed via EBP. We will restore EBP and jump.
                        proc.cpu.ebp = _ebp;
                        // ESP must be valid. The handler might assume ESP is below its locals.
                        // We'll set ESP to the node itself (the registration frame), minus a little buffer, 
                        // as it's safe. But fault_esp is also fine. Let's use fault_esp.
                        proc.cpu.esp = fault_esp;
                        proc.cpu.eip = specific_handler;
                        return true;
                    } else if filter_action == -1 { // EXCEPTION_CONTINUE_EXECUTION
                        logs.log(LogLevel::Info, "seh", &format!("_except_handler3 continuing execution"), Some(proc.pid));
                        proc.cpu.esp = fault_esp;
                        return true;
                    }
                    
                    level = enclosing;
                }
                
                // If we get here, no handler in the scopetable caught it.
                // Move to next node.
                node = proc.memory.read_u32(node).unwrap_or(0xFFFF_FFFF);
                continue;
            }

            // Skip invalid / trampoline "handlers" — not real SEH handlers.
            node = proc.memory.read_u32(node).unwrap_or(0xFFFF_FFFF);
            continue;
        }

        // Build a minimal EXCEPTION_RECORD on the guest stack.
        // Layout: ExceptionCode, ExceptionFlags, NextRecord*, ExceptionAddress,
        //         NumberParameters  — 5 dwords = 20 bytes.
        let rec_va = fault_esp.wrapping_sub(20);
        let _ = proc.memory.write_u32(rec_va,      EXCEPTION_CODE);
        let _ = proc.memory.write_u32(rec_va + 4,  0);              // ExceptionFlags
        let _ = proc.memory.write_u32(rec_va + 8,  0);              // NextRecord = NULL
        let _ = proc.memory.write_u32(rec_va + 12, fault_eip);      // ExceptionAddress
        let _ = proc.memory.write_u32(rec_va + 16, 0);              // NumberParameters

        // Build a minimal CONTEXT_i386 (0x2CC bytes).
        // We fill ContextFlags, EIP, and ESP; everything else is zeroed.
        const CTX_SIZE: u32 = 0x2CC;
        let ctx_va = rec_va.wrapping_sub(CTX_SIZE);
        let _ = proc.memory.write_bytes(ctx_va, &vec![0u8; CTX_SIZE as usize]);
        let _ = proc.memory.write_u32(ctx_va,        0x0001_0007); // CONTEXT_FULL
        let _ = proc.memory.write_u32(ctx_va + 0xB8, fault_eip);   // EIP offset
        let _ = proc.memory.write_u32(ctx_va + 0xC4, fault_esp);   // ESP offset

        // Prepare ESP below the on-stack structures and call the handler.
        // handler(ExceptionRecord*, EstablisherFrame*, ContextRecord*, Dispatcher*)
        // stdcall — 4 args (callee cleans the stack).
        let args = [rec_va, node, ctx_va, 0u32];
        let esp_for_handler = ctx_va.wrapping_sub(16); // room below context

        let saved_eip = proc.cpu.eip;
        let saved_esp = proc.cpu.esp;
        proc.cpu.eip = fault_eip; // in case handler reads it (shouldn't matter)
        proc.cpu.esp = esp_for_handler;

        let flow = call_guest_fn_args(proc, api, fs, logs, handler, &args, 1);

        // ── Check if longjmp fired ──────────────────────────────────────────
        // If _except_handler4_common called longjmp, our longjmp_fn already
        // restored cpu.eip/esp to the __except block.  Detect this by checking
        // whether EIP now points to real (mapped) code that is NOT the sentinel.
        let eip_after = proc.cpu.eip;
        let longjmp_fired = eip_after != CALL_SENTINEL
            && eip_after != 0
            && eip_after < TRAMPOLINE_BASE
            && proc.memory.read_instruction_window(eip_after).is_ok();

        if longjmp_fired {
            // longjmp redirected execution to the __except block.  The handler
            // already restored ESP from the jmp_buf, so we don't touch it.
            logs.log(LogLevel::Info, "seh",
                &format!("SEH longjmp → 0x{eip_after:08X} for fault: {reason}"),
                Some(proc.pid));
            return true;
        }

        // Handler returned normally (or faulted).
        match flow {
            Flow::Continue => {}
            _ => {
                // Handler faulted or the process exited — restore and search on.
                proc.cpu.eip = saved_eip;
                proc.cpu.esp = saved_esp;
                node = proc.memory.read_u32(node).unwrap_or(0xFFFF_FFFF);
                continue;
            }
        }

        let retval = proc.cpu.eax;
        if retval == EXCEPTION_CONTINUE_EXECUTION {
            logs.log(LogLevel::Info, "seh",
                &format!("SEH handler at 0x{handler:08X} absorbed fault: {reason}"),
                Some(proc.pid));
            // Restore ESP to the pre-fault position; keep EIP from handler.
            proc.cpu.esp = fault_esp;
            return true;
        }
        if retval == EXCEPTION_EXECUTE_HANDLER {
            logs.log(LogLevel::Info, "seh",
                &format!("SEH EXECUTE_HANDLER at 0x{handler:08X}: {reason}"),
                Some(proc.pid));
            proc.cpu.esp = fault_esp;
            return true;
        }
        // EXCEPTION_CONTINUE_SEARCH (1) — try the next node in the chain.
        node = proc.memory.read_u32(node).unwrap_or(0xFFFF_FFFF);
    }
    false
}

/// Decode the instruction at `addr` for a crash report. For memory-operand
/// control transfers (e.g. `call [iat]`) it also resolves the pointer value,
/// which usually reveals an unpatched import / null function pointer.
fn describe_instr(proc: &GuestProcess, addr: u32) -> String {
    let bytes = match proc.memory.read_instruction_window(addr) {
        Ok(b) => b,
        Err(_) => return format!("0x{addr:08X} <unreadable>"),
    };
    let mut dec = Decoder::with_ip(32, bytes, addr as u64, DecoderOptions::NONE);
    let mut ins = Instruction::default();
    dec.decode_out(&mut ins);

    let raw: Vec<String> = bytes[..ins.len().min(bytes.len())]
        .iter().map(|b| format!("{b:02X}")).collect();
    let mut s = format!("0x{addr:08X}: {:?} [{}]", ins.mnemonic(), raw.join(" "));

    // If an operand is memory, resolve its address and the dword stored there.
    for i in 0..ins.op_count() {
        if ins.op_kind(i) == OpKind::Memory {
            let mem_addr = calc_addr(&ins, &proc.cpu);
            let val = proc.memory.read_u32(mem_addr).unwrap_or(0xDEAD_BEEF);
            s.push_str(&format!("  mem[0x{mem_addr:08X}] = 0x{val:08X}"));
        } else if ins.op_kind(i) == OpKind::Register {
            let r = ins.op_register(i);
            s.push_str(&format!("  {:?}=0x{:08X}", r, read_reg(r, &proc.cpu)));
        }
    }
    s
}

pub fn step(proc: &mut GuestProcess) -> StepResult {
    let eip = proc.cpu.eip;
    let bytes = match proc.memory.read_instruction_window(eip) {
        Ok(b) => b,
        Err(e) => return StepResult::Fault(format!("fetch 0x{eip:08X}: {e}")),
    };

    let mut dec = Decoder::with_ip(32, bytes, eip as u64, DecoderOptions::NONE);
    let mut instr = Instruction::default();
    dec.decode_out(&mut instr);

    if instr.is_invalid() {
        return StepResult::Fault(format!(
            "invalid opcode at 0x{eip:08X}: {:02X?}",
            &bytes[..bytes.len().min(4)]
        ));
    }

    proc.cpu.eip = proc.cpu.eip.wrapping_add(instr.len() as u32);

    execute(&instr, &mut proc.cpu, &mut proc.memory)
}

fn execute(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    use Mnemonic::*;
    match instr.mnemonic() {
        Nop | Pause => StepResult::Continue,

        Mov => exec_mov(instr, cpu, mem),
        Movzx => exec_movzx(instr, cpu, mem),
        Movsx => exec_movsx(instr, cpu, mem),
        Xchg => exec_xchg(instr, cpu, mem),
        Cmpxchg => exec_cmpxchg(instr, cpu, mem),
        Xadd => exec_xadd(instr, cpu, mem),
        Lea => exec_lea(instr, cpu, mem),

        Push => exec_push(instr, cpu, mem),
        Pop => exec_pop(instr, cpu, mem),
        Pushad => exec_pushad(cpu, mem),
        Popad => exec_popad(cpu, mem),
        Pushfd => exec_pushfd(cpu, mem),
        Popfd => exec_popfd(cpu, mem),

        Add => exec_alu(instr, cpu, mem, AluOp::Add),
        Sub => exec_alu(instr, cpu, mem, AluOp::Sub),
        Adc => exec_alu(instr, cpu, mem, AluOp::Adc),
        Sbb => exec_alu(instr, cpu, mem, AluOp::Sbb),
        And => exec_alu(instr, cpu, mem, AluOp::And),
        Or => exec_alu(instr, cpu, mem, AluOp::Or),
        Xor => exec_alu(instr, cpu, mem, AluOp::Xor),
        Cmp => exec_alu(instr, cpu, mem, AluOp::Cmp),
        Test => exec_alu(instr, cpu, mem, AluOp::Test),
        Not => exec_not(instr, cpu, mem),
        Neg => exec_neg(instr, cpu, mem),
        Inc => exec_inc(instr, cpu, mem),
        Dec => exec_dec(instr, cpu, mem),
        Imul => exec_imul(instr, cpu, mem),
        Mul => exec_mul(instr, cpu, mem),
        Idiv => exec_idiv(instr, cpu, mem),
        Div => exec_div(instr, cpu, mem),
        Cdq => {
            cpu.edx = if cpu.eax >> 31 != 0 { 0xFFFF_FFFF } else { 0 };
            StepResult::Continue
        }
        Cwde => {
            cpu.eax = (cpu.eax as i16 as i32) as u32;
            StepResult::Continue
        }

        Shl | Sal => exec_shift(instr, cpu, mem, ShiftOp::Shl),
        Shr => exec_shift(instr, cpu, mem, ShiftOp::Shr),
        Sar => exec_shift(instr, cpu, mem, ShiftOp::Sar),
        Rol => exec_shift(instr, cpu, mem, ShiftOp::Rol),
        Ror => exec_shift(instr, cpu, mem, ShiftOp::Ror),

        Call => exec_call(instr, cpu, mem),
        Ret => exec_ret(instr, cpu, mem),
        Jmp => exec_jmp(instr, cpu, mem),

        Je => exec_jcc(instr, cpu, get_zf(cpu.eflags)),
        Jne => exec_jcc(instr, cpu, !get_zf(cpu.eflags)),
        Jl => exec_jcc(instr, cpu, get_sf(cpu.eflags) != get_of(cpu.eflags)),
        Jle => exec_jcc(
            instr,
            cpu,
            get_zf(cpu.eflags) || get_sf(cpu.eflags) != get_of(cpu.eflags),
        ),
        Jg => exec_jcc(
            instr,
            cpu,
            !get_zf(cpu.eflags) && get_sf(cpu.eflags) == get_of(cpu.eflags),
        ),
        Jge => exec_jcc(instr, cpu, get_sf(cpu.eflags) == get_of(cpu.eflags)),
        Jb => exec_jcc(instr, cpu, get_cf(cpu.eflags)),
        Jbe => exec_jcc(instr, cpu, get_cf(cpu.eflags) || get_zf(cpu.eflags)),
        Ja => exec_jcc(instr, cpu, !get_cf(cpu.eflags) && !get_zf(cpu.eflags)),
        Jae => exec_jcc(instr, cpu, !get_cf(cpu.eflags)),
        Js => exec_jcc(instr, cpu, get_sf(cpu.eflags)),
        Jns => exec_jcc(instr, cpu, !get_sf(cpu.eflags)),
        Jo => exec_jcc(instr, cpu, get_of(cpu.eflags)),
        Jno => exec_jcc(instr, cpu, !get_of(cpu.eflags)),
        Jp => exec_jcc(instr, cpu, get_pf(cpu.eflags)),
        Jnp => exec_jcc(instr, cpu, !get_pf(cpu.eflags)),
        Jecxz => exec_jcc(instr, cpu, cpu.ecx == 0),
        Loop => {
            cpu.ecx = cpu.ecx.wrapping_sub(1);
            exec_jcc(instr, cpu, cpu.ecx != 0)
        }
        Loope => {
            cpu.ecx = cpu.ecx.wrapping_sub(1);
            exec_jcc(instr, cpu, cpu.ecx != 0 && get_zf(cpu.eflags))
        }
        Loopne => {
            cpu.ecx = cpu.ecx.wrapping_sub(1);
            exec_jcc(instr, cpu, cpu.ecx != 0 && !get_zf(cpu.eflags))
        }

        Sete => exec_setcc(instr, cpu, mem, get_zf(cpu.eflags)),
        Setne => exec_setcc(instr, cpu, mem, !get_zf(cpu.eflags)),
        Setl => exec_setcc(instr, cpu, mem, get_sf(cpu.eflags) != get_of(cpu.eflags)),
        Setle => exec_setcc(
            instr,
            cpu,
            mem,
            get_zf(cpu.eflags) || get_sf(cpu.eflags) != get_of(cpu.eflags),
        ),
        Setg => exec_setcc(
            instr,
            cpu,
            mem,
            !get_zf(cpu.eflags) && get_sf(cpu.eflags) == get_of(cpu.eflags),
        ),
        Setge => exec_setcc(instr, cpu, mem, get_sf(cpu.eflags) == get_of(cpu.eflags)),
        Setb => exec_setcc(instr, cpu, mem, get_cf(cpu.eflags)),
        Setbe => exec_setcc(instr, cpu, mem, get_cf(cpu.eflags) || get_zf(cpu.eflags)),
        Seta => exec_setcc(instr, cpu, mem, !get_cf(cpu.eflags) && !get_zf(cpu.eflags)),
        Setae => exec_setcc(instr, cpu, mem, !get_cf(cpu.eflags)),
        Sets => exec_setcc(instr, cpu, mem, get_sf(cpu.eflags)),
        Setns => exec_setcc(instr, cpu, mem, !get_sf(cpu.eflags)),

        // Conditional moves (CMOVcc): dst = src if condition. Flags unaffected.
        Cmove  => exec_cmovcc(instr, cpu, mem, get_zf(cpu.eflags)),
        Cmovne => exec_cmovcc(instr, cpu, mem, !get_zf(cpu.eflags)),
        Cmovl  => exec_cmovcc(instr, cpu, mem, get_sf(cpu.eflags) != get_of(cpu.eflags)),
        Cmovle => exec_cmovcc(instr, cpu, mem, get_zf(cpu.eflags) || get_sf(cpu.eflags) != get_of(cpu.eflags)),
        Cmovg  => exec_cmovcc(instr, cpu, mem, !get_zf(cpu.eflags) && get_sf(cpu.eflags) == get_of(cpu.eflags)),
        Cmovge => exec_cmovcc(instr, cpu, mem, get_sf(cpu.eflags) == get_of(cpu.eflags)),
        Cmovb  => exec_cmovcc(instr, cpu, mem, get_cf(cpu.eflags)),
        Cmovbe => exec_cmovcc(instr, cpu, mem, get_cf(cpu.eflags) || get_zf(cpu.eflags)),
        Cmova  => exec_cmovcc(instr, cpu, mem, !get_cf(cpu.eflags) && !get_zf(cpu.eflags)),
        Cmovae => exec_cmovcc(instr, cpu, mem, !get_cf(cpu.eflags)),
        Cmovs  => exec_cmovcc(instr, cpu, mem, get_sf(cpu.eflags)),
        Cmovns => exec_cmovcc(instr, cpu, mem, !get_sf(cpu.eflags)),
        Cmovo  => exec_cmovcc(instr, cpu, mem, get_of(cpu.eflags)),
        Cmovno => exec_cmovcc(instr, cpu, mem, !get_of(cpu.eflags)),
        Cmovp  => exec_cmovcc(instr, cpu, mem, get_pf(cpu.eflags)),
        Cmovnp => exec_cmovcc(instr, cpu, mem, !get_pf(cpu.eflags)),

        Leave => {
            cpu.esp = cpu.ebp;
            match mem.read_u32(cpu.esp) {
                Ok(v) => {
                    cpu.ebp = v;
                    cpu.esp = cpu.esp.wrapping_add(4);
                    StepResult::Continue
                }
                Err(e) => StepResult::Fault(e.to_string()),
            }
        }

        Stosd => exec_stos(instr, cpu, mem, 4),
        Stosw => exec_stos(instr, cpu, mem, 2),
        Stosb => exec_stos(instr, cpu, mem, 1),
        // MOVSD is ambiguous: both the string op (A5) and SSE scalar double (F2 0F 10/11)
        // use the same iced-x86 mnemonic. Dispatch by checking for XMM operands.
        Movsd => {
            let has_xmm = (0..instr.op_count())
                .any(|i| instr.op_kind(i) == OpKind::Register && xmm_idx(instr.op_register(i)).is_some());
            if has_xmm { exec_xmm_mov(instr, cpu, mem) }
            else        { exec_movs(instr, cpu, mem, 4) }
        }
        Movsw => exec_movs(instr, cpu, mem, 2),
        Movsb => exec_movs(instr, cpu, mem, 1),
        Scasd => exec_scas(instr, cpu, mem, 4),
        Scasw => exec_scas(instr, cpu, mem, 2),
        Scasb => exec_scas(instr, cpu, mem, 1),
        Lodsd => match mem.read_u32(cpu.esi) {
            Ok(v) => {
                cpu.eax = v;
                cpu.esi = cpu.esi.wrapping_add(4);
                StepResult::Continue
            }
            Err(e) => StepResult::Fault(e.to_string()),
        },
        Lodsb => match mem.read_u8(cpu.esi) {
            Ok(v) => {
                cpu.eax = (cpu.eax & 0xFFFFFF00) | v as u32;
                cpu.esi = cpu.esi.wrapping_add(1);
                StepResult::Continue
            }
            Err(e) => StepResult::Fault(e.to_string()),
        },

        // SSE / XMM 
        Xorps | Xorpd | Pxor => exec_xmm_binop(instr, cpu, mem, |a, b| xmm_xor(a, b)),
        Andps | Andpd | Pand | Andnps | Andnpd | Pandn
            => exec_xmm_binop(instr, cpu, mem, |a, b| xmm_and(a, b)),
        Orps | Orpd | Por => exec_xmm_binop(instr, cpu, mem, |a, b| xmm_or(a, b)),

        // 128-bit moves
        Movaps | Movups | Movdqa | Movdqu | Movapd | Movupd => exec_xmm_mov(instr, cpu, mem),
        // scalar SSE moves (Movsd already dispatched above)
        Movss => exec_xmm_mov(instr, cpu, mem),
        // low/high 64-bit partial moves
        Movlpd | Movlps => exec_movlp(instr, cpu, mem),
        Movhpd | Movhps => exec_movhp(instr, cpu, mem),
        // non-temporal stores — treat as regular stores
        Movntps | Movntpd | Movntdq | Movnti => exec_xmm_mov(instr, cpu, mem),

        Movd | Movq => exec_movd(instr, cpu, mem),

        Pcmpeqb | Pcmpeqw | Pcmpeqd => exec_pcmpeq(instr, cpu, mem),
        Pmovmskb => exec_pmovmskb(instr, cpu, mem),
        Psrldq | Pslldq => exec_pshift_dq(instr, cpu, mem),
        Punpcklbw | Punpcklwd | Punpckldq | Punpcklqdq => exec_punpckl(instr, cpu, mem),
        Punpckhbw | Punpckhwd | Punpckhdq | Punpckhqdq => exec_punpckh(instr, cpu, mem),

        // packed int — stub (wrong values are acceptable for CRT init)
        Paddb | Paddw | Paddd | Paddq
        | Psubb | Psubw | Psubd | Psubq
        | Pminub | Pmaxub | Pminuw | Pmaxuw | Pminud | Pmaxud
        | Pmullw | Pmulhw | Pmulhuw | Pmulld
        | Psrlw | Psrld | Psrlq | Psllw | Pslld | Psllq | Psraw | Psrad
        | Pshufb | Pshuflw | Pshufhw | Pshufd | Shufps | Shufpd
        | Palignr | Pblendw | Pblendvb | Blendvps | Blendvpd | Blendps | Blendpd
        | Pabsb | Pabsw | Pabsd
        | Packuswb | Packusdw | Packsswb | Packssdw
        => exec_xmm_mov(instr, cpu, mem),

        // scalar FP arithmetic — stub to keep XMM state from crashing
        Addss | Addsd | Subss | Subsd | Mulss | Mulsd | Divss | Divsd
        | Maxss | Maxsd | Minss | Minsd | Sqrtss | Sqrtsd | Rcpss | Rsqrtss
        => exec_xmm_mov(instr, cpu, mem),

        // FP comparisons — set EFLAGS conservatively (ZF=0, CF=0, PF=0 = "unordered false")
        Ucomiss | Ucomisd | Comiss | Comisd => { cpu.eflags &= !(CF | ZF | PF); StepResult::Continue }

        // FP conversions — zero the destination
        Cvtsi2ss | Cvtsi2sd | Cvttss2si | Cvttsd2si | Cvtss2si | Cvtsd2si
        | Cvtss2sd | Cvtsd2ss | Cvtdq2ps | Cvtdq2pd | Cvtps2dq | Cvtpd2dq
        | Cvttps2dq | Cvttpd2dq
        => exec_xmm_mov(instr, cpu, mem),

        // FP control
        Ldmxcsr | Stmxcsr | Fldcw | Fnstcw | Fstcw => exec_sse_ctrl(instr, cpu, mem),

        // x87 FPU — we don't track the FP stack; treat as no-ops
        Fld | Fld1 | Fldz | Fldpi | Fldl2e | Fldl2t | Fldlg2 | Fldln2
        | Fst | Fstp | Fild | Fistp | Fist | Fisttp
        | Fadd | Faddp | Fiadd | Fsub | Fsubp | Fsubr | Fsubrp | Fisub | Fisubr
        | Fmul | Fmulp | Fimul | Fdiv | Fdivp | Fdivr | Fdivrp | Fidiv | Fidivr
        | Fabs | Fchs | Fsqrt | Frndint | Fscale | Fprem | Fprem1 | Fxtract | F2xm1
        | Fsin | Fcos | Fsincos | Fptan | Fpatan | Fyl2x | Fyl2xp1
        | Fcom | Fcomp | Fcompp | Fucom | Fucomp | Fucompp | Fcomi | Fcomip | Fucomi | Fucomip
        | Ftst | Fxam | Fnstsw | Fstenv | Fldenv | Fsave | Frstor | Fnclex | Fninit | Fnop
        | Sahf | Emms | Ffree | Fxch
        | Wait
        => StepResult::Continue,

        Int3 => StepResult::Fault("INT3 breakpoint".into()),
        Int   => {
            let v = instr.immediate8();
            if v == 0x29 {
                // __fastfail — the program is aborting; ECX holds the code.
                let code = cpu.ecx;
                let kind = match code {
                    0 => "LEGACY_GS_VIOLATION",
                    2 => "STACK_COOKIE_CHECK_FAILURE",
                    3 => "CORRUPT_LIST_ENTRY",
                    5 => "INVALID_ARG",
                    7 => "FATAL_APP_EXIT",
                    8 => "RANGE_CHECK_FAILURE",
                    _ => "?",
                };
                StepResult::Fault(format!("__fastfail (INT 0x29) code={code} ({kind})"))
            } else {
                StepResult::Fault(format!("INT 0x{v:02X}"))
            }
        }
        Hlt   => StepResult::Exit(0),

        // Unknown — skip rather than crash. CRT init contains many obscure instructions
        // that don't affect program output. A wrong skip is visible; a crash isn't useful.
        _other => StepResult::Continue,
    }
}

// operand helpers

pub fn read_reg(r: Register, cpu: &X86Cpu) -> u32 {
    match r {
        Register::EAX => cpu.eax,
        Register::ECX => cpu.ecx,
        Register::EDX => cpu.edx,
        Register::EBX => cpu.ebx,
        Register::ESP => cpu.esp,
        Register::EBP => cpu.ebp,
        Register::ESI => cpu.esi,
        Register::EDI => cpu.edi,
        Register::AX => cpu.eax & 0xFFFF,
        Register::CX => cpu.ecx & 0xFFFF,
        Register::DX => cpu.edx & 0xFFFF,
        Register::BX => cpu.ebx & 0xFFFF,
        Register::SP => cpu.esp & 0xFFFF,
        Register::BP => cpu.ebp & 0xFFFF,
        Register::SI => cpu.esi & 0xFFFF,
        Register::DI => cpu.edi & 0xFFFF,
        Register::AL => cpu.eax & 0xFF,
        Register::CL => cpu.ecx & 0xFF,
        Register::DL => cpu.edx & 0xFF,
        Register::BL => cpu.ebx & 0xFF,
        Register::AH => (cpu.eax >> 8) & 0xFF,
        Register::CH => (cpu.ecx >> 8) & 0xFF,
        Register::DH => (cpu.edx >> 8) & 0xFF,
        Register::BH => (cpu.ebx >> 8) & 0xFF,
        Register::EIP => cpu.eip,
        Register::None => 0,
        _ => 0,
    }
}

pub fn write_reg(r: Register, v: u32, cpu: &mut X86Cpu) {
    match r {
        Register::EAX => cpu.eax = v,
        Register::ECX => cpu.ecx = v,
        Register::EDX => cpu.edx = v,
        Register::EBX => cpu.ebx = v,
        Register::ESP => cpu.esp = v,
        Register::EBP => cpu.ebp = v,
        Register::ESI => cpu.esi = v,
        Register::EDI => cpu.edi = v,
        Register::AX => cpu.eax = (cpu.eax & 0xFFFF_0000) | (v & 0xFFFF),
        Register::CX => cpu.ecx = (cpu.ecx & 0xFFFF_0000) | (v & 0xFFFF),
        Register::DX => cpu.edx = (cpu.edx & 0xFFFF_0000) | (v & 0xFFFF),
        Register::BX => cpu.ebx = (cpu.ebx & 0xFFFF_0000) | (v & 0xFFFF),
        Register::SP => cpu.esp = (cpu.esp & 0xFFFF_0000) | (v & 0xFFFF),
        Register::BP => cpu.ebp = (cpu.ebp & 0xFFFF_0000) | (v & 0xFFFF),
        Register::SI => cpu.esi = (cpu.esi & 0xFFFF_0000) | (v & 0xFFFF),
        Register::DI => cpu.edi = (cpu.edi & 0xFFFF_0000) | (v & 0xFFFF),
        Register::AL => cpu.eax = (cpu.eax & 0xFFFFFF00) | (v & 0xFF),
        Register::CL => cpu.ecx = (cpu.ecx & 0xFFFFFF00) | (v & 0xFF),
        Register::DL => cpu.edx = (cpu.edx & 0xFFFFFF00) | (v & 0xFF),
        Register::BL => cpu.ebx = (cpu.ebx & 0xFFFFFF00) | (v & 0xFF),
        Register::AH => cpu.eax = (cpu.eax & 0xFFFF_00FF) | ((v & 0xFF) << 8),
        Register::CH => cpu.ecx = (cpu.ecx & 0xFFFF_00FF) | ((v & 0xFF) << 8),
        Register::DH => cpu.edx = (cpu.edx & 0xFFFF_00FF) | ((v & 0xFF) << 8),
        Register::BH => cpu.ebx = (cpu.ebx & 0xFFFF_00FF) | ((v & 0xFF) << 8),
        _ => {}
    }
}

fn reg_size(r: Register) -> usize {
    match r {
        Register::AL
        | Register::CL
        | Register::DL
        | Register::BL
        | Register::AH
        | Register::CH
        | Register::DH
        | Register::BH => 1,
        Register::AX
        | Register::CX
        | Register::DX
        | Register::BX
        | Register::SP
        | Register::BP
        | Register::SI
        | Register::DI => 2,
        _ => 4,
    }
}

fn mem_size(instr: &Instruction) -> usize {
    use iced_x86::MemorySize::*;
    match instr.memory_size() {
        UInt8 | Int8 => 1,
        UInt16 | Int16 => 2,
        _ => 4,
    }
}

fn calc_addr(instr: &Instruction, cpu: &X86Cpu) -> u32 {
    let base  = read_reg(instr.memory_base(),  cpu);
    let index = read_reg(instr.memory_index(), cpu);
    let scale = instr.memory_index_scale();
    let disp  = instr.memory_displacement32();
    let flat  = base.wrapping_add(index.wrapping_mul(scale)).wrapping_add(disp);

    // Apply segment base for FS-relative accesses (TEB).
    // GS is not used in 32-bit mode; other segments use flat base 0.
    if instr.memory_segment() == Register::FS {
        crate::pe::loader::TEB_VA.wrapping_add(flat)
    } else {
        flat
    }
}

// Read operand value (op_idx = 0 for dst, 1 for src usually)
fn read_op(
    instr: &Instruction,
    i: u32,
    cpu: &X86Cpu,
    mem: &GuestMemory,
) -> std::result::Result<u32, String> {
    match instr.op_kind(i) {
        OpKind::Register => Ok(read_reg(instr.op_register(i), cpu)),
        OpKind::Immediate8 => Ok(instr.immediate8() as u32),
        OpKind::Immediate8to32 => Ok(instr.immediate8to32() as u32),
        OpKind::Immediate8to16 => Ok(instr.immediate8to16() as u32),
        OpKind::Immediate16 => Ok(instr.immediate16() as u32),
        OpKind::Immediate32 => Ok(instr.immediate32()),
        OpKind::Immediate32to64 => Ok(instr.immediate32to64() as u32),
        OpKind::NearBranch16 => Ok(instr.near_branch16() as u32),
        OpKind::NearBranch32 => Ok(instr.near_branch32()),
        OpKind::Memory => {
            let addr = calc_addr(instr, cpu);
            (match mem_size(instr) {
                1 => mem.read_u8(addr).map(|v| v as u32),
                2 => mem.read_u16(addr).map(|v| v as u32),
                _ => mem.read_u32(addr),
            })
            .map_err(|e| e.to_string())
        }
        k => Err(format!("unhandled op kind {:?}", k)),
    }
}

// Write operand (only register or memory destinations)
fn write_op(
    instr: &Instruction,
    i: u32,
    val: u32,
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
) -> std::result::Result<(), String> {
    match instr.op_kind(i) {
        OpKind::Register => {
            write_reg(instr.op_register(i), val, cpu);
            Ok(())
        }
        OpKind::Memory => {
            let addr = calc_addr(instr, cpu);
            (match mem_size(instr) {
                1 => mem.write_u8(addr, val as u8),
                2 => mem.write_u16(addr, val as u16),
                _ => mem.write_u32(addr, val),
            })
            .map_err(|e| e.to_string())
        }
        k => Err(format!("unhandled dst op kind {:?}", k)),
    }
}

fn op_size(instr: &Instruction, i: u32) -> usize {
    match instr.op_kind(i) {
        OpKind::Register => reg_size(instr.op_register(i)),
        OpKind::Memory => mem_size(instr),
        _ => 4,
    }
}

fn fault(e: impl std::fmt::Display) -> StepResult {
    StepResult::Fault(e.to_string())
}

// instruction implementations

fn exec_mov(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    // Skip segment register moves
    let dst_reg = instr.op0_register();
    if matches!(
        dst_reg,
        Register::CS | Register::DS | Register::ES | Register::FS | Register::GS | Register::SS
    ) {
        return StepResult::Continue;
    }
    match read_op(instr, 1, cpu, mem) {
        Err(e) => fault(e),
        Ok(v) => match write_op(instr, 0, v, cpu, mem) {
            Err(e) => fault(e),
            Ok(()) => StepResult::Continue,
        },
    }
}

fn exec_movzx(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    match read_op(instr, 1, cpu, mem) {
        Err(e) => fault(e),
        Ok(v) => {
            let masked = match op_size(instr, 1) {
                1 => v & 0xFF,
                2 => v & 0xFFFF,
                _ => v,
            };
            match write_op(instr, 0, masked, cpu, mem) {
                Err(e) => fault(e),
                Ok(()) => StepResult::Continue,
            }
        }
    }
}

fn exec_movsx(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    match read_op(instr, 1, cpu, mem) {
        Err(e) => fault(e),
        Ok(v) => {
            let extended = match op_size(instr, 1) {
                1 => (v as i8 as i32) as u32,
                2 => (v as i16 as i32) as u32,
                _ => v,
            };
            match write_op(instr, 0, extended, cpu, mem) {
                Err(e) => fault(e),
                Ok(()) => StepResult::Continue,
            }
        }
    }
}

fn exec_xchg(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let a = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let b = match read_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    if let Err(e) = write_op(instr, 0, b, cpu, mem) {
        return fault(e);
    }
    if let Err(e) = write_op(instr, 1, a, cpu, mem) {
        return fault(e);
    }
    StepResult::Continue
}

// CMPXCHG dst, src: compare accumulator (EAX/AX/AL) with dst.
// If equal: ZF=1, dst = src. Else: ZF=0, accumulator = dst.
fn exec_cmpxchg(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let size = op_size(instr, 0);
    let dst = match read_op(instr, 0, cpu, mem) { Err(e) => return fault(e), Ok(v) => v };
    let src = match read_op(instr, 1, cpu, mem) { Err(e) => return fault(e), Ok(v) => v };
    let acc = match size {
        1 => cpu.eax & 0xFF,
        2 => cpu.eax & 0xFFFF,
        _ => cpu.eax,
    };

    // Flags reflect (acc - dst), like CMP.
    let r = acc.wrapping_sub(dst);
    if size == 1 { set_sub8(&mut cpu.eflags, acc as u8, dst as u8, r as u8); }
    else         { set_sub32(&mut cpu.eflags, acc, dst, r); }

    if acc == dst {
        if let Err(e) = write_op(instr, 0, src, cpu, mem) { return fault(e); }
    } else {
        match size {
            1 => cpu.eax = (cpu.eax & 0xFFFF_FF00) | (dst & 0xFF),
            2 => cpu.eax = (cpu.eax & 0xFFFF_0000) | (dst & 0xFFFF),
            _ => cpu.eax = dst,
        }
    }
    StepResult::Continue
}

// XADD dst, src: temp = dst + src; src = dst; dst = temp.
fn exec_xadd(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let dst = match read_op(instr, 0, cpu, mem) { Err(e) => return fault(e), Ok(v) => v };
    let src = match read_op(instr, 1, cpu, mem) { Err(e) => return fault(e), Ok(v) => v };
    let sum = dst.wrapping_add(src);
    set_add32(&mut cpu.eflags, dst, src, sum);
    if let Err(e) = write_op(instr, 1, dst, cpu, mem) { return fault(e); }
    if let Err(e) = write_op(instr, 0, sum, cpu, mem) { return fault(e); }
    StepResult::Continue
}

fn exec_lea(instr: &Instruction, cpu: &mut X86Cpu, _mem: &mut GuestMemory) -> StepResult {
    let addr = calc_addr(instr, cpu);
    write_reg(instr.op0_register(), addr, cpu);
    StepResult::Continue
}

fn exec_push(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let v = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    cpu.esp = cpu.esp.wrapping_sub(4);
    match mem.write_u32(cpu.esp, v) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_pop(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let v = match mem.read_u32(cpu.esp) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    cpu.esp = cpu.esp.wrapping_add(4);
    match write_op(instr, 0, v, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_pushad(cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let saved_esp = cpu.esp;
    for &v in &[
        cpu.eax, cpu.ecx, cpu.edx, cpu.ebx, saved_esp, cpu.ebp, cpu.esi, cpu.edi,
    ] {
        cpu.esp = cpu.esp.wrapping_sub(4);
        if let Err(e) = mem.write_u32(cpu.esp, v) {
            return fault(e);
        }
    }
    StepResult::Continue
}

fn exec_popad(cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let mut vals = [0u32; 8];
    for v in vals.iter_mut() {
        *v = match mem.read_u32(cpu.esp) {
            Err(e) => return fault(e),
            Ok(v) => v,
        };
        cpu.esp = cpu.esp.wrapping_add(4);
    }
    cpu.edi = vals[0];
    cpu.esi = vals[1];
    cpu.ebp = vals[2];
    // vals[3] = esp (ignored)
    cpu.ebx = vals[4];
    cpu.edx = vals[5];
    cpu.ecx = vals[6];
    cpu.eax = vals[7];
    StepResult::Continue
}

fn exec_pushfd(cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let f = cpu.eflags;
    cpu.esp = cpu.esp.wrapping_sub(4);
    match mem.write_u32(cpu.esp, f) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_popfd(cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    match mem.read_u32(cpu.esp) {
        Err(e) => fault(e),
        Ok(v) => {
            cpu.eflags = v;
            cpu.esp = cpu.esp.wrapping_add(4);
            StepResult::Continue
        }
    }
}

enum AluOp {
    Add,
    Sub,
    Adc,
    Sbb,
    And,
    Or,
    Xor,
    Cmp,
    Test,
}

fn exec_alu(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory, op: AluOp) -> StepResult {
    let dst = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let src = match read_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let w = op_size(instr, 0) as u32; // operand width in bytes (1/2/4)

    let (result, write_back) = match op {
        AluOp::Add => (flags_add(&mut cpu.eflags, dst, src, 0, w), true),
        AluOp::Adc => {
            let c = get_cf(cpu.eflags) as u32;
            (flags_add(&mut cpu.eflags, dst, src, c, w), true)
        }
        AluOp::Sub | AluOp::Cmp => {
            (flags_sub(&mut cpu.eflags, dst, src, 0, w), matches!(op, AluOp::Sub))
        }
        AluOp::Sbb => {
            let c = get_cf(cpu.eflags) as u32;
            (flags_sub(&mut cpu.eflags, dst, src, c, w), true)
        }
        AluOp::And | AluOp::Test => {
            let m = if w >= 4 { 0xFFFF_FFFF } else { (1u32 << (w * 8)) - 1 };
            let r = (dst & src) & m;
            flags_logic(&mut cpu.eflags, r, w);
            (r, matches!(op, AluOp::And))
        }
        AluOp::Or => {
            let m = if w >= 4 { 0xFFFF_FFFF } else { (1u32 << (w * 8)) - 1 };
            let r = (dst | src) & m;
            flags_logic(&mut cpu.eflags, r, w);
            (r, true)
        }
        AluOp::Xor => {
            let m = if w >= 4 { 0xFFFF_FFFF } else { (1u32 << (w * 8)) - 1 };
            let r = (dst ^ src) & m;
            flags_logic(&mut cpu.eflags, r, w);
            (r, true)
        }
    };

    if write_back {
        match write_op(instr, 0, result, cpu, mem) {
            Err(e) => return fault(e),
            Ok(()) => {}
        }
    }
    StepResult::Continue
}

fn exec_not(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let v = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    match write_op(instr, 0, !v, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_neg(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let v = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let w = op_size(instr, 0) as u32;
    let r = flags_sub(&mut cpu.eflags, 0, v, 0, w); // NEG = 0 - v
    match write_op(instr, 0, r, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_inc(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let v = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let w = op_size(instr, 0) as u32;
    let old_cf = cpu.eflags & CF;
    let r = flags_add(&mut cpu.eflags, v, 1, 0, w);
    cpu.eflags = (cpu.eflags & !CF) | old_cf; // INC doesn't affect CF
    match write_op(instr, 0, r, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_dec(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let v = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let w = op_size(instr, 0) as u32;
    let old_cf = cpu.eflags & CF;
    let r = flags_sub(&mut cpu.eflags, v, 1, 0, w);
    cpu.eflags = (cpu.eflags & !CF) | old_cf; // DEC doesn't affect CF
    match write_op(instr, 0, r, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_imul(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    match instr.op_count() {
        1 => {
            let src = match read_op(instr, 0, cpu, mem) {
                Err(e) => return fault(e),
                Ok(v) => v,
            };
            let r = (cpu.eax as i32 as i64) * (src as i32 as i64);
            cpu.eax = r as u32;
            cpu.edx = (r >> 32) as u32;
            let overflow = cpu.edx != if cpu.eax >> 31 != 0 { 0xFFFF_FFFF } else { 0 };
            if overflow {
                cpu.eflags |= CF | OF;
            } else {
                cpu.eflags &= !(CF | OF);
            }
        }
        2 => {
            let dst = match read_op(instr, 0, cpu, mem) {
                Err(e) => return fault(e),
                Ok(v) => v,
            };
            let src = match read_op(instr, 1, cpu, mem) {
                Err(e) => return fault(e),
                Ok(v) => v,
            };
            let r = ((dst as i32 as i64) * (src as i32 as i64)) as u32;
            if let Err(e) = write_op(instr, 0, r, cpu, mem) {
                return fault(e);
            }
        }
        _ => {
            let src1 = match read_op(instr, 1, cpu, mem) {
                Err(e) => return fault(e),
                Ok(v) => v,
            };
            let src2 = match read_op(instr, 2, cpu, mem) {
                Err(e) => return fault(e),
                Ok(v) => v,
            };
            let r = ((src1 as i32 as i64) * (src2 as i32 as i64)) as u32;
            if let Err(e) = write_op(instr, 0, r, cpu, mem) {
                return fault(e);
            }
        }
    }
    StepResult::Continue
}

fn exec_mul(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let src = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let r = (cpu.eax as u64) * (src as u64);
    cpu.eax = r as u32;
    cpu.edx = (r >> 32) as u32;
    StepResult::Continue
}

fn exec_idiv(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let divisor = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    } as i32;
    if divisor == 0 {
        return StepResult::Fault("division by zero".into());
    }
    let dividend = ((cpu.edx as i64) << 32) | (cpu.eax as i64);
    cpu.eax = (dividend / divisor as i64) as u32;
    cpu.edx = (dividend % divisor as i64) as u32;
    StepResult::Continue
}

fn exec_div(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let divisor = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    } as u64;
    if divisor == 0 {
        return StepResult::Fault("division by zero".into());
    }
    let dividend = ((cpu.edx as u64) << 32) | (cpu.eax as u64);
    cpu.eax = (dividend / divisor) as u32;
    cpu.edx = (dividend % divisor) as u32;
    StepResult::Continue
}

enum ShiftOp {
    Shl,
    Shr,
    Sar,
    Rol,
    Ror,
}

fn exec_shift(
    instr: &Instruction,
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
    op: ShiftOp,
) -> StepResult {
    let dst = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let cnt = (match read_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    } & 0x1F) as u32;
    if cnt == 0 {
        return StepResult::Continue;
    }
    let result = match op {
        ShiftOp::Shl => {
            cpu.eflags = if cnt == 1 {
                let overflow = (dst >> 31) ^ (dst >> 30) & 1;
                (cpu.eflags & !(CF | OF)) | ((dst >> (32 - cnt)) & 1) | (overflow << 11)
            } else {
                (cpu.eflags & !CF) | ((dst >> (32 - cnt)) & 1)
            };
            dst << cnt
        }
        ShiftOp::Shr => {
            cpu.eflags = (cpu.eflags & !CF) | ((dst >> (cnt - 1)) & 1);
            dst >> cnt
        }
        ShiftOp::Sar => {
            cpu.eflags = (cpu.eflags & !CF) | ((dst >> (cnt - 1)) & 1);
            ((dst as i32) >> cnt) as u32
        }
        ShiftOp::Rol => dst.rotate_left(cnt),
        ShiftOp::Ror => dst.rotate_right(cnt),
    };
    set_szp(&mut cpu.eflags, result);
    match write_op(instr, 0, result, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_call(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let target = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let ret = cpu.eip;
    cpu.esp = cpu.esp.wrapping_sub(4);
    if let Err(e) = mem.write_u32(cpu.esp, ret) {
        return fault(e);
    }
    cpu.eip = target;
    if target >= TRAMPOLINE_BASE {
        StepResult::ApiTrap(target)
    } else {
        StepResult::Continue
    }
}

fn exec_ret(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let ret = match mem.read_u32(cpu.esp) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    cpu.esp = cpu.esp.wrapping_add(4);
    // optional immediate (stdcall cleanup)
    if instr.op_count() > 0 {
        let imm = match read_op(instr, 0, cpu, mem) {
            Err(e) => return fault(e),
            Ok(v) => v,
        };
        cpu.esp = cpu.esp.wrapping_add(imm);
    }
    cpu.eip = ret;
    if ret >= TRAMPOLINE_BASE {
        StepResult::ApiTrap(ret)
    } else {
        StepResult::Continue
    }
}

fn exec_jmp(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let target = match read_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    cpu.eip = target;
    if target >= TRAMPOLINE_BASE {
        StepResult::ApiTrap(target)
    } else {
        StepResult::Continue
    }
}

fn exec_jcc(instr: &Instruction, cpu: &mut X86Cpu, cond: bool) -> StepResult {
    if cond {
        let target = match instr.op_kind(0) {
            OpKind::NearBranch16 => instr.near_branch16() as u32,
            OpKind::NearBranch32 => instr.near_branch32(),
            _ => return StepResult::Fault("jcc bad operand".into()),
        };
        cpu.eip = target;
    }
    StepResult::Continue
}

fn exec_setcc(
    instr: &Instruction,
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
    cond: bool,
) -> StepResult {
    match write_op(instr, 0, cond as u32, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

// Conditional move: dst (reg) = src if cond. Flags are not modified. When the
// condition is false the destination keeps its value. We only read the source
// when the move is taken so an untaken CMOV with a memory operand can't fault.
fn exec_cmovcc(
    instr: &Instruction,
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
    cond: bool,
) -> StepResult {
    if !cond {
        return StepResult::Continue;
    }
    let src = match read_op(instr, 1, cpu, mem) {
        Ok(v) => v,
        Err(e) => return fault(e),
    };
    match write_op(instr, 0, src, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_stos(
    instr: &Instruction,
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
    sz: usize,
) -> StepResult {
    let count = if instr.has_rep_prefix() { cpu.ecx } else { 1 };
    let step = if get_df(cpu.eflags) {
        (0u32).wrapping_sub(sz as u32)
    } else {
        sz as u32
    };
    for _ in 0..count {
        let r = match sz {
            1 => mem.write_u8(cpu.edi, cpu.eax as u8),
            2 => mem.write_u16(cpu.edi, cpu.eax as u16),
            _ => mem.write_u32(cpu.edi, cpu.eax),
        };
        if let Err(e) = r {
            return fault(e);
        }
        cpu.edi = cpu.edi.wrapping_add(step);
    }
    if instr.has_rep_prefix() {
        cpu.ecx = 0;
    }
    StepResult::Continue
}

fn exec_movs(
    instr: &Instruction,
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
    sz: usize,
) -> StepResult {
    let count = if instr.has_rep_prefix() { cpu.ecx } else { 1 };
    let step = if get_df(cpu.eflags) {
        (0u32).wrapping_sub(sz as u32)
    } else {
        sz as u32
    };
    for _ in 0..count {
        let v = match sz {
            1 => mem.read_u8(cpu.esi).map(|v| v as u32),
            2 => mem.read_u16(cpu.esi).map(|v| v as u32),
            _ => mem.read_u32(cpu.esi),
        };
        let v = match v {
            Err(e) => return fault(e),
            Ok(v) => v,
        };
        let r = match sz {
            1 => mem.write_u8(cpu.edi, v as u8),
            2 => mem.write_u16(cpu.edi, v as u16),
            _ => mem.write_u32(cpu.edi, v),
        };
        if let Err(e) = r {
            return fault(e);
        }
        cpu.esi = cpu.esi.wrapping_add(step);
        cpu.edi = cpu.edi.wrapping_add(step);
    }
    if instr.has_rep_prefix() {
        cpu.ecx = 0;
    }
    StepResult::Continue
}

fn exec_scas(
    instr: &Instruction,
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
    sz: usize,
) -> StepResult {
    let repe = instr.has_rep_prefix();
    let repne = instr.has_repne_prefix();
    let step = if get_df(cpu.eflags) {
        (0u32).wrapping_sub(sz as u32)
    } else {
        sz as u32
    };
    loop {
        if (repe || repne) && cpu.ecx == 0 {
            break;
        }
        let v = match sz {
            1 => mem.read_u8(cpu.edi).map(|v| v as u32),
            _ => mem.read_u32(cpu.edi),
        };
        let v = match v {
            Err(e) => return fault(e),
            Ok(v) => v,
        };
        let r = cpu.eax.wrapping_sub(v);
        set_sub32(&mut cpu.eflags, cpu.eax, v, r);
        cpu.edi = cpu.edi.wrapping_add(step);
        if repe {
            cpu.ecx -= 1;
            if !get_zf(cpu.eflags) {
                break;
            }
        } else if repne {
            cpu.ecx -= 1;
            if get_zf(cpu.eflags) {
                break;
            }
        } else {
            break;
        }
    }
    StepResult::Continue
}

// XMM / SSE helpers

// MOVLPD/MOVLPS: move low 64 bits (preserving upper 64 when loading into XMM)
fn exec_movlp(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let dst_reg = instr.op0_register();
    if let Some(i) = xmm_idx(dst_reg) {
        let addr = calc_addr(instr, cpu);
        match mem.read_bytes(addr, 8) {
            Err(e) => return fault(e),
            Ok(b)  => cpu.xmm[i][..8].copy_from_slice(&b),
        }
    } else {
        let src = xmm_idx(instr.op1_register()).map(|i| cpu.xmm[i]).unwrap_or([0u8; 16]);
        let addr = calc_addr(instr, cpu);
        if let Err(e) = mem.write_bytes(addr, &src[..8]) { return fault(e); }
    }
    StepResult::Continue
}

// MOVHPD/MOVHPS: move high 64 bits
fn exec_movhp(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let dst_reg = instr.op0_register();
    if let Some(i) = xmm_idx(dst_reg) {
        let addr = calc_addr(instr, cpu);
        match mem.read_bytes(addr, 8) {
            Err(e) => return fault(e),
            Ok(b)  => cpu.xmm[i][8..].copy_from_slice(&b),
        }
    } else {
        let src = xmm_idx(instr.op1_register()).map(|i| cpu.xmm[i]).unwrap_or([0u8; 16]);
        let addr = calc_addr(instr, cpu);
        if let Err(e) = mem.write_bytes(addr, &src[8..]) { return fault(e); }
    }
    StepResult::Continue
}

fn xmm_idx(r: Register) -> Option<usize> {
    match r {
        Register::XMM0 => Some(0),
        Register::XMM1 => Some(1),
        Register::XMM2 => Some(2),
        Register::XMM3 => Some(3),
        Register::XMM4 => Some(4),
        Register::XMM5 => Some(5),
        Register::XMM6 => Some(6),
        Register::XMM7 => Some(7),
        _ => None,
    }
}

fn read_xmm_op(
    instr: &Instruction,
    op: u32,
    cpu: &X86Cpu,
    mem: &GuestMemory,
) -> std::result::Result<[u8; 16], String> {
    match instr.op_kind(op) {
        OpKind::Register => Ok(xmm_idx(instr.op_register(op))
            .map(|i| cpu.xmm[i])
            .unwrap_or([0u8; 16])),
        OpKind::Memory => {
            let addr = calc_addr(instr, cpu);
            let bytes = mem.read_bytes(addr, 16).map_err(|e| e.to_string())?;
            Ok(bytes.try_into().unwrap_or([0u8; 16]))
        }
        _ => Ok([0u8; 16]),
    }
}

fn write_xmm_op(
    instr: &Instruction,
    op: u32,
    val: [u8; 16],
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
) -> std::result::Result<(), String> {
    match instr.op_kind(op) {
        OpKind::Register => {
            if let Some(i) = xmm_idx(instr.op_register(op)) {
                cpu.xmm[i] = val;
            }
            Ok(())
        }
        OpKind::Memory => {
            let addr = calc_addr(instr, cpu);
            mem.write_bytes(addr, &val).map_err(|e| e.to_string())
        }
        _ => Ok(()),
    }
}

fn xmm_xor(a: [u8; 16], b: [u8; 16]) -> [u8; 16] {
    std::array::from_fn(|i| a[i] ^ b[i])
}
fn xmm_and(a: [u8; 16], b: [u8; 16]) -> [u8; 16] {
    std::array::from_fn(|i| a[i] & b[i])
}
fn xmm_or(a: [u8; 16], b: [u8; 16]) -> [u8; 16] {
    std::array::from_fn(|i| a[i] | b[i])
}

fn exec_xmm_binop(
    instr: &Instruction,
    cpu: &mut X86Cpu,
    mem: &mut GuestMemory,
    op: impl Fn([u8; 16], [u8; 16]) -> [u8; 16],
) -> StepResult {
    let dst = match read_xmm_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let src = match read_xmm_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let r = op(dst, src);
    match write_xmm_op(instr, 0, r, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_xmm_mov(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let src = match read_xmm_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    match write_xmm_op(instr, 0, src, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_movd(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    // MOVD can be xmm←r32/m32 or r32/m32←xmm
    let dst_reg = instr.op0_register();
    let src_reg = instr.op1_register();

    if xmm_idx(dst_reg).is_some() {
        // xmm ← gp/mem (32-bit, zero-extend)
        let v = match read_op(instr, 1, cpu, mem) {
            Err(e) => return fault(e),
            Ok(v) => v,
        };
        let mut arr = [0u8; 16];
        arr[..4].copy_from_slice(&v.to_le_bytes());
        if let Some(i) = xmm_idx(dst_reg) {
            cpu.xmm[i] = arr;
        }
    } else if xmm_idx(src_reg).is_some() {
        // gp/mem ← xmm low 32 bits
        let src = xmm_idx(src_reg).map(|i| cpu.xmm[i]).unwrap_or([0u8; 16]);
        let v = u32::from_le_bytes(src[..4].try_into().unwrap());
        match write_op(instr, 0, v, cpu, mem) {
            Err(e) => return fault(e),
            Ok(()) => {}
        }
    }
    StepResult::Continue
}

fn exec_pcmpeq(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let dst = match read_xmm_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let src = match read_xmm_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    // PCMPEQB: byte-wise equal, result byte = 0xFF or 0x00
    let r: [u8; 16] = std::array::from_fn(|i| if dst[i] == src[i] { 0xFF } else { 0x00 });
    match write_xmm_op(instr, 0, r, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_pmovmskb(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let src = match read_xmm_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let mask: u32 = src
        .iter()
        .enumerate()
        .fold(0u32, |acc, (i, &b)| acc | (((b >> 7) as u32) << i));
    write_reg(instr.op0_register(), mask, cpu);
    StepResult::Continue
}

fn exec_pshift_dq(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let src = match read_xmm_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let count = (match read_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    }) as usize;
    let bytes = count.min(16);
    let r: [u8; 16] = if instr.mnemonic() == Mnemonic::Psrldq {
        // shift right (towards higher addresses)
        std::array::from_fn(|i| if i + bytes < 16 { src[i + bytes] } else { 0 })
    } else {
        // shift left (towards lower addresses)
        std::array::from_fn(|i| if i >= bytes { src[i - bytes] } else { 0 })
    };
    match write_xmm_op(instr, 0, r, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_punpckl(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let dst = match read_xmm_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let src = match read_xmm_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    // PUNPCKLBW: interleave low 8 bytes of dst and src
    let r: [u8; 16] = std::array::from_fn(|i| if i % 2 == 0 { dst[i / 2] } else { src[i / 2] });
    match write_xmm_op(instr, 0, r, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_punpckh(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    let dst = match read_xmm_op(instr, 0, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let src = match read_xmm_op(instr, 1, cpu, mem) {
        Err(e) => return fault(e),
        Ok(v) => v,
    };
    let r: [u8; 16] = std::array::from_fn(|i| {
        if i % 2 == 0 {
            dst[8 + i / 2]
        } else {
            src[8 + i / 2]
        }
    });
    match write_xmm_op(instr, 0, r, cpu, mem) {
        Err(e) => fault(e),
        Ok(()) => StepResult::Continue,
    }
}

fn exec_sse_ctrl(instr: &Instruction, cpu: &mut X86Cpu, mem: &mut GuestMemory) -> StepResult {
    // LDMXCSR, STMXCSR, FLDCW, FSTCW — control word loads/stores, safely ignore
    if instr.mnemonic() == Mnemonic::Stmxcsr
        || instr.mnemonic() == Mnemonic::Fnstcw
        || instr.mnemonic() == Mnemonic::Fstcw
    {
        // write a benign control word to memory
        if instr.op_count() > 0 && instr.op_kind(0) == OpKind::Memory {
            let addr = calc_addr(instr, cpu);
            let _ = mem.write_u32(addr, 0x1F80); // default MXCSR
        }
    }
    StepResult::Continue
}

// run result

use serde::{Deserialize, Serialize};
use crate::vm::process::UiEvent;

#[derive(Debug, Serialize, Deserialize)]
pub struct SliceResult {
    pub pid: u32,
    pub stdout: String,
    pub stderr: String,
    pub state: ProcessState,
    pub instructions: u32,
    pub ui_events: Vec<UiEvent>,
    /// Children spawned this slice (pid, path) — filled in by the VM, which
    /// owns the process table. The executor leaves it empty.
    pub spawned: Vec<(u32, String)>,
}

impl SliceResult {
    fn done(proc: &mut GuestProcess) -> Self {
        SliceResult {
            pid: proc.pid,
            stdout: String::from_utf8_lossy(&proc.console.drain_stdout()).into_owned(),
            stderr: String::from_utf8_lossy(&proc.console.drain_stderr()).into_owned(),
            state: proc.state.clone(),
            instructions: 0,
            ui_events: std::mem::take(&mut proc.ui_events),
            spawned: Vec::new(),
        }
    }
}
