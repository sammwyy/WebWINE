/**
 * Taskbar — the bottom bar with start button, window list, tray, and clock.
 * Everything is painted by the active theme (`ww-taskbar*` classes); themes
 * can also swap the start button content via `components.StartButton`.
 */

import { useState, useEffect } from "react";
import { useWindowStore } from "@/state/windowStore";
import { useSettingsStore } from "@/state/settingsStore";
import { useCurrentTheme } from "@/shared/lib/theme-registry";
import { Icon } from "@/shared/ui";
import { ChevronUpGlyph } from "@/shared/icons";
import { playSound } from "@/shared/lib/personalization";

import { StartMenu } from "./StartMenu";
import { TrayMenu } from "./TrayMenu";
import { Clock } from "./Clock";
import { SHELL_ACTION_EVENT } from "@/shared/lib/guest-launch";
import type { ShellActionDetail } from "@/shared/lib/shortcut-target";

/** Windows 10 logo; themes can replace it via `components.StartButton`. */
function DefaultStartButton() {
  return (
    <svg className="ww-start-logo" viewBox="0 0 88 88" aria-hidden="true">
      <path
        fill="currentColor"
        d="M0,12.4L35.7,7.4v35.3H0V12.4z M39.6,6.9L88,0v42.7H39.6V6.9z M39.6,46.9H88V88l-48.4-6.9V46.9z M0,46.9h35.7v34L0,75.9V46.9z"
      />
    </svg>
  );
}

interface TaskbarProps {
  /** Hidden file inputs managed by the parent (Desktop) for uploads. */
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  folderInputRef: React.RefObject<HTMLInputElement | null>;
}

export function Taskbar({ fileInputRef, folderInputRef }: TaskbarProps) {
  const [startOpen, setStartOpen] = useState(false);
  const [trayOpen, setTrayOpen] = useState(false);
  const windows = useWindowStore((s) => s.windows);
  const activeId = useWindowStore((s) => s.activeId);
  const showLabels = useSettingsStore((s) => s.taskbarLabels);
  const theme = useCurrentTheme();
  const StartButton = theme?.components?.StartButton ?? DefaultStartButton;

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
    <div id="taskbar" className="ww-taskbar !fixed !z-[9000]" onContextMenu={(e) => e.preventDefault()}>
      <button
        id="start-button"
        type="button"
        className={`ww-start-button ${startOpen ? "open" : ""}`}
        title="Start"
        aria-haspopup="true"
        aria-expanded={startOpen}
        onClick={(e) => {
          e.stopPropagation();
          setTrayOpen(false);
          if (!startOpen) playSound("menu");
          setStartOpen((v) => !v);
        }}
      >
        <StartButton />
      </button>

      {startOpen && <StartMenu onClose={() => setStartOpen(false)} />}

      <div id="taskbar-window-list" className="ww-taskbar-items" aria-label="Open windows" role="toolbar">
        {windows.map((win) => (
          <button
            key={win.id}
            type="button"
            className={`ww-taskbar-item ${win.id === activeId && !win.minimized ? "active" : ""} ${win.minimized ? "minimized" : ""} ${showLabels ? "" : "icon-only"}`}
            title={win.title}
            onClick={() => useWindowStore.getState().activateFromTaskbar(win.id)}
          >
            <Icon icon={win.icon} size={theme?.taskbarIconSize ?? 16} />
            {showLabels && <span>{win.title}</span>}
          </button>
        ))}
      </div>

      <div id="taskbar-tray" className="ww-tray">
        <button
          id="tray-toggle"
          type="button"
          className="ww-tray-btn"
          title="Show hidden icons"
          aria-haspopup="true"
          aria-expanded={trayOpen}
          onClick={(e) => {
            e.stopPropagation();
            setStartOpen(false);
            setTrayOpen((v) => !v);
          }}
        >
          <ChevronUpGlyph />
        </button>

        {trayOpen && <TrayMenu onClose={() => setTrayOpen(false)} />}

        <Clock />
      </div>

      <button
        type="button"
        className="ww-show-desktop"
        title="Show desktop"
        onClick={() => useWindowStore.getState().minimizeAll()}
      />
    </div>
  );
}
