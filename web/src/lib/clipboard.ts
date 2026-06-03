import type { RuntimeBridge } from "./runtime-bridge.js";

export type ClipboardOp = "copy" | "cut";

export interface ClipboardEntry {
  path: string;
  name: string;
  op: ClipboardOp;
}

/**
 * Paste the clipboard entry into destDir, reading the file from the runtime
 * and optionally deleting the source on cut.
 */
export async function performPaste(
  entry: ClipboardEntry,
  destDir: string,
  runtime: RuntimeBridge,
): Promise<void> {
  const destPath = `${destDir}\\${entry.name}`;
  const bytes = await runtime.readFile(entry.path);
  await runtime.mountFile(destPath, bytes.buffer as ArrayBuffer);
  if (entry.op === "cut") {
    await runtime.deleteNode(entry.path);
  }
}

/** Upload files from a FileList into destDir, creating parent dirs as needed. */
export async function mountFiles(
  files: File[],
  destDir: string,
  runtime: RuntimeBridge,
  opts: { preserveRelativePath?: boolean } = {},
): Promise<string[]> {
  const createdDirs = new Set<string>();
  const uploaded: string[] = [];

  for (const file of files) {
    const relativePath =
      opts.preserveRelativePath && file.webkitRelativePath
        ? file.webkitRelativePath.replace(/\//g, "\\")
        : file.name;
    const guestPath = `${destDir}\\${relativePath}`;
    const buffer = await file.arrayBuffer();
    await ensureParentDirs(guestPath, destDir, runtime, createdDirs);
    await runtime.mountFile(guestPath, buffer);
    uploaded.push(relativePath);
  }
  return uploaded;
}

async function ensureParentDirs(
  path: string,
  rootDir: string,
  runtime: RuntimeBridge,
  createdDirs: Set<string>,
): Promise<void> {
  const parts = path.split("\\");
  parts.pop();
  let current = parts[0];
  for (const part of parts.slice(1)) {
    current = `${current}\\${part}`;
    if (current === rootDir || !current.startsWith(`${rootDir}\\`)) continue;
    if (createdDirs.has(current)) continue;
    try {
      await runtime.createDirectory(current);
    } catch {
      // Directory may already exist — that is fine.
    }
    createdDirs.add(current);
  }
}
