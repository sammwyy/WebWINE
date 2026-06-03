import type { DirectoryEntry } from "../worker.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import { showContextMenu, SEPARATOR } from "./context-menu.js";
import { clipboardSet, clipboardHas, clipboardPaste } from "./clipboard.js";
import { showInputDialog } from "./dialog.js";
import { log } from "../log.js";
import { resolveIcon, ICON_PLACEHOLDER } from "./icon-resolver.js";

const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";
const ICON_CELL_W = 88;
const ICON_CELL_H = 104;
const ICON_PAD = 14;
const ICON_POSITIONS_KEY = "webwine.desktop.iconPositions";

type IconPosition = { col: number; row: number };

let resizeInitialized = false;

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

  const positions = loadIconPositions();
  const activePaths = new Set(entries.map((entry) => entry.path));
  for (const path of Object.keys(positions)) {
    if (!activePaths.has(path)) delete positions[path];
  }

  const used = new Set<string>();
  for (const entry of entries) {
    const icon = buildIcon(entry, runtime);
    placeIcon(icon, entry.path, positions, used, grid);
    grid.appendChild(icon);
  }
  saveIconPositions(positions);
  initIconResizeHandler();
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

  // ── Icon image with optional overlay ────────────────────────────────────
  const wrap = document.createElement("div");
  wrap.className = "desktop-icon-img-wrap";

  const img = document.createElement("img");
  img.className = "desktop-icon-img";
  img.draggable = false;
  img.alt = "";
  img.src = ICON_PLACEHOLDER;

  wrap.appendChild(img);
  icon.appendChild(wrap);

  // Resolve asynchronously — updates the src once the icon is ready
  resolveIcon(entry, runtime).then((resolved) => {
    img.src = resolved.src;
    if (resolved.overlay) {
      let ov = wrap.querySelector<HTMLImageElement>(".desktop-icon-overlay");
      if (!ov) {
        ov = document.createElement("img");
        ov.className = "desktop-icon-overlay";
        ov.draggable = false;
        ov.alt = "";
        wrap.appendChild(ov);
      }
      ov.src = resolved.overlay;
    }
  }).catch(() => { /* keep placeholder */ });

  const label = document.createElement("div");
  label.className = "desktop-icon-label";
  label.textContent = entry.name;

  icon.append(label);

  let clicks = 0;
  let clickTimer: ReturnType<typeof setTimeout> | null = null;

  icon.addEventListener("click", (e) => {
    e.stopPropagation();
    if (icon.dataset.suppressClick === "true") return;
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

  makeMovableIcon(icon, entry.path);
  return icon;
}

function placeIcon(
  icon: HTMLElement,
  path: string,
  positions: Record<string, IconPosition>,
  used: Set<string>,
  grid: HTMLElement,
) {
  const pos = positions[path];
  const key = pos ? slotKey(pos) : "";
  const finalPos = pos && !used.has(key) ? pos : nextFreeSlot(used, grid);
  positions[path] = finalPos;
  used.add(slotKey(finalPos));
  applyIconPosition(icon, finalPos);
}

function makeMovableIcon(icon: HTMLElement, path: string) {
  icon.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    const grid = document.getElementById("icon-grid")!;
    const startX = e.clientX;
    const startY = e.clientY;
    const startLeft = icon.offsetLeft;
    const startTop = icon.offsetTop;
    let dragging = false;

    icon.setPointerCapture(e.pointerId);

    function onMove(ev: PointerEvent) {
      const dx = ev.clientX - startX;
      const dy = ev.clientY - startY;
      if (!dragging && Math.hypot(dx, dy) < 5) return;

      dragging = true;
      icon.classList.add("dragging");
      icon.style.left = `${clamp(startLeft + dx, ICON_PAD, grid.clientWidth - ICON_CELL_W)}px`;
      icon.style.top = `${clamp(startTop + dy, ICON_PAD, grid.clientHeight - ICON_CELL_H)}px`;
    }

    function onUp(ev: PointerEvent) {
      icon.releasePointerCapture(ev.pointerId);
      icon.removeEventListener("pointermove", onMove);
      icon.removeEventListener("pointerup", onUp);

      if (!dragging) return;

      icon.classList.remove("dragging");
      const requestedPos = {
        col: Math.max(0, Math.round((icon.offsetLeft - ICON_PAD) / ICON_CELL_W)),
        row: Math.max(0, Math.round((icon.offsetTop - ICON_PAD) / ICON_CELL_H)),
      };
      const positions = loadIconPositions();
      const used = new Set(
        Object.entries(positions)
          .filter(([otherPath]) => otherPath !== path)
          .map(([, pos]) => slotKey(pos)),
      );
      const pos = used.has(slotKey(requestedPos))
        ? nearestFreeSlot(requestedPos, used, grid)
        : requestedPos;
      positions[path] = pos;
      saveIconPositions(positions);
      applyIconPosition(icon, pos);

      icon.dataset.suppressClick = "true";
      setTimeout(() => {
        delete icon.dataset.suppressClick;
      }, 80);
    }

    icon.addEventListener("pointermove", onMove);
    icon.addEventListener("pointerup", onUp);
  });
}

function initIconResizeHandler() {
  if (resizeInitialized) return;
  resizeInitialized = true;
  window.addEventListener("resize", () => {
    const positions = loadIconPositions();
    document.querySelectorAll<HTMLElement>(".desktop-icon[data-path]").forEach((icon) => {
      const path = icon.dataset.path;
      if (!path || !positions[path]) return;
      applyIconPosition(icon, positions[path]);
    });
  });
}

function applyIconPosition(icon: HTMLElement, pos: IconPosition) {
  const grid = document.getElementById("icon-grid");
  const left = ICON_PAD + pos.col * ICON_CELL_W;
  const top = ICON_PAD + pos.row * ICON_CELL_H;

  icon.style.left = grid
    ? `${clamp(left, ICON_PAD, grid.clientWidth - ICON_CELL_W)}px`
    : `${left}px`;
  icon.style.top = grid
    ? `${clamp(top, ICON_PAD, grid.clientHeight - ICON_CELL_H)}px`
    : `${top}px`;
}

function nextFreeSlot(used: Set<string>, grid: HTMLElement): IconPosition {
  const columns = Math.max(1, Math.floor((grid.clientWidth - ICON_PAD * 2) / ICON_CELL_W));
  let index = 0;

  while (true) {
    const pos = {
      col: index % columns,
      row: Math.floor(index / columns),
    };
    if (!used.has(slotKey(pos))) return pos;
    index++;
  }
}

function nearestFreeSlot(origin: IconPosition, used: Set<string>, grid: HTMLElement): IconPosition {
  const columns = Math.max(1, Math.floor((grid.clientWidth - ICON_PAD * 2) / ICON_CELL_W));
  const rows = Math.max(1, Math.floor((grid.clientHeight - ICON_PAD * 2) / ICON_CELL_H));

  for (let radius = 0; radius < columns + rows; radius++) {
    for (let row = Math.max(0, origin.row - radius); row <= Math.min(rows - 1, origin.row + radius); row++) {
      for (let col = Math.max(0, origin.col - radius); col <= Math.min(columns - 1, origin.col + radius); col++) {
        const pos = { col, row };
        if (!used.has(slotKey(pos))) return pos;
      }
    }
  }

  return nextFreeSlot(used, grid);
}

function slotKey(pos: IconPosition): string {
  return `${pos.col}:${pos.row}`;
}

function loadIconPositions(): Record<string, IconPosition> {
  try {
    const raw = localStorage.getItem(ICON_POSITIONS_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, IconPosition>;
    return Object.fromEntries(
      Object.entries(parsed).filter(([, pos]) =>
        Number.isInteger(pos.col) && Number.isInteger(pos.row) && pos.col >= 0 && pos.row >= 0
      ),
    );
  } catch {
    return {};
  }
}

function saveIconPositions(positions: Record<string, IconPosition>) {
  try {
    localStorage.setItem(ICON_POSITIONS_KEY, JSON.stringify(positions));
  } catch {
    // Non-persistent storage should not break desktop rendering.
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), Math.max(min, max));
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

// ---------------------------------------------------------------------------
// Theme-change listener
// ---------------------------------------------------------------------------

/**
 * Listen for the webwine:theme-changed event and refresh the desktop so
 * every icon is re-resolved against the new theme.
 * Must be called once after the runtime is ready.
 */
export function initDesktopThemeListener(runtime: RuntimeBridge) {
  window.addEventListener("webwine:theme-changed", () => {
    refreshDesktop(runtime);
  });
}
