use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::vm::process::{MenuItemData, UiEvent, WindowEntry};

const MODULE_STATE_KEY: &str = "mfc42u.module_state";
const MAIN_WINDOW_KEY: &str = "mfc42u.main_window";

/// AfxGetModuleState (MFC42U ordinal 1165).
///
/// MFC callers retain this pointer and access fields beyond offset 0x2000, so
/// it must be a stable, process-lifetime allocation rather than a tiny fresh
/// buffer.  Zero initialization is a safe baseline for the state we do not yet
/// model, and lets MFC's own startup routines populate the fields they own.
fn afx_get_module_state(c: &mut ApiContext) -> Handled {
    let ptr = match c.dll_state.get(MODULE_STATE_KEY).copied() {
        Some(ptr) => ptr,
        None => {
            let ptr = c.heap_alloc(0x4000);
            c.dll_state.insert(MODULE_STATE_KEY.to_string(), ptr);
            ptr
        }
    };
    c.ret_cdecl(ptr);
    Handled::Ok
}

/// Lightweight MFC object constructors invoked with `this` in ECX and no
/// stack arguments. Preserving their thiscall stack shape avoids corrupting
/// the surrounding CRT initializer frame.
fn noop_thiscall_ctor(c: &mut ApiContext) -> Handled {
    let this = c.cpu.ecx;
    c.ret_cdecl(this);
    Handled::Ok
}

fn noop_thiscall_ctor_1(c: &mut ApiContext) -> Handled {
    let this = c.cpu.ecx;
    c.ret_stdcall(this, 1);
    Handled::Ok
}

/// AfxWinMain (MFC42U ordinal 1569).
///
/// MFC 4.2 is a large C++ framework whose public ABI is mostly ordinal-only.
/// Until its CWnd/CFrameWnd object model is complete, keep MFC applications
/// alive behind a real host window instead of returning from WinMain before
/// they can show anything. The bridge owns the window/message wait and exits
/// cleanly when the frontend posts WM_CLOSE.
fn afx_win_main(c: &mut ApiContext) -> Handled {
    if let Some(&hwnd) = c.dll_state.get(MAIN_WINDOW_KEY) {
        if let Some(message) = c.gui.queue.pop_front() {
            if message.message == 0x0010 {
                c.gui.windows.remove(&hwnd);
                c.ui_events.push(UiEvent::DestroyWindow { hwnd });
                c.dll_state.remove(MAIN_WINDOW_KEY);
                c.ret_stdcall(0, 4);
                return Handled::Ok;
            }
        }
        return Handled::Block;
    }

    let filename = c.exe_path.rsplit(['\\', '/']).next().unwrap_or(c.exe_path);
    let app = filename.strip_suffix(".exe").or_else(|| filename.strip_suffix(".EXE"))
        .unwrap_or(filename);
    let is_paint = app.eq_ignore_ascii_case("mspaint");
    let title = if is_paint { "untitled - Paint".to_string() } else { app.to_string() };
    let hwnd = c.gui.next_hwnd;
    c.gui.next_hwnd += 4;
    c.gui.windows.insert(hwnd, WindowEntry::new_toplevel(0, 800, 600, "mfc42u", &title));
    c.dll_state.insert(MAIN_WINDOW_KEY.to_string(), hwnd);
    c.ui_events.push(UiEvent::CreateWindow {
        hwnd,
        title,
        x: 70,
        y: 50,
        width: 800,
        height: 600,
    });
    c.ui_events.push(UiEvent::ShowWindow { hwnd, show: true });
    c.ui_events.push(UiEvent::FillRect {
        hwnd,
        x: 0,
        y: 0,
        w: 800,
        h: 600,
        color: 0xFF_FFFF,
    });
    if is_paint {
        let labels = ["File", "Edit", "View", "Image", "Colors", "Help"];
        let items = labels.into_iter().enumerate().map(|(i, text)| MenuItemData {
            text: text.to_string(),
            id: 0xE000 + i as u32,
            separator: false,
            disabled: false,
            children: Vec::new(),
        }).collect();
        c.ui_events.push(UiEvent::SetMenu { hwnd, items });
    }
    Handled::Block
}

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("mfc42u.dll", "#1165", afx_get_module_state),
        ("mfc42u.dll", "#323", noop_thiscall_ctor),
        ("mfc42u.dll", "#540", noop_thiscall_ctor),
        ("mfc42u.dll", "#459", noop_thiscall_ctor),
        ("mfc42u.dll", "#414", noop_thiscall_ctor),
        ("mfc42u.dll", "#561", noop_thiscall_ctor_1),
        ("mfc42u.dll", "#415", noop_thiscall_ctor_1),
        ("mfc42u.dll", "#1569", afx_win_main),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}
