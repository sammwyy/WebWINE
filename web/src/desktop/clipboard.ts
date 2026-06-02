import type { RuntimeBridge } from "../runtime-bridge.js";
import { log } from "../log.js";

export type ClipboardOp = "copy" | "cut";

interface ClipboardEntry {
  path: string;
  name: string;
  op: ClipboardOp;
}

let entry: ClipboardEntry | null = null;

export function clipboardSet(path: string, name: string, op: ClipboardOp) {
  entry = { path, name, op };
  log("fs", `${op}: ${path}`);
}

export function clipboardHas(): boolean {
  return entry !== null;
}

export function clipboardIsCut(): boolean {
  return entry?.op === "cut";
}

export async function clipboardPaste(
  destDir: string,
  runtime: RuntimeBridge,
  onDone: () => void,
) {
  if (!entry) return;

  const { path, name, op } = entry;
  const destPath = `${destDir}\\${name}`;

  try {
    const bytes = await runtime.readFile(path);
    await runtime.mountFile(destPath, bytes.buffer as ArrayBuffer);

    if (op === "cut") {
      await runtime.deleteNode(path);
      entry = null;
    }

    log("fs", `pasted ${name} -> ${destPath}`);
    onDone();
  } catch (err) {
    log("fs", `paste failed: ${err}`, "error");
  }
}
