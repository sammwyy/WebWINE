use super::WinApiRegistry;
use crate::util::{register_entries, ret_0_0, ret_0_1, Entry};

pub fn register(r: &mut WinApiRegistry) {
    register_entries(r, ENTRIES);
}

const ENTRIES: &[Entry] = &[
    ("comdlg32.dll", "GetOpenFileNameA", ret_0_1),
    ("comdlg32.dll", "GetOpenFileNameW", ret_0_1),
    ("comdlg32.dll", "GetSaveFileNameA", ret_0_1),
    ("comdlg32.dll", "GetSaveFileNameW", ret_0_1),
    ("comdlg32.dll", "ChooseColorA", ret_0_1),
    ("comdlg32.dll", "ChooseColorW", ret_0_1),
    ("comdlg32.dll", "ChooseFontA", ret_0_1),
    ("comdlg32.dll", "ChooseFontW", ret_0_1),
    ("comdlg32.dll", "PrintDlgA", ret_0_1),
    ("comdlg32.dll", "PrintDlgW", ret_0_1),
    ("comdlg32.dll", "CommDlgExtendedError", ret_0_0),
];
