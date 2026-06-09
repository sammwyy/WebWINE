import React, { useCallback } from "react";
import { clamp } from "@/shared/lib/utils";
import { useWindowStore } from "@/state/windowStore";

interface WindowTitlebarProps {
  windowId: string;
  className?: string;
  children?: React.ReactNode;
}

export function WindowTitlebar({
  windowId,
  className,
  children,
}: WindowTitlebarProps) {
  const {
    windows,
    closeWindow,
    minimizeWindow,
    maximizeWindow,
    restoreWindow,
  } = useWindowStore();
  const record = windows.find((w) => w.id === windowId);

  const onTitlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!record) return;
      if ((e.target as HTMLElement).closest(".window-controls")) return;
      if (record.maximized) return;

      const el = document.getElementById(record.id);
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
    [record],
  );

  const onMaxClick = useCallback(() => {
    if (!record) return;
    if (record.maximized) restoreWindow(record.id);
    else maximizeWindow(record.id);
  }, [record, maximizeWindow, restoreWindow]);

  const onMinClick = useCallback(() => {
    if (!record) return;
    minimizeWindow(record.id);
  }, [record, minimizeWindow]);

  if (!record) return null;

  const controlBtnClass =
    "relative w-[46px] h-full bg-transparent border-none text-[var(--window-control-text)] cursor-pointer text-[0px] rounded-none leading-none hover:bg-[var(--window-control-hover)] active:bg-[var(--window-control-active)]";

  return (
    <div
      className={`flex items-center gap-[7px] bg-[var(--window-titlebar-bg)] pl-[10px] pr-[6px] h-[30px] cursor-move select-none flex-shrink-0 border-b border-[var(--window-titlebar-border)] ${className || ""}`}
      onPointerDown={onTitlePointerDown}
    >
      {children || (
        <>
          {record.icon && (
            <span
              className="text-[14px] leading-none flex-shrink-0"
              aria-hidden="true"
            >
              {record.icon.includes("/") ? (
                <img
                  src={record.icon}
                  alt=""
                  style={{ width: 16, height: 16, objectFit: "contain" }}
                  draggable={false}
                  onError={(e) => {
                    e.currentTarget.style.display = "none";
                  }}
                />
              ) : (
                record.icon
              )}
            </span>
          )}
          <span className="flex-1 text-[12px] font-semibold text-[var(--window-titlebar-text)] overflow-hidden whitespace-nowrap text-ellipsis">
            {record.title}
          </span>
        </>
      )}

      <div className="flex self-stretch ml-auto -mr-1.5 window-controls">
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
              className={`${controlBtnClass} ${
                record.maximized
                  ? "after:content-[''] after:absolute after:left-[calc(50%+2px)] after:top-[calc(50%+2px)] after:w-2.5 after:h-2.5 after:border after:border-current after:bg-[var(--window-frame-bg)] after:-translate-x-1/2 after:-translate-y-1/2 before:content-[''] before:absolute before:left-[calc(50%-2px)] before:top-[calc(50%-2px)] before:w-2.5 before:h-2.5 before:border before:border-current before:bg-transparent before:-translate-x-1/2 before:-translate-y-1/2"
                  : "after:content-[''] after:absolute after:left-1/2 after:top-1/2 after:w-2.5 after:h-2.5 after:border after:border-current after:-translate-x-1/2 after:-translate-y-1/2"
              }`}
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
  );
}
