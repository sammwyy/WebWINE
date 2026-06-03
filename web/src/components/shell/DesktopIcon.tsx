/**
 * DesktopIcon — a single draggable icon on the desktop.
 *
 * Resolves its own icon image asynchronously. Handles single click (select),
 * double-click (open), drag (reposition), and right-click (context menu).
 */

import { useState, useEffect, useRef, useCallback } from "react";
import { useRuntimeStore } from "../../stores/useRuntimeStore.js";
import { useClipboardStore } from "../../stores/useClipboardStore.js";
import { useDesktopStore } from "../../stores/useDesktopStore.js";
import {
  resolveIcon,
  ICON_PLACEHOLDER,
} from "../../lib/icon-resolver.js";
import { ContextMenu, SEPARATOR } from "./ContextMenu.js";
import type { MenuItem } from "./ContextMenu.js";
import { log } from "../../stores/useLogStore.js";
import { performPaste } from "../../lib/clipboard.js";
import type { DirectoryEntry } from "../../lib/runtime-bridge.js";
import { useThemeStore } from "../../stores/useThemeStore.js";
import { DESKTOP_ICON_LAYOUTS } from "../../stores/useDesktopStore.js";
import { launchGuestPath } from "../../lib/guest-launch.js";
import styles from "./DesktopIcon.module.css";

const ICON_PAD = 12;
const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";

interface DesktopIconProps {
  entry: DirectoryEntry;
  position: IconPosition;
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
  const { theme } = useThemeStore();
  const layout = DESKTOP_ICON_LAYOUTS[iconSize];

  const elRef = useRef<HTMLDivElement>(null);
  const { entry: clipEntry } = useClipboardStore();

  const [imgSrc, setImgSrc] = useState(ICON_PLACEHOLDER);
  const [isShortcut, setIsShortcut] = useState(false);
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
        setIsShortcut(resolved.isShortcut ?? false);
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

  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (suppressRef.current) return;

      selectIcon(path, e.ctrlKey || e.metaKey);

      clicksRef.current++;
      if (clicksRef.current === 1) {
        clickTimerRef.current = setTimeout(() => {
          clicksRef.current = 0;
        }, 400);
      } else if (clicksRef.current >= 2) {
        if (clickTimerRef.current) clearTimeout(clickTimerRef.current);
        clicksRef.current = 0;
        handleOpen();
      }
    },
    [handleOpen],
  );

  // Pointer-based drag.
  const iconRef = useRef<HTMLDivElement>(null);

  const handlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (e.button !== 0) return;
      const el = iconRef.current;
      if (!el || !gridEl) return;

      const startX = e.clientX;
      const startY = e.clientY;
      const startLeft = el.offsetLeft;
      const startTop = el.offsetTop;
      let isDragging = false;

      el.setPointerCapture(e.pointerId);

      const onMove = (ev: PointerEvent) => {
        const dx = ev.clientX - startX;
        const dy = ev.clientY - startY;
        if (!isDragging && Math.hypot(dx, dy) < 5) return;
        isDragging = true;
        setDragging(true);
        el.style.left = `${clampV(startLeft + dx, ICON_PAD, Math.max(ICON_PAD, gridEl.clientWidth - layout.cellWidth))}px`;
        el.style.top = `${clampV(startTop + dy, ICON_PAD, Math.max(ICON_PAD, gridEl.clientHeight - layout.cellHeight))}px`;
      };

      const onUp = (ev: PointerEvent) => {
        el.releasePointerCapture(ev.pointerId);
        el.removeEventListener("pointermove", onMove);
        el.removeEventListener("pointerup", onUp);

        setDragging(false);
        if (!isDragging) return;

        const newPos: IconPosition = {
          col: Math.max(0, Math.round((el.offsetLeft - ICON_PAD) / layout.cellWidth)),
          row: Math.max(0, Math.round((el.offsetTop - ICON_PAD) / layout.cellHeight)),
        };
        setPosition(entry.path, newPos);

        suppressRef.current = true;
        setTimeout(() => {
          suppressRef.current = false;
        }, 80);
      };

      el.addEventListener("pointermove", onMove);
      el.addEventListener("pointerup", onUp);
    },
    [entry.path, gridEl, layout.cellHeight, layout.cellWidth, setPosition],
  );

  const buildContextMenu = useCallback((): MenuItem[] => {
    const isExe = entry.name.toLowerCase().endsWith(".exe");
    const isShortcut = entry.name.toLowerCase().endsWith(".lnk");
    const isFile = entry.kind === "file";
    const items: MenuItem[] = [];

    if ((isExe || isShortcut) && runtime) {
      items.push({
        label: "Open",
        action: () => void launchGuestPath(entry.path, runtime),
      });
    } else {
      items.push({ label: "Open", action: handleOpen });
    }

    if (isExe && runtime) {
      items.push({
        label: "Run as debug",
        action: () =>
          void import("../../apps/process-console/ProcessConsoleApp.js").then((m) =>
            m.openProcessConsole(entry.path, runtime, { debug: true }),
          ),
      });
      items.push({
        label: "Inspect",
        action: () =>
          void import("../../apps/pe-inspector/PeInspectorApp.js").then((m) =>
            m.openPeInspector(entry.path, runtime),
          ),
      });
    }

    items.push(SEPARATOR);
    items.push({
      label: "Copy",
      disabled: !isFile,
      action: () => {
        clipboard.set(entry.path, entry.name, "copy");
      },
    });
    items.push({
      label: "Cut",
      disabled: !isFile,
      action: () => {
        clipboard.set(entry.path, entry.name, "cut");
      },
    });
    items.push({
      label: "Rename",
      action: () => onRename(entry),
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

    items.push(SEPARATOR);
    items.push({
      label: "Properties",
      action: () =>
        import("../../apps/properties/PropertiesApp.js").then((m) =>
          m.openProperties(entry),
        ),
    });

    return items;
  }, [entry, runtime, clipboard, handleOpen, onRefresh, onRename]);

  return (
    <>
      <div
        ref={iconRef}
        className={[
          styles["desktop-icon"],
          "desktop-icon",
          selected ? styles.selected + " selected" : "",
          dragging ? styles.dragging + " dragging" : "",
          clipEntry?.path === path && clipEntry.op === "cut" ? styles.cut + " cut" : "",
        ]
          .filter(Boolean)
          .join(" ")}
        data-path={entry.path}
        style={style}
        onClick={handleClick}
        onPointerDown={handlePointerDown}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          if (!selected) {
            selectIcon(path);
          }
          setCtx({ x: e.clientX, y: e.clientY });
        }}
      >
        <div className={`${styles["desktop-icon-img-wrap"]} desktop-icon-img-wrap`}>
          <img
            src={imgSrc}
            alt=""
            className={`${styles["desktop-icon-img"]} desktop-icon-img`}
            draggable={false}
          />
          {isShortcut && (
            <img
              src={`/themes/${theme}/icons/shell/shortcut.webp`}
              alt="Shortcut"
              className={`${styles["desktop-icon-overlay"]} desktop-icon-overlay`}
              draggable={false}
              onError={(e) => { e.currentTarget.style.display = "none"; }}
            />
          )}
        </div>
        <div className={`${styles["desktop-icon-label"]} desktop-icon-label`}>{entry.name}</div>
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
