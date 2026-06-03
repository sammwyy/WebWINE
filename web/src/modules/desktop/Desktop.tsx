/**
 * Desktop — the main desktop surface: wallpaper, icon grid, drag-and-drop.
 *
 * Manages file upload inputs (hidden), drag-and-drop zone, and desktop icon
 * context menu (right-click on empty area).
 */

import { useEffect, useRef, useState, useCallback, useMemo } from "react";
import { useRuntimeStore } from "../../state/runtimeStore";
import { useDesktopStore } from "../../state/desktopStore";
import { useClipboardStore } from "../../state/clipboardStore";
import { log } from "../../state/logStore";
import { mountFiles, performPaste } from "../../shared/lib/clipboard";
import { DesktopIcon } from "./DesktopIcon";
import { ContextMenu, SEPARATOR, type MenuItem } from "./ContextMenu";
import { WindowLayer } from "../windows/WindowManager";

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
  const gridRef = useRef<HTMLDivElement>(null);
  const [dragOver, setDragOver] = useState(false);
  const [ctx, setCtx] = useState<{ x: number; y: number } | null>(null);
  const [renameEntry, setRenameEntry] = useState<import("../../core/wasm/worker").DirectoryEntry | null>(null);
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

  const buildDesktopMenu = useCallback((): MenuItem[] => {
    const pasteDisabled = !runtime || !clipboard.has();

    const pasteAction = async () => {
      if (!runtime || !clipboard.entry) return;

      try {
        await performPaste(clipboard.entry, DESKTOP_PATH, runtime);

        if (clipboard.entry.op === "cut") {
          clipboard.clear();
        }

        doRefresh();
      } catch (err) {
        log("fs", `paste failed: ${err}`, "error");
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
            checked: true,
            disabled: true,
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
      SEPARATOR,
      {
        label: "Paste",
        disabled: pasteDisabled,
        action: pasteAction,
      },
      {
        label: "Paste shortcut",
        disabled: true,
      },
      SEPARATOR,
      {
        label: "New",
        children: [
          {
            label: "Folder",
            disabled: !runtime,
            action: () => {
              setRenameEntry({
                name: "",
                path: "",
                kind: "directory",
                size: 0,
              });

              setRenameValue("New Folder");
            },
          },
          {
            label: "Shortcut",
            disabled: true,
          },
          SEPARATOR,
          {
            label: "Text Document",
            disabled: !runtime,
            action: () => {
              setRenameEntry({
                name: "",
                path: "",
                kind: "file",
                size: 0,
              });

              setRenameValue("New Text Document.txt");
            },
          },
        ],
      },
      SEPARATOR,
      {
        label: "Display settings",
        disabled: true,
      },
      {
        label: "Personalize",
        disabled: true,
      },
    ];
  }, [
    clipboard,
    runtime,
    doRefresh,
    iconSize,
    setIconSize,
  ]);

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
      className={`fixed inset-[0_0_40px_0] overflow-hidden ${dragOver ? "outline-2 outline-dashed outline-[var(--accent)] outline-offset-[-4px]" : ""}`}
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
      <div id="icon-grid" className="absolute inset-0" ref={gridRef}>
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
          className="fixed left-1/2 top-[40%] -translate-x-1/2 -translate-y-1/2 p-3 text-[#f2f2f2] border border-[rgba(255,255,255,0.1)] rounded-none shadow-[0_16px_42px_rgba(0,0,0,0.55)] backdrop-blur-[28px] backdrop-saturate-[1.35] z-[9600]"
          style={{ background: "linear-gradient(rgba(255,255,255,0.035), rgba(255,255,255,0.015)), rgba(31,31,31,0.88)" }}
        >
          <input
            className="block w-full mb-2 bg-[#1b1b1b] border border-[#5f5f5f] rounded-none text-white shadow-none outline-none hover:border-[#8a8a8a] focus:border-[#0078d7] focus:shadow-[inset_0_-2px_0_#0078d7] px-2 py-1 text-[13px]"
            autoFocus
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void confirmRename();
              if (e.key === "Escape") setRenameEntry(null);
            }}
          />
          <div className="flex gap-2 justify-end">
            <button className="bg-[#0078d7] border border-[#0078d7] rounded-none text-white shadow-none hover:bg-[#006cc1] hover:border-[#006cc1] active:bg-[#006cc1] px-4 py-1 text-[13px] cursor-pointer" type="button" onClick={confirmRename}>OK</button>
            <button className="bg-[#2d2d2d] border border-[#3f3f3f] rounded-none text-white shadow-none hover:bg-[#3a3a3a] hover:border-[#545454] active:bg-[#242424] px-4 py-1 text-[13px] cursor-pointer" type="button" onClick={() => setRenameEntry(null)}>Cancel</button>
          </div>
        </div>
      )}
    </div>
  );
}
