import type { DirectoryEntry } from "../worker.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import { showContextMenu, SEPARATOR } from "./context-menu.js";
import { clipboardSet, clipboardHas, clipboardPaste } from "./clipboard.js";
import { showInputDialog } from "./dialog.js";
import { log } from "../log.js";

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

export function initDesktopContextMenu(runtime: RuntimeBridge) {
  const desktop = document.getElementById("desktop")!;

  desktop.addEventListener("contextmenu", (e) => {
    const target = e.target as HTMLElement;
    if (target.closest(".desktop-icon")) return; // handled by icon

    e.preventDefault();
    showContextMenu(e.clientX, e.clientY, [
      {
        label: "New File",
        action: () => showInputDialog({
          title: "New file name",
          placeholder: "untitled.txt",
          onConfirm: async (name) => {
            await runtime.mountFile(`${DESKTOP_PATH}\\${name}`, new ArrayBuffer(0));
            await refreshDesktop(runtime);
          },
        }),
      },
      {
        label: "New Folder",
        action: () => showInputDialog({
          title: "New folder name",
          placeholder: "New Folder",
          onConfirm: async (name) => {
            try {
              await runtime.createDirectory(`${DESKTOP_PATH}\\${name}`);
            } catch (err) {
              log("fs", `create folder failed: ${err}`, "error");
            }
            await refreshDesktop(runtime);
          },
        }),
      },
      SEPARATOR,
      {
        label: "Paste",
        disabled: !clipboardHas(),
        action: () => clipboardPaste(DESKTOP_PATH, runtime, () => refreshDesktop(runtime)),
      },
    ]);
  });
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

  icon.addEventListener("click", (e) => {
    e.stopPropagation();
    document.querySelectorAll(".desktop-icon.selected").forEach((el) => {
      if (el !== icon) el.classList.remove("selected");
    });
    icon.classList.add("selected");

    clicks++;
    if (clicks === 1) {
      clickTimer = setTimeout(() => { clicks = 0; }, 400);
    } else if (clicks >= 2) {
      if (clickTimer) clearTimeout(clickTimer);
      clicks = 0;
      handleRun(entry, runtime);
    }
  });

  icon.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    e.stopPropagation();

    document.querySelectorAll(".desktop-icon.selected").forEach((el) => {
      if (el !== icon) el.classList.remove("selected");
    });
    icon.classList.add("selected");

    showContextMenu(e.clientX, e.clientY, buildIconMenu(entry, runtime));
  });

  return icon;
}

function buildIconMenu(entry: DirectoryEntry, runtime: RuntimeBridge) {
  const isExe = entry.name.toLowerCase().endsWith(".exe");
  const isFile = entry.kind === "file";

  const items = [];

  if (isExe) {
    items.push({
      label: "Run",
      action: () => import("../windows/process-console.js").then((m) =>
        m.openProcessConsole(entry.path, runtime)
      ),
    });
    items.push({
      label: "Run as debug",
      action: () => import("../windows/process-console.js").then((m) =>
        m.openProcessConsole(entry.path, runtime, { debug: true })
      ),
    });
    items.push({
      label: "Inspect",
      action: () => import("../windows/pe-inspector.js").then((m) =>
        m.openPeInspector(entry.path, runtime)
      ),
    });
  } else {
    items.push({
      label: "Open",
      action: () => handleRun(entry, runtime),
    });
  }

  items.push(SEPARATOR);

  items.push({
    label: "Copy",
    disabled: !isFile,
    action: () => {
      clipboardSet(entry.path, entry.name, "copy");
      document.querySelectorAll(".desktop-icon").forEach((el) =>
        (el as HTMLElement).classList.remove("cut")
      );
    },
  });

  items.push({
    label: "Cut",
    disabled: !isFile,
    action: () => {
      clipboardSet(entry.path, entry.name, "cut");
      const icon = document.querySelector(`.desktop-icon[data-path="${CSS.escape(entry.path)}"]`);
      if (icon) icon.classList.add("cut");
    },
  });

  items.push({
    label: "Rename",
    action: () => showInputDialog({
      title: "Rename",
      initial: entry.name,
      onConfirm: async (newName) => {
        try {
          await runtime.renameNode(entry.path, newName);
          await refreshDesktop(runtime);
        } catch (err) {
          log("fs", `rename failed: ${err}`, "error");
        }
      },
    }),
  });

  items.push({
    label: "Delete",
    danger: true,
    action: async () => {
      try {
        await runtime.deleteNode(entry.path);
        await refreshDesktop(runtime);
      } catch (err) {
        log("fs", `delete failed: ${err}`, "error");
      }
    },
  });

  items.push(SEPARATOR);

  items.push({
    label: "Properties",
    action: () => import("../windows/properties.js").then((m) =>
      m.openProperties(entry)
    ),
  });

  return items;
}

function handleRun(entry: DirectoryEntry, runtime: RuntimeBridge) {
  const name = entry.name.toLowerCase();

  if (entry.kind === "directory") {
    import("../windows/explorer.js").then((m) =>
      m.openExplorer(entry.path, runtime)
    );
  } else if (name.endsWith(".exe")) {
    import("../windows/process-console.js").then((m) =>
      m.openProcessConsole(entry.path, runtime)
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
