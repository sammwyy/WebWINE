use super::WinApiRegistry;
use crate::util::{
    register_entries, ret_0_0, ret_0_1, ret_0_2, ret_0_4, ret_0_7, ret_0_9, ret_1_1, ret_1_3,
    Entry,
};

pub fn register(r: &mut WinApiRegistry) {
    register_entries(r, ENTRIES);
}

const ENTRIES: &[Entry] = &[
    ("dbghelp.dll", "SymInitialize", ret_1_3),
    ("dbghelp.dll", "SymCleanup", ret_1_1),
    ("dbghelp.dll", "SymGetOptions", ret_0_0),
    ("dbghelp.dll", "SymSetOptions", ret_0_1),
    ("dbghelp.dll", "SymGetSymFromAddr", ret_0_4),
    ("dbghelp.dll", "SymFromAddr", ret_0_4),
    ("dbghelp.dll", "SymGetLineFromAddr64", ret_0_4),
    ("dbghelp.dll", "StackWalk64", ret_0_9),
    ("dbghelp.dll", "SymFunctionTableAccess64", ret_0_2),
    ("dbghelp.dll", "SymGetModuleBase64", ret_0_2),
    ("dbghelp.dll", "MiniDumpWriteDump", ret_0_7),
    ("dbghelp.dll", "ImageNtHeader", ret_0_1),
];
