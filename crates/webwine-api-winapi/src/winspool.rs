//! winspool.drv — printer spooler. No printers in the browser sandbox.

use super::{ApiContext, Handled, WinApiRegistry};

const ERROR_INVALID_PRINTER_NAME: u32 = 1801;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, super::HandlerFn)] = &[
        ("OpenPrinterA", |c| open_printer(c, false)),
        ("OpenPrinterW", |c| open_printer(c, true)),
        ("ClosePrinter", close_printer),
        ("GetPrinterDriverA", |c| get_printer_driver(c, false)),
        ("GetPrinterDriverW", |c| get_printer_driver(c, true)),
        ("EnumPrintersA", |c| enum_printers(c, false)),
        ("EnumPrintersW", |c| enum_printers(c, true)),
        ("DocumentPropertiesA", document_properties),
        ("DocumentPropertiesW", document_properties),
        ("GetDefaultPrinterA", |c| get_default_printer(c, false)),
        ("GetDefaultPrinterW", |c| get_default_printer(c, true)),
        ("StartDocPrinterA", start_doc_printer),
        ("StartDocPrinterW", start_doc_printer),
        ("EndDocPrinter", end_doc_printer),
        ("StartPagePrinter", start_page_printer),
        ("EndPagePrinter", end_page_printer),
        ("WritePrinter", write_printer),
        ("GetPrinterA", get_printer),
        ("GetPrinterW", get_printer),
    ];
    for &(name, handler) in fns {
        r.add("winspool.drv", name, handler);
    }
}

fn open_printer(c: &mut ApiContext, wide: bool) -> Handled {
    // BOOL OpenPrinter(name, phPrinter, defaults)
    let name_ptr = c.arg(0);
    let ph = c.arg(1);
    let _ = wide;
    if ph != 0 {
        let _ = c.memory.write_u32(ph, 0);
    }
    if name_ptr != 0 {
        c.cpu.last_error = ERROR_INVALID_PRINTER_NAME;
    } else {
        c.cpu.last_error = ERROR_INVALID_PRINTER_NAME;
    }
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn close_printer(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}

fn get_printer_driver(c: &mut ApiContext, _wide: bool) -> Handled {
    // BOOL GetPrinterDriver(hPrinter, pEnvironment, Level, pDriverInfo, cbBuf, pcbNeeded)
    let needed = c.arg(5);
    if needed != 0 {
        let _ = c.memory.write_u32(needed, 0);
    }
    c.cpu.last_error = ERROR_INVALID_PRINTER_NAME;
    c.ret_stdcall(0, 6);
    Handled::Ok
}

fn enum_printers(c: &mut ApiContext, _wide: bool) -> Handled {
    // BOOL EnumPrinters(Flags, Name, Level, pPrinterEnum, cbBuf, pcbNeeded, pcReturned)
    let needed = c.arg(5);
    let returned = c.arg(6);
    if needed != 0 {
        let _ = c.memory.write_u32(needed, 0);
    }
    if returned != 0 {
        let _ = c.memory.write_u32(returned, 0);
    }
    // Success with zero printers.
    c.ret_stdcall(1, 7);
    Handled::Ok
}

fn document_properties(c: &mut ApiContext) -> Handled {
    // LONG DocumentProperties(...) — return required DEVMODE size as -1 on error,
    // or size when fMode == 0. Report failure.
    c.ret_stdcall(u32::MAX, 6);
    Handled::Ok
}

fn get_default_printer(c: &mut ApiContext, wide: bool) -> Handled {
    // BOOL GetDefaultPrinter(pszBuffer, pcchBuffer)
    let buf = c.arg(0);
    let pcch = c.arg(1);
    let name = "WebWINE Printer";
    let needed = if wide {
        name.encode_utf16().count() + 1
    } else {
        name.len() + 1
    };
    if pcch != 0 {
        let have = c.memory.read_u32(pcch).unwrap_or(0) as usize;
        if buf == 0 || have < needed {
            let _ = c.memory.write_u32(pcch, needed as u32);
            c.cpu.last_error = ERROR_INSUFFICIENT_BUFFER;
            c.ret_stdcall(0, 2);
            return Handled::Ok;
        }
        if wide {
            for (i, u) in name.encode_utf16().enumerate() {
                let _ = c.memory.write_u16(buf + i as u32 * 2, u);
            }
            let _ = c
                .memory
                .write_u16(buf + name.encode_utf16().count() as u32 * 2, 0);
        } else {
            let _ = c.memory.write_bytes(buf, name.as_bytes());
            let _ = c.memory.write_u8(buf + name.len() as u32, 0);
        }
        let _ = c.memory.write_u32(pcch, needed as u32);
    }
    c.ret_stdcall(1, 2);
    Handled::Ok
}

fn start_doc_printer(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3); // job id 0 = failure
    Handled::Ok
}

fn end_doc_printer(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}

fn start_page_printer(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}

fn end_page_printer(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}

fn write_printer(c: &mut ApiContext) -> Handled {
    let written = c.arg(3);
    if written != 0 {
        let _ = c.memory.write_u32(written, 0);
    }
    c.ret_stdcall(0, 4);
    Handled::Ok
}

fn get_printer(c: &mut ApiContext) -> Handled {
    let needed = c.arg(4);
    if needed != 0 {
        let _ = c.memory.write_u32(needed, 0);
    }
    c.ret_stdcall(0, 5);
    Handled::Ok
}
