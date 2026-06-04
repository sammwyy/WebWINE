import { RuntimeBridge } from "@/core/bridge/runtime-bridge";

const SNAPSHOT_FILE_NAME = "vfs_snapshot.bin";

export async function loadVfsSnapshot(runtime: RuntimeBridge): Promise<boolean> {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(SNAPSHOT_FILE_NAME);
    const file = await fileHandle.getFile();
    const buffer = await file.arrayBuffer();
    await runtime.importVfs(new Uint8Array(buffer));
    return true;
  } catch (err) {
    if ((err as Error).name === "NotFoundError") {
      return false;
    }
    console.warn("Failed to load VFS snapshot from OPFS:", err);
    return false;
  }
}

export async function saveVfsSnapshot(runtime: RuntimeBridge): Promise<void> {
  try {
    const bytes = await runtime.exportVfs();
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(SNAPSHOT_FILE_NAME, { create: true });
    // @ts-expect-error FileSystemFileHandle.createWritable is in the File System Access API
    const writable = await fileHandle.createWritable();
    await writable.write(bytes);
    await writable.close();
  } catch (err) {
    console.error("Failed to save VFS snapshot to OPFS:", err);
  }
}

export async function exportVfsToFile(runtime: RuntimeBridge): Promise<void> {
  try {
    const bytes = await runtime.exportVfs();
    const blob = new Blob([bytes], { type: "application/octet-stream" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = SNAPSHOT_FILE_NAME;
    a.click();
    URL.revokeObjectURL(url);
  } catch (err) {
    console.error("Failed to export VFS to file:", err);
  }
}

export async function importVfsFromFile(runtime: RuntimeBridge): Promise<void> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".bin";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return resolve();
      try {
        const buffer = await file.arrayBuffer();
        await runtime.importVfs(new Uint8Array(buffer));
        await saveVfsSnapshot(runtime);
        resolve();
      } catch (err) {
        console.error("Failed to import VFS from file:", err);
        reject(err);
      }
    };
    input.click();
  });
}
