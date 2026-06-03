/**
 * Taskbar — the bottom bar with start button, window list, tray, and clock.
 */

import { useState, useCallback } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import { useRuntimeStore } from "../../stores/useRuntimeStore.js";
import { StartMenu, type StartMenuAction } from "./StartMenu.js";
import { TrayMenu } from "./TrayMenu.js";
import { Clock } from "./Clock.js";

const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";

interface TaskbarProps {
  /** Hidden file inputs managed by the parent (Desktop) for uploads. */
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  folderInputRef: React.RefObject<HTMLInputElement | null>;
}

export function Taskbar({ fileInputRef, folderInputRef }: TaskbarProps) {
  const [startOpen, setStartOpen] = useState(false);
  const [trayOpen, setTrayOpen] = useState(false);

  const { windows, activeId, activateFromTaskbar } = useWindowStore();
  const { runtime } = useRuntimeStore();

  const handleStartAction = useCallback(
    (action: StartMenuAction) => {
      if (!runtime) return;
      if (action === "explorer") {
        import("../../apps/explorer/ExplorerApp.js").then((m) =>
          m.openExplorer(DESKTOP_PATH, runtime),
        );
      } else if (action === "themes") {
        import("../../apps/theme-switcher/ThemeSwitcherApp.js").then((m) =>
          m.openThemeSwitcher(),
        );
      } else if (action === "upload-file") {
        fileInputRef.current?.click();
      } else if (action === "upload-folder") {
        folderInputRef.current?.click();
      }
    },
    [runtime, fileInputRef, folderInputRef],
  );

  return (
    <div id="taskbar">
      <button
        id="start-button"
        type="button"
        aria-haspopup="true"
        aria-expanded={startOpen}
        onClick={(e) => {
          e.stopPropagation();
          setTrayOpen(false);
          setStartOpen((v) => !v);
        }}
      >
        <span className="start-mark" aria-hidden="true" />
        <span>WebWINE</span>
      </button>

      {startOpen && (
        <StartMenu
          onAction={handleStartAction}
          onClose={() => setStartOpen(false)}
        />
      )}

      <div
        id="taskbar-window-list"
        aria-label="Open windows"
        role="toolbar"
      >
        {windows.map((win) => (
          <button
            key={win.id}
            className={[
              "taskbar-window-button",
              win.id === activeId ? "active" : "",
              win.minimized ? "minimized" : "",
              win.maximized ? "maximized" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            type="button"
            title={win.title}
            onClick={() => activateFromTaskbar(win.id)}
          >
            {win.icon && (
              <span className="taskbar-window-icon" aria-hidden="true">
                {win.icon}
              </span>
            )}
            <span className="taskbar-window-title">{win.title}</span>
          </button>
        ))}
      </div>

      <div id="taskbar-tray">
        <button
          id="tray-toggle"
          type="button"
          aria-label="Show hidden icons"
          aria-haspopup="true"
          aria-expanded={trayOpen}
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
