use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::vm::handles::{
    KernelObject, CURRENT_PROCESS, CURRENT_THREAD, INVALID_HANDLE, STD_ERROR_HANDLE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use webwine_api::winapi::context::ApiRuntimeEnv;

// Win32 error codes used by the file APIs.
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_INVALID_HANDLE: u32 = 6;
const ERROR_FILE_EXISTS: u32 = 80;
const ERROR_PROC_NOT_FOUND: u32 = 127;
const ERROR_ALREADY_EXISTS: u32 = 183;
const INVALID_FILE_SIZE: u32 = 0xFFFF_FFFF;

const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const INVALID_FILE_ATTRIBUTES: u32 = 0xFFFF_FFFF;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        // advapi32 registry: we have no registry,
        ("kernel32.dll", "ExitProcess", exit_process),
        ("kernel32.dll", "#99", r0_2),
        ("kernel32.dll", "GetStdHandle", get_std_handle),
        ("kernel32.dll", "WriteFile", write_file),
        ("kernel32.dll", "WriteConsoleA", write_console_a),
        ("kernel32.dll", "WriteConsoleW", write_console_w),
        ("kernel32.dll", "ReadFile", read_file),
        ("kernel32.dll", "CloseHandle", close_handle),
        ("kernel32.dll", "GetLastError", get_last_error),
        ("kernel32.dll", "SetLastError", set_last_error),
        // Obsolete MS-DOS-compat handle-limit knob; real Windows just echoes
        // the requested count back.
        ("kernel32.dll", "SetHandleCount", |c| {
            let n = c.arg(0);
            c.ret_stdcall(n, 1);
            Handled::Ok
        }),
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
        ("kernel32.dll", "VirtualQuery", virtual_query),
        ("kernel32.dll", "GetModuleHandleA", get_module_handle_a),
        ("kernel32.dll", "GetModuleHandleW", get_module_handle_w),
        ("kernel32.dll", "GetModuleFileNameA", get_module_filename_a),
        ("kernel32.dll", "GetModuleFileNameW", get_module_filename_w),
        ("kernel32.dll", "GetTempPathA", get_temp_path_a),
        ("kernel32.dll", "GetTempPathW", get_temp_path_w),
        ("kernel32.dll", "GetTempFileNameA", get_temp_file_name_a),
        ("kernel32.dll", "GetTempFileNameW", get_temp_file_name_w),
        (
            "kernel32.dll",
            "SetCurrentDirectoryA",
            set_current_directory_a,
        ),
        (
            "kernel32.dll",
            "SetCurrentDirectoryW",
            set_current_directory_w,
        ),
        ("kernel32.dll", "GetProcAddress", get_proc_address),
        ("kernel32.dll", "SetErrorMode", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "lstrlenA", |c| {
            let n = c.cstr(c.arg(0)).len() as u32;
            c.ret_stdcall(n, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "lstrlenW", |c| {
            let n = c.wstr(c.arg(0)).encode_utf16().count() as u32;
            c.ret_stdcall(n, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "lstrcpyA", lstrcpy_a),
        ("kernel32.dll", "lstrcpyW", lstrcpy_w),
        ("kernel32.dll", "lstrcpynA", lstrcpyn_a),
        ("kernel32.dll", "lstrcpynW", lstrcpyn_w),
        ("kernel32.dll", "lstrcatA", lstrcat_a),
        ("kernel32.dll", "lstrcatW", lstrcat_w),
        ("kernel32.dll", "lstrcmpA", lstrcmp_a),
        ("kernel32.dll", "lstrcmpW", lstrcmp_w),
        ("kernel32.dll", "lstrcmpiA", lstrcmpi_a),
        ("kernel32.dll", "lstrcmpiW", lstrcmpi_w),
        (
            "kernel32.dll",
            "GetPrivateProfileStringA",
            get_private_profile_string_a,
        ),
        (
            "kernel32.dll",
            "GetPrivateProfileStringW",
            get_private_profile_string_w,
        ),
        ("kernel32.dll", "SetProcessShutdownParameters", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        // UI language / locale (cmd.exe resolves these via GetProcAddress). 0x409 = en-US.
        ("kernel32.dll", "SetThreadUILanguage", |c| {
            let l = c.arg(0);
            c.ret_stdcall(if l == 0 { 0x409 } else { l }, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "GetThreadUILanguage", |c| {
            c.ret_stdcall(0x409, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetUserDefaultUILanguage", |c| {
            c.ret_stdcall(0x409, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetSystemDefaultUILanguage", |c| {
            c.ret_stdcall(0x409, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetUserDefaultLangID", |c| {
            c.ret_stdcall(0x409, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetSystemDefaultLangID", |c| {
            c.ret_stdcall(0x409, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetUserDefaultLCID", |c| {
            c.ret_stdcall(0x409, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetSystemDefaultLCID", |c| {
            c.ret_stdcall(0x409, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetLocaleInfoA", |c| {
            get_locale_info(c, false)
        }),
        ("kernel32.dll", "GetLocaleInfoW", |c| {
            get_locale_info(c, true)
        }),
        ("kernel32.dll", "GetThreadLocale", |c| {
            c.ret_stdcall(0x409, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "SetThreadLocale", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        // Report Windows XP (5.1 build 2600). GetVersion: (build<<16)|(minor<<8)|major.
        ("kernel32.dll", "GetVersion", |c| {
            c.ret_stdcall(0x0A28_0105, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetVersionExA", get_version_ex),
        ("kernel32.dll", "GetVersionExW", get_version_ex),
        ("kernel32.dll", "GetTimeZoneInformation", get_time_zone_information),
        ("kernel32.dll", "LoadLibraryA", load_library_a),
        ("kernel32.dll", "LoadLibraryW", load_library_w),
        ("kernel32.dll", "LoadLibraryExA", |c| {
            let n = c.cstr(c.arg(0));
            c.logs.log(
                webwine_api::logs::LogLevel::Trace,
                "api",
                &format!("LoadLibraryExA {n:?}"),
                Some(c.pid),
            );
            c.ret_stdcall(FAKE_MODULE, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "LoadLibraryExW", |c| {
            let n = c.wstr(c.arg(0));
            c.logs.log(
                webwine_api::logs::LogLevel::Trace,
                "api",
                &format!("LoadLibraryExW {n:?}"),
                Some(c.pid),
            );
            c.ret_stdcall(FAKE_MODULE, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "FreeLibrary", r1_1),
        ("kernel32.dll", "IsDebuggerPresent", r0_0),
        (
            "kernel32.dll",
            "IsProcessorFeaturePresent",
            is_processor_feature_present,
        ),
        ("kernel32.dll", "InitializeSListHead", r0_1),
        ("kernel32.dll", "QueryDepthSList", r0_1),
        ("kernel32.dll", "InterlockedPushEntrySList", r0_2),
        ("kernel32.dll", "InterlockedFlushSList", r0_1),
        // GetProcessAffinityMask(hProc, *procMask, *sysMask): both masks must be
        // written; leaving them untouched made callers read stack garbage (or 0,
        // i.e. "no CPUs") when sizing thread pools.
        ("kernel32.dll", "GetProcessAffinityMask", |c| {
            for a in [1u32, 2] {
                if c.arg(a) != 0 {
                    let _ = c.memory.write_u32(c.arg(a), 1);
                }
            }
            c.ret_stdcall(1, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "GetNativeSystemInfo", get_system_info),
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
        ("kernel32.dll", "GlobalMemoryStatus", global_memory_status),
        (
            "kernel32.dll",
            "GlobalMemoryStatusEx",
            global_memory_status_ex,
        ),
        (
            "kernel32.dll",
            "GetCurrentProcessId",
            get_current_process_id,
        ),
        ("kernel32.dll", "GetCurrentThreadId", get_current_thread_id),
        ("kernel32.dll", "GetCurrentProcess", get_current_process),
        ("kernel32.dll", "GetCurrentThread", get_current_thread),
        ("kernel32.dll", "GetSystemInfo", get_system_info),
        ("kernel32.dll", "GetSystemTimeAsFileTime", get_system_time),
        ("kernel32.dll", "GetSystemTime", get_system_time_struct),
        ("kernel32.dll", "GetLocalTime", get_system_time_struct),
        ("kernel32.dll", "GetComputerNameA", get_computer_name_a),
        ("kernel32.dll", "GetComputerNameW", get_computer_name_w),
        (
            "kernel32.dll",
            "QueryPerformanceCounter",
            query_perf_counter,
        ),
        ("kernel32.dll", "QueryPerformanceFrequency", query_perf_freq),
        ("kernel32.dll", "GetTickCount", get_tick_count),
        ("kernel32.dll", "GetTickCount64", get_tick_count64),
        ("kernel32.dll", "FlushFileBuffers", r1_1),
        ("kernel32.dll", "SetUnhandledExceptionFilter", r0_1),
        ("kernel32.dll", "UnhandledExceptionFilter", r0_1),
        ("kernel32.dll", "GetEnvironmentVariableW", get_env_var_w),
        ("kernel32.dll", "GetEnvironmentVariableA", get_env_var_a),
        ("kernel32.dll", "AreFileApisANSI", |c| {
            c.ret_stdcall(1, 0);
            Handled::Ok
        }),
        // Secure DLL search-path APIs (putty et al. resolve these dynamically).
        ("kernel32.dll", "SetDefaultDllDirectories", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "AddDllDirectory", |c| {
            c.ret_stdcall(0x44_0001, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "RemoveDllDirectory", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "SetDllDirectoryW", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "SetDllDirectoryA", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "SetSearchPathMode", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        // GetSystemDirectory/GetWindowsDirectory(lpBuffer, uSize) â€” 2 args.
        ("kernel32.dll", "GetSystemDirectoryA", |c| {
            sysdir(c, false, "C:\\Windows\\System32")
        }),
        ("kernel32.dll", "GetSystemDirectoryW", |c| {
            sysdir(c, true, "C:\\Windows\\System32")
        }),
        ("kernel32.dll", "GetWindowsDirectoryA", |c| {
            sysdir(c, false, "C:\\Windows")
        }),
        ("kernel32.dll", "GetWindowsDirectoryW", |c| {
            sysdir(c, true, "C:\\Windows")
        }),
        ("kernel32.dll", "GetSystemWindowsDirectoryW", |c| {
            sysdir(c, true, "C:\\Windows")
        }),
        (
            "kernel32.dll",
            "ExpandEnvironmentStringsW",
            expand_env_strings_w,
        ),
        (
            "kernel32.dll",
            "ExpandEnvironmentStringsA",
            expand_env_strings_a,
        ),
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
        (
            "kernel32.dll",
            "GetVolumeInformationW",
            get_volume_information_w,
        ),
        (
            "kernel32.dll",
            "GetVolumeInformationA",
            get_volume_information_a,
        ),
        (
            "kernel32.dll",
            "FileTimeToSystemTime",
            file_time_to_system_time,
        ),
        (
            "kernel32.dll",
            "FileTimeToLocalFileTime",
            file_time_to_local_file_time,
        ),
        (
            "kernel32.dll",
            "SystemTimeToFileTime",
            system_time_to_file_time,
        ),
        (
            "kernel32.dll",
            "LocalFileTimeToFileTime",
            file_time_to_local_file_time,
        ),
        ("kernel32.dll", "GetDateFormatW", |c| {
            date_time_format(c, true, true)
        }),
        ("kernel32.dll", "GetDateFormatA", |c| {
            date_time_format(c, false, true)
        }),
        ("kernel32.dll", "GetTimeFormatW", |c| {
            date_time_format(c, true, false)
        }),
        ("kernel32.dll", "GetTimeFormatA", |c| {
            date_time_format(c, false, false)
        }),
        ("kernel32.dll", "GetDiskFreeSpaceW", |c| {
            // (root, *sectorsPerCluster, *bytesPerSector, *freeClusters, *totalClusters)
            if c.arg(1) != 0 {
                let _ = c.memory.write_u32(c.arg(1), 8);
            }
            if c.arg(2) != 0 {
                let _ = c.memory.write_u32(c.arg(2), 512);
            }
            if c.arg(3) != 0 {
                let _ = c.memory.write_u32(c.arg(3), 0x10000);
            }
            if c.arg(4) != 0 {
                let _ = c.memory.write_u32(c.arg(4), 0x20000);
            }
            c.ret_stdcall(1, 5);
            Handled::Ok
        }),
        ("kernel32.dll", "GetDiskFreeSpaceExW", |c| {
            // (dir, *freeAvail(u64), *total(u64), *totalFree(u64))
            for a in [1u32, 2, 3] {
                let p = c.arg(a);
                if p != 0 {
                    let _ = c.memory.write_u32(p, 0x4000_0000);
                    let _ = c.memory.write_u32(p + 4, 0);
                }
            }
            c.ret_stdcall(1, 4);
            Handled::Ok
        }),
        ("kernel32.dll", "GetEnvironmentStringsW", |c| {
            let p = env_block(c, true);
            c.ret_stdcall(p, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetEnvironmentStrings", |c| {
            let p = env_block(c, false);
            c.ret_stdcall(p, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "FreeEnvironmentStringsW", r1_1),
        ("kernel32.dll", "GetEnvironmentStringsA", |c| {
            let p = env_block(c, false);
            c.ret_stdcall(p, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "FreeEnvironmentStringsA", r1_1),
        ("kernel32.dll", "InitializeCriticalSection", k32_init_cs),
        (
            "kernel32.dll",
            "InitializeCriticalSectionAndSpinCount",
            k32_init_cs_spin,
        ),
        ("kernel32.dll", "InitializeCriticalSectionEx", k32_init_cs_ex),
        ("kernel32.dll", "DeleteCriticalSection", k32_delete_cs),
        ("kernel32.dll", "EnterCriticalSection", k32_enter_cs),
        ("kernel32.dll", "LeaveCriticalSection", k32_leave_cs),
        ("kernel32.dll", "TryEnterCriticalSection", k32_try_enter_cs),
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
        (
            "kernel32.dll",
            "GetConsoleScreenBufferInfo",
            get_console_screen_buffer_info,
        ),
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
        ("kernel32.dll", "MulDiv", mul_div),
        ("kernel32.dll", "GetACP", |c| {
            c.ret_stdcall(1252, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "GetOEMCP", |c| {
            c.ret_stdcall(437, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "IsValidCodePage", r1_1),
        ("kernel32.dll", "GetCPInfo", get_cp_info),
        ("kernel32.dll", "LCMapStringW", |c| lcmap_string(c, true, 6)),
        ("kernel32.dll", "LCMapStringA", |c| {
            lcmap_string(c, false, 6)
        }),
        // LCMapStringEx(name, flags, src, srclen, dst, dstlen, version,
        // reserved, sortHandle) - same conversion, 9 args, wide only.
        ("kernel32.dll", "LCMapStringEx", |c| {
            lcmap_string(c, true, 9)
        }),
        ("kernel32.dll", "CreateFileA", create_file_a),
        ("kernel32.dll", "CreateFileW", create_file_w),
        ("kernel32.dll", "GetFileSize", get_file_size),
        ("kernel32.dll", "GetFileSizeEx", get_file_size_ex),
        ("kernel32.dll", "GetFileTime", get_file_time),
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
        ("kernel32.dll", "GetStringTypeW", get_string_type_w),
        ("kernel32.dll", "GetStringTypeExW", get_string_type_ex_w),
        ("kernel32.dll", "GetStringTypeA", get_string_type_a),
        ("kernel32.dll", "GetStringTypeExA", get_string_type_a),
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
        ("kernel32.dll", "WaitForSingleObjectEx", r0_3), // (handle, ms, alertable)
        ("kernel32.dll", "WaitForMultipleObjects", r0_4), // (n, handles, all, ms)
        ("kernel32.dll", "WaitForMultipleObjectsEx", r0_5),
        ("kernel32.dll", "SignalObjectAndWait", r0_4),
        ("kernel32.dll", "CreateEventA", |c| {
            c.ret_stdcall(0xE700_0001, 4);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateEventW", |c| {
            c.ret_stdcall(0xE700_0001, 4);
            Handled::Ok
        }),
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
        // API-MS-WIN API set forwarders
        // ApiSetQueryApiSetPresence(ApiSetName, Present*) â€” checks if an API set
        // DLL is present.  We say "no" (FALSE) so callers skip optional features.
        (
            "api-ms-win-core-apiquery-l1-1-0.dll",
            "ApiSetQueryApiSetPresence",
            |c| {
                let p = c.arg(1);
                if p != 0 {
                    let _ = c.memory.write_u32(p, 0);
                }
                c.ret_stdcall(0, 2);
                Handled::Ok
            },
        ),
        // GlobalAlloc / GlobalFree / GlobalLock / GlobalUnlock â€” thin wrappers
        // around the process heap; we just forward to our heap routines.
        ("kernel32.dll", "GlobalAlloc", global_alloc),
        // LocalAlloc(uFlags, uBytes) â€” same as GlobalAlloc in our model.
        ("kernel32.dll", "LocalAlloc", global_alloc),
        ("kernel32.dll", "LocalFree", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "LocalLock", |c| {
            let p = c.arg(0);
            c.ret_stdcall(p, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "LocalUnlock", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "LocalReAlloc", |c| {
            let p = c.arg(0);
            let n = c.arg(1);
            let r = c.heap_realloc(p, n);
            c.ret_stdcall(r, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "LocalSize", local_global_size),
        ("kernel32.dll", "SetPriorityClass", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("kernel32.dll", "SetThreadPriority", r1_2),
        ("kernel32.dll", "SetProcessDEPPolicy", r1_1),
        ("kernel32.dll", "HeapSetInformation", r1_4),
        ("kernel32.dll", "OpenEventA", |c| { c.ret_stdcall(0xE700_0001, 3); Handled::Ok }),
        ("kernel32.dll", "OpenEventW", |c| { c.ret_stdcall(0xE700_0001, 3); Handled::Ok }),
        ("kernel32.dll", "CreateThread", create_thread),
        ("kernel32.dll", "RegisterApplicationRestart", |c| { c.ret_stdcall(0, 2); Handled::Ok }),
        ("kernel32.dll", "GlobalAddAtomA", global_add_atom_a),
        ("kernel32.dll", "GlobalAddAtomW", global_add_atom_w),
        ("kernel32.dll", "GlobalFindAtomA", global_find_atom_a),
        ("kernel32.dll", "GlobalFindAtomW", global_find_atom_w),
        ("kernel32.dll", "GlobalDeleteAtom", global_delete_atom),
        ("kernel32.dll", "GlobalGetAtomNameA", global_get_atom_name_a),
        ("kernel32.dll", "GlobalGetAtomNameW", global_get_atom_name_w),
        ("kernel32.dll", "AddAtomA", global_add_atom_a),
        ("kernel32.dll", "AddAtomW", global_add_atom_w),
        ("kernel32.dll", "FindAtomA", global_find_atom_a),
        ("kernel32.dll", "FindAtomW", global_find_atom_w),
        ("kernel32.dll", "DeleteAtom", global_delete_atom),
        ("kernel32.dll", "GetAtomNameA", global_get_atom_name_a),
        ("kernel32.dll", "GetAtomNameW", global_get_atom_name_w),
        // Registry value query (W10 explorer). 7 args; report not-found.
        ("kernel32.dll", "RegGetValueW", |c| {
            if c.arg(6) != 0 {
                let _ = c.memory.write_u32(c.arg(6), 0);
            }
            c.ret_stdcall(2, 7);
            Handled::Ok
        }),
        ("kernel32.dll", "RegGetValueA", |c| {
            if c.arg(6) != 0 {
                let _ = c.memory.write_u32(c.arg(6), 0);
            }
            c.ret_stdcall(2, 7);
            Handled::Ok
        }),
        // Named mutexes / events with the extended (4-arg) variants -> fake handle.
        ("kernel32.dll", "CreateMutexW", |c| {
            // Freshly created (we own it): GetLastError must read ERROR_SUCCESS so
            // single-instance apps treat us as the primary instance, not a duplicate.
            c.set_last_error(0);
            c.ret_stdcall(0x4D54_0002, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateMutexA", |c| {
            c.set_last_error(0);
            c.ret_stdcall(0x4D54_0002, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateMutexExW", |c| {
            c.ret_stdcall(0x4D54_0001, 4);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateMutexExA", |c| {
            c.ret_stdcall(0x4D54_0001, 4);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateEventExW", |c| {
            c.ret_stdcall(0x4576_0001, 4);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateSemaphoreExW", |c| {
            c.ret_stdcall(0x5365_0001, 6);
            Handled::Ok
        }),
        ("kernel32.dll", "ReleaseMutex", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        // Condition variables (explorer's CRT/threadpool resolve these dynamically).
        ("kernel32.dll", "InitializeConditionVariable", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "WakeConditionVariable", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "WakeAllConditionVariable", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "SleepConditionVariableCS", |c| {
            c.ret_stdcall(1, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "SleepConditionVariableSRW", |c| {
            c.ret_stdcall(1, 4);
            Handled::Ok
        }),
        // Threadpool timers/work -> fake handles, no-op (we have no real threads).
        ("kernel32.dll", "CreateThreadpoolTimer", |c| {
            c.ret_stdcall(0x5450_0001, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "SetThreadpoolTimer", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("kernel32.dll", "CloseThreadpoolTimer", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "WaitForThreadpoolTimerCallbacks", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateThreadpoolWork", |c| {
            c.ret_stdcall(0x5450_0002, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "SubmitThreadpoolWork", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "CloseThreadpoolWork", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "WaitForThreadpoolWorkCallbacks", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("kernel32.dll", "RegisterTraceGuidsW", |c| {
            c.ret_stdcall(0, 8);
            Handled::Ok
        }),
        ("kernel32.dll", "RegisterTraceGuidsA", |c| {
            c.ret_stdcall(0, 8);
            Handled::Ok
        }),
        ("kernel32.dll", "UnregisterTraceGuids", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "GetTraceLoggerHandle", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "TraceMessage", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateThreadpoolWait", |c| {
            c.ret_stdcall(0x5450_0003, 3);
            Handled::Ok
        }),
        ("kernel32.dll", "GetPriorityClass", |c| {
            c.ret_stdcall(0x20, 1);
            Handled::Ok
        }),
        // Activation contexts (theming/manifests): no-op stubs so explorer's
        // dynamic resolve + calls succeed instead of returning NULL.
        ("kernel32.dll", "CreateActCtxW", |c| {
            c.ret_stdcall(0xAC70_0001, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "CreateActCtxA", |c| {
            c.ret_stdcall(0xAC70_0001, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "ActivateActCtx", |c| {
            let o = c.arg(1);
            if o != 0 {
                let _ = c.memory.write_u32(o, 1);
            }
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("kernel32.dll", "DeactivateActCtx", |c| {
            c.ret_stdcall(1, 2);
            Handled::Ok
        }),
        ("kernel32.dll", "ReleaseActCtx", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "AddRefActCtx", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "GlobalFree", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "GlobalLock", |c| {
            let p = c.arg(0);
            c.ret_stdcall(p, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "GlobalUnlock", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "GlobalHandle", |c| {
            let p = c.arg(0);
            c.ret_stdcall(p, 1);
            Handled::Ok
        }),
        ("kernel32.dll", "GlobalSize", local_global_size),
        (
            "api-ms-win-core-heap-l2-1-0.dll",
            "GlobalAlloc",
            global_alloc,
        ),
        ("api-ms-win-core-heap-l2-1-0.dll", "GlobalFree", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("api-ms-win-core-heap-l2-1-0.dll", "GlobalLock", |c| {
            let p = c.arg(0);
            c.ret_stdcall(p, 1);
            Handled::Ok
        }),
        ("api-ms-win-core-heap-l2-1-0.dll", "GlobalUnlock", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        // FormatMessageW forwarded through the localization API set.
        (
            "api-ms-win-core-localization-l1-2-0.dll",
            "FormatMessageW",
            format_message_w_fwd,
        ),
        (
            "api-ms-win-core-localization-l1-2-0.dll",
            "FormatMessageA",
            format_message_a_fwd,
        ),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn write_ansi_z(c: &mut ApiContext, dst: u32, value: &str) {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    let _ = c.memory.write_bytes(dst, &bytes);
}

fn write_wide_z(c: &mut ApiContext, dst: u32, value: &str) {
    let mut bytes = Vec::with_capacity((value.len() + 1) * 2);
    for unit in value.encode_utf16().chain(std::iter::once(0)) {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let _ = c.memory.write_bytes(dst, &bytes);
}

fn get_temp_path_a(c: &mut ApiContext) -> Handled {
    let path = "C:\\Temp\\";
    let cap = c.arg(0) as usize;
    if c.arg(1) != 0 && cap > path.len() { write_ansi_z(c, c.arg(1), path); }
    c.ret_stdcall(path.len() as u32, 2);
    Handled::Ok
}

fn get_temp_path_w(c: &mut ApiContext) -> Handled {
    let path = "C:\\Temp\\";
    let len = path.encode_utf16().count();
    if c.arg(1) != 0 && (c.arg(0) as usize) > len { write_wide_z(c, c.arg(1), path); }
    c.ret_stdcall(len as u32, 2);
    Handled::Ok
}

fn get_temp_file_name_a(c: &mut ApiContext) -> Handled {
    let dir = c.cstr(c.arg(0));
    let prefix = c.cstr(c.arg(1));
    let unique = if c.arg(2) == 0 {
        *c.rand_seed = c.rand_seed.wrapping_add(1);
        (*c.rand_seed & 0xFFFF).max(1)
    } else { c.arg(2) & 0xFFFF };
    let separator = if dir.ends_with(['\\', '/']) { "" } else { "\\" };
    let name = format!("{dir}{separator}{}{unique:04X}.tmp", prefix.chars().take(3).collect::<String>());
    write_ansi_z(c, c.arg(3), &name);
    c.ret_stdcall(unique, 4);
    Handled::Ok
}

fn get_temp_file_name_w(c: &mut ApiContext) -> Handled {
    let dir = c.wstr(c.arg(0));
    let prefix = c.wstr(c.arg(1));
    let unique = if c.arg(2) == 0 {
        *c.rand_seed = c.rand_seed.wrapping_add(1);
        (*c.rand_seed & 0xFFFF).max(1)
    } else { c.arg(2) & 0xFFFF };
    let separator = if dir.ends_with(['\\', '/']) { "" } else { "\\" };
    let name = format!("{dir}{separator}{}{unique:04X}.tmp", prefix.chars().take(3).collect::<String>());
    write_wide_z(c, c.arg(3), &name);
    c.ret_stdcall(unique, 4);
    Handled::Ok
}

fn create_thread(c: &mut ApiContext) -> Handled {
    let thread_id = c.next_child_pid.max(2);
    if c.arg(5) != 0 { let _ = c.memory.write_u32(c.arg(5), thread_id); }
    c.ret_stdcall(0x7A00_0000 | (thread_id & 0xFFFF), 6);
    Handled::Ok
}

fn get_file_time(c: &mut ApiContext) -> Handled {
    for index in 1..=3 {
        let out = c.arg(index);
        if out != 0 { let _ = c.memory.write_bytes(out, &[0; 8]); }
    }
    c.ret_stdcall(1, 4);
    Handled::Ok
}

fn get_time_zone_information(c: &mut ApiContext) -> Handled {
    let out = c.arg(0);
    if out != 0 { let _ = c.memory.write_bytes(out, &vec![0; 172]); }
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn lstrcpy_a(c: &mut ApiContext) -> Handled {
    let dst = c.arg(0);
    let value = c.cstr(c.arg(1));
    write_ansi_z(c, dst, &value);
    c.ret_stdcall(dst, 2);
    Handled::Ok
}

fn lstrcpy_w(c: &mut ApiContext) -> Handled {
    let dst = c.arg(0);
    let value = c.wstr(c.arg(1));
    write_wide_z(c, dst, &value);
    c.ret_stdcall(dst, 2);
    Handled::Ok
}

fn lstrcpyn_a(c: &mut ApiContext) -> Handled {
    let dst = c.arg(0);
    let max = c.arg(2) as usize;
    let value = c.cstr(c.arg(1));
    let mut end = max.saturating_sub(1).min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    if max > 0 { write_ansi_z(c, dst, &value[..end]); }
    c.ret_stdcall(dst, 3);
    Handled::Ok
}

fn lstrcpyn_w(c: &mut ApiContext) -> Handled {
    let dst = c.arg(0);
    let max = c.arg(2) as usize;
    let value = c.wstr(c.arg(1));
    if max > 0 {
        let truncated: String = value.chars().take(max - 1).collect();
        write_wide_z(c, dst, &truncated);
    }
    c.ret_stdcall(dst, 3);
    Handled::Ok
}

fn lstrcat_a(c: &mut ApiContext) -> Handled {
    let dst = c.arg(0);
    let value = c.cstr(dst) + &c.cstr(c.arg(1));
    write_ansi_z(c, dst, &value);
    c.ret_stdcall(dst, 2);
    Handled::Ok
}

fn lstrcat_w(c: &mut ApiContext) -> Handled {
    let dst = c.arg(0);
    let value = c.wstr(dst) + &c.wstr(c.arg(1));
    write_wide_z(c, dst, &value);
    c.ret_stdcall(dst, 2);
    Handled::Ok
}

fn lstrcmp_a(c: &mut ApiContext) -> Handled {
    let result = c.cstr(c.arg(0)).cmp(&c.cstr(c.arg(1))) as i32;
    c.ret_stdcall(result as u32, 2);
    Handled::Ok
}

fn lstrcmp_w(c: &mut ApiContext) -> Handled {
    let result = c.wstr(c.arg(0)).cmp(&c.wstr(c.arg(1))) as i32;
    c.ret_stdcall(result as u32, 2);
    Handled::Ok
}

fn lstrcmpi_a(c: &mut ApiContext) -> Handled {
    let result = c.cstr(c.arg(0)).to_lowercase().cmp(&c.cstr(c.arg(1)).to_lowercase()) as i32;
    c.ret_stdcall(result as u32, 2);
    Handled::Ok
}

fn lstrcmpi_w(c: &mut ApiContext) -> Handled {
    let result = c.wstr(c.arg(0)).to_lowercase().cmp(&c.wstr(c.arg(1)).to_lowercase()) as i32;
    c.ret_stdcall(result as u32, 2);
    Handled::Ok
}

// PathFindFileName(path): pointer to the last component.
//
// Matching Wine (dlls/kernelbase/path.c, PathFindFileNameW): ':' counts as a
// separator, and a separator only advances the result when it is followed by a
// character that is not itself a separator. So "C:\dir\" returns the whole
// string rather than a pointer to the empty tail, and "C:file" returns "file".
pub(crate) fn path_find_file_name_a(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let mut last_slash = p;
    let mut curr = p;
    loop {
        let b = ctx.memory.read_u8(curr).unwrap_or(0);
        if b == 0 {
            break;
        }
        let next = ctx.memory.read_u8(curr + 1).unwrap_or(0);
        if matches!(b, b'\\' | b'/' | b':') && next != 0 && next != b'\\' && next != b'/' {
            last_slash = curr + 1;
        }
        curr += 1;
    }
    ctx.ret_stdcall(last_slash, 1);
    Handled::Ok
}

pub(crate) fn path_find_file_name_w(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let mut last_slash = p;
    let mut curr = p;
    loop {
        let w = ctx.memory.read_u16(curr).unwrap_or(0);
        if w == 0 {
            break;
        }
        let next = ctx.memory.read_u16(curr + 2).unwrap_or(0);
        let sep = |c: u16| c == '\\' as u16 || c == '/' as u16;
        if (sep(w) || w == ':' as u16) && next != 0 && !sep(next) {
            last_slash = curr + 2;
        }
        curr += 2;
    }
    ctx.ret_stdcall(last_slash, 1);
    Handled::Ok
}

pub(crate) fn strcmp_ni_a(ctx: &mut ApiContext) -> Handled {
    let n = ctx.arg(2) as usize;
    let a = ctx
        .cstr(ctx.arg(0))
        .chars()
        .take(n)
        .collect::<String>()
        .to_lowercase();
    let b = ctx
        .cstr(ctx.arg(1))
        .chars()
        .take(n)
        .collect::<String>()
        .to_lowercase();
    ctx.ret_stdcall(a.cmp(&b) as i32 as u32, 3);
    Handled::Ok
}

pub(crate) fn strcmp_ni_w(ctx: &mut ApiContext) -> Handled {
    let n = ctx.arg(2) as usize;
    let a = ctx
        .wstr(ctx.arg(0))
        .chars()
        .take(n)
        .collect::<String>()
        .to_lowercase();
    let b = ctx
        .wstr(ctx.arg(1))
        .chars()
        .take(n)
        .collect::<String>()
        .to_lowercase();
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

pub(crate) fn sh_reg_get_bool_us_value(ctx: &mut ApiContext) -> Handled {
    let default = ctx.arg(3);
    ctx.ret_stdcall(default, 4);
    Handled::Ok
}

pub(crate) fn sh_reg_create_us_key(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(3);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0x5A5A_0001);
    }
    ctx.ret_stdcall(0, 5);
    Handled::Ok
}

pub(crate) fn sh_create_thread_ref(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    if out != 0 {
        let _ = ctx.memory.write_u32(out, 0);
    }
    ctx.ret_stdcall(0x8000_4005, 2);
    Handled::Ok
}

pub(crate) fn sh_get_special_folder_path_a(ctx: &mut ApiContext) -> Handled {
    let out = ctx.arg(1);
    let path = b"C:\\Users\\guest";
    if out != 0 {
        let _ = ctx.memory.write_bytes(out, path);
        let _ = ctx.memory.write_u8(out + path.len() as u32, 0);
    }
    ctx.ret_stdcall(1, 4);
    Handled::Ok
}

pub(crate) fn sh_get_special_folder_path_w(ctx: &mut ApiContext) -> Handled {
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

// GlobalSize / LocalSize(hMem) -> block size, 0 if the handle is unknown.
// GlobalAlloc/LocalAlloc hand out plain heap pointers here, so the recorded
// allocation size is the answer.
fn local_global_size(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    let n = ctx.heap_sizes.get(&p).copied().unwrap_or(0);
    ctx.ret_stdcall(n, 1);
    Handled::Ok
}

fn global_alloc(ctx: &mut ApiContext) -> Handled {
    // GlobalAlloc(uFlags, dwBytes) â€” allocate `dwBytes` from the heap.
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

    let bytes = ctx
        .memory
        .read_bytes(buf, (count * 2) as usize)
        .unwrap_or_default();
    let u16s: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
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

// ReadConsoleW(hInput, lpBuffer, nChars, lpNumRead, pInputControl) â€” wide.
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

// ReadConsoleA(hInput, lpBuffer, nChars, lpNumRead, pInputControl) â€” narrow.
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
const OPEN_ALWAYS: u32 = 4;
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
    // Wine (dlls/kernelbase/file.c, CreateFileW): a *successful* CREATE_ALWAYS
    // that overwrote a file, or OPEN_ALWAYS that opened one, still reports
    // ERROR_ALREADY_EXISTS; every other success clears the error. Callers use
    // this to tell "created" from "reused" without a second stat.
    ctx.cpu.last_error = if exists && (disposition == CREATE_ALWAYS || disposition == OPEN_ALWAYS) {
        ERROR_ALREADY_EXISTS
    } else {
        0
    };
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

/// Byte length of the VFS file behind `handle`, if it is a file handle.
fn handle_file_size(ctx: &ApiContext, handle: u32) -> Option<u64> {
    match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { path, .. }) => {
            Some(ctx.fs.read_file(path).map(|b| b.len()).unwrap_or(0) as u64)
        }
        _ => None,
    }
}

// GetFileSize(hFile, lpFileSizeHigh) -> low dword, INVALID_FILE_SIZE on error.
fn get_file_size(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let high = ctx.arg(1);
    let Some(size) = handle_file_size(ctx, handle) else {
        ctx.cpu.last_error = ERROR_INVALID_HANDLE;
        ctx.ret_stdcall(INVALID_FILE_SIZE, 2);
        return Handled::Ok;
    };
    if high != 0 {
        let _ = ctx.memory.write_u32(high, (size >> 32) as u32);
    }
    ctx.ret_stdcall(size as u32, 2);
    Handled::Ok
}

// GetFileSizeEx(hFile, PLARGE_INTEGER) -> BOOL. A different shape from
// GetFileSize: the 64-bit size goes to the out parameter and the return value
// is a success flag (dlls/kernelbase/file.c). Aliasing it onto GetFileSize made
// every caller read the size as a boolean and leave *lpFileSize untouched.
fn get_file_size_ex(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let out = ctx.arg(1);
    let Some(size) = handle_file_size(ctx, handle) else {
        ctx.cpu.last_error = ERROR_INVALID_HANDLE;
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    };
    if out != 0 {
        let _ = ctx.memory.write_u32(out, size as u32);
        let _ = ctx.memory.write_u32(out + 4, (size >> 32) as u32);
    }
    ctx.ret_stdcall(1, 2);
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
        ctx.logs.log(
            webwine_api::logs::LogLevel::Trace,
            "api",
            &format!("CreateDirectory {path:?} -> already exists"),
            Some(ctx.pid),
        );
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    }
    let result = ctx.fs.create_dir(&path);
    let ok = result.is_ok();
    if let Err(e) = &result {
        // ERROR_PATH_NOT_FOUND: a real CreateDirectory only makes the leaf
        // component, same as here — a missing ancestor is a genuine failure,
        // not something we should silently work around.
        ctx.cpu.last_error = 3;
        ctx.logs.log(
            webwine_api::logs::LogLevel::Trace,
            "api",
            &format!("CreateDirectory {path:?} -> failed: {e}"),
            Some(ctx.pid),
        );
    }
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
                let is_dir = matches!(e.kind, webwine_api::fs::vfs::EntryKind::Directory);
                out.push((e.name, is_dir, e.size));
            }
        }
    }
    out
}

// Fill a WIN32_FIND_DATAW at `p` for one entry (wide cFileName at +44).
fn fill_find_data_w(ctx: &mut ApiContext, p: u32, name: &str, is_dir: bool, size: u64) {
    let attrs = if is_dir {
        FILE_ATTRIBUTE_DIRECTORY
    } else {
        FILE_ATTRIBUTE_NORMAL
    };
    let _ = ctx.memory.write_bytes(p, &[0u8; 44]); // attrs + 3 FILETIMEs + sizes + reserved
    let _ = ctx.memory.write_u32(p, attrs);
    let _ = ctx.memory.write_u32(p + 28, (size >> 32) as u32);
    let _ = ctx.memory.write_u32(p + 32, size as u32);
    let mut wide: Vec<u8> = name
        .encode_utf16()
        .take(259)
        .flat_map(|c| c.to_le_bytes())
        .collect();
    wide.extend_from_slice(&[0, 0]);
    let _ = ctx.memory.write_bytes(p + 44, &wide);
}

// Same, ANSI cFileName for WIN32_FIND_DATAA (cFileName at +44, bytes).
fn fill_find_data_a(ctx: &mut ApiContext, p: u32, name: &str, is_dir: bool, size: u64) {
    let attrs = if is_dir {
        FILE_ATTRIBUTE_DIRECTORY
    } else {
        FILE_ATTRIBUTE_NORMAL
    };
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
    if wide {
        fill_find_data_w(ctx, data, &name, is_dir, size);
    } else {
        fill_find_data_a(ctx, data, &name, is_dir, size);
    }
    ctx.handles
        .insert(KernelObject::FindHandle { matches, cursor: 1 })
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
            if wide {
                fill_find_data_w(ctx, data, &name, is_dir, size);
            } else {
                fill_find_data_a(ctx, data, &name, is_dir, size);
            }
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
//                      *fsFlags, fsNameBuf, fsNameSize) â€” 8 args.
fn get_volume_info(ctx: &mut ApiContext, wide: bool) -> Handled {
    let vol_buf = ctx.arg(1);
    let serial = ctx.arg(3);
    let max_comp = ctx.arg(4);
    let fs_flags = ctx.arg(5);
    let fs_buf = ctx.arg(6);
    let write = |ctx: &mut ApiContext, p: u32, s: &str| {
        if p == 0 {
            return;
        }
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
    if serial != 0 {
        let _ = ctx.memory.write_u32(serial, 0x1234_ABCD);
    }
    if max_comp != 0 {
        let _ = ctx.memory.write_u32(max_comp, 255);
    }
    if fs_flags != 0 {
        let _ = ctx.memory.write_u32(fs_flags, 0);
    }
    ctx.ret_stdcall(1, 8);
    Handled::Ok
}

fn get_volume_information_w(ctx: &mut ApiContext) -> Handled {
    get_volume_info(ctx, true)
}
fn get_volume_information_a(ctx: &mut ApiContext) -> Handled {
    get_volume_info(ctx, false)
}

// Convert a FILETIME (100ns since 1601) to civil (year, month, day, dow, h, m, s).
fn filetime_to_civil(ft: u64) -> (u16, u16, u16, u16, u16, u16, u16) {
    let secs = (ft / 10_000_000) as i64;
    let days_1601 = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let (h, mi, s) = (
        (tod / 3600) as u16,
        ((tod % 3600) / 60) as u16,
        (tod % 60) as u16,
    );
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

// FileTimeToLocalFileTime(lpFileTime, lpLocalFileTime): no timezone â€” copy.
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

// Days since 1970-01-01 for a civil date (Howard Hinnant's days_from_civil,
// the exact inverse of the civil_from_days used by `filetime_to_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

// SystemTimeToFileTime(lpSystemTime, lpFileTime): actually convert the caller's
// SYSTEMTIME. Emitting a constant made every date the app supplied (file times,
// timestamps it had just built with GetSystemTime) collapse onto one value, so
// date arithmetic and sorting silently produced garbage.
fn system_time_to_file_time(ctx: &mut ApiContext) -> Handled {
    let st = ctx.arg(0);
    let out = ctx.arg(1);
    let rd = |ctx: &ApiContext, off: u32| ctx.memory.read_u16(st + off).unwrap_or(0) as i64;
    let (year, month, day) = (rd(ctx, 0), rd(ctx, 2), rd(ctx, 6));
    let (hour, min, sec, ms) = (rd(ctx, 8), rd(ctx, 10), rd(ctx, 12), rd(ctx, 14));

    if st == 0 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        ctx.cpu.last_error = ERROR_INVALID_PARAMETER;
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    }

    // 1601-01-01 -> 1970-01-01 is 134774 days; FILETIME ticks are 100 ns.
    let days = days_from_civil(year, month, day) + 134_774;
    let secs = days * 86400 + hour * 3600 + min * 60 + sec;
    let ft = (secs.max(0) as u64) * 10_000_000 + (ms.max(0) as u64) * 10_000;

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
        let (h12, ap) = if h == 0 {
            (12, "AM")
        } else if h < 12 {
            (h, "AM")
        } else if h == 12 {
            (12, "PM")
        } else {
            (h - 12, "PM")
        };
        format!("{:02}:{:02} {}", h12, rd(ctx, 10), ap)
    };
    let n = if wide {
        let units: Vec<u16> = text.encode_utf16().collect();
        let w = if cch > 0 {
            units.len().min(cch as usize - 1)
        } else {
            units.len()
        };
        if buf != 0 {
            let mut b: Vec<u8> = units[..w].iter().flat_map(|u| u.to_le_bytes()).collect();
            b.extend_from_slice(&[0, 0]);
            let _ = ctx.memory.write_bytes(buf, &b);
        }
        w + 1
    } else {
        let bytes = text.as_bytes();
        let w = if cch > 0 {
            bytes.len().min(cch as usize - 1)
        } else {
            bytes.len()
        };
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
//                cwd, startupInfo, processInfo) â€” 10 args.
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
        .push(webwine_api::vm::process::SpawnRequest { path, pi_addr: pi });
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
        let v = ctx.proc_address("", &name);
        ctx.logs.log(
            webwine_api::logs::LogLevel::Trace,
            "api",
            &format!("GetProcAddress {name:?} -> 0x{v:08X}"),
            Some(ctx.pid),
        );
        if v == 0 {
            // Surface misses at warn — delay-load helpers raise 0xC06D007F next.
            ctx.logs.log(
                webwine_api::logs::LogLevel::Warn,
                "api",
                &format!("GetProcAddress miss: {name}"),
                Some(ctx.pid),
            );
        }
        v
    };
    if va == 0 {
        ctx.set_last_error(ERROR_PROC_NOT_FOUND);
    } else {
        ctx.set_last_error(0);
    }
    ctx.return_stdcall(va, 2);
    Handled::Ok
}

// Beep(dwFreq, dwDuration) â€” emit a UI beep for the frontend (Web Audio).
fn beep(ctx: &mut ApiContext) -> Handled {
    let freq = ctx.arg(0);
    let duration = ctx.arg(1);
    ctx.ui_events
        .push(webwine_api::vm::process::UiEvent::Beep { freq, duration });
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// GetFullPathNameW(lpFileName, nBufferLength, lpBuffer, lpFilePart) â€” resolve
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

const HEAP_ZERO_MEMORY: u32 = 0x0000_0008;

fn heap_alloc(ctx: &mut ApiContext) -> Handled {
    // HeapAlloc(hHeap, dwFlags, dwBytes) — 3 args stdcall.
    let flags = ctx.arg(1);
    let size = ctx.arg(2);
    let ptr = if flags & HEAP_ZERO_MEMORY != 0 {
        ctx.heap_alloc_zeroed(size)
    } else {
        ctx.heap_alloc(size)
    };
    if ptr == 0 {
        ctx.cpu.last_error = 8; // ERROR_NOT_ENOUGH_MEMORY
    }
    ctx.ret_stdcall(ptr, 3);
    Handled::Ok
}

fn heap_free(ctx: &mut ApiContext) -> Handled {
    // HeapFree(hHeap, dwFlags, lpMem) — return the block to the free list so
    // subsequent HeapAlloc can reuse it (critical for game asset load loops).
    let p = ctx.arg(2);
    ctx.heap_free_block(p);
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

fn heap_realloc(ctx: &mut ApiContext) -> Handled {
    // HeapReAlloc(hHeap, dwFlags, lpMem, dwBytes)
    let flags = ctx.arg(1);
    let old = ctx.arg(2);
    let size = ctx.arg(3);
    let prev_size = ctx.heap_sizes.get(&old).copied().unwrap_or(0);
    let ptr = ctx.heap_realloc(old, size);
    if ptr == 0 {
        ctx.cpu.last_error = 8; // ERROR_NOT_ENOUGH_MEMORY
    } else if flags & HEAP_ZERO_MEMORY != 0 && size > prev_size {
        // Zero the grown tail. In-place growth is already zeroed by heap_realloc;
        // a moved block's fresh pages are zero from ensure_mapped — still write
        // explicitly so the documented flag is never a silent no-op.
        let _ = ctx.memory.write_bytes(
            ptr.wrapping_add(prev_size),
            &vec![0u8; (size - prev_size) as usize],
        );
    }
    ctx.ret_stdcall(ptr, 4);
    Handled::Ok
}

// HeapSize(hHeap, dwFlags, lpMem) -> allocated size, (SIZE_T)-1 on failure.
// The bump allocator records every block size, so answer from it instead of
// reporting 0 (which reads as a valid zero-byte block, not as an error).
fn heap_size(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(2);
    let n = ctx.heap_sizes.get(&p).copied().unwrap_or(0xFFFF_FFFF);
    ctx.ret_stdcall(n, 3);
    Handled::Ok
}

fn mul_div(ctx: &mut ApiContext) -> Handled {
    let number = ctx.arg(0) as i32 as i64;
    let numerator = ctx.arg(1) as i32 as i64;
    let denominator = ctx.arg(2) as i32 as i64;
    let result = if denominator == 0 { -1 } else { (number * numerator) / denominator };
    ctx.ret_stdcall(result as i32 as u32, 3);
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

/// VirtualQuery(lpAddress, &mbi, dwLength) — fills a MEMORY_BASIC_INFORMATION
/// (28 bytes on x86) and returns the number of bytes written (0x1C), or 0 if
/// the buffer is too small. The MSVC CRT's __scrt_is_nonwritable_in_current_image
/// __fastfails when this returns 0, so the previous stub (always 0) crashed any
/// app built with a recent toolchain (e.g. Windows Media Player) during startup.
fn virtual_query(ctx: &mut ApiContext) -> Handled {
    // Map our PageProt bits to the Win32 PAGE_* protection constants.
    fn to_win_prot(bits: u32) -> u32 {
        let (r, w, x) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
        match (x, w, r) {
            (true, true, _) => 0x40,       // PAGE_EXECUTE_READWRITE
            (true, false, true) => 0x20,   // PAGE_EXECUTE_READ
            (true, false, false) => 0x10,  // PAGE_EXECUTE
            (false, true, _) => 0x04,      // PAGE_READWRITE
            (false, false, true) => 0x02,  // PAGE_READONLY
            (false, false, false) => 0x01, // PAGE_NOACCESS
        }
    }

    let addr = ctx.arg(0);
    let buf = ctx.arg(1);
    let len = ctx.arg(2);
    if len < 0x1C {
        ctx.ret_stdcall(0, 3);
        return Handled::Ok;
    }
    let page = addr & !0xFFF;

    // Region containing addr, if any (Copy fields → immutable borrow ends here).
    let mapped = ctx
        .memory
        .regions
        .range(..=addr)
        .next_back()
        .filter(|(_, r)| addr < r.base.wrapping_add(r.size))
        .map(|(_, r)| (r.base, r.base.wrapping_add(r.size), r.prot.bits()));

    let (base_addr, alloc_base, alloc_prot, region_size, state, protect, mem_type) = match mapped {
        Some((base, end, bits)) => {
            let p = to_win_prot(bits);
            (
                page,
                base,
                p,
                end.wrapping_sub(page),
                0x1000u32, /*MEM_COMMIT*/
                p,
                0x20000u32, /*MEM_PRIVATE*/
            )
        }
        None => {
            // Free hole: RegionSize spans up to the next mapped region.
            let next = ctx
                .memory
                .regions
                .range(addr..)
                .next()
                .map(|(&b, _)| b)
                .unwrap_or(0);
            let size = if next > page { next - page } else { 0x1000 };
            (
                page, 0, 0, size, 0x10000u32, /*MEM_FREE*/
                0x01u32,    /*PAGE_NOACCESS*/
                0,
            )
        }
    };

    let _ = ctx.memory.write_u32(buf, base_addr);
    let _ = ctx.memory.write_u32(buf + 0x04, alloc_base);
    let _ = ctx.memory.write_u32(buf + 0x08, alloc_prot);
    let _ = ctx.memory.write_u32(buf + 0x0C, region_size);
    let _ = ctx.memory.write_u32(buf + 0x10, state);
    let _ = ctx.memory.write_u32(buf + 0x14, protect);
    let _ = ctx.memory.write_u32(buf + 0x18, mem_type);
    ctx.ret_stdcall(0x1C, 3);
    Handled::Ok
}

// ── Global / local atom table ──────────────────────────────────────────────
// Lightweight string atoms for RegisterClass / DDE / OLE. Stored per-process
// in dll_state: "atom.name.<lower>" → atom id, "atom.id.<id>" → name length
// packed with a heap pointer is overkill; we keep name strings in heap and
// map both directions via dll_state keys.

const ATOM_BASE: u32 = 0xC000; // first dynamic string atom (Win32 convention)
const ATOM_NEXT_KEY: &str = "atom.next";

fn atom_next(ctx: &mut ApiContext) -> u32 {
    let n = ctx.dll_state.get(ATOM_NEXT_KEY).copied().unwrap_or(ATOM_BASE);
    ctx.dll_state.insert(ATOM_NEXT_KEY.to_string(), n.wrapping_add(1));
    n
}

fn atom_key_name(name: &str) -> String {
    format!("atom.name.{}", name.to_ascii_lowercase())
}

fn atom_key_id(id: u32) -> String {
    format!("atom.id.{id}")
}

fn global_add_atom_a(ctx: &mut ApiContext) -> Handled {
    let s = ctx.cstr(ctx.arg(0));
    global_add_atom(ctx, &s)
}

fn global_add_atom_w(ctx: &mut ApiContext) -> Handled {
    let s = ctx.wstr(ctx.arg(0));
    global_add_atom(ctx, &s)
}

fn global_add_atom(ctx: &mut ApiContext, name: &str) -> Handled {
    if name.is_empty() {
        ctx.set_last_error(ERROR_INVALID_PARAMETER);
        ctx.ret_stdcall(0, 1);
        return Handled::Ok;
    }
    let key = atom_key_name(name);
    if let Some(&id) = ctx.dll_state.get(&key) {
        // Existing atom — bump a fake refcount slot (id+ref stored? just return).
        ctx.ret_stdcall(id, 1);
        return Handled::Ok;
    }
    let id = atom_next(ctx);
    // Store name bytes in the process heap so GetAtomName can read them back.
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    let ptr = ctx.heap_alloc(bytes.len() as u32);
    if ptr != 0 {
        let _ = ctx.memory.write_bytes(ptr, &bytes);
        ctx.dll_state.insert(key, id);
        ctx.dll_state.insert(atom_key_id(id), ptr);
        // Refcount (starts at 1) living as a side key.
        ctx.dll_state.insert(format!("atom.ref.{id}"), 1);
        ctx.ret_stdcall(id, 1);
    } else {
        ctx.set_last_error(8); // ERROR_NOT_ENOUGH_MEMORY
        ctx.ret_stdcall(0, 1);
    }
    Handled::Ok
}

fn global_find_atom_a(ctx: &mut ApiContext) -> Handled {
    let s = ctx.cstr(ctx.arg(0));
    global_find_atom(ctx, &s)
}

fn global_find_atom_w(ctx: &mut ApiContext) -> Handled {
    let s = ctx.wstr(ctx.arg(0));
    global_find_atom(ctx, &s)
}

fn global_find_atom(ctx: &mut ApiContext, name: &str) -> Handled {
    let id = ctx
        .dll_state
        .get(&atom_key_name(name))
        .copied()
        .unwrap_or(0);
    if id == 0 {
        ctx.set_last_error(ERROR_FILE_NOT_FOUND); // ERROR_FILE_NOT_FOUND is what Win32 uses
    }
    ctx.ret_stdcall(id, 1);
    Handled::Ok
}

/// GlobalDeleteAtom / DeleteAtom: free one reference. 0 on success, nAtom on fail.
fn global_delete_atom(ctx: &mut ApiContext) -> Handled {
    let atom = ctx.arg(0) & 0xFFFF;
    if atom == 0 {
        ctx.set_last_error(ERROR_INVALID_HANDLE);
        ctx.ret_stdcall(atom, 1);
        return Handled::Ok;
    }
    let ref_key = format!("atom.ref.{atom}");
    match ctx.dll_state.get(&ref_key).copied() {
        Some(r) if r > 1 => {
            ctx.dll_state.insert(ref_key, r - 1);
            ctx.ret_stdcall(0, 1);
        }
        Some(_) => {
            // Last reference — drop tables. Leave the name heap block (harmless).
            ctx.dll_state.remove(&ref_key);
            if let Some(ptr) = ctx.dll_state.remove(&atom_key_id(atom)) {
                // Recover the name to drop the reverse map.
                let name = ctx.cstr(ptr);
                ctx.dll_state.remove(&atom_key_name(&name));
                ctx.heap_free_block(ptr);
            }
            ctx.ret_stdcall(0, 1);
        }
        None => {
            // Unknown atom: still report success for integer atoms / stubs that
            // never went through AddAtom (old GlobalAddAtom always returned 1).
            ctx.ret_stdcall(0, 1);
        }
    }
    Handled::Ok
}

fn global_get_atom_name_a(ctx: &mut ApiContext) -> Handled {
    let atom = ctx.arg(0) & 0xFFFF;
    let buf = ctx.arg(1);
    let size = ctx.arg(2); // bytes including null
    let n = write_atom_name_a(ctx, atom, buf, size);
    ctx.ret_stdcall(n, 3);
    Handled::Ok
}

fn global_get_atom_name_w(ctx: &mut ApiContext) -> Handled {
    let atom = ctx.arg(0) & 0xFFFF;
    let buf = ctx.arg(1);
    let size = ctx.arg(2); // WCHARs including null
    let n = write_atom_name_w(ctx, atom, buf, size);
    ctx.ret_stdcall(n, 3);
    Handled::Ok
}

fn write_atom_name_a(ctx: &mut ApiContext, atom: u32, buf: u32, size: u32) -> u32 {
    let Some(&ptr) = ctx.dll_state.get(&atom_key_id(atom)) else {
        ctx.set_last_error(ERROR_INVALID_HANDLE);
        return 0;
    };
    let name = ctx.cstr(ptr);
    if buf == 0 || size == 0 {
        return name.len() as u32;
    }
    let n = name.len().min(size.saturating_sub(1) as usize);
    let _ = ctx.memory.write_bytes(buf, &name.as_bytes()[..n]);
    let _ = ctx.memory.write_u8(buf + n as u32, 0);
    n as u32
}

fn write_atom_name_w(ctx: &mut ApiContext, atom: u32, buf: u32, size: u32) -> u32 {
    let Some(&ptr) = ctx.dll_state.get(&atom_key_id(atom)) else {
        ctx.set_last_error(ERROR_INVALID_HANDLE);
        return 0;
    };
    let name = ctx.cstr(ptr);
    let wide: Vec<u16> = name.encode_utf16().collect();
    if buf == 0 || size == 0 {
        return wide.len() as u32;
    }
    let n = wide.len().min(size.saturating_sub(1) as usize);
    for (i, &ch) in wide.iter().take(n).enumerate() {
        let _ = ctx.memory.write_u16(buf + (i as u32) * 2, ch);
    }
    let _ = ctx.memory.write_u16(buf + (n as u32) * 2, 0);
    n as u32
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

// GetModuleHandleEx(flags, name, &out_module) â€” write image base, return TRUE
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
        if k.eq_ignore_ascii_case(name) {
            Some(val)
        } else {
            None
        }
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

pub(crate) fn expand_env_strings_w(ctx: &mut ApiContext) -> Handled {
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
        } else {
            len + 1
        }
    } else {
        let bytes = path.as_bytes();
        let len = bytes.len() as u32;
        if buf != 0 && size > len {
            let _ = ctx.memory.write_bytes(buf, bytes);
            let _ = ctx.memory.write_u8(buf + len, 0);
            len
        } else {
            len + 1
        }
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
        None => {
            ctx.cpu.last_error = ERROR_ENVVAR_NOT_FOUND;
            ctx.ret_stdcall(0, 3);
        }
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
        None => {
            ctx.cpu.last_error = ERROR_ENVVAR_NOT_FOUND;
            ctx.ret_stdcall(0, 3);
        }
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
// STARTF_USESTDHANDLES â†’ the CRT falls back to GetStdHandle) and set cb.
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

// FILETIME (100ns since 1601-01-01) for a fixed base date, advanced by the
// shared virtual clock so successive reads differ. Base = 2024-01-01 00:00 UTC.
fn fake_filetime() -> u64 {
    BASE_2024_FILETIME + crate::winmm::tick_ms() as u64 * 10_000
}

/// 2024-01-01 00:00:00 UTC as a FILETIME, the base every clock we expose shares.
const BASE_2024_FILETIME: u64 = 133_485_408_000_000_000;
/// The same instant as a Unix time_t.
const BASE_2024_UNIX: u32 = 1_704_067_200;

/// Seconds since the Unix epoch, advancing with the same virtual clock as
/// GetSystemTimeAsFileTime and GetTickCount so the CRT's `time()` and the
/// Win32 clocks cannot disagree (and `srand(time(NULL))` gets a fresh seed).
pub(crate) fn unix_time_secs() -> u32 {
    BASE_2024_UNIX + crate::winmm::tick_ms() / 1000
}

// GetSystemTimeAsFileTime / GetSystemTimePreciseAsFileTime(LPFILETIME): write a
// plausible, monotonically-advancing 64-bit FILETIME (was all-zero = year 1601,
// which breaks elapsed-time math and RNG seeding).
fn get_system_time(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let ft = fake_filetime();
        let _ = ctx.memory.write_u32(p, ft as u32);
        let _ = ctx.memory.write_u32(p + 4, (ft >> 32) as u32);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

// GetSystemTime / GetLocalTime(LPSYSTEMTIME): fill the 16-byte SYSTEMTIME with a
// fixed, valid date (2024-01-01 12:00:00, a Monday). We have no real clock.
fn get_system_time_struct(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        // wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds
        let ms = (crate::winmm::tick_ms() % 1000) as u16;
        for (off, v) in [
            (0, 2024u16),
            (2, 1),
            (4, 1),
            (6, 1),
            (8, 12),
            (10, 0),
            (12, 0),
            (14, ms),
        ] {
            let _ = ctx.memory.write_u16(p + off, v);
        }
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

// GlobalMemoryStatus(LPMEMORYSTATUS): 32-bit fields, so report ~2 GB to avoid
// overflow. Reports 25% load, 2 GB phys (1.5 GB free), 2 GB virtual.
fn global_memory_status(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        const G2: u32 = 2 * 1024 * 1024 * 1024 - 1; // ~2 GB (fits in u32)
        let _ = ctx.memory.write_u32(p, 32); // dwLength
        let _ = ctx.memory.write_u32(p + 4, 25); // dwMemoryLoad
        let _ = ctx.memory.write_u32(p + 8, G2); // dwTotalPhys
        let _ = ctx.memory.write_u32(p + 12, G2 / 4 * 3); // dwAvailPhys (~1.5 GB)
        let _ = ctx.memory.write_u32(p + 16, G2); // dwTotalPageFile
        let _ = ctx.memory.write_u32(p + 20, G2 / 4 * 3); // dwAvailPageFile
        let _ = ctx.memory.write_u32(p + 24, G2); // dwTotalVirtual
        let _ = ctx.memory.write_u32(p + 28, G2 / 4 * 3); // dwAvailVirtual
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

// GlobalMemoryStatusEx(LPMEMORYSTATUSEX): 64-bit fields. Reports 4 GB phys.
fn global_memory_status_ex(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let write_u64 = |c: &mut ApiContext, off: u32, v: u64| {
            let _ = c.memory.write_u32(p + off, v as u32);
            let _ = c.memory.write_u32(p + off + 4, (v >> 32) as u32);
        };
        const G4: u64 = 4 * 1024 * 1024 * 1024;
        // dwLength (+0) is set by the caller; leave it. dwMemoryLoad (+4).
        let _ = ctx.memory.write_u32(p + 4, 25);
        write_u64(ctx, 8, G4); // ullTotalPhys
        write_u64(ctx, 16, G4 / 4 * 3); // ullAvailPhys
        write_u64(ctx, 24, G4); // ullTotalPageFile
        write_u64(ctx, 32, G4 / 4 * 3); // ullAvailPageFile
        write_u64(ctx, 40, G4); // ullTotalVirtual
        write_u64(ctx, 48, G4 / 4 * 3); // ullAvailVirtual
        write_u64(ctx, 56, 0); // ullAvailExtendedVirtual
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// GetSystemInfo / GetNativeSystemInfo(LPSYSTEM_INFO): fill the 36-byte struct so
// apps reading dwPageSize / dwNumberOfProcessors / dwAllocationGranularity /
// address bounds get sane values instead of garbage.
fn get_system_info(ctx: &mut ApiContext) -> Handled {
    let p = ctx.arg(0);
    if p != 0 {
        let _ = ctx.memory.write_u16(p, 0); // wProcessorArchitecture = INTEL
        let _ = ctx.memory.write_u16(p + 2, 0); // wReserved
        let _ = ctx.memory.write_u32(p + 4, 0x1000); // dwPageSize = 4 KB
        let _ = ctx.memory.write_u32(p + 8, 0x0001_0000); // lpMinimumApplicationAddress
        let _ = ctx.memory.write_u32(p + 12, 0x7FFE_FFFF); // lpMaximumApplicationAddress
        let _ = ctx.memory.write_u32(p + 16, 1); // dwActiveProcessorMask
        let _ = ctx.memory.write_u32(p + 20, 1); // dwNumberOfProcessors
        let _ = ctx.memory.write_u32(p + 24, 586); // dwProcessorType = PENTIUM
        let _ = ctx.memory.write_u32(p + 28, 0x0001_0000); // dwAllocationGranularity = 64 KB
        let _ = ctx.memory.write_u16(p + 32, 6); // wProcessorLevel
        let _ = ctx.memory.write_u16(p + 34, 0x0E08); // wProcessorRevision
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

// GetComputerName(lpBuffer, lpnSize): write "WEBWINE" + update the size in/out.
fn get_computer_name_a(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(0);
    let size_ptr = ctx.arg(1);
    let name = b"WEBWINE";
    if buf != 0 {
        let _ = ctx.memory.write_bytes(buf, name);
        let _ = ctx.memory.write_u8(buf + name.len() as u32, 0);
    }
    if size_ptr != 0 {
        let _ = ctx.memory.write_u32(size_ptr, name.len() as u32);
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

fn get_computer_name_w(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(0);
    let size_ptr = ctx.arg(1);
    let name = "WEBWINE";
    if buf != 0 {
        for (i, c) in name.encode_utf16().enumerate() {
            let _ = ctx.memory.write_u16(buf + i as u32 * 2, c);
        }
        let _ = ctx.memory.write_u16(buf + name.len() as u32 * 2, 0);
    }
    if size_ptr != 0 {
        let _ = ctx.memory.write_u32(size_ptr, name.len() as u32);
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

// GetUserName(lpBuffer, lpnSize): write "guest" + the size INCLUDING the null
// (per the Win32 contract), unlike GetComputerName.
pub(crate) fn get_user_name_a(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(0);
    let size_ptr = ctx.arg(1);
    let name = b"guest";
    if buf != 0 {
        let _ = ctx.memory.write_bytes(buf, name);
        let _ = ctx.memory.write_u8(buf + name.len() as u32, 0);
    }
    if size_ptr != 0 {
        let _ = ctx.memory.write_u32(size_ptr, name.len() as u32 + 1);
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

pub(crate) fn get_user_name_w(ctx: &mut ApiContext) -> Handled {
    let buf = ctx.arg(0);
    let size_ptr = ctx.arg(1);
    let name = "guest";
    if buf != 0 {
        for (i, c) in name.encode_utf16().enumerate() {
            let _ = ctx.memory.write_u16(buf + i as u32 * 2, c);
        }
        let _ = ctx.memory.write_u16(buf + name.len() as u32 * 2, 0);
    }
    if size_ptr != 0 {
        let _ = ctx.memory.write_u32(size_ptr, name.len() as u32 + 1);
    }
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

fn query_perf_counter(ctx: &mut ApiContext) -> Handled {
    // Frequency is reported as 1 MHz, so the counter is in microseconds.
    let p = ctx.arg(0);
    if p != 0 {
        let micros = crate::winmm::tick_ms() as u64 * 1000;
        let _ = ctx.memory.write_u32(p, micros as u32);
        let _ = ctx.memory.write_u32(p + 4, (micros >> 32) as u32);
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

fn get_tick_count(ctx: &mut ApiContext) -> Handled {
    ctx.ret_stdcall(crate::winmm::tick_ms(), 0);
    Handled::Ok
}

// GetTickCount64 returns a ULONGLONG, which x86 stdcall passes back in EDX:EAX.
// Sharing the 32-bit GetTickCount handler left EDX holding whatever the guest
// last put there, so the high dword was random and callers saw times millions
// of years apart.
fn get_tick_count64(ctx: &mut ApiContext) -> Handled {
    let ms = crate::winmm::tick_ms() as u64;
    ctx.cpu.edx = (ms >> 32) as u32;
    ctx.ret_stdcall(ms as u32, 0);
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

// GetFileType(hFile): FILE_TYPE_DISK (1) for a real file, FILE_TYPE_CHAR (2)
// for the console handles. Answering CHAR for everything told the CRT that
// every opened file was a tty, so it line-buffered and refused to seek.
fn get_file_type(ctx: &mut ApiContext) -> Handled {
    let handle = ctx.arg(0);
    let ty = match ctx.handles.get(handle) {
        Some(KernelObject::VfsFile { .. }) => 1, // FILE_TYPE_DISK
        Some(_) => 2,                            // FILE_TYPE_CHAR (console)
        None => 2,
    };
    ctx.ret_stdcall(ty, 1);
    Handled::Ok
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

// RaiseException(code, flags, argCount, args).
//
// We do not model full SEH for every code, so a genuinely fatal exception still
// terminates. Debugger-notification codes are fire-and-forget. Visual C++
// delay-load helpers raise 0xC06D007E/7F when LoadLibrary/GetProcAddress fail;
// those are meant to be caught by `__try/__except` in `__delayLoadHelper2`.
// Without a working SEH path for them the process dies on the first missing
// delay-import — return as if the exception were handled so the delay helper
// can take its failure path (NULL pfn) instead of killing the guest.
fn raise_exception(ctx: &mut ApiContext) -> Handled {
    /// MS_VC_EXCEPTION: the "name this thread" notification the MSVC debugger
    /// consumes. Always continuable, always ignorable.
    const MS_VC_THREAD_NAME: u32 = 0x406D_1388;
    /// OutputDebugString's notification exception.
    const DBG_PRINTEXCEPTION_C: u32 = 0x4001_000A;
    const DBG_PRINTEXCEPTION_WIDE_C: u32 = 0x4001_000B;
    /// MSVC C++ throw. Continuable in the sense that the app's own
    /// __CxxFrameHandler is expected to unwind it; we cannot run that, so it
    /// stays fatal, but say so explicitly in the log.
    const CXX_EXCEPTION: u32 = 0xE06D_7363;
    /// VcppException(ERROR_MOD_NOT_FOUND) — delay-load failed LoadLibrary.
    const VCPP_MOD_NOT_FOUND: u32 = 0xC06D_007E;
    /// VcppException(ERROR_PROC_NOT_FOUND) — delay-load failed GetProcAddress.
    const VCPP_PROC_NOT_FOUND: u32 = 0xC06D_007F;

    let code = ctx.arg(0);
    let flags = ctx.arg(1);
    const EXCEPTION_NONCONTINUABLE: u32 = 1;

    match code {
        MS_VC_THREAD_NAME | DBG_PRINTEXCEPTION_C | DBG_PRINTEXCEPTION_WIDE_C => {
            ctx.ret_stdcall(0, 4);
            Handled::Ok
        }
        VCPP_MOD_NOT_FOUND | VCPP_PROC_NOT_FOUND => {
            // Record the Win32 error the delay helper is signalling so subsequent
            // GetLastError reflects the miss.
            ctx.set_last_error(if code == VCPP_MOD_NOT_FOUND {
                126 // ERROR_MOD_NOT_FOUND
            } else {
                127 // ERROR_PROC_NOT_FOUND
            });
            ctx.logs.log(
                webwine_api::logs::LogLevel::Warn,
                "api",
                &format!(
                    "RaiseException 0x{code:08X} (VC++ delay-load {} not found) — continuing; check prior GetProcAddress/LoadLibrary miss",
                    if code == VCPP_MOD_NOT_FOUND { "module" } else { "proc" }
                ),
                Some(ctx.pid),
            );
            // Clean the RaiseException frame and resume; the delay-load helper's
            // __except filter is not run, but most helpers still tolerate a
            // resumed call site when the failure was already recorded via
            // GetLastError / NULL return paths.
            ctx.ret_stdcall(0, 4);
            Handled::Ok
        }
        _ => {
            let note = if code == CXX_EXCEPTION {
                " (C++ throw; SEH unwinding is not modelled, so it cannot reach a catch block)"
            } else if flags & EXCEPTION_NONCONTINUABLE == 0 {
                " (continuable, but no SEH handler chain is modelled)"
            } else {
                ""
            };
            ctx.logs.log(
                webwine_api::logs::LogLevel::Warn,
                "api",
                &format!("RaiseException code=0x{code:08X}{note}"),
                Some(ctx.pid),
            );
            Handled::ExitProcess(1)
        }
    }
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

const ERROR_INVALID_PARAMETER: u32 = 87;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;

// locale / character-classification support
//
// These four used to be blanket failure stubs (return 0). The MSVC CRT builds
// its `isalpha`/`toupper` tables and runs `setlocale` through them at startup,
// so a hard failure left the tables empty and every ctype query answering "no".

// GetCPInfo(codepage, lpCPInfo) -> BOOL.
// CPINFO { UINT MaxCharSize; BYTE DefaultChar[2]; BYTE LeadByte[12]; }
fn get_cp_info(ctx: &mut ApiContext) -> Handled {
    let cp = crate::codepage::resolve(ctx.arg(0));
    let p = ctx.arg(1);
    if p == 0 {
        ctx.cpu.last_error = ERROR_INVALID_PARAMETER;
        ctx.ret_stdcall(0, 2);
        return Handled::Ok;
    }
    // We only expose single-byte codepages plus UTF-8; neither has lead bytes.
    let max_char = if cp == 65001 { 4 } else { 1 };
    let _ = ctx.memory.write_u32(p, max_char);
    let _ = ctx.memory.write_u8(p + 4, b'?'); // DefaultChar
    let _ = ctx.memory.write_u8(p + 5, 0);
    let _ = ctx.memory.write_bytes(p + 6, &[0u8; 12]); // LeadByte: none
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

/// Shared body for GetStringType*: classify `count` units into `out`.
/// A negative count means "NUL-terminated", and Wine converts that to
/// `lstrlenW(src) + 1` so the terminator gets an entry too.
fn string_type_core(
    ctx: &mut ApiContext,
    ty: u32,
    src: u32,
    count: u32,
    out: u32,
    wide: bool,
) -> bool {
    if src == 0 || out == 0 || !(1..=3).contains(&ty) {
        ctx.cpu.last_error = ERROR_INVALID_PARAMETER;
        return false;
    }
    let units: Vec<u16> = if wide {
        if count == 0xFFFF_FFFF {
            let mut u = ctx.memory.read_wstr_units(src);
            u.push(0);
            u
        } else {
            (0..count)
                .map(|i| ctx.memory.read_u16(src + i * 2).unwrap_or(0))
                .collect()
        }
    } else {
        let cp = crate::codepage::resolve(0);
        let bytes = if count == 0xFFFF_FFFF {
            let mut b = ctx.memory.read_cstr_bytes(src);
            b.push(0);
            b
        } else {
            ctx.memory
                .read_bytes(src, count as usize)
                .unwrap_or_default()
        };
        bytes
            .iter()
            .map(|&b| crate::codepage::byte_to_wchar(cp, b))
            .collect()
    };
    for (i, &c) in units.iter().enumerate() {
        let t = match ty {
            1 => crate::codepage::char_type1(c),
            2 => crate::codepage::char_type2(c),
            _ => crate::codepage::char_type3(c),
        };
        let _ = ctx.memory.write_u16(out + (i as u32) * 2, t);
    }
    true
}

// GetStringTypeW(dwInfoType, lpSrcStr, cchSrc, lpCharType) - note the info type
// comes FIRST here, unlike every "Ex"/ANSI variant.
fn get_string_type_w(ctx: &mut ApiContext) -> Handled {
    let (ty, src, count, out) = (ctx.arg(0), ctx.arg(1), ctx.arg(2), ctx.arg(3));
    let ok = string_type_core(ctx, ty, src, count, out, true);
    ctx.ret_stdcall(ok as u32, 4);
    Handled::Ok
}

// GetStringTypeExW(locale, dwInfoType, lpSrcStr, cchSrc, lpCharType).
fn get_string_type_ex_w(ctx: &mut ApiContext) -> Handled {
    let (ty, src, count, out) = (ctx.arg(1), ctx.arg(2), ctx.arg(3), ctx.arg(4));
    let ok = string_type_core(ctx, ty, src, count, out, true);
    ctx.ret_stdcall(ok as u32, 5);
    Handled::Ok
}

// GetStringTypeA/ExA(locale, dwInfoType, lpSrcStr, cchSrc, lpCharType).
fn get_string_type_a(ctx: &mut ApiContext) -> Handled {
    let (ty, src, count, out) = (ctx.arg(1), ctx.arg(2), ctx.arg(3), ctx.arg(4));
    let ok = string_type_core(ctx, ty, src, count, out, false);
    ctx.ret_stdcall(ok as u32, 5);
    Handled::Ok
}

const LCMAP_LOWERCASE: u32 = 0x0000_0100;
const LCMAP_UPPERCASE: u32 = 0x0000_0200;
const LCMAP_SORTKEY: u32 = 0x0000_0400;

// LCMapString(W|A) / LCMapStringEx: case mapping (the flags anyone actually
// uses). Argument positions are identical for all three; only the trailing
// argument count differs. Unsupported mappings pass the text through unchanged
// rather than failing, which is what callers can recover from.
fn lcmap_string(ctx: &mut ApiContext, wide: bool, nargs: u32) -> Handled {
    let flags = ctx.arg(1);
    let src = ctx.arg(2);
    let srclen = ctx.arg(3);
    let dst = ctx.arg(4);
    let dstlen = ctx.arg(5);

    if src == 0 || (dst == 0 && dstlen != 0) {
        ctx.cpu.last_error = ERROR_INVALID_PARAMETER;
        ctx.ret_stdcall(0, nargs);
        return Handled::Ok;
    }

    let cp = crate::codepage::resolve(0);
    let units: Vec<u16> = if wide {
        wc_source(ctx, src, srclen)
    } else {
        mb_source(ctx, src, srclen)
            .iter()
            .map(|&b| crate::codepage::byte_to_wchar(cp, b))
            .collect()
    };

    let mapped: Vec<u16> = units
        .iter()
        .map(|&c| {
            if flags & LCMAP_UPPERCASE != 0 {
                crate::codepage::to_upper(c)
            } else if flags & (LCMAP_LOWERCASE | LCMAP_SORTKEY) != 0 {
                // A sort key is only ever compared against another sort key,
                // so a case-folded copy is a valid (if coarse) collation order.
                crate::codepage::to_lower(c)
            } else {
                c
            }
        })
        .collect();

    // A sort key is a byte string even in the wide entry point.
    let byte_output = !wide || flags & LCMAP_SORTKEY != 0;
    let needed = mapped.len() as u32;
    if dstlen == 0 {
        ctx.ret_stdcall(needed, nargs);
        return Handled::Ok;
    }
    if dstlen < needed {
        ctx.cpu.last_error = ERROR_INSUFFICIENT_BUFFER;
        ctx.ret_stdcall(0, nargs);
        return Handled::Ok;
    }
    if byte_output {
        let bytes = crate::codepage::encode(0, &mapped);
        let _ = ctx.memory.write_bytes(dst, &bytes);
    } else {
        for (i, &c) in mapped.iter().enumerate() {
            let _ = ctx.memory.write_u16(dst + (i as u32) * 2, c);
        }
    }
    ctx.ret_stdcall(needed, nargs);
    Handled::Ok
}

const LOCALE_RETURN_NUMBER: u32 = 0x2000_0000;

/// en-US values for the LCTYPEs apps and the CRT actually query.
fn locale_info_value(lctype: u32) -> Option<&'static str> {
    const DAYS: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    const ABBREV_DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const ABBREV_MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    Some(match lctype {
        0x0001 => "0409",                    // LOCALE_ILANGUAGE
        0x0002 => "English (United States)", // LOCALE_SLANGUAGE
        0x0003 => "ENU",                     // LOCALE_SABBREVLANGNAME
        0x0005 => "US",                      // LOCALE_ICOUNTRY
        0x0006 => "United States",           // LOCALE_SCOUNTRY
        0x0007 => "USA",                     // LOCALE_SABBREVCTRYNAME
        0x000B => "437",                     // LOCALE_IDEFAULTCODEPAGE
        0x000C => ",",                       // LOCALE_SLIST
        0x000D => "1",                       // LOCALE_IMEASURE (US)
        0x000E => ".",                       // LOCALE_SDECIMAL
        0x000F => ",",                       // LOCALE_STHOUSAND
        0x0010 => "3;0",                     // LOCALE_SGROUPING
        0x0011 => "2",                       // LOCALE_IDIGITS
        0x0014 => "$",                       // LOCALE_SCURRENCY
        0x001D => "/",                       // LOCALE_SDATE
        0x001E => ":",                       // LOCALE_STIME
        0x001F => "M/d/yyyy",                // LOCALE_SSHORTDATE
        0x0020 => "dddd, MMMM d, yyyy",      // LOCALE_SLONGDATE
        0x0028 => "AM",                      // LOCALE_S1159
        0x0029 => "PM",                      // LOCALE_S2359
        0x0050 => "",                        // LOCALE_SPOSITIVESIGN
        0x0051 => "-",                       // LOCALE_SNEGATIVESIGN
        0x0059 => "en",                      // LOCALE_SISO639LANGNAME
        0x005A => "US",                      // LOCALE_SISO3166CTRYNAME
        0x005C => "en-US",                   // LOCALE_SNAME
        0x1001 => "English",                 // LOCALE_SENGLANGUAGE
        0x1002 => "United States",           // LOCALE_SENGCOUNTRY
        0x1003 => "h:mm:ss tt",              // LOCALE_STIMEFORMAT
        0x1004 => "1252",                    // LOCALE_IDEFAULTANSICODEPAGE
        0x1010 => "1",                       // LOCALE_INEGNUMBER
        0x002A..=0x0030 => DAYS[(lctype - 0x2A) as usize],
        0x0031..=0x0037 => ABBREV_DAYS[(lctype - 0x31) as usize],
        0x0038..=0x0043 => MONTHS[(lctype - 0x38) as usize],
        0x0044..=0x004F => ABBREV_MONTHS[(lctype - 0x44) as usize],
        _ => return None,
    })
}

// GetLocaleInfo(A|W)(locale, lctype, lpLCData, cchData) -> chars written
// including the NUL, or the required size when cchData is 0. With
// LOCALE_RETURN_NUMBER the value is written as a DWORD instead.
fn get_locale_info(ctx: &mut ApiContext, wide: bool) -> Handled {
    let lctype = ctx.arg(1);
    let buf = ctx.arg(2);
    let cch = ctx.arg(3);

    let Some(value) = locale_info_value(lctype & 0x000F_FFFF) else {
        ctx.cpu.last_error = ERROR_INVALID_PARAMETER;
        ctx.ret_stdcall(0, 4);
        return Handled::Ok;
    };

    if lctype & LOCALE_RETURN_NUMBER != 0 {
        // The caller wants a DWORD; the string form is always decimal here.
        if cch < 2 {
            ctx.ret_stdcall(2, 4); // sizeof(DWORD) in "chars"
            return Handled::Ok;
        }
        if buf == 0 {
            ctx.cpu.last_error = ERROR_INVALID_PARAMETER;
            ctx.ret_stdcall(0, 4);
            return Handled::Ok;
        }
        let n = value.parse::<u32>().unwrap_or(0);
        let _ = ctx.memory.write_u32(buf, n);
        ctx.ret_stdcall(2, 4);
        return Handled::Ok;
    }

    let units: Vec<u16> = value.encode_utf16().collect();
    let needed = if wide {
        units.len() as u32 + 1
    } else {
        crate::codepage::encode(0, &units).len() as u32 + 1
    };
    if cch == 0 {
        ctx.ret_stdcall(needed, 4);
        return Handled::Ok;
    }
    if buf == 0 || cch < needed {
        ctx.cpu.last_error = ERROR_INSUFFICIENT_BUFFER;
        ctx.ret_stdcall(0, 4);
        return Handled::Ok;
    }
    if wide {
        for (i, &c) in units.iter().enumerate() {
            let _ = ctx.memory.write_u16(buf + (i as u32) * 2, c);
        }
        let _ = ctx.memory.write_u16(buf + (units.len() as u32) * 2, 0);
    } else {
        let mut bytes = crate::codepage::encode(0, &units);
        bytes.push(0);
        let _ = ctx.memory.write_bytes(buf, &bytes);
    }
    ctx.ret_stdcall(needed, 4);
    Handled::Ok
}

// IsProcessorFeaturePresent(feature): answer per feature instead of "yes" to
// everything. Claiming every PF_* bit invited the CRT and app hot paths into
// AVX/XSAVE/RDRAND code our interpreter cannot execute. The list below is what
// the executor really implements: SSE/SSE2 data movement (movaps/movdqu/pxor/
// punpck) and a non-executable-page model. cmpxchg8b, rdtsc, cpuid and MMX are
// not decoded, so they answer FALSE.
fn is_processor_feature_present(ctx: &mut ApiContext) -> Handled {
    const PF_XMMI_INSTRUCTIONS_AVAILABLE: u32 = 6;
    const PF_XMMI64_INSTRUCTIONS_AVAILABLE: u32 = 10;
    const PF_NX_ENABLED: u32 = 12;

    let present = matches!(
        ctx.arg(0),
        PF_XMMI_INSTRUCTIONS_AVAILABLE | PF_XMMI64_INSTRUCTIONS_AVAILABLE | PF_NX_ENABLED
    );
    ctx.ret_stdcall(present as u32, 1);
    Handled::Ok
}

/// Read exactly `count` bytes of source text. A negative (0xFFFFFFFF) length
/// means "NUL-terminated", and Wine turns that into `strlen(src) + 1` so the
/// terminator is part of the conversion and of the returned count
/// (dlls/kernelbase/locale.c, MultiByteToWideChar).
fn mb_source(ctx: &ApiContext, src: u32, srclen: u32) -> Vec<u8> {
    if srclen == 0xFFFF_FFFF {
        let mut out = Vec::new();
        let mut addr = src;
        while let Ok(b) = ctx.memory.read_u8(addr) {
            out.push(b);
            addr = addr.wrapping_add(1);
            if b == 0 {
                break;
            }
        }
        if out.last() != Some(&0) {
            out.push(0);
        }
        out
    } else {
        ctx.memory
            .read_bytes(src, srclen as usize)
            .unwrap_or_default()
    }
}

/// Same for a WCHAR source: `lstrlenW(src) + 1` when the length is negative.
fn wc_source(ctx: &ApiContext, src: u32, srclen: u32) -> Vec<u16> {
    if srclen == 0xFFFF_FFFF {
        let mut out = Vec::new();
        let mut addr = src;
        while let Ok(w) = ctx.memory.read_u16(addr) {
            out.push(w);
            addr = addr.wrapping_add(2);
            if w == 0 {
                break;
            }
        }
        if out.last() != Some(&0) {
            out.push(0);
        }
        out
    } else {
        (0..srclen)
            .map(|i| ctx.memory.read_u16(src + i * 2).unwrap_or(0))
            .collect()
    }
}

// MultiByteToWideChar(codepage, flags, src, srclen, dst, dstlen).
//
// Wine semantics (dlls/kernelbase/locale.c):
//   * srclen < 0  -> strlen(src)+1, so the NUL is converted and counted;
//     an explicit srclen converts exactly that many bytes and appends nothing.
//   * dstlen == 0 -> return the required length in WCHARs, write nothing.
//   * dstlen < required -> fill dst up to dstlen, SetLastError(
//     ERROR_INSUFFICIENT_BUFFER) and return 0 (`mbstowcs_sbcs`).
fn multibyte_to_widechar(ctx: &mut ApiContext) -> Handled {
    let codepage = ctx.arg(0);
    let src = ctx.arg(2);
    let srcl = ctx.arg(3);
    let dst = ctx.arg(4);
    let dstl = ctx.arg(5);

    if src == 0 || srcl == 0 || (dst == 0 && dstl != 0) {
        ctx.cpu.last_error = ERROR_INVALID_PARAMETER;
        ctx.ret_stdcall(0, 6);
        return Handled::Ok;
    }

    let wide = crate::codepage::decode(codepage, &mb_source(ctx, src, srcl));
    let needed = wide.len() as u32;
    if dstl == 0 {
        ctx.ret_stdcall(needed, 6);
        return Handled::Ok;
    }

    let n = needed.min(dstl);
    for (i, &c) in wide.iter().take(n as usize).enumerate() {
        let _ = ctx.memory.write_u16(dst + (i as u32) * 2, c);
    }
    if dstl < needed {
        ctx.cpu.last_error = ERROR_INSUFFICIENT_BUFFER;
        ctx.ret_stdcall(0, 6);
    } else {
        ctx.ret_stdcall(needed, 6);
    }
    Handled::Ok
}

// WideCharToMultiByte(codepage, flags, src, srclen, dst, dstlen, defchar, used).
// Mirror image of the above; `defchar`/`used` are handled by the codepage
// tables ('?' substitution) rather than honoured literally.
fn widechar_to_multibyte(ctx: &mut ApiContext) -> Handled {
    let codepage = ctx.arg(0);
    let src = ctx.arg(2);
    let srcl = ctx.arg(3);
    let dst = ctx.arg(4);
    let dstl = ctx.arg(5);
    let used = ctx.arg(7);

    if src == 0 || srcl == 0 || (dst == 0 && dstl != 0) {
        ctx.cpu.last_error = ERROR_INVALID_PARAMETER;
        ctx.ret_stdcall(0, 8);
        return Handled::Ok;
    }

    let units = wc_source(ctx, src, srcl);
    let bytes = crate::codepage::encode(codepage, &units);
    if used != 0 {
        let any_default = units.iter().any(|&w| {
            w >= 0x80 && crate::codepage::wchar_to_byte(codepage, w) == b'?' && w != '?' as u16
        });
        let _ = ctx.memory.write_u32(used, any_default as u32);
    }

    let needed = bytes.len() as u32;
    if dstl == 0 {
        ctx.ret_stdcall(needed, 8);
        return Handled::Ok;
    }

    let n = needed.min(dstl) as usize;
    let _ = ctx.memory.write_bytes(dst, &bytes[..n]);
    if dstl < needed {
        ctx.cpu.last_error = ERROR_INSUFFICIENT_BUFFER;
        ctx.ret_stdcall(0, 8);
    } else {
        ctx.ret_stdcall(needed, 8);
    }
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

// ── Critical sections (kernel32 wrappers over RTL_CRITICAL_SECTION layout) ──
// Single-threaded guest: enter always succeeds; fields still match Wine/Windows
// so debuggers and apps that inspect the structure see valid state.

const CS_DEBUG_INFO: u32 = 0;
const CS_LOCK_COUNT: u32 = 4;
const CS_RECURSION: u32 = 8;
const CS_OWNING_THREAD: u32 = 12;
const CS_LOCK_SEMAPHORE: u32 = 16;
const CS_SPIN_COUNT: u32 = 20;

fn init_cs_fields(ctx: &mut ApiContext, cs: u32, spin: u32) {
    if cs == 0 {
        return;
    }
    let _ = ctx.memory.write_u32(cs + CS_DEBUG_INFO, 0xFFFF_FFFF);
    let _ = ctx.memory.write_u32(cs + CS_LOCK_COUNT, 0xFFFF_FFFF);
    let _ = ctx.memory.write_u32(cs + CS_RECURSION, 0);
    let _ = ctx.memory.write_u32(cs + CS_OWNING_THREAD, 0);
    let _ = ctx.memory.write_u32(cs + CS_LOCK_SEMAPHORE, 0);
    let _ = ctx.memory.write_u32(cs + CS_SPIN_COUNT, spin);
}

fn k32_init_cs(ctx: &mut ApiContext) -> Handled {
    init_cs_fields(ctx, ctx.arg(0), 0);
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn k32_init_cs_spin(ctx: &mut ApiContext) -> Handled {
    // BOOL InitializeCriticalSectionAndSpinCount(lpCS, dwSpinCount)
    init_cs_fields(ctx, ctx.arg(0), ctx.arg(1));
    ctx.ret_stdcall(1, 2);
    Handled::Ok
}

fn k32_init_cs_ex(ctx: &mut ApiContext) -> Handled {
    // BOOL InitializeCriticalSectionEx(lpCS, dwSpinCount, Flags)
    init_cs_fields(ctx, ctx.arg(0), ctx.arg(1));
    ctx.ret_stdcall(1, 3);
    Handled::Ok
}

fn k32_delete_cs(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    if cs != 0 {
        let _ = ctx.memory.write_bytes(cs, &[0u8; 24]);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn k32_enter_cs(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    if cs != 0 {
        let rec = ctx.memory.read_u32(cs + CS_RECURSION).unwrap_or(0);
        let _ = ctx.memory.write_u32(cs + CS_LOCK_COUNT, 0);
        let _ = ctx.memory.write_u32(cs + CS_RECURSION, rec.wrapping_add(1));
        let _ = ctx.memory.write_u32(cs + CS_OWNING_THREAD, 1);
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn k32_leave_cs(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    if cs != 0 {
        let rec = ctx.memory.read_u32(cs + CS_RECURSION).unwrap_or(1);
        let next = rec.saturating_sub(1);
        let _ = ctx.memory.write_u32(cs + CS_RECURSION, next);
        if next == 0 {
            let _ = ctx.memory.write_u32(cs + CS_OWNING_THREAD, 0);
            let _ = ctx.memory.write_u32(cs + CS_LOCK_COUNT, 0xFFFF_FFFF);
        }
    }
    ctx.ret_stdcall(0, 1);
    Handled::Ok
}

fn k32_try_enter_cs(ctx: &mut ApiContext) -> Handled {
    let cs = ctx.arg(0);
    if cs != 0 {
        let rec = ctx.memory.read_u32(cs + CS_RECURSION).unwrap_or(0);
        let _ = ctx.memory.write_u32(cs + CS_LOCK_COUNT, 0);
        let _ = ctx.memory.write_u32(cs + CS_RECURSION, rec.wrapping_add(1));
        let _ = ctx.memory.write_u32(cs + CS_OWNING_THREAD, 1);
    }
    ctx.ret_stdcall(1, 1);
    Handled::Ok
}

// Constant-return helpers for APIs whose documented behaviour is a fixed
// success/failure with no guest-visible side effects (e.g. FreeLibrary → TRUE).
// Naming: r{val}_{nargs}. Arg counts are load-bearing for stdcall.
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

// â”€â”€â”€ FormatMessage â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// FormatMessage(dwFlags, lpSource, dwMessageId, dwLanguageId,
//               lpBuffer, nSize, Arguments) â€” 7 args, stdcall.
//
// Flags we honour:
//   FORMAT_MESSAGE_FROM_SYSTEM (0x1000)  â†’ look up dwMessageId in the table.
//   FORMAT_MESSAGE_FROM_STRING (0x0400)  â†’ format lpSource as template.
//   FORMAT_MESSAGE_ALLOCATE_BUFFER (0x100) â†’ heap-allocate; write ptr at lpBuffer.
//   FORMAT_MESSAGE_IGNORE_INSERTS (0x200) â†’ skip %n substitution.
//
// Everything else silently falls back to a generic "unknown error" message so
// the caller's error-checking paths still work.

const FORMAT_MESSAGE_ALLOCATE_BUFFER: u32 = 0x0000_0100;
const FORMAT_MESSAGE_IGNORE_INSERTS: u32 = 0x0000_0200;
const FORMAT_MESSAGE_FROM_HMODULE: u32 = 0x0000_0800;
const FORMAT_MESSAGE_FROM_STRING: u32 = 0x0000_0400;
const FORMAT_MESSAGE_FROM_SYSTEM: u32 = 0x0000_1000;

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
            Some('%') => {
                out.push('%');
                i += 2;
                continue;
            }
            Some('n') | Some('r') => {
                out.push_str("\r\n");
                i += 2;
                continue;
            }
            Some('t') => {
                out.push('\t');
                i += 2;
                continue;
            }
            Some('b') => {
                out.push(' ');
                i += 2;
                continue;
            }
            Some('0') => break, // %0: end of message, no trailing newline
            Some(d) if d.is_ascii_digit() => {
                let n = (d as u8 - b'1') as usize; // %1 -> index 0
                i += 2;
                // Optional !printf-spec! (e.g. !d!, !u!, !s!, !x!).
                let mut spec = String::new();
                if chars.get(i) == Some(&'!') {
                    i += 1;
                    while i < chars.len() && chars[i] != '!' {
                        spec.push(chars[i]);
                        i += 1;
                    }
                    if i < chars.len() {
                        i += 1;
                    } // closing !
                }
                if ignore {
                    continue;
                }
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
                        out.push_str(&if wide {
                            ctx.memory.read_wstr(val)
                        } else {
                            ctx.memory.read_cstr(val)
                        });
                    }
                }
                continue;
            }
            _ => {
                out.push('%');
                i += 1;
                continue;
            }
        }
    }
    out
}

/// Shared core: build the formatted string and write it to lpBuffer.
fn format_message_core(ctx: &mut ApiContext, wide: bool) -> u32 {
    let flags = ctx.arg(0);
    let source = ctx.arg(1);
    let msg_id = ctx.arg(2);
    // arg(3) = language id â€” ignored, we always produce en-US
    let lp_buf = ctx.arg(4);
    let n_size = ctx.arg(5);
    let arg_ptr = ctx.arg(6);

    // Build the message text.
    let text: String = if flags & FORMAT_MESSAGE_FROM_STRING != 0 {
        // lpSource is a pointer to the format string.
        let template = if wide {
            ctx.memory.read_wstr(source)
        } else {
            ctx.memory.read_cstr(source)
        };
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
        format!(
            "{}\r\n",
            with_ins.trim_end_matches(|c| c == '\r' || c == '\n')
        )
    } else {
        // Unsupported flags â€” return empty (caller checks len).
        return 0;
    };

    if text.is_empty() {
        return 0;
    }
    let char_count = text.chars().count() as u32;

    if flags & FORMAT_MESSAGE_ALLOCATE_BUFFER != 0 {
        // Allocate a heap buffer, write pointer to *lpBuffer.
        let byte_len = if wide {
            char_count * 2 + 2
        } else {
            char_count + 1
        };
        let p = ctx.heap_alloc(byte_len);
        if wide {
            let encoded: Vec<u8> = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
            let _ = ctx.memory.write_bytes(p, &encoded);
            let _ = ctx.memory.write_u16(p + char_count * 2, 0);
        } else {
            let _ = ctx.memory.write_bytes(p, text.as_bytes());
            let _ = ctx.memory.write_u8(p + char_count, 0);
        }
        if lp_buf != 0 {
            let _ = ctx.memory.write_u32(lp_buf, p);
        }
        char_count
    } else if lp_buf != 0 && n_size > 0 {
        let limit = (n_size as usize).min(text.chars().count() + 1);
        if wide {
            let encoded: Vec<u8> = text
                .encode_utf16()
                .take(limit.saturating_sub(1))
                .flat_map(|c| c.to_le_bytes())
                .collect();
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
