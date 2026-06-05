/**
 * RuntimeBridge — communicates with the WASM worker via postMessage.
 *
 * This file is a direct adaptation of the original runtime-bridge.ts.
 * The only addition is onGlobalLog() so the React layer can route log events
 * to the Zustand log store instead of touching the DOM directly.
 */

import type { DirectoryEntry, LogEvent, PeInfo, ProcessInfo, UiEvent } from "../wasm/worker";
export type { DirectoryEntry, LogEvent, PeInfo, ProcessInfo, UiEvent } from "../wasm/worker";

export interface ProcessHandlers {
  stdout?: (text: string) => void;
  stderr?: (text: string) => void;
  ui?: (events: UiEvent[]) => void;
  log?: (events: LogEvent[]) => void;
  exited?: (code: number) => void;
  crashed?: (reason: string) => void;
}

type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (reason: string) => void;
};

export class RuntimeBridge {
  private worker: Worker;
  private pending = new Map<string, PendingRequest>();
  private reqCounter = 0;
  private readyPromise: Promise<void>;
  private readyResolve!: () => void;

  /** Called for every global log batch (worker-level and process-level). */
  private globalLogListener: ((events: LogEvent[]) => void) | null = null;

  /** Called when a guest process spawns a child via CreateProcess. */
  private spawnListener: ((pid: number, path: string) => void) | null = null;
  private vfsListeners: Set<() => void> = new Set();

  /** Per-PID output callbacks, set by process console windows. */
  private pidHandlers = new Map<number, ProcessHandlers>();

  constructor() {
    this.readyPromise = new Promise((r) => {
      this.readyResolve = r;
    });

    this.worker = new Worker(new URL("../wasm/worker.ts", import.meta.url), {
      type: "module",
    });

    this.worker.onmessage = (e) => this.handleMessage(e.data);
    this.worker.postMessage({ type: "init" });
  }

  /** Register a callback that receives every log batch (for the global log panel). */
  onGlobalLog(cb: (events: LogEvent[]) => void): void {
    this.globalLogListener = cb;
  }

  onProcessOutput(pid: number, handlers: ProcessHandlers): void {
    this.pidHandlers.set(pid, handlers);
  }

  onProcessSpawned(cb: (pid: number, path: string) => void): void {
    this.spawnListener = cb;
  }

  onVfsChanged(cb: () => void): () => void {
    this.vfsListeners.add(cb);
    return () => this.vfsListeners.delete(cb);
  }

  private handleMessage(msg: Record<string, unknown>): void {
    if (msg.type === "ready") {
      this.readyResolve();
      return;
    }

    if (msg.type === "logs") {
      this.globalLogListener?.(msg.events as LogEvent[]);
      return;
    }

    if (msg.type === "process_stdout") {
      this.pidHandlers.get(msg.pid as number)?.stdout?.(msg.text as string);
      return;
    }
    if (msg.type === "process_stderr") {
      this.pidHandlers.get(msg.pid as number)?.stderr?.(msg.text as string);
      return;
    }
    if (msg.type === "process_ui") {
      this.pidHandlers
        .get(msg.pid as number)
        ?.ui?.(msg.events as UiEvent[]);
      return;
    }
    if (msg.type === "process_spawned") {
      this.spawnListener?.(msg.pid as number, msg.path as string);
      return;
    }
    if (msg.type === "process_log") {
      const events = msg.events as LogEvent[];
      this.pidHandlers.get(msg.pid as number)?.log?.(events);
      this.globalLogListener?.(events);
      return;
    }
    if (msg.type === "process_exited") {
      this.pidHandlers.get(msg.pid as number)?.exited?.(msg.exit_code as number);
      this.pidHandlers.delete(msg.pid as number);
      return;
    }
    if (msg.type === "process_crashed") {
      this.pidHandlers.get(msg.pid as number)?.crashed?.(msg.reason as string);
      this.pidHandlers.delete(msg.pid as number);
      return;
    }
    if (msg.type === "vfs_changed") {
      for (const listener of this.vfsListeners) {
        listener();
      }
      return;
    }

    const reqId = msg.requestId as string | undefined;
    if (reqId && this.pending.has(reqId)) {
      const { resolve, reject } = this.pending.get(reqId)!;
      this.pending.delete(reqId);
      if (msg.type === "error") {
        reject(msg.message as string);
      } else {
        resolve(msg);
      }
    }
  }

  private nextId(): string {
    return String(++this.reqCounter);
  }

  private send(msg: Record<string, unknown>, transfer?: Transferable[]): void {
    if (transfer) {
      this.worker.postMessage(msg, transfer);
    } else {
      this.worker.postMessage(msg);
    }
  }

  async ready(): Promise<void> {
    return this.readyPromise;
  }

  async mountFile(path: string, buffer: ArrayBuffer): Promise<void> {
    this.send({ type: "mount_file", path, bytes: buffer }, [buffer]);
  }

  async createDirectory(path: string): Promise<void> {
    this.send({ type: "create_dir", path });
  }

  async listDir(path: string): Promise<DirectoryEntry[]> {
    const requestId = this.nextId();
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (r) => resolve((r as { entries: DirectoryEntry[] }).entries),
        reject,
      });
      this.send({ type: "list_dir", requestId, path });
    });
  }

  async readFile(path: string): Promise<Uint8Array> {
    const requestId = this.nextId();
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (r) =>
          resolve(new Uint8Array((r as { bytes: ArrayBuffer }).bytes)),
        reject,
      });
      this.send({ type: "read_file", requestId, path });
    });
  }

  async readRawFile(path: string): Promise<Uint8Array> {
    const requestId = this.nextId();
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (r) =>
          resolve(new Uint8Array((r as { bytes: ArrayBuffer }).bytes)),
        reject,
      });
      this.send({ type: "read_raw_file", requestId, path });
    });
  }

  async deleteNode(path: string): Promise<void> {
    this.send({ type: "delete_node", path });
  }

  async renameNode(path: string, newName: string): Promise<void> {
    this.send({ type: "rename_node", path, new_name: newName });
  }

  async launchProcess(
    path: string,
  ): Promise<{ pid: number; info: ProcessInfo; launchLogs: LogEvent[] }> {
    const requestId = this.nextId();
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (r) => {
          const msg = r as {
            pid: number;
            info: ProcessInfo;
            launchLogs: LogEvent[];
          };
          const launchLogs = msg.launchLogs ?? [];
          this.globalLogListener?.(launchLogs);
          resolve({ pid: msg.pid, info: msg.info, launchLogs });
        },
        reject,
      });
      this.send({ type: "launch_process", requestId, path });
    });
  }

  async launchProcessWithArgs(
    path: string,
    args: string,
  ): Promise<{ pid: number; info: ProcessInfo; launchLogs: LogEvent[] }> {
    const requestId = this.nextId();
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (r) => {
          const msg = r as {
            pid: number;
            info: ProcessInfo;
            launchLogs: LogEvent[];
          };
          const launchLogs = msg.launchLogs ?? [];
          this.globalLogListener?.(launchLogs);
          resolve({ pid: msg.pid, info: msg.info, launchLogs });
        },
        reject,
      });
      this.send({ type: "launch_process_with_args", requestId, path, args });
    });
  }

  async runProcessSlice(pid: number): Promise<void> {
    this.send({ type: "run_process", pid });
  }

  async registerApp(app: { name: string; exePath: string; icon: string; action: string }): Promise<void> {
    return new Promise((resolve, reject) => {
      const requestId = Math.random().toString(36).slice(2);
      this.pendingRequests.set(requestId, { resolve, reject });
      this.send({ type: "register_app", requestId, app });
    });
  }

  writeStdin(pid: number, text: string): void {
    this.send({ type: "write_stdin", pid, text });
  }

  postWindowMessage(
    pid: number,
    hwnd: number,
    message: number,
    wparam = 0,
    lparam = 0,
  ): void {
    this.send({ type: "post_message", pid, hwnd, message, wparam, lparam });
  }

  killProcess(pid: number): void {
    this.send({ type: "kill_process", pid });
  }

  async inspectPe(path: string): Promise<PeInfo> {
    const requestId = this.nextId();
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (r) => resolve((r as { info: PeInfo }).info),
        reject,
      });
      this.send({ type: "inspect_pe", requestId, path });
    });
  }

  async exportVfs(): Promise<Uint8Array> {
    const requestId = this.nextId();
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (r) =>
          resolve(new Uint8Array((r as { bytes: ArrayBuffer }).bytes)),
        reject,
      });
      this.send({ type: "export_vfs", requestId });
    });
  }

  async importVfs(bytes: Uint8Array): Promise<void> {
    this.send({ type: "import_vfs", bytes: bytes.buffer });
  }
}
