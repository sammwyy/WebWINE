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

interface WindowFrameProps {
  record: WindowRecord;
}

export function WindowFrame({ record }: WindowFrameProps) {
  const { closeWindow, focusWindow, minimizeWindow, maximizeWindow, restoreWindow, activeId } =
    useWindowStore();
  const isActive = activeId === record.id;

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
        useWindowStore.getState().updateStyle(record.id, {
          left: el.style.left,
          top: el.style.top,
        });
      };
      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
    },
    [record.maximized, record.id],
  );

  const onMaxClick = useCallback(() => {
    if (record.maximized) restoreWindow(record.id);
    else maximizeWindow(record.id);
  }, [record.id, record.maximized, maximizeWindow, restoreWindow]);

  const onMinClick = useCallback(() => {
    minimizeWindow(record.id);
  }, [record.id, minimizeWindow]);

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

  const controlBtnClass =
    "relative w-[46px] h-full bg-transparent border-none text-[var(--window-control-text)] cursor-pointer text-[0px] rounded-none leading-none hover:bg-[var(--window-control-hover)] active:bg-[var(--window-control-active)]";

  return (
    <div
      ref={elRef}
      id={record.id}
      className={windowClasses}
      style={style}
      onMouseDownCapture={() => focusWindow(record.id)}
      onContextMenu={(e) => {
        e.stopPropagation();
      }}
    >
      <div
        className="flex items-center gap-[7px] bg-[var(--window-titlebar-bg)] pl-[10px] pr-[6px] h-[30px] cursor-move select-none flex-shrink-0 border-b border-[var(--window-titlebar-border)]"
        onPointerDown={onTitlePointerDown}
      >
        {record.icon && (
          <span className="text-[14px] leading-none flex-shrink-0" aria-hidden="true">
            {record.icon.includes("/") ? (
              <img src={record.icon} alt="" style={{ width: 16, height: 16, objectFit: "contain" }} draggable={false} onError={(e) => { e.currentTarget.style.display = "none"; }} />
            ) : (
              record.icon
            )}
          </span>
        )}
        <span className="flex-1 text-[12px] font-semibold text-[var(--window-titlebar-text)] overflow-hidden whitespace-nowrap text-ellipsis">{record.title}</span>

        <div className="flex self-stretch ml-1 -mr-1.5 window-controls">
          {record.variant !== "dialog" && (
            <>
              <button
                type="button"
                className={`${controlBtnClass} after:content-[''] after:absolute after:left-1/2 after:top-[58%] after:w-3 after:h-px after:bg-current after:-translate-x-1/2 after:-translate-y-1/2`}
                onClick={onMinClick}
                aria-label="Minimize"
              />
              <button
                type="button"
                className={`${controlBtnClass} ${record.maximized ? "after:content-[''] after:absolute after:left-[calc(50%+2px)] after:top-[calc(50%+2px)] after:w-2.5 after:h-2.5 after:border after:border-current after:bg-[var(--window-frame-bg)] after:-translate-x-1/2 after:-translate-y-1/2 before:content-[''] before:absolute before:left-[calc(50%-2px)] before:top-[calc(50%-2px)] before:w-2.5 before:h-2.5 before:border before:border-current before:bg-transparent before:-translate-x-1/2 before:-translate-y-1/2" : "after:content-[''] after:absolute after:left-1/2 after:top-1/2 after:w-2.5 after:h-2.5 after:border after:border-current after:-translate-x-1/2 after:-translate-y-1/2"}`}
                onClick={onMaxClick}
                aria-label={record.maximized ? "Restore" : "Maximize"}
                disabled={!record.resizable}
              />
            </>
          )}
          <button
            type="button"
            className={`${controlBtnClass} hover:!bg-[var(--window-close-hover)] hover:!text-[var(--window-close-hover-text)] before:content-[''] before:absolute before:left-1/2 before:top-1/2 before:w-3 before:h-px before:bg-current before:-translate-x-1/2 before:-translate-y-1/2 before:rotate-45 after:content-[''] after:absolute after:left-1/2 after:top-1/2 after:w-3 after:h-px after:bg-current after:-translate-x-1/2 after:-translate-y-1/2 after:-rotate-45`}
            onClick={() => closeWindow(record.id)}
            aria-label="Close"
          />
        </div>
      </div>

      <div className="flex-1 overflow-auto p-0 flex flex-col bg-[var(--window-content-bg)] text-[var(--window-content-text)] [--text:var(--window-content-text)] [--text-muted:var(--text-muted)] [--window-border:#2b2b2b] [--window-bg:var(--window-content-bg)]">
        {record.content}
      </div>
    </div>
  );
}
