import type { DirectoryEntry } from "../worker.js";
import type { RuntimeBridge } from "../runtime-bridge.js";

const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";

export async function refreshDesktop(runtime: RuntimeBridge) {
  const grid = document.getElementById("icon-grid")!;
  grid.innerHTML = "";

  let entries: DirectoryEntry[];
  try {
    entries = await runtime.listDir(DESKTOP_PATH);
  } catch (err) {
    console.error("Failed to list desktop:", err);
    return;
  }

  for (const entry of entries) {
    grid.appendChild(buildIcon(entry, runtime));
  }
}

function buildIcon(entry: DirectoryEntry, runtime: RuntimeBridge): HTMLElement {
  const icon = document.createElement("div");
  icon.className = "desktop-icon";
  icon.dataset.path = entry.path;

  const img = document.createElement("div");
  img.className = "desktop-icon-img";
  img.textContent = iconEmoji(entry);

  const label = document.createElement("div");
  label.className = "desktop-icon-label";
  label.textContent = entry.name;

  icon.append(img, label);

  let clicks = 0;
  let clickTimer: ReturnType<typeof setTimeout> | null = null;

  icon.addEventListener("click", () => {
    clicks++;
    if (clicks === 1) {
      icon.classList.add("selected");
      clickTimer = setTimeout(() => { clicks = 0; }, 400);
    } else if (clicks === 2) {
      if (clickTimer) clearTimeout(clickTimer);
      clicks = 0;
      handleOpen(entry, runtime);
    }
  });

  document.addEventListener("click", (e) => {
    if (!icon.contains(e.target as Node)) {
      icon.classList.remove("selected");
    }
  });

  return icon;
}

function handleOpen(entry: DirectoryEntry, runtime: RuntimeBridge) {
  const name = entry.name.toLowerCase();

  if (entry.kind === "directory") {
    import("../windows/explorer.js").then((m) =>
      m.openExplorer(entry.path, runtime)
    );
  } else if (name.endsWith(".exe")) {
    import("../windows/process-window.js").then((m) =>
      m.openProcessWindow(entry.path, runtime)
    );
  } else {
    import("../windows/raw-viewer.js").then((m) =>
      m.openRawViewer(entry.path, runtime)
    );
  }
}

function iconEmoji(entry: DirectoryEntry): string {
  if (entry.kind === "directory") return "📁";
  const n = entry.name.toLowerCase();
  if (n.endsWith(".exe")) return "⚙";
  if (n.endsWith(".txt") || n.endsWith(".log")) return "📄";
  if (n.endsWith(".bmp") || n.endsWith(".png") || n.endsWith(".jpg")) return "🖼";
  return "📃";
}
