use super::WinApiRegistry;
use crate::util::{
    register_entries, ret_0_2, ret_0_4, ret_0_5, ret_0_6, ret_0_8, ret_1_1, ret_1_4, Entry,
};

pub fn register(r: &mut WinApiRegistry) {
    register_entries(r, ENTRIES);
}

const ENTRIES: &[Entry] = &[
    ("wininet.dll", "InternetOpenA", ret_0_5),
    ("wininet.dll", "InternetOpenW", ret_0_5),
    ("wininet.dll", "InternetCloseHandle", ret_1_1),
    ("wininet.dll", "InternetOpenUrlA", ret_0_6),
    ("wininet.dll", "InternetOpenUrlW", ret_0_6),
    ("wininet.dll", "InternetConnectA", ret_0_8),
    ("wininet.dll", "InternetConnectW", ret_0_8),
    ("wininet.dll", "InternetReadFile", ret_0_4),
    ("wininet.dll", "InternetSetOptionA", ret_1_4),
    ("wininet.dll", "HttpOpenRequestA", ret_0_8),
    ("wininet.dll", "HttpSendRequestA", ret_0_5),
    ("wininet.dll", "HttpQueryInfoA", ret_0_5),
    ("wininet.dll", "InternetGetConnectedState", ret_0_2),
];
