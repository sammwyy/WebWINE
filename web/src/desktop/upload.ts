import type { RuntimeBridge } from "../runtime-bridge.js";
import { log } from "../log.js";

const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";

export function initUpload(runtime: RuntimeBridge, onUploaded: () => void) {
  const btn = document.getElementById("upload-btn")!;
  const input = document.getElementById("file-input") as HTMLInputElement;

  btn.addEventListener("click", () => input.click());

  input.addEventListener("change", async () => {
    if (!input.files) return;
    await mountFiles(Array.from(input.files), runtime);
    input.value = "";
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

async function mountFiles(files: File[], runtime: RuntimeBridge) {
  for (const file of files) {
    const guestPath = `${DESKTOP_PATH}\\${file.name}`;
    const buffer = await file.arrayBuffer();
    try {
      await runtime.mountFile(guestPath, buffer);
      log("fs", `uploaded ${file.name} (${file.size} bytes)`);
    } catch (err) {
      log("fs", `failed to mount ${file.name}: ${err}`, "error");
    }
  }
}
