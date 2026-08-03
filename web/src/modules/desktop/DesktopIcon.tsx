/**
 * DesktopIcon — a single draggable icon on the desktop.
 *
 * Resolves its own icon image asynchronously. Handles single click (select),
 * double-click (open), drag (reposition), and right-click (context menu).
 */

import { useState, useEffect, useRef, useCallback } from "react";

import { useRuntimeStore } from "@/state/runtimeStore";
import { useClipboardStore } from "@/state/clipboardStore";
import { useDesktopStore } from "@/state/desktopStore";
import { openProcessConsole } from "../../apps/process-console/ProcessConsoleApp";
import { openPeInspector } from "../../apps/pe-inspector/PeInspectorApp";
import { openProperties } from "../../apps/properties/PropertiesApp";
import {
  resolveIcon,
  ICON_PLACEHOLDER,
} from "@/shared/lib/icons/icon-resolver";
import { ContextMenu, SEPARATOR } from "./ContextMenu";
import type { MenuItem } from "./ContextMenu";
import { log } from "@/state/logStore";
import type { DirectoryEntry } from "@/core/bridge/runtime-bridge";
import { DESKTOP_ICON_LAYOUTS } from "@/state/desktopStore";
import { launchGuestPath } from "@/shared/lib/guest-launch";
import {
  copyPayloadToDir,
  createShortcut,
  decodeDragPayload,
  encodeDragPayload,
  toPayloadEntry,
} from "@/shared/lib/clipboard";

const ICON_PAD = 12;

/**
 * Firefox does not reliably expose custom dataTransfer MIME types during
 * dragover, so the desktop uses this counter to tell "icon rearrange" from
 * "external file upload".
 */
let desktopIconDragDepth = 0;

export function isDesktopIconDragActive(): boolean {
  return desktopIconDragDepth > 0;
}

interface DesktopIconProps {
  entry: DirectoryEntry;
  position: { col: number; row: number };
  gridEl: HTMLElement | null;
  onRefresh: () => void;
  onRename: (entry: DirectoryEntry) => void;
}

export function DesktopIcon({
  entry,
  position,
  gridEl,
  onRefresh,
  onRename,
}: DesktopIconProps) {
  const { runtime } = useRuntimeStore();
  const clipboard = useClipboardStore();
  const { setPosition, selectedIds, selectIcon, iconSize } = useDesktopStore();

  const layout = DESKTOP_ICON_LAYOUTS[iconSize];

  const elRef = useRef<HTMLDivElement>(null);
  const { entry: clipEntry } = useClipboardStore();

  const [imgSrc, setImgSrc] = useState(ICON_PLACEHOLDER);
  const [overlaySrc, setOverlaySrc] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [ctx, setCtx] = useState<{ x: number; y: number } | null>(null);

  const selected = selectedIds.includes(entry.path);

  const path = entry.path;

  // Resolve icon image asynchronously.
  useEffect(() => {
    if (!runtime) return;
    resolveIcon(entry, runtime)
      .then((resolved) => {
        setImgSrc(resolved.src);
        setOverlaySrc(resolved.overlay ?? null);
      })
      .catch(() => { /* keep placeholder */ });
  }, [entry, runtime]);

  // Compute absolute position from grid slot.
  const left = ICON_PAD + position.col * layout.cellWidth;
  const top = ICON_PAD + position.row * layout.cellHeight;
  const gridW = gridEl?.clientWidth ?? Infinity;
  const gridH = gridEl?.clientHeight ?? Infinity;
  const maxLeft = Math.max(ICON_PAD, gridW - layout.cellWidth);
  const maxTop = Math.max(ICON_PAD, gridH - layout.cellHeight);

  const style: React.CSSProperties = {
    left: Math.min(left, maxLeft),
    top: Math.min(top, maxTop),
  };

  const clampV = (v: number, lo: number, hi: number) =>
    Math.min(Math.max(v, lo), Math.max(lo, hi));

  // Double-click / single-click tracking.
  const clicksRef = useRef(0);
  const clickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const suppressRef = useRef(false);

  const handleOpen = useCallback(() => {
    if (!runtime) return;
    void launchGuestPath(entry.path, runtime);
  }, [entry, runtime]);

  const selectedEntries = useCallback(() => {
    const state = useDesktopStore.getState();
    const paths = state.selectedIds.includes(entry.path) ? state.selectedIds : [entry.path];
    const selected = state.entries.filter((item) => paths.includes(item.path));
    return selected.length > 0 ? selected : [entry];
  }, [entry]);

  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (suppressRef.current) return;

      selectIcon(path, e.ctrlKey || e.metaKey);

      clicksRef.current++;
      if (clicksRef.current === 1) {
        clickTimerRef.current = setTimeout(() => {
          clicksRef.current = 0;
        }, 300);
      } else if (clicksRef.current >= 2) {
        if (clickTimerRef.current) clearTimeout(clickTimerRef.current);
        clicksRef.current = 0;
        handleOpen();
      }
    },
    [handleOpen, path, selectIcon],
  );


  const buildContextMenu = useCallback((): MenuItem[] => {
    const isExe = entry.name.toLowerCase().endsWith(".exe");
    const isFile = entry.kind === "file";
    const isDirectory = entry.kind === "directory";

    const items: MenuItem[] = [
      {
        label: "Open",
        action: handleOpen,
      },
    ];

    if (isExe && runtime) {
      items.push({
        label: "Run with mode...",
        children: [
          {
            label: "Keep terminal open",
            action: () => {
              void openProcessConsole(entry.path, runtime);
            },
          },
          {
            label: "Debug mode",
            action: () => {
              void openProcessConsole(entry.path, runtime, { debug: true });
            },
          },
        ],
      });

      items.push({
        label: "Inspect",
        action: () => {
          void openPeInspector(entry.path, runtime);
        },
      });
    }

    items.push(SEPARATOR);

    items.push({
      label: "Cut",
      disabled: !isFile && !isDirectory,
      action: () => {
        clipboard.setMany(selectedEntries().map(toPayloadEntry), "cut");
      },
    });

    items.push({
      label: "Copy",
      disabled: !isFile && !isDirectory,
      action: () => {
        clipboard.setMany(selectedEntries().map(toPayloadEntry), "copy");
      },
    });

    items.push({
      label: "Create shortcut",
      disabled: !runtime,
      action: async () => {
        if (!runtime) return;
        try {
          for (const selectedEntry of selectedEntries()) {
            await createShortcut(
              selectedEntry.path,
              selectedEntry.name,
              parentPath(selectedEntry.path),
              runtime,
            );
          }
          onRefresh();
        } catch (err) {
          log("fs", `create shortcut failed: ${err}`, "error");
        }
      },
    });

    items.push({
      label: "Delete",
      danger: true,
      action: async () => {
        if (!runtime) return;

        try {
          await runtime.deleteNode(entry.path);
          onRefresh();
        } catch (err) {
          log("fs", `delete failed: ${err}`, "error");
        }
      },
    });

    items.push({
      label: "Rename",
      action: () => onRename(entry),
    });

    items.push(SEPARATOR);

    items.push({
      label: "Properties",
      action: () => {
        void openProperties(entry, runtime!);
      },
    });

    return items;
  }, [entry, runtime, clipboard, handleOpen, onRefresh, onRename, selectedEntries]);
  return (
    <>
      <div

        className={[
          "absolute",
          "w-[var(--desktop-icon-cell-w,82px)] h-[var(--desktop-icon-cell-h,92px)]",
          "flex flex-col items-center justify-start",
          "pt-[6px] px-[3px] pb-[4px] gap-[4px]",
          "box-border rounded-none border border-transparent",
          "cursor-default select-none text-center overflow-hidden",
          "text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.9)]",
          "transition-colors duration-75",
          "hover:bg-[rgba(255,255,255,0.105)] hover:border-[rgba(255,255,255,0.18)]",
          selected
            ? "bg-[rgba(0,120,215,0.34)] border-[rgba(0,120,215,0.95)] hover:bg-[rgba(0,120,215,0.42)]"
            : "",
          dragging ? "opacity-80 z-[5] !transition-none" : "",
          clipEntry?.path === path && clipEntry.op === "cut" ? "opacity-45" : "",
        ].join(" ")}
        data-path={entry.path}
        style={style}
        draggable
        onClick={handleClick}
        onDragStart={(e) => {
          desktopIconDragDepth += 1;
          let payload = selectedEntries().map(toPayloadEntry);
          if (!payload.find((p) => p.path === entry.path)) {
            selectIcon(path);
            payload = [toPayloadEntry(entry)];
          }
          const encoded = encodeDragPayload(payload);
          // Custom types + text/plain fallback (Firefox dragover often hides customs).
          e.dataTransfer.setData("application/x-webwine-paths", encoded);
          e.dataTransfer.setData("text/plain", `webwine-desktop:${encoded}`);
          e.dataTransfer.setData("application/x-webwine-desktop-drag", "1");

          const rect = e.currentTarget.getBoundingClientRect();
          const offsetX = e.clientX - rect.left;
          const offsetY = e.clientY - rect.top;

          const offset = {
            x: offsetX,
            y: offsetY,
            path: entry.path,
          };
          e.dataTransfer.setData(
            "application/x-webwine-drag-offset",
            JSON.stringify(offset),
          );
          e.dataTransfer.effectAllowed = "copyMove";

          // Transparent drag image so we can move the real icon under the cursor.
          try {
            const img = new Image();
            img.src =
              "data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=";
            e.dataTransfer.setDragImage(img, 0, 0);
          } catch {
            // Firefox can throw if the image isn't ready; default ghost is fine.
          }

          setDragging(true);
          e.currentTarget.dataset.dragStartX = offsetX.toString();
          e.currentTarget.dataset.dragStartY = offsetY.toString();
        }}
        onDrag={(e) => {
          // Firefox often ends a drag with (0,0) — ignore that sample.
          if (e.clientX === 0 && e.clientY === 0) return;
          const el = e.currentTarget;
          if (!gridEl) return;

          const offsetX = parseFloat(el.dataset.dragStartX || "0");
          const offsetY = parseFloat(el.dataset.dragStartY || "0");
          const gridRect = gridEl.getBoundingClientRect();

          const newLeft = e.clientX - gridRect.left - offsetX;
          const newTop = e.clientY - gridRect.top - offsetY;

          el.style.left = `${clampV(newLeft, ICON_PAD, Math.max(ICON_PAD, gridEl.clientWidth - layout.cellWidth))}px`;
          el.style.top = `${clampV(newTop, ICON_PAD, Math.max(ICON_PAD, gridEl.clientHeight - layout.cellHeight))}px`;
        }}
        onDragEnd={(e) => {
          desktopIconDragDepth = Math.max(0, desktopIconDragDepth - 1);
          const el = e.currentTarget;
          // Drop may have already committed a new cell via the store — prefer that
          // so we don't snap back to the pre-drag cell for one frame.
          const latest =
            useDesktopStore.getState().positions[entry.path] ?? position;
          const restoredLeft = Math.min(
            ICON_PAD + latest.col * layout.cellWidth,
            maxLeft,
          );
          const restoredTop = Math.min(
            ICON_PAD + latest.row * layout.cellHeight,
            maxTop,
          );
          el.style.left = `${restoredLeft}px`;
          el.style.top = `${restoredTop}px`;
          setDragging(false);
        }}
        onDragOver={(e) => {
          if (entry.kind !== "directory") return;
          e.preventDefault();
          e.dataTransfer.dropEffect = e.ctrlKey ? "copy" : "move";
        }}
        onDrop={async (e) => {
          if (entry.kind !== "directory" || !runtime) return;
          e.preventDefault();
          e.stopPropagation();
          const payload = decodeDragPayload(e.dataTransfer.getData("application/x-webwine-paths"));
          if (payload.length === 0) return;
          try {
            await copyPayloadToDir(payload, entry.path, runtime, !e.ctrlKey);
            onRefresh();
          } catch (err) {
            log("fs", `drop failed: ${err}`, "error");
          }
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          if (!selected) {
            selectIcon(path);
          }
          setCtx({ x: e.clientX, y: e.clientY });
        }}
      >
        <div className="relative w-[var(--icon-size)] h-[var(--icon-size)] flex-none drop-shadow-[0_1px_1px_rgba(0,0,0,0.45)]">
          <img
            src={imgSrc || undefined}
            alt=""
            className="w-full h-full object-contain"
            draggable={false}
          />
          {overlaySrc && (
            <img
              src={overlaySrc || undefined}
              alt="Shortcut overlay"
              className="absolute bottom-0 left-0 w-1/2 h-1/2 object-contain"
              draggable={false}
              onError={(e) => { e.currentTarget.style.display = "none"; }}
            />
          )}
        </div>
        <div className="max-w-[74px] text-center break-words text-white [text-shadow:0_1px_3px_rgba(0,0,0,0.95)] text-[12px] leading-[15px] line-clamp-2">
          {displayName(entry.name)}
        </div>
      </div>

      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          items={buildContextMenu()}
          onClose={() => setCtx(null)}
        />
      )}
    </>
  );
}

function displayName(name: string): string {
  return name.toLowerCase().endsWith(".lnk") ? name.slice(0, -4) : name;
}

function parentPath(path: string): string {
  const idx = path.lastIndexOf("\\");
  if (idx <= 2) return `${path[0].toUpperCase()}:\\`;
  return path.slice(0, idx);
}
