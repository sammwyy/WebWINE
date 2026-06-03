// Minimal Base Class Library, implemented as internal calls. Managed code calls
// into mscorlib through MemberRefs; rather than interpret mscorlib's own IL we
// intercept the handful of methods console programs need, keyed by
// "Namespace.Type::Method" — the same interception strategy as the Win32 layer.

use super::interp::{ClrRuntime, Value};

/// Dispatch a BCL call. `args` are in call order (arg0 first). Returns a value
/// for non-void methods.
pub fn dispatch(key: &str, args: Vec<Value>, rt: &mut ClrRuntime) -> Option<Value> {
    match key {
        "System.Console::WriteLine" => {
            for a in &args {
                rt.stdout.push_str(&a.display());
            }
            rt.stdout.push('\n');
            None
        }
        "System.Console::Write" => {
            for a in &args {
                rt.stdout.push_str(&a.display());
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
        "System.Object::.ctor" => None, // base object constructor: nothing to do
        _ => {
            // Unknown BCL method: ignore its effect but keep running. A wrong
            // guess here is contained; the program usually still makes progress.
            None
        }
    }
}
