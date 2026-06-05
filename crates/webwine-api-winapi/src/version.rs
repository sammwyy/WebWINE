use super::{ApiContext, Handled, WinApiRegistry};
use crate::util::{register_entries, ret_0_3, ret_0_4, ret_0_5, Entry};
use webwine_api::winapi::context::ApiRuntimeEnv;

pub fn register(r: &mut WinApiRegistry) {
    register_entries(r, ENTRIES);
}

const ENTRIES: &[Entry] = &[
    ("version.dll", "GetFileVersionInfoSizeA", get_file_version_info_size),
    ("version.dll", "GetFileVersionInfoSizeW", get_file_version_info_size),
    ("version.dll", "GetFileVersionInfoSizeExW", get_file_version_info_size_ex),
    ("version.dll", "GetFileVersionInfoSizeExA", get_file_version_info_size_ex),
    ("version.dll", "GetFileVersionInfoA", ret_0_4),
    ("version.dll", "GetFileVersionInfoW", ret_0_4),
    ("version.dll", "GetFileVersionInfoExW", ret_0_5),
    ("version.dll", "GetFileVersionInfoExA", ret_0_5),
    ("version.dll", "VerQueryValueA", ret_0_4),
    ("version.dll", "VerQueryValueW", ret_0_4),
    ("version.dll", "VerLanguageNameA", ret_0_3),
    ("version.dll", "VerLanguageNameW", ret_0_3),
];

fn get_file_version_info_size(c: &mut ApiContext) -> Handled {
    write_zero_handle(c, 1);
    c.return_stdcall(0, 2);
    Handled::Ok
}

fn get_file_version_info_size_ex(c: &mut ApiContext) -> Handled {
    write_zero_handle(c, 2);
    c.return_stdcall(0, 3);
    Handled::Ok
}

fn write_zero_handle(c: &mut impl ApiRuntimeEnv, arg_index: u32) {
    let handle_ptr = c.arg(arg_index);
    if handle_ptr != 0 {
        c.write_u32(handle_ptr, 0);
    }
}
