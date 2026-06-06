import init, { Runtime } from "../../pkg/webwine_wasm";

type InMsg =
  | { type: "init" }
  | { type: "mount_file"; path: string; bytes: ArrayBuffer }
  | { type: "create_dir"; path: string }
  | { type: "list_dir"; requestId: string; path: string }
  | { type: "read_file"; requestId: string; path: string }
  | { type: "read_raw_file"; requestId: string; path: string }
  | { type: "inspect_pe"; requestId: string; path: string }
  | { type: "delete_node"; path: string }
  | { type: "rename_node"; path: string; new_name: string }
  | { type: "launch_process"; requestId: string; path: string }
  | { type: "launch_process_with_args"; requestId: string; path: string; args: string }
  | { type: "run_process"; pid: number }
  | { type: "write_stdin"; pid: number; text: string }
  | { type: "post_message"; pid: number; hwnd: number; message: number; wparam: number; lparam: number }
  | { type: "post_dialog_reply"; pid: number; button: number; file: string }
  | { type: "kill_process"; pid: number }
  | { type: "export_vfs"; requestId: string }
  | { type: "import_vfs"; bytes: ArrayBuffer }
  | { type: "reg_list_subkeys"; requestId: string; path: string }
  | { type: "reg_list_values"; requestId: string; path: string }
  | { type: "reg_create_key"; requestId: string; path: string }
  | { type: "reg_delete_key"; requestId: string; path: string }
  | { type: "reg_set_value"; requestId: string; path: string; name: string; value: RegValue }
  | { type: "reg_delete_value"; requestId: string; path: string; name: string }
  | { type: "register_app"; requestId: string; app: { name: string; exePath: string; icon: string; action: string } };

type OutMsg =
  | { type: "ready" }
  | { type: "error"; requestId?: string; message: string }
  | { type: "dir_list"; requestId: string; entries: DirectoryEntry[] }
  | { type: "file_data"; requestId: string; path: string; bytes: ArrayBuffer }
  | { type: "pe_info"; requestId: string; info: PeInfo }
  | { type: "process_launched"; requestId: string; pid: number; info: ProcessInfo; launchLogs: LogEvent[] }
  | { type: "process_stdout"; pid: number; text: string }
  | { type: "process_stderr"; pid: number; text: string }
  | { type: "process_ui"; pid: number; events: UiEvent[] }
  | { type: "process_log"; pid: number; events: LogEvent[] }
  | { type: "process_spawned"; parent: number; pid: number; path: string }
  | { type: "process_exited"; pid: number; exit_code: number }
  | { type: "process_crashed"; pid: number; reason: string }
  | { type: "logs"; events: LogEvent[] }
  | { type: "vfs_exported"; requestId: string; bytes: ArrayBuffer }
  | { type: "vfs_changed" }
  | { type: "reg_subkeys"; requestId: string; subkeys: string[] }
  | { type: "reg_values"; requestId: string; values: NamedValue[] }
  | { type: "reg_ok"; requestId: string; result: boolean }
  | { type: "reg_changed" }
  | { type: "app_registered"; requestId: string };

/** A registry value, mirroring the Rust `RegValue` (serde adjacently tagged). */
export type RegValue =
  | { type: "Sz"; data: string }
  | { type: "ExpandSz"; data: string }
  | { type: "Dword"; data: number }
  | { type: "Qword"; data: number }
  | { type: "Binary"; data: number[] }
  | { type: "MultiSz"; data: string[] }
  | { type: "None" };

export interface NamedValue {
  name: string;
  value: RegValue;
}

/** A resolved menu node (mirrors the Rust `MenuItemData`). */
export interface MenuItemData {
  text: string;
  id: number;
  separator: boolean;
  disabled: boolean;
  children: MenuItemData[];
}

export interface DirectoryEntry {
  name: string;
  path: string;
  kind: "file" | "directory";
  size: number;
}

export interface LogEvent {
  level: "trace" | "debug" | "info" | "warn" | "error";
  target: string;
  message: string;
  pid?: number;
}

export interface PeSection {
  name: string;
  virtual_address: number;
  virtual_size: number;
  raw_size: number;
  readable: boolean;
  writable: boolean;
  executable: boolean;
}

export interface PeImportModule {
  dll: string;
  functions: string[];
}

export interface ProcessInfo {
  pid: number;
  path: string;
  image_base: number;
  entry_point: number;
  state: { state: "created" } | { state: "running" } | { state: "blocked" }
       | { state: "waiting_for_input" }
       | { state: "exited"; exit_code: number }
       | { state: "crashed"; reason: string };
}

export interface PeInfo {
  machine: string;
  machine_id: number;
  subsystem: string;
  subsystem_id: number;
  is_pe32: boolean;
  is_dll: boolean;
  image_base: number;
  entry_point_rva: number;
  size_of_image: number;
  sections: PeSection[];
  imports: PeImportModule[];
}

export type UiEvent =
  | { kind: "message_box"; title: string; text: string; style: number }
  | { kind: "create_window"; hwnd: number; title: string; x: number; y: number; width: number; height: number }
  | { kind: "show_window"; hwnd: number; show: boolean }
  | { kind: "set_window_text"; hwnd: number; title: string }
  | { kind: "destroy_window"; hwnd: number }
  | { kind: "clear_client"; hwnd: number }
  | { kind: "draw_text"; hwnd: number; x: number; y: number; text: string; color: number }
  | { kind: "fill_rect"; hwnd: number; x: number; y: number; w: number; h: number; color: number }
  | { kind: "rect"; hwnd: number; x: number; y: number; w: number; h: number; fill: number; stroke: number }
  | { kind: "ellipse"; hwnd: number; x: number; y: number; w: number; h: number; fill: number; stroke: number }
  | { kind: "line"; hwnd: number; x1: number; y1: number; x2: number; y2: number; color: number }
  | { kind: "set_pixel"; hwnd: number; x: number; y: number; color: number }
  | { kind: "blit"; hwnd: number; x: number; y: number; w: number; h: number; src_w: number; src_h: number; pixels: number[] }
  | { kind: "beep"; freq: number; duration: number }
  | { kind: "file_dialog"; save: boolean; title: string; filter: string; initial_dir: string; default_name: string }
  | { kind: "set_menu"; hwnd: number; items: MenuItemData[] }
  // Direct3D8 GPU command stream (consumed by a VideoDriver / WebGLVideoDriver).
  | { kind: "gpu_clear"; hwnd: number; color: number }
  | { kind: "gpu_texture"; hwnd: number; id: number; w: number; h: number; pixels: number[] }
  | { kind: "gpu_draw_tris"; hwnd: number; texture: number; blend: number; verts: number[] }
  | { kind: "gpu_present"; hwnd: number };

export interface SliceResult {
  pid: number;
  stdout: string;
  stderr: string;
  state: ProcessInfo["state"];
  instructions: number;
  ui_events: UiEvent[];
  spawned: [number, string][];
}

let runtime: Runtime | null = null;
const runningPids = new Set<number>();

function send(msg: OutMsg) {
  postMessage(msg);
}

// ---- registry persistence (IndexedDB; localStorage is unavailable in workers) ----

const IDB_NAME = "webwine";
const IDB_STORE = "kv";
const REG_KEY = "registry";

function idb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, 1);
    req.onupgradeneeded = () => {
      if (!req.result.objectStoreNames.contains(IDB_STORE)) {
        req.result.createObjectStore(IDB_STORE);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function idbGet(key: string): Promise<unknown> {
  const db = await idb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, "readonly").objectStore(IDB_STORE).get(key);
    tx.onsuccess = () => resolve(tx.result);
    tx.onerror = () => reject(tx.error);
  });
}

async function idbSet(key: string, value: unknown): Promise<void> {
  const db = await idb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, "readwrite").objectStore(IDB_STORE).put(value, key);
    tx.onsuccess = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

async function loadRegistry() {
  if (!runtime) return;
  try {
    const snap = await idbGet(REG_KEY);
    if (snap !== undefined && snap !== null) {
      runtime.importRegistry(snap);
    }
  } catch {
    // First run / private mode: keep the in-memory default hive.
  }
}

let persistTimer: ReturnType<typeof setTimeout> | null = null;
/** Debounced flush of the in-memory hive to IndexedDB. */
function persistRegistry() {
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    if (!runtime) return;
    try {
      void idbSet(REG_KEY, runtime.exportRegistry());
    } catch {
      // ignore quota / serialization failures
    }
  }, 400);
}

function flushLogs() {
  if (!runtime) return;
  const events = runtime.drainLogs() as LogEvent[];
  if (events.length > 0) {
    send({ type: "logs", events });
  }
}

async function runProcessLoop(pid: number) {
  if (!runtime) return;
  if (runningPids.has(pid)) return;
  runningPids.add(pid);
  const BUDGET = 50_000;

  try {
    while (true) {
      const result = runtime.runProcessSlice(pid, BUDGET) as SliceResult;

      // Attribute logs drained during this process's run to it (for debug mode),
      // and still forward to the global system-log panel via the bridge.
      const logs = runtime.drainLogs() as LogEvent[];
      if (logs.length > 0) send({ type: "process_log", pid, events: logs });

      if (result.stdout) send({ type: "process_stdout", pid, text: result.stdout });
      if (result.stderr) send({ type: "process_stderr", pid, text: result.stderr });
      if (result.ui_events && result.ui_events.length > 0)
        send({ type: "process_ui", pid, events: result.ui_events });
      if (result.spawned && result.spawned.length > 0) {
        for (const [cpid, path] of result.spawned)
          send({ type: "process_spawned", parent: pid, pid: cpid, path });
      }

      const s = result.state.state;
      if (s === "exited") {
        send({ type: "process_exited", pid, exit_code: (result.state as { state: "exited"; exit_code: number }).exit_code });
        send({ type: "vfs_changed" });
        break;
      }
      if (s === "crashed") {
        send({ type: "process_crashed", pid, reason: (result.state as { state: "crashed"; reason: string }).reason });
        send({ type: "vfs_changed" });
        break;
      }
      if (s !== "running" && s !== "created") break;

      // yield to event loop so we don't block the worker
      await new Promise<void>((r) => setTimeout(r, 0));
    }
  } finally {
    runningPids.delete(pid);
    // A guest run may have mutated the hive (RegSetValueEx, RegCreateKeyEx).
    persistRegistry();
  }
}

self.onmessage = async (e: MessageEvent<InMsg>) => {
  const msg = e.data;

  if (msg.type === "init") {
    await init();
    runtime = new Runtime();
    await loadRegistry();
    flushLogs();
    send({ type: "ready" });
    return;
  }

  if (!runtime) {
    send({ type: "error", message: "Runtime not initialized" });
    return;
  }

  try {
    if (msg.type === "mount_file") {
      runtime.mountFile(msg.path, new Uint8Array(msg.bytes));
      flushLogs();
      send({ type: "vfs_changed" });
    } else if (msg.type === "create_dir") {
      runtime.createDirectory(msg.path);
      flushLogs();
      send({ type: "vfs_changed" });
    } else if (msg.type === "list_dir") {
      const entries = runtime.listDirectory(msg.path) as DirectoryEntry[];
      flushLogs();
      send({ type: "dir_list", requestId: msg.requestId, entries });
    } else if (msg.type === "read_file") {
      const bytes = runtime.readFile(msg.path) as Uint8Array;
      flushLogs();
      send({
        type: "file_data",
        requestId: msg.requestId,
        path: msg.path,
        bytes: bytes.buffer as ArrayBuffer,
      });
    } else if (msg.type === "read_raw_file") {
      const bytes = runtime.readRawFile(msg.path) as Uint8Array;
      flushLogs();
      send({
        type: "file_data",
        requestId: msg.requestId,
        path: msg.path,
        bytes: bytes.buffer as ArrayBuffer,
      });
    } else if (msg.type === "inspect_pe") {
      const info = runtime.inspectPe(msg.path) as PeInfo;
      flushLogs();
      send({ type: "pe_info", requestId: msg.requestId, info });
    } else if (msg.type === "launch_process") {
      const pid = runtime.launchProcess(msg.path) as number;
      // Loader/PE logs produced during launch — handed to the console for debug mode.
      const launchLogs = runtime.drainLogs() as LogEvent[];
      const info = runtime.getProcessInfo(pid) as ProcessInfo;
      send({ type: "process_launched", requestId: msg.requestId, pid, info, launchLogs });
    } else if (msg.type === "launch_process_with_args") {
      const pid = runtime.launchProcessWithArgs(msg.path, msg.args) as number;
      const launchLogs = runtime.drainLogs() as LogEvent[];
      const info = runtime.getProcessInfo(pid) as ProcessInfo;
      send({ type: "process_launched", requestId: msg.requestId, pid, info, launchLogs });
    } else if (msg.type === "run_process") {
      runProcessLoop(msg.pid);
    } else if (msg.type === "write_stdin") {
      runtime.writeProcessStdin(msg.pid, msg.text);
      runProcessLoop(msg.pid);
    } else if (msg.type === "post_message") {
      runtime.postWindowMessage(msg.pid, msg.hwnd, msg.message, msg.wparam, msg.lparam);
      // Resume the (suspended) message loop.
      runProcessLoop(msg.pid);
    } else if (msg.type === "post_dialog_reply") {
      runtime.postDialogReply(msg.pid, msg.button, msg.file);
      runProcessLoop(msg.pid); // resume the process blocked in the dialog
    } else if (msg.type === "kill_process") {
      runtime.killProcess(msg.pid);
      flushLogs();
    } else if (msg.type === "delete_node") {
      runtime.deleteNode(msg.path);
      flushLogs();
      send({ type: "vfs_changed" });
    } else if (msg.type === "rename_node") {
      runtime.renameNode(msg.path, msg.new_name);
      flushLogs();
      send({ type: "vfs_changed" });
    } else if (msg.type === "export_vfs") {
      const bytes = runtime.exportVfs() as Uint8Array;
      flushLogs();
      send({
        type: "vfs_exported",
        requestId: msg.requestId,
        bytes: bytes.buffer as ArrayBuffer,
      });
    } else if (msg.type === "import_vfs") {
      runtime.importVfs(new Uint8Array(msg.bytes));
      flushLogs();
    } else if (msg.type === "reg_list_subkeys") {
      const subkeys = runtime.regListSubkeys(msg.path) as string[];
      send({ type: "reg_subkeys", requestId: msg.requestId, subkeys });
    } else if (msg.type === "reg_list_values") {
      const values = runtime.regListValues(msg.path) as NamedValue[];
      send({ type: "reg_values", requestId: msg.requestId, values });
    } else if (msg.type === "reg_create_key") {
      runtime.regCreateKey(msg.path);
      persistRegistry();
      send({ type: "reg_ok", requestId: msg.requestId, result: true });
      send({ type: "reg_changed" });
    } else if (msg.type === "reg_delete_key") {
      const result = runtime.regDeleteKey(msg.path) as boolean;
      persistRegistry();
      send({ type: "reg_ok", requestId: msg.requestId, result });
      send({ type: "reg_changed" });
    } else if (msg.type === "reg_set_value") {
      runtime.regSetValue(msg.path, msg.name, msg.value);
      persistRegistry();
      send({ type: "reg_ok", requestId: msg.requestId, result: true });
      send({ type: "reg_changed" });
    } else if (msg.type === "reg_delete_value") {
      const result = runtime.regDeleteValue(msg.path, msg.name) as boolean;
      persistRegistry();
      send({ type: "reg_ok", requestId: msg.requestId, result });
      send({ type: "reg_changed" });
    } else if (msg.type === "register_app") {
      // Icon is client-only UI metadata and is intentionally not sent to the core.
      runtime.registerApp({
        name: msg.app.name,
        exe_path: msg.app.exePath,
        action: msg.app.action,
      });
      flushLogs();
      send({ type: "app_registered", requestId: msg.requestId });
      send({ type: "vfs_changed" });
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const requestId = "requestId" in msg ? (msg as { requestId: string }).requestId : undefined;
    send({ type: "error", requestId, message });
  }
};
