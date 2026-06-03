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
import type { DirectoryEntry } from "../../lib/worker.js";
import type { IconPosition } from "../../stores/useDesktopStore.js";

const ICON_CELL_W = 88;
const ICON_CELL_H = 104;
const ICON_PAD = 14;
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
  const { setPosition } = useDesktopStore();

  const [iconSrc, setIconSrc] = useState(ICON_PLACEHOLDER);
  const [overlaySrc, setOverlaySrc] = useState<string | undefined>(undefined);
  const [selected, setSelected] = useState(false);
  const [cut, setCut] = useState(false);
  const [ctx, setCtx] = useState<{ x: number; y: number } | null>(null);

  // Resolve icon image asynchronously.
  useEffect(() => {
    if (!runtime) return;
    resolveIcon(entry, runtime)
      .then((resolved) => {
        setIconSrc(resolved.src);
        setOverlaySrc(resolved.overlay);
      })
      .catch(() => { /* keep placeholder */ });
  }, [entry, runtime]);

  // Compute absolute position from grid slot.
  const left = ICON_PAD + position.col * ICON_CELL_W;
  const top = ICON_PAD + position.row * ICON_CELL_H;
  const gridW = gridEl?.clientWidth ?? Infinity;
  const gridH = gridEl?.clientHeight ?? Infinity;

  const style: React.CSSProperties = {
    left: Math.min(left, gridW - ICON_CELL_W),
    top: Math.min(top, gridH - ICON_CELL_H),
  };

  const clampV = (v: number, lo: number, hi: number) =>
    Math.min(Math.max(v, lo), Math.max(lo, hi));

  // Double-click / single-click tracking.
  const clicksRef = useRef(0);
  const clickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const suppressRef = useRef(false);

  const handleOpen = useCallback(() => {
    if (!runtime) return;
    const name = entry.name.toLowerCase();
    if (entry.kind === "directory") {
      import("../../apps/explorer/ExplorerApp.js").then((m) =>
        m.openExplorer(entry.path, runtime),
      );
    } else if (name.endsWith(".exe")) {
      import("../../apps/process-console/ProcessConsoleApp.js").then((m) =>
        m.openProcessConsole(entry.path, runtime),
      );
    } else {
      import("../../apps/text-reader/TextReaderApp.js").then((m) =>
        m.openTextReader(entry.path, runtime),
      );
    }
  }, [entry, runtime]);

  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (suppressRef.current) return;

      document
        .querySelectorAll(".desktop-icon.selected")
        .forEach((el) => el !== e.currentTarget && el.classList.remove("selected"));
      setSelected(true);

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
      let dragging = false;

      el.setPointerCapture(e.pointerId);

      const onMove = (ev: PointerEvent) => {
        const dx = ev.clientX - startX;
        const dy = ev.clientY - startY;
        if (!dragging && Math.hypot(dx, dy) < 5) return;
        dragging = true;
        el.classList.add("dragging");
        el.style.left = `${clampV(startLeft + dx, ICON_PAD, gridEl.clientWidth - ICON_CELL_W)}px`;
        el.style.top = `${clampV(startTop + dy, ICON_PAD, gridEl.clientHeight - ICON_CELL_H)}px`;
      };

      const onUp = (ev: PointerEvent) => {
        el.releasePointerCapture(ev.pointerId);
        el.removeEventListener("pointermove", onMove);
        el.removeEventListener("pointerup", onUp);

        if (!dragging) return;
        el.classList.remove("dragging");

        const newPos: IconPosition = {
          col: Math.max(0, Math.round((el.offsetLeft - ICON_PAD) / ICON_CELL_W)),
          row: Math.max(0, Math.round((el.offsetTop - ICON_PAD) / ICON_CELL_H)),
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
    [entry.path, gridEl, setPosition],
  );

  const buildContextMenu = useCallback((): MenuItem[] => {
    const isExe = entry.name.toLowerCase().endsWith(".exe");
    const isFile = entry.kind === "file";
    const items: MenuItem[] = [];

    if (isExe && runtime) {
      items.push({
        label: "Run",
        action: () =>
          import("../../apps/process-console/ProcessConsoleApp.js").then((m) =>
            m.openProcessConsole(entry.path, runtime),
          ),
      });
      items.push({
        label: "Run as debug",
        action: () =>
          import("../../apps/process-console/ProcessConsoleApp.js").then((m) =>
            m.openProcessConsole(entry.path, runtime, { debug: true }),
          ),
      });
      items.push({
        label: "Inspect",
        action: () =>
          import("../../apps/pe-inspector/PeInspectorApp.js").then((m) =>
            m.openPeInspector(entry.path, runtime),
          ),
      });
    } else {
      items.push({ label: "Open", action: handleOpen });
    }

    items.push(SEPARATOR);
    items.push({
      label: "Copy",
      disabled: !isFile,
      action: () => {
        clipboard.set(entry.path, entry.name, "copy");
        document
          .querySelectorAll(".desktop-icon")
          .forEach((el) => (el as HTMLElement).classList.remove("cut"));
      },
    });
    items.push({
      label: "Cut",
      disabled: !isFile,
      action: () => {
        clipboard.set(entry.path, entry.name, "cut");
        setCut(true);
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
          "desktop-icon",
          selected ? "selected" : "",
          cut ? "cut" : "",
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
          setSelected(true);
          setCtx({ x: e.clientX, y: e.clientY });
        }}
      >
        <div className="desktop-icon-img-wrap">
          <img
            className="desktop-icon-img"
            src={iconSrc}
            alt=""
            draggable={false}
          />
          {overlaySrc && (
            <img
              className="desktop-icon-overlay"
              src={overlaySrc}
              alt=""
              draggable={false}
            />
          )}
        </div>
        <div className="desktop-icon-label">{entry.name}</div>
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
