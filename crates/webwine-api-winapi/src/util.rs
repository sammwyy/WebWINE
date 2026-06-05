use super::{ApiContext, Handled, HandlerFn, WinApiRegistry};

pub type Entry = (&'static str, &'static str, HandlerFn);

pub fn register_entries(r: &mut WinApiRegistry, entries: &[Entry]) {
    for &(dll, name, handler) in entries {
        r.add(dll, name, handler);
    }
}

pub fn ret_0_0(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 0);
    Handled::Ok
}

pub fn ret_0_1(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1);
    Handled::Ok
}

pub fn ret_0_2(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

pub fn ret_0_3(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 3);
    Handled::Ok
}

pub fn ret_0_4(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 4);
    Handled::Ok
}

pub fn ret_0_5(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 5);
    Handled::Ok
}

pub fn ret_0_6(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 6);
    Handled::Ok
}

pub fn ret_0_7(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 7);
    Handled::Ok
}

pub fn ret_0_8(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 8);
    Handled::Ok
}

pub fn ret_0_9(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 9);
    Handled::Ok
}

pub fn ret_1_1(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 1);
    Handled::Ok
}

pub fn ret_1_3(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 3);
    Handled::Ok
}

pub fn ret_1_4(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 4);
    Handled::Ok
}

pub fn ret_1_5(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(1, 5);
    Handled::Ok
}
