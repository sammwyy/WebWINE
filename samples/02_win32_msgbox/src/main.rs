use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxA, MB_OK, MB_ICONINFORMATION};

fn main() {
    let title = b"WebWINE\0";
    let text = b"Hello from Win32!\0";

    unsafe {
        MessageBoxA(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
