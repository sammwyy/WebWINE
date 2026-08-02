// A small CIL (Common Intermediate Language) interpreter. It executes the IL of
// managed methods on an evaluation stack, dispatching `call`s either to other
// MethodDefs in the same assembly (interpreted recursively) or to BCL methods,
// which are handled as internal calls in `webwine-api-net20`.

use crate::clr::metadata::{decompress_uint, CodedKind, T_MEMBERREF, T_METHODDEF, T_STANDALONESIG, T_TYPEREF};
use crate::clr::ClrImage;
use crate::error::{Result, VmError};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use webwine_api::vm::process::UiEvent;
use webwine_api_net20::{ManagedObject, Net20Runtime, Value};

/// Resolved call target: either interpreted IL or a BCL internal call.
pub enum CallTarget {
    Method { row: u32, argc: usize, returns_value: bool },
    Bcl { key: String, argc: usize, returns_value: bool },
}

pub struct ClrRuntime<'a> {
    img: &'a ClrImage,
    pub stdout: String,
    pub exit_code: i32,
    halted: bool,
    steps: u64,
    step_limit: u64,
    call_depth: usize,
    static_fields: HashMap<u32, Value>,
    pub ui_events: Vec<UiEvent>,
    waiting_for_input: bool,
}

impl<'a> ClrRuntime<'a> {
    pub fn new(img: &'a ClrImage) -> Self {
        ClrRuntime {
            img,
            stdout: String::new(),
            exit_code: 0,
            halted: false,
            steps: 0,
            step_limit: 1_000_000,
            call_depth: 0,
            static_fields: HashMap::new(),
            ui_events: Vec::new(),
            waiting_for_input: false,
        }
    }

    /// Run the assembly's entry-point method to completion.
    pub fn run_entry(&mut self) -> Result<i32> {
        let row = self
            .img
            .entry_method_row()
            .ok_or_else(|| VmError::Unsupported("managed entry point is not a MethodDef".into()))?;
        self.run_method(row, vec![Value::Array(Rc::new(RefCell::new(Vec::new())))])?;
        Ok(self.exit_code)
    }

    /// Execute a MethodDef body with the given arguments. Returns the value left
    /// on the stack at `ret`, if the method is non-void.
    pub fn run_method(&mut self, method_row: u32, mut args: Vec<Value>) -> Result<Option<Value>> {
        if self.call_depth > 200 {
            return Err(VmError::Unsupported("CLR call stack overflow (depth > 200)".into()));
        }
        self.call_depth += 1;

        let m = &self.img.meta;
        let rva = m.col(T_METHODDEF, method_row, 0);
        if rva == 0 {
            self.call_depth -= 1;
            return Err(VmError::Unsupported(format!(
                "MethodDef {method_row} has no IL body (extern/abstract)"
            )));
        }

        let (code, local_count) = self.read_body(rva)?;
        let mut locals: Vec<Value> = vec![Value::I4(0); local_count];
        let mut stack: Vec<Value> = Vec::new();
        let mut ip = 0usize;

        while ip < code.len() {
            if self.halted {
                break;
            }
            self.steps += 1;
            if self.steps > self.step_limit {
                return Err(VmError::Unsupported("CIL step limit exceeded".into()));
            }

            let op = code[ip];
            ip += 1;
            match op {
                0x00 => {}                                   // nop
                0x01 => {}                                   // break
                0x02..=0x05 => stack.push(args.get((op - 0x02) as usize).cloned().unwrap_or(Value::Null)), // ldarg.0..3
                0x06..=0x09 => stack.push(locals[(op - 0x06) as usize].clone()), // ldloc.0..3
                0x0A..=0x0D => {
                    let v = stack.pop().unwrap_or(Value::Null);
                    locals[(op - 0x0A) as usize] = v;
                }
                0x0E => {
                    // ldarg.s <u8>
                    let i = code[ip] as usize; ip += 1;
                    stack.push(args.get(i).cloned().unwrap_or(Value::Null));
                }
                0x10 => {
                    // starg.s <u8>
                    let i = code[ip] as usize; ip += 1;
                    let value = stack.pop().unwrap_or(Value::Null);
                    if i < args.len() { args[i] = value; }
                }
                0x11 => {
                    // ldloc.s <u8>
                    let i = code[ip] as usize; ip += 1;
                    stack.push(locals.get(i).cloned().unwrap_or(Value::Null));
                }
                0x12 => {
                    // ldloca.s â€” push a placeholder; address semantics unsupported.
                    ip += 1;
                    stack.push(Value::Null);
                }
                0x13 => {
                    // stloc.s <u8>
                    let i = code[ip] as usize; ip += 1;
                    let v = stack.pop().unwrap_or(Value::Null);
                    if i < locals.len() { locals[i] = v; }
                }
                0x14 => stack.push(Value::Null),             // ldnull
                0x15 => stack.push(Value::I4(-1)),           // ldc.i4.m1
                0x16..=0x1E => stack.push(Value::I4((op as i32) - 0x16)), // ldc.i4.0..8
                0x1F => {
                    // ldc.i4.s <i8>
                    let v = code[ip] as i8 as i32; ip += 1;
                    stack.push(Value::I4(v));
                }
                0x20 => {
                    // ldc.i4 <i32>
                    let v = i32::from_le_bytes([code[ip], code[ip + 1], code[ip + 2], code[ip + 3]]);
                    ip += 4;
                    stack.push(Value::I4(v));
                }
                0x21 => {
                    // ldc.i8 <i64>
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&code[ip..ip + 8]);
                    ip += 8;
                    stack.push(Value::I8(i64::from_le_bytes(b)));
                }
                0x22 => {
                    // ldc.r4 <f32>
                    let v = f32::from_le_bytes([code[ip], code[ip + 1], code[ip + 2], code[ip + 3]]);
                    ip += 4;
                    stack.push(Value::R8(v as f64));
                }
                0x23 => {
                    // ldc.r8 <f64>
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&code[ip..ip + 8]);
                    ip += 8;
                    stack.push(Value::R8(f64::from_le_bytes(b)));
                }
                0x25 => {
                    // dup
                    let v = stack.last().cloned().unwrap_or(Value::Null);
                    stack.push(v);
                }
                0x26 => {
                    stack.pop(); // pop
                }
                0x28 | 0x6F => {
                    // call (0x28) / callvirt (0x6F)
                    let tok = u32::from_le_bytes([code[ip], code[ip + 1], code[ip + 2], code[ip + 3]]);
                    ip += 4;
                    self.do_call(tok, &mut stack)?;
                }
                0x45 => {
                    // switch <count> <i32 targets...>
                    let count = read_u32(&code, ip) as usize;
                    ip += 4;
                    let table_end = ip + count * 4;
                    let index = stack.pop().unwrap_or(Value::I4(-1)).as_i4();
                    if index >= 0 && (index as usize) < count {
                        let off = read_i32(&code, ip + index as usize * 4);
                        ip = (table_end as i32 + off) as usize;
                    } else {
                        ip = table_end;
                    }
                }
                0x2A => {
                    // ret
                    self.call_depth -= 1;
                    return Ok(stack.pop());
                }
                0x72 => {
                    // ldstr <US token>
                    let tok = u32::from_le_bytes([code[ip], code[ip + 1], code[ip + 2], code[ip + 3]]);
                    ip += 4;
                    let s = self.img.meta.get_user_string(tok & 0x00FF_FFFF);
                    stack.push(Value::Str(s));
                }
                0x8D => {
                    // newarr <token>
                    let _tok = u32::from_le_bytes([code[ip], code[ip + 1], code[ip + 2], code[ip + 3]]);
                    ip += 4;
                    let size = stack.pop().unwrap_or(Value::I4(0)).as_i4();
                    stack.push(Value::Array(Rc::new(RefCell::new(vec![Value::Null; size.max(0) as usize]))));
                }
                0x8E => {
                    // ldlen
                    let arr = stack.pop().unwrap_or(Value::Null);
                    let len = match arr {
                        Value::Array(a) => a.borrow().len() as i32,
                        _ => 0,
                    };
                    stack.push(Value::I4(len));
                }
                0x9A => {
                    // ldelem.ref
                    let idx = stack.pop().unwrap_or(Value::I4(0)).as_i4();
                    let arr = stack.pop().unwrap_or(Value::Null);
                    let val = match arr {
                        Value::Array(a) => a.borrow().get(idx as usize).cloned().unwrap_or(Value::Null),
                        _ => Value::Null,
                    };
                    stack.push(val);
                }
                0x9B => {
                    // stelem.ref
                    let val = stack.pop().unwrap_or(Value::Null);
                    let idx = stack.pop().unwrap_or(Value::I4(0)).as_i4();
                    // We must pop the array, modify it. Since it's Rc<RefCell>, this works!
                    let arr = stack.pop().unwrap_or(Value::Null);
                    if let Value::Array(a) = arr {
                        if let Some(slot) = a.borrow_mut().get_mut(idx as usize) {
                            *slot = val;
                        }
                    }
                }
                // arithmetic
                0x58 => bin_i4(&mut stack, |a, b| a.wrapping_add(b)), // add
                0x59 => bin_i4(&mut stack, |a, b| a.wrapping_sub(b)), // sub
                0x5A => bin_i4(&mut stack, |a, b| a.wrapping_mul(b)), // mul
                0x5B => bin_i4(&mut stack, |a, b| if b != 0 { a.wrapping_div(b) } else { 0 }), // div
                0x5D => bin_i4(&mut stack, |a, b| if b != 0 { a.wrapping_rem(b) } else { 0 }), // rem
                0x5F => bin_i4(&mut stack, |a, b| a & b),            // and
                0x60 => bin_i4(&mut stack, |a, b| a | b),            // or
                0x61 => bin_i4(&mut stack, |a, b| a ^ b),            // xor
                0x62 => bin_i4(&mut stack, |a, b| a.wrapping_shl(b as u32)), // shl
                0x63 => bin_i4(&mut stack, |a, b| a.wrapping_shr(b as u32)), // shr
                0x65 => {
                    // neg
                    let v = stack.pop().unwrap_or(Value::I4(0)).as_i4();
                    stack.push(Value::I4(v.wrapping_neg()));
                }
                0x66 => {
                    // not
                    let v = stack.pop().unwrap_or(Value::I4(0)).as_i4();
                    stack.push(Value::I4(!v));
                }
                0x69 => {
                    // conv.i4
                    let value = stack.pop().unwrap_or(Value::I4(0)).as_i4();
                    stack.push(Value::I4(value));
                }
                0x6A => {
                    // conv.i8
                    let value = stack.pop().unwrap_or(Value::I4(0)).as_i4();
                    stack.push(Value::I8(value as i64));
                }
                0x73 => {
                    // newobj <constructor token>
                    let tok = read_u32(&code, ip);
                    ip += 4;
                    self.do_newobj(tok, &mut stack)?;
                }
                0x74 | 0x75 => {
                    // castclass / isinst. Object identity is sufficient for the
                    // lightweight managed compatibility layer.
                    ip += 4;
                }
                0x7B => {
                    // ldfld <token>
                    let tok = read_u32(&code, ip);
                    ip += 4;
                    let object = stack.pop().unwrap_or(Value::Null);
                    let value = match object {
                        Value::Object(object) => object.borrow().fields.get(&tok).cloned().unwrap_or(Value::Null),
                        _ => Value::Null,
                    };
                    stack.push(value);
                }
                0x7D => {
                    // stfld <token>
                    let tok = read_u32(&code, ip);
                    ip += 4;
                    let value = stack.pop().unwrap_or(Value::Null);
                    let object = stack.pop().unwrap_or(Value::Null);
                    if let Value::Object(object) = object {
                        object.borrow_mut().fields.insert(tok, value);
                    }
                }
                0x7E => {
                    // ldsfld <token>
                    let tok = read_u32(&code, ip);
                    ip += 4;
                    stack.push(self.static_fields.get(&tok).cloned().unwrap_or(Value::Null));
                }
                0x80 => {
                    // stsfld <token>
                    let tok = read_u32(&code, ip);
                    ip += 4;
                    let value = stack.pop().unwrap_or(Value::Null);
                    self.static_fields.insert(tok, value);
                }
                0x8C | 0xA5 => {
                    // box / unbox.any. Values are already represented uniformly.
                    ip += 4;
                }
                // comparisons (push 0/1)
                0xFE => {
                    // two-byte opcodes
                    let op2 = code[ip]; ip += 1;
                    match op2 {
                        0x01 => cmp_i4(&mut stack, |a, b| a == b),  // ceq
                        0x02 => cmp_i4(&mut stack, |a, b| a > b),   // cgt
                        0x04 => cmp_i4(&mut stack, |a, b| a < b),   // clt
                        0x09 => {
                            // ldarg <u16>
                            let i = u16::from_le_bytes([code[ip], code[ip + 1]]) as usize; ip += 2;
                            stack.push(args.get(i).cloned().unwrap_or(Value::Null));
                        }
                        0x06 | 0x07 => {
                            // ldftn / ldvirtftn <method token>. Delegates are
                            // opaque in this compatibility layer.
                            let tok = read_u32(&code, ip);
                            ip += 4;
                            stack.push(Value::I4(tok as i32));
                        }
                        0x0B => {
                            // starg <u16>
                            let i = u16::from_le_bytes([code[ip], code[ip + 1]]) as usize; ip += 2;
                            let value = stack.pop().unwrap_or(Value::Null);
                            if i < args.len() { args[i] = value; }
                        }
                        0x0C => {
                            // ldloc <u16>
                            let i = u16::from_le_bytes([code[ip], code[ip + 1]]) as usize; ip += 2;
                            stack.push(locals.get(i).cloned().unwrap_or(Value::Null));
                        }
                        0x0E => {
                            // stloc <u16>
                            let i = u16::from_le_bytes([code[ip], code[ip + 1]]) as usize; ip += 2;
                            let v = stack.pop().unwrap_or(Value::Null);
                            if i < locals.len() { locals[i] = v; }
                        }
                        0x15 => {
                            // initobj <token>
                            ip += 4;
                            stack.pop();
                        }
                        _ => return Err(VmError::Unsupported(format!("unimplemented CIL opcode FE {op2:02X}"))),
                    }
                }
                // branches
                0x2B => ip = branch_short(&code, ip, true),                  // br.s
                0x2C => ip = branch_short(&code, ip, !pop_bool(&mut stack)),  // brfalse.s
                0x2D => ip = branch_short(&code, ip, pop_bool(&mut stack)),  // brtrue.s
                0x2E => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| a == b), // beq.s
                0x2F => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| a >= b), // bge.s
                0x30 => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| a > b),  // bgt.s
                0x31 => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| a <= b), // ble.s
                0x32 => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| a < b),  // blt.s
                0x33 => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| a != b), // bne.un.s
                0x34 => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| (a as u32) >= (b as u32)), // bge.un.s
                0x35 => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| (a as u32) > (b as u32)),  // bgt.un.s
                0x36 => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| (a as u32) <= (b as u32)), // ble.un.s
                0x37 => ip = branch_cmp_s(&code, ip, &mut stack, |a, b| (a as u32) < (b as u32)),  // blt.un.s
                0x38 => ip = branch_long(&code, ip, true),                  // br
                0x39 => ip = branch_long(&code, ip, !pop_bool(&mut stack)),   // brfalse
                0x3A => ip = branch_long(&code, ip, pop_bool(&mut stack)),  // brtrue
                0x3B => ip = branch_cmp(&code, ip, &mut stack, |a, b| a == b), // beq
                0x3C => ip = branch_cmp(&code, ip, &mut stack, |a, b| a >= b), // bge
                0x3D => ip = branch_cmp(&code, ip, &mut stack, |a, b| a > b), // bgt
                0x3E => ip = branch_cmp(&code, ip, &mut stack, |a, b| a <= b), // ble
                0x3F => ip = branch_cmp(&code, ip, &mut stack, |a, b| a < b), // blt
                0x40 => ip = branch_cmp(&code, ip, &mut stack, |a, b| a != b), // bne.un
                0x41 => ip = branch_cmp(&code, ip, &mut stack, |a, b| (a as u32) >= (b as u32)),
                0x42 => ip = branch_cmp(&code, ip, &mut stack, |a, b| (a as u32) > (b as u32)),
                0x43 => ip = branch_cmp(&code, ip, &mut stack, |a, b| (a as u32) <= (b as u32)),
                0x44 => ip = branch_cmp(&code, ip, &mut stack, |a, b| (a as u32) < (b as u32)),
                0xA2 => {
                    // stelem.ref
                    let value = stack.pop().unwrap_or(Value::Null);
                    let index = stack.pop().unwrap_or(Value::I4(0)).as_i4();
                    let array = stack.pop().unwrap_or(Value::Null);
                    if let Value::Array(array) = array {
                        if let Some(slot) = array.borrow_mut().get_mut(index.max(0) as usize) {
                            *slot = value;
                        }
                    }
                }
                0xD0 => {
                    // ldtoken <token>
                    let tok = read_u32(&code, ip);
                    ip += 4;
                    stack.push(Value::I4(tok as i32));
                }
                0xDC => {} // endfinally
                0xDD => ip = branch_long(&code, ip, true), // leave
                0xDE => ip = branch_short(&code, ip, true), // leave.s
                _ => return Err(VmError::Unsupported(format!("unimplemented CIL opcode 0x{op:02X}"))),
            }
        }
        self.call_depth -= 1;
        Ok(None)
    }

    fn do_call(&mut self, tok: u32, stack: &mut Vec<Value>) -> Result<()> {
        let target = self.resolve_call(tok)?;
        match target {
            CallTarget::Method { row, argc, returns_value } => {
                let args = pop_args(stack, argc);
                let result = self.run_method(row, args)?;
                if returns_value {
                    stack.push(result.unwrap_or(Value::Null));
                }
            }
            CallTarget::Bcl { key, argc, returns_value } => {
                let args = pop_args(stack, argc);
                let result = webwine_api_net20::dispatch(&key, args, self);
                if returns_value {
                    if let Some(v) = result {
                        stack.push(v);
                    } else {
                        stack.push(Value::Null);
                    }
                }
            }
        }
        Ok(())
    }

    fn do_newobj(&mut self, tok: u32, stack: &mut Vec<Value>) -> Result<()> {
        let target = self.resolve_call(tok)?;
        match target {
            CallTarget::Method { row, argc, .. } => {
                let explicit = pop_args(stack, argc.saturating_sub(1));
                let object = Value::Object(Rc::new(RefCell::new(ManagedObject {
                    type_name: self.img.method_owner(row),
                    fields: HashMap::new(),
                    text: String::new(),
                })));
                let mut args = Vec::with_capacity(explicit.len() + 1);
                args.push(object.clone());
                args.extend(explicit);
                self.run_method(row, args)?;
                stack.push(object);
            }
            CallTarget::Bcl { key, argc, .. } => {
                let explicit = pop_args(stack, argc.saturating_sub(1));
                let object = Value::Object(Rc::new(RefCell::new(ManagedObject {
                    type_name: key.split("::").next().unwrap_or("System.Object").to_string(),
                    fields: HashMap::new(),
                    text: String::new(),
                })));
                let mut args = Vec::with_capacity(explicit.len() + 1);
                args.push(object.clone());
                args.extend(explicit);
                webwine_api_net20::dispatch(&key, args, self);
                stack.push(object);
            }
        }
        Ok(())
    }

    /// Resolve a method token (MethodDef or MemberRef) into a call target.
    fn resolve_call(&self, tok: u32) -> Result<CallTarget> {
        let table = (tok >> 24) as u8;
        let row = tok & 0x00FF_FFFF;
        let m = &self.img.meta;
        match table {
            T_METHODDEF => {
                let sig = m.get_blob(m.col(T_METHODDEF, row, 4));
                let (argc, returns_value) = parse_method_sig(sig);
                Ok(CallTarget::Method { row, argc, returns_value })
            }
            T_MEMBERREF => {
                let (parent_table, parent_row) =
                    m.decode_coded(T_MEMBERREF, row, 0, CodedKind::MemberRefParent);
                let name = m.get_string(m.col(T_MEMBERREF, row, 1));
                let sig = m.get_blob(m.col(T_MEMBERREF, row, 2));
                let (argc, returns_value) = parse_method_sig(sig);

                let type_name = if parent_table == T_TYPEREF {
                    let tn = m.get_string(m.col(T_TYPEREF, parent_row, 1));
                    let ns = m.get_string(m.col(T_TYPEREF, parent_row, 2));
                    if ns.is_empty() { tn } else { format!("{ns}.{tn}") }
                } else {
                    String::new()
                };
                Ok(CallTarget::Bcl { key: format!("{type_name}::{name}"), argc, returns_value })
            }
            _ => Err(VmError::Unsupported(format!(
                "unsupported call token table 0x{table:02X}"
            ))),
        }
    }

    /// Parse a method body header at `rva`, returning (code bytes, local count).
    fn read_body(&self, rva: u32) -> Result<(Vec<u8>, usize)> {
        let head = self
            .img
            .rva_bytes(rva, 12)
            .or_else(|| self.img.rva_bytes(rva, 1))
            .ok_or_else(|| VmError::Pe("method RVA out of range".into()))?;
        let b0 = head[0];
        if b0 & 0x03 == 0x02 {
            // tiny header: size in the top 6 bits, no locals.
            let code_size = (b0 >> 2) as usize;
            let code = self
                .img
                .rva_bytes(rva + 1, code_size)
                .ok_or_else(|| VmError::Pe("tiny method body out of range".into()))?
                .to_vec();
            Ok((code, 0))
        } else {
            let head = self
                .img
                .rva_bytes(rva, 12)
                .ok_or_else(|| VmError::Pe("fat method header out of range".into()))?;
            let code_size =
                u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
            let locals_tok = u32::from_le_bytes([head[8], head[9], head[10], head[11]]);
            let local_count = self.local_count(locals_tok);
            let code = self
                .img
                .rva_bytes(rva + 12, code_size)
                .ok_or_else(|| VmError::Pe("fat method body out of range".into()))?
                .to_vec();
            Ok((code, local_count))
        }
    }

    fn local_count(&self, locals_tok: u32) -> usize {
        if locals_tok == 0 || (locals_tok >> 24) as u8 != T_STANDALONESIG {
            return 0;
        }
        let m = &self.img.meta;
        let sig = m.get_blob(m.col(T_STANDALONESIG, locals_tok & 0x00FF_FFFF, 0));
        // LocalVarSig: 0x07 LOCAL_SIG, then a compressed local count.
        if sig.first() != Some(&0x07) {
            return 0;
        }
        let (count, _) = decompress_uint(&sig[1..]);
        count as usize
    }

    pub fn halt(&mut self, code: i32) {
        self.halted = true;
        self.exit_code = code;
    }

    pub fn is_waiting_for_input(&self) -> bool {
        self.waiting_for_input
    }
}

impl Net20Runtime for ClrRuntime<'_> {
    fn stdout_mut(&mut self) -> &mut String {
        &mut self.stdout
    }

    fn halt(&mut self, code: i32) {
        ClrRuntime::halt(self, code);
    }

    fn show_window(&mut self, title: String) {
        let hwnd = 0x0001_0010;
        self.ui_events.push(UiEvent::CreateWindow {
            hwnd,
            title,
            x: 80,
            y: 60,
            width: 800,
            height: 600,
        });
        self.ui_events.push(UiEvent::ShowWindow { hwnd, show: true });
        self.waiting_for_input = true;
    }
}

/// Parse a method signature blob: returns (arg count to pop, returns a value).
/// Includes the implicit `this` argument for instance methods (HASTHIS).
fn parse_method_sig(sig: &[u8]) -> (usize, bool) {
    if sig.is_empty() {
        return (0, false);
    }
    let flags = sig[0];
    let has_this = flags & 0x20 != 0;
    let mut i = 1;
    // Skip generic arity if present (GENERIC flag 0x10).
    if flags & 0x10 != 0 {
        let (_, n) = decompress_uint(&sig[i..]);
        i += n;
    }
    let (param_count, n) = decompress_uint(&sig[i..]);
    i += n;
    // Return type: first element. VOID (0x01) means no value is produced.
    let returns_value = sig.get(i).copied().unwrap_or(0x01) != 0x01;
    (param_count as usize + has_this as usize, returns_value)
}

fn pop_args(stack: &mut Vec<Value>, argc: usize) -> Vec<Value> {
    let n = argc.min(stack.len());
    let mut args = stack.split_off(stack.len() - n);
    let _ = &mut args; // args are already in call order (arg0..argN-1)
    args
}

fn bin_i4(stack: &mut Vec<Value>, f: impl Fn(i32, i32) -> i32) {
    let b = stack.pop().map(|v| v.as_i4()).unwrap_or(0);
    let a = stack.pop().map(|v| v.as_i4()).unwrap_or(0);
    stack.push(Value::I4(f(a, b)));
}

fn cmp_i4(stack: &mut Vec<Value>, f: impl Fn(i32, i32) -> bool) {
    let b = stack.pop().map(|v| v.as_i4()).unwrap_or(0);
    let a = stack.pop().map(|v| v.as_i4()).unwrap_or(0);
    stack.push(Value::I4(f(a, b) as i32));
}

fn pop_bool(stack: &mut Vec<Value>) -> bool {
    !matches!(stack.pop(), Some(Value::I4(0)) | Some(Value::Null) | None)
}

fn branch_short(code: &[u8], ip: usize, take: bool) -> usize {
    let off = code[ip] as i8 as i32;
    let next = ip + 1;
    if take { (next as i32 + off) as usize } else { next }
}

fn branch_long(code: &[u8], ip: usize, take: bool) -> usize {
    let off = i32::from_le_bytes([code[ip], code[ip + 1], code[ip + 2], code[ip + 3]]);
    let next = ip + 4;
    if take { (next as i32 + off) as usize } else { next }
}

fn branch_cmp_s(code: &[u8], ip: usize, stack: &mut Vec<Value>, f: impl Fn(i32, i32) -> bool) -> usize {
    let b = stack.pop().map(|v| v.as_i4()).unwrap_or(0);
    let a = stack.pop().map(|v| v.as_i4()).unwrap_or(0);
    branch_short(code, ip, f(a, b))
}

fn branch_cmp(code: &[u8], ip: usize, stack: &mut Vec<Value>, f: impl Fn(i32, i32) -> bool) -> usize {
    let b = stack.pop().map(|v| v.as_i4()).unwrap_or(0);
    let a = stack.pop().map(|v| v.as_i4()).unwrap_or(0);
    branch_long(code, ip, f(a, b))
}

fn read_u32(code: &[u8], ip: usize) -> u32 {
    u32::from_le_bytes([code[ip], code[ip + 1], code[ip + 2], code[ip + 3]])
}

fn read_i32(code: &[u8], ip: usize) -> i32 {
    i32::from_le_bytes([code[ip], code[ip + 1], code[ip + 2], code[ip + 3]])
}
