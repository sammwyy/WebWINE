/**
 * Desktop — the main desktop surface: wallpaper, icon grid, drag-and-drop.
 *
 * Manages file upload inputs (hidden), drag-and-drop zone, and desktop icon
 * context menu (right-click on empty area). Listens for theme/fs change events
 * to refresh icons.
 */

import { useEffect, useRef, useState, useCallback, useMemo } from "react";
import { useRuntimeStore } from "../../stores/useRuntimeStore.js";
import { useDesktopStore } from "../../stores/useDesktopStore.js";
import { useClipboardStore } from "../../stores/useClipboardStore.js";
import { log } from "../../stores/useLogStore.js";
import { mountFiles, performPaste } from "../../lib/clipboard.js";
import { DesktopIcon } from "./DesktopIcon.js";
import { ContextMenu, SEPARATOR, type MenuItem } from "./ContextMenu.js";
import styles from "./Desktop.module.css";
import { WindowLayer } from "../window/WindowLayer.js";

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
  } = useDesktopStore();
  const clipboard = useClipboardStore();
  const iconSizeOptions = [
    { key: "small" as const, label: "Small icons" },
    { key: "medium" as const, label: "Medium icons" },
    { key: "large" as const, label: "Large icons" },
  ];

  const gridRef = useRef<HTMLDivElement>(null);
  const [dragOver, setDragOver] = useState(false);
  const [ctx, setCtx] = useState<{ x: number; y: number } | null>(null);
  const [renameEntry, setRenameEntry] = useState<import("../../lib/worker.js").DirectoryEntry | null>(null);
  const [renameValue, setRenameValue] = useState("");

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
  } as React.CSSProperties;

  // Initial load and event-based refreshes.
  useEffect(() => {
    doRefresh();
  }, [doRefresh]);

  useEffect(() => {
    const handler = () => doRefresh();
    window.addEventListener("webwine:fs-changed", handler);
    window.addEventListener("webwine:theme-changed", handler);
    return () => {
      window.removeEventListener("webwine:fs-changed", handler);
      window.removeEventListener("webwine:theme-changed", handler);
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

  const buildDesktopMenu = useCallback((): MenuItem[] => {
    const sizeItems = iconSizeOptions.map((option) => ({
      label: option.label,
      disabled: iconSize === option.key,
      action: () => setIconSize(option.key),
    }));

    return [
      ...sizeItems,
      SEPARATOR,
      {
        label: "New File",
        disabled: !runtime,
        action: () => {
          setRenameEntry({ name: "", path: "", kind: "file", size: 0 });
          setRenameValue("untitled.txt");
        },
      },
      {
        label: "New Folder",
        disabled: !runtime,
        action: () => {
          setRenameEntry({ name: "", path: "", kind: "directory", size: 0 });
          setRenameValue("New Folder");
        },
      },
      SEPARATOR,
      {
        label: "Paste",
        disabled: !runtime || !clipboard.has(),
        action: async () => {
          if (!runtime || !clipboard.entry) return;
          try {
            await performPaste(clipboard.entry, DESKTOP_PATH, runtime);
            if (clipboard.entry.op === "cut") clipboard.clear();
            doRefresh();
          } catch (err) {
            log("fs", `paste failed: ${err}`, "error");
          }
        },
      },
    ];
  }, [clipboard, runtime, doRefresh, iconSize, setIconSize]);

  const confirmRename = useCallback(async () => {
    if (!runtime || !renameEntry || !renameValue.trim()) return;
    const name = renameValue.trim();
    try {
      if (renameEntry.path) {
        await runtime.renameNode(renameEntry.path, name);
      } else if (renameEntry.kind === "directory") {
        await runtime.createDirectory(`${DESKTOP_PATH}\\${name}`);
      } else {
        await runtime.mountFile(`${DESKTOP_PATH}\\${name}`, new ArrayBuffer(0));
      }
      doRefresh();
    } catch (err) {
      log("fs", `operation failed: ${err}`, "error");
    }
    setRenameEntry(null);
  }, [runtime, renameEntry, renameValue, doRefresh]);

  return (
    <div
      id="desktop"
      className={`${styles.desktop} ${dragOver ? styles["drag-over"] + " drag-over" : ""}`}
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
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={async (e) => {
        e.preventDefault();
        setDragOver(false);
        const files = e.dataTransfer?.files;
        if (files) await handleFilesUploaded(Array.from(files));
      }}
    >
      <div id="icon-grid" className={styles["icon-grid"]} ref={gridRef}>
        {entries.map((entry) => (
          <DesktopIcon
            key={entry.path}
            entry={entry}
            position={
              positions[entry.path] ?? { col: 0, row: 0 }
            }
            gridEl={gridRef.current}
            onRefresh={doRefresh}
            onRename={(e) => {
              setRenameEntry(e);
              setRenameValue(e.name);
            }}
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

      {/* Inline rename / new file dialog */}
      {renameEntry !== null && (
        <div
          className="ctx-menu"
          style={{ position: "fixed", left: "50%", top: "40%", transform: "translate(-50%,-50%)", padding: 12 }}
        >
          <input
            className="dialog-input"
            autoFocus
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void confirmRename();
              if (e.key === "Escape") setRenameEntry(null);
            }}
            style={{ marginBottom: 8, display: "block", width: "100%" }}
          />
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
            <button className="dialog-btn dialog-btn-default" type="button" onClick={confirmRename}>OK</button>
            <button className="dialog-btn" type="button" onClick={() => setRenameEntry(null)}>Cancel</button>
          </div>
        </div>
      )}
    </div>
  );
}
