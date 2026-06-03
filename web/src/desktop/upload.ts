import type { RuntimeBridge } from "../runtime-bridge.js";
import { log } from "../log.js";

const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";

export function initUpload(runtime: RuntimeBridge, onUploaded: () => void) {
  const fileInput = document.getElementById("file-input") as HTMLInputElement;
  const folderInput = document.getElementById("folder-input") as HTMLInputElement;

  window.addEventListener("webwine:upload-file", () => fileInput.click());
  window.addEventListener("webwine:upload-folder", () => folderInput.click());

  fileInput.addEventListener("change", async () => {
    if (!fileInput.files) return;
    await mountFiles(Array.from(fileInput.files), runtime);
    fileInput.value = "";
    onUploaded();
  });

  folderInput.addEventListener("change", async () => {
    if (!folderInput.files) return;
    await mountFiles(Array.from(folderInput.files), runtime, { preserveRelativePath: true });
    folderInput.value = "";
    onUploaded();
  });

  const desktop = document.getElementById("desktop")!;

  desktop.addEventListener("dragover", (e) => {
    e.preventDefault();
    desktop.classList.add("drag-over");
  });

  desktop.addEventListener("dragleave", () => {
    desktop.classList.remove("drag-over");
  });

  desktop.addEventListener("drop", async (e) => {
    e.preventDefault();
    desktop.classList.remove("drag-over");
    const files = e.dataTransfer?.files;
    if (!files) return;
    await mountFiles(Array.from(files), runtime);
    onUploaded();
  });
}

async function mountFiles(
  files: File[],
  runtime: RuntimeBridge,
  opts: { preserveRelativePath?: boolean } = {},
) {
  const createdDirs = new Set<string>();

  for (const file of files) {
    const relativePath =
      opts.preserveRelativePath && file.webkitRelativePath
        ? file.webkitRelativePath.replace(/\//g, "\\")
        : file.name;
    const guestPath = `${DESKTOP_PATH}\\${relativePath}`;
    const buffer = await file.arrayBuffer();
    try {
      await ensureParentDirs(guestPath, runtime, createdDirs);
      await runtime.mountFile(guestPath, buffer);
      log("fs", `uploaded ${relativePath} (${file.size} bytes)`);
    } catch (err) {
      log("fs", `failed to mount ${relativePath}: ${err}`, "error");
    }
  }
}

async function ensureParentDirs(path: string, runtime: RuntimeBridge, createdDirs: Set<string>) {
  const parts = path.split("\\");
  parts.pop();

  let current = parts[0];
  for (const part of parts.slice(1)) {
    current = `${current}\\${part}`;
    if (current === DESKTOP_PATH || !current.startsWith(`${DESKTOP_PATH}\\`)) continue;
    if (createdDirs.has(current)) continue;

    try {
      await runtime.createDirectory(current);
    } catch {
      // Existing directories are fine; mountFile will report real path errors.
    }
    createdDirs.add(current);
  }
}
