use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, super::HandlerFn)] = &[
        ("OpenPrinterA", |c| {
            if c.arg(1) != 0 { let _ = c.memory.write_u32(c.arg(1), 0); }
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("OpenPrinterW", |c| {
            if c.arg(1) != 0 { let _ = c.memory.write_u32(c.arg(1), 0); }
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("ClosePrinter", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("GetPrinterDriverA", |c| {
            if c.arg(5) != 0 { let _ = c.memory.write_u32(c.arg(5), 0); }
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        ("GetPrinterDriverW", |c| {
            if c.arg(5) != 0 { let _ = c.memory.write_u32(c.arg(5), 0); }
            c.ret_stdcall(0, 6);
            Handled::Ok
        }),
        ("EnumPrintersA", enum_printers),
        ("EnumPrintersW", enum_printers),
        ("DocumentPropertiesA", |c| { c.ret_stdcall(u32::MAX, 6); Handled::Ok }),
        ("DocumentPropertiesW", |c| { c.ret_stdcall(u32::MAX, 6); Handled::Ok }),
    ];
    for &(name, handler) in fns {
        r.add("winspool.drv", name, handler);
    }
}

fn enum_printers(c: &mut super::ApiContext) -> Handled {
    if c.arg(5) != 0 { let _ = c.memory.write_u32(c.arg(5), 0); }
    if c.arg(6) != 0 { let _ = c.memory.write_u32(c.arg(6), 0); }
    c.ret_stdcall(1, 7);
    Handled::Ok
}
