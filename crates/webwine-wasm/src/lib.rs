use wasm_bindgen::prelude::*;
use webwine_core::{WebWineVm, DirEntry, LogEvent, PeInfo, ProcessInfo, SliceResult};

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct Runtime {
    vm: WebWineVm,
}

#[wasm_bindgen]
impl Runtime {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Runtime {
        Runtime {
            vm: WebWineVm::new(),
        }
    }

    #[wasm_bindgen(js_name = mountFile)]
    pub fn mount_file(&mut self, path: &str, bytes: &[u8]) -> Result<(), JsValue> {
        self.vm
            .mount_file(path, bytes.to_vec())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = createDirectory)]
    pub fn create_directory(&mut self, path: &str) -> Result<(), JsValue> {
        self.vm
            .create_dir(path)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = listDirectory)]
    pub fn list_directory(&self, path: &str) -> Result<JsValue, JsValue> {
        let entries: Vec<DirEntry> = self
            .vm
            .list_dir(path)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&entries).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = readFile)]
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, JsValue> {
        self.vm
            .read_file(path)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = deleteNode)]
    pub fn delete_node(&mut self, path: &str) -> Result<(), JsValue> {
        self.vm
            .delete_node(path)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = renameNode)]
    pub fn rename_node(&mut self, path: &str, new_name: &str) -> Result<(), JsValue> {
        self.vm
            .rename_node(path, new_name)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = inspectPe)]
    pub fn inspect_pe(&mut self, path: &str) -> Result<JsValue, JsValue> {
        let info: PeInfo = self
            .vm
            .inspect_pe(path)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = launchProcess)]
    pub fn launch_process(&mut self, path: &str) -> Result<u32, JsValue> {
        self.vm
            .launch_process(path)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = getProcessInfo)]
    pub fn get_process_info(&self, pid: u32) -> Result<JsValue, JsValue> {
        let info: ProcessInfo = self
            .vm
            .get_process_info(pid)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = listProcesses)]
    pub fn list_processes(&self) -> Result<JsValue, JsValue> {
        let list: Vec<ProcessInfo> = self.vm.list_processes();
        serde_wasm_bindgen::to_value(&list).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = killProcess)]
    pub fn kill_process(&mut self, pid: u32) -> Result<(), JsValue> {
        self.vm
            .kill_process(pid)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = runProcessSlice)]
    pub fn run_process_slice(&mut self, pid: u32, budget: u32) -> Result<JsValue, JsValue> {
        let result: SliceResult = self.vm
            .run_process_slice(pid, budget)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = writeProcessStdin)]
    pub fn write_process_stdin(&mut self, pid: u32, text: &str) -> Result<(), JsValue> {
        self.vm
            .write_stdin(pid, text)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = postWindowMessage)]
    pub fn post_window_message(
        &mut self, pid: u32, hwnd: u32, message: u32, wparam: u32, lparam: u32,
    ) -> Result<(), JsValue> {
        self.vm
            .post_window_message(pid, hwnd, message, wparam, lparam)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = drainLogs)]
    pub fn drain_logs(&mut self) -> Result<JsValue, JsValue> {
        let events: Vec<LogEvent> = self.vm.drain_logs();
        serde_wasm_bindgen::to_value(&events).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
