/**
 * WindowFrame — the shared window chrome used by every surface on the desktop.
 *
 * Handles the title bar, window controls (minimize / maximize / close),
 * dragging, z-order focus, and resize. Content is passed as children.
 */

import { useRef, useEffect, useCallback } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import type { WindowRecord } from "../../stores/useWindowStore.js";

interface WindowFrameProps {
  record: WindowRecord;
}

export function WindowFrame({ record }: WindowFrameProps) {
  const { closeWindow, focusWindow, minimizeWindow, maximizeWindow, restoreWindow } =
    useWindowStore();

  const elRef = useRef<HTMLDivElement>(null);
  const isDialog = record.variant === "dialog";

  /** Resolve centering transform into concrete left/top, then start dragging. */
  const onTitleBarMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if ((e.target as HTMLElement).closest(".window-controls")) return;
      if (record.maximized) return;
      e.preventDefault();

      const el = elRef.current;
      if (!el) return;

      const rect = el.getBoundingClientRect();
      // Clear transform so subsequent left/top math stays simple.
      el.style.transform = "none";
      el.style.left = `${rect.left}px`;
      el.style.top = `${rect.top}px`;

      const ox = e.clientX - rect.left;
      const oy = e.clientY - rect.top;

      const onMove = (ev: MouseEvent) => {
        el.style.left = `${ev.clientX - ox}px`;
        el.style.top = `${ev.clientY - oy}px`;
      };
      const onUp = () => {
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };
      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [record.maximized],
  );

  /** Double-click on titlebar toggles maximize. */
  const onTitleBarDblClick = useCallback(
    (e: React.MouseEvent) => {
      if (isDialog || (e.target as HTMLElement).closest(".window-controls")) return;
      if (record.maximized) restoreWindow(record.id);
      else maximizeWindow(record.id);
    },
    [isDialog, record.id, record.maximized, maximizeWindow, restoreWindow],
  );

  const onMaxClick = useCallback(() => {
    if (record.maximized) restoreWindow(record.id);
    else maximizeWindow(record.id);
  }, [record.id, record.maximized, maximizeWindow, restoreWindow]);

  const windowClasses = [
    "window",
    `window--${record.variant}`,
    record.resizable && !record.maximized ? "window--resizable" : "",
    record.maximized ? "window--maximized" : "",
    record.minimized ? "window--minimized" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const style: React.CSSProperties = {
    ...record.style,
    display: record.minimized ? "none" : undefined,
  };

  return (
    <div
      ref={elRef}
      id={record.id}
      className={windowClasses}
      style={style}
      onMouseDown={() => focusWindow(record.id)}
    >
      <div
        className="window-titlebar"
        onMouseDown={onTitleBarMouseDown}
        onDoubleClick={onTitleBarDblClick}
      >
        {record.icon && (
          <span className="window-icon" aria-hidden="true">
            {record.icon.includes("/") ? (
              <img src={record.icon} alt="" style={{ width: 16, height: 16, objectFit: "contain" }} draggable={false} onError={(e) => { e.currentTarget.style.display = "none"; }} />
            ) : (
              record.icon
            )}
          </span>
        )}
        <span className="window-title">{record.title}</span>

        <div className="window-controls">
          {!isDialog && (
            <>
              <button
                className="window-control window-minimize"
                type="button"
                title="Minimize"
                aria-label="Minimize"
                onClick={() => minimizeWindow(record.id)}
              />
              <button
                className="window-control window-maximize"
                type="button"
                title={record.maximized ? "Restore" : "Maximize"}
                aria-label={record.maximized ? "Restore" : "Maximize"}
                onClick={onMaxClick}
              />
            </>
          )}
          <button
            className="window-control window-close"
            type="button"
            title="Close"
            aria-label="Close"
            onClick={() => closeWindow(record.id)}
          />
        </div>
      </div>

      <div className="window-body">{record.content}</div>
    </div>
  );
}
