use super::{ApiContext, Handled, WinApiRegistry};
use crate::vm::handles::{
    KernelObject, CURRENT_PROCESS, CURRENT_THREAD, INVALID_HANDLE, STD_ERROR_HANDLE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};

// Win32 error codes used by the file APIs.
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_FILE_EXISTS: u32 = 80;

const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const INVALID_FILE_ATTRIBUTES: u32 = 0xFFFF_FFFF;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        // advapi32 registry: we have no registry, so report "key not found" and
        // clean the exact arg counts (a wrong count corrupts the guest stack).
        // ERROR_FILE_NOT_FOUND (2) makes apps fall back to defaults.
        ("advapi32.dll", "RegOpenKeyExA", reg_open_key),
        ("advapi32.dll", "RegOpenKeyExW", reg_open_key),
        ("advapi32.dll", "RegOpenKeyA", |c| { let o = c.arg(2); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 3); Handled::Ok }),
        ("advapi32.dll", "RegQueryValueExA", reg_query_value),
        ("advapi32.dll", "RegQueryValueExW", |c| { c.ret_stdcall(2, 6); Handled::Ok }),
        ("advapi32.dll", "RegCreateKeyA", |c| { let o = c.arg(2); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 3); Handled::Ok }),
        ("advapi32.dll", "RegCreateKeyW", |c| { let o = c.arg(2); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 3); Handled::Ok }),
        ("advapi32.dll", "RegCreateKeyExA", |c| { let o = c.arg(7); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 9); Handled::Ok }),
        ("advapi32.dll", "RegSetValueExA", |c| { c.ret_stdcall(0, 6); Handled::Ok }),
        ("advapi32.dll", "RegSetValueExW", |c| { c.ret_stdcall(0, 6); Handled::Ok }),
        ("advapi32.dll", "RegCloseKey", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("advapi32.dll", "RegOpenKeyW", |c| { let o = c.arg(2); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 3); Handled::Ok }),
        ("advapi32.dll", "RegCreateKeyExW", |c| { let o = c.arg(7); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(2, 9); Handled::Ok }),
        ("advapi32.dll", "RegDeleteValueW", |c| { c.ret_stdcall(2, 2); Handled::Ok }),
        ("advapi32.dll", "RegDeleteKeyW", |c| { c.ret_stdcall(2, 2); Handled::Ok }),
        ("advapi32.dll", "RegEnumValueW", |c| { c.ret_stdcall(0x103, 8); Handled::Ok }),  // ERROR_NO_MORE_ITEMS
        ("advapi32.dll", "RegEnumKeyExW", |c| { c.ret_stdcall(0x103, 8); Handled::Ok }),
        ("advapi32.dll", "RegQueryInfoKeyW", |c| { c.ret_stdcall(0, 12); Handled::Ok }),
        ("advapi32.dll", "RegQueryValueW", |c| { c.ret_stdcall(2, 4); Handled::Ok }),
        ("advapi32.dll", "RegNotifyChangeKeyValue", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("advapi32.dll", "RegFlushKey", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "SHGetValueA", |c| { c.ret_stdcall(2, 6); Handled::Ok }),
        ("shlwapi.dll", "SHGetValueW", |c| { c.ret_stdcall(2, 6); Handled::Ok }),
        ("shlwapi.dll", "SHSetValueA", |c| { c.ret_stdcall(0, 6); Handled::Ok }),
        ("shlwapi.dll", "SHSetValueW", |c| { c.ret_stdcall(0, 6); Handled::Ok }),
        ("shlwapi.dll", "SHRegGetBoolUSValueA", sh_reg_get_bool_us_value),
        ("shlwapi.dll", "SHRegGetBoolUSValueW", sh_reg_get_bool_us_value),
        ("shlwapi.dll", "SHRegGetUSValueA", |c| { c.ret_stdcall(2, 8); Handled::Ok }),
        ("shlwapi.dll", "SHRegGetUSValueW", |c| { c.ret_stdcall(2, 8); Handled::Ok }),
        ("shlwapi.dll", "SHRegCreateUSKeyA", sh_reg_create_us_key),
        ("shlwapi.dll", "SHRegCreateUSKeyW", sh_reg_create_us_key),
        ("shlwapi.dll", "SHRegWriteUSValueA", |c| { c.ret_stdcall(0, 7); Handled::Ok }),
        ("shlwapi.dll", "SHRegWriteUSValueW", |c| { c.ret_stdcall(0, 7); Handled::Ok }),
        ("shlwapi.dll", "SHRegCloseUSKey", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "PathFindFileNameA", path_find_file_name_a),
        ("shlwapi.dll", "PathFindFileNameW", path_find_file_name_w),
        ("shlwapi.dll", "PathRemoveArgsA", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "PathRemoveArgsW", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "PathRemoveBlanksA", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "PathRemoveBlanksW", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "StrCmpNIA", strcmp_ni_a),
        ("shlwapi.dll", "StrCmpNIW", strcmp_ni_w),
        ("shlwapi.dll", "#241", |c| { c.ret_stdcall(0, 6); Handled::Ok }),
        ("shlwapi.dll", "#433", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("shlwapi.dll", "#437", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "#563", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("shlwapi.dll", "#618", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "#16", |c| { c.ret_stdcall(0, 4); Handled::Ok }),
        ("shlwapi.dll", "SHCreateThreadRef", sh_create_thread_ref),
        ("shlwapi.dll", "SHSetThreadRef", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("shlwapi.dll", "SHGetThreadRef", |c| { let out = c.arg(0); if out != 0 { let _ = c.memory.write_u32(out, 0); } c.ret_stdcall(0x8000_4005, 1); Handled::Ok }),
        ("shlwapi.dll", "SHReleaseThreadRef", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("comctl32.dll", "InitCommonControlsEx", |c| { c.ret_stdcall(1, 1); Handled::Ok }),

        ("kernel32.dll", "ExitProcess", exit_process),
        ("kernel32.dll", "GetStdHandle", get_std_handle),
        ("kernel32.dll", "WriteFile", write_file),
        ("kernel32.dll", "WriteConsoleA", write_console_a),
        ("kernel32.dll", "WriteConsoleW", write_console_w),
        ("kernel32.dll", "ReadFile", read_file),
        ("kernel32.dll", "CloseHandle", close_handle),
        ("kernel32.dll", "GetLastError", get_last_error),
        ("kernel32.dll", "SetLastError", set_last_error),
        ("kernel32.dll", "GetProcessHeap", get_process_heap),
        ("kernel32.dll", "HeapAlloc", heap_alloc),
        ("kernel32.dll", "HeapFree", heap_free),
        ("kernel32.dll", "HeapReAlloc", heap_realloc),
        ("kernel32.dll", "HeapSize", heap_size),
        ("kernel32.dll", "HeapCreate", heap_create),
        ("kernel32.dll", "HeapDestroy", r1_1),
        ("kernel32.dll", "VirtualAlloc", virtual_alloc),
        ("kernel32.dll", "VirtualFree", r1_3),
        ("kernel32.dll", "VirtualProtect", virtual_protect),
        ("kernel32.dll", "VirtualQuery", r0_3),
        ("kernel32.dll", "GetModuleHandleA", get_module_handle_a),
        ("kernel32.dll", "GetModuleHandleW", get_module_handle_w),
        ("kernel32.dll", "GetModuleFileNameA", get_module_filename_a),
        ("kernel32.dll", "GetModuleFileNameW", get_module_filename_w),
        ("kernel32.dll", "SetCurrentDirectoryA", set_current_directory_a),
        ("kernel32.dll", "SetCurrentDirectoryW", set_current_directory_w),
        ("kernel32.dll", "GetProcAddress", get_proc_address),
        ("kernel32.dll", "SetErrorMode", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "lstrlenA", |c| { let n = c.cstr(c.arg(0)).len() as u32; c.ret_stdcall(n, 1); Handled::Ok }),
        ("kernel32.dll", "lstrlenW", |c| { let n = c.wstr(c.arg(0)).encode_utf16().count() as u32; c.ret_stdcall(n, 1); Handled::Ok }),
        ("kernel32.dll", "GetPrivateProfileStringA", get_private_profile_string_a),
        ("kernel32.dll", "GetPrivateProfileStringW", get_private_profile_string_w),
        ("kernel32.dll", "SetProcessShutdownParameters", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        // UI language / locale (cmd.exe resolves these via GetProcAddress). 0x409 = en-US.
        ("kernel32.dll", "SetThreadUILanguage", |c| { let l = c.arg(0); c.ret_stdcall(if l == 0 { 0x409 } else { l }, 1); Handled::Ok }),
        ("kernel32.dll", "GetThreadUILanguage", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetUserDefaultUILanguage", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetSystemDefaultUILanguage", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetUserDefaultLangID", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetSystemDefaultLangID", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetUserDefaultLCID", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetSystemDefaultLCID", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "GetLocaleInfoA", r0_4),
        ("kernel32.dll", "GetLocaleInfoW", r0_4),
        ("kernel32.dll", "GetThreadLocale", |c| { c.ret_stdcall(0x409, 0); Handled::Ok }),
        ("kernel32.dll", "SetThreadLocale", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        // Report Windows XP (5.1 build 2600). GetVersion: (build<<16)|(minor<<8)|major.
        ("kernel32.dll", "GetVersion", |c| { c.ret_stdcall(0x0A28_0105, 0); Handled::Ok }),
        ("kernel32.dll", "GetVersionExA", get_version_ex),
        ("kernel32.dll", "GetVersionExW", get_version_ex),
        ("kernel32.dll", "LoadLibraryA", load_library_a),
        ("kernel32.dll", "LoadLibraryW", load_library_w),
        ("kernel32.dll", "LoadLibraryExA", |c| { c.ret_stdcall(FAKE_MODULE, 3); Handled::Ok }),
        ("kernel32.dll", "LoadLibraryExW", |c| { c.ret_stdcall(FAKE_MODULE, 3); Handled::Ok }),
        ("kernel32.dll", "FreeLibrary", r1_1),
        ("kernel32.dll", "IsDebuggerPresent", r0_0),
        ("kernel32.dll", "IsProcessorFeaturePresent", r1_1),
        ("kernel32.dll", "InitializeSListHead", r0_1),
        ("kernel32.dll", "QueryDepthSList", r0_1),
        ("kernel32.dll", "InterlockedPushEntrySList", r0_2),
        ("kernel32.dll", "InterlockedFlushSList", r0_1),
        ("kernel32.dll", "GetProcessAffinityMask", r1_3),
        ("kernel32.dll", "GetNativeSystemInfo", r0_1),
        (
            "kernel32.dll",
            "GetCurrentDirectoryW",
            get_current_directory_w,
        ),
        ("kernel32.dll", "SetThreadErrorMode", r1_2),
        ("kernel32.dll", "GetThreadPriority", r0_1),
        ("kernel32.dll", "AddVectoredExceptionHandler", r1_2),
        ("kernel32.dll", "RemoveVectoredExceptionHandler", r1_1),
        ("kernel32.dll", "SetThreadStackGuarantee", r1_1),
        ("kernel32.dll", "GetModuleHandleExW", get_module_handle_ex),
        ("kernel32.dll", "GetModuleHandleExA", get_module_handle_ex),
        (
            "kernel32.dll",
            "GetSystemTimePreciseAsFileTime",
            get_system_time,
        ),
        ("kernel32.dll", "InitOnceExecuteOnce", r1_4),
        ("kernel32.dll", "AcquireSRWLockExclusive", r0_1),
        ("kernel32.dll", "ReleaseSRWLockExclusive", r0_1),
        ("kernel32.dll", "AcquireSRWLockShared", r0_1),
        ("kernel32.dll", "ReleaseSRWLockShared", r0_1),
        ("kernel32.dll", "TryAcquireSRWLockExclusive", r1_1),
        ("kernel32.dll", "InitializeSRWLock", r0_1),
        ("kernel32.dll", "FlsAlloc", fls_alloc),
        ("kernel32.dll", "FlsSetValue", tls_set),
        ("kernel32.dll", "FlsGetValue", tls_get),
        ("kernel32.dll", "FlsFree", r1_1),
        ("kernel32.dll", "GetProcessHeaps", r0_2),
        ("kernel32.dll", "GetCommandLineA", get_command_line_a),
        ("kernel32.dll", "GetCommandLineW", get_command_line_w),
        ("kernel32.dll", "GetStartupInfoA", get_startup_info),
        ("kernel32.dll", "GetStartupInfoW", get_startup_info),
        (
            "kernel32.dll",
            "GetCurrentProcessId",
            get_current_process_id,
        ),
        ("kernel32.dll", "GetCurrentThreadId", get_current_thread_id),
        ("kernel32.dll", "GetCurrentProcess", get_current_process),
        ("kernel32.dll", "GetCurrentThread", get_current_thread),
        ("kernel32.dll", "GetSystemInfo", r0_1),
        ("kernel32.dll", "GetSystemTimeAsFileTime", get_system_time),
        (
            "kernel32.dll",
            "QueryPerformanceCounter",
            query_perf_counter,
        ),
        ("kernel32.dll", "QueryPerformanceFrequency", query_perf_freq),
        ("kernel32.dll", "GetTickCount", get_tick_count),
        ("kernel32.dll", "GetTickCount64", get_tick_count),
        ("kernel32.dll", "FlushFileBuffers", r1_1),
        ("kernel32.dll", "SetFilePointer", r0_4),
        ("kernel32.dll", "SetUnhandledExceptionFilter", r0_1),
        ("kernel32.dll", "UnhandledExceptionFilter", r0_1),
        ("kernel32.dll", "GetEnvironmentVariableW", get_env_var_w),
        ("kernel32.dll", "GetEnvironmentVariableA", get_env_var_a),
        ("kernel32.dll", "AreFileApisANSI", |c| { c.ret_stdcall(1, 0); Handled::Ok }),
        // Secure DLL search-path APIs (putty et al. resolve these dynamically).
        ("kernel32.dll", "SetDefaultDllDirectories", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("kernel32.dll", "AddDllDirectory", |c| { c.ret_stdcall(0x44_0001, 1); Handled::Ok }),
        ("kernel32.dll", "RemoveDllDirectory", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("kernel32.dll", "SetDllDirectoryW", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("kernel32.dll", "SetDllDirectoryA", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("kernel32.dll", "SetSearchPathMode", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        // Winsock (ws2_32): present so apps like putty init networking and reach
        // their UI. Socket ops fail (no real network); init + byte-swap work.
        ("ws2_32.dll", "WSAStartup", |c| { let d = c.arg(1); if d != 0 { let _ = c.memory.write_u16(d, 0x0202); let _ = c.memory.write_u16(d + 2, 0x0202); } c.ret_stdcall(0, 2); Handled::Ok }),
        ("ws2_32.dll", "WSACleanup", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("ws2_32.dll", "WSAGetLastError", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("ws2_32.dll", "WSASetLastError", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("ws2_32.dll", "WSACreateEvent", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("ws2_32.dll", "WSACloseEvent", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("ws2_32.dll", "WSAAsyncSelect", |c| { c.ret_stdcall(0, 4); Handled::Ok }),
        ("ws2_32.dll", "WSAEventSelect", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("ws2_32.dll", "WSAIoctl", |c| { c.ret_stdcall(0xFFFF_FFFF, 9); Handled::Ok }),
        ("ws2_32.dll", "WSAAddressToStringA", |c| { c.ret_stdcall(0xFFFF_FFFF, 5); Handled::Ok }),
        ("ws2_32.dll", "WSAStringToAddressA", |c| { c.ret_stdcall(0xFFFF_FFFF, 5); Handled::Ok }),
        ("ws2_32.dll", "socket", |c| { c.ret_stdcall(0xFFFF_FFFF, 3); Handled::Ok }),       // INVALID_SOCKET
        ("ws2_32.dll", "WSASocketA", |c| { c.ret_stdcall(0xFFFF_FFFF, 6); Handled::Ok }),
        ("ws2_32.dll", "closesocket", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("ws2_32.dll", "connect", |c| { c.ret_stdcall(0xFFFF_FFFF, 3); Handled::Ok }),
        ("ws2_32.dll", "bind", |c| { c.ret_stdcall(0xFFFF_FFFF, 3); Handled::Ok }),
        ("ws2_32.dll", "listen", |c| { c.ret_stdcall(0xFFFF_FFFF, 2); Handled::Ok }),
        ("ws2_32.dll", "accept", |c| { c.ret_stdcall(0xFFFF_FFFF, 3); Handled::Ok }),
        ("ws2_32.dll", "send", |c| { c.ret_stdcall(0xFFFF_FFFF, 4); Handled::Ok }),
        ("ws2_32.dll", "recv", |c| { c.ret_stdcall(0xFFFF_FFFF, 4); Handled::Ok }),
        ("ws2_32.dll", "sendto", |c| { c.ret_stdcall(0xFFFF_FFFF, 6); Handled::Ok }),
        ("ws2_32.dll", "recvfrom", |c| { c.ret_stdcall(0xFFFF_FFFF, 6); Handled::Ok }),
        ("ws2_32.dll", "shutdown", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("ws2_32.dll", "select", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("ws2_32.dll", "ioctlsocket", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("ws2_32.dll", "getsockname", |c| { c.ret_stdcall(0xFFFF_FFFF, 3); Handled::Ok }),
        ("ws2_32.dll", "getpeername", |c| { c.ret_stdcall(0xFFFF_FFFF, 3); Handled::Ok }),
        ("ws2_32.dll", "getsockopt", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("ws2_32.dll", "setsockopt", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("ws2_32.dll", "gethostname", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("ws2_32.dll", "gethostbyname", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("ws2_32.dll", "getaddrinfo", |c| { c.ret_stdcall(0xFFFF_FFFF, 4); Handled::Ok }),
        ("ws2_32.dll", "freeaddrinfo", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("ws2_32.dll", "getnameinfo", |c| { c.ret_stdcall(0xFFFF_FFFF, 7); Handled::Ok }),
        ("ws2_32.dll", "inet_addr", |c| { c.ret_stdcall(0xFFFF_FFFF, 1); Handled::Ok }),
        ("ws2_32.dll", "inet_ntoa", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("ws2_32.dll", "htons", |c| { let v = c.arg(0) as u16; c.ret_stdcall(v.swap_bytes() as u32, 1); Handled::Ok }),
        ("ws2_32.dll", "ntohs", |c| { let v = c.arg(0) as u16; c.ret_stdcall(v.swap_bytes() as u32, 1); Handled::Ok }),
        ("ws2_32.dll", "htonl", |c| { let v = c.arg(0); c.ret_stdcall(v.swap_bytes(), 1); Handled::Ok }),
        ("ws2_32.dll", "ntohl", |c| { let v = c.arg(0); c.ret_stdcall(v.swap_bytes(), 1); Handled::Ok }),
        ("ws2_32.dll", "__WSAFDIsSet", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("ws2_32.dll", "WSAWaitForMultipleEvents", |c| { c.ret_stdcall(0xFFFF_FFFF, 5); Handled::Ok }),
        ("ws2_32.dll", "WSAEnumNetworkEvents", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("ws2_32.dll", "WSAGetOverlappedResult", |c| { c.ret_stdcall(0, 5); Handled::Ok }),
        ("ws2_32.dll", "getservbyname", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("ws2_32.dll", "getservbyport", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("ws2_32.dll", "inet_ntop", |c| { c.ret_stdcall(0, 4); Handled::Ok }),
        ("ws2_32.dll", "inet_pton", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        // comctl32 — common controls (putty's config dialog uses drag lists etc.)
        ("comctl32.dll", "InitCommonControls", |c| { c.ret_stdcall(0, 0); Handled::Ok }),

        ("comctl32.dll", "DrawInsert", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("comctl32.dll", "LBItemFromPt", |c| { c.ret_stdcall(0xFFFF_FFFF, 4); Handled::Ok }),
        ("comctl32.dll", "MakeDragList", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("comctl32.dll", "ImageList_Create", |c| { c.ret_stdcall(0x494C_0001, 5); Handled::Ok }),
        ("comctl32.dll", "ImageList_Destroy", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("comctl32.dll", "ImageList_AddMasked", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("comctl32.dll", "ImageList_ReplaceIcon", |c| { c.ret_stdcall(0, 3); Handled::Ok }),
        ("comctl32.dll", "_TrackMouseEvent", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("comctl32.dll", "CreateUpDownControl", |c| { c.ret_stdcall(0, 12); Handled::Ok }),
        ("comctl32.dll", "PropertySheetW", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("comctl32.dll", "PropertySheetA", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        // GetSystemDirectory/GetWindowsDirectory(lpBuffer, uSize) — 2 args.
        ("kernel32.dll", "GetSystemDirectoryA", |c| sysdir(c, false, "C:\\Windows\\System32")),
        ("kernel32.dll", "GetSystemDirectoryW", |c| sysdir(c, true,  "C:\\Windows\\System32")),
        ("kernel32.dll", "GetWindowsDirectoryA", |c| sysdir(c, false, "C:\\Windows")),
        ("kernel32.dll", "GetWindowsDirectoryW", |c| sysdir(c, true,  "C:\\Windows")),
        ("kernel32.dll", "GetSystemWindowsDirectoryW", |c| sysdir(c, true, "C:\\Windows")),
        ("kernel32.dll", "ExpandEnvironmentStringsW", expand_env_strings_w),
        ("kernel32.dll", "ExpandEnvironmentStringsA", expand_env_strings_a),
        ("kernel32.dll", "SetEnvironmentVariableW", r1_2),
        ("kernel32.dll", "SetEnvironmentVariableA", r1_2),
        (
            "kernel32.dll",
            "GetCurrentDirectoryA",
            get_current_directory_a,
        ),
        ("kernel32.dll", "GetFullPathNameW", get_full_path_name_w),
        ("kernel32.dll", "GetFullPathNameA", get_full_path_name_a),
        ("kernel32.dll", "GetFileAttributesW", get_file_attributes_w),
        ("kernel32.dll", "GetFileAttributesA", get_file_attributes_a),
        ("kernel32.dll", "FindFirstFileW", find_first_file_w),
        ("kernel32.dll", "FindFirstFileA", find_first_file_a),
        ("kernel32.dll", "FindNextFileW", find_next_file_w),
        ("kernel32.dll", "FindNextFileA", find_next_file_a),
        ("kernel32.dll", "FindClose", find_close),
        ("kernel32.dll", "GetVolumeInformationW", get_volume_information_w),
        ("kernel32.dll", "GetVolumeInformationA", get_volume_information_a),
        ("kernel32.dll", "FileTimeToSystemTime", file_time_to_system_time),
        ("kernel32.dll", "FileTimeToLocalFileTime", file_time_to_local_file_time),
        ("kernel32.dll", "SystemTimeToFileTime", system_time_to_file_time),
        ("kernel32.dll", "LocalFileTimeToFileTime", file_time_to_local_file_time),
        ("kernel32.dll", "GetDateFormatW", |c| date_time_format(c, true, true)),
        ("kernel32.dll", "GetDateFormatA", |c| date_time_format(c, false, true)),
        ("kernel32.dll", "GetTimeFormatW", |c| date_time_format(c, true, false)),
        ("kernel32.dll", "GetTimeFormatA", |c| date_time_format(c, false, false)),
        ("kernel32.dll", "GetDiskFreeSpaceW", |c| {
            // (root, *sectorsPerCluster, *bytesPerSector, *freeClusters, *totalClusters)
            if c.arg(1) != 0 { let _ = c.memory.write_u32(c.arg(1), 8); }
            if c.arg(2) != 0 { let _ = c.memory.write_u32(c.arg(2), 512); }
            if c.arg(3) != 0 { let _ = c.memory.write_u32(c.arg(3), 0x10000); }
            if c.arg(4) != 0 { let _ = c.memory.write_u32(c.arg(4), 0x20000); }
            c.ret_stdcall(1, 5); Handled::Ok
        }),
        ("kernel32.dll", "GetDiskFreeSpaceExW", |c| {
            // (dir, *freeAvail(u64), *total(u64), *totalFree(u64))
            for a in [1u32, 2, 3] {
                let p = c.arg(a);
                if p != 0 { let _ = c.memory.write_u32(p, 0x4000_0000); let _ = c.memory.write_u32(p + 4, 0); }
            }
            c.ret_stdcall(1, 4); Handled::Ok
        }),
        ("kernel32.dll", "GetStdHandle", get_std_handle),
        ("kernel32.dll", "WriteConsoleW", write_console_w),
        ("kernel32.dll", "GetEnvironmentStringsW", |c| { let p = env_block(c, true); c.ret_stdcall(p, 0); Handled::Ok }),
        ("kernel32.dll", "GetEnvironmentStrings", |c| { let p = env_block(c, false); c.ret_stdcall(p, 0); Handled::Ok }),
        ("kernel32.dll", "FreeEnvironmentStringsW", r1_1),
        ("kernel32.dll", "GetEnvironmentStringsA", |c| { let p = env_block(c, false); c.ret_stdcall(p, 0); Handled::Ok }),
        ("kernel32.dll", "FreeEnvironmentStringsA", r1_1),
        ("kernel32.dll", "InitializeCriticalSection", r0_1),
        (
            "kernel32.dll",
            "InitializeCriticalSectionAndSpinCount",
            r1_2,
        ),
        ("kernel32.dll", "InitializeCriticalSectionEx", r1_3),
        ("kernel32.dll", "DeleteCriticalSection", r0_1),
        ("kernel32.dll", "EnterCriticalSection", r0_1),
        ("kernel32.dll", "LeaveCriticalSection", r0_1),
        ("kernel32.dll", "TryEnterCriticalSection", r1_1),
        ("kernel32.dll", "TlsAlloc", tls_alloc),
        ("kernel32.dll", "TlsSetValue", tls_set),
        ("kernel32.dll", "TlsGetValue", tls_get),
        ("kernel32.dll", "TlsFree", r1_1),
        ("kernel32.dll", "GetConsoleCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetConsoleOutputCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetConsoleMode", get_console_mode),
        ("kernel32.dll", "SetConsoleMode", r1_2),
        ("kernel32.dll", "GetConsoleScreenBufferInfo", get_console_screen_buffer_info),
        ("kernel32.dll", "SetConsoleTextAttribute", r1_2),
        ("kernel32.dll", "SetConsoleCursorPosition", r1_2),
        ("kernel32.dll", "SetConsoleTitleA", r1_1),
        ("kernel32.dll", "SetConsoleTitleW", r1_1),
        ("kernel32.dll", "GetConsoleTitleA", r0_2),
        ("kernel32.dll", "GetConsoleTitleW", r0_2),
        ("kernel32.dll", "SetConsoleScreenBufferSize", r1_2),
        ("kernel32.dll", "FillConsoleOutputCharacterA", r1_5),
        ("kernel32.dll", "FillConsoleOutputAttribute", r1_5),
        ("kernel32.dll", "ScrollConsoleScreenBufferA", r1_5),
        ("kernel32.dll", "SetConsoleWindowInfo", r1_3),
        ("kernel32.dll", "GetConsoleCursorInfo", r1_2),
        ("kernel32.dll", "SetConsoleCursorInfo", r1_2),
        ("kernel32.dll", "ReadConsoleA", read_console_a),
        ("kernel32.dll", "ReadConsoleW", read_console_w),
        ("kernel32.dll", "SetConsoleInputExeNameW", r1_1),
        ("kernel32.dll", "SetConsoleInputExeNameA", r1_1),
        ("kernel32.dll", "GetConsoleInputExeNameW", r1_2),
        ("kernel32.dll", "GetConsoleInputExeNameA", r1_2),
        ("kernel32.dll", "CopyFileExW", r1_6),
        ("kernel32.dll", "CopyFileExA", r1_6),
        ("kernel32.dll", "WriteConsoleInputW", r1_4),
        ("kernel32.dll", "ReadConsoleInputW", r0_4),
        ("kernel32.dll", "ReadConsoleInputA", r0_4),
        ("kernel32.dll", "PeekConsoleInputW", r0_4),
        ("kernel32.dll", "GetNumberOfConsoleInputEvents", r0_2),
        ("kernel32.dll", "FlushConsoleInputBuffer", r1_1),
        ("kernel32.dll", "SetConsoleCtrlHandler", r1_2),
        ("kernel32.dll", "MultiByteToWideChar", multibyte_to_widechar),
        ("kernel32.dll", "WideCharToMultiByte", widechar_to_multibyte),
        ("kernel32.dll", "GetACP", |c| {
            c.ret_stdcall(1252, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetOEMCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "IsValidCodePage", r1_1),
        ("kernel32.dll", "GetCPInfo", r0_2),
        ("kernel32.dll", "LCMapStringW", r0_6),
        ("kernel32.dll", "LCMapStringEx", r0_8),
        ("kernel32.dll", "CreateFileA", create_file_a),
        ("kernel32.dll", "CreateFileW", create_file_w),
        ("kernel32.dll", "GetFileSize", get_file_size),
        ("kernel32.dll", "GetFileSizeEx", get_file_size),
        ("kernel32.dll", "SetFilePointer", set_file_pointer),
        ("kernel32.dll", "CreateDirectoryA", create_directory_a),
        ("kernel32.dll", "CreateDirectoryW", create_directory_w),
        ("kernel32.dll", "DeleteFileA", delete_file_a),
        ("kernel32.dll", "DeleteFileW", delete_file_w),
        ("kernel32.dll", "GetFileType", get_file_type),
        ("kernel32.dll", "SetHandleInformation", r1_3),
        ("kernel32.dll", "DuplicateHandle", dup_handle),
        ("kernel32.dll", "TerminateProcess", terminate_process),
        ("kernel32.dll", "RaiseException", raise_exception),
        ("kernel32.dll", "GetStringTypeW", r0_4),
        ("kernel32.dll", "GetStringTypeA", r0_5),
        ("kernel32.dll", "FormatMessageA", format_message_a),
        ("kernel32.dll", "FormatMessageW", format_message_w),
        ("kernel32.dll", "OutputDebugStringA", output_debug_string_a),
        ("kernel32.dll", "OutputDebugStringW", r0_1),
        ("kernel32.dll", "EncodePointer", |c| {
            let p = c.arg(0);
            c.ret_stdcall(p ^ 0xDEAD, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "DecodePointer", |c| {
            let p = c.arg(0);
            c.ret_stdcall(p ^ 0xDEAD, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "Sleep", r0_1),
        ("kernel32.dll", "WaitForSingleObject", r0_2), // WAIT_OBJECT_0
        ("kernel32.dll", "WaitForSingleObjectEx", r0_3),   // (handle, ms, alertable)
        ("kernel32.dll", "WaitForMultipleObjects", r0_4),  // (n, handles, all, ms)
        ("kernel32.dll", "WaitForMultipleObjectsEx", r0_5),
        ("kernel32.dll", "SignalObjectAndWait", r0_4),
        ("ntdll.dll", "RtlIsStateSeparationEnabled", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("kernel32.dll", "CreateEventA", |c| { c.ret_stdcall(0xE700_0001, 4); Handled::Ok }),
        ("kernel32.dll", "CreateEventW", |c| { c.ret_stdcall(0xE700_0001, 4); Handled::Ok }),
        ("kernel32.dll", "SetEvent", r1_1),
        ("kernel32.dll", "ResetEvent", r1_1),
        ("kernel32.dll", "CreateProcessA", create_process_a),
        ("kernel32.dll", "CreateProcessW", create_process_w),
        ("kernel32.dll", "GetExitCodeProcess", get_exit_code_process),
        ("kernel32.dll", "Beep", beep),
        ("kernel32.dll", "InterlockedIncrement", interlocked_inc),
        ("kernel32.dll", "InterlockedDecrement", interlocked_dec),
        ("kernel32.dll", "InterlockedExchange", interlocked_xchg),
        (
            "kernel32.dll",
            "InterlockedCompareExchange",
            interlocked_cmpxchg,
        ),
        // ── API-MS-WIN API set forwarders ──────────────────────────────────
        // ApiSetQueryApiSetPresence(ApiSetName, Present*) — checks if an API set
        // DLL is present.  We say "no" (FALSE) so callers skip optional features.
        ("api-ms-win-core-apiquery-l1-1-0.dll", "ApiSetQueryApiSetPresence",
            |c| { let p = c.arg(1); if p != 0 { let _ = c.memory.write_u32(p, 0); } c.ret_stdcall(0, 2); Handled::Ok }),
        // GlobalAlloc / GlobalFree / GlobalLock / GlobalUnlock — thin wrappers
        // around the process heap; we just forward to our heap routines.
        ("kernel32.dll",                         "GlobalAlloc",  global_alloc),
        // LocalAlloc(uFlags, uBytes) — same as GlobalAlloc in our model.
        ("kernel32.dll",                         "LocalAlloc",   global_alloc),
        ("kernel32.dll",                         "LocalFree",    |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll",                         "LocalLock",    |c| { let p = c.arg(0); c.ret_stdcall(p, 1); Handled::Ok }),
        ("kernel32.dll",                         "LocalUnlock",  |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("kernel32.dll",                         "LocalReAlloc", |c| { let p = c.arg(0); let n = c.arg(1); let r = c.heap_realloc(p, n); c.ret_stdcall(r, 3); Handled::Ok }),
        ("kernel32.dll",                         "LocalSize",    |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll",                         "SetPriorityClass", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        // Registry value query (W10 explorer). 7 args; report not-found.
        ("kernel32.dll", "RegGetValueW", |c| { if c.arg(6) != 0 { let _ = c.memory.write_u32(c.arg(6), 0); } c.ret_stdcall(2, 7); Handled::Ok }),
        ("kernel32.dll", "RegGetValueA", |c| { if c.arg(6) != 0 { let _ = c.memory.write_u32(c.arg(6), 0); } c.ret_stdcall(2, 7); Handled::Ok }),
        // Named mutexes / events with the extended (4-arg) variants -> fake handle.
        ("kernel32.dll", "CreateMutexW",   |c| { c.ret_stdcall(0x4D54_0002, 3); Handled::Ok }),
        ("kernel32.dll", "CreateMutexA",   |c| { c.ret_stdcall(0x4D54_0002, 3); Handled::Ok }),
        ("kernel32.dll", "CreateMutexExW", |c| { c.ret_stdcall(0x4D54_0001, 4); Handled::Ok }),
        ("kernel32.dll", "CreateMutexExA", |c| { c.ret_stdcall(0x4D54_0001, 4); Handled::Ok }),
        ("kernel32.dll", "CreateEventExW", |c| { c.ret_stdcall(0x4576_0001, 4); Handled::Ok }),
        ("kernel32.dll", "CreateSemaphoreExW", |c| { c.ret_stdcall(0x5365_0001, 6); Handled::Ok }),
        ("kernel32.dll", "ReleaseMutex", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        // Condition variables (explorer's CRT/threadpool resolve these dynamically).
        ("kernel32.dll", "InitializeConditionVariable", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "WakeConditionVariable", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "WakeAllConditionVariable", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "SleepConditionVariableCS", |c| { c.ret_stdcall(1, 3); Handled::Ok }),
        ("kernel32.dll", "SleepConditionVariableSRW", |c| { c.ret_stdcall(1, 4); Handled::Ok }),
        // Threadpool timers/work -> fake handles, no-op (we have no real threads).
        ("kernel32.dll", "CreateThreadpoolTimer", |c| { c.ret_stdcall(0x5450_0001, 3); Handled::Ok }),
        ("kernel32.dll", "SetThreadpoolTimer", |c| { c.ret_stdcall(0, 4); Handled::Ok }),
        ("kernel32.dll", "CloseThreadpoolTimer", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "WaitForThreadpoolTimerCallbacks", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("kernel32.dll", "CreateThreadpoolWork", |c| { c.ret_stdcall(0x5450_0002, 3); Handled::Ok }),
        ("kernel32.dll", "SubmitThreadpoolWork", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "CloseThreadpoolWork", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "WaitForThreadpoolWorkCallbacks", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("kernel32.dll", "RegisterTraceGuidsW", |c| { c.ret_stdcall(0, 8); Handled::Ok }),
        ("kernel32.dll", "RegisterTraceGuidsA", |c| { c.ret_stdcall(0, 8); Handled::Ok }),
        ("kernel32.dll", "UnregisterTraceGuids", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "GetTraceLoggerHandle", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "TraceMessage", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("kernel32.dll", "CreateThreadpoolWait", |c| { c.ret_stdcall(0x5450_0003, 3); Handled::Ok }),
        // ntdll bits explorer resolves dynamically; safe no-ops.
        ("ntdll.dll", "RtlDllShutdownInProgress", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("kernel32.dll",                         "GetPriorityClass", |c| { c.ret_stdcall(0x20, 1); Handled::Ok }),
        // Activation contexts (theming/manifests): no-op stubs so explorer's
        // dynamic resolve + calls succeed instead of returning NULL.
        ("kernel32.dll", "CreateActCtxW", |c| { c.ret_stdcall(0xAC70_0001, 1); Handled::Ok }),
        ("kernel32.dll", "CreateActCtxA", |c| { c.ret_stdcall(0xAC70_0001, 1); Handled::Ok }),
        ("kernel32.dll", "ActivateActCtx", |c| { let o = c.arg(1); if o != 0 { let _ = c.memory.write_u32(o, 1); } c.ret_stdcall(1, 2); Handled::Ok }),
        ("kernel32.dll", "DeactivateActCtx", |c| { c.ret_stdcall(1, 2); Handled::Ok }),
        ("kernel32.dll", "ReleaseActCtx", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll", "AddRefActCtx", |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll",                         "GlobalFree",   |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("kernel32.dll",                         "GlobalLock",   |c| { let p = c.arg(0); c.ret_stdcall(p, 1); Handled::Ok }),
        ("kernel32.dll",                         "GlobalUnlock", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        ("kernel32.dll",                         "GlobalHandle", |c| { let p = c.arg(0); c.ret_stdcall(p, 1); Handled::Ok }),
        ("kernel32.dll",                         "GlobalSize",   |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("api-ms-win-core-heap-l2-1-0.dll",      "GlobalAlloc",  global_alloc),
        ("api-ms-win-core-heap-l2-1-0.dll",      "GlobalFree",   |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("api-ms-win-core-heap-l2-1-0.dll",      "GlobalLock",   |c| { let p = c.arg(0); c.ret_stdcall(p, 1); Handled::Ok }),
        ("api-ms-win-core-heap-l2-1-0.dll",      "GlobalUnlock", |c| { c.ret_stdcall(1, 1); Handled::Ok }),
        // FormatMessageW forwarded through the localization API set.
        ("api-ms-win-core-localization-l1-2-0.dll", "FormatMessageW", format_message_w_fwd),
        ("api-ms-win-core-localization-l1-2-0.dll", "FormatMessageA", format_message_a_fwd),
        // ── MFC42U.DLL Stubs ───────────────────────────────────────────────
        // Paint (mspaint.exe) imports ordinal 1165 from MFC42U.DLL.
        // It's likely AfxGetModuleState() or AfxGetApp() which returns a pointer
        // to a large CWinApp / AFX_MODULE_STATE structure.
        // Returning 0 crashes because it tries to write to [EAX+0x14].
        // We return a dummy heap pointer so the writes succeed.
        ("shell32.dll", "#68", |c| { c.ret_stdcall(0, 6); Handled::Ok }),   // RunFileDlg (6 args)
        ("shell32.dll", "#188", |c| { c.ret_stdcall(0, 3); Handled::Ok }), // SHGetSetSettings (3 args, void)
        ("shell32.dll", "#100", |c| { let out = c.arg(2); if out != 0 { let _ = c.memory.write_u32(out, 0); } c.ret_stdcall(0x8000_4005, 3); Handled::Ok }), // SHCreateStdEnumFmtEtc (3 args)
        ("shell32.dll", "#245", |c| { c.ret_stdcall(0, 2); Handled::Ok }), // SHTestTokenMembership (2 args)
        ("shell32.dll", "#660", |c| { c.ret_stdcall(0, 3); Handled::Ok }), // SHWaitForFileToOpen (3 args)
        ("shell32.dll", "#723", |c| { let out = c.arg(1); if out != 0 { let _ = c.memory.write_u32(out, 0); } c.ret_stdcall(0, 2); Handled::Ok }),
        ("shell32.dll", "SHGetSpecialFolderPathW", sh_get_special_folder_path_w),
        ("shell32.dll", "SHGetSpecialFolderPathA", sh_get_special_folder_path_a),
        ("shdocvw.dll", "#110", |c| { c.ret_stdcall(0, 0); Handled::Ok }),  // WinList_Init — S_OK
        ("shdocvw.dll", "#111", |c| { c.ret_stdcall(0, 0); Handled::Ok }),  // WinList_Terminate
        ("shdocvw.dll", "#125", |c| { c.ret_stdcall(0, 0); Handled::Ok }),  // SHCreateFromDesktop
        ("shdocvw.dll", "DllInstall", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("ole32.dll", "CoCreateInstance", |c| {
            let out = c.arg(4);
            if out != 0 {
                let _ = c.memory.write_u32(out, 0);
            }
            c.ret_stdcall(0x8004_0154, 5);
            Handled::Ok
        }),
        ("ole32.dll", "OleUninitialize", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        ("ole32.dll", "CoUninitialize", |c| { c.ret_stdcall(0, 0); Handled::Ok }),
        // COM task allocator — must return real memory or C++ code throws bad_alloc.
        ("ole32.dll", "CoTaskMemAlloc",   |c| { let n = c.arg(0); let p = c.heap_alloc(n); c.ret_stdcall(p, 1); Handled::Ok }),
        ("ole32.dll", "CoTaskMemFree",    |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("ole32.dll", "CoTaskMemRealloc", |c| { let p = c.arg(0); let n = c.arg(1); let r = c.heap_realloc(p, n); c.ret_stdcall(r, 2); Handled::Ok }),
        ("ole32.dll", "CoInitialize",     |c| { c.ret_stdcall(0, 1); Handled::Ok }),       // S_OK
        ("ole32.dll", "CoInitializeEx",   |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("ole32.dll", "OleInitialize",    |c| { c.ret_stdcall(0, 1); Handled::Ok }),
        ("ole32.dll", "CoCreateGuid",     |c| { let p = c.arg(0); if p != 0 { let _ = c.memory.write_bytes(p, &[0u8; 16]); } c.ret_stdcall(0, 1); Handled::Ok }),
        ("ole32.dll", "CoGetMalloc",      |c| { let o = c.arg(1); if o != 0 { let _ = c.memory.write_u32(o, 0); } c.ret_stdcall(0x8000_4001u32, 2); Handled::Ok }),
        ("mfc42u.dll", "#1165", |c| {
            let ptr = c.heap_alloc(256); // give it a nice 256 byte chunk to write to
            c.ret_stdcall(ptr, 1);       // guess: 1 arg? The log said "cleaned 1 args"
            Handled::Ok
        }),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn path_find_file_name_a(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let mut last_slash = 0;
    let mut curr = p;
    while let Ok(b) = ctx.memory.read_u8(curr) {
        if b == 0 { break; }
        if b == b'\\' || b == b'/' {
            last_slash = curr + 1;
        }
        curr += 1;
    }
    let res = if last_slash == 0 { p } else { last_slash };
    ctx.ret_stdcall(res, 1);
    Handled::Ok
}

fn path_find_file_name_w(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let mut last_slash = 0;
    let mut curr = p;
    loop {
        let low = ctx.memory.read_u8(curr).unwrap_or(0);
        let high = ctx.memory.read_u8(curr + 1).unwrap_or(0);
        let w = u16::from_le_bytes([low, high]);
        if w == 0 { break; }
        if w == '\\' as u16 || w == '/' as u16 {
            last_slash = curr + 2;
        }
        curr += 2;
    }
    let res = if last_slash == 0 { p } else { last_slash };
    ctx.ret_stdcall(res, 1);
    Handled::Ok
}

fn strcmp_ni_a(ctx: &mut ApiContext) -> Handled {
    let n = ctx.arg(2) as usize;
    let a = ctx.cstr(ctx.arg(0)).chars().take(n).collect::<String>().to_lowercase();
    let b = ctx.cstr(ctx.arg(1)).chars().take(n).collect::<String>().to_lowercase();
    ctx.ret_stdcall(a.cmp(&b) as i32 as u32, 3);
    Handled::Ok
}

fn strcmp_ni_w(ctx: &mut ApiContext) -> Handled {
    let n = ctx.arg(2) as usize;
    let a = ctx.wstr(ctx.arg(0)).chars().take(n).collect::<String>().to_lowercase();
    let b = ctx.wstr(ctx.arg(1)).chars().take(n).collect::<String>().to_lowercase();
    ctx.ret_stdcall(a.cmp(&b) as i32 as u32, 3);
    Handled::Ok
}

fn get_private_profile_string_a(ctx: &mut ApiContext) -> Handled {
    let default = ctx.cstr(ctx.arg(2));
    let out = ctx.arg(3);
    let max = ctx.arg(4) as usize;
    let n = default.len().min(max.saturating_sub(1));
    if out != 0 && max > 0 {
        let _ = ctx.memory.write_bytes(out, &default.as_bytes()[..n]);
        let _ = ctx.memory.write_u8(out + n as u32, 0);
    }
    ctx.ret_stdcall(n as u32, 6);
    Handled::Ok
}

fn get_private_profile_string_w(ctx: &mut ApiContext) -> Handled {
    let default = ctx.wstr(ctx.arg(2));
    let out = ctx.arg(3);
    let max = ctx.arg(4) as usize;
    let wide: Vec<u16> = default.encode_utf16().collect();
    let n = wide.len().min(max.saturating_sub(1));
    if out != 0 && max > 0 {
        for (i, &ch) in wide.iter().take(n).enumerate() {
            let _ = ctx.memory.write_u16(out + (i as u32) * 2, ch);
        }
        let _ = ctx.memory.write_u16(out + (n as u32) * 2, 0);
    }
    ctx.ret_stdcall(n as u32, 6);
    Handled::Ok
}

fn sh_reg_get_bool_us_value(ctx: &mut ApiContext) -> Handled {
    let default = ctx.arg(3);
    ctx.ret_stdcall(default, 4);
    Handled::Ok
}

fn sh_reg_create_us_key(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(3);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0x5A5A_0001);
    }
    ctx.ret_stdcall(0, 5);
    Handled::Ok
}

fn sh_create_thread_ref(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0);
    }
    ctx.ret_stdcall(0x8000_4005, 2);
    Handled::Ok
}

fn sh_get_special_folder_path_a(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    let path = b"C:\\Users\\guest";
    if out != 0 {
        let _ = ctx.memory.write_bytes(out, path);
        let _ = ctx.memory.write_u8(out + path.len() as u32, 0);
    }
    ctx.ret_stdcall(1, 4);
    Handled::Ok
}

fn sh_get_special_folder_path_w(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let mut len = 0u32;
        for ch in "C:\\Users\\guest".encode_utf16() {
            let _ = ctx.memory.write_u16(out + len * 2, ch);
            len += 1;
        }
        let _ = ctx.memory.write_u16(out + len * 2, 0);
    }
    ctx.ret_stdcall(1, 4);
    Handled::Ok
}

/// FormatMessageW forwarded from the localization API set DLL.
/// Delegates to the same implementation used by kernel32.dll!FormatMessageW.
fn format_message_w_fwd(ctx: &mut ApiContext) -> Handled {
    let r = format_message_core(ctx, true);
    ctx.ret_stdcall(r, 7);
    Handled::Ok
}
fn format_message_a_fwd(ctx: &mut ApiContext) -> Handled {
    let r = format_message_core(ctx, false);
    ctx.ret_stdcall(r, 7);
    Handled::Ok
}

fn global_alloc(ctx: &mut ApiContext) -> Handled {
    // GlobalAlloc(uFlags, dwBytes) — allocate `dwBytes` from the heap.
    // We ignore the flags (GMEM_FIXED, GMEM_ZEROINIT etc.) and just allocate.
    let size = ctx.arg(1);
    let ptr = ctx.heap_alloc(size);
    ctx.ret_stdcall(ptr, 2);
    Handled::Ok
}

fn exit_process(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(ctx.arg(0))
}

fn terminate_process(ctx: &mut ApiContext) -> Handled {
    Handled::ExitProcess(ctx.arg(1))
}

fn get_std_handle(ctx: &mut ApiContext) -> Handled {
    let id = ctx.arg(0);
    let h = match id {
        0xFFFF_FFF6 => STD_INPUT_HANDLE,
        0xFFFF_FFF5 => STD_OUTPUT_HANDLE,
        0xFFFF_FFF4 => STD_ERROR_HANDLE,
        _ => INVALID_HANDLE,
    };
    ctx.ret_stdcall(h, 1);
    Handled::Ok
}

fn write_file(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let buf = ctx.arg(1);
    let count = ctx.arg(2);
    let out = ctx.arg(3);
    let bytes = ctx
        .memory
        .read_bytes(buf, count as usize)
        .unwrap_or_default();

    // VFS-backed file handle?
    let file = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, cursor, .. }) => Some((path.clone(), *cursor)),
        _ => None,
    };

    if let Some((path, cursor)) = file {
        let mut content = ctx.fs.read_file(&path).unwrap_or_default();
        let start = cursor as usize;
        let end = start + bytes.len();
        if content.len() < end {
            content.resize(end, 0);
        }
        content[start..end].copy_from_slice(&bytes);
        let _ = ctx.fs.mount_file(&path, content);
        if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(handle) {
            *cursor += bytes.len() as u64;
        }
    } else {
        route_output(handle, &bytes, ctx);
    }

    if out != 0 {
        let _ = ctx.memory.write_u32(out, bytes.len() as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

fn write_console_a(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let buf = ctx.arg(1);
    let count = ctx.arg(2);
    let out = ctx.arg(3);
    let bytes = ctx
        .memory
        .read_bytes(buf, count as usize)
        .unwrap_or_default();
    route_output(handle, &bytes, ctx);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, bytes.len() as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

fn write_console_w(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let buf = ctx.arg(1);
    let count = ctx.arg(2);
    let out = ctx.arg(3);
    
    let bytes = ctx.memory.read_bytes(buf, (count * 2) as usize).unwrap_or_default();
    let u16s: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let s = String::from_utf16_lossy(&u16s);
    
    route_output(handle, s.as_bytes(), ctx);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, count);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

fn route_output(handle: u32, bytes: &[u8], ctx: &mut ApiContext) {
    match handle {
        h if h == STD_OUTPUT_HANDLE => ctx.console.stdout.extend_from_slice(bytes),
        h if h == STD_ERROR_HANDLE => ctx.console.stderr.extend_from_slice(bytes),
        _ => ctx.console.stdout.extend_from_slice(bytes), // assume stdout for file handles
    }
}

fn read_file(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let buf = ctx.arg(1);
    let max = ctx.arg(2);
    let out = ctx.arg(3);

    let file = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, cursor, .. }) => Some((path.clone(), *cursor)),
        _ => None,
    };

    let n = if let Some((path, cursor)) = file {
        let content = ctx.fs.read_file(&path).unwrap_or_default();
        let start = (cursor as usize).min(content.len());
        let n = (max as usize).min(content.len() - start);
        if n > 0 {
            let _ = ctx.memory.write_bytes(buf, &content[start..start + n]);
            if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(handle) {
                *cursor += n as u64;
            }
        }
        n
    } else {
        // Console stdin is line-buffered in the default console mode. Do not
        // return a partial command to callers like cmd.exe before Enter.
        let Some(data) = read_console_line(ctx, max as usize) else {
            return Handled::Block;
        };
        let n = data.len();
        if !data.is_empty() {
            let _ = ctx.memory.write_bytes(buf, &data);
        }
        n
    };

    if out != 0 {
        let _ = ctx.memory.write_u32(out, n as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

fn close_handle(ctx: &mut ApiContext) -> Handled {
    let h = ctx.arg(0);
    ctx.handles.remove(h);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// Read one line of console input (up to a newline or the char limit) from the
// process stdin buffer. Blocks (WaitingForInput) when no input is available, so
// the frontend can supply typed text. Returns the raw bytes consumed.
fn read_console_line(ctx: &mut ApiContext, limit: usize) -> Option<Vec<u8>> {
    if limit == 0 {
        return Some(Vec::new());
    }
    if ctx.console.stdin.is_empty() {
        return None; // caller turns this into Handled::Block
    }

    let newline = ctx.console.stdin.iter().position(|&b| b == b'\n');
    let available = match newline {
        Some(pos) => (pos + 1).min(limit),
        None if ctx.console.stdin.len() >= limit => limit,
        None => return None,
    };

    let mut line = Vec::with_capacity(available);
    for _ in 0..available {
        if let Some(b) = ctx.console.stdin.pop_front() {
            line.push(b);
        }
    }
    Some(line)
}

// ReadConsoleW(hInput, lpBuffer, nChars, lpNumRead, pInputControl) — wide.
fn read_console_w(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let n_chars = ctx.arg(2) as usize;
    let out = ctx.arg(3);
    let Some(line) = read_console_line(ctx, n_chars) else {
        return Handled::Block;
    };
    let wide: Vec<u8> = line
        .iter()
        .flat_map(|&b| (b as u16).to_le_bytes())
        .collect();
    let _ = ctx.memory.write_bytes(buf, &wide);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, line.len() as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

// ReadConsoleA(hInput, lpBuffer, nChars, lpNumRead, pInputControl) — narrow.
fn read_console_a(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let n_chars = ctx.arg(2) as usize;
    let out = ctx.arg(3);
    let Some(line) = read_console_line(ctx, n_chars) else {
        return Handled::Block;
    };
    let _ = ctx.memory.write_bytes(buf, &line);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, line.len() as u32);
    }
    ctx.ret_stdcall(1, 5);
    Handled::Ok
}

// file APIs (Milestone 7)

const CREATE_NEW: u32 = 1;
const CREATE_ALWAYS: u32 = 2;
const OPEN_EXISTING: u32 = 3;
const TRUNCATE_EXISTING: u32 = 5;

fn create_file(ctx: &mut ApiContext, name: String, nargs: u32) -> Handled {
    let access = ctx.arg(1);
    let disposition = ctx.arg(4);
    let path = ctx.resolve_path(&name);
    let exists = ctx.fs.node_exists(&path);
    let writable = access & 0x4000_0000 != 0; // GENERIC_WRITE

    // disposition rules
    if disposition == OPEN_EXISTING && !exists {
        ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
        ctx.ret_stdcall(INVALID_HANDLE, nargs);
        return Handled::Ok;
    }
    if disposition == CREATE_NEW && exists {
        ctx.cpu.last_error = ERROR_FILE_EXISTS;
        ctx.ret_stdcall(INVALID_HANDLE, nargs);
        return Handled::Ok;
    }

    let truncate = disposition == CREATE_ALWAYS || disposition == TRUNCATE_EXISTING;
    if !exists || truncate {
        if ctx.fs.mount_file(&path, Vec::new()).is_err() {
            ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
            ctx.ret_stdcall(INVALID_HANDLE, nargs);
            return Handled::Ok;
        }
    }

    let h = ctx.handles.insert(KernelObject::VfsFile {
        path,
        cursor: 0,
        writable,
    });
    ctx.cpu.last_error = 0;
    ctx.ret_stdcall(h, nargs);
    Handled::Ok
}

fn create_file_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    create_file(ctx, name, 7)
}

fn create_file_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    create_file(ctx, name, 7)
}

fn get_file_size(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let high = ctx.arg(1);
    let size = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, .. }) => {
            ctx.fs.read_file(path).map(|b| b.len()).unwrap_or(0) as u32
        }
        _ => 0,
    };
    if high != 0 {
        let _ = ctx.memory.write_u32(high, 0);
    }
    ctx.ret_stdcall(size, 2);
    Handled::Ok
}

fn set_file_pointer(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let dist = ctx.arg(1) as i32 as i64;
    let method = ctx.arg(3); // FILE_BEGIN=0, FILE_CURRENT=1, FILE_END=2

    let (cur, size) = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, cursor, .. }) => (
            *cursor as i64,
            ctx.fs.read_file(path).map(|b| b.len()).unwrap_or(0) as i64,
        ),
        _ => {
            ctx.ret_stdcall(INVALID_HANDLE, 4);
            return Handled::Ok;
        }
    };
    let base = match method {
        1 => cur,
        2 => size,
        _ => 0,
    };
    let new_pos = (base + dist).max(0) as u64;
    if let Some(KernelObject::VfsFile { cursor, .. }) = ctx.handles.get_mut(handle) {
        *cursor = new_pos;
    }
    ctx.ret_stdcall(new_pos as u32, 4);
    Handled::Ok
}

fn create_directory(ctx: &mut ApiContext, name: String) -> Handled {
    let path = ctx.resolve_path(&name);
    if ctx.fs.node_exists(&path) {
        ctx.cpu.last_error = 183; // ERROR_ALREADY_EXISTS
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    }
    let ok = ctx.fs.create_dir(&path).is_ok();
    ctx.ret_stdcall(ok as u32, 2);
    Handled::Ok
}

fn create_directory_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    create_directory(ctx, name)
}

fn create_directory_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    create_directory(ctx, name)
}

fn delete_file(ctx: &mut ApiContext, name: String) -> Handled {
    let path = ctx.resolve_path(&name);
    let ok = ctx.fs.delete_node(&path).is_ok();
    ctx.ret_stdcall(ok as u32, 1);
    Handled::Ok
}

fn delete_file_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    delete_file(ctx, name)
}

fn delete_file_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    delete_file(ctx, name)
}

fn get_file_attributes(ctx: &mut ApiContext, name: String) -> Handled {
    let path = ctx.resolve_path(&name);
    let attr = if !ctx.fs.node_exists(&path) {
        ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
        INVALID_FILE_ATTRIBUTES
    } else if ctx.fs.read_file(&path).is_ok() {
        FILE_ATTRIBUTE_NORMAL
    } else {
        FILE_ATTRIBUTE_DIRECTORY
    };
    ctx.ret_stdcall(attr, 1);
    Handled::Ok
}

fn get_file_attributes_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    get_file_attributes(ctx, name)
}

fn get_file_attributes_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    get_file_attributes(ctx, name)
}

// Match a filename against a DOS wildcard pattern (* and ?), case-insensitive.
fn wildcard_match(pattern: &str, name: &str) -> bool {
    fn m(p: &[u8], n: &[u8]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(b'*'), _) => m(&p[1..], n) || (!n.is_empty() && m(p, &n[1..])),
            (Some(b'?'), Some(_)) => m(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => m(&p[1..], &n[1..]),
            _ => false,
        }
    }
    m(pattern.as_bytes(), name.as_bytes())
}

// Split a search path "C:\dir\pattern" into (dir, pattern). A bare "*.*" or "*"
// matches everything.
fn split_search(path: &str) -> (String, String) {
    let p = path.replace('/', "\\");
    match p.rfind('\\') {
        Some(i) => (p[..i].to_string(), p[i + 1..].to_string()),
        None => (String::new(), p),
    }
}

// Build the match list for a FindFirstFile pattern against the VFS.
fn find_matches(ctx: &ApiContext, raw: &str) -> Vec<(String, bool, u64)> {
    let resolved = ctx.resolve_path(raw);
    let (dir, pat) = split_search(&resolved);
    let pat = if pat.is_empty() { "*".to_string() } else { pat };
    let mut out = Vec::new();
    if let Ok(entries) = ctx.fs.list_dir(&dir) {
        for e in entries {
            if wildcard_match(&pat, &e.name) {
                let is_dir = matches!(e.kind, crate::fs::vfs::EntryKind::Directory);
                out.push((e.name, is_dir, e.size));
            }
        }
    }
    out
}

// Fill a WIN32_FIND_DATAW at `p` for one entry (wide cFileName at +44).
fn fill_find_data_w(ctx: &mut ApiContext, p: u32, name: &str, is_dir: bool, size: u64) {
    let attrs = if is_dir { FILE_ATTRIBUTE_DIRECTORY } else { FILE_ATTRIBUTE_NORMAL };
    let _ = ctx.memory.write_bytes(p, &[0u8; 44]); // attrs + 3 FILETIMEs + sizes + reserved
    let _ = ctx.memory.write_u32(p, attrs);
    let _ = ctx.memory.write_u32(p + 28, (size >> 32) as u32);
    let _ = ctx.memory.write_u32(p + 32, size as u32);
    let mut wide: Vec<u8> = name.encode_utf16().take(259).flat_map(|c| c.to_le_bytes()).collect();
    wide.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(p + 44, &wide);
}

// Same, ANSI cFileName for WIN32_FIND_DATAA (cFileName at +44, bytes).
fn fill_find_data_a(ctx: &mut ApiContext, p: u32, name: &str, is_dir: bool, size: u64) {
    let attrs = if is_dir { FILE_ATTRIBUTE_DIRECTORY } else { FILE_ATTRIBUTE_NORMAL };
    let _ = ctx.memory.write_bytes(p, &[0u8; 44]);
    let _ = ctx.memory.write_u32(p, attrs);
    let _ = ctx.memory.write_u32(p + 28, (size >> 32) as u32);
    let _ = ctx.memory.write_u32(p + 32, size as u32);
    let mut bytes = name.as_bytes().to_vec();
    bytes.truncate(259);
    bytes.push(0);
    let _ = ctx.memory.write_bytes(p + 44, &bytes);
}

const INVALID_HANDLE_VALUE: u32 = 0xFFFF_FFFF;
const ERROR_NO_MORE_FILES: u32 = 18;

fn find_first_common(ctx: &mut ApiContext, raw: &str, data: u32, wide: bool) -> u32 {
    let matches = find_matches(ctx, raw);
    if matches.is_empty() {
        ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
        return INVALID_HANDLE_VALUE;
    }
    let (name, is_dir, size) = matches[0].clone();
    if wide { fill_find_data_w(ctx, data, &name, is_dir, size); }
    else { fill_find_data_a(ctx, data, &name, is_dir, size); }
    ctx.handles.insert(KernelObject::FindHandle { matches, cursor: 1 })
}

fn find_first_file_w(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.wstr(ctx.arg(0));
    let data = ctx.arg(1);
    let h = find_first_common(ctx, &raw, data, true);
    ctx.ret_stdcall(h, 2);
    Handled::Ok
}

fn find_first_file_a(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.cstr(ctx.arg(0));
    let data = ctx.arg(1);
    let h = find_first_common(ctx, &raw, data, false);
    ctx.ret_stdcall(h, 2);
    Handled::Ok
}

fn find_next_common(ctx: &mut ApiContext, handle: u32, data: u32, wide: bool) -> u32 {
    let next = match ctx.handles.get(handle) {
        Some(KernelObject::FindHandle { matches, cursor }) if *cursor < matches.len() => {
            Some(matches[*cursor].clone())
        }
        _ => None,
    };
    match next {
        Some((name, is_dir, size)) => {
            if wide { fill_find_data_w(ctx, data, &name, is_dir, size); }
            else { fill_find_data_a(ctx, data, &name, is_dir, size); }
            if let Some(KernelObject::FindHandle { cursor, .. }) = ctx.handles.get_mut(handle) {
                *cursor += 1;
            }
            1
        }
        None => {
            ctx.cpu.last_error = ERROR_NO_MORE_FILES;
            0
        }
    }
}

fn find_next_file_w(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let data = ctx.arg(1);
    let r = find_next_common(ctx, handle, data, true);
    ctx.ret_stdcall(r, 2);
    Handled::Ok
}

fn find_next_file_a(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let data = ctx.arg(1);
    let r = find_next_common(ctx, handle, data, false);
    ctx.ret_stdcall(r, 2);
    Handled::Ok
}

fn find_close(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    ctx.handles.remove(handle);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// GetVolumeInformation(root, volNameBuf, volNameSize, *serial, *maxComp,
//                      *fsFlags, fsNameBuf, fsNameSize) — 8 args.
fn get_volume_info(ctx: &mut ApiContext, wide: bool) -> Handled {
    let vol_buf = ctx.arg(1);
    let serial = ctx.arg(3);
    let max_comp = ctx.arg(4);
    let fs_flags = ctx.arg(5);
    let fs_buf = ctx.arg(6);
    let write = |ctx: &mut ApiContext, p: u32, s: &str| {
        if p == 0 { return; }
        if wide {
            let mut b: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
            b.extend_from_slice(&[0, 0]);
            let _ = ctx.memory.write_bytes(p, &b);
        } else {
            let mut b = s.as_bytes().to_vec();
            b.push(0);
            let _ = ctx.memory.write_bytes(p, &b);
        }
    };
    write(ctx, vol_buf, "WEBWINE");
    write(ctx, fs_buf, "NTFS");
    if serial != 0 { let _ = ctx.memory.write_u32(serial, 0x1234_ABCD); }
    if max_comp != 0 { let _ = ctx.memory.write_u32(max_comp, 255); }
    if fs_flags != 0 { let _ = ctx.memory.write_u32(fs_flags, 0); }
    ctx.ret_stdcall(1, 8);
    Handled::Ok
}

fn get_volume_information_w(ctx: &mut ApiContext) -> Handled { get_volume_info(ctx, true) }
fn get_volume_information_a(ctx: &mut ApiContext) -> Handled { get_volume_info(ctx, false) }

// Convert a FILETIME (100ns since 1601) to civil (year, month, day, dow, h, m, s).
fn filetime_to_civil(ft: u64) -> (u16, u16, u16, u16, u16, u16, u16) {
    let secs = (ft / 10_000_000) as i64;
    let days_1601 = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let (h, mi, s) = ((tod / 3600) as u16, ((tod % 3600) / 60) as u16, (tod % 60) as u16);
    // days since 1970-01-01 (1601->1970 = 134774 days)
    let days = days_1601 - 134774;
    let dow = (days.rem_euclid(7) + 4) % 7; // 1970-01-01 was Thursday (4)
    // Howard Hinnant's civil_from_days
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as u16, m as u16, d as u16, dow as u16, h, mi, s)
}

fn read_filetime(ctx: &ApiContext, p: u32) -> u64 {
    let lo = ctx.memory.read_u32(p).unwrap_or(0) as u64;
    let hi = ctx.memory.read_u32(p + 4).unwrap_or(0) as u64;
    (hi << 32) | lo
}

// FileTimeToSystemTime(lpFileTime, lpSystemTime). SYSTEMTIME: 8 u16s.
fn file_time_to_system_time(ctx: &mut ApiContext) -> Handled {
    let ft = read_filetime(ctx, ctx.arg(0));
    let st = ctx.arg(1);
    let (y, mo, d, dow, h, mi, s) = filetime_to_civil(ft);
    if st != 0 {
        for (i, v) in [y, mo, dow, d, h, mi, s, 0].iter().enumerate() {
            let _ = ctx.memory.write_u16(st + (i as u32) * 2, *v);
        }
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// FileTimeToLocalFileTime(lpFileTime, lpLocalFileTime): no timezone — copy.
fn file_time_to_local_file_time(ctx: &mut ApiContext) -> Handled {
    let ft = read_filetime(ctx, ctx.arg(0));
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, ft as u32);
        let _ = ctx.memory.write_u32(out + 4, (ft >> 32) as u32);
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// SystemTimeToFileTime(lpSystemTime, lpFileTime): approximate (fixed epoch ok
// for display). We just emit a constant recent FILETIME.
fn system_time_to_file_time(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    let ft: u64 = 133_000_000_000_000_000; // ~2022
    if out != 0 {
        let _ = ctx.memory.write_u32(out, ft as u32);
        let _ = ctx.memory.write_u32(out + 4, (ft >> 32) as u32);
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// GetDateFormat/GetTimeFormat(locale, flags, lpDate(SYSTEMTIME), lpFormat,
// lpStr, cchStr) -> chars written. We produce MM/dd/yyyy and hh:mm.
fn date_time_format(ctx: &mut ApiContext, wide: bool, is_date: bool) -> Handled {
    let st = ctx.arg(2);
    let buf = ctx.arg(4);
    let cch = ctx.arg(5);
    let rd = |ctx: &ApiContext, off: u32| ctx.memory.read_u16(st + off).unwrap_or(0);
    let text = if is_date {
        format!("{:02}/{:02}/{:04}", rd(ctx, 2), rd(ctx, 6), rd(ctx, 0)) // MM/dd/yyyy
    } else {
        let h = rd(ctx, 8);
        let (h12, ap) = if h == 0 { (12, "AM") } else if h < 12 { (h, "AM") } else if h == 12 { (12, "PM") } else { (h - 12, "PM") };
        format!("{:02}:{:02} {}", h12, rd(ctx, 10), ap)
    };
    let n = if wide {
        let units: Vec<u16> = text.encode_utf16().collect();
        let w = if cch > 0 { units.len().min(cch as usize - 1) } else { units.len() };
        if buf != 0 {
            let mut b: Vec<u8> = units[..w].iter().flat_map(|u| u.to_le_bytes()).collect();
            b.extend_from_slice(&[0, 0]);
            let _ = ctx.memory.write_bytes(buf, &b);
        }
        w + 1
    } else {
        let bytes = text.as_bytes();
        let w = if cch > 0 { bytes.len().min(cch as usize - 1) } else { bytes.len() };
        if buf != 0 {
            let _ = ctx.memory.write_bytes(buf, &bytes[..w]);
            let _ = ctx.memory.write_u8(buf + w as u32, 0);
        }
        w + 1
    };
    ctx.ret_stdcall(n as u32, 6);
    Handled::Ok
}

// CreateProcessA(appName, cmdLine, procAttr, threadAttr, inherit, flags, env,
//                cwd, startupInfo, processInfo) — 10 args.
fn create_process(ctx: &mut ApiContext, app: String, cmd: String) -> Handled {
    let pi = ctx.arg(9); // lpProcessInformation

    // Prefer lpApplicationName; otherwise take the first token of the command line.
    let target = if !app.is_empty() {
        app
    } else {
        first_token(&cmd)
    };
    let path = ctx.resolve_path(&target);

    if ctx.fs.read_file(&path).is_err() {
        ctx.cpu.last_error = ERROR_FILE_NOT_FOUND;
        ctx.ret_stdcall(0, 10); // FALSE
        return Handled::Ok;
    }

    let child = ctx.next_child_pid;
    // Fill PROCESS_INFORMATION { hProcess, hThread, dwProcessId, dwThreadId }.
    if pi != 0 {
        let fake_proc = 0x0000_0F00 | child;
        let _ = ctx.memory.write_u32(pi, fake_proc);
        let _ = ctx.memory.write_u32(pi + 4, fake_proc | 0x1_0000);
        let _ = ctx.memory.write_u32(pi + 8, child);
        let _ = ctx.memory.write_u32(pi + 12, child * 100);
    }
    ctx.spawns
        .push(crate::vm::process::SpawnRequest { path, pi_addr: pi });
    ctx.cpu.last_error = 0;
    ctx.ret_stdcall(1, 10); // TRUE
    Handled::Ok
}

fn create_process_a(ctx: &mut ApiContext) -> Handled {
    let app = if ctx.arg(0) != 0 {
        ctx.cstr(ctx.arg(0))
    } else {
        String::new()
    };
    let cmd = if ctx.arg(1) != 0 {
        ctx.cstr(ctx.arg(1))
    } else {
        String::new()
    };
    create_process(ctx, app, cmd)
}

fn create_process_w(ctx: &mut ApiContext) -> Handled {
    let app = if ctx.arg(0) != 0 {
        ctx.wstr(ctx.arg(0))
    } else {
        String::new()
    };
    let cmd = if ctx.arg(1) != 0 {
        ctx.wstr(ctx.arg(1))
    } else {
        String::new()
    };
    create_process(ctx, app, cmd)
}

fn first_token(cmd: &str) -> String {
    let cmd = cmd.trim();
    if cmd.starts_with('"') {
        cmd[1..].split('"').next().unwrap_or("").to_string()
    } else {
        cmd.split_whitespace().next().unwrap_or("").to_string()
    }
}

fn get_exit_code_process(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0);
    } // assume exited 0
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// GetProcAddress(hModule, lpProcName) — return a real trampoline for a function
// we implement (so the guest can call it), else 0 (caller uses its fallback).
fn get_proc_address(ctx: &mut ApiContext) -> Handled {
    let name_arg = ctx.arg(1);
    let va = if name_arg < 0x1_0000 {
        0 // imported by ordinal — not supported
    } else {
        let name = ctx.cstr(name_arg);
        let v = ctx.proc_addr.get(&name).copied().unwrap_or(0);
        if v == 0 {
            // A miss means a dynamically-resolved import we don't provide; the
            // guest may call NULL. Logged at trace for diagnosis ("Run as debug").
            ctx.logs.log(crate::logs::LogLevel::Trace, "api",
                &format!("GetProcAddress miss: {name}"), Some(ctx.pid));
        }
        v
    };
    ctx.ret_stdcall(va, 2);
    Handled::Ok
}

// Beep(dwFreq, dwDuration) — emit a UI beep for the frontend (Web Audio).
fn beep(ctx: &mut ApiContext) -> Handled {
    let freq = ctx.arg(0);
    let duration = ctx.arg(1);
    ctx.ui_events.push(crate::vm::process::UiEvent::Beep { freq, duration });
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// GetFullPathNameW(lpFileName, nBufferLength, lpBuffer, lpFilePart) — resolve
// to an absolute path. Rust std calls this to canonicalize before CreateFile.
fn get_full_path_name_w(ctx: &mut ApiContext) -> Handled {
    let input = ctx.wstr(ctx.arg(0));
    let buf_len = ctx.arg(1); // WCHARs
    let out = ctx.arg(2);
    let file_part = ctx.arg(3);

    let full = ctx.resolve_path(&input);
    let wide: Vec<u16> = full.encode_utf16().collect();
    let needed = wide.len() as u32;

    if out != 0 && buf_len > needed {
        for (i, &c) in wide.iter().enumerate() {
            let _ = ctx.memory.write_u16(out + (i as u32) * 2, c);
        }
        let _ = ctx.memory.write_u16(out + needed * 2, 0);
        if file_part != 0 {
            let last = full.rfind('\\').map(|i| i + 1).unwrap_or(0) as u32;
            let _ = ctx.memory.write_u32(file_part, out + last * 2);
        }
        ctx.ret_stdcall(needed, 4);
    } else {
        ctx.ret_stdcall(needed + 1, 4); // required size incl. null
    }
    Handled::Ok
}

fn get_full_path_name_a(ctx: &mut ApiContext) -> Handled {
    let input = ctx.cstr(ctx.arg(0));
    let buf_len = ctx.arg(1);
    let out = ctx.arg(2);
    let file_part = ctx.arg(3);

    let full = ctx.resolve_path(&input);
    let bytes = full.as_bytes();
    let needed = bytes.len() as u32;

    if out != 0 && buf_len > needed {
        let _ = ctx.memory.write_bytes(out, bytes);
        let _ = ctx.memory.write_u8(out + needed, 0);
        if file_part != 0 {
            let last = full.rfind('\\').map(|i| i + 1).unwrap_or(0) as u32;
            let _ = ctx.memory.write_u32(file_part, out + last);
        }
        ctx.ret_stdcall(needed, 4);
    } else {
        ctx.ret_stdcall(needed + 1, 4);
    }
    Handled::Ok
}

fn get_current_directory_w(ctx: &mut ApiContext) -> Handled {
    let buf_len = ctx.arg(0);
    let out = ctx.arg(1);
    let wide: Vec<u16> = ctx.cwd.encode_utf16().collect();
    let needed = wide.len() as u32;
    if out != 0 && buf_len > needed {
        for (i, &c) in wide.iter().enumerate() {
            let _ = ctx.memory.write_u16(out + (i as u32) * 2, c);
        }
        let _ = ctx.memory.write_u16(out + needed * 2, 0);
        ctx.ret_stdcall(needed, 2);
    } else {
        ctx.ret_stdcall(needed + 1, 2);
    }
    Handled::Ok
}

fn get_current_directory_a(ctx: &mut ApiContext) -> Handled {
    let buf_len = ctx.arg(0);
    let out = ctx.arg(1);
    let bytes = ctx.cwd.clone().into_bytes();
    let needed = bytes.len() as u32;
    if out != 0 && buf_len > needed {
        let _ = ctx.memory.write_bytes(out, &bytes);
        let _ = ctx.memory.write_u8(out + needed, 0);
        ctx.ret_stdcall(needed, 2);
    } else {
        ctx.ret_stdcall(needed + 1, 2);
    }
    Handled::Ok
}

// SetCurrentDirectory(path): update the process working directory. Returns
// nonzero on success (we always accept; the path need not exist in our VFS).
fn set_current_directory_a(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.cstr(ctx.arg(0));
    *ctx.cwd = ctx.resolve_path(&raw);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn set_current_directory_w(ctx: &mut ApiContext) -> Handled {
    let raw = ctx.wstr(ctx.arg(0));
    *ctx.cwd = ctx.resolve_path(&raw);
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// We have no registry. Report "key not found" so apps fall back to defaults
// (ERROR_FILE_NOT_FOUND). Returning success with bogus data breaks apps that
// read real settings from the registry (e.g. cmd.exe's Command Processor key).
fn reg_open_key(ctx: &mut ApiContext) -> Handled {
    let phk = ctx.arg(4);
    if phk != 0 {
        let _ = ctx.memory.write_u32(phk, 0);
    }
    ctx.ret_stdcall(2, 5); // ERROR_FILE_NOT_FOUND
    Handled::Ok
}

fn reg_query_value(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(2, 6); // ERROR_FILE_NOT_FOUND
    Handled::Ok
}

fn get_last_error(ctx: &mut ApiContext) -> Handled {
    let e = ctx.cpu.last_error;
    ctx.ret_stdcall(e, 0);
    Handled::Ok
}

fn set_last_error(ctx: &mut ApiContext) -> Handled {
    ctx.cpu.last_error = ctx.arg(0);
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn get_process_heap(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0x1000_0000, 0);
    Handled::Ok
}

fn heap_alloc(ctx: &mut ApiContext) -> Handled {
    let size = ctx.arg(2);
    let ptr = ctx.heap_alloc(size);
    ctx.ret_stdcall(ptr, 3);
    Handled::Ok
}

fn heap_free(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

fn heap_realloc(ctx: &mut ApiContext) -> Handled {
    // HeapReAlloc(hHeap, dwFlags, lpMem, dwBytes)
    let old = ctx.arg(2);
    let size = ctx.arg(3);
    let ptr = ctx.heap_realloc(old, size);
    ctx.ret_stdcall(ptr, 4);
    Handled::Ok
}

fn heap_size(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0, 3);
    Handled::Ok
}

fn heap_create(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(0x1000_0000, 3);
    Handled::Ok
}

fn virtual_alloc(ctx: &mut ApiContext) -> Handled {
    let addr = ctx.arg(0);
    let size = ctx.arg(1);
    let ptr = if addr != 0 {
        addr
    } else {
        ctx.heap_alloc(size)
    };
    ctx.ret_stdcall(ptr, 4);
    Handled::Ok
}

fn virtual_protect(ctx: &mut ApiContext) -> Handled {
    let old_out = ctx.arg(3);
    if old_out != 0 {
        let _ = ctx.memory.write_u32(old_out, 0x40);
    }
    ctx.ret_stdcall(1, 4);
    Handled::Ok
}

// A non-zero fake HMODULE for dynamically loaded DLLs. GetProcAddress resolves
// functions by name regardless of the module handle, so a single sentinel is
// enough — and returning non-NULL lets delay-load helpers succeed instead of
// raising ERROR_MOD_NOT_FOUND (0xC06D007E).
const FAKE_MODULE: u32 = 0x5AD0_0000;

fn load_library_a(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(FAKE_MODULE, 1);
    Handled::Ok
}

fn load_library_w(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(FAKE_MODULE, 1);
    Handled::Ok
}

fn get_module_handle_a(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(
        ctx.memory
            .regions
            .values()
            .next()
            .map(|r| r.base)
            .unwrap_or(0x0040_0000),
        1,
    );
    Handled::Ok
}

fn get_module_handle_w(ctx: &mut ApiContext) -> Handled {
    get_module_handle_a(ctx)
}

// GetModuleHandleEx(flags, name, &out_module) — write image base, return TRUE
fn get_module_handle_ex(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(2);
    let base = ctx
        .memory
        .regions
        .values()
        .next()
        .map(|r| r.base)
        .unwrap_or(0x0040_0000);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, base);
    }
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

fn get_module_filename_a(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let cap = ctx.arg(2);
    let mut name = ctx.exe_path.as_bytes().to_vec();
    name.push(0);
    let n = name.len().min(cap as usize);
    let _ = ctx.memory.write_bytes(buf, &name[..n]);
    ctx.ret_stdcall(n.saturating_sub(1) as u32, 3); // excl. null
    Handled::Ok
}

fn get_module_filename_w(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(1);
    let cap = ctx.arg(2);
    let wide: Vec<u16> = ctx.exe_path.encode_utf16().collect();
    let n = wide.len().min(cap.saturating_sub(1) as usize);
    for (i, &c) in wide[..n].iter().enumerate() {
        let _ = ctx.memory.write_u16(buf + (i as u32) * 2, c);
    }
    let _ = ctx.memory.write_u16(buf + (n as u32) * 2, 0);
    ctx.ret_stdcall(n as u32, 3);
    Handled::Ok
}

// Default process environment. cmd.exe aborts with "Null environment" if this
// is empty, and apps expect the usual variables. Block format: consecutive
// "NAME=VALUE\0" entries terminated by a final empty string (extra \0).
const ENV_VARS: &[&str] = &[
    "ALLUSERSPROFILE=C:\\Users\\guest",
    "APPDATA=C:\\Users\\guest\\AppData\\Roaming",
    "ComSpec=C:\\Windows\\System32\\cmd.exe",
    "COMPUTERNAME=WEBWINE",
    "HOMEDRIVE=C:",
    "HOMEPATH=\\Users\\guest",
    "NUMBER_OF_PROCESSORS=1",
    "OS=Windows_NT",
    "Path=C:\\Windows\\System32;C:\\Windows",
    "PATHEXT=.COM;.EXE;.BAT;.CMD",
    "PROMPT=$P$G",
    "SystemDrive=C:",
    "SystemRoot=C:\\Windows",
    "TEMP=C:\\Users\\guest\\AppData\\Local\\Temp",
    "TMP=C:\\Users\\guest\\AppData\\Local\\Temp",
    "USERNAME=guest",
    "USERPROFILE=C:\\Users\\guest",
    "windir=C:\\Windows",
];

// Look up an environment variable's value by name (case-insensitive).
fn env_lookup(name: &str) -> Option<&'static str> {
    ENV_VARS.iter().find_map(|v| {
        let (k, val) = v.split_once('=')?;
        if k.eq_ignore_ascii_case(name) { Some(val) } else { None }
    })
}

fn expand_env_text(src: &str) -> String {
    let mut out = String::new();
    let mut rest = src;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('%') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let name = &after_start[..end];
        if let Some(value) = env_lookup(name) {
            out.push_str(value);
        } else {
            out.push('%');
            out.push_str(name);
            out.push('%');
        }
        rest = &after_start[end + 1..];
    }
    out.push_str(rest);
    out
}

fn expand_env_strings_a(ctx: &mut ApiContext) -> Handled {
    let expanded = expand_env_text(&ctx.cstr(ctx.arg(0)));
    let out = ctx.arg(1);
    let size = ctx.arg(2);
    let needed = expanded.len() as u32 + 1;
    if out != 0 && size > 0 {
        let n = expanded.len().min(size.saturating_sub(1) as usize);
        let _ = ctx.memory.write_bytes(out, &expanded.as_bytes()[..n]);
        let _ = ctx.memory.write_u8(out + n as u32, 0);
    }
    ctx.ret_stdcall(needed, 3);
    Handled::Ok
}

fn expand_env_strings_w(ctx: &mut ApiContext) -> Handled {
    let expanded = expand_env_text(&ctx.wstr(ctx.arg(0)));
    let out = ctx.arg(1);
    let size = ctx.arg(2);
    let units: Vec<u16> = expanded.encode_utf16().collect();
    let needed = units.len() as u32 + 1;
    if out != 0 && size > 0 {
        let n = units.len().min(size.saturating_sub(1) as usize);
        for (i, &ch) in units.iter().take(n).enumerate() {
            let _ = ctx.memory.write_u16(out + (i as u32) * 2, ch);
        }
        let _ = ctx.memory.write_u16(out + (n as u32) * 2, 0);
    }
    ctx.ret_stdcall(needed, 3);
    Handled::Ok
}

// ERROR_ENVVAR_NOT_FOUND
const ERROR_ENVVAR_NOT_FOUND: u32 = 203;

// GetSystemDirectory/GetWindowsDirectory(lpBuffer, uSize) -> length written.
fn sysdir(ctx: &mut ApiContext, wide: bool, path: &str) -> Handled {
    let buf = ctx.arg(0);
    let size = ctx.arg(1);
    let n = if wide {
        let units: Vec<u16> = path.encode_utf16().collect();
        let len = units.len() as u32;
        if buf != 0 && size > len {
            let mut b: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
            b.extend_from_slice(&[0, 0]);
            let _ = ctx.memory.write_bytes(buf, &b);
            len
        } else { len + 1 }
    } else {
        let bytes = path.as_bytes();
        let len = bytes.len() as u32;
        if buf != 0 && size > len {
            let _ = ctx.memory.write_bytes(buf, bytes);
            let _ = ctx.memory.write_u8(buf + len, 0);
            len
        } else { len + 1 }
    };
    ctx.ret_stdcall(n, 2);
    Handled::Ok
}

fn get_env_var_a(ctx: &mut ApiContext) -> Handled {
    let name = ctx.cstr(ctx.arg(0));
    let buf = ctx.arg(1);
    let size = ctx.arg(2);
    match env_lookup(&name) {
        Some(val) => {
            let mut bytes = val.as_bytes().to_vec();
            let needed = bytes.len() as u32 + 1;
            if buf != 0 && size >= needed {
                bytes.push(0);
                let _ = ctx.memory.write_bytes(buf, &bytes);
                ctx.ret_stdcall(needed - 1, 3);
            } else {
                ctx.ret_stdcall(needed, 3); // required size incl. NUL
            }
        }
        None => { ctx.cpu.last_error = ERROR_ENVVAR_NOT_FOUND; ctx.ret_stdcall(0, 3); }
    }
    Handled::Ok
}

fn get_env_var_w(ctx: &mut ApiContext) -> Handled {
    let name = ctx.wstr(ctx.arg(0));
    let buf = ctx.arg(1);
    let size = ctx.arg(2); // in WCHARs
    match env_lookup(&name) {
        Some(val) => {
            let units: Vec<u16> = val.encode_utf16().collect();
            let needed = units.len() as u32 + 1;
            if buf != 0 && size >= needed {
                let mut bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
                bytes.extend_from_slice(&[0, 0]);
                let _ = ctx.memory.write_bytes(buf, &bytes);
                ctx.ret_stdcall(needed - 1, 3);
            } else {
                ctx.ret_stdcall(needed, 3);
            }
        }
        None => { ctx.cpu.last_error = ERROR_ENVVAR_NOT_FOUND; ctx.ret_stdcall(0, 3); }
    }
    Handled::Ok
}

// Build the environment block in guest memory and return its pointer.
fn env_block(ctx: &mut ApiContext, wide: bool) -> u32 {
    if wide {
        let mut units: Vec<u16> = Vec::new();
        for v in ENV_VARS {
            units.extend(v.encode_utf16());
            units.push(0);
        }
        units.push(0); // final terminator
        let bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
        let p = ctx.heap_alloc(bytes.len() as u32);
        let _ = ctx.memory.write_bytes(p, &bytes);
        p
    } else {
        let mut bytes: Vec<u8> = Vec::new();
        for v in ENV_VARS {
            bytes.extend_from_slice(v.as_bytes());
            bytes.push(0);
        }
        bytes.push(0);
        let p = ctx.heap_alloc(bytes.len() as u32);
        let _ = ctx.memory.write_bytes(p, &bytes);
        p
    }
}

fn get_command_line_a(ctx: &mut ApiContext) -> Handled {
    // The full command line (argv[0] is the quoted image path). PEB-area scratch.
    let va = 0x7FFD_F100;
    let line = format!("{}\0", ctx.cmdline);
    let _ = ctx.memory.write_bytes(va, line.as_bytes());
    ctx.ret_stdcall(va, 0);
    Handled::Ok
}

fn get_command_line_w(ctx: &mut ApiContext) -> Handled {
    let va = 0x7FFD_F200;
    let line = format!("{}\0", ctx.cmdline);
    let wide: Vec<u8> = line.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    let _ = ctx.memory.write_bytes(va, &wide);
    ctx.ret_stdcall(va, 0);
    Handled::Ok
}

// STARTUPINFO is 68 bytes; cb at +0. Zero everything (so dwFlags has no
// STARTF_USESTDHANDLES → the CRT falls back to GetStdHandle) and set cb.
fn get_startup_info(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_bytes(p, &[0u8; 68]);
        let _ = ctx.memory.write_u32(p, 68); // cb = sizeof(STARTUPINFO)
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn get_current_process_id(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(ctx.pid, 0);
    Handled::Ok
}

fn get_current_thread_id(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(ctx.pid * 100, 0);
    Handled::Ok
}

fn get_current_process(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(CURRENT_PROCESS, 0);
    Handled::Ok
}

fn get_current_thread(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(CURRENT_THREAD, 0);
    Handled::Ok
}

fn get_system_time(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_bytes(p, &[0u8; 8]);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn query_perf_counter(ctx: &mut ApiContext) -> Handled {
    // Frequency is reported as 1 MHz, so the counter is in microseconds.
    let p = ctx.arg(0);
    if p != 0 {
        let micros = crate::winapi::winmm::tick_ms() as u64 * 1000;
        let _ = ctx.memory.write_u32(p, micros as u32);
        let _ = ctx.memory.write_u32(p + 4, (micros >> 32) as u32);
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn get_tick_count(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(crate::winapi::winmm::tick_ms(), 0);
    Handled::Ok
}

fn query_perf_freq(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx
            .memory
            .write_bytes(p, &[0x40, 0x42, 0x0F, 0, 0, 0, 0, 0]);
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn get_file_type(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(2, 1);
    Handled::Ok // FILE_TYPE_CHAR
}

// GetConsoleScreenBufferInfo(hConsole, lpInfo): fill a CONSOLE_SCREEN_BUFFER_INFO
// so console apps that query the buffer geometry (cmd.exe) proceed.
// Layout: dwSize(COORD)@0, dwCursorPosition(COORD)@4, wAttributes(WORD)@8,
//         srWindow(SMALL_RECT)@10, dwMaximumWindowSize(COORD)@18.
// GetVersionEx(lpVersionInfo): fill OSVERSIONINFO for Windows XP (5.1.2600).
// dwOSVersionInfoSize@0, dwMajorVersion@4, dwMinorVersion@8, dwBuildNumber@12,
// dwPlatformId@16 (VER_PLATFORM_WIN32_NT=2), szCSDVersion@20.
fn get_version_ex(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_u32(p + 4, 5); // major
        let _ = ctx.memory.write_u32(p + 8, 1); // minor
        let _ = ctx.memory.write_u32(p + 12, 2600); // build
        let _ = ctx.memory.write_u32(p + 16, 2); // VER_PLATFORM_WIN32_NT
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn get_console_screen_buffer_info(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(1);
    if p != 0 {
        let cols: u16 = 80;
        let rows: u16 = 25;
        let coord = |x: u16, y: u16| (x as u32) | ((y as u32) << 16);
        let _ = ctx.memory.write_u32(p, coord(cols, rows)); // dwSize
        let _ = ctx.memory.write_u32(p + 4, coord(0, 0)); // dwCursorPosition
        let _ = ctx.memory.write_u16(p + 8, 0x07); // wAttributes (gray on black)
                                                   // srWindow: Left=0, Top=0, Right=cols-1, Bottom=rows-1 (i16 each)
        let _ = ctx.memory.write_u16(p + 10, 0);
        let _ = ctx.memory.write_u16(p + 12, 0);
        let _ = ctx.memory.write_u16(p + 14, cols - 1);
        let _ = ctx.memory.write_u16(p + 16, rows - 1);
        let _ = ctx.memory.write_u32(p + 18, coord(cols, rows)); // dwMaximumWindowSize
    }
    ctx.ret_stdcall(1, 2); // success
    Handled::Ok
}

// GetConsoleMode(hConsole, lpMode): succeed with a standard console mode so
// console apps (cmd.exe) believe they are attached to a real console.
fn get_console_mode(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let lp_mode = ctx.arg(1);
    // Input handle gets input flags; output handles get output flags.
    let mode = if handle == STD_INPUT_HANDLE {
        0x1F7 // ENABLE_PROCESSED/LINE/ECHO/MOUSE/INSERT/QUICKEDIT/EXTENDED
    } else {
        0x3 // ENABLE_PROCESSED_OUTPUT | ENABLE_WRAP_AT_EOL_OUTPUT
    };
    if lp_mode != 0 {
        let _ = ctx.memory.write_u32(lp_mode, mode);
    }
    ctx.ret_stdcall(1, 2); // success
    Handled::Ok
}

fn dup_handle(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(5);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, ctx.arg(2));
    }
    ctx.ret_stdcall(1, 7);
    Handled::Ok
}

fn raise_exception(ctx: &mut ApiContext) -> Handled {
    let code = ctx.arg(0);
    ctx.logs.log(crate::logs::LogLevel::Warn, "api",
        &format!("RaiseException code=0x{code:08X}"), Some(ctx.pid));
    Handled::ExitProcess(1)
}

fn output_debug_string_a(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let s = ctx.cstr(p);
    ctx.console.stderr.extend_from_slice(s.as_bytes());
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn tls_alloc(ctx: &mut ApiContext) -> Handled {
    let slot = *ctx.next_tls;
    *ctx.next_tls += 1;
    ctx.tls_slots.insert(slot, 0);
    ctx.ret_stdcall(slot, 0);
    Handled::Ok
}

fn fls_alloc(ctx: &mut ApiContext) -> Handled {
    let slot = *ctx.next_tls;
    *ctx.next_tls += 1;
    ctx.tls_slots.insert(slot, 0);
    ctx.ret_stdcall(slot, 1);
    Handled::Ok
}

fn tls_set(ctx: &mut ApiContext) -> Handled {
    let slot = ctx.arg(0);
    let value = ctx.arg(1);
    ctx.tls_slots.insert(slot, value);
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

fn tls_get(ctx: &mut ApiContext) -> Handled {
    let slot = ctx.arg(0);
    let value = ctx.tls_slots.get(&slot).copied().unwrap_or(0);
    ctx.ret_stdcall(value, 1);
    Handled::Ok
}

fn multibyte_to_widechar(ctx: &mut ApiContext) -> Handled {
    let src = ctx.arg(2);
    let srcl = ctx.arg(3);
    let dst = ctx.arg(4);
    let dstl = ctx.arg(5);
    let s = if srcl == 0xFFFF_FFFF {
        ctx.cstr(src)
    } else {
        String::from_utf8_lossy(
            &ctx.memory
                .read_bytes(src, srcl as usize)
                .unwrap_or_default(),
        )
        .into_owned()
    };
    let wide: Vec<u16> = s.encode_utf16().collect();
    let out_len = if dstl == 0 {
        wide.len() + 1
    } else {
        if dst != 0 {
            for (i, &c) in wide.iter().take(dstl as usize).enumerate() {
                let _ = ctx.memory.write_u16(dst + (i as u32) * 2, c);
            }
            if (wide.len() as u32) < dstl {
                let _ = ctx.memory.write_u16(dst + wide.len() as u32 * 2, 0);
            }
        }
        wide.len().min(dstl as usize) + 1
    };
    ctx.ret_stdcall(out_len as u32, 6);
    Handled::Ok
}

fn widechar_to_multibyte(ctx: &mut ApiContext) -> Handled {
    let src = ctx.arg(2);
    let srcl = ctx.arg(3);
    let dst = ctx.arg(4);
    let dstl = ctx.arg(5);
    let nchars = if srcl == 0xFFFF_FFFF {
        ctx.memory.read_wstr(src).len() + 1
    } else {
        srcl as usize
    };
    let s = ctx.memory.read_wstr(src);
    let bytes = s.as_bytes();
    let n = bytes.len().min(if dstl == 0 { 0 } else { dstl as usize });
    if n > 0 && dst != 0 {
        let _ = ctx.memory.write_bytes(dst, &bytes[..n]);
    }
    ctx.ret_stdcall(if dstl == 0 { nchars as u32 } else { n as u32 }, 8);
    Handled::Ok
}

fn interlocked_inc(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let v = ctx.memory.read_u32(p).unwrap_or(0).wrapping_add(1);
    let _ = ctx.memory.write_u32(p, v);
    ctx.ret_stdcall(v, 1);
    Handled::Ok
}

fn interlocked_dec(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let v = ctx.memory.read_u32(p).unwrap_or(0).wrapping_sub(1);
    let _ = ctx.memory.write_u32(p, v);
    ctx.ret_stdcall(v, 1);
    Handled::Ok
}

fn interlocked_xchg(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let new = ctx.arg(1);
    let old = ctx.memory.read_u32(p).unwrap_or(0);
    let _ = ctx.memory.write_u32(p, new);
    ctx.ret_stdcall(old, 2);
    Handled::Ok
}

fn interlocked_cmpxchg(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let new = ctx.arg(1);
    let cmp = ctx.arg(2);
    let old = ctx.memory.read_u32(p).unwrap_or(0);
    if old == cmp {
        let _ = ctx.memory.write_u32(p, new);
    }
    ctx.ret_stdcall(old, 3);
    Handled::Ok
}

// stub helpers

// Stub family: return a constant in EAX and clean exactly `n` stdcall args.
// Naming: r{val}_{nargs}. Correct arg counts matter — a wrong count drifts
// the guest stack and eventually derails a `ret`.
macro_rules! stubs {
    ($($name:ident => ($val:expr, $n:expr)),* $(,)?) => {
        $( #[allow(dead_code)] fn $name(c: &mut ApiContext) -> Handled { c.ret_stdcall($val, $n); Handled::Ok } )*
    };
}

stubs! {
    r0_0 => (0, 0), r0_1 => (0, 1), r0_2 => (0, 2), r0_3 => (0, 3),
    r0_4 => (0, 4), r0_5 => (0, 5), r0_6 => (0, 6), r0_7 => (0, 7),
    r0_8 => (0, 8),
    r1_0 => (1, 0), r1_1 => (1, 1), r1_2 => (1, 2), r1_3 => (1, 3),
    r1_4 => (1, 4), r1_5 => (1, 5), r1_6 => (1, 6), r1_7 => (1, 7),
}

fn stub_invalid_handle(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(INVALID_HANDLE, 7);
    Handled::Ok
}

// ─── FormatMessage ────────────────────────────────────────────────────────────
// FormatMessage(dwFlags, lpSource, dwMessageId, dwLanguageId,
//               lpBuffer, nSize, Arguments) — 7 args, stdcall.
//
// Flags we honour:
//   FORMAT_MESSAGE_FROM_SYSTEM (0x1000)  → look up dwMessageId in the table.
//   FORMAT_MESSAGE_FROM_STRING (0x0400)  → format lpSource as template.
//   FORMAT_MESSAGE_ALLOCATE_BUFFER (0x100) → heap-allocate; write ptr at lpBuffer.
//   FORMAT_MESSAGE_IGNORE_INSERTS (0x200) → skip %n substitution.
//
// Everything else silently falls back to a generic "unknown error" message so
// the caller's error-checking paths still work.

const FORMAT_MESSAGE_ALLOCATE_BUFFER: u32 = 0x0000_0100;
const FORMAT_MESSAGE_IGNORE_INSERTS:  u32 = 0x0000_0200;
const FORMAT_MESSAGE_FROM_HMODULE:    u32 = 0x0000_0800;
const FORMAT_MESSAGE_FROM_STRING:     u32 = 0x0000_0400;
const FORMAT_MESSAGE_FROM_SYSTEM:     u32 = 0x0000_1000;

/// Minimal table of Win32 error strings (MESSAGE_RESOURCE_ENTRY equivalents).
/// cmd.exe prints these via its own internal wording anyway; we just need
/// non-empty strings so callers don't treat a 0-length result as a failure.
fn win32_error_string(code: u32) -> Option<&'static str> {
    Some(match code {
        0    => "The operation completed successfully.",
        1    => "Incorrect function.",
        2    => "The system cannot find the file specified.",
        3    => "The system cannot find the path specified.",
        4    => "The system cannot open the file.",
        5    => "Access is denied.",
        6    => "The handle is invalid.",
        7    => "The storage control blocks were destroyed.",
        8    => "Not enough storage is available to process this command.",
        9    => "The storage control block address is invalid.",
        10   => "The environment is incorrect.",
        11   => "An attempt was made to load a program with an incorrect format.",
        12   => "The access code is invalid.",
        13   => "The data is invalid.",
        14   => "Not enough storage is available to complete this operation.",
        15   => "The system cannot find the drive specified.",
        16   => "The directory cannot be removed.",
        17   => "The system cannot move the file to a different disk drive.",
        18   => "There are no more files.",
        19   => "The media is write protected.",
        20   => "The system cannot find the device specified.",
        32   => "The process cannot access the file because it is being used by another process.",
        33   => "The process cannot access the file because another process has locked a portion of the file.",
        80   => "The file exists.",
        87   => "The parameter is incorrect.",
        122  => "The data area passed to a system call is too small.",
        183  => "Cannot create a file when that file already exists.",
        203  => "The system could not find the environment option that was entered.",
        206  => "The filename or extension is too long.",
        232  => "The pipe is being closed.",
        995  => "The I/O operation has been aborted because of either a thread exit or an application request.",
        997  => "Overlapped I/O operation is in progress.",
        1004 => "Invalid flags.",
        1168 => "Element not found.",
        1392 => "The file or directory is corrupted and unreadable.",
        1450 => "Insufficient system resources exist to complete the requested service.",
        3221225477 => "Access violation.",  // 0xC0000005
        _    => return None,
    })
}

/// Apply basic %1 .. %9 argument insertion from the `Arguments` va_list pointer.
/// We read at most 9 dword-wide pointers (each a pointer to a C string) and
/// substitute %1..%9.  If IGNORE_INSERTS is set, or Arguments is NULL, we just
/// pass the template through unchanged.
fn apply_inserts(template: &str, flags: u32, arg_ptr: u32, ctx: &ApiContext, wide: bool) -> String {
    let ignore = flags & FORMAT_MESSAGE_IGNORE_INSERTS != 0;
    // Read the insert argument array (up to 9 DWORDs). Each may be an integer
    // value (for %n!d!) or a pointer to a string (for %n!s! / bare %n).
    let raw: Vec<u32> = (0..9)
        .map(|i| ctx.memory.read_u32(arg_ptr + i * 4).unwrap_or(0))
        .collect();

    let chars: Vec<char> = template.chars().collect();
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // Escape/control sequences (always processed).
        match chars.get(i + 1).copied() {
            Some('%') => { out.push('%'); i += 2; continue; }
            Some('n') | Some('r') => { out.push_str("\r\n"); i += 2; continue; }
            Some('t') => { out.push('\t'); i += 2; continue; }
            Some('b') => { out.push(' '); i += 2; continue; }
            Some('0') => break, // %0: end of message, no trailing newline
            Some(d) if d.is_ascii_digit() => {
                let n = (d as u8 - b'1') as usize; // %1 -> index 0
                i += 2;
                // Optional !printf-spec! (e.g. !d!, !u!, !s!, !x!).
                let mut spec = String::new();
                if chars.get(i) == Some(&'!') {
                    i += 1;
                    while i < chars.len() && chars[i] != '!' { spec.push(chars[i]); i += 1; }
                    if i < chars.len() { i += 1; } // closing !
                }
                if ignore { continue; }
                let val = raw.get(n).copied().unwrap_or(0);
                let kind = spec.chars().rev().find(|c| c.is_ascii_alphabetic());
                match kind {
                    Some('d') | Some('i') => out.push_str(&(val as i32).to_string()),
                    Some('u') => out.push_str(&val.to_string()),
                    Some('x') => out.push_str(&format!("{val:x}")),
                    Some('X') => out.push_str(&format!("{val:X}")),
                    Some('c') => out.push(val as u8 as char),
                    _ => {
                        // string insert: val is a pointer
                        out.push_str(&if wide { ctx.memory.read_wstr(val) } else { ctx.memory.read_cstr(val) });
                    }
                }
                continue;
            }
            _ => { out.push('%'); i += 1; continue; }
        }
    }
    out
}

/// Shared core: build the formatted string and write it to lpBuffer.
fn format_message_core(ctx: &mut ApiContext, wide: bool) -> u32 {
    let flags   = ctx.arg(0);
    let source  = ctx.arg(1);
    let msg_id  = ctx.arg(2);
    // arg(3) = language id — ignored, we always produce en-US
    let lp_buf  = ctx.arg(4);
    let n_size  = ctx.arg(5);
    let arg_ptr = ctx.arg(6);

    // Build the message text.
    let text: String = if flags & FORMAT_MESSAGE_FROM_STRING != 0 {
        // lpSource is a pointer to the format string.
        let template = if wide { ctx.memory.read_wstr(source) } else { ctx.memory.read_cstr(source) };
        apply_inserts(&template, flags, arg_ptr, ctx, wide)
    } else if flags & FORMAT_MESSAGE_FROM_HMODULE != 0 && ctx.messages.contains_key(&msg_id) {
        // Module message-table resource (cmd.exe banner/messages/output).
        let template = ctx.messages.get(&msg_id).cloned().unwrap_or_default();
        apply_inserts(&template, flags, arg_ptr, ctx, wide)
    } else if flags & FORMAT_MESSAGE_FROM_SYSTEM != 0 {
        let raw = win32_error_string(msg_id)
            .unwrap_or("Unknown error.")
            .to_string();
        let with_ins = apply_inserts(&raw, flags, arg_ptr, ctx, wide);
        // Windows appends "\r\n" to system messages.
        format!("{}\r\n", with_ins.trim_end_matches(|c| c == '\r' || c == '\n'))
    } else {
        // Unsupported flags — return empty (caller checks len).
        return 0;
    };

    if text.is_empty() { return 0; }
    let char_count = text.chars().count() as u32;

    if flags & FORMAT_MESSAGE_ALLOCATE_BUFFER != 0 {
        // Allocate a heap buffer, write pointer to *lpBuffer.
        let byte_len = if wide { char_count * 2 + 2 } else { char_count + 1 };
        let p = ctx.heap_alloc(byte_len);
        if wide {
            let encoded: Vec<u8> = text.encode_utf16()
                .flat_map(|c| c.to_le_bytes()).collect();
            let _ = ctx.memory.write_bytes(p, &encoded);
            let _ = ctx.memory.write_u16(p + char_count * 2, 0);
        } else {
            let _ = ctx.memory.write_bytes(p, text.as_bytes());
            let _ = ctx.memory.write_u8(p + char_count, 0);
        }
        if lp_buf != 0 { let _ = ctx.memory.write_u32(lp_buf, p); }
        char_count
    } else if lp_buf != 0 && n_size > 0 {
        let limit = (n_size as usize).min(text.chars().count() + 1);
        if wide {
            let encoded: Vec<u8> = text.encode_utf16()
                .take(limit.saturating_sub(1))
                .flat_map(|c| c.to_le_bytes()).collect();
            let written = (encoded.len() / 2) as u32;
            let _ = ctx.memory.write_bytes(lp_buf, &encoded);
            let _ = ctx.memory.write_u16(lp_buf + written * 2, 0);
            written
        } else {
            let bytes = text.as_bytes();
            let n = (limit.saturating_sub(1)).min(bytes.len());
            let _ = ctx.memory.write_bytes(lp_buf, &bytes[..n]);
            let _ = ctx.memory.write_u8(lp_buf + n as u32, 0);
            n as u32
        }
    } else {
        0
    }
}

fn format_message_a(ctx: &mut ApiContext) -> Handled {
    let r = format_message_core(ctx, false);
    ctx.ret_stdcall(r, 7);
    Handled::Ok
}

fn format_message_w(ctx: &mut ApiContext) -> Handled {
    let r = format_message_core(ctx, true);
    ctx.ret_stdcall(r, 7);
    Handled::Ok
}
