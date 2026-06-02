import init, { Runtime } from "./pkg/webwine_wasm.js";

type InMsg =
  | { type: "init" }
  | { type: "mount_file"; path: string; bytes: ArrayBuffer }
  | { type: "create_dir"; path: string }
  | { type: "list_dir"; requestId: string; path: string }
  | { type: "read_file"; requestId: string; path: string };

type OutMsg =
  | { type: "ready" }
  | { type: "error"; requestId?: string; message: string }
  | { type: "dir_list"; requestId: string; entries: DirectoryEntry[] }
  | { type: "file_data"; requestId: string; path: string; bytes: ArrayBuffer }
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
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const requestId = "requestId" in msg ? (msg as { requestId: string }).requestId : undefined;
    send({ type: "error", requestId, message });
  }
};
