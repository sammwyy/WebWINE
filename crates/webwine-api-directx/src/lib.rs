pub mod ddraw;
pub mod dinput;
pub mod dsound;

pub use webwine_api::winapi::{ApiContext, Handled, HandlerFn, WinApiRegistry};

/// Register DirectX-era DLL stubs.
pub fn register(r: &mut WinApiRegistry) {
    ddraw::register(r);
    dsound::register(r);
    dinput::register(r);
}
