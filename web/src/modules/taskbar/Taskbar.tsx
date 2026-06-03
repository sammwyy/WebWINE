/**
 * Taskbar — the bottom bar with start button, window list, tray, and clock.
 */

import { useState, useEffect } from "react";
import { useWindowStore } from "../../state/windowStore";

import { StartMenu } from "./StartMenu";
import { TrayMenu } from "./TrayMenu";
import { Clock } from "./Clock";
import { SHELL_ACTION_EVENT } from "../../shared/lib/guest-launch";
import type { ShellActionDetail } from "../../shared/lib/shortcut-target";

interface TaskbarProps {
  /** Hidden file inputs managed by the parent (Desktop) for uploads. */
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  folderInputRef: React.RefObject<HTMLInputElement | null>;
}

export function Taskbar({ fileInputRef, folderInputRef }: TaskbarProps) {
  const [startOpen, setStartOpen] = useState(false);
  const [trayOpen, setTrayOpen] = useState(false);
  const { windows, activeId, activateFromTaskbar } = useWindowStore();


  useEffect(() => {
    const onShellAction = (e: Event) => {
      const detail = (e as CustomEvent<ShellActionDetail>).detail;
      if (!detail) return;
      if (detail.action === "upload-file") {
        fileInputRef.current?.click();
      } else if (detail.action === "upload-folder") {
        folderInputRef.current?.click();
      }
    };

    window.addEventListener(SHELL_ACTION_EVENT, onShellAction);
    return () => window.removeEventListener(SHELL_ACTION_EVENT, onShellAction);
  }, [fileInputRef, folderInputRef]);

  return (
    <div
      id="taskbar"
      className="fixed bottom-0 left-0 right-0 h-[var(--taskbar-height)] bg-[var(--taskbar-bg)] border-t border-[var(--taskbar-border)] flex items-center gap-0 z-[9000] text-[var(--taskbar-text)] shadow-[0_-1px_0_rgba(0,0,0,0.55)]" style={{
        backdropFilter: "blur(8px)",
        WebkitBackdropFilter: "blur(8px)",
      }}
    >
      <button
        id="start-button"
        type="button"
        className="h-full w-14 min-w-14 inline-flex justify-center items-center bg-transparent text-white border-0 rounded-none cursor-pointer flex-none hover:bg-[rgba(255,255,255,0.095)] active:bg-[rgba(255,255,255,0.16)] data-[expanded=true]:bg-[rgba(255,255,255,0.16)]"
        aria-haspopup="true"
        aria-expanded={startOpen}
        data-expanded={startOpen}
        onClick={(e) => {
          e.stopPropagation();
          setTrayOpen(false);
          setStartOpen((v) => !v);
        }}
      >
        <svg viewBox="0 0 88 88" width="17" height="17" aria-hidden="true">
          <path
            fill="currentColor"
            d="M0,12.4L35.7,7.4v35.3H0V12.4z M39.6,6.9L88,0v42.7H39.6V6.9z M39.6,46.9H88V88l-48.4-6.9V46.9z M0,46.9h35.7v34L0,75.9V46.9z"
          />
        </svg>
      </button>

      {startOpen && (
        <StartMenu
          onClose={() => setStartOpen(false)}
        />
      )}

      <div
        id="taskbar-window-list"
        className="flex items-center gap-0 flex-1 min-w-0 overflow-x-auto overflow-y-hidden [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
        aria-label="Open windows"
        role="toolbar"
      >
        {windows.map((win) => (
          <button
            key={win.id}
            className={`flex items-center gap-1.5 min-w-0 h-10 min-h-10 px-3 max-w-[190px] flex-[0_1_160px] bg-transparent border-0 rounded-none text-white text-[13px] cursor-pointer overflow-hidden whitespace-nowrap hover:bg-[rgba(255,255,255,0.095)] active:bg-[rgba(255,255,255,0.16)] ${win.id === activeId ? "bg-[rgba(255,255,255,0.16)] relative after:content-[''] after:absolute after:left-2.5 after:right-2.5 after:bottom-0 after:h-0.5 after:bg-[#76b9ed]" : ""} ${win.minimized ? "opacity-75" : ""} flex-none w-12 min-w-[48px] max-w-[48px] p-0 justify-center`}
            type="button"
            title={win.title}
            onClick={() => activateFromTaskbar(win.id)}
          >
            {win.icon && (
              <span className="flex-none text-[14px] leading-none" aria-hidden="true">
                {win.icon.includes("/") ? (
                  <img src={win.icon} alt="" style={{ width: 16, height: 16, objectFit: "contain" }} draggable={false} onError={(e) => { e.currentTarget.style.display = "none"; }} />
                ) : (
                  win.icon
                )}
              </span>
            )}
          </button>
        ))}
      </div>

      <div id="taskbar-tray" className="relative flex items-center gap-1 flex-none ml-auto px-1 text-white text-[12px]">
        <button
          id="tray-toggle"
          className="w-10 h-10 bg-transparent text-white border-0 rounded-none cursor-pointer text-[13px] leading-none hover:bg-[rgba(255,255,255,0.095)] active:bg-[rgba(255,255,255,0.16)] data-[expanded=true]:bg-[rgba(255,255,255,0.16)] flex items-center justify-center"
          type="button"
          aria-label="Show hidden icons"
          aria-haspopup="true"
          aria-expanded={trayOpen}
          data-expanded={trayOpen}
          onClick={(e) => {
            e.stopPropagation();
            setStartOpen(false);
            setTrayOpen((v) => !v);
          }}
        >
          ^
        </button>

        {trayOpen && <TrayMenu onClose={() => setTrayOpen(false)} />}

        <Clock />
      </div>
    </div>
  );
}
