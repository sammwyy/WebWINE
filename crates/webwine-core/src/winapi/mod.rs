pub use webwine_api::winapi::{context, ApiContext, Handled, HandlerFn, WinApiRegistry};

/// Register all known handlers. Called once from `WebWineVm::new()`.
pub fn register_all(r: &mut WinApiRegistry) {
    webwine_api_winapi::register(r);
    webwine_api_directx::register(r);
    r.finalize();
}
