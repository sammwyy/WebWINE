import type { DirectoryEntry, LogEvent, PeInfo, ProcessInfo } from "./worker.js";
import { appendLogs } from "./log.js";

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

  constructor() {
    this.readyPromise = new Promise((r) => { this.readyResolve = r; });

    this.worker = new Worker(new URL("./worker.ts", import.meta.url), {
      type: "module",
    });

    this.worker.onmessage = (e) => this.handleMessage(e.data);
    this.worker.postMessage({ type: "init" });
  }

  // per-pid output callbacks, set by process console windows
  private pidHandlers = new Map<number, {
    stdout?: (text: string) => void;
    stderr?: (text: string) => void;
    exited?: (code: number) => void;
    crashed?: (reason: string) => void;
  }>();

  onProcessOutput(pid: number, handlers: {
    stdout?: (text: string) => void;
    stderr?: (text: string) => void;
    exited?: (code: number) => void;
    crashed?: (reason: string) => void;
  }) {
    this.pidHandlers.set(pid, handlers);
  }

  private handleMessage(msg: Record<string, unknown>) {
    if (msg.type === "ready") {
      this.readyResolve();
      return;
    }

    if (msg.type === "logs") {
      appendLogs(msg.events as LogEvent[]);
      return;
    }

    if (msg.type === "process_stdout") {
      const h = this.pidHandlers.get(msg.pid as number);
      h?.stdout?.(msg.text as string);
      return;
    }
    if (msg.type === "process_stderr") {
      const h = this.pidHandlers.get(msg.pid as number);
      h?.stderr?.(msg.text as string);
      return;
    }
    if (msg.type === "process_exited") {
      const h = this.pidHandlers.get(msg.pid as number);
      h?.exited?.(msg.exit_code as number);
      this.pidHandlers.delete(msg.pid as number);
      return;
    }
    if (msg.type === "process_crashed") {
      const h = this.pidHandlers.get(msg.pid as number);
      h?.crashed?.(msg.reason as string);
      this.pidHandlers.delete(msg.pid as number);
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
        resolve: (r) => resolve(new Uint8Array((r as { bytes: ArrayBuffer }).bytes)),
        reject,
      });
      this.send({ type: "read_file", requestId, path });
    });
  }

  async deleteNode(path: string): Promise<void> {
    this.send({ type: "delete_node", path });
  }

  async renameNode(path: string, newName: string): Promise<void> {
    this.send({ type: "rename_node", path, new_name: newName });
  }

  async launchProcess(path: string): Promise<{ pid: number; info: ProcessInfo }> {
    const requestId = this.nextId();
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (r) => {
          const msg = r as { pid: number; info: ProcessInfo };
          resolve({ pid: msg.pid, info: msg.info });
        },
        reject,
      });
      this.send({ type: "launch_process", requestId, path });
    });
  }

  runProcess(pid: number): void {
    this.send({ type: "run_process", pid });
  }

  writeStdin(pid: number, text: string): void {
    this.send({ type: "write_stdin", pid, text });
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
}
