use std::cell::RefCell;
use std::rc::Rc;

/// A value on the managed evaluation stack.
#[derive(Clone, Debug)]
pub enum Value {
    I4(i32),
    I8(i64),
    R8(f64),
    Str(String),
    Array(Rc<RefCell<Vec<Value>>>),
    Null,
}

impl Value {
    pub fn as_i4(&self) -> i32 {
        match self {
            Value::I4(v) => *v,
            Value::I8(v) => *v as i32,
            Value::R8(v) => *v as i32,
            _ => 0,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::I4(v) => v.to_string(),
            Value::I8(v) => v.to_string(),
            Value::R8(v) => v.to_string(),
            Value::Str(s) => s.clone(),
            Value::Array(a) => format!("System.Object[{}]", a.borrow().len()),
            Value::Null => String::new(),
        }
    }
}

pub trait Net20Runtime {
    fn stdout_mut(&mut self) -> &mut String;
    fn halt(&mut self, code: i32);
}

/// Dispatch a BCL call. `args` are in call order (arg0 first). Returns a value
/// for non-void methods.
pub fn dispatch<R: Net20Runtime>(key: &str, args: Vec<Value>, rt: &mut R) -> Option<Value> {
    match key {
        "System.Console::WriteLine" => {
            for a in &args {
                rt.stdout_mut().push_str(&a.display());
            }
            rt.stdout_mut().push('\n');
            None
        }
        "System.Console::Write" => {
            for a in &args {
                rt.stdout_mut().push_str(&a.display());
            }
            None
        }
        "System.String::Concat" => {
            let mut s = String::new();
            for a in &args {
                s.push_str(&a.display());
            }
            Some(Value::Str(s))
        }
        "System.Int32::ToString" | "System.Object::ToString" | "System.String::ToString" => {
            Some(Value::Str(args.first().map(|v| v.display()).unwrap_or_default()))
        }
        "System.Environment::Exit" => {
            let code = args.first().map(|v| v.as_i4()).unwrap_or(0);
            rt.halt(code);
            None
        }
        "System.Object::.ctor" => None,
        _ => None,
    }
}
