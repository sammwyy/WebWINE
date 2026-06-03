/**
 * WindowFrame — the shared window chrome used by every surface on the desktop.
 *
 * Handles the title bar, window controls (minimize / maximize / close),
 * dragging, z-order focus, and resize. Content is passed as children.
 */

import { useRef, useCallback } from "react";
import { clamp } from "../../lib/utils.js";
import { useWindowStore } from "../../stores/useWindowStore.js";
import { useThemeStore } from "../../stores/useThemeStore.js";
import styles from "./WindowFrame.module.css";
import type { WindowRecord } from "../../stores/useWindowStore.js";

interface WindowFrameProps {
  record: WindowRecord;
}

export function WindowFrame({ record }: WindowFrameProps) {
  const { closeWindow, focusWindow, minimizeWindow, maximizeWindow, restoreWindow, activeWindowId } =
    useWindowStore();
  const isActive = activeWindowId === record.id;

  const elRef = useRef<HTMLDivElement>(null);

  /** Resolve centering transform into concrete left/top, then start dragging. */
  const onTitlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if ((e.target as HTMLElement).closest(".window-controls")) return;
      if (record.maximized) return;

      const el = elRef.current;
      if (!el) return;

      const rect = el.getBoundingClientRect();
      el.style.left = `${rect.left}px`;
      el.style.top = `${rect.top}px`;
      el.style.transform = "none";

      const ox = e.clientX - rect.left;
      const oy = e.clientY - rect.top;

      const onMove = (ev: PointerEvent) => {
        const x = clamp(ev.clientX - ox, 0, window.innerWidth - rect.width);
        const y = clamp(ev.clientY - oy, 0, window.innerHeight - 30);
        el.style.left = `${x}px`;
        el.style.top = `${y}px`;
      };
      const onUp = () => {
        document.removeEventListener("pointermove", onMove);
        document.removeEventListener("pointerup", onUp);
      };
      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
    },
    [record.maximized],
  );

  const onMaxClick = useCallback(() => {
    if (record.maximized) restoreWindow(record.id);
    else maximizeWindow(record.id);
  }, [record.id, record.maximized, maximizeWindow, restoreWindow]);

  const onMinClick = useCallback(() => {
    minimizeWindow(record.id);
  }, [record.id, minimizeWindow]);

  const windowClasses = [
    styles.window,
    "window",
    styles[`window--${record.variant}`],
    `window--${record.variant}`,
    record.resizable && !record.maximized ? `${styles["window--resizable"]} window--resizable` : "",
    record.maximized ? `${styles["window--maximized"]} window--maximized` : "",
    record.minimized ? `${styles["window--minimized"]} window--minimized` : "",
    isActive ? styles["window--active"] + " window--active" : "",
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
        className={`${styles["window-titlebar"]} window-titlebar`}
        onPointerDown={onTitlePointerDown}
      >
        {record.icon && (
          <span className={`${styles["window-icon"]} window-icon`} aria-hidden="true">
            {record.icon.includes("/") ? (
              <img src={record.icon} alt="" style={{ width: 16, height: 16, objectFit: "contain" }} draggable={false} onError={(e) => { e.currentTarget.style.display = "none"; }} />
            ) : (
              record.icon
            )}
          </span>
        )}
        <span className={`${styles["window-title"]} window-title`}>{record.title}</span>

        <div className={`${styles["window-controls"]} window-controls`}>
          {record.variant === "default" && (
            <>
              <button
                type="button"
                className={`${styles["window-control"]} ${styles["window-minimize"]} window-control window-minimize`}
                onClick={onMinClick}
                aria-label="Minimize"
                disabled={!record.minimizable}
              />
              <button
                type="button"
                className={`${styles["window-control"]} ${styles["window-maximize"]} window-control window-maximize`}
                onClick={onMaxClick}
                aria-label={record.maximized ? "Restore" : "Maximize"}
                disabled={!record.resizable}
              />
            </>
          )}
          <button
            type="button"
            className={`${styles["window-control"]} ${styles["window-close"]} window-control window-close`}
            onClick={() => closeWindow(record.id)}
            aria-label="Close"
          />
        </div>
      </div>

      <div className={`${styles["window-body"]} window-body`}>{record.content}</div>
    </div>
  );
}
