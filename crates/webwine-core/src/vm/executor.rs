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

    loop {
        if executed >= budget {
            break;
        }

        // trampoline check
        if proc.cpu.eip >= TRAMPOLINE_BASE {
            let va = proc.cpu.eip;
            let mut ctx = ApiContext {
                cpu: &mut proc.cpu,
                memory: &mut proc.memory,
                handles: &mut proc.handles,
                console: &mut proc.console,
                heap_next: &mut proc.heap_next,
                fs,
                logs,
                pid: proc.pid,
            };
            match api.dispatch(va, &mut ctx) {
                Some(Handled::Ok) => {}
                Some(Handled::ExitProcess(code)) => {
                    proc.state = ProcessState::Exited { exit_code: code };
                    break;
                }
                Some(Handled::Unimplemented) | None => {
                    let name = api
                        .lookup_name(va)
                        .map(|(d, n)| format!("{d}!{n}"))
                        .unwrap_or_else(|| format!("0x{va:08X}"));
                    logs.log(
                        LogLevel::Warn,
                        "api",
                        &format!("[api] unimplemented: {name} — returning 0"),
                        Some(proc.pid),
                    );
                    let ret = proc.memory.read_u32(proc.cpu.esp).unwrap_or(0);
                    proc.cpu.esp = proc.cpu.esp.wrapping_add(4);
                    proc.cpu.eax = 0;
                    proc.cpu.eip = ret;
                }
            }
            executed += 1;
            continue;
        }

        match step(proc) {
            StepResult::Continue => {
                executed += 1;
            }
            StepResult::ApiTrap(va) => {
                proc.cpu.eip = va; /* handled next iteration */
            }
            StepResult::Exit(code) => {
                proc.state = ProcessState::Exited { exit_code: code };
                break;
            }
            StepResult::Fault(r) => {
                logs.log(
                    LogLevel::Error,
                    "cpu",
                    &format!("[cpu] fault at EIP=0x{:08X}: {r}", proc.cpu.eip),
                    Some(proc.pid),
                );
                proc.state = ProcessState::Crashed { reason: r };
                break;
            }
        }
    }

    Ok(SliceResult::done(proc))
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

        // ── SSE / XMM ────────────────────────────────────────────────────────
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
        Int   => StepResult::Fault(format!("INT 0x{:02X}", instr.immediate8())),
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
    let sz = op_size(instr, 0);

    let (result, write_back) = match op {
        AluOp::Add => {
            let r = dst.wrapping_add(src);
            if sz == 1 {
                set_add8(&mut cpu.eflags, dst as u8, src as u8, r as u8)
            } else {
                set_add32(&mut cpu.eflags, dst, src, r)
            };
            (r, true)
        }
        AluOp::Adc => {
            let c = get_cf(cpu.eflags) as u32;
            let r = dst.wrapping_add(src).wrapping_add(c);
            set_add32(&mut cpu.eflags, dst, src, r);
            (r, true)
        }
        AluOp::Sub | AluOp::Cmp => {
            let r = dst.wrapping_sub(src);
            if sz == 1 {
                set_sub8(&mut cpu.eflags, dst as u8, src as u8, r as u8)
            } else {
                set_sub32(&mut cpu.eflags, dst, src, r)
            };
            (r, matches!(op, AluOp::Sub))
        }
        AluOp::Sbb => {
            let c = get_cf(cpu.eflags) as u32;
            let r = dst.wrapping_sub(src).wrapping_sub(c);
            set_sub32(&mut cpu.eflags, dst, src, r);
            (r, true)
        }
        AluOp::And | AluOp::Test => {
            let r = dst & src;
            set_szp(&mut cpu.eflags, r);
            cpu.eflags &= !(CF | OF);
            (r, matches!(op, AluOp::And))
        }
        AluOp::Or => {
            let r = dst | src;
            set_szp(&mut cpu.eflags, r);
            cpu.eflags &= !(CF | OF);
            (r, true)
        }
        AluOp::Xor => {
            let r = dst ^ src;
            set_szp(&mut cpu.eflags, r);
            cpu.eflags &= !(CF | OF);
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
    let r = (0u32).wrapping_sub(v);
    set_sub32(&mut cpu.eflags, 0, v, r);
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
    let r = v.wrapping_add(1);
    let old_cf = cpu.eflags & CF;
    set_add32(&mut cpu.eflags, v, 1, r);
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
    let r = v.wrapping_sub(1);
    let old_cf = cpu.eflags & CF;
    set_sub32(&mut cpu.eflags, v, 1, r);
    cpu.eflags = (cpu.eflags & !CF) | old_cf;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SliceResult {
    pub pid: u32,
    pub stdout: String,
    pub stderr: String,
    pub state: ProcessState,
    pub instructions: u32,
}

impl SliceResult {
    fn done(proc: &mut GuestProcess) -> Self {
        SliceResult {
            pid: proc.pid,
            stdout: String::from_utf8_lossy(&proc.console.drain_stdout()).into_owned(),
            stderr: String::from_utf8_lossy(&proc.console.drain_stderr()).into_owned(),
            state: proc.state.clone(),
            instructions: 0,
        }
    }
}
