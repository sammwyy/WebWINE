import type { DirectoryEntry, LogEvent, PeInfo } from "./worker.js";
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

  private handleMessage(msg: Record<string, unknown>) {
    if (msg.type === "ready") {
      this.readyResolve();
      return;
    }

    if (msg.type === "logs") {
      appendLogs(msg.events as LogEvent[]);
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
