//! Wine-aligned dialog manager (ported from `.refs/wine/dlls/user32/dialog.c`).
//!
//! Parses `DLGTEMPLATE` / `DIALOGEX`, creates the dialog HWND + child controls,
//! and implements GetDlgItem / SetDlgItemText / EndDialog / IsDialogMessage, etc.

use super::{ApiContext, Handled};
use webwine_api::vm::process::{DialogControlData, GuestMsg, UiEvent, WindowEntry};

// Win32 constants

const WM_INITDIALOG: u32 = 0x0110;
const WM_COMMAND: u32 = 0x0111;
const WM_CLOSE: u32 = 0x0010;
const WM_SETTEXT: u32 = 0x000C;
const WM_GETTEXT: u32 = 0x000D;
const WM_GETTEXTLENGTH: u32 = 0x000E;

const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_DISABLED: u32 = 0x0800_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_GROUP: u32 = 0x0002_0000;
const WS_BORDER: u32 = 0x0080_0000;
const WS_POPUP: u32 = 0x8000_0000;
const WS_EX_CLIENTEDGE: u32 = 0x0000_0200;
const WS_EX_NOPARENTNOTIFY: u32 = 0x0000_0004;

const DS_SETFONT: u32 = 0x0000_0040;
const DS_NOFAILCREATE: u32 = 0x0000_0010;
const DS_CONTROL: u32 = 0x0000_0400;
const DS_MODALFRAME: u32 = 0x0000_0080;

const BS_PUSHBUTTON: u32 = 0x0000_0000;
const BS_DEFPUSHBUTTON: u32 = 0x0000_0001;
const BS_CHECKBOX: u32 = 0x0000_0002;
const BS_AUTOCHECKBOX: u32 = 0x0000_0003;
const BS_RADIOBUTTON: u32 = 0x0000_0004;
const BS_3STATE: u32 = 0x0000_0005;
const BS_AUTO3STATE: u32 = 0x0000_0006;
const BS_GROUPBOX: u32 = 0x0000_0007;
const BS_AUTORADIOBUTTON: u32 = 0x0000_0009;
const BS_TYPEMASK: u32 = 0x0000_000F;

const BN_CLICKED: u32 = 0;
const IDOK: i32 = 1;
const IDCANCEL: i32 = 2;

const SS_LEFT: u32 = 0x0000_0000;
const SS_CENTER: u32 = 0x0000_0001;
const SS_RIGHT: u32 = 0x0000_0002;
const SS_ICON: u32 = 0x0000_0003;
const SS_TYPEMASK: u32 = 0x0000_001F;

/// Default dialog base units (system font, Wine/classic ~8×16).
const DEFAULT_X_BASE: u32 = 8;
const DEFAULT_Y_BASE: u32 = 16;

// Parsed structures

#[derive(Debug, Clone)]
pub struct DlgTemplate {
    pub style: u32,
    pub ex_style: u32,
    pub help_id: u32,
    pub nb_items: u16,
    pub x: i16,
    pub y: i16,
    pub cx: i16,
    pub cy: i16,
    pub caption: String,
    pub point_size: i16,
    pub dialog_ex: bool,
    pub controls: Vec<DlgControl>,
}

#[derive(Debug, Clone)]
pub struct DlgControl {
    pub style: u32,
    pub ex_style: u32,
    pub help_id: u32,
    pub x: i16,
    pub y: i16,
    pub cx: i16,
    pub cy: i16,
    pub id: i32,
    pub class_name: String,
    pub window_name: String,
}

// Byte reader

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    fn remaining(&self) -> bool {
        self.pos < self.data.len()
    }
    fn align_dword(&mut self) {
        self.pos = (self.pos + 3) & !3;
    }
    fn u16(&mut self) -> u16 {
        if self.pos + 2 > self.data.len() {
            return 0;
        }
        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        v
    }
    fn u32(&mut self) -> u32 {
        if self.pos + 4 > self.data.len() {
            return 0;
        }
        let v = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        v
    }
    fn i16(&mut self) -> i16 {
        self.u16() as i16
    }
    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }
    /// Null-terminated UTF-16 string.
    fn wstr(&mut self) -> String {
        let mut units = Vec::new();
        loop {
            let u = self.u16();
            if u == 0 {
                break;
            }
            units.push(u);
        }
        String::from_utf16_lossy(&units)
    }
}

fn builtin_class(id: u16) -> Option<&'static str> {
    // Wine: 0x80-0x85 and 0-5 map to the same six builtins.
    let id = if (0x80..=0x85).contains(&id) {
        id - 0x80
    } else {
        id
    };
    match id {
        0 => Some("Button"),
        1 => Some("Edit"),
        2 => Some("Static"),
        3 => Some("ListBox"),
        4 => Some("ScrollBar"),
        5 => Some("ComboBox"),
        _ => None,
    }
}

/// Parse a dialog template (Wine `DIALOG_ParseTemplate32` + controls).
pub fn parse_dialog_template(data: &[u8]) -> Option<DlgTemplate> {
    if data.len() < 18 {
        return None;
    }
    let mut c = Cursor::new(data);

    let w0 = c.u16();
    let w1 = c.u16();
    let (dialog_ex, style, ex_style, help_id) = if w0 == 1 && w1 == 0xffff {
        let help_id = c.u32();
        let ex_style = c.u32();
        let style = c.u32();
        (true, style, ex_style, help_id)
    } else {
        // Classic: first DWORD is style (we already consumed two WORDs of it).
        let style = (w0 as u32) | ((w1 as u32) << 16);
        let ex_style = c.u32();
        (false, style, ex_style, 0)
    };

    let nb_items = c.u16();
    let x = c.i16();
    let y = c.i16();
    let cx = c.i16();
    let cy = c.i16();

    // Menu name
    match c.u16() {
        0x0000 => {}
        0xffff => {
            let _ = c.u16();
        }
        _ => {
            c.pos -= 2;
            let _ = c.wstr();
        }
    }

    // Class name
    match c.u16() {
        0x0000 => {}
        0xffff => {
            let _ = c.u16();
        }
        _ => {
            c.pos -= 2;
            let _ = c.wstr();
        }
    }

    let caption = c.wstr();

    let mut point_size = 0i16;
    if style & DS_SETFONT != 0 {
        point_size = c.i16();
        if dialog_ex {
            let _weight = c.u16();
            let _italic = c.u16(); // Wine: BYTE italic + BYTE charset packed; read as word
                                   // Actually dialogEx: WORD weight, BYTE italic, BYTE charset
                                   // We already read weight as u16; italic+charset is next u16
        }
        let _face = c.wstr();
    }

    c.align_dword();

    let mut controls = Vec::with_capacity(nb_items as usize);
    for _ in 0..nb_items {
        if !c.remaining() {
            break;
        }
        let ctrl = parse_control(&mut c, dialog_ex)?;
        controls.push(ctrl);
        c.align_dword();
    }

    Some(DlgTemplate {
        style,
        ex_style,
        help_id,
        nb_items,
        x,
        y,
        cx,
        cy,
        caption,
        point_size,
        dialog_ex,
        controls,
    })
}

fn parse_control(c: &mut Cursor<'_>, dialog_ex: bool) -> Option<DlgControl> {
    let (help_id, style, ex_style) = if dialog_ex {
        let help_id = c.u32();
        let ex_style = c.u32();
        let style = c.u32();
        (help_id, style, ex_style)
    } else {
        let style = c.u32();
        let ex_style = c.u32();
        (0, style, ex_style)
    };
    let x = c.i16();
    let y = c.i16();
    let cx = c.i16();
    let cy = c.i16();
    let id = if dialog_ex {
        c.i32()
    } else {
        c.u16() as i16 as i32
    };

    // Class
    let class_name = match c.u16() {
        0xffff => {
            let cid = c.u16();
            builtin_class(cid).unwrap_or("Static").to_string()
        }
        _ => {
            c.pos -= 2;
            c.wstr()
        }
    };

    // Title
    let window_name = match c.u16() {
        0xffff => {
            let _ord = c.u16();
            String::new() // ordinal resource (icon etc.)
        }
        _ => {
            c.pos -= 2;
            c.wstr()
        }
    };

    // Creation data
    let data_words = c.u16() as usize;
    if data_words > 0 {
        c.pos = c.pos.saturating_add(data_words.saturating_mul(2));
        // Wine: count includes the size word itself for some paths; we skip `data_words` WORDs
        // after already consuming the length WORD. Match Wine: `p += GET_WORD(p) / sizeof(WORD)`
        // then p++. Our length was already consumed; skip remaining.
    }

    Some(DlgControl {
        style,
        ex_style,
        help_id,
        x,
        y,
        cx,
        cy,
        id,
        class_name,
        window_name,
    })
}

fn mul_div(v: i32, num: i32, den: i32) -> i32 {
    if den == 0 {
        return v;
    }
    ((v as i64) * (num as i64) / (den as i64)) as i32
}

fn dlg_to_px(x: i16, y: i16, cx: i16, cy: i16, x_base: u32, y_base: u32) -> (i32, i32, i32, i32) {
    (
        mul_div(x as i32, x_base as i32, 4),
        mul_div(y as i32, y_base as i32, 8),
        mul_div(cx as i32, x_base as i32, 4).max(1),
        mul_div(cy as i32, y_base as i32, 8).max(1),
    )
}

// Public API handlers

/// CreateDialogParamA/W / DialogBoxParamA/W — load RT_DIALOG then create.
pub fn create_dialog_param(ctx: &mut ApiContext, unicode: bool) -> Handled {
    let _hinst = ctx.arg(0);
    let name_arg = ctx.arg(1);
    let parent = ctx.arg(2);
    let dlg_proc = ctx.arg(3);
    let init_param = ctx.arg(4);

    let template = resolve_dialog_template(ctx, name_arg, unicode);
    let Some(bytes) = template else {
        ctx.logs.log(
            webwine_api::logs::LogLevel::Warn,
            "dialog",
            &format!("CreateDialogParam: template not found (arg=0x{name_arg:08X})"),
            Some(ctx.pid),
        );
        ctx.ret_stdcall(0, 5);
        return Handled::Ok;
    };
    let hwnd = create_dialog_from_bytes(ctx, &bytes, parent, dlg_proc, init_param, false);
    ctx.ret_stdcall(hwnd, 5);
    Handled::Ok
}

/// CreateDialogIndirectParamA/W / DialogBoxIndirectParamA/W.
pub fn create_dialog_indirect(ctx: &mut ApiContext, _unicode: bool) -> Handled {
    let _hinst = ctx.arg(0);
    let tmpl = ctx.arg(1);
    let parent = ctx.arg(2);
    let dlg_proc = ctx.arg(3);
    let init_param = ctx.arg(4);

    // Read a generous blob from guest memory (dialogs are small).
    let bytes = ctx.memory.read_bytes(tmpl, 0x8000).unwrap_or_default();
    if bytes.is_empty() {
        ctx.ret_stdcall(0, 5);
        return Handled::Ok;
    }
    let hwnd = create_dialog_from_bytes(ctx, &bytes, parent, dlg_proc, init_param, false);
    ctx.ret_stdcall(hwnd, 5);
    Handled::Ok
}

/// DialogBox* — modeless-equivalent create; apps that call EndDialog still work.
pub fn dialog_box_param(ctx: &mut ApiContext, unicode: bool) -> Handled {
    // Same as CreateDialogParam for now; 7-Zip uses CreateDialog + message loop.
    create_dialog_param(ctx, unicode)
}

pub fn dialog_box_indirect(ctx: &mut ApiContext, unicode: bool) -> Handled {
    create_dialog_indirect(ctx, unicode)
}

fn resolve_dialog_template(ctx: &ApiContext, name_arg: u32, unicode: bool) -> Option<Vec<u8>> {
    // INTRESOURCE: high word 0
    if name_arg < 0x1_0000 {
        return ctx.dialogs.get(&name_arg).cloned();
    }
    let name = if unicode {
        ctx.wstr(name_arg)
    } else {
        ctx.cstr(name_arg)
    };
    if name.is_empty() {
        return None;
    }
    // Numeric string "#123"
    if let Some(rest) = name.strip_prefix('#') {
        if let Ok(id) = rest.parse::<u32>() {
            return ctx.dialogs.get(&id).cloned();
        }
    }
    ctx.dialogs_by_name
        .get(&name.to_ascii_lowercase())
        .cloned()
        .or_else(|| {
            // Try parse as decimal id string
            name.parse::<u32>()
                .ok()
                .and_then(|id| ctx.dialogs.get(&id).cloned())
        })
}

fn create_dialog_from_bytes(
    ctx: &mut ApiContext,
    bytes: &[u8],
    _parent: u32,
    dlg_proc: u32,
    init_param: u32,
    _modal: bool,
) -> u32 {
    let Some(tmpl) = parse_dialog_template(bytes) else {
        ctx.logs.log(
            webwine_api::logs::LogLevel::Warn,
            "dialog",
            "failed to parse dialog template",
            Some(ctx.pid),
        );
        return 0;
    };

    let x_base = DEFAULT_X_BASE;
    let y_base = DEFAULT_Y_BASE;
    let (_px, _py, pw, ph) = dlg_to_px(tmpl.x, tmpl.y, tmpl.cx, tmpl.cy, x_base, y_base);
    // Client size: add a little chrome margin for non-DS_CONTROL dialogs.
    let width = pw.max(120);
    let height = ph.max(80);

    let hwnd = ctx.gui.next_hwnd;
    ctx.gui.next_hwnd += 4;

    let mut entry = WindowEntry::new_toplevel(
        dlg_proc, // DispatchMessage will call this as dialog proc
        width,
        height,
        "#32770",
        &tmpl.caption,
    );
    entry.is_dialog = true;
    entry.dlg_proc = if dlg_proc != 0 { Some(dlg_proc) } else { None };
    entry.x_base_unit = x_base;
    entry.y_base_unit = y_base;
    entry.style = tmpl.style & !WS_VISIBLE; // Wine strips visible until after init
    entry.ex_style = tmpl.ex_style;
    entry.visible = false;

    ctx.gui.windows.insert(hwnd, entry);

    ctx.ui_events.push(UiEvent::CreateWindow {
        hwnd,
        title: tmpl.caption.clone(),
        x: 100,
        y: 60,
        width,
        height,
    });

    // Create child controls (Wine DIALOG_CreateControls32).
    let mut control_data = Vec::new();
    let mut first_tab: u32 = 0;
    let mut def_button: u32 = 0;

    for ctrl in &tmpl.controls {
        let mut style = ctrl.style;
        style &= !WS_POPUP;
        style |= WS_CHILD;
        let mut ex_style = ctrl.ex_style | WS_EX_NOPARENTNOTIFY;
        if style & WS_BORDER != 0 {
            style &= !WS_BORDER;
            ex_style |= WS_EX_CLIENTEDGE;
        }

        let (cx, cy, cw, ch) = dlg_to_px(ctrl.x, ctrl.y, ctrl.cx, ctrl.cy, x_base, y_base);
        let child = ctx.gui.next_hwnd;
        ctx.gui.next_hwnd += 4;

        let class = if ctrl.class_name.is_empty() {
            "Static".to_string()
        } else {
            ctrl.class_name.clone()
        };

        let mut child_entry = WindowEntry::new_toplevel(0, cw, ch, &class, &ctrl.window_name);
        child_entry.parent = Some(hwnd);
        child_entry.id = ctrl.id;
        child_entry.style = style;
        child_entry.ex_style = ex_style;
        child_entry.x = cx;
        child_entry.y = cy;
        child_entry.visible = style & WS_VISIBLE != 0 || true; // children default visible in dialogs
        child_entry.enabled = style & WS_DISABLED == 0;
        child_entry.needs_paint = false;

        // Default pushbutton tracking
        let btn_type = style & BS_TYPEMASK;
        if class.eq_ignore_ascii_case("Button") && btn_type == BS_DEFPUSHBUTTON {
            def_button = child;
        }

        ctx.gui.windows.insert(child, child_entry);
        ctx.gui.dlg_ctrl.insert((hwnd, ctrl.id), child);
        if let Some(parent) = ctx.gui.windows.get_mut(&hwnd) {
            parent.children.push(child);
        }

        if first_tab == 0 && style & WS_TABSTOP != 0 {
            first_tab = child;
        }

        control_data.push(DialogControlData {
            hwnd: child,
            id: ctrl.id,
            class_name: class,
            text: ctrl.window_name.clone(),
            x: cx,
            y: cy,
            w: cw,
            h: ch,
            style,
            enabled: style & WS_DISABLED == 0,
            checked: false,
            visible: true,
        });
    }

    // Emit layout so the frontend paints controls immediately.
    ctx.ui_events.push(UiEvent::DialogLayout {
        hwnd,
        title: tmpl.caption.clone(),
        width,
        height,
        controls: control_data,
    });

    // WM_INITDIALOG (Wine order).
    let focus = if first_tab != 0 {
        first_tab
    } else {
        ctx.gui
            .windows
            .get(&hwnd)
            .and_then(|w| w.children.first().copied())
            .unwrap_or(0)
    };
    ctx.gui.queue.push_back(GuestMsg {
        hwnd,
        message: WM_INITDIALOG,
        wparam: focus,
        lparam: init_param,
    });

    // Show if template wanted visible (Wine does this after INITDIALOG returns TRUE;
    // we show proactively so the user sees chrome even before the dlgProc runs).
    if tmpl.style & WS_VISIBLE != 0 || tmpl.style & DS_CONTROL == 0 {
        if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
            w.visible = true;
            w.style |= WS_VISIBLE;
            w.needs_paint = true;
        }
        ctx.ui_events.push(UiEvent::ShowWindow { hwnd, show: true });
    }

    let _ = def_button; // reserved for DM_GETDEFID
    let _ = DS_MODALFRAME;

    ctx.logs.log(
        webwine_api::logs::LogLevel::Info,
        "dialog",
        &format!(
            "created dialog hwnd=0x{hwnd:08X} \"{}\" {}x{} items={}",
            tmpl.caption,
            width,
            height,
            tmpl.controls.len()
        ),
        Some(ctx.pid),
    );

    hwnd
}

pub fn get_dlg_item(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let id = ctx.arg(1) as i32;
    let child = ctx
        .gui
        .dlg_ctrl
        .get(&(hwnd, id))
        .copied()
        .or_else(|| {
            // Fallback: scan children
            ctx.gui.windows.get(&hwnd).and_then(|w| {
                w.children
                    .iter()
                    .find_map(|&ch| ctx.gui.windows.get(&ch).filter(|c| c.id == id).map(|_| ch))
            })
        })
        .unwrap_or(0);
    ctx.ret_stdcall(child, 2);
    Handled::Ok
}

pub fn set_dlg_item_text(ctx: &mut ApiContext, unicode: bool) -> Handled {
    let hwnd = ctx.arg(0);
    let id = ctx.arg(1) as i32;
    let text = if unicode {
        ctx.wstr(ctx.arg(2))
    } else {
        ctx.cstr(ctx.arg(2))
    };
    let child = ctx.gui.dlg_ctrl.get(&(hwnd, id)).copied().unwrap_or(0);
    if child != 0 {
        if let Some(w) = ctx.gui.windows.get_mut(&child) {
            w.title = text.clone();
        }
        ctx.ui_events.push(UiEvent::ControlText {
            hwnd,
            control_hwnd: child,
            text,
        });
        ctx.ret_stdcall(1, 3);
    } else {
        ctx.ret_stdcall(0, 3);
    }
    Handled::Ok
}

pub fn get_dlg_item_text(ctx: &mut ApiContext, unicode: bool) -> Handled {
    let hwnd = ctx.arg(0);
    let id = ctx.arg(1) as i32;
    let buf = ctx.arg(2);
    let max = ctx.arg(3) as usize;
    let child = ctx.gui.dlg_ctrl.get(&(hwnd, id)).copied().unwrap_or(0);
    let text = ctx
        .gui
        .windows
        .get(&child)
        .map(|w| w.title.clone())
        .unwrap_or_default();
    if buf == 0 || max == 0 {
        ctx.ret_stdcall(0, 4);
        return Handled::Ok;
    }
    if unicode {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let n = wide.len().min(max.saturating_sub(1));
        for (i, &ch) in wide.iter().take(n).enumerate() {
            let _ = ctx.memory.write_u16(buf + (i as u32) * 2, ch);
        }
        let _ = ctx.memory.write_u16(buf + (n as u32) * 2, 0);
        ctx.ret_stdcall(n as u32, 4);
    } else {
        let bytes = text.as_bytes();
        let n = bytes.len().min(max.saturating_sub(1));
        let _ = ctx.memory.write_bytes(buf, &bytes[..n]);
        let _ = ctx.memory.write_u8(buf + n as u32, 0);
        ctx.ret_stdcall(n as u32, 4);
    }
    Handled::Ok
}

pub fn send_dlg_item_message(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let id = ctx.arg(1) as i32;
    let msg = ctx.arg(2);
    let wparam = ctx.arg(3);
    let lparam = ctx.arg(4);
    let child = ctx.gui.dlg_ctrl.get(&(hwnd, id)).copied().unwrap_or(0);
    if child == 0 {
        ctx.ret_stdcall(0, 5);
        return Handled::Ok;
    }
    // Minimal handling for common control messages.
    let result = match msg {
        WM_SETTEXT => {
            let text = ctx.wstr(lparam);
            if let Some(w) = ctx.gui.windows.get_mut(&child) {
                w.title = text.clone();
            }
            ctx.ui_events.push(UiEvent::ControlText {
                hwnd,
                control_hwnd: child,
                text,
            });
            1
        }
        WM_GETTEXTLENGTH => ctx
            .gui
            .windows
            .get(&child)
            .map(|w| w.title.encode_utf16().count() as u32)
            .unwrap_or(0),
        WM_GETTEXT => {
            let max = wparam as usize;
            let text = ctx
                .gui
                .windows
                .get(&child)
                .map(|w| w.title.clone())
                .unwrap_or_default();
            if lparam != 0 && max > 0 {
                let wide: Vec<u16> = text.encode_utf16().collect();
                let n = wide.len().min(max.saturating_sub(1));
                for (i, &ch) in wide.iter().take(n).enumerate() {
                    let _ = ctx.memory.write_u16(lparam + (i as u32) * 2, ch);
                }
                let _ = ctx.memory.write_u16(lparam + (n as u32) * 2, 0);
                n as u32
            } else {
                0
            }
        }
        _ => {
            // Queue to child if it has a wndproc; else 0.
            if let Some(w) = ctx.gui.windows.get(&child) {
                if w.wndproc != 0 {
                    ctx.gui.queue.push_back(GuestMsg {
                        hwnd: child,
                        message: msg,
                        wparam,
                        lparam,
                    });
                }
            }
            0
        }
    };
    ctx.ret_stdcall(result, 5);
    Handled::Ok
}

pub fn check_dlg_button(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let id = ctx.arg(1) as i32;
    let check = ctx.arg(2); // BST_UNCHECKED=0, BST_CHECKED=1, BST_INDETERMINATE=2
    let child = ctx.gui.dlg_ctrl.get(&(hwnd, id)).copied().unwrap_or(0);
    if child != 0 {
        if let Some(w) = ctx.gui.windows.get_mut(&child) {
            w.checked = check != 0;
        }
        // Refresh layout text/state via control text event is enough for now.
        ctx.ret_stdcall(1, 3);
    } else {
        ctx.ret_stdcall(0, 3);
    }
    Handled::Ok
}

pub fn is_dlg_button_checked(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let id = ctx.arg(1) as i32;
    let child = ctx.gui.dlg_ctrl.get(&(hwnd, id)).copied().unwrap_or(0);
    let checked = ctx
        .gui
        .windows
        .get(&child)
        .map(|w| w.checked as u32)
        .unwrap_or(0);
    ctx.ret_stdcall(checked, 2);
    Handled::Ok
}

pub fn end_dialog(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let result = ctx.arg(1) as i32;
    let children = if let Some(w) = ctx.gui.windows.get_mut(&hwnd) {
        w.dlg_result = result;
        w.visible = false;
        let c = w.children.clone();
        w.children.clear();
        c
    } else {
        Vec::new()
    };

    for ch in children {
        ctx.gui.windows.remove(&ch);
        // purge dlg_ctrl entries
        ctx.gui.dlg_ctrl.retain(|&(p, _), _| p != hwnd);
    }
    
    ctx.ui_events.push(UiEvent::DestroyWindow { hwnd });
    ctx.gui.windows.remove(&hwnd);
    ctx.gui.dialog_pending = false;
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

pub fn get_dialog_base_units(ctx: &mut ApiContext) -> Handled {
    let v = DEFAULT_X_BASE | (DEFAULT_Y_BASE << 16);
    ctx.ret_stdcall(v, 0);
    Handled::Ok
}

pub fn map_dialog_rect(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let rect = ctx.arg(1);
    let (xb, yb) = ctx
        .gui
        .windows
        .get(&hwnd)
        .map(|w| (w.x_base_unit, w.y_base_unit))
        .unwrap_or((DEFAULT_X_BASE, DEFAULT_Y_BASE));
    if rect != 0 {
        let left = ctx.memory.read_u32(rect).unwrap_or(0) as i32;
        let top = ctx.memory.read_u32(rect + 4).unwrap_or(0) as i32;
        let right = ctx.memory.read_u32(rect + 8).unwrap_or(0) as i32;
        let bottom = ctx.memory.read_u32(rect + 12).unwrap_or(0) as i32;
        let _ = ctx
            .memory
            .write_u32(rect, mul_div(left, xb as i32, 4) as u32);
        let _ = ctx
            .memory
            .write_u32(rect + 4, mul_div(top, yb as i32, 8) as u32);
        let _ = ctx
            .memory
            .write_u32(rect + 8, mul_div(right, xb as i32, 4) as u32);
        let _ = ctx
            .memory
            .write_u32(rect + 12, mul_div(bottom, yb as i32, 8) as u32);
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

/// IsDialogMessage — simplified Wine path for Tab / Enter / Esc.
pub fn is_dialog_message(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let msg_ptr = ctx.arg(1);
    if msg_ptr == 0
        || !ctx
            .gui
            .windows
            .get(&hwnd)
            .map(|w| w.is_dialog)
            .unwrap_or(false)
    {
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    }
    let message = ctx.memory.read_u32(msg_ptr + 4).unwrap_or(0);
    let wparam = ctx.memory.read_u32(msg_ptr + 8).unwrap_or(0);
    // WM_KEYDOWN = 0x0100
    const WM_KEYDOWN: u32 = 0x0100;
    const VK_TAB: u32 = 0x09;
    const VK_RETURN: u32 = 0x0D;
    const VK_ESCAPE: u32 = 0x1B;

    if message == WM_KEYDOWN {
        match wparam {
            VK_ESCAPE => {
                ctx.gui.queue.push_back(GuestMsg {
                    hwnd,
                    message: WM_COMMAND,
                    wparam: IDCANCEL as u32,
                    lparam: ctx
                        .gui
                        .dlg_ctrl
                        .get(&(hwnd, IDCANCEL))
                        .copied()
                        .unwrap_or(0),
                });
                ctx.ret_stdcall(1, 2);
                return Handled::Ok;
            }
            VK_RETURN => {
                ctx.gui.queue.push_back(GuestMsg {
                    hwnd,
                    message: WM_COMMAND,
                    wparam: IDOK as u32,
                    lparam: ctx.gui.dlg_ctrl.get(&(hwnd, IDOK)).copied().unwrap_or(0),
                });
                ctx.ret_stdcall(1, 2);
                return Handled::Ok;
            }
            VK_TAB => {
                // Cycle tabstops — best effort
                let _ = next_dlg_tab_item(ctx, hwnd, 0, false);
                ctx.ret_stdcall(1, 2);
                return Handled::Ok;
            }
            _ => {}
        }
    }
    ctx.ret_stdcall(0, 2);
    Handled::Ok
}

fn next_dlg_tab_item(ctx: &mut ApiContext, hwnd: u32, _start: u32, _prev: bool) -> u32 {
    let Some(w) = ctx.gui.windows.get(&hwnd) else {
        return 0;
    };
    for &ch in &w.children {
        if let Some(c) = ctx.gui.windows.get(&ch) {
            if c.style & WS_TABSTOP != 0 && c.visible && c.enabled {
                return ch;
            }
        }
    }
    0
}

pub fn get_next_dlg_tab_item(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let ctrl = ctx.arg(1);
    let prev = ctx.arg(2) != 0;
    let r = next_dlg_tab_item(ctx, hwnd, ctrl, prev);
    ctx.ret_stdcall(r, 3);
    Handled::Ok
}

pub fn get_next_dlg_group_item(ctx: &mut ApiContext) -> Handled {
    // Same as tab for now
    get_next_dlg_tab_item(ctx)
}

/// DefDlgProc — minimal: forward to DefWindowProc-ish (return 0).
pub fn def_dlg_proc(ctx: &mut ApiContext) -> Handled {
    let hwnd = ctx.arg(0);
    let msg = ctx.arg(1);
    let _wparam = ctx.arg(2);
    let _lparam = ctx.arg(3);
    // DM_GETDEFID = WM_USER+0 = 0x400
    const DM_GETDEFID: u32 = 0x400;
    const DM_SETDEFID: u32 = 0x401;
    const DC_HASDEFID: u32 = 0x534B;
    match msg {
        DM_GETDEFID => {
            // Return MAKELONG(id, DC_HASDEFID) for first default button or IDOK
            let id = ctx
                .gui
                .windows
                .get(&hwnd)
                .and_then(|w| {
                    w.children.iter().find_map(|&ch| {
                        ctx.gui.windows.get(&ch).and_then(|c| {
                            if c.class_name.eq_ignore_ascii_case("Button")
                                && c.style & BS_TYPEMASK == BS_DEFPUSHBUTTON
                            {
                                Some(c.id as u32)
                            } else {
                                None
                            }
                        })
                    })
                })
                .or_else(|| ctx.gui.dlg_ctrl.get(&(hwnd, IDOK)).map(|_| IDOK as u32))
                .unwrap_or(0);
            let r = id | (DC_HASDEFID << 16);
            ctx.ret_stdcall(r, 4);
        }
        DM_SETDEFID => {
            ctx.ret_stdcall(1, 4);
        }
        WM_CLOSE => {
            // Post IDCANCEL
            ctx.gui.queue.push_back(GuestMsg {
                hwnd,
                message: WM_COMMAND,
                wparam: IDCANCEL as u32,
                lparam: 0,
            });
            ctx.ret_stdcall(0, 4);
        }
        _ => {
            ctx.ret_stdcall(0, 4);
        }
    }
    Handled::Ok
}

/// Host click on a dialog control → WM_COMMAND to the dialog.
pub fn post_control_command(ctx: &mut ApiContext, dlg: u32, ctrl: u32) {
    let id = ctx.gui.windows.get(&ctrl).map(|w| w.id as u32).unwrap_or(0);
    // MAKEWPARAM(id, BN_CLICKED)
    let wparam = id | (BN_CLICKED << 16);
    ctx.gui.queue.push_back(GuestMsg {
        hwnd: dlg,
        message: WM_COMMAND,
        wparam,
        lparam: ctrl,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal DLGTEMPLATE: empty menu/class, caption "Hi", one static control.
    fn sample_template() -> Vec<u8> {
        let mut v = Vec::new();
        // style = DS_SETFONT|WS_POPUP|WS_CAPTION|WS_SYSMENU (simplified)
        let style: u32 = 0x80C8_0040; // includes DS_SETFONT
        v.extend_from_slice(&style.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes()); // exStyle
        v.extend_from_slice(&1u16.to_le_bytes()); // cdit = 1
        v.extend_from_slice(&0i16.to_le_bytes()); // x
        v.extend_from_slice(&0i16.to_le_bytes()); // y
        v.extend_from_slice(&100i16.to_le_bytes()); // cx
        v.extend_from_slice(&50i16.to_le_bytes()); // cy
        v.extend_from_slice(&0u16.to_le_bytes()); // menu
        v.extend_from_slice(&0u16.to_le_bytes()); // class
                                                  // caption "Hi\0"
        for ch in "Hi\0".encode_utf16() {
            v.extend_from_slice(&ch.to_le_bytes());
        }
        // DS_SETFONT: pointsize 8, face "MS Shell Dlg\0"
        v.extend_from_slice(&8u16.to_le_bytes());
        for ch in "MS Shell Dlg\0".encode_utf16() {
            v.extend_from_slice(&ch.to_le_bytes());
        }
        // align dword
        while v.len() % 4 != 0 {
            v.push(0);
        }
        // Control: Static "Hello" id=100
        let cstyle: u32 = WS_CHILD | WS_VISIBLE | SS_LEFT;
        v.extend_from_slice(&cstyle.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes()); // ex
        v.extend_from_slice(&10i16.to_le_bytes());
        v.extend_from_slice(&10i16.to_le_bytes());
        v.extend_from_slice(&80i16.to_le_bytes());
        v.extend_from_slice(&12i16.to_le_bytes());
        v.extend_from_slice(&100u16.to_le_bytes()); // id
        v.extend_from_slice(&0xffffu16.to_le_bytes()); // class atom
        v.extend_from_slice(&0x0082u16.to_le_bytes()); // Static
        for ch in "Hello\0".encode_utf16() {
            v.extend_from_slice(&ch.to_le_bytes());
        }
        v.extend_from_slice(&0u16.to_le_bytes()); // no creation data
        while v.len() % 4 != 0 {
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_classic_dialog_with_static() {
        let t = parse_dialog_template(&sample_template()).expect("parse");
        assert_eq!(t.caption, "Hi");
        assert_eq!(t.controls.len(), 1);
        assert_eq!(t.controls[0].class_name, "Static");
        assert_eq!(t.controls[0].window_name, "Hello");
        assert_eq!(t.controls[0].id, 100);
    }
}
