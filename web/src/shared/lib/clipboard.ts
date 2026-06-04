import type { RuntimeBridge } from "../../core/bridge/runtime-bridge";
import type { DirectoryEntry } from "../../core/wasm/worker";

export type ClipboardOp = "copy" | "cut";

export interface ClipboardEntry {
  path: string;
  name: string;
  op: ClipboardOp;
  kind?: DirectoryEntry["kind"];
}

export interface FilePayloadEntry {
  path: string;
  name: string;
  kind: DirectoryEntry["kind"];
}

/**
 * Paste the clipboard entries into destDir, reading raw files from the runtime
 * and optionally deleting the source on cut.
 */
export async function performPaste(
  entryOrEntries: ClipboardEntry | ClipboardEntry[],
  destDir: string,
  runtime: RuntimeBridge,
): Promise<void> {
  const entries = Array.isArray(entryOrEntries) ? entryOrEntries : [entryOrEntries];
  for (const entry of entries) {
    if (entry.op === "cut" && samePath(parentPath(entry.path), destDir)) continue;
    await copyNode(entry, destDir, runtime, false);
  }
  for (const entry of entries) {
    if (entry.op === "cut" && !samePath(parentPath(entry.path), destDir)) {
      await runtime.deleteNode(entry.path);
    }
  }
  emitFsChanged();
}

export async function pasteShortcut(
  entries: ClipboardEntry[],
  destDir: string,
  runtime: RuntimeBridge,
): Promise<void> {
  for (const entry of entries) {
    await createShortcut(entry.path, entry.name, destDir, runtime);
  }
  emitFsChanged();
}

export async function copyPayloadToDir(
  entries: FilePayloadEntry[],
  destDir: string,
  runtime: RuntimeBridge,
  move = false,
): Promise<void> {
  for (const entry of entries) {
    if (move && samePath(parentPath(entry.path), destDir)) continue;
    await copyNode(entry, destDir, runtime, false);
  }
  if (move) {
    for (const entry of entries) {
      if (samePath(parentPath(entry.path), destDir)) continue;
      await runtime.deleteNode(entry.path);
    }
  }
  emitFsChanged();
}

export async function createShortcut(
  targetPath: string,
  sourceName: string,
  destDir: string,
  runtime: RuntimeBridge,
): Promise<void> {
  const baseName = sourceName.toLowerCase().endsWith(".lnk")
    ? sourceName.slice(0, -4)
    : sourceName;
  const shortcutName = await uniqueName(destDir, `${baseName}.lnk`, runtime);
  const bytes = new TextEncoder().encode(targetPath);
  await runtime.mountFile(`${destDir}\\${shortcutName}`, bytes.buffer);
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
  emitFsChanged();
  return uploaded;
}

export function toPayloadEntry(entry: DirectoryEntry): FilePayloadEntry {
  return { path: entry.path, name: entry.name, kind: entry.kind };
}

export function encodeDragPayload(entries: FilePayloadEntry[]): string {
  return JSON.stringify(entries);
}

export function decodeDragPayload(data: string): FilePayloadEntry[] {
  try {
    const parsed = JSON.parse(data) as FilePayloadEntry[];
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (entry) =>
        typeof entry.path === "string" &&
        typeof entry.name === "string" &&
        (entry.kind === "file" || entry.kind === "directory"),
    );
  } catch {
    return [];
  }
}

export function emitFsChanged(): void {
  window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
}

async function copyNode(
  entry: FilePayloadEntry | ClipboardEntry,
  destDir: string,
  runtime: RuntimeBridge,
  preserveName: boolean,
): Promise<void> {
  if (entry.kind === "directory" || (!entry.kind && await isDirectory(entry.path, runtime))) {
    await copyDirectory(entry.path, destDir, entry.name, runtime, preserveName);
    return;
  }

  const destName = preserveName ? entry.name : await uniqueName(destDir, entry.name, runtime);
  const bytes = await runtime.readRawFile(entry.path);
  const buffer = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
  await runtime.mountFile(`${destDir}\\${destName}`, buffer);
}

async function copyDirectory(
  sourcePath: string,
  destDir: string,
  name: string,
  runtime: RuntimeBridge,
  preserveName: boolean,
): Promise<void> {
  const normalizedSource = normalizePath(sourcePath);
  const normalizedDest = normalizePath(destDir);
  if (samePath(normalizedSource, normalizedDest) || isDescendantPath(normalizedDest, normalizedSource)) {
    throw new Error("Cannot copy a folder into itself.");
  }

  const destName = preserveName ? name : await uniqueName(destDir, name, runtime);
  const destPath = `${destDir}\\${destName}`;
  await runtime.createDirectory(destPath);

  const children = await runtime.listDir(sourcePath);
  for (const child of children) {
    await copyNode(child, destPath, runtime, true);
  }
}

async function isDirectory(path: string, runtime: RuntimeBridge): Promise<boolean> {
  try {
    await runtime.listDir(path);
    return true;
  } catch {
    return false;
  }
}

async function uniqueName(destDir: string, name: string, runtime: RuntimeBridge): Promise<string> {
  const entries = await runtime.listDir(destDir);
  const existing = new Set(entries.map((entry) => entry.name.toLowerCase()));
  if (!existing.has(name.toLowerCase())) return name;

  const dot = name.lastIndexOf(".");
  const hasExtension = dot > 0;
  const stem = hasExtension ? name.slice(0, dot) : name;
  const ext = hasExtension ? name.slice(dot) : "";

  for (let i = 2; i < 1000; i++) {
    const candidate = `${stem} (${i})${ext}`;
    if (!existing.has(candidate.toLowerCase())) return candidate;
  }
  throw new Error(`Could not find a free name for ${name}.`);
}

function normalizePath(path: string): string {
  const trimmed = path.trim().replace(/\//g, "\\");
  if (/^[a-z]:\\?$/i.test(trimmed)) return `${trimmed[0].toUpperCase()}:\\`;
  return trimmed.replace(/\\+$/g, "");
}

function parentPath(path: string): string {
  const normalized = normalizePath(path);
  const idx = normalized.lastIndexOf("\\");
  if (idx <= 2) return `${normalized[0].toUpperCase()}:\\`;
  return normalized.slice(0, idx);
}

function samePath(a: string, b: string): boolean {
  return normalizePath(a).toLowerCase() === normalizePath(b).toLowerCase();
}

function isDescendantPath(path: string, parent: string): boolean {
  const normalizedPath = normalizePath(path).toLowerCase();
  const normalizedParent = normalizePath(parent).toLowerCase();
  return normalizedPath.startsWith(`${normalizedParent}\\`);
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
