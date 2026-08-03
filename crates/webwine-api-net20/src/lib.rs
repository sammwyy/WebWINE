use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct ManagedObject {
    pub type_name: String,
    pub fields: HashMap<u32, Value>,
    pub text: String,
}

/// A value on the managed evaluation stack.
#[derive(Clone, Debug)]
pub enum Value {
    I4(i32),
    I8(i64),
    R8(f64),
    Str(String),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<ManagedObject>>),
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
            Value::Object(o) => {
                let object = o.borrow();
                if object.text.is_empty() { object.type_name.clone() } else { object.text.clone() }
            }
            Value::Null => String::new(),
        }
    }
}

pub trait Net20Runtime {
    fn stdout_mut(&mut self) -> &mut String;
    fn halt(&mut self, code: i32);
    fn show_window(&mut self, _title: String) {}
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
        "System.Windows.Forms.Control::set_Text" | "System.Windows.Forms.Form::set_Text" => {
            if let (Some(Value::Object(object)), Some(value)) = (args.first(), args.get(1)) {
                object.borrow_mut().text = value.display();
            }
            None
        }
        "System.Windows.Forms.Control::get_Text" | "System.Windows.Forms.Form::get_Text" => {
            match args.first() {
                Some(Value::Object(object)) => Some(Value::Str(object.borrow().text.clone())),
                _ => Some(Value::Str(String::new())),
            }
        }
        "System.Windows.Forms.Application::Run" => {
            let title = args.first().map(Value::display)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Managed Windows application".to_string());
            rt.show_window(title);
            None
        }
        "System.Object::.ctor" => None,
        _ => None,
    }
}
