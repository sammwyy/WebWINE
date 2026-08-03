/**
 * WindowFrame — the shared window chrome used by every surface on the desktop.
 *
 * Handles the title bar, window controls (minimize / maximize / close),
 * dragging, z-order focus, and resize. Content is passed as children.
 */

import { useRef, useCallback } from "react";

import { clamp } from "@/shared/lib/utils";
import { useWindowStore } from "@/state/windowStore";
import type { WindowRecord } from "@/state/windowStore";
import { WindowTitlebar } from "./WindowTitlebar";

interface WindowFrameProps {
  record: WindowRecord;
}

export function WindowFrame({ record }: WindowFrameProps) {
  const { closeWindow, focusWindow, activeId } = useWindowStore();
  const isActive = activeId === record.id;

  let variantClass = "min-w-[320px] min-h-[180px]";
  if (record.variant === "dialog") variantClass = "min-w-[280px]";

  let activeClass = isActive
    ? "shadow-[0_0_0_1px_var(--window-focus-border),0_12px_32px_rgba(0,0,0,0.50)]"
    : "";

  if (isActive && record.maximized) {
    activeClass = "shadow-[0_0_0_1px_var(--window-focus-border,#7ecbff)]";
  }

  const windowClasses = [
    "absolute bg-[var(--window-frame-bg)] border border-[var(--window-frame-border)] rounded-[var(--window-frame-radius)] flex flex-col shadow-[0_12px_40px_rgba(0,0,0,0.55)] pointer-events-auto overflow-hidden",
    variantClass,
    record.resizable && !record.maximized ? "resize" : "",
    record.maximized ? "!resize-none !rounded-none" : "",
    activeClass,
  ]
    .filter(Boolean)
    .join(" ");

  const style: React.CSSProperties = {
    ...record.style,
    display: record.minimized ? "none" : undefined,
  };

  return (
    <div
      id={record.id}
      className={windowClasses}
      style={style}
      onMouseDownCapture={() => focusWindow(record.id)}
      onContextMenu={(e) => {
        e.stopPropagation();
      }}
    >
      {!record.hideTitlebar && <WindowTitlebar windowId={record.id} />}

      <div className="flex-1 overflow-auto p-0 flex flex-col bg-[var(--window-content-bg)] text-[var(--window-content-text)] [--text:var(--window-content-text)] [--text-muted:var(--text-muted)] [--window-border:#2b2b2b] [--window-bg:var(--window-content-bg)]">
        {record.content}
      </div>
    </div>
  );
}
