use wasm_bindgen::prelude::*;
use webwine_core::{WebWineVm, DirEntry, LogEvent, PeInfo, ProcessInfo, SliceResult};
use webwine_core::registry::{RegValue, RegistrySnapshot};

/// One value row for the regedit UI.
#[derive(serde::Serialize)]
struct NamedValue {
    name: String,
    value: RegValue,
}

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

    #[wasm_bindgen(js_name = readRawFile)]
    pub fn read_raw_file(&self, path: &str) -> Result<Vec<u8>, JsValue> {
        self.vm
            .read_raw_file(path)
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

    #[wasm_bindgen(js_name = inspectClr)]
    pub fn inspect_clr(&mut self, path: &str) -> Result<JsValue, JsValue> {
        let info = self
            .vm
            .inspect_clr(path)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = isManagedFile)]
    pub fn is_managed_file(&self, path: &str) -> bool {
        self.vm.is_managed_file(path)
    }

    #[wasm_bindgen(js_name = launchProcess)]
    pub fn launch_process(&mut self, path: &str) -> Result<u32, JsValue> {
        self.vm
            .launch_process(path)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = launchProcessWithArgs)]
    pub fn launch_process_with_args(&mut self, path: &str, args: &str) -> Result<u32, JsValue> {
        self.vm
            .launch_process_with_args(path, args)
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
    /// Answer a modal dialog the process is blocked on. `button` is the clicked
    /// Win32 ID (or 1/0 for a file dialog ok/cancel); `file` is the chosen path
    /// (empty string = none).
    #[wasm_bindgen(js_name = postDialogReply)]
    pub fn post_dialog_reply(&mut self, pid: u32, button: u32, file: &str) -> Result<(), JsValue> {
        let file = if file.is_empty() { None } else { Some(file.to_string()) };
        self.vm
            .post_dialog_reply(pid, button, file)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = drainLogs)]
    pub fn drain_logs(&mut self) -> Result<JsValue, JsValue> {
        let events: Vec<LogEvent> = self.vm.drain_logs();
        serde_wasm_bindgen::to_value(&events).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = registerApp)]
    pub fn register_app(&mut self, app_json: &JsValue) -> Result<(), JsValue> {
        let app: webwine_core::AppRegistration = serde_wasm_bindgen::from_value(app_json.clone())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.vm
            .register_app(&app)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // ---- registry ----

    /// Serialize the whole registry hive (for persisting to browser storage).
    #[wasm_bindgen(js_name = exportRegistry)]
    pub fn export_registry(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.vm.registry.export())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Replace the registry hive from a previously exported snapshot.
    #[wasm_bindgen(js_name = importRegistry)]
    pub fn import_registry(&mut self, snapshot: &JsValue) -> Result<(), JsValue> {
        let snap: RegistrySnapshot = serde_wasm_bindgen::from_value(snapshot.clone())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.vm.registry.import(snap);
        Ok(())
    }

    /// Immediate child key names of `path` (for the regedit tree).
    #[wasm_bindgen(js_name = regListSubkeys)]
    pub fn reg_list_subkeys(&self, path: &str) -> Result<JsValue, JsValue> {
        let subs = self.vm.registry.subkeys(path);
        serde_wasm_bindgen::to_value(&subs).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Values of `path` as `[{name, value}]` (for the regedit value pane).
    #[wasm_bindgen(js_name = regListValues)]
    pub fn reg_list_values(&self, path: &str) -> Result<JsValue, JsValue> {
        let rows: Vec<NamedValue> = self
            .vm
            .registry
            .values_of(path)
            .map(|m| m.iter().map(|(name, value)| NamedValue { name: name.clone(), value: value.clone() }).collect())
            .unwrap_or_default();
        serde_wasm_bindgen::to_value(&rows).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// True if `path` exists.
    #[wasm_bindgen(js_name = regKeyExists)]
    pub fn reg_key_exists(&self, path: &str) -> bool {
        self.vm.registry.key_exists(path)
    }

    #[wasm_bindgen(js_name = regCreateKey)]
    pub fn reg_create_key(&mut self, path: &str) {
        self.vm.registry.ensure_key(path);
    }

    #[wasm_bindgen(js_name = regDeleteKey)]
    pub fn reg_delete_key(&mut self, path: &str) -> bool {
        self.vm.registry.delete_key_path(path)
    }

    /// Set a value. `value` is a serialized `RegValue` (e.g. `{type:"Dword",data:1}`).
    #[wasm_bindgen(js_name = regSetValue)]
    pub fn reg_set_value(&mut self, path: &str, name: &str, value: &JsValue) -> Result<(), JsValue> {
        let v: RegValue = serde_wasm_bindgen::from_value(value.clone())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.vm.registry.set_value_path(path, name, v);
        Ok(())
    }

    #[wasm_bindgen(js_name = regDeleteValue)]
    pub fn reg_delete_value(&mut self, path: &str, name: &str) -> bool {
        self.vm.registry.delete_value_path(path, name)
    }
}
