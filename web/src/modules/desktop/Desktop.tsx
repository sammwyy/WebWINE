/**
 * Desktop — the main desktop surface: wallpaper, icon grid, drag-and-drop.
 *
 * Manages file upload inputs (hidden), drag-and-drop zone, and desktop icon
 * context menu (right-click on empty area).
 */

import { useEffect, useRef, useState, useCallback, useMemo } from "react";
import { useRuntimeStore } from "@/state/runtimeStore";
import {
  useDesktopStore,
  DESKTOP_ICON_LAYOUTS,
  DESKTOP_GRID_MAX_ROWS,
  pixelToGrid,
  findFreeSlot,
  occupiedCells,
  type IconPosition,
} from "@/state/desktopStore";
import { useClipboardStore } from "@/state/clipboardStore";
import { log } from "@/state/logStore";
import {
  copyPayloadToDir,
  decodeDragPayload,
  mountFiles,
  pasteShortcut,
  performPaste,
  parentPath,
} from "@/shared/lib/clipboard";
import { DesktopIcon, isDesktopIconDragActive } from "./DesktopIcon";
import { ContextMenu, SEPARATOR, type MenuItem } from "./ContextMenu";
import { WindowLayer } from "../windows/WindowManager";
import { useSettingsStore } from "@/state/settingsStore";
import { useCurrentTheme } from "@/shared/lib/theme-registry";
import { wallpaperStyle } from "@/shared/lib/personalization";
import { showPrompt } from "@/apps/prompt/PromptDialog";
import { openControlPanel } from "@/apps/control-panel/ControlPanelApp";
import type { DirectoryEntry } from "@/core/wasm/worker";

function isInternalDesktopDrag(dt: DataTransfer): boolean {
  if (isDesktopIconDragActive()) return true;
  const types = Array.from(dt.types ?? []);
  // Firefox may only expose "text/plain" during dragover.
  return (
    types.includes("application/x-webwine-desktop-drag") ||
    types.includes("application/x-webwine-paths") ||
    types.some((t) => t.toLowerCase() === "application/x-webwine-desktop-drag")
  );
}

const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";

interface DesktopProps {
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  folderInputRef: React.RefObject<HTMLInputElement | null>;
}

export function Desktop({ fileInputRef, folderInputRef }: DesktopProps) {
  const { runtime } = useRuntimeStore();
  const {
    entries,
    positions,
    refresh,
    clearSelection,
    iconSize,
    setIconSize,
    selectedIds,
    setPositions,
  } = useDesktopStore();
  const clipboard = useClipboardStore();
  const gridRef = useRef<HTMLDivElement>(null);
  const [dragOver, setDragOver] = useState(false);
  const [ctx, setCtx] = useState<{ x: number; y: number } | null>(null);
  const theme = useCurrentTheme();
  const settings = useSettingsStore();
  const { showDesktopIcons } = settings;
  const wallpaper = wallpaperStyle(settings, theme);

  const doRefresh = useCallback(() => {
    if (runtime) refresh(runtime, DESKTOP_PATH);
  }, [runtime, refresh]);

  const layout = useMemo(
    () => {
      switch (iconSize) {
        case "small":
          return { iconSize: 52, cellWidth: 80, cellHeight: 92 };
        case "large":
          return { iconSize: 88, cellWidth: 112, cellHeight: 120 };
        default:
          return { iconSize: 72, cellWidth: 96, cellHeight: 104 };
      }
    },
    [iconSize],
  );

  const desktopStyle = {
    "--icon-size": `${layout.iconSize}px`,
    "--desktop-icon-cell-w": `${layout.cellWidth}px`,
    "--desktop-icon-cell-h": `${layout.cellHeight}px`,
    bottom: "var(--ww-taskbar-h)",
  } as React.CSSProperties;

  // Initial load and event-based refreshes.
  useEffect(() => {
    doRefresh();
  }, [doRefresh]);

  useEffect(() => {
    const handler = () => doRefresh();
    window.addEventListener("webwine:fs-changed", handler);
    return () => {
      window.removeEventListener("webwine:fs-changed", handler);
    };
  }, [doRefresh]);

  // Upload handlers.
  const handleFilesUploaded = useCallback(
    async (files: File[], opts?: { preserveRelativePath?: boolean }) => {
      if (!runtime) return;
      try {
        const uploaded = await mountFiles(files, DESKTOP_PATH, runtime, opts);
        for (const name of uploaded) {
          log("fs", `uploaded ${name}`);
        }
      } catch (err) {
        log("fs", `upload failed: ${err}`, "error");
      }
      doRefresh();
    },
    [runtime, doRefresh],
  );

  const selectedClipboardEntries = useCallback(
    () =>
      entries
        .filter((entry) => selectedIds.includes(entry.path))
        .map((entry) => ({
          path: entry.path,
          name: entry.name,
          kind: entry.kind,
        })),
    [entries, selectedIds],
  );

  /** Rename `entry`, or create it on the desktop when it has no path yet. */
  const promptName = useCallback(
    async (entry: DirectoryEntry, initial: string) => {
      const title = entry.path ? "Rename" : entry.kind === "directory" ? "New Folder" : "New Text Document";
      const name = await showPrompt({ title, label: entry.path ? "New name:" : "Name:", value: initial });
      if (!runtime || !name) return;
      try {
        if (entry.path) {
          await runtime.renameNode(entry.path, name);
        } else if (entry.kind === "directory") {
          await runtime.createDirectory(`${DESKTOP_PATH}\\${name}`);
        } else {
          await runtime.mountFile(`${DESKTOP_PATH}\\${name}`, new ArrayBuffer(0));
        }
        doRefresh();
      } catch (err) {
        log("fs", `operation failed: ${err}`, "error");
      }
    },
    [runtime, doRefresh],
  );

  const buildDesktopMenu = useCallback((): MenuItem[] => {
    const pasteDisabled = !runtime || !clipboard.has();

    const pasteAction = async () => {
      if (!runtime || !clipboard.has()) return;

      try {
        await performPaste(clipboard.entries, DESKTOP_PATH, runtime);

        if (clipboard.isCut()) {
          clipboard.clear();
        }

        doRefresh();
      } catch (err) {
        log("fs", `paste failed: ${err}`, "error");
      }
    };

    const pasteShortcutAction = async () => {
      if (!runtime || !clipboard.has()) return;
      try {
        await pasteShortcut(clipboard.entries, DESKTOP_PATH, runtime);
        doRefresh();
      } catch (err) {
        log("fs", `paste shortcut failed: ${err}`, "error");
      }
    };

    return [
      {
        label: "View",
        children: [
          {
            label: "Large icons",
            checked: iconSize === "large",
            action: () => setIconSize("large"),
          },
          {
            label: "Medium icons",
            checked: iconSize === "medium",
            action: () => setIconSize("medium"),
          },
          {
            label: "Small icons",
            checked: iconSize === "small",
            action: () => setIconSize("small"),
          },
          SEPARATOR,
          {
            label: "Auto arrange icons",
            disabled: true,
          },
          {
            label: "Align icons to grid",
            checked: true,
            disabled: true,
          },
          SEPARATOR,
          {
            label: "Show desktop icons",
            checked: showDesktopIcons,
            action: () => useSettingsStore.getState().set({ showDesktopIcons: !showDesktopIcons }),
          },
        ],
      },
      {
        label: "Sort by",
        children: [
          {
            label: "Name",
            disabled: true,
          },
          {
            label: "Size",
            disabled: true,
          },
          {
            label: "Item type",
            disabled: true,
          },
          {
            label: "Date modified",
            disabled: true,
          },
        ],
      },
      {
        label: "Refresh",
        disabled: !runtime,
        action: doRefresh,
      },
      {
        label: "Upload file here",
        disabled: !runtime,
        action: () => fileInputRef.current?.click(),
      },
      {
        label: "Upload folder here",
        disabled: !runtime,
        action: () => folderInputRef.current?.click(),
      },
      SEPARATOR,
      {
        label: "Paste",
        disabled: pasteDisabled,
        action: pasteAction,
      },
      {
        label: "Paste shortcut",
        disabled: pasteDisabled,
        action: pasteShortcutAction,
      },
      SEPARATOR,
      {
        label: "New",
        children: [
          {
            label: "Folder",
            disabled: !runtime,
            action: () => void promptName({ name: "", path: "", kind: "directory", size: 0 }, "New Folder"),
          },
          {
            label: "Shortcut",
            disabled: true,
          },
          SEPARATOR,
          {
            label: "Text Document",
            disabled: !runtime,
            action: () =>
              void promptName({ name: "", path: "", kind: "file", size: 0 }, "New Text Document.txt"),
          },
        ],
      },
      SEPARATOR,
      {
        label: "Display settings",
        disabled: !runtime,
        action: () => runtime && openControlPanel(runtime, "desktop"),
      },
      {
        label: "Personalize",
        disabled: !runtime,
        action: () => runtime && openControlPanel(runtime, "personalization"),
      },
    ];
  }, [
    clipboard,
    runtime,
    doRefresh,
    iconSize,
    setIconSize,
    fileInputRef,
    folderInputRef,
    promptName,
    showDesktopIcons,
  ]);

  useEffect(() => {
    const onKeyDown = async (e: KeyboardEvent) => {
      if (!runtime || !(e.ctrlKey || e.metaKey)) return;
      if (document.activeElement instanceof HTMLInputElement) return;
      if (
        document.activeElement instanceof HTMLElement &&
        document.activeElement.closest("#window-layer")
      ) {
        return;
      }

      const key = e.key.toLowerCase();
      if (!["x", "c", "v"].includes(key)) return;

      const selected = selectedClipboardEntries();
      if ((key === "x" || key === "c") && selected.length === 0) return;

      e.preventDefault();
      try {
        if (key === "x") {
          clipboard.setMany(selected, "cut");
        } else if (key === "c") {
          clipboard.setMany(selected, "copy");
        } else if (clipboard.has()) {
          await performPaste(clipboard.entries, DESKTOP_PATH, runtime);
          if (clipboard.isCut()) clipboard.clear();
          doRefresh();
        }
      } catch (err) {
        log("fs", `clipboard operation failed: ${err}`, "error");
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [clipboard, doRefresh, runtime, selectedClipboardEntries]);

  return (
    <div
      id="desktop"
      className={`ww-desktop ${wallpaper ? "has-wallpaper" : ""} !fixed top-0 left-0 right-0 !h-auto overflow-hidden ${dragOver ? "outline-2 outline-dashed outline-[var(--accent)] outline-offset-[-4px]" : ""}`}
      style={desktopStyle}
      onClick={(e) => {
        if (e.target === e.currentTarget || (e.target as HTMLElement).id === "icon-grid") {
          clearSelection();
        }
      }}
      onContextMenu={(e) => {
        if ((e.target as HTMLElement).closest(".desktop-icon")) return;
        e.preventDefault();
        setCtx({ x: e.clientX, y: e.clientY });
      }}
      onDragOver={(e) => {
        e.preventDefault();
        e.stopPropagation();
        if (isInternalDesktopDrag(e.dataTransfer)) {
          // Icon rearrange — never show the "upload files" outline.
          e.dataTransfer.dropEffect = e.ctrlKey ? "copy" : "move";
          setDragOver(false);
        } else {
          e.dataTransfer.dropEffect = "copy";
          setDragOver(true);
        }
      }}
      onDragLeave={(e) => {
        // Ignore leave events that bubble from child icons during rearrange.
        if (isDesktopIconDragActive()) return;
        if (e.currentTarget.contains(e.relatedTarget as Node)) return;
        setDragOver(false);
      }}
      onDrop={async (e) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(false);
        let pathData = e.dataTransfer.getData("application/x-webwine-paths");
        if (!pathData) {
          // Firefox fallback written as text/plain.
          const plain = e.dataTransfer.getData("text/plain");
          if (plain.startsWith("webwine-desktop:")) {
            pathData = plain.slice("webwine-desktop:".length);
          }
        }
        const payload = decodeDragPayload(pathData);
        const offsetRaw = e.dataTransfer.getData(
          "application/x-webwine-drag-offset",
        );
        type DragOffset = { x: number; y: number; path: string };
        let offsets: DragOffset | null = null;
        if (offsetRaw) {
          try {
            const parsed = JSON.parse(offsetRaw) as Partial<DragOffset>;
            if (
              typeof parsed.x === "number" &&
              typeof parsed.y === "number" &&
              typeof parsed.path === "string"
            ) {
              offsets = { x: parsed.x, y: parsed.y, path: parsed.path };
            }
          } catch {
            offsets = null;
          }
        }

        if (runtime && payload.length > 0) {
          const isFromDesktop = payload.every(
            (p) =>
              parentPath(p.path).toLowerCase() === DESKTOP_PATH.toLowerCase(),
          );

          if (isFromDesktop) {
            const gridEl = gridRef.current;
            if (gridEl) {
              const layout = DESKTOP_ICON_LAYOUTS[iconSize];
              const gridRect = gridEl.getBoundingClientRect();
              const movingPaths = payload.map((p) => p.path);

              // Target cell under the grab point (or cursor if no offset).
              const prefer = offsets?.path
                ? pixelToGrid(
                    e.clientX - offsets.x - gridRect.left,
                    e.clientY - offsets.y - gridRect.top,
                    layout.cellWidth,
                    layout.cellHeight,
                  )
                : pixelToGrid(
                    e.clientX - gridRect.left,
                    e.clientY - gridRect.top,
                    layout.cellWidth,
                    layout.cellHeight,
                  );

              const grabbedPath = offsets?.path ?? payload[0]?.path;
              const oldGrabbed =
                (grabbedPath && positions[grabbedPath]) || { col: 0, row: 0 };
              const dCol = prefer.col - oldGrabbed.col;
              const dRow = prefer.row - oldGrabbed.row;

              // Cells already taken by icons that are not being moved.
              const occupied = occupiedCells(positions, movingPaths);
              const updates: Record<string, IconPosition> = {};

              // Place the grabbed icon first at prefer (or next free).
              if (grabbedPath) {
                const slot = findFreeSlot(
                  occupied,
                  DESKTOP_GRID_MAX_ROWS,
                  prefer,
                );
                occupied.add(`${slot.col},${slot.row}`);
                updates[grabbedPath] = slot;
              }

              // Others keep relative delta when free; otherwise claim free slots.
              for (const p of payload) {
                if (p.path === grabbedPath) continue;
                const oldPos = positions[p.path] ?? { col: 0, row: 0 };
                const desired: IconPosition = {
                  col: Math.max(0, oldPos.col + dCol),
                  row: Math.max(0, oldPos.row + dRow),
                };
                const slot = findFreeSlot(
                  occupied,
                  DESKTOP_GRID_MAX_ROWS,
                  desired,
                );
                occupied.add(`${slot.col},${slot.row}`);
                updates[p.path] = slot;
              }

              setPositions(updates);
            }
          } else {
            try {
              await copyPayloadToDir(payload, DESKTOP_PATH, runtime, !e.ctrlKey);
              doRefresh();
            } catch (err) {
              log("fs", `drop failed: ${err}`, "error");
            }
          }
          return;
        }
        const files = e.dataTransfer?.files;
        if (files) await handleFilesUploaded(Array.from(files));
      }}
    >
      {wallpaper && <div className="ww-wallpaper" style={wallpaper} aria-hidden="true" />}

      <div id="icon-grid" className="absolute inset-0" ref={gridRef}>
        {showDesktopIcons &&
          entries.map((entry) => (
            <DesktopIcon
              key={entry.path}
              entry={entry}
              position={positions[entry.path] ?? { col: 0, row: 0 }}
              gridEl={gridRef.current}
              onRefresh={doRefresh}
              onRename={(e) => void promptName(e, e.name)}
            />
          ))}
      </div>

      <WindowLayer />

      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          items={buildDesktopMenu()}
          onClose={() => setCtx(null)}
        />
      )}

      {/* Hidden file upload inputs */}
      <input
        ref={fileInputRef as React.RefObject<HTMLInputElement>}
        id="file-input"
        type="file"
        multiple
        hidden
        onChange={async (e) => {
          if (!e.target.files) return;
          await handleFilesUploaded(Array.from(e.target.files));
          e.target.value = "";
        }}
      />
      <input
        ref={folderInputRef as React.RefObject<HTMLInputElement>}
        id="folder-input"
        type="file"
        multiple
        // @ts-expect-error webkitdirectory is non-standard
        webkitdirectory="true"
        hidden
        onChange={async (e) => {
          if (!e.target.files) return;
          await handleFilesUploaded(Array.from(e.target.files), {
            preserveRelativePath: true,
          });
          e.target.value = "";
        }}
      />
    </div>
  );
}
