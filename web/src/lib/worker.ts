import init, { Runtime } from "../pkg/webwine_wasm.js";

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
  | { type: "run_process"; pid: number }
  | { type: "write_stdin"; pid: number; text: string }
  | { type: "post_message"; pid: number; hwnd: number; message: number; wparam: number; lparam: number }
  | { type: "kill_process"; pid: number };

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
  | { type: "logs"; events: LogEvent[] };

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
  | { kind: "beep"; freq: number; duration: number };

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

function send(msg: OutMsg) {
  postMessage(msg);
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
  const BUDGET = 50_000;

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
      break;
    }
    if (s === "crashed") {
      send({ type: "process_crashed", pid, reason: (result.state as { state: "crashed"; reason: string }).reason });
      break;
    }
    if (s !== "running" && s !== "created") break;

    // yield to event loop so we don't block the worker
    await new Promise<void>((r) => setTimeout(r, 0));
  }
}

self.onmessage = async (e: MessageEvent<InMsg>) => {
  const msg = e.data;

  if (msg.type === "init") {
    await init();
    runtime = new Runtime();
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
    } else if (msg.type === "create_dir") {
      runtime.createDirectory(msg.path);
      flushLogs();
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
    } else if (msg.type === "run_process") {
      runProcessLoop(msg.pid);
    } else if (msg.type === "write_stdin") {
      runtime.writeProcessStdin(msg.pid, msg.text);
    } else if (msg.type === "post_message") {
      runtime.postWindowMessage(msg.pid, msg.hwnd, msg.message, msg.wparam, msg.lparam);
      // Resume the (suspended) message loop.
      runProcessLoop(msg.pid);
    } else if (msg.type === "kill_process") {
      runtime.killProcess(msg.pid);
      flushLogs();
    } else if (msg.type === "delete_node") {
      runtime.deleteNode(msg.path);
      flushLogs();
    } else if (msg.type === "rename_node") {
      runtime.renameNode(msg.path, msg.new_name);
      flushLogs();
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const requestId = "requestId" in msg ? (msg as { requestId: string }).requestId : undefined;
    send({ type: "error", requestId, message });
  }
};
